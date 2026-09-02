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
