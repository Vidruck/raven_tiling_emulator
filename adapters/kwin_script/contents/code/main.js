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
 * @version 3.4
 */

/** @type {string[]} Lista inmutable de clases base de navegadores Gecko y aplicaciones CSD que requieren estabilización. */
var HARDCODED_QUARANTINE_BASE = [
  "firefox",
  "zen",
  "floorp",
  "waterfox",
  "librewolf",
  "tor-browser",
  "gecko",
  "chrome",
  "chromium",
  "brave",
  "electron",
  "code",
  "spotify",
  "intellij",
  "java"
];

/** @type {string[]} Lista combinada y activa de clases en periodo de cuarentena. */
var _quarantine_classes = HARDCODED_QUARANTINE_BASE.slice();

/** @type {Array<{class: string, action: string, pip?: boolean}>} Reglas de comportamiento personalizadas recibidas del demonio Rust. */
var _window_rules = [];

/**
 * @brief Fusiona la lista base de cuarentena con las reglas personalizadas provistas por la configuración de usuario.
 * @param {string} res JSON serializado con la lista de clases adicionales.
 */
function updateQuarantineClasses(res) {
  if (!res) return;
  try {
    var userList = JSON.parse(res);
    if (!Array.isArray(userList)) return;
    
    var map = {};
    for (var i = 0; i < HARDCODED_QUARANTINE_BASE.length; i++) {
      map[HARDCODED_QUARANTINE_BASE[i]] = true;
    }
    for (var j = 0; j < userList.length; j++) {
      if (userList[j]) {
        map[userList[j].toString().toLowerCase().trim()] = true;
      }
    }
    
    var merged = [];
    for (var key in map) {
      merged.push(key);
    }
    _quarantine_classes = merged;
  } catch (e) {
    Logger.error("updateQuarantineClasses", "Error fusionando clases de cuarentena", e);
  }
}

/**
 * @brief Obtiene de forma segura el identificador único (internalId) de una ventana.
 * @param {KWin::Window} w Instancia de la ventana de KWin.
 * @returns {string|null} Identificador textual único o null si no es válida.
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
 * @brief Obtiene el identificador compuesto del área de trabajo (workspace ID) para una ventana.
 *
 * Formato: `"nombre_salida||id_escritorio"` (ej. `"DP-1||1"`).
 *
 * @param {KWin::Window} window Instancia de la ventana.
 * @returns {string} Identificador único del espacio de trabajo.
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

/** @type {RegExp} Expresión regular para clases de aplicaciones que deben mantenerse siempre flotantes (herramientas, selectores, GUI de Raven). */
const FLOATING_CLASSES_REGEX = /kcolorchooser|colorpicker|gcolor|eyedropper|spectacle|klipper|plasma\.clipboard|org\.kde\.kclock|org\.kde\.polkit|polkit|pinentry|zenity|kdialog|xdotool|portal|desktopdialog|plasmoidviewer|^raven_gui$|^raven-gui$|^raven config$/i;

/** @type {RegExp} Expresión regular para títulos descriptivos de mini-widgets y selectores auxiliares. */
const FLOATING_CAPTION_REGEX = /color picker|selector de color|mini player|mini-player|miniplayer|zuno widget|now playing widget|pip|quick view|raven control center|raven tiling emulator — control center/i;

/**
 * @brief Evalúa si una ventana debe ser administrada por el ciclo de vida de Raven.
 *
 * Excluye paneles, tooltips, notificaciones, menús emergentes (popups) y ventanas de escritorio.
 *
 * @param {KWin::Window} w Instancia de la ventana.
 * @returns {boolean} true si la ventana es apta para ser gestionada.
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

/** @type {RegExp} Expresión regular multilingüe para detectar reproductores flotantes Picture-in-Picture (PiP). */
const PIP_CAPTION_REGEX = /picture[- ]?in[- ]?picture|imagen[- ]en[- ]imagen|pantalla en pantalla|reproductor en miniatura|incrustation|bild[- ]in[- ]bild|imagem em imagem|immagine nell'immagine|^pip$/i;

