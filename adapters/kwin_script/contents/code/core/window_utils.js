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

