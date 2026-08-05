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
    if (w.dialog || w.utility || w.specialWindow || w.modal || w.transientFor) return true;

    // Fullscreen nativo (YouTube, juegos, etc.) NO es flotante:
    // se envía como fs=true al motor Rust que le asigna pantalla completa.
    if (w.fullScreen) return false;

    if (w.maximizeMode !== 0 || w.maximized) return true;

    const strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
    const strCap = w.caption ? w.caption.toString().toLowerCase() : "";

    let isPip = PIP_CAPTION_REGEX.test(strCap) || w.keepAbove;

    // Evaluamos reglas dinámicas enviadas desde la interfaz de usuario
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

    const isRaven = strClass.indexOf("raven") !== -1 || strCap.indexOf("raven control center") !== -1;
    const isSpectacle = strClass.indexOf("spectacle") !== -1;
    const isKlipper = strClass.indexOf("klipper") !== -1 || strClass.indexOf("plasma.clipboard") !== -1;
    const isVirtPopup = (strClass.indexOf("qemu") !== -1 || strClass.indexOf("virt-manager") !== -1) && !w.normalWindow;

    // Popups flotantes de aplicaciones pesadas (JetBrains, Zen, Firefox) sin título
    let isHeavyAppPopup = false;
    if (strClass.indexOf("jetbrains") !== -1 || strClass.indexOf("idea") !== -1 || strClass.indexOf("zen") !== -1 || strClass.indexOf("firefox") !== -1) {
      if (!strCap || strCap.trim() === "" || strCap === "win0") {
        const fg = w.frameGeometry;
        const wWidth = fg ? fg.width : 0;
        const wHeight = fg ? fg.height : 0;
        if (wWidth > 0 && wHeight > 0 && wWidth < 450 && wHeight < 350) {
          isHeavyAppPopup = true;
        }
      }
    }

    return Boolean(isPip || isSpectacle || isKlipper || isVirtPopup || isRaven || isHeavyAppPopup);
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
