/**
 * @fileoverview Puente de Raven (Raven Bridge) para KDE Plasma 6 (Wayland) — v2.8
 * Proporciona la integración entre el compositor de ventanas KWin y el
 * motor de mosaico (tiling engine) nativo en Rust a través de D-Bus.
 *
 * Arquitectura Push-Based (v2.8):
 *   - El daemon Rust invoca receiveCommands() directamente cuando calcula un layout.
 *   - Si el push falla o el bridge no está listo, listenForCommands() actúa como fallback
 *     con un intervalo extendido de 500ms para minimizar carga.
 *
 * @author Alejandro González Hernández (Vidruck)
 */

// --- Globals de estado ---
var _debounceTimer = null;
var _is_listening = false;
var _watchdog_timer = null;
var _push_mode_active = false;  // true cuando el canal push D-Bus está funcionando

// Diccionario global de estado de ventanas indexado por internalId (UUID string).
// Evita crear objetos temporales en los manejadores de eventos.
var _window_state = {};

// Lista de clases de ventana que requieren cuarentena de estabilización geométrica.
var _quarantine_classes = [];

// Lista de reglas de ventanas.
var _window_rules = [];

// --- Pool estático de timers reutilizables ---
// Evita la creación/destrucción dinámica de QTimer que presiona al GC de QJSEngine.
var TIMER_POOL_SIZE = 10;
var _timer_pool = [];
var _timer_pool_ready = false;

/**
 * Inicializa el pool de timers estáticos preasignados.
 * Debe llamarse una sola vez durante init().
 */
function initTimerPool() {
  try {
    for (var i = 0; i < TIMER_POOL_SIZE; i++) {
      var t = new QTimer();
      t.singleShot = true;
      var slot = { timer: t, busy: false, callback: null };
      (function(s) {
        s.timer.timeout.connect(function() {
          s.busy = false;
          try { if (s.callback) s.callback(); } catch(e) {}
          s.callback = null;
        });
      })(slot);
      _timer_pool.push(slot);
    }
    _timer_pool_ready = true;
  } catch (e) {
    print("[Raven] Error inicializando pool de timers: " + e);
    _timer_pool_ready = false;
  }
}

try {
  _debounceTimer = new QTimer();
  _debounceTimer.interval = 50;
  _debounceTimer.singleShot = true;
  _debounceTimer.timeout.connect(syncState);
} catch (e) {
  print("[Raven Bridge] Error inicializando timer de debounce: " + e);
}

/**
 * Obtiene de forma segura el identificador único (ID) de una ventana.
 *
 * @param {KWin::Window} w - Objeto de ventana de KWin.
 * @returns {string|null} Identificador único en formato cadena de texto (string) o null si es inválido.
 */
function getSafeWindowId(w) {
  try {
    if (!w || !w.internalId) {
      return null;
    }
    return w.internalId.toString();
  } catch (e) {
    return null;
  }
}

/**
 * Obtiene el identificador único del área de trabajo (workspace ID) para una ventana.
 * Combina el nombre de la salida (output) y el identificador del escritorio virtual (virtual desktop ID).
 *
 * @param {KWin::Window} window - Objeto de ventana de KWin.
 * @returns {string} Identificador único del área de trabajo en formato "salida||escritorio".
 */
function getWorkspaceId(window) {
  try {
    if (!window || window.deleted) {
      return "default||default_desk";
    }
    var output = window.output || workspace.activeOutput;
    var outName = output ? output.name : "default";
    var desktopId =
      window.desktops && window.desktops.length > 0
        ? window.desktops[0].id.toString()
        : workspace.currentDesktop
          ? workspace.currentDesktop.id.toString()
          : "default_desk";
    return outName + "||" + desktopId;
  } catch (e) {
    return "default||default_desk";
  }
}

/**
 * Determina si una ventana es gestionable (manageable) por el motor de mosaico (tiling engine).
 *
 * @param {KWin::Window} w - Objeto de ventana de KWin.
 * @returns {boolean} Verdadero si la ventana debe ser gestionada; de lo contrario, falso.
 */
