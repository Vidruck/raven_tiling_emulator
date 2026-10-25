/**
 * @file logger.js
 * @brief Módulo de registro estructurado y diagnóstico para el puente de KWin (Plasma 6).
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/**
 * @namespace Logger
 * @brief Objeto estático de registro de mensajes para el motor de scripts de KWin.
 *
 * Emite trazas estandarizadas visualizables mediante `journalctl -f | grep RAVEN`.
 */
var Logger = {
  /** @type {boolean} Bandera de depuración detallada. Desactivada por defecto para máxima eficiencia. */
  debug_enabled: false,

  /**
   * @brief Emite un mensaje informativo de operación estándar.
   * @param {string} ctx Contexto o nombre de la función emisora.
   * @param {string} msg Mensaje descriptivo.
   */
  info: function (ctx, msg) {
    print("[RAVEN] [INFO] [" + ctx + "] " + msg);
  },

  /**
   * @brief Emite una advertencia de anomalía recuperable o estado inconsistente.
   * @param {string} ctx Contexto de ejecución.
   * @param {string} msg Mensaje de advertencia.
   */
  warn: function (ctx, msg) {
    print("[RAVEN] [WARN] [" + ctx + "] " + msg);
  },

  /**
   * @brief Registra un error crítico junto con la traza de excepción asociada.
   * @param {string} ctx Contexto del error.
   * @param {string} msg Mensaje del error.
   * @param {Error|string} [err] Objeto de error o traza.
   */
  error: function (ctx, msg, err) {
    var trace = err ? " | Trace: " + err : "";
    print("[RAVEN] [ERROR] [" + ctx + "] " + msg + trace);
  },

  /**
   * @brief Emite mensajes de depuración sólo si debug_enabled es true.
   * @param {string} ctx Contexto.
   * @param {string} msg Mensaje de diagnóstico.
   */
  debug: function (ctx, msg) {
    if (this.debug_enabled) {
      print("[RAVEN] [DEBUG] [" + ctx + "] " + msg);
    }
  }
};
/**
 * @file timer_pool.js
 * @brief Pool estático de temporizadores reutilizables para optimización del recolector de basura (GC) en QJSEngine.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/** @type {number} Capacidad máxima de temporizadores preasignados en el pool. */
var TIMER_POOL_SIZE = 10;

/** @type {Array<{timer: QTimer, busy: boolean, callback: Function|null}>} Lista de slots de temporizadores. */
var _timer_pool = [];

/** @type {boolean} Indica si el pool ha completado su inicialización. */
var _timer_pool_ready = false;

/** @type {Array<{at: number, callback: Function}>} Cola de callbacks pendientes para ejecución diferida */
var _deferred_queue = [];
var _queue_timer = null;

/**
 * @brief Inicializa el pool de temporizadores estáticos preasignados.
 *
 * Crea instancias persistentes de QTimer para evitar la creación y destrucción constante
 * de objetos durante eventos masivos de redimensionamiento o movimiento.
 * Debe invocarse una única vez durante el ciclo de arranque `initDBusBridge()`.
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
          var cb = s.callback;
          s.callback = null;
          try { if (cb) cb(); } catch (e) { }
          processDeferredQueue();
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

function processDeferredQueue() {
  if (!_timer_pool_ready || _deferred_queue.length === 0) return;
  var now = Date.now();
  for (var i = 0; i < _deferred_queue.length; i++) {
    var item = _deferred_queue[i];
    if (item.at <= now) {
      _deferred_queue.splice(i, 1);
      i--;
      try { item.callback(); } catch (e) {}
    } else {
      var remaining = Math.max(0, item.at - now);
      for (var s = 0; s < _timer_pool.length; s++) {
        if (!_timer_pool[s].busy) {
          _deferred_queue.splice(i, 1);
          i--;
          _timer_pool[s].busy = true;
          _timer_pool[s].callback = item.callback;
          _timer_pool[s].timer.interval = remaining;
          _timer_pool[s].timer.start();
          break;
        }
      }
    }
  }
}

/**
 * @brief Ejecuta un callback después de un retardo temporal en milisegundos.
 *
 * Utiliza un slot preasignado del pool de temporizadores. Si el pool está temporalmente saturado,
 * agenda el callback en una cola de vencimiento procesada por un timer maestro persistente,
 * eliminando fugas de memoria y allocations continuas en QJSEngine.
 */
function setKWinTimeout(callback, delayMs) {
  if (_timer_pool_ready && _timer_pool.length > 0) {
    for (var i = 0; i < _timer_pool.length; i++) {
      var slot = _timer_pool[i];
      if (!slot.busy) {
        slot.busy = true;
        slot.callback = callback;
        slot.timer.interval = delayMs;
        slot.timer.start();
        return slot;
      }
    }
  }

  // Si los slots fijos están saturados, agendar en la cola persistente sin instanciar QTimers huérfanos
  _deferred_queue.push({ at: Date.now() + delayMs, callback: callback });
  if (!_queue_timer) {
    _queue_timer = new QTimer();
    _queue_timer.interval = 16;
    _queue_timer.timeout.connect(function () {
      processDeferredQueue();
      if (_deferred_queue.length === 0 && _queue_timer && _queue_timer.active) {
        _queue_timer.stop();
      }
    });
  }
  if (!_queue_timer.active) {
    _queue_timer.start();
  }
  return null;
}

