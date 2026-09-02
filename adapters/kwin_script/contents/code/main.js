/**
 * @fileoverview Modulo de Logger para el puente Raven en KWin (Plasma 6).
 */

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
/**
 * @fileoverview Pool estático de timers reutilizables para minimizar la carga del Garbage Collector en QJSEngine.
 */

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

/**
 * Ejecuta un callback después de un retardo de tiempo especificado en milisegundos,
 * reutilizando un QTimer estático del pool si está disponible.
 *
 * @param {Function} callback - Función que se ejecutará al vencer el temporizador.
 * @param {number} delayMs - Tiempo de espera en milisegundos.
 * @returns {Object|null} El slot de timer asignado o null en caso de fallo.
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

  // Fallback a asignación dinámica
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
 * @fileoverview Funciones auxiliares de geometría y cálculo de coordenadas para KWin.
 */

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
 * @fileoverview Evaluaciones y utilidades de estado de ventanas KWin.
 */

// Lista de clases de ventana base (Gecko / CSD conocidas) inamovibles.
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

// Lista activa fusionada y deduplicada de clases en cuarentena.
var _quarantine_classes = HARDCODED_QUARANTINE_BASE.slice();

// Lista de reglas de ventanas enviadas desde la UI.
var _window_rules = [];

/**
 * Fusiona la lista base de cuarentena con las personalizaciones enviadas desde la UI.
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
// Expresión regular para clases de aplicaciones que son inherentemente flotantes/herramientas
// Nota: Para Raven, únicamente se filtra la GUI de configuración del emulador (raven_gui / raven config / control center)
const FLOATING_CLASSES_REGEX = /kcolorchooser|colorpicker|gcolor|eyedropper|spectacle|klipper|plasma\.clipboard|org\.kde\.kclock|org\.kde\.polkit|polkit|pinentry|zenity|kdialog|xdotool|portal|desktopdialog|plasmoidviewer|^raven_gui$|^raven-gui$|^raven config$/i;

// Expresiones regulares para widgets auxiliares o mini-reproductores por caption
const FLOATING_CAPTION_REGEX = /color picker|selector de color|mini player|mini-player|miniplayer|zuno widget|now playing widget|pip|quick view|raven control center|raven tiling emulator — control center/i;

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

// Expresión regular para detectar títulos Picture-in-Picture en múltiples idiomas
const PIP_CAPTION_REGEX = /picture[- ]?in[- ]?picture|imagen[- ]en[- ]imagen|pantalla en pantalla|reproductor en miniatura|incrustation|bild[- ]in[- ]bild|imagem em imagem|immagine nell'immagine|^pip$/i;

/**
 * Determina si una ventana debe comportarse como flotante (floating).
 *
 * @param {KWin::Window} w - Objeto de ventana de KWin.
 * @returns {boolean} Verdadero si es flotante; de lo contrario, falso.
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
 * Determina si dos ventanas comparten al menos un escritorio virtual activo.
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
 * @fileoverview Lógica de estabilización CSD para ventanas de arranque asíncrono.
 */

/**
 * Procesa el ingreso de una nueva ventana evaluando si requiere estabilización temporal CSD.
 *
 * @param {KWin::Window} w - Ventana que se está añadiendo.
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
 * @fileoverview Resalte visual con el sistema de Outline de KWin.
 */

/**
 * Destaca visualmente una ventana usando el Outline de KWin.
 *
 * @param {KWin::Window} w - Ventana a destacar.
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
 * @fileoverview Servicios de comunicación D-Bus para sincronización entre KWin y el demonio Rust.
 */

var _debounceTimer = null;

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
      syncState();
    } catch (err) { }
  }
}

/**
 * Sincroniza el estado completo del compositor enviándolo al demonio (daemon) de Rust vía D-Bus.
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
 * Sincroniza de forma incremental el cambio de geometría o estado (delta sync) de una única ventana.
 *
 * @param {KWin::Window} w - Objeto de ventana modificado.
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
      const outputs = workspace.screens || [];
      for (let i = 0; i < outputs.length; i++) {
        if (outputs[i].name === target_output_name) {
          workspace.sendClientToScreen(win, outputs[i]);
          break;
        }
      }
    }
    if (target_desktop_id) {
      const desktops = workspace.desktops || [];
      for (let j = 0; j < desktops.length; j++) {
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
              w.__raven_dynamic_float = Boolean(cmd.floating);
              w.keepAbove = Boolean(cmd.keep_above);
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
 * @fileoverview Registro e inyección de accesos directos globales nativos de KWin para Raven.
 */

/**
 * Registra los atajos globales nativos de KWin para controlar a Raven.
 * Expone las acciones del gestor de ventanas al panel de preferencias del sistema.
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
  registerShortcut("RavenToggleFloating", "Raven: Alternar Ventana Flotante Dinámica (Quick Peek)", "Meta+Shift+F", function () {
    var aw = workspace.activeWindow;
    var awId = aw ? getSafeWindowId(aw) : "";
    dispatchToRavenArg("toggleFloating", awId);
  });
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
  registerShortcut("RavenCycleLayout", "Raven: Ciclar Layout", "Meta+Shift+L", function() {
    dispatchToRaven("cycleLayout");
    if (workspace.activeWindow) highlightWindow(workspace.activeWindow);
  });
  registerShortcut("RavenMigrateDesktop", "Raven: Enviar a Escritorio Siguiente", "Meta+Shift+Right", function () {
    dispatchToRaven("migrateActiveToDesktop");
  });
  registerShortcut("RavenMigratePrevDesktop", "Raven: Enviar a Escritorio Anterior", "Meta+Shift+Left", function () {
    dispatchToRaven("migrateActiveToPrevDesktop");
  });

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
 * @fileoverview Punto de entrada base (Plantilla) para el puente Raven (Raven Bridge) en KDE Plasma 6.
 * Este archivo agrupa y carga los submódulos modulares a través de compilación por node o despliegue.
 */


/**
 * Inicializa y registra los atajos de teclado globales de Raven en KWin.
 */
function initShortcuts() {
  registerRavenShortcuts();
}

/**
 * Inicializa el puente D-Bus completo, incluyendo:
 * - Pool de timers estáticos
 * - Clases de cuarentena del daemon
 * - Reglas de ventanas del daemon
 * - Hooks de ciclo de vida de ventanas (windowAdded, windowRemoved, etc.)
 * - Seguimiento de ventana activa
 * - Notificación de bridge listo al daemon Rust
 * - Primera sincronización completa
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
    var awId = aw ? getSafeWindowId(aw) : "";
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "windowActivated",
        awId || ""
      );
    } catch (e) { }
  });

  workspace.currentDesktopChanged.connect(function () {
    requestStateSync();
  });

  // 4. Solicitar configuración del daemon y notificar arranque de forma diferida (50ms)
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

// Registro inicial de ciclo de vida
try {
  Logger.info("Main", "Inicializando el puente de Raven Tiling Emulator v3.0");
  initShortcuts();
  initDBusBridge();
  Logger.info("Main", "Puente inicializado exitosamente");
} catch (e) {
  Logger.error("Main", "Error crítico al inicializar el puente", e);
}
