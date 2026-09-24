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

/**
 * @brief Ejecuta un callback después de un retardo temporal en milisegundos.
 *
 * Reutiliza un slot libre de QTimer del pool estático. Si todos los slots están ocupados,
 * recurre a una asignación dinámica de reserva (*fallback*).
 *
 * @param {Function} callback Función a ejecutar al expirar el tiempo.
 * @param {number} delayMs Tiempo de espera en milisegundos.
 * @returns {Object|QTimer|null} Referencia al temporizador asignado o null en caso de fallo crítico.
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

  // Fallback a asignación dinámica si el pool se encuentra saturado
  try {
    var fallbackTimer = new QTimer();
    fallbackTimer.singleShot = true;
    fallbackTimer.interval = delayMs;
    fallbackTimer.timeout.connect(function () {
      try { callback(); } catch (e) { }
    });
    fallbackTimer.start();
    return fallbackTimer;
  } catch (e) {
    Logger.error("setKWinTimeout", "Error en fallback de temporizador", e);
    return null;
  }
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

/**
 * @brief Obtiene de forma segura el área útil de trabajo (excluyendo paneles y docks de Plasma)
 * para un monitor y escritorio virtual especificados.
 *
 * @param {KWin::Output} output Salida física o monitor reportado por KWin.
 * @param {KWin::VirtualDesktop} desktop Escritorio virtual activo.
 * @returns {{x: number, y: number, w: number, h: number}} Geometría utilizable del espacio de trabajo.
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
 * @file window_utils.js
 * @brief Funciones de clasificación, filtrado y evaluación heurística de ventanas de KWin.
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.0
 */

var _window_rules = [];

function getSafeWindowId(w) {
  try {
    return (w && w.internalId) ? w.internalId.toString() : null;
  } catch (e) {
    return null;
  }
}

function getWorkspaceId(window) {
  try {
    if (!window || window.deleted) return "default||default_desk";
    const out = window.output || workspace.activeOutput;
    const outName = out ? out.name : "default";
    const deskId = (window.desktops && window.desktops.length > 0)
      ? window.desktops[0].id.toString()
      : (workspace.currentDesktop ? workspace.currentDesktop.id.toString() : "default_desk");
    return outName + "||" + deskId;
  } catch (e) {
    return "default||default_desk";
  }
}

const FLOATING_CLASSES_REGEX = /kcolorchooser|colorpicker|gcolor|eyedropper|spectacle|klipper|plasma\.clipboard|org\.kde\.kclock|org\.kde\.polkit|polkit|pinentry|zenity|kdialog|xdotool|portal|desktopdialog|plasmoidviewer|^raven_gui$|^raven-gui$|^raven config$/i;
const FLOATING_CAPTION_REGEX = /color picker|selector de color|mini player|mini-player|miniplayer|zuno widget|now playing widget|pip|quick view|raven control center|raven tiling emulator — control center/i;
const PIP_CAPTION_REGEX = /picture[- ]?in[- ]?picture|imagen[- ]en[- ]imagen|pantalla en pantalla|reproductor en miniatura|incrustation|bild[- ]in[- ]bild|imagem em imagem|immagine nell'immagine|^pip$/i;

function isManageable(w) {
  try {
    if (!w || w.deleted || !w.managed) return false;
    if (w.popupWindow || w.tooltip || w.onScreenDisplay || w.notification || w.specialWindow || w.splash || w.transientFor != null) return false;
    if (w.desktopWindow || w.dock || w.skipTaskbar || w.skipPager) return false;

    const strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
    if (strClass.indexOf("spectacle") !== -1 && w.fullScreen) return false;
    if (!w.normalWindow && !w.dialog && !w.utility) return false;
    if (w.transient || (w.dialog && w.transientFor != null)) return false;
    if (w.frameGeometry && (w.frameGeometry.width <= 0 || w.frameGeometry.height <= 0)) return false;

    return true;
  } catch (e) {
    return false;
  }
}

function isFloating(w) {
  try {
    if (!w || w.deleted || w.__raven_dynamic_float) return true;
    if (w.dialog || w.utility || w.specialWindow || w.modal || w.transient || w.transientFor != null) return true;
    if (w.fullScreen) return false;
    if (w.maximizeMode !== 0) return true;

    const strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
    const strCap = w.caption ? w.caption.toString().toLowerCase() : "";
    let isPip = PIP_CAPTION_REGEX.test(strCap);

    if (_window_rules && _window_rules.length > 0) {
      for (let i = 0; i < _window_rules.length; i++) {
        const rule = _window_rules[i];
        if (rule && rule.class && strClass.indexOf(rule.class.toLowerCase()) !== -1) {
          if (rule.pip) isPip = true;
          if (rule.action === "float") {
            if (isPip && !w.keepAbove) w.keepAbove = true;
            return true;
          }
        }
      }
    }

    if (isPip && !w.keepAbove) w.keepAbove = true;
    if (FLOATING_CLASSES_REGEX.test(strClass) || FLOATING_CAPTION_REGEX.test(strCap)) return true;

    const minS = w.minSize;
    const maxS = w.maxSize;
    if (minS && maxS && minS.width > 0 && minS.height > 0) {
      if (minS.width === maxS.width && minS.height === maxS.height) return true;
      if (maxS.width > 0 && maxS.height > 0 && maxS.width <= 500 && maxS.height <= 450) return true;
    }

    const fg = w.frameGeometry;
    if (fg && fg.width > 0 && fg.height > 0 && fg.width < 380 && fg.height < 320) return true;

    return Boolean(isPip);
  } catch (e) {
    return true;
  }
}

