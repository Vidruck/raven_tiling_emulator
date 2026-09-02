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
