/**
 * @fileoverview Puente de Raven (Raven Bridge) para KDE Plasma 6 (Wayland) — v3.0
 * Proporciona la integración entre el compositor de ventanas KWin y el
 * motor de mosaico (tiling engine) nativo en Rust a través de D-Bus.
 *
 * Arquitectura Single-Trip (v3.0):
 *   - El puente opera 100% libre de polling (bucle de consulta).
 *   - Las peticiones asíncronas de sincronización (D-Bus) retornan la nueva geometría en el mismo viaje.
 *   - Reduce drásticamente la latencia y la carga del Garbage Collector en QJSEngine.
 *
 * @author Alejandro González Hernández (Vidruck)
 */

// --- Logger ---
var Logger = {
  // Activa esto a true si necesitas depurar localmente. Se deja en false por eficiencia.
  debug_enabled: false,

  info: function (ctx, msg) {
    print("[RAVEN] [INFO] [" + ctx + "] " + msg);
  },
  warn: function (ctx, msg) {
    print("[RAVEN] [WARN] [" + ctx + "] " + msg);
  },
  error: function (ctx, msg, err) {
    var trace = err ? " | Trace: " + err : "";
    print("[RAVEN] [ERROR] [" + ctx + "] " + msg + trace);
  },
  debug: function (ctx, msg) {
    if (this.debug_enabled) {
      print("[RAVEN] [DEBUG] [" + ctx + "] " + msg);
    }
  }
};

// --- Globals de estado ---
var _debounceTimer = null;

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
      (function (s) {
        s.timer.timeout.connect(function () {
          s.busy = false;
          try { if (s.callback) s.callback(); } catch (e) { }
          s.callback = null;
        });
      })(slot);
      _timer_pool.push(slot);
    }
    _timer_pool_ready = true;
  } catch (e) {
    Logger.error("initTimerPool", "Error inicializando pool de timers", e);
    _timer_pool_ready = false;
  }
}