function isManageable(w) {
  try {
    if (!w || w.deleted || !w.managed) {
      return false;
    }
    if (
      w.popupWindow ||
      w.tooltip ||
      w.onScreenDisplay ||
      w.notification ||
      w.specialWindow
    ) {
      return false;
    }
    if (w.desktopWindow || w.dock || w.splash || w.skipTaskbar || w.skipPager) {
      return false;
    }

    var strClass = w.resourceClass
      ? w.resourceClass.toString().toLowerCase()
      : "";
    if (strClass.indexOf("spectacle") !== -1 && w.fullScreen) {
      return false;
    }
    if (!w.normalWindow && !w.dialog && !w.utility) {
      return false;
    }

    return true;
  } catch (e) {
    return false;
  }
}

/**
 * Determina si una ventana debe comportarse como flotante (floating).
 *
 * @param {KWin::Window} w - Objeto de ventana de KWin.
 * @returns {boolean} Verdadero si es flotante; de lo contrario, falso.
 */
function isFloating(w) {
  try {
    if (!w || w.deleted) {
      return true;
    }
    if (w.dialog || w.utility || w.specialWindow || w.modal || w.transientFor) {
      return true;
    }
    if (w.fullScreen) {
      return true;
    }

    var strClass = w.resourceClass
      ? w.resourceClass.toString().toLowerCase()
      : "";
    var strCap = w.caption ? w.caption.toString().toLowerCase() : "";

    var isPip =
      strCap.indexOf("picture-in-picture") !== -1 ||
      strCap.indexOf("picture in picture") !== -1 ||
      strCap.indexOf("pictureinpicture") !== -1 ||
      strCap.indexOf("imagen en imagen") !== -1 ||
      strCap.indexOf("imagen-en-imagen") !== -1 ||
      strCap.indexOf("reproductor en miniatura") !== -1 ||
      strCap.indexOf("incrustation") !== -1 ||
      strCap.indexOf("bild-in-bild") !== -1 ||
      strCap.indexOf("bild in bild") !== -1 ||
      strCap.indexOf("imagem em imagem") !== -1 ||
      strCap.indexOf("immagine nell'immagine") !== -1 ||
      strCap.indexOf("pip") !== -1 ||
      w.keepAbove;
      
    // Evaluamos reglas dinámicas
    if (_window_rules && _window_rules.length > 0) {
      for (var i = 0; i < _window_rules.length; i++) {
        var rule = _window_rules[i];
        if (rule && rule.class && strClass.indexOf(rule.class.toLowerCase()) !== -1) {
          if (rule.pip) {
            isPip = true;
          }
          if (rule.action === "float") {
            if (isPip && !w.keepAbove) w.keepAbove = true;
            return true;
          }
        }
      }
    }

    if (isPip && !w.keepAbove) {
      w.keepAbove = true;
    }

    var isRaven =
      strClass.indexOf("raven") !== -1 ||
      strCap.indexOf("raven control center") !== -1;
    var isSpectacle = strClass.indexOf("spectacle") !== -1;
    var isKlipper =
      strClass.indexOf("klipper") !== -1 ||
      strClass.indexOf("plasma.clipboard") !== -1;
    var isVirtPopup =
      (strClass.indexOf("qemu") !== -1 ||
        strClass.indexOf("virt-manager") !== -1) &&
      !w.normalWindow;

    return Boolean(isPip || isSpectacle || isKlipper || isVirtPopup || isRaven);
  } catch (e) {
    return true;
  }
}

/**
 * Solicita de forma asíncrona la sincronización de estado (state sync) con filtrado de rebotes (debouncing).
 */
function requestStateSync() {
  try {
    if (!_debounceTimer) {
      _debounceTimer = new QTimer();
      _debounceTimer.interval = 50;
      _debounceTimer.singleShot = true;
      _debounceTimer.timeout.connect(syncState);
    }
    if (_debounceTimer.active) {
      _debounceTimer.stop();
    }
    _debounceTimer.start();
  } catch (e) {
    print("[Raven] Error en requestStateSync: " + e);
    try {
      sinkState();
    } catch (err) {}
  }
}