/**
 * @brief Evalúa si una ventana debe flotar libremente sin someterse a la división en mosaico.
 *
 * Aplica un análisis de 6 fases:
 * 1. Tipo nativo (diálogos modales, utilidades transitorias).
 * 2. Pantalla completa nativa (delegada a control espacial de pantalla completa).
 * 3. Detección y anclaje superior de Picture-in-Picture (PiP).
 * 4. Filtrado por lista de exclusión (FLOATING_CLASSES_REGEX).
 * 5. Heurística de tamaño fijo (minSize === maxSize).
 * 6. Micro-dimensiones (< 380x320 px) para widgets dedicados.
 *
 * @param {KWin::Window} w Instancia de la ventana.
 * @returns {boolean} true si debe flotar libremente.
 */
function isFloating(w) {
  try {
    if (!w || w.deleted) return true;
    if (w.__raven_dynamic_float) return true;

    // 1. Tipos de ventana nativos de Wayland / X11 auxiliares o transitorios
    if (w.dialog || w.utility || w.specialWindow || w.modal || w.transientFor) return true;

    // 2. Fullscreen nativo (YouTube, juegos, etc.) NO es flotante:
    // se envía como fs=true al motor Rust que le asigna pantalla completa.
    if (w.fullScreen) return false;

    if (w.maximizeMode !== 0 || w.maximized) return true;

    const strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
    const strCap = w.caption ? w.caption.toString().toLowerCase() : "";

    let isPip = PIP_CAPTION_REGEX.test(strCap);

    // 3. Evaluamos reglas dinámicas enviadas desde la interfaz de usuario
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

    if (isPip && !w.keepAbove) {
      w.keepAbove = true;
    }

    // 4. Filtrado por clase o título de herramientas conocidas
    if (FLOATING_CLASSES_REGEX.test(strClass) || FLOATING_CAPTION_REGEX.test(strCap)) {
      return true;
    }

    const isVirtPopup = (strClass.indexOf("qemu") !== -1 || strClass.indexOf("virt-manager") !== -1) && !w.normalWindow;
    if (isVirtPopup) {
      return true;
    }

    // 5. Heurística de dimensiones fijas / restringidas (ej: widgets de Zuno, micro-selectores)
    const minS = w.minSize;
    const maxS = w.maxSize;
    if (minS && maxS && minS.width > 0 && minS.height > 0) {
      // Ventana de tamaño completamente rígido (no redimensionable)
      if (minS.width === maxS.width && minS.height === maxS.height) {
        return true;
      }
      // Rango extremadamente estrecho o panel auxiliar
      if (maxS.width > 0 && maxS.height > 0 && maxS.width <= 500 && maxS.height <= 450) {
        return true;
      }
    }

    // 6. Heurística geométrica de micro-ventanas / popups flotantes sin título o dimensiones mínimas
    const fg = w.frameGeometry;
    const wWidth = fg ? fg.width : 0;
    const wHeight = fg ? fg.height : 0;
    if (wWidth > 0 && wHeight > 0) {
      // Ventanas diminutas creadas como mini-widgets flotantes (ej. Zuno mini-player, color pickers sin clase específica)
      if (wWidth < 380 && wHeight < 320) {
        return true;
      }

      // Popups flotantes de aplicaciones complejas (JetBrains, Zen, Firefox) sin título
      if ((strClass.indexOf("jetbrains") !== -1 || strClass.indexOf("idea") !== -1 || strClass.indexOf("zen") !== -1 || strClass.indexOf("firefox") !== -1) &&
          (!strCap || strCap.trim() === "" || strCap === "win0") && (wWidth < 450 && wHeight < 350)) {
        return true;
      }
    }

    return Boolean(isPip);
  } catch (e) {
    return true;
  }
}

/**
 * @brief Determina si dos ventanas coexisten en el mismo escritorio virtual.
 * @param {KWin::Window} w1 Primera ventana.
 * @param {KWin::Window} w2 Segunda ventana.
 * @returns {boolean} true si comparten al menos un escritorio virtual.
 */
