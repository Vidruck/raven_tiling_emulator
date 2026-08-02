/**
 * @fileoverview Lógica de cuarentena para ventanas CSD/Gecko al nacer.
 */

/**
 * Calcula el tiempo óptimo de cuarentena para una ventana según la heurística
 * de arranque en frío (Cold Start) o caliente (Warm Start).
 *
 * @param {string} strClass - Nombre en minúsculas de la clase de recurso de la ventana.
 * @returns {number} Duración del temporizador de estabilización en milisegundos.
 */
function calculateQuarantineDuration(strClass) {
  if (strClass === "") {
    return 150; // Sin clase asignada al nacer, dar más margen
  }

  let similarCount = 0;
  const allWindows = workspace.windowList();
  for (let k = 0; k < allWindows.length; k++) {
    const wc = allWindows[k].resourceClass ? allWindows[k].resourceClass.toString().toLowerCase() : "";
    if (wc === strClass) {
      similarCount++;
    }
  }

  return similarCount <= 1 ? 120 : 80; // 120ms Cold start, 80ms Warm start
}

/**
 * Procesa el ingreso de una nueva ventana evaluando si requiere entrar en cuarentena.
 *
 * @param {KWin::Window} w - Ventana que se está añadiendo.
 */
function processNewWindow(w) {
  if (!w || w.deleted || !isManageable(w)) {
    return;
  }

  const strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
  let needsQuarantine = false;

  if (strClass === "") {
    needsQuarantine = true;
  } else {
    for (let i = 0; i < _quarantine_classes.length; i++) {
      if (strClass.indexOf(_quarantine_classes[i]) !== -1) {
        needsQuarantine = true;
        break;
      }
    }
  }

  if (needsQuarantine) {
    w.__raven_quarantined = true;
    bindWindow(w);

    const quarantineDurationMs = calculateQuarantineDuration(strClass);

    w.__raven_stab_timer = setKWinTimeout(() => {
      if (w && !w.deleted) {
        w.__raven_quarantined = false;
        w.__raven_strict_birth = true;
        w.__raven_stab_timer = null;
        requestStateSync();
      }
    }, quarantineDurationMs);
  } else {
    bindWindow(w);
    requestStateSync();
  }
}