/**
 * Normaliza y obtiene un objeto de geometría en coordenadas enteras de pantalla a partir de un Rect.
 *
 * @param {QtRect} rect - Estructura de geometría nativa de Qt.
 * @returns {Object} Objeto con las propiedades normalizadas {x, y, w, h}.
 */
function getRectGeometry(rect) {
  if (!rect) {
    return { x: 0, y: 0, w: 1920, h: 1080 };
  }
  
  function getProp(obj, p1, p2, def) {
    if (typeof obj[p1] === "function") return obj[p1]();
    if (obj[p1] !== undefined) return obj[p1];
    if (typeof obj[p2] === "function") return obj[p2]();
    if (obj[p2] !== undefined) return obj[p2];
    return def;
  }

  return {
    x: Math.round(getProp(rect, "x", "x", 0)),
    y: Math.round(getProp(rect, "y", "y", 0)),
    w: Math.round(getProp(rect, "width", "w", 1920)),
    h: Math.round(getProp(rect, "height", "h", 1080)),
  };
}
/**
 * Obtiene de forma segura el área útil de la pantalla (screen geometry) para un escritorio virtual y salida dados.
 *
 * @param {KWin::Output} output - Salida física de pantalla.
 * @param {KWin::VirtualDesktop} desktop - Escritorio virtual.
 * @returns {Object} Geometría útil del área de trabajo.
 */
function getSafeScreenGeometry(output, desktop) {
  if (!output) {
    return { x: 0, y: 0, w: 1920, h: 1080 };
  }
  try {
    var area = workspace.clientArea(0, output, desktop);
    if (area && area.width > 0 && area.height > 0) {
      return getRectGeometry(area);
    }
  } catch (e) {}
  try {
    if (output.geometry) {
      return getRectGeometry(output.geometry);
    }
  } catch (e) {}
  return { x: 0, y: 0, w: 1920, h: 1080 };
}

/**
 * Sincroniza el estado completo del compositor enviándolo al demonio (daemon) de Rust vía D-Bus.
 */
function syncState() {
  var windows = workspace.windowList();
  var winState = [];
  var screens = {};

  var outs = workspace.screens || [];
  var desks = workspace.desktops || [];
  var currentDesk = workspace.currentDesktop;

  try {
    for (var o = 0; o < outs.length; o++) {
      var output = outs[o];
      var outName = output ? output.name : "default";

      if (desks && desks.length > 0) {
        for (var d = 0; d < desks.length; d++) {
          var desktop = desks[d];
          var deskId = desktop ? desktop.id.toString() : "default_desk";
          var wsId = outName + "||" + deskId;
          screens[wsId] = getSafeScreenGeometry(output, desktop);
        }
      } else {
        var deskId = currentDesk ? currentDesk.id.toString() : "default_desk";
        var wsId = outName + "||" + deskId;
        screens[wsId] = getSafeScreenGeometry(output, currentDesk);
      }
    }
  } catch (e) {
    print("[Raven] Error topología pantallas: " + e);
  }

  for (var i = 0; i < windows.length; i++) {
    var w = windows[i];
    try {
      if (!isManageable(w) || w.__raven_quarantined) {
        continue;
      }
      var safeId = getSafeWindowId(w);
      if (!safeId) {
        continue;
      }

      var output = w.output || workspace.activeOutput;
      var outName = output ? output.name : "default";

      var deskIds = [];
      if (w.desktops) {
        for (var d = 0; d < w.desktops.length; d++) {
          deskIds.push(w.desktops[d].id.toString());
        }
      }

      var wsId = getWorkspaceId(w);
      var geom = getRectGeometry(w.frameGeometry);

      winState.push({
        id: safeId,
        ws: wsId,
        desktops: deskIds,
        output: outName,
        f: isFloating(w),
        m: Boolean(w.minimized),
        p: Boolean(w.keepAbove),
        x: geom.x,
        y: geom.y,
        w: geom.w,
        h: geom.h,
        min_w: w.minSize ? Math.round(w.minSize.width) : 0,
        min_h: w.minSize ? Math.round(w.minSize.height) : 0,
        sb: Boolean(w.__raven_strict_birth),
      });
    } catch (e) {
      print("[Raven] Error mapeando ventana: " + e);
    }
  }

  var masterOutputs = [];
  for (var o = 0; o < outs.length; o++) {
    if (outs[o] && outs[o].name) {
      masterOutputs.push(outs[o].name.toString());
    }
  }

  var masterDesktops = [];
  for (var d = 0; d < desks.length; d++) {
    if (desks[d] && desks[d].id) {
      masterDesktops.push(desks[d].id.toString());
    }
  }

  var payload = {
    windows: winState,
    screens: screens,
    topology: {
      outputs: masterOutputs,
      desktops: masterDesktops,
    },
  };

  try {
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "syncState",
      JSON.stringify(payload),
    );
  } catch (e) {
    print("[Raven Bridge] D-bus Drop: " + e);
  }
}