/**
 * @file geometry.js
 * @brief Funciones auxiliares de cálculo y normalización geométrica de pantallas y ventanas en KWin.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/**
 * @brief Normaliza un rectángulo nativo de Qt/KWin a un objeto de coordenadas enteras estándar.
 *
 * Resuelve polimorfismos entre métodos de acceso `x()`/`width()` y propiedades directas `x`/`width`.
 *
 * @param {QtRect|Object} rect Estructura geométrica provista por el compositor.
 * @returns {{x: number, y: number, w: number, h: number}} Objeto normalizado con enteros redondeados.
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

function getSafeScreenGeometry(output, desktop) {
  if (!output) {
    return { x: 0, y: 0, w: 1920, h: 1080 };
  }

  // 1. Intentar clientArea con PlacementArea / MaximizeArea
  // En KWin 6 (QJSEngine), clientArea(ClientAreaOption, Output, VirtualDesktop)
  // KWin.PlacementArea = 0 (área de colocación libre excluyendo paneles)
  // KWin.MaximizeArea = 1 (área máxima para ventanas maximizadas excluyendo paneles y struts)
  var optionsToTry = [];
  if (typeof KWin !== "undefined") {
    if (KWin.PlacementArea !== undefined) optionsToTry.push(KWin.PlacementArea);
    if (KWin.MaximizeArea !== undefined) optionsToTry.push(KWin.MaximizeArea);
  }
  optionsToTry.push(0);
  optionsToTry.push(1);

  for (var i = 0; i < optionsToTry.length; i++) {
    try {
      var opt = optionsToTry[i];
      var area = workspace.clientArea(opt, output, desktop);
      if (area) {
        var geom = getRectGeometry(area);
        if (geom.w > 0 && geom.h > 0) {
          return geom;
        }
      }
    } catch (e) { }
  }

  // 2. Fallback a geometry del output físico si clientArea aún no está disponible
  try {
    if (output.geometry) {
      var outGeom = getRectGeometry(output.geometry);
      if (outGeom.w > 0 && outGeom.h > 0) {
        return outGeom;
      }
    }
  } catch (e) { }

  return { x: 0, y: 0, w: 1920, h: 1080 };
}
/**
 * @file window_utils.js
 * @brief Extracción y normalización ligera de metadatos de ventanas de KWin.
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.1
 */

function getSafeWindowId(w) {
  try {
    return (w && w.internalId) ? w.internalId.toString() : null;
  } catch (e) {
    return null;
  }
}

function isManageable(w) {
  try {
    if (!w || w.deleted) return false;
    if (w.__raven_manageable !== undefined) return w.__raven_manageable;
    if (!w.managed) {
      w.__raven_manageable = false;
      return false;
    }
    if (w.popupWindow || w.tooltip || w.onScreenDisplay || w.notification || w.criticalNotification || w.appletPopup || w.specialWindow || w.splash) {
      w.__raven_manageable = false;
      return false;
    }
    if (w.desktopWindow || w.dock || w.skipTaskbar || w.skipPager) {
      w.__raven_manageable = false;
      return false;
    }

    const strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
    const strName = w.resourceName ? w.resourceName.toString().toLowerCase() : "";
    const strCap = w.caption ? w.caption.toString().toLowerCase() : "";

    // Filtrar herramientas específicas del sistema solicitadas por el usuario
    if (strClass.indexOf("spectacle") !== -1 || strName.indexOf("spectacle") !== -1 ||
        strClass.indexOf("flameshot") !== -1 || strName.indexOf("flameshot") !== -1 ||
        strClass.indexOf("ksnip") !== -1 || strName.indexOf("ksnip") !== -1) {
      w.__raven_manageable = false;
      return false;
    }

    if (strClass.indexOf("plasma-welcome") !== -1 || strClass.indexOf("plasma.welcome") !== -1 ||
        strClass.indexOf("welcome") !== -1 || strName.indexOf("welcome") !== -1 ||
        strName.indexOf("plasma-welcome") !== -1 || strCap.indexOf("centro de bienvenida") !== -1 ||
        strCap.indexOf("welcome center") !== -1 || strCap.indexOf("bienvenida de plasma") !== -1 ||
        strCap.indexOf("bienvenida") !== -1) {
      w.__raven_manageable = false;
      return false;
    }

    if (strClass.indexOf("plasmashell") !== -1 ||
        strClass.indexOf("kscreenlocker") !== -1 || strClass.indexOf("screenlocker") !== -1 ||
        strClass.indexOf("lockscreen") !== -1 || strClass.indexOf("plasmalogin") !== -1 ||
        strClass.indexOf("sddm") !== -1 ||
        strClass.indexOf("login") !== -1 || strName.indexOf("login") !== -1 ||
        strCap.indexOf("inicio de sesión") !== -1 || strCap.indexOf("pantalla de inicio de sesión") !== -1 ||
        strCap.indexOf("login screen") !== -1 || strCap.indexOf("bloqueo de pantalla") !== -1 ||
        strCap.indexOf("screen lock") !== -1) {
      w.__raven_manageable = false;
      return false;
    }

    if (!w.normalWindow && !w.dialog && !w.utility) {
      w.__raven_manageable = false;
      return false;
    }

    if (w.frameGeometry && (w.frameGeometry.width <= 0 || w.frameGeometry.height <= 0)) {
      return false; // No cachear si la geometría aún no está inicializada
    }

    w.__raven_manageable = true;
    return true;
  } catch (e) {
    return false;
  }
}