try {
  _debounceTimer = new QTimer();
  _debounceTimer.interval = 50;
  _debounceTimer.singleShot = true;
  _debounceTimer.timeout.connect(syncState);
} catch (e) {
  Logger.error("Global", "Error inicializando timer de debounce", e);
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
      strCap.indexOf("pantalla en pantalla") !== -1 ||
      strCap.indexOf("reproductor en miniatura") !== -1 ||
      strCap.indexOf("incrustation") !== -1 ||
      strCap.indexOf("bild-in-bild") !== -1 ||
      strCap.indexOf("bild in bild") !== -1 ||
      strCap.indexOf("imagem em imagem") !== -1 ||
      strCap.indexOf("immagine nell'immagine") !== -1 ||
      strCap === "pip" ||
      strCap === "picture in picture" ||
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
    Logger.error("requestStateSync", "Fallo al solicitar sincronización de estado", e);
    try {
      sinkState();
    } catch (err) { }
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
  } catch (e) { }
  try {
    if (output.geometry) {
      return getRectGeometry(output.geometry);
    }
  } catch (e) { }
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
    Logger.error("syncState", "Error iterando topología de pantallas", e);
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
      Logger.error("syncState", "Error extrayendo geometría/estado de ventana", e);
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
      current_desktop: currentDesk ? currentDesk.id.toString() : "",
    },
  };

  try {
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "syncStateAndUpdateLayout",
      JSON.stringify(payload),
      function (response) {
        if (response && response !== "[]") {
          applyCommands(response);
        }
      }
    );
  } catch (e) {
    Logger.error("syncState", "D-Bus Drop: Fallo enviando payload", e);
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
      function (response) {
        if (response && response !== "[]") {
          applyCommands(response);
        }
      }
    );
  } catch (e) {
    Logger.error("syncWindowDelta", "Fallo en sincronización incremental", e);
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
    Logger.error("migrateWindow", "Fallo al migrar ventana", e);
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
              var targetGeom = {
                x: Math.round(cmd.x),
                y: Math.round(cmd.y),
                width: Math.round(cmd.width),
                height: Math.round(cmd.height),
              };

              var dbgCls = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
              if (dbgCls.indexOf("zen") !== -1 || dbgCls.indexOf("firefox") !== -1) {
                Logger.info("ZenDebug", "Applying commands [" + w.internalId + "] TARGET=" + targetGeom.width + "x" + targetGeom.height + " minSize=" + (w.minSize ? w.minSize.width + "x" + w.minSize.height : "null"));
              }

              w.frameGeometry = targetGeom;

              (function (capturedWindow) {
                setKWinTimeout(function () {
                  if (capturedWindow && !capturedWindow.deleted) {
                    capturedWindow.__raven_mutating = false;
                  }
                }, 400);
              })(w);
            } catch (e) { }
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
    Logger.error("applyCommands", "Fallo crítico aplicando comandos del daemon. Payload: " + commandsJson, e);
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
      try { callback(); } catch (e) { }
      try { t.stop(); } catch (err) { }
    });
    t.start();
    return t;
  } catch (e) {
    try { callback(); } catch (err) { }
    return null;
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

    if (w.captionChanged !== undefined) {
      w.captionChanged.connect(function () {
        if (
          w &&
          !w.deleted &&
          !w.__raven_mutating
        ) {
          requestStateSync();
        }
      });
    }

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
      var dbgCls = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
      var isGecko = dbgCls.indexOf("zen") !== -1 || dbgCls.indexOf("firefox") !== -1;

      if (w.__raven_quarantined && w.__raven_stab_timer) {
        if (isGecko) {
          Logger.info("ZenDebug", "Quarantine MUTATION [" + w.internalId + "] geom=" + w.frameGeometry.width + "x" + w.frameGeometry.height + " minSize=" + (w.minSize ? w.minSize.width + "x" + w.minSize.height : "null"));
        }
        var t = w.__raven_stab_timer;
        if (t.timer) {
          t.timer.stop();
          t.timer.start();
        } else if (typeof t.stop === "function") {
          t.stop();
          t.start();
        }
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

      if (isGecko) {
        Logger.info("ZenDebug", "Delta SYNC [" + w.internalId + "] geom=" + w.frameGeometry.width + "x" + w.frameGeometry.height + " minSize=" + (w.minSize ? w.minSize.width + "x" + w.minSize.height : "null"));
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
    Logger.error("bindWindow", "Error enlazando eventos de ventana", e);
  }
}
/**
 * Registra los atajos globales nativos de KWin para controlar a Raven.
 * 
 * Expone las acciones del gestor de ventanas al panel de preferencias del sistema.
 * Utiliza llamadas D-Bus Single-Trip para aplicar el layout inmediatamente después del atajo.
 */
function registerRavenShortcuts() {
  function dispatchToRaven(actionStr) {
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        actionStr,
        function (response) {
          if (response && response !== "[]") {
            applyCommands(response);
          }
        }
      );
    } catch (e) {
      Logger.error("Shortcuts", "Fallo al enviar atajo D-Bus: " + actionStr, e);
    }
  }

  function dispatchToRavenArg(actionStr, arg) {
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        actionStr,
        arg,
        function (response) {
          if (response && response !== "[]") {
            applyCommands(response);
          }
        }
      );
    } catch (e) {
      Logger.error("Shortcuts", "Fallo al enviar atajo D-Bus con arg: " + actionStr, e);
    }
  }

  registerShortcut("RavenToggleTiling", "Raven: Alternar Mosaico (On/Off)", "Meta+Space", function () {
    dispatchToRaven("toggleTiling");
  });
  registerShortcut("RavenFocusNext", "Raven: Enfocar Siguiente", "Meta+J", function () {
    dispatchToRaven("focusNext");
  });
  registerShortcut("RavenFocusPrev", "Raven: Enfocar Anterior", "Meta+K", function () {
    dispatchToRaven("focusPrev");
  });
  registerShortcut("RavenSwapNext", "Raven: Intercambiar Siguiente", "Meta+Shift+J", function () {
    dispatchToRaven("swapNext");
  });
  registerShortcut("RavenSwapPrev", "Raven: Intercambiar Anterior", "Meta+Shift+K", function () {
    dispatchToRaven("swapPrev");
  });
  registerShortcut("RavenIncreaseRatio", "Raven: Expandir Master", "Meta+H", function () {
    dispatchToRaven("increaseRatio");
  });
  registerShortcut("RavenDecreaseRatio", "Raven: Contraer Master", "Meta+L", function () {
    dispatchToRaven("decreaseRatio");
  });
  registerShortcut("RavenMigrateMonitor", "Raven: Enviar a Otro Monitor", "Meta+Shift+M", function () {
    dispatchToRaven("migrateActiveToScreen");
  });

  // Shortcuts para uso desde Plasmoid / Externo
  registerShortcut("RavenIncrementGaps", "Raven: Incrementar Gaps", "Meta+=", function () {
    dispatchToRavenArg("incrementGaps", 2);
  });
  registerShortcut("RavenDecrementGaps", "Raven: Decrementar Gaps", "Meta+-", function () {
    dispatchToRavenArg("incrementGaps", -2);
  });
  registerShortcut("RavenIncrementMaster", "Raven: Incrementar Master", "Meta+]", function () {
    dispatchToRaven("incrementMaster");
  });
  registerShortcut("RavenDecrementMaster", "Raven: Decrementar Master", "Meta+[", function () {
    dispatchToRaven("decrementMaster");
  });
  registerShortcut("RavenMigratePrevMonitor", "Raven: Enviar a Monitor Anterior", "Meta+Shift+N", function () {
    dispatchToRaven("migrateActiveToPrevScreen");
  });
  registerShortcut("RavenMigrateDesktop", "Raven: Enviar a Escritorio Siguiente", "Meta+Shift+Right", function () {
    dispatchToRaven("migrateActiveToDesktop");
  });
  registerShortcut("RavenMigratePrevDesktop", "Raven: Enviar a Escritorio Anterior", "Meta+Shift+Left", function () {
    dispatchToRaven("migrateActiveToPrevDesktop");
  });
}