/**
 * Sincroniza de forma incremental el cambio de geometría o estado (delta sync) de una única ventana.
 *
 * @param {KWin::Window} w - Objeto de ventana modificado.
 */
function syncWindowDelta(w) {
  try {
    if (!w || w.deleted || !isManageable(w) || w.__raven_quarantined) {
      return;
    }
    var safeId = getSafeWindowId(w);
    if (!safeId) {
      return;
    }

    var geom = getRectGeometry(w.frameGeometry);
    var deskIds = [];
    if (w.desktops) {
      for (var d = 0; d < w.desktops.length; d++) {
        deskIds.push(w.desktops[d].id.toString());
      }
    }

    var deltaPayload = {
      id: safeId,
      ws: getWorkspaceId(w),
      output: w.output ? w.output.name : "default",
      desktops: deskIds,
      f: isFloating(w),
      m: Boolean(w.minimized),
      p: Boolean(w.keepAbove),
      x: geom.x,
      y: geom.y,
      w: geom.w,
      h: geom.h,
      min_w: w.minSize ? Math.round(w.minSize.width) : 0,
      min_h: w.minSize ? Math.round(w.minSize.height) : 0,
      sb: Boolean(w.__raven_strict_birth),
    };
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "syncWindowDelta",
      JSON.stringify(deltaPayload),
    );
  } catch (e) {
    print("[Raven] Error Delta Sync: " + e);
  }
}

/**
 * Migra nativamente una ventana a una pantalla (output) o escritorio virtual específico.
 *
 * @param {KWin::Window} win - Objeto de ventana.
 * @param {string|null} target_output_name - Nombre de la salida destino o null.
 * @param {string|null} target_desktop_id - Identificador del escritorio virtual destino o null.
 */
function migrateWindow(win, target_output_name, target_desktop_id) {
  if (!win || win.deleted) {
    return;
  }
  try {
    if (target_output_name) {
      var outputs = workspace.screens || [];
      for (var i = 0; i < outputs.length; i++) {
        if (outputs[i].name === target_output_name) {
          workspace.sendClientToScreen(win, outputs[i]);
          break;
        }
      }
    }
    if (target_desktop_id) {
      var desktops = workspace.desktops || [];
      for (var j = 0; j < desktops.length; j++) {
        if (desktops[j].id.toString() === target_desktop_id) {
          win.desktops = [desktops[j]];
          break;
        }
      }
    }
  } catch (e) {
    print("[Raven] Fallo en migración nativa: " + e);
  }
}

/**
 * Procesa y aplica los comandos JSON recibidos desde el demonio (daemon) de Rust.
 *
 * @param {string} commandsJson - Carga de comandos serializada en JSON.
 */