function isWindowNativeFloating(w) {
  try {
    if (!w || w.deleted || w.__raven_dynamic_float) return true;
    if (w.specialWindow || w.modal || w.transient || w.transientFor != null) return true;
    if (w.dialog || w.utility) return true;
    if (w.fullScreen) return false;
    if (w.maximizeMode === 3) return true;
    if (w.resizeable === false) return true;

    var cls = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
    var name = w.resourceName ? w.resourceName.toString().toLowerCase() : "";
    var cap = w.caption ? w.caption.toString().toLowerCase() : "";
    if (cls.indexOf("zuno") !== -1 || name.indexOf("zuno") !== -1 || cap.indexOf("zuno") !== -1 ||
        cls.indexOf("raven_gui") !== -1 || name.indexOf("raven_gui") !== -1 || cap.indexOf("raven") !== -1) {
      return true;
    }

    return false;
  } catch (e) {
    return true;
  }
}

/**
 * @brief Identifica heurísticamente si una ventana es Picture-in-Picture (PiP).
 */
function isPipWindow(w) {
  try {
    if (!w || w.deleted) return false;
    const cap = w.caption ? w.caption.toString().toLowerCase() : "";
    if (
      cap.indexOf("picture-in-picture") !== -1 ||
      cap.indexOf("picture in picture") !== -1 ||
      cap.indexOf("imagen en imagen") !== -1 ||
      cap.indexOf("pantalla en pantalla") !== -1 ||
      cap.indexOf("reproductor en miniatura") !== -1 ||
      cap.indexOf("incrustation") !== -1 ||
      cap.indexOf("bild-in-bild") !== -1 ||
      cap.indexOf("imagem em imagem") !== -1 ||
      cap === "pip"
    ) {
      return true;
    }
    return false;
  } catch (e) {
    return false;
  }
}

/**
 * @file quarantine.js
 * @brief Registro inicial de ventanas hacia el demonio Rust.
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.0 — Delegación completa a raven_backend_kwin
 *
 * La lógica de cuarentena, categorización, filtrado de flood y temporizadores de
 * estabilización (Cold/Warm start) ahora reside en Rust (`KWinQuarantineManager`).
 * El script de KWin simplemente vincula los listeners y despacha el delta de estado.
 */

/**
 * @brief Evalúa y procesa la incorporación de una nueva ventana al sistema de mosaico.
 *
 * @param {KWin::Window} w Instancia de la ventana naciente.
 */
function processNewWindow(w) {
  if (!w || w.deleted || !isManageable(w)) {
    return;
  }

  // Asignar inicialmente 'encima de todas las ventanas' (keepAbove) a ventanas PiP
  if (isPipWindow(w) && !w.__raven_pip_init) {
    w.__raven_pip_init = true;
    w.keepAbove = true;
  }

  bindWindow(w);
  requestStateSync();
}

/**
 * @file focus.js
 * @brief Resalte visual interactivo mediante el sistema de contorno (Outline) nativo de KWin.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/**
 * @brief Destaca momentáneamente una ventana proyectando el marco de contorno (Outline) del compositor.
 *
 * Utilizado al conmutar foco mediante atajos (`Meta+J` / `Meta+K`) para dar retroalimentación visual inmediata.
 * El contorno se desvanece automáticamente tras 200 ms.
 *
 * @param {KWin::Window} w Instancia de la ventana a destacar.
 */
function highlightWindow(w) {
  try {
    if (!w) return;
    if (workspace.showOutline) {
      workspace.showOutline(w.frameGeometry);
      setKWinTimeout(function() {
        if (workspace.hideOutline) workspace.hideOutline();
      }, 200);
    }
  } catch (e) { }
}

/**
 * @file dbus_bridge.js
 * @brief Orquestador D-Bus ultraligero entre KWin y el demonio Raven en Rust.
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.0
 */

