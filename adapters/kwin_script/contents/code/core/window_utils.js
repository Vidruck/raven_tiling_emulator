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