function isSameDesktop(w1, w2) {
  if (!w1.desktops || !w2.desktops || w1.desktops.length === 0 || w2.desktops.length === 0) {
    return true; // En Wayland/ventanas fijadas, asumir coincidencia
  }
  for (let i = 0; i < w1.desktops.length; i++) {
    if (w2.desktops.indexOf(w1.desktops[i]) !== -1) {
      return true;
    }
  }
  return false;
}
/**
 * @file quarantine.js
 * @brief Lógica de estabilización temporal (cuarentena CSD) para ventanas de arranque asíncrono en Wayland.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/**
 * @brief Evalúa y procesa la incorporación de una nueva ventana al sistema de mosaico.
 *
 * Determina si la ventana requiere un periodo de cuarentena temporal para estabilizar
 * sus dimensiones iniciales antes de ser empaquetada por el motor de cálculo:
 * - Ventanas sin `resourceClass` definido en su primer ciclo de vida (120 ms).
 * - Aplicaciones basadas en Gecko, Electron, JVM o CSD conocidas (80 ms).
 *
 * @param {KWin::Window} w Instancia de la ventana naciente.
 */
function processNewWindow(w) {
  if (!w || w.deleted || !isManageable(w)) {
    return;
  }

  const strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
  let needsQuarantine = (strClass === "");

  if (!needsQuarantine && _quarantine_classes) {
    for (let i = 0; i < _quarantine_classes.length; i++) {
      if (strClass.indexOf(_quarantine_classes[i]) !== -1) {
        needsQuarantine = true;
        break;
      }
    }
  }

  bindWindow(w);

  if (needsQuarantine) {
    w.__raven_quarantined = true;
    const durationMs = (strClass === "") ? 120 : 80;

    w.__raven_stab_timer = setKWinTimeout(() => {
      if (w && !w.deleted) {
        w.__raven_quarantined = false;
        w.__raven_strict_birth = true;
        w.__raven_stab_timer = null;
        requestStateSync();
      }
    }, durationMs);
  } else {
    requestStateSync();
  }
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
 * @brief Orquestador de comunicación D-Bus bidireccional entre el compositor KWin y el demonio Rust.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/** @type {QTimer|null} Temporizador de anti-rebote (debounce) para agrupar ráfagas de eventos de ventana. */
var _debounceTimer = null;

/**
 * @brief Solicita una sincronización global de estado con amortiguación temporal (debouncing de 50 ms).
 *
 * Agrupa múltiples eventos simultáneos de KWin (ej. cambio de escritorio + foco) en un único payload D-Bus.
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
      syncState();
    } catch (err) { }
  }
}

/**
 * @brief Extrae la topología completa de pantallas, escritorios y ventanas activas y la despacha al demonio Rust vía D-Bus.
 *
 * Invoca el método `syncStateAndUpdateLayout` en `org.kde.raven.Events` y aplica de inmediato los comandos geométricos devueltos.
 */
function syncState() {
  const windows = workspace.windowList();
  const winState = [];
  const screens = {};

  const outs = workspace.screens || [];
  const desks = workspace.desktops || [];
  const currentDesk = workspace.currentDesktop;

  try {
    for (let o = 0; o < outs.length; o++) {
      const output = outs[o];
      const outName = output ? output.name : "default";

      if (desks && desks.length > 0) {
        for (let d = 0; d < desks.length; d++) {
          const desktop = desks[d];
          const deskId = desktop ? desktop.id.toString() : "default_desk";
          const wsId = outName + "||" + deskId;
          screens[wsId] = getSafeScreenGeometry(output, desktop);
        }
      } else {
        const deskId = currentDesk ? currentDesk.id.toString() : "default_desk";
        const wsId = outName + "||" + deskId;
        screens[wsId] = getSafeScreenGeometry(output, currentDesk);
      }
    }
  } catch (e) {
    Logger.error("syncState", "Error iterando topología de pantallas", e);
  }

  for (let i = 0; i < windows.length; i++) {
    const w = windows[i];
    try {
      if (!isManageable(w) || w.__raven_quarantined) {
        continue;
      }
      const safeId = getSafeWindowId(w);
      if (!safeId) {
        continue;
      }

      const output = w.output || workspace.activeOutput;
      const outName = output ? output.name : "default";

      const deskIds = [];
      if (w.desktops) {
        for (let d = 0; d < w.desktops.length; d++) {
          deskIds.push(w.desktops[d].id.toString());
        }
      }

      const strCap = w.caption ? w.caption.toString() : "";
      const strClass = w.resourceClass ? w.resourceClass.toString() : "";
      const geom = getRectGeometry(w.frameGeometry);

      winState.push({
        id: safeId,
        desktops: deskIds,
        output: outName,
        f: isFloating(w),
        m: Boolean(w.minimized),
        p: false,
        x: geom.x,
        y: geom.y,
        w: geom.w,
        h: geom.h,
        min_w: w.minSize ? Math.round(w.minSize.width) : 0,
        min_h: w.minSize ? Math.round(w.minSize.height) : 0,
        sb: Boolean(w.__raven_strict_birth),
        iq: Boolean(w.__raven_quarantined),
        fs: Boolean(w.fullScreen),
        cls: strClass,
        cap: strCap,
      });
    } catch (e) {
      Logger.error("syncState", "Error extrayendo geometría/estado de ventana", e);
    }
  }

  const masterOutputs = [];
  for (let o = 0; o < outs.length; o++) {
    if (outs[o] && outs[o].name) {
      masterOutputs.push(outs[o].name.toString());
    }
  }

  const masterDesktops = [];
  for (let d = 0; d < desks.length; d++) {
    if (desks[d] && desks[d].id) {
      masterDesktops.push(desks[d].id.toString());
    }
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
 * @brief Sincroniza de forma incremental el cambio de geometría o estado (delta sync) de una única ventana.
 *
 * Utilizado tras el redimensionamiento o movimiento manual de una ventana por el usuario para actualizar el modelo espacial de Rust.
 *
 * @param {KWin::Window} w Instancia de la ventana modificada.
 */
function syncWindowDelta(w) {
  try {
    if (!w || w.deleted || !isManageable(w) || w.__raven_quarantined) {
      return;
    }

    const safeId = getSafeWindowId(w);
    if (!safeId) {
      return;
    }

    const geom = getRectGeometry(w.frameGeometry);
    const deskIds = [];
    if (w.desktops) {
      for (let d = 0; d < w.desktops.length; d++) {
        deskIds.push(w.desktops[d].id.toString());
      }
    }

    const deltaPayload = {
      id: safeId,
      desktops: deskIds,
      output: w.output ? w.output.name : "default",
      f: isFloating(w),
      m: Boolean(w.minimized),
      p: false,
      x: geom.x,
      y: geom.y,
      w: geom.w,
      h: geom.h,
      min_w: w.minSize ? Math.round(w.minSize.width) : 0,
      min_h: w.minSize ? Math.round(w.minSize.height) : 0,
      sb: Boolean(w.__raven_strict_birth),
      iq: Boolean(w.__raven_quarantined),
      fs: Boolean(w.fullScreen),
      cls: w.resourceClass ? w.resourceClass.toString() : "",
      cap: w.caption ? w.caption.toString() : "",
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
 * @brief Migra nativamente una ventana a un monitor o escritorio virtual especificado.
 *
 * @param {KWin::Window} win Instancia de la ventana a desplazar.
 * @param {string|null} target_output_name Nombre del monitor destino o null si permanece en el mismo.
 * @param {string|null} target_desktop_id Identificador del escritorio virtual destino o null.
 */
function migrateWindow(win, target_output_name, target_desktop_id) {
  if (!win || win.deleted) {
    return;
  }
  try {
    if (target_output_name) {
      const outputs = workspace.screens || [];
      for (let i = 0; i < outputs.length; i++) {
        const out = outputs[i];
        if (out && out.name === target_output_name) {
          try {
            if (typeof workspace.sendClientToScreen === "function") {
              workspace.sendClientToScreen(win, out);
            }
          } catch (errScreen) {
            Logger.debug("migrateWindow", "workspace.sendClientToScreen fallback: " + errScreen);
          }

          try {
            if (out.geometry && win.frameGeometry) {
              const geom = out.geometry;
              const curW = win.frameGeometry.width || 800;
              const curH = win.frameGeometry.height || 600;
              const newX = geom.x + Math.max(0, Math.floor((geom.width - curW) / 2));
              const newY = geom.y + Math.max(0, Math.floor((geom.height - curH) / 2));
              win.frameGeometry = {
                x: newX,
                y: newY,
                width: Math.min(curW, geom.width),
                height: Math.min(curH, geom.height)
              };
            }
          } catch (errGeom) {
            Logger.debug("migrateWindow", "win.frameGeometry relocation fallback: " + errGeom);
          }

          try {
            win.output = out;
          } catch (errOut) {}
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
  } catch (e) {
    Logger.error("migrateWindow", "Fallo al migrar ventana", e);
  }
}

/**
 * @brief Procesa y aplica en KWin la lista atómica de comandos JSON calculados por el demonio Rust.
 *
 * Ejecuta en dos pasadas:
 * 1. Mutaciones de estado y flotación (`set_floating`, `keepAbove`).
 * 2. Transformaciones geométricas (`move`, `focus`, `minimize`, `unminimize`, `migrate_to_output`).
 * Incorpora banderas `__raven_mutating` para suprimir ciclos recursivos de feedback con KWin.
 *
 * @param {string} commandsJson Cadena JSON con el vector de RavenAction devuelto por el demonio.
 */
function applyCommands(commandsJson) {
  if (!commandsJson) {
    return;
  }
  try {
    const cmds = JSON.parse(commandsJson);
    const windows = workspace.windowList();

    // Pasada 1: Comandos de mutación de estado/flags (set_floating, etc.)
    for (let i = 0; i < cmds.length; i++) {
      const cmd = cmds[i];
      if (cmd.action === "set_floating") {
        for (let j = 0; j < windows.length; j++) {
          const w = windows[j];
          if (getSafeWindowId(w) === cmd.window_id) {
            try {
              const wasFloating = Boolean(w.__raven_dynamic_float);
              w.__raven_dynamic_float = Boolean(cmd.floating);
              w.keepAbove = Boolean(cmd.keep_above);

              // Feedback visual táctil: si pasa a flotante, aplicar un ligero desajuste (nudge)
              if (cmd.floating && !wasFloating && !w.fullScreen && w.maximizeMode === 0) {
                w.__raven_mutating = true;
                const fg = w.frameGeometry;
                w.frameGeometry = {
                  x: fg.x + 24,
                  y: fg.y + 24,
                  width: Math.max(300, Math.round(fg.width * 0.95)),
                  height: Math.max(200, Math.round(fg.height * 0.95))
                };
                (function (cw) {
                  setKWinTimeout(function () {
                    if (cw && !cw.deleted) {
                      cw.__raven_mutating = false;
                    }
                  }, 150);
                })(w);
              }
            } catch (e) {
              Logger.error("applyCommands", "Error asignando estado flotante dinámico", e);
            }
            break;
          }
        }
      }
    }

    // Pasada 2: Comandos de posicionamiento, foco y migración
    for (let i = 0; i < cmds.length; i++) {
      const cmd = cmds[i];
      if (cmd.action === "set_floating") {
        continue;
      }
      if (cmd.action === "request_sync") {
        requestStateSync();
        continue;
      }

      if (cmd.action === "saturation_warning") {
        Logger.info("Raven UI", "Saturation warning received from core");
        try {
          if (workspace.showOutline) {
            const activeWin = workspace.activeWindow;
            if (activeWin) {
              workspace.showOutline(activeWin.frameGeometry);
            } else {
              const ca = workspace.clientArea(0, 0, workspace.currentDesktop);
              workspace.showOutline({
                x: ca.x, y: ca.y, width: ca.width, height: ca.height
              });
            }
            setKWinTimeout(function() {
              if (workspace.hideOutline) workspace.hideOutline();
            }, 300);
          }
        } catch(e) {}
        continue;
      }

      for (let j = 0; j < windows.length; j++) {
        const w = windows[j];
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

              if (w.maximizeMode !== 0 || w.fullScreen) {
                break;
              }

              w.__raven_mutating = true;
              const targetGeom = {
                x: Math.round(cmd.x),
                y: Math.round(cmd.y),
                width: Math.round(cmd.width),
                height: Math.round(cmd.height),
              };

              w.frameGeometry = targetGeom;

              (function (capturedWindow) {
                setKWinTimeout(function () {
                  if (capturedWindow && !capturedWindow.deleted) {
                    capturedWindow.__raven_mutating = false;
                  }
                }, 120);
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
          } else if (cmd.action === "saturation_warning") {
            Logger.warn("Saturation", "Pantalla cerca de saturación: " + cmd.active + "/" + cmd.cmax + " ventanas");
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
 * @brief Enlaza los eventos y señales reactivas del ciclo de vida de una ventana con el puente de Raven.
 *
 * Suscribe escuchadores para:
 * - Minimización y restauración (`minimizedChanged`).
 * - Maximización y desmaximización (`maximizedChanged`).
 * - Pantalla completa (`fullScreenChanged`).
 * - Cambio de título (`captionChanged` para refresco PiP).
 * - Migración de pantalla o escritorio virtual (`outputChanged`, `desktopsChanged`).
 * - Modificación geométrica interactiva (`frameGeometryChanged`, `interactiveMoveResizeFinished`).
 *
 * @param {KWin::Window} w Instancia de la ventana a enlazar.
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

    if (w.fullScreenChanged !== undefined) {
      w.fullScreenChanged.connect(function () {
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
    }

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
      if (w.__raven_quarantined && w.__raven_stab_timer) {
        const t = w.__raven_stab_timer;
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
 * @file shortcuts.js
 * @brief Registro e integración de atajos de teclado globales nativos de KWin en KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/**
 * @brief Registra todos los atajos de teclado globales de Raven en el subsistema de accesos rápidos de KWin.
 *
 * Expone las acciones del gestor de mosaico en la sección "KWin" del panel de Preferencias del Sistema.
 * Cada combinación despacha un método D-Bus hacia `org.kde.raven.Daemon` y ejecuta inmediatamente los comandos retornados.
 */
function registerRavenShortcuts() {
  /**
   * @brief Envía una acción sin parámetros al demonio de Raven vía D-Bus.
   * @param {string} actionStr Nombre del método D-Bus.
   */
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

  /**
   * @brief Envía una acción con un argumento al demonio de Raven vía D-Bus.
   * @param {string} actionStr Nombre del método D-Bus.
   * @param {number|string} arg Valor del parámetro.
   */
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

  // ── GESTIÓN DE ESTADO Y FLOTACIÓN ──
  registerShortcut("RavenToggleTiling", "Raven: Alternar Mosaico (On/Off)", "Meta+Space", function () {
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "toggleTiling",
        function (response) {
          if (response && response !== "[]") {
            applyCommands(response);
          }
          // Verificar estado resultante para coordinar el comportamiento del puente
          callDBus(
            "org.kde.raven.Daemon",
            "/Events",
            "org.kde.raven.Events",
            "getTilingState",
            function (stateRes) {
              var isEnabled = (stateRes === "true" || stateRes === true);
              if (isEnabled) {
                // Modo activado: Sincronización forzada completa ("patada del puente a Rust")
                syncState();
              } else {
                // Modo desactivado: Desacomodo sutil (offset en cascada) para feedback visual claro
                var winList = workspace.windowList ? workspace.windowList() : [];
                var activeOut = workspace.activeOutput;
                var offset = 18;
                for (var i = 0; i < winList.length; i++) {
                  var w = winList[i];
                  if (w && !w.deleted && isManageable(w) && !w.minimized && !w.fullScreen && w.maximizeMode === 0) {
                    if (!activeOut || !w.output || w.output.name === activeOut.name) {
                      try {
                        w.__raven_mutating = true;
                        var geom = w.frameGeometry;
                        w.frameGeometry = {
                          x: geom.x + offset,
                          y: geom.y + offset,
                          width: geom.width,
                          height: geom.height
                        };
                        offset += 14;
                        (function (cw) {
                          setKWinTimeout(function () {
                            if (cw && !cw.deleted) {
                              cw.__raven_mutating = false;
                            }
                          }, 150);
                        })(w);
                      } catch (errGeom) {}
                    }
                  }
                }
              }
            }
          );
        }
      );
    } catch (e) {
      Logger.error("Shortcuts", "Fallo en RavenToggleTiling: " + e);
    }
  });
  registerShortcut("RavenToggleFloating", "Raven: Alternar Ventana Flotante Dinámica (Quick Peek)", "Meta+Shift+F", function () {
    var aw = workspace.activeWindow;
    var awId = aw ? getSafeWindowId(aw) : "";
    dispatchToRavenArg("toggleFloating", awId);
  });

  // ── NAVEGACIÓN Y FOCO VISUAL ──
  registerShortcut("RavenFocusNext", "Raven: Siguiente Ventana", "Meta+J", function () {
    dispatchToRaven("focusNext");
  });
  registerShortcut("RavenFocusPrev", "Raven: Ventana Anterior", "Meta+K", function () {
    dispatchToRaven("focusPrev");
  });
  registerShortcut("RavenFocusLeft", "Raven: Foco Izquierda", "Meta+Left", function () {
    dispatchToRaven("focusLeft");
  });
  registerShortcut("RavenFocusRight", "Raven: Foco Derecha", "Meta+Right", function () {
    dispatchToRaven("focusRight");
  });
  registerShortcut("RavenFocusUp", "Raven: Foco Arriba", "Meta+Up", function () {
    dispatchToRaven("focusUp");
  });
  registerShortcut("RavenFocusDown", "Raven: Foco Abajo", "Meta+Down", function () {
    dispatchToRaven("focusDown");
  });

  // ── INTERCAMBIO Y RATIOS DE COMPOSICIÓN ──
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

  // ── MIGRACIÓN ENTRE MONITORES Y ESCRITORIOS ──
  registerShortcut("RavenMigrateMonitor", "Raven: Enviar a Monitor Siguiente", "Meta+Shift+M", function () {
    dispatchToRaven("migrateActiveToScreen");
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

  // ── MÁRGENES, CAPACIDAD Y CICLADO DE ALGORITMOS ──
  registerShortcut("RavenIncrementGaps", "Raven: Incrementar Gaps", "Meta+=", function () {
    dispatchToRavenArg("incrementGaps", 2);
  });
  registerShortcut("RavenDecrementGaps", "Raven: Decrementar Gaps", "Meta+-", function () {
    dispatchToRavenArg("incrementGaps", -2);
  });
  registerShortcut("RavenIncrementMaster", "Raven: Incrementar Capacidad Master", "Meta+]", function () {
    dispatchToRaven("incrementMaster");
  });
  registerShortcut("RavenDecrementMaster", "Raven: Decrementar Capacidad Master", "Meta+[", function () {
    dispatchToRaven("decrementMaster");
  });
  registerShortcut("RavenCycleLayout", "Raven: Ciclar Algoritmo de Disposición", "Meta+Shift+L", function() {
    dispatchToRaven("cycleLayout");
    if (workspace.activeWindow) highlightWindow(workspace.activeWindow);
  });

  // ── REDIMENSIONAMIENTO FINO POR VENTANA ──
  registerShortcut("RavenResizeWidthInc", "Raven: Aumentar Ancho de Ventana", "Meta+Alt+Right", function () {
    dispatchToRaven("resize_width_inc");
  });
  registerShortcut("RavenResizeWidthDec", "Raven: Reducir Ancho de Ventana", "Meta+Alt+Left", function () {
    dispatchToRaven("resize_width_dec");
  });
  registerShortcut("RavenResizeHeightInc", "Raven: Aumentar Alto de Ventana", "Meta+Alt+Down", function () {
    dispatchToRaven("resize_height_inc");
  });
  registerShortcut("RavenResizeHeightDec", "Raven: Reducir Alto de Ventana", "Meta+Alt+Up", function () {
    dispatchToRaven("resize_height_dec");
  });
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

  workspace.activeWindowChanged.connect(function () {
    var aw = workspace.activeWindow;
    if (aw && !isManageable(aw)) {
      // Ignorar paneles, diálogos de escritorio y plasmoides para no perder el foco previo de apps
      return;
    }
    var awId = aw ? getSafeWindowId(aw) : "";
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
        "getQuarantineClasses",
        function (res) {
          updateQuarantineClasses(res);
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