var _debounceTimer = null;

/**
 * @brief Solicita una sincronización global de estado agrupando eventos en 20ms.
 */
function requestStateSync() {
  try {
    if (!_debounceTimer) {
      _debounceTimer = new QTimer();
      _debounceTimer.interval = 20;
      _debounceTimer.singleShot = true;
      _debounceTimer.timeout.connect(syncState);
    }
    if (_debounceTimer.active) _debounceTimer.stop();
    _debounceTimer.start();
  } catch (e) {
    try { syncState(); } catch (err) { }
  }
}

/**
 * @brief Extrae la topología y lista de ventanas y la despacha a Rust.
 */
function syncState() {
  const windows = workspace.windowList();
  const winState = [];
  const screens = {};
  const outs = workspace.screens || [];
  const desks = workspace.desktops || [];
  const currentDesk = workspace.currentDesktop;
  const masterOutputs = [];
  const masterDesktops = [];

  try {
    for (let o = 0; o < outs.length; o++) {
      const out = outs[o];
      if (out && out.name) {
        masterOutputs.push(out.name.toString());
        if (desks.length > 0) {
          for (let d = 0; d < desks.length; d++) {
            const desk = desks[d];
            if (desk && desk.id) {
              screens[out.name + "||" + desk.id.toString()] = getSafeScreenGeometry(out, desk);
            }
          }
        } else {
          const deskId = currentDesk ? currentDesk.id.toString() : "default_desk";
          screens[out.name + "||" + deskId] = getSafeScreenGeometry(out, currentDesk);
        }
      }
    }
    for (let d = 0; d < desks.length; d++) {
      if (desks[d] && desks[d].id) masterDesktops.push(desks[d].id.toString());
    }
  } catch (e) { }

  for (let i = 0; i < windows.length; i++) {
    const w = windows[i];
    try {
      if (!isManageable(w)) continue;
      const safeId = getSafeWindowId(w);
      if (safeId) winState.push(buildWindowState(w, safeId));
    } catch (e) { }
  }

  const payload = {
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
        if (response && response !== "[]") applyCommands(response);
      }
    );
  } catch (e) {
    Logger.error("syncState", "Fallo enviando payload D-Bus", e);
  }
}

/**
 * @brief Sincroniza incrementalmente el estado de una ventana individual.
 */
function syncWindowDelta(w) {
  try {
    if (!w || w.deleted || !isManageable(w)) return;
    const safeId = getSafeWindowId(w);
    if (!safeId) return;

    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "syncWindowDelta",
      JSON.stringify(buildWindowState(w, safeId)),
      function (response) {
        if (response && response !== "[]") applyCommands(response);
      }
    );
  } catch (e) {
    Logger.error("syncWindowDelta", "Fallo en delta sync", e);
  }
}

/**
 * @brief Migra una ventana a un monitor o escritorio virtual.
 */
function migrateWindow(win, target_output_name, target_desktop_id) {
  if (!win || win.deleted) return;
  try {
    if (target_output_name) {
      const outputs = workspace.screens || [];
      for (let i = 0; i < outputs.length; i++) {
        if (outputs[i] && outputs[i].name === target_output_name) {
          try { if (typeof workspace.sendClientToScreen === "function") workspace.sendClientToScreen(win, outputs[i]); } catch (err) {}
          try { win.output = outputs[i]; } catch (errOut) {}
          break;
        }
      }
    }
    if (target_desktop_id) {
      const desktops = workspace.desktops || [];
      for (let j = 0; j < desktops.length; j++) {
        if (desktops[j] && desktops[j].id.toString() === target_desktop_id) {
          win.desktops = [desktops[j]];
          break;
        }
      }
    }
  } catch (e) { }
}

/**
 * @brief Comprueba si una ventana madre tiene actualmente un diálogo, confirmación o hija transitoria abierta.
 */
function hasActiveTransientOrModalChild(parentWin, allWindows) {
  if (!parentWin || parentWin.deleted) return false;
  const list = allWindows || workspace.windowList();
  for (let i = 0; i < list.length; i++) {
    const child = list[i];
    if (!child || child === parentWin || child.deleted || child.minimized || child.hidden) continue;
    if (child.transientFor === parentWin) {
      return true;
    }
  }
  return false;
}

/**
 * @brief Aplica las acciones calculadas por Rust en KWin.
 */