/**
 * Inicializa el script puente de Raven conectando los listeners de KWin y disparando la sincronización inicial.
 */
function init() {
  Logger.info("init", "Inicializando v2.9 (Push-Based)...");

  // Inicializar pool de timers estáticos (debe ser lo primero)
  initTimerPool();

  // Inyectar los atajos nativos de KWin
  registerRavenShortcuts();

  var initialWindows = workspace.windowList();
  for (var i = 0; i < initialWindows.length; i++) {
    bindWindow(initialWindows[i]);
  }

  // Conectar señales globales
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
        } catch (e) { }
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
        } catch (e) { }
      },
    );
  } catch (e) { }

  requestStateSync();
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
  } catch (e) { }

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
      if (strClass.indexOf("zen") !== -1 || strClass.indexOf("firefox") !== -1) {
        Logger.info("ZenDebug", "onWindowAdded [" + w.internalId + "] " + strClass + " geom=" + w.frameGeometry.width + "x" + w.frameGeometry.height + " minSize=" + (w.minSize ? w.minSize.width + "x" + w.minSize.height : "null"));
      }
      w.__raven_quarantined = true;
      bindWindow(w);

      // Heurística de Cold Start vs Warm Start
      var qTime = 50; // Warm start por defecto
      if (strClass !== "") {
        var similarCount = 0;
        var allW = workspace.windowList();
        for (var k = 0; k < allW.length; k++) {
          var wc = allW[k].resourceClass ? allW[k].resourceClass.toString().toLowerCase() : "";
          if (wc === strClass) {
            similarCount++;
          }
        }
        if (similarCount <= 1) {
          qTime = 100; // Cold start
        }
      } else {
        qTime = 150; // Si nace sin clase, darle un poco más de tiempo
      }

      // Usar pool de timers estático para la cuarentena dinámica
      w.__raven_stab_timer = setKWinTimeout(function () {
        if (w && !w.deleted) {
          // Re-asegurar que no se maximizó durante la cuarentena
          try {
            if (w.maximizeMode !== 0 && typeof w.setMaximize === "function") {
              w.setMaximize(false, false);
            }
          } catch (e) { }

          w.__raven_quarantined = false;
          w.__raven_strict_birth = true;
          w.__raven_stab_timer = null;
          if (strClass.indexOf("zen") !== -1 || strClass.indexOf("firefox") !== -1) {
            Logger.info("ZenDebug", "Quarantine ENDED [" + w.internalId + "] " + strClass + " geom=" + w.frameGeometry.width + "x" + w.frameGeometry.height);
          }
          requestStateSync();
        }
      }, qTime);
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
        function () { },
      );
    }
  }
}

try {
  init();
} catch (e) {
  Logger.error("Global", "Error crítico inicializando el bridge", e);
}