function applyCommands(commandsJson) {
  if (!commandsJson) {
    return;
  }
  try {
    var cmds = JSON.parse(commandsJson);
    var windows = workspace.windowList();

    for (var i = 0; i < cmds.length; i++) {
      var cmd = cmds[i];
      if (cmd.action === "request_sync") {
        requestStateSync();
        continue;
      }

      for (var j = 0; j < windows.length; j++) {
        var w = windows[j];
        if (getSafeWindowId(w) === cmd.window_id) {
          if (!w || w.deleted) {
            break;
          }

          if (cmd.action === "move") {
            try {
              if (
                w.interactiveMove ||
                w.interactiveResize ||
                w.__raven_ui_migrating
              ) {
                break;
              }
              
              // Forzar desmaximización antes de aplicar frameGeometry
              // Si la ventana está maximizada (interna de KWin), ignorará los gaps o el frameGeometry dictado
              if (w.maximizeMode !== 0 && typeof w.setMaximize === "function") {
                w.setMaximize(false, false);
              }

              w.__raven_mutating = true;
              w.frameGeometry = {
                x: Math.round(cmd.x),
                y: Math.round(cmd.y),
                width: Math.round(cmd.width),
                height: Math.round(cmd.height),
              };

              (function (capturedWindow) {
                setKWinTimeout(function () {
                  if (capturedWindow && !capturedWindow.deleted) {
                    capturedWindow.__raven_mutating = false;
                  }
                }, 400);
              })(w);
            } catch (e) {}
          } else if (cmd.action === "focus") {
            workspace.activeWindow = w;
          } else if (cmd.action === "request_feedback") {
            if (w.__raven_strict_birth) {
              w.__raven_strict_birth = false;

              (function (cw) {
                setKWinTimeout(function () {
                  if (cw && !cw.deleted) {
                    requestStateSync();
                  }
                }, 480);
              })(w);
            }
          } else if (cmd.action === "minimize") {
            w.__raven_mutating = true;
            w.minimized = true;
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                  requestStateSync();
                }
              }, 100);
            })(w);
          } else if (cmd.action === "unminimize") {
            w.__raven_mutating = true;
            w.minimized = false;
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                  requestStateSync();
                }
              }, 100);
            })(w);
          } else if (cmd.action === "migrate_to_output") {
            w.__raven_mutating = true;
            migrateWindow(w, cmd.target_ws, null);
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                  requestStateSync();
                }
              }, 150);
            })(w);
          } else if (cmd.action === "migrate_to_desktop") {
            w.__raven_mutating = true;
            migrateWindow(w, null, cmd.target_ws);
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                  requestStateSync();
                }
              }, 150);
            })(w);
          }
          break;
        }
      }
    }
  } catch (e) {
    print("[Raven Bridge] Error applyCommands: " + e);
  }
}

/**
 * Ejecuta una función después de un retardo usando el pool estático de timers.
 * Evita crear/destruir QTimer dinámicamente (reduce presión sobre el GC de QJSEngine).
 *
 * @param {function} callback - Función a ejecutar al completarse el tiempo.
 * @param {number} ms - Tiempo de espera en milisegundos.
 * @returns {object|null} Entrada del pool usada, o null si no hay disponibles.
 */
function setKWinTimeout(callback, ms) {
  // Intentar usar un slot disponible del pool estático
  if (_timer_pool_ready) {
    for (var i = 0; i < _timer_pool.length; i++) {
      var slot = _timer_pool[i];
      if (!slot.busy) {
        // La señal timeout ya fue conectada permanentemente en initTimerPool
        slot.busy = true;
        slot.callback = callback;
        slot.timer.interval = ms;
        slot.timer.start();
        return slot;
      }
    }
  }
  // Fallback: crear timer efímero si el pool está lleno
  try {
    var t = new QTimer();
    t.interval = ms;
    t.singleShot = true;
    t.timeout.connect(function () {
      try { callback(); } catch(e) {}
      try { t.stop(); } catch(err) {}
    });
    t.start();
    return t;
  } catch (e) {
    try { callback(); } catch (err) {}
    return null;
  }
}

/**
 * CANAL PUSH (v2.8): Método público invocado directamente por el daemon Rust via D-Bus.
 * Cuando el daemon calcula un nuevo layout, llama a este método en lugar de esperar
 * a que el bridge lo solicite (eliminando la latencia del long-polling).
 *
 * Al recibir comandos por push, activa _push_mode_active para que el fallback
 * (listenForCommands) se duerma y no consuma CPU innecesariamente.
 *
 * @param {string} commandsJson - Carga de comandos serializada en JSON desde el daemon.
 */
function receiveCommands(commandsJson) {
  // Activar modo push: el fallback pollér pausará su frecuencia
  if (!_push_mode_active) {
    _push_mode_active = true;
    print("[Raven Bridge] ✅ Canal Push D-Bus activo. Reduciendo frecuencia de fallback.");
  }
  if (commandsJson && commandsJson !== "[]") {
    applyCommands(commandsJson);
  }
}

