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
    if (!w || w.deleted || !w.managed) return false;
    if (w.popupWindow || w.tooltip || w.onScreenDisplay || w.notification || w.specialWindow || w.splash) return false;
    if (w.desktopWindow || w.dock || w.skipTaskbar || w.skipPager) return false;

    const strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
    const strName = w.resourceName ? w.resourceName.toString().toLowerCase() : "";
    const strCap = w.caption ? w.caption.toString().toLowerCase() : "";

    // Filtrar herramientas específicas del sistema solicitadas por el usuario
    if (strClass.indexOf("spectacle") !== -1 || strName.indexOf("spectacle") !== -1) return false;
    if (strClass.indexOf("plasma-welcome") !== -1 || strClass.indexOf("plasma.welcome") !== -1 ||
        strName.indexOf("plasma-welcome") !== -1 || strCap.indexOf("centro de bienvenida") !== -1 ||
        strCap.indexOf("welcome center") !== -1 || strCap.indexOf("bienvenida de plasma") !== -1) {
      return false;
    }
    if (strClass.indexOf("plasmashell") !== -1 || strClass.indexOf("systemsettings") !== -1 ||
        strClass.indexOf("kscreenlocker") !== -1 || strClass.indexOf("screenlocker") !== -1 ||
        strClass.indexOf("lockscreen") !== -1 || strClass.indexOf("plasmalogin") !== -1 ||
        strClass.indexOf("sddm") !== -1 || strClass.indexOf("kcm_") !== -1 ||
        strCap.indexOf("inicio de sesión") !== -1 || strCap.indexOf("bloqueo de pantalla") !== -1) {
      return false;
    }

    if (!w.normalWindow && !w.dialog && !w.utility) return false;
    if (w.transient || w.transientFor != null) return false;
    if (w.frameGeometry && (w.frameGeometry.width <= 0 || w.frameGeometry.height <= 0)) return false;

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
    return false;
  } catch (e) {
    return true;
  }
}
