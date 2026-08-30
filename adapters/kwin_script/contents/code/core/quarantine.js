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