/**
 * CANAL FALLBACK: Escucha comandos pendientes mediante polling cuando el canal push
 * no está disponible. Intervalo base: 500ms en reposo, 30ms cuando hay actividad.
 * Si _push_mode_active está activado, el intervalo se extiende a 2000ms para
 * minimizar el impacto en CPU mientras el canal push opera.
 */
function listenForCommands() {
  if (_is_listening) {
    return;
  }
  _is_listening = true;
  if (_watchdog_timer) {
    try {
      _watchdog_timer.stop();
    } catch (e) {}
  }
  _watchdog_timer = setKWinTimeout(function () {
    _is_listening = false;
    listenForCommands();
  }, 6000);

  try {
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "getPendingCommands",
      function (response) {
        if (_watchdog_timer) {
          try {
            _watchdog_timer.stop();
          } catch (e) {}
        }
        _is_listening = false;

        if (response && response !== "[]") {
          applyCommands(response);
          // Hay actividad: sondear rápido, pero si push está activo, es un extra
          setKWinTimeout(listenForCommands, _push_mode_active ? 2000 : 30);
        } else {
          // Reposo: 500ms en modo normal, 2000ms si el push está cubriendo el trabajo
          setKWinTimeout(listenForCommands, _push_mode_active ? 2000 : 500);
        }
      },
    );
  } catch (e) {
    _is_listening = false;
    setKWinTimeout(listenForCommands, 1000);
  }
}

/**
 * Enlaza (binds) los eventos principales de una ventana a las funciones de sincronización del puente de Raven.
 *
 * @param {KWin::Window} w - Objeto de ventana.
 */
function bindWindow(w) {
  try {
    if (!isManageable(w) || w.__raven_bound) {
      return;
    }
    w.__raven_bound = true;

    w.minimizedChanged.connect(function () {
      if (
        w &&
        !w.deleted &&
        !w.__raven_mutating &&
        !w.interactiveMove &&
        !w.interactiveResize
      ) {
        requestStateSync();
      }
    });

    w.maximizedChanged.connect(function () {
      if (
        w &&
        !w.deleted &&
        !w.__raven_mutating &&
        !w.interactiveMove &&
        !w.interactiveResize
      ) {
        requestStateSync();
      }
    });

    w.outputChanged.connect(function () {
      if (!w || w.deleted || w.__raven_mutating) {
        return;
      }
      if (!w.interactiveMove && !w.interactiveResize) {
        w.__raven_ui_migrating = true;
        (function (cw) {
          setKWinTimeout(function () {
            if (cw && !cw.deleted) {
              cw.__raven_ui_migrating = false;
            }
          }, 250);
        })(w);
      }
      requestStateSync();
    });

    w.desktopsChanged.connect(function () {
      if (!w || w.deleted || w.__raven_mutating) {
        return;
      }
      if (!w.interactiveMove && !w.interactiveResize) {
        w.__raven_ui_migrating = true;
        (function (cw) {
          setKWinTimeout(function () {
            if (cw && !cw.deleted) {
              cw.__raven_ui_migrating = false;
            }
          }, 250);
        })(w);
      }
      requestStateSync();
    });

    w.frameGeometryChanged.connect(function () {
      if (!w || w.deleted) {
        return;
      }
      if (w.__raven_quarantined && w.__raven_stab_timer) {
        w.__raven_stab_timer.stop();
        w.__raven_stab_timer.start();
        return;
      }

      if (w.interactiveMove || w.interactiveResize) {
        w.__was_interacting = true;
        return;
      }
      if (w.__was_interacting && !w.interactiveMove && !w.interactiveResize) {
        w.__was_interacting = false;
        requestStateSync();
        return;
      }
      if (w.__raven_mutating || w.__raven_ui_migrating) {
        return;
      }

      syncWindowDelta(w);
    });

    if (w.interactiveMoveResizeFinished !== undefined) {
      w.interactiveMoveResizeFinished.connect(function () {
        if (w && !w.deleted) {
          w.__was_interacting = false;
          requestStateSync();
        }
      });
    }
  } catch (e) {
    print("[Raven] Error bindWindow: " + e);
  }
}
/**
 * Inicializa el script puente de Raven conectando los listeners de KWin y disparando la sincronización inicial.
 */