function applyCommands(commandsJson) {
  if (!commandsJson) return;
  try {
    const cmds = JSON.parse(commandsJson);
    if (!cmds || !cmds.length) return;

    const windows = workspace.windowList();
    const winMap = {};
    for (let i = 0; i < windows.length; i++) {
      const w = windows[i];
      const id = getSafeWindowId(w);
      if (id) winMap[id] = w;
    }

    for (let i = 0; i < cmds.length; i++) {
      const cmd = cmds[i];
      if (cmd.action === "request_sync") {
        requestStateSync();
        continue;
      }
      if (cmd.action === "saturation_warning") {
        Logger.warn("Saturation", "Saturación: " + cmd.active + "/" + cmd.cmax);
        continue;
      }

      const w = cmd.window_id ? winMap[cmd.window_id] : null;
      if (!w || w.deleted) continue;

      switch (cmd.action) {
        case "set_floating":
          w.__raven_dynamic_float = Boolean(cmd.floating);
          w.keepAbove = Boolean(cmd.keep_above);
          break;

        case "set_keep_above":
          w.keepAbove = Boolean(cmd.keep_above);
          break;

        case "set_maximize":
          w.__raven_mutating = true;
          try {
            const shouldMax = Boolean(cmd.maximized !== undefined ? cmd.maximized : cmd.floating);
            w.setMaximize(shouldMax, shouldMax);
          } catch (eMax) {}
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 60);
          })(w);
          break;

        case "move":
        case "rectify_window":
          if (w.minimized || w.interactiveMove || w.interactiveResize || w.fullScreen) break;
          if (w.maximized || (w.maximizeMode !== undefined && w.maximizeMode === 3)) break;
          if (!w.normalWindow || w.transient || w.transientFor != null || w.modal || w.dialog || w.utility) break;
          if (hasActiveTransientOrModalChild(w, windows)) break;

          // Si la ventana tiene maximización parcial (ej. Teams con MAXIMIZED_VERT),
          // desmarcarla en KWin para que acepte libremente la nueva geometría
          if (w.maximizeMode !== undefined && w.maximizeMode !== 0) {
            try {
              w.setMaximize(false, false);
            } catch (eUnmax) {}
          }

          const targetGeom = {
            x: Math.round(cmd.x),
            y: Math.round(cmd.y),
            width: Math.round(cmd.width),
            height: Math.round(cmd.height),
          };

          w.__raven_mutating = true;
          w.frameGeometry = targetGeom;

          // Auditoría y verificación suave de convergencia:
          // Si el cliente Wayland CSD (Gecko, GTK, Electron) confirma su superficie inicial
          // milisegundos después y se queda en el centro, re-aplicamos targetGeom de inmediato.
          (function (cw, tgt, wid) {
            setKWinTimeout(function () {
              if (!cw || cw.deleted) return;
              if (cw.minimized) {
                cw.__raven_mutating = false;
                return;
              }
              try {
                var cur = cw.frameGeometry;
                if (cur && (Math.abs(cur.x - tgt.x) > 2 || Math.abs(cur.y - tgt.y) > 2 ||
                            Math.abs(cur.width - tgt.width) > 2 || Math.abs(cur.height - tgt.height) > 2)) {
                  cw.frameGeometry = tgt;
                }
              } catch (eReapply) {}

              try {
                var fgApplied = cw.frameGeometry || tgt;
                callDBus(
                  "org.kde.raven.Daemon",
                  "/Events",
                  "org.kde.raven.Events",
                  "commandAppliedState",
                  wid,
                  Math.round(fgApplied.x),
                  Math.round(fgApplied.y),
                  Math.round(fgApplied.width),
                  Math.round(fgApplied.height)
                );
              } catch (eAudit) {}

              cw.__raven_mutating = false;
            }, 60);
          })(w, targetGeom, cmd.window_id);
          break;

        case "focus":
          if (!w.minimized && workspace.activeWindow !== w &&
              (!workspace.activeWindow || (!workspace.activeWindow.popupWindow && !workspace.activeWindow.modal && !workspace.activeWindow.dialog && !workspace.activeWindow.transient && workspace.activeWindow.transientFor == null)) &&
              !hasActiveTransientOrModalChild(w, windows)) {
            workspace.activeWindow = w;
          }
          break;

        case "minimize":
          if (!w.minimized) {
            w.__raven_mutating = true;
            try {
              w.minimized = true;
            } catch (eMin) {}

            try {
              if (workspace.activeWindow === w) {
                var nextWin = null;
                for (var mi = 0; mi < windows.length; mi++) {
                  var cand = windows[mi];
                  if (cand && cand !== w && !cand.deleted && !cand.minimized && !cand.hidden && isManageable(cand)) {
                    nextWin = cand;
                    break;
                  }
                }
                if (nextWin) {
                  workspace.activeWindow = nextWin;
                }
              }
            } catch (eFoc) {}

            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                }
              }, 300);
            })(w);
          }
          break;

        case "unminimize":
          if (w.minimized) {
            w.__raven_mutating = true;
            try {
              w.minimized = false;
            } catch (eUnmin) {}
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                }
              }, 300);
            })(w);
          }
          break;

        case "close":
          try {
            if (typeof w.closeWindow === "function") {
              w.closeWindow();
            }
          } catch (eClose) {}
          break;

        case "migrate_to_output":
          w.__raven_mutating = true;
          migrateWindow(w, cmd.target_ws, null);
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 40);
          })(w);
          break;

        case "migrate_to_desktop":
          w.__raven_mutating = true;
          migrateWindow(w, null, cmd.target_ws);
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 40);
          })(w);
          break;
      }
    }
  } catch (e) {
    Logger.error("applyCommands", "Error procesando comandos", e);
  }
}