function isSameDesktop(w1, w2) {
  if (!w1.desktops || !w2.desktops || w1.desktops.length === 0 || w2.desktops.length === 0) return true;
  for (let i = 0; i < w1.desktops.length; i++) {
    if (w2.desktops.indexOf(w1.desktops[i]) !== -1) return true;
  }
  return false;
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

  bindWindow(w);
  syncWindowDelta(w);
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
 * @brief Solicita una sincronización global de estado agrupando eventos en 40ms.
 */
function requestStateSync() {
  try {
    if (!_debounceTimer) {
      _debounceTimer = new QTimer();
      _debounceTimer.interval = 40;
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
        const deskId = currentDesk ? currentDesk.id.toString() : "default_desk";
        screens[out.name + "||" + deskId] = getSafeScreenGeometry(out, currentDesk);
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

        case "move":
        case "rectify_window":
          if (w.minimized || w.interactiveMove || w.interactiveResize || w.fullScreen) break;
          if (w.transientChildren && w.transientChildren.length > 0) break;
          if (w.maximizeMode !== 0) {
            try { w.setMaximize(false, false); } catch (eMax) {}
          }

          const targetGeom = {
            x: Math.round(cmd.x),
            y: Math.round(cmd.y),
            width: Math.round(cmd.width),
            height: Math.round(cmd.height),
          };

          const curFg = w.frameGeometry;
          if (cmd.action === "move" && curFg &&
              Math.round(curFg.x) === targetGeom.x &&
              Math.round(curFg.y) === targetGeom.y &&
              Math.round(curFg.width) === targetGeom.width &&
              Math.round(curFg.height) === targetGeom.height) {
            break;
          }

          w.__raven_mutating = true;
          w.frameGeometry = targetGeom;

          // Auditoría de geometría aplicada
          try {
            const fgApplied = w.frameGeometry || targetGeom;
            callDBus(
              "org.kde.raven.Daemon",
              "/Events",
              "org.kde.raven.Events",
              "commandAppliedState",
              cmd.window_id,
              Math.round(fgApplied.x),
              Math.round(fgApplied.y),
              Math.round(fgApplied.width),
              Math.round(fgApplied.height)
            );
          } catch (eAudit) {}

          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) cw.__raven_mutating = false;
            }, 60);
          })(w);
          break;

        case "focus":
          if (workspace.activeWindow !== w && (!workspace.activeWindow || !workspace.activeWindow.popupWindow)) {
            workspace.activeWindow = w;
          }
          break;

        case "minimize":
          w.__raven_mutating = true;
          w.minimized = true;
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 60);
          })(w);
          break;

        case "unminimize":
          w.__raven_mutating = true;
          w.minimized = false;
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 60);
          })(w);
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

        case "release_quarantine":
        case "request_feedback":
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

    if (w.captionChanged !== undefined) {
      w.captionChanged.connect(function () {
        if (w && !w.deleted && !w.__raven_mutating) {
          const cap = w.caption ? w.caption.toString().toLowerCase() : "";
          if (PIP_CAPTION_REGEX.test(cap) || FLOATING_CAPTION_REGEX.test(cap)) requestStateSync();
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
      if (w.transientChildren && w.transientChildren.length > 0) return;

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
    f: isFloating(w),
    m: Boolean(w.minimized),
    p: false,
    x: geom.x,
    y: geom.y,
    w: geom.w,
    h: geom.h,
    min_w: w.minSize ? Math.round(w.minSize.width) : 0,
    min_h: w.minSize ? Math.round(w.minSize.height) : 0,
    sb: false,
    iq: false,
    fs: Boolean(w.fullScreen),
    sus: false,
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
 * @brief Registra los atajos de teclado globales en el gestor de accesos directos de KWin.
 */
function initShortcuts() {
  registerRavenShortcuts();
}

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

  // 2. Enlazar ventanas existentes al puente (sin disparar syncs masivos)
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
    requestStateSync();
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
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "getWindowRules",
        function (res) {
          try {
            if (res) {
              _window_rules = JSON.parse(res);
            }
          } catch (e) { }
        }
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
  initShortcuts();
  initDBusBridge();
  Logger.info("Main", "Puente inicializado exitosamente");
} catch (e) {
  Logger.error("Main", "Error crítico al inicializar el puente", e);
}