function init() {
  print("[Raven Bridge] Inicializando v2.8 (Push-Based con Fallback)...");

  // Inicializar pool de timers estáticos (debe ser lo primero)
  initTimerPool();

  var initialWindows = workspace.windowList();
  for (var i = 0; i < initialWindows.length; i++) {
    bindWindow(initialWindows[i]);
  }

  // Conectar con funciones estáticas nombradas (no closures anónimas)
  workspace.windowAdded.connect(onWindowAdded);
  workspace.windowRemoved.connect(onWindowRemoved);
  workspace.currentDesktopChanged.connect(onDesktopChanged);
  workspace.windowActivated.connect(onWindowActivated);

  try {
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "bridgeReady",
      function () {
        // Al estar listo, pedimos las configuraciones dinámicas
        try {
          callDBus(
            "org.kde.raven.Daemon",
            "/Events",
            "org.kde.raven.Events",
            "getQuarantineClasses",
            function (res) {
              if (res) _quarantine_classes = JSON.parse(res);
            }
          );
        } catch (e) {}
        try {
          callDBus(
            "org.kde.raven.Daemon",
            "/Events",
            "org.kde.raven.Events",
            "getWindowRules",
            function (res) {
              if (res) _window_rules = JSON.parse(res);
            }
          );
        } catch (e) {}
      },
    );
  } catch (e) {}

  requestStateSync();
  // Arrancar el canal fallback (se auto-suprime cuando push está activo)
  listenForCommands();
}

// ---- Manejadores de eventos globales (funciones estáticas, sin closures) ----

function onWindowAdded(w) {
  if (!isManageable(w)) {
    return;
  }
  
  // Forzar desmaximización al nacer para evitar que navegadores/Electron 
  // restauren su estado maximizado previo y floten sobre los gaps.
  try {
    if (w.maximizeMode !== 0 && typeof w.setMaximize === "function") {
      w.setMaximize(false, false);
    }
  } catch(e) {}

  setKWinTimeout(function () {
    if (!w || w.deleted) {
      return;
    }

    var strClass = w.resourceClass
      ? w.resourceClass.toString().toLowerCase()
      : "";
    var needsQuarantine = false;

    if (strClass === "") {
      needsQuarantine = true;
    } else {
      for (var i = 0; i < _quarantine_classes.length; i++) {
        if (strClass.indexOf(_quarantine_classes[i]) !== -1) {
          needsQuarantine = true;
          break;
        }
      }
    }

    if (needsQuarantine) {
      w.__raven_quarantined = true;
      bindWindow(w);
      // Usar pool de timers para la cuarentena de 250ms
      setKWinTimeout(function() {
        if (w && !w.deleted) {
          // Re-asegurar que no se maximizó durante la cuarentena
          try {
            if (w.maximizeMode !== 0 && typeof w.setMaximize === "function") {
              w.setMaximize(false, false);
            }
          } catch(e) {}
          
          w.__raven_quarantined = false;
          w.__raven_strict_birth = true;
          w.__raven_stab_timer = null;
          requestStateSync();
        }
      }, 300);
    } else {
      bindWindow(w);
      requestStateSync();
    }
  }, 60);
}

/** Manejador estático de evento 'windowRemoved'. */
function onWindowRemoved() {
  requestStateSync();
}

/** Manejador estático de evento 'currentDesktopChanged'. */
function onDesktopChanged() {
  requestStateSync();
}

/**
 * Manejador estático de evento 'windowActivated'.
 * @param {KWin::Window} w - Ventana activada.
 */
function onWindowActivated(w) {
  if (w && isManageable(w)) {
    var id = getSafeWindowId(w);
    if (id) {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "windowActivated",
        id,
        function () {},
      );
    }
  }
}

try {
  init();
} catch (e) {
  print("[Raven Bridge] Error crítico: " + e);
}