/**
 * @brief Enlaza reactivamente los eventos del ciclo de vida de una ventana.
 */
function bindWindow(w) {
  try {
    if (!isManageable(w) || w.__raven_bound) return;
    w.__raven_bound = true;

    const onStateChange = function () {
      if (w && !w.deleted && !w.__raven_mutating && !w.interactiveMove && !w.interactiveResize) {
        requestStateSync();
      }
    };

    w.minimizedChanged.connect(onStateChange);
    w.maximizedChanged.connect(onStateChange);
    if (w.fullScreenChanged !== undefined) w.fullScreenChanged.connect(onStateChange);
    if (w.transientChanged !== undefined) {
      w.transientChanged.connect(function () {
        delete w.__raven_manageable;
        requestStateSync();
      });
    }
    if (w.modalChanged !== undefined) {
      w.modalChanged.connect(function () {
        delete w.__raven_manageable;
        requestStateSync();
      });
    }

    if (w.captionChanged !== undefined) {
      w.captionChanged.connect(function () {
        delete w.__raven_manageable;
        if (w && !w.deleted && !w.__raven_mutating) {
          syncWindowDelta(w);
        }
      });
    }

    const onOutputOrDesktop = function () {
      if (!w || w.deleted || w.__raven_mutating) return;
      if (!w.interactiveMove && !w.interactiveResize) {
        w.__raven_ui_migrating = true;
        (function (cw) {
          setKWinTimeout(function () {
            if (cw && !cw.deleted) cw.__raven_ui_migrating = false;
          }, 60);
        })(w);
      }
      requestStateSync();
    };

    w.outputChanged.connect(onOutputOrDesktop);
    w.desktopsChanged.connect(onOutputOrDesktop);

    w.frameGeometryChanged.connect(function () {
      if (!w || w.deleted) return;
      if (w.interactiveMove || w.interactiveResize) {
        w.__was_interacting = true;
        return;
      }
      if (w.__was_interacting && !w.interactiveMove && !w.interactiveResize) {
        w.__was_interacting = false;
        requestStateSync();
        return;
      }
      if (w.__raven_mutating || w.__raven_ui_migrating) return;
      if (hasActiveTransientOrModalChild(w, null)) return;

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
    Logger.error("bindWindow", "Error enlazando ventana", e);
  }
}

/**
 * @brief Extrae y normaliza el estado espacial de una ventana.
 */
function buildWindowState(w, safeId) {
  const geom = getRectGeometry(w.frameGeometry);
  const deskIds = [];
  if (w.desktops) {
    for (let d = 0; d < w.desktops.length; d++) deskIds.push(w.desktops[d].id.toString());
  }
  const output = w.output || workspace.activeOutput;
  return {
    id: safeId,
    desktops: deskIds,
    output: output ? output.name : "default",
    f: isWindowNativeFloating(w),
    m: Boolean(w.minimized),
    p: Boolean(w.keepAbove),
    x: geom.x,
    y: geom.y,
    w: geom.w,
    h: geom.h,
    min_w: w.minSize ? Math.round(w.minSize.width) : 0,
    min_h: w.minSize ? Math.round(w.minSize.height) : 0,
    fs: Boolean(w.fullScreen),
    max: Boolean(w.maximized || (w.maximizeMode !== undefined && w.maximizeMode === 3)),
    cls: w.resourceClass ? w.resourceClass.toString() : "",
    cls_name: w.resourceName ? w.resourceName.toString() : "",
    cap: w.caption ? w.caption.toString() : "",
  };
}
/**
 * @file shortcuts.js
 * @brief Registro de atajos de teclado globales en KWin (KDE Plasma 6).
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.0
 */

function registerRavenShortcuts() {
  function dispatchToRaven(actionStr, arg) {
    try {
      if (arg !== undefined) {
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", actionStr, arg, function (res) {
          if (res && res !== "[]") applyCommands(res);
        });
      } else {
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", actionStr, function (res) {
          if (res && res !== "[]") applyCommands(res);
        });
      }
    } catch (e) {
      Logger.error("Shortcuts", "Fallo al enviar atajo D-Bus: " + actionStr, e);
    }
  }

  // Gestión de estado y flotación
  registerShortcut("RavenToggleTiling", "Raven: Alternar Mosaico (On/Off)", "Meta+Space", function () {
    try {
      callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", "toggleTiling", function (response) {
        if (response && response !== "[]") applyCommands(response);
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", "getTilingState", function (stateRes) {
          if (stateRes === "true" || stateRes === true) syncState();
        });
      });
    } catch (e) {
      Logger.error("Shortcuts", "Fallo en RavenToggleTiling: " + e);
    }
  });

  registerShortcut("RavenToggleFloating", "Raven: Alternar Ventana Flotante Dinámica", "Meta+Shift+F", function () {
    const aw = workspace.activeWindow;
    dispatchToRaven("toggleFloating", aw ? getSafeWindowId(aw) || "" : "");
  });

  registerShortcut("RavenToggleMaximize", "Raven: Alternar Maximizar Ventana", "Meta+F", function () {
    const aw = workspace.activeWindow;
    dispatchToRaven("toggleMaximize", aw ? getSafeWindowId(aw) || "" : "");
  });

  registerShortcut("RavenMinimizeActive", "Raven: Minimizar Ventana", "Meta+X", function () {
    const aw = workspace.activeWindow;
    dispatchToRaven("minimizeActive", aw ? getSafeWindowId(aw) || "" : "");
  });

  registerShortcut("RavenCloseActive", "Raven: Cerrar Ventana", "Meta+Q", function () {
    const aw = workspace.activeWindow;
    dispatchToRaven("closeActive", aw ? getSafeWindowId(aw) || "" : "");
  });

  // Navegación y foco
  registerShortcut("RavenFocusNext", "Raven: Siguiente Ventana", "Meta+J", function () { dispatchToRaven("focusNext"); });
  registerShortcut("RavenFocusPrev", "Raven: Ventana Anterior", "Meta+K", function () { dispatchToRaven("focusPrev"); });
  registerShortcut("RavenFocusLeft", "Raven: Foco Izquierda", "Meta+Left", function () { dispatchToRaven("focusLeft"); });
  registerShortcut("RavenFocusRight", "Raven: Foco Derecha", "Meta+Right", function () { dispatchToRaven("focusRight"); });
  registerShortcut("RavenFocusUp", "Raven: Foco Arriba", "Meta+Up", function () { dispatchToRaven("focusUp"); });
  registerShortcut("RavenFocusDown", "Raven: Foco Abajo", "Meta+Down", function () { dispatchToRaven("focusDown"); });

  // Intercambio y ratios
  registerShortcut("RavenSwapNext", "Raven: Intercambiar Siguiente", "Meta+Shift+J", function () { dispatchToRaven("swapNext"); });
  registerShortcut("RavenSwapPrev", "Raven: Intercambiar Anterior", "Meta+Shift+K", function () { dispatchToRaven("swapPrev"); });
  registerShortcut("RavenIncreaseRatio", "Raven: Expandir Master", "Meta+H", function () { dispatchToRaven("increaseRatio"); });
  registerShortcut("RavenDecreaseRatio", "Raven: Contraer Master", "Meta+L", function () { dispatchToRaven("decreaseRatio"); });

  // Migración entre monitores y escritorios
  registerShortcut("RavenMigrateMonitor", "Raven: Enviar a Monitor Siguiente", "Meta+Shift+M", function () { dispatchToRaven("migrateActiveToScreen"); });
  registerShortcut("RavenMigratePrevMonitor", "Raven: Enviar a Monitor Anterior", "Meta+Shift+N", function () { dispatchToRaven("migrateActiveToPrevScreen"); });
  registerShortcut("RavenMigrateDesktop", "Raven: Enviar a Escritorio Siguiente", "Meta+Shift+Right", function () { dispatchToRaven("migrateActiveToDesktop"); });
  registerShortcut("RavenMigratePrevDesktop", "Raven: Enviar a Escritorio Anterior", "Meta+Shift+Left", function () { dispatchToRaven("migrateActiveToPrevDesktop"); });

  // Márgenes, capacidad y layouts
  registerShortcut("RavenIncrementGaps", "Raven: Incrementar Gaps", "Meta+=", function () { dispatchToRaven("incrementGaps", 2); });
  registerShortcut("RavenDecrementGaps", "Raven: Decrementar Gaps", "Meta+-", function () { dispatchToRaven("incrementGaps", -2); });
  registerShortcut("RavenIncrementMaster", "Raven: Incrementar Capacidad Master", "Meta+]", function () { dispatchToRaven("incrementMaster"); });
  registerShortcut("RavenDecrementMaster", "Raven: Decrementar Capacidad Master", "Meta+[", function () { dispatchToRaven("decrementMaster"); });
  registerShortcut("RavenCycleLayout", "Raven: Ciclar Algoritmo de Disposición", "Meta+Shift+L", function() {
    dispatchToRaven("cycleLayout");
    if (workspace.activeWindow) highlightWindow(workspace.activeWindow);
  });

  // Redimensionamiento fino por ventana
  registerShortcut("RavenResizeWidthInc", "Raven: Aumentar Ancho de Ventana", "Meta+Alt+Right", function () { dispatchToRaven("resize_width_inc"); });
  registerShortcut("RavenResizeWidthDec", "Raven: Reducir Ancho de Ventana", "Meta+Alt+Left", function () { dispatchToRaven("resize_width_dec"); });
  registerShortcut("RavenResizeHeightInc", "Raven: Aumentar Alto de Ventana", "Meta+Alt+Down", function () { dispatchToRaven("resize_height_inc"); });
  registerShortcut("RavenResizeHeightDec", "Raven: Reducir Alto de Ventana", "Meta+Alt+Up", function () { dispatchToRaven("resize_height_dec"); });
}
/**
 * @file index.js
 * @brief Punto de entrada modular y orquestador del puente Raven (Raven Bridge) en KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */


/**
 * @brief Inicializa el ciclo de vida del puente D-Bus y enlaza las señales del compositor KWin.
 *
 * Secuencia de arranque:
 * 1. Inicializa el pool estático de temporizadores (`initTimerPool`).
 * 2. Enlaza todas las ventanas gestionables ya presentes en el espacio de trabajo (`bindWindow`).
 * 3. Registra ganchos de eventos del compositor (`windowAdded`, `windowRemoved`, `activeWindowChanged`, `currentDesktopChanged`).
 * 4. Suscribe la señal D-Bus `tilingCommandsPending` y notifica `bridgeReady` al demonio Rust.
 * 5. Ejecuta la primera sincronización de estado global (`requestStateSync`).
 */
function initDBusBridge() {
  // 1. Inicializar pool de timers estáticos
  initTimerPool();

  // 2. Registrar atajos de teclado de KWin
  try {
    if (typeof registerRavenShortcuts === "function") {
      registerRavenShortcuts();
    }
  } catch (eShortcuts) {
    Logger.error("Main", "Error registrando atajos", eShortcuts);
  }

  // 3. Enlazar ventanas existentes al puente (sin disparar syncs masivos)
  var existingWindows = workspace.windowList();
  for (var i = 0; i < existingWindows.length; i++) {
    var w = existingWindows[i];
    if (w && !w.deleted && isManageable(w)) {
      bindWindow(w);
    }
  }

  // 3. Hooks de ciclo de vida de ventanas
  workspace.windowAdded.connect(function (w) {
    processNewWindow(w);
  });

  workspace.windowRemoved.connect(function (w) {
    // Blindaje: notificar cierre inmediato al motor Rust o sincronizar si era subventana/diálogo
    if (w) {
      var safeId = getSafeWindowId(w);
      if (safeId && w.__raven_bound) {
        try {
          callDBus(
            "org.kde.raven.Daemon",
            "/Events",
            "org.kde.raven.Events",
            "windowClosed",
            safeId,
            function (response) {
              if (response && response !== "[]") applyCommands(response);
            }
          );
          return;
        } catch (e) {}
      }
      if (w.__raven_bound || w.normalWindow || w.dialog || w.utility) {
        requestStateSync();
      }
    }
  });

  if (workspace.windowActivated) {
    workspace.windowActivated.connect(function (aw) {
      if (!aw || aw.deleted) {
        return;
      }
      // Blindaje de popups y menús desplegables: ignorar paneles, subventanas hijas y popups
      if (!isManageable(aw) || aw.transientFor != null || aw.popupWindow) {
        return;
      }
      var awId = getSafeWindowId(aw);
      if (awId) {
        try {
          callDBus(
            "org.kde.raven.Daemon",
            "/Events",
            "org.kde.raven.Events",
            "windowActivated",
            awId
          );
        } catch (e) { }
      }
    });
  }

  workspace.currentDesktopChanged.connect(function () {
    requestStateSync();
  });

  // Escuchar cambios en el área útil de trabajo (ej. aparición, redimensionamiento o movimiento del panel de Plasma)
  if (workspace.clientAreaChanged) {
    workspace.clientAreaChanged.connect(function () {
      requestStateSync();
    });
  }

  // Escuchar cambios o reconfiguración de pantallas
  if (workspace.screensChanged) {
    workspace.screensChanged.connect(function () {
      requestStateSync();
    });
  }

  // 4. Solicitar configuración del daemon y notificar arranque de forma diferida (100ms)
  setKWinTimeout(function () {
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "bridgeReady"
      );
    } catch (e) { }

    try {
      registerDBusSignal(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "tilingCommandsPending",
        function (commandsJson) {
          if (commandsJson && commandsJson !== "[]") {
            applyCommands(commandsJson);
          }
        }
      );
    } catch (e) { }

    // Sincronización inicial única y limpia tras levantar el entorno
    requestStateSync();
  }, 100);
}

// Registro e inicialización de ciclo de vida en el motor de scripting de KWin
try {
  Logger.info("Main", "Inicializando el puente de Raven Tiling Emulator v3.4");
  initDBusBridge();
  Logger.info("Main", "Puente inicializado exitosamente");
} catch (e) {
  Logger.error("Main", "Error crítico al inicializar el puente", e);
}
