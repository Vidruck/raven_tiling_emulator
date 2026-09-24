/**
 * @file quarantine.js
 * @brief Registro inicial de ventanas hacia el demonio Rust.
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.0 — Delegación completa a raven_backend_kwin
 *
 * La lógica de cuarentena, categorización, filtrado de flood y temporizadores de
 * estabilización (Cold/Warm start) ahora reside en Rust (`KWinQuarantineManager`).
 * El script de KWin simplemente vincula los listeners y despacha el delta de estado.
 */

/**
 * @brief Evalúa y procesa la incorporación de una nueva ventana al sistema de mosaico.
 *
 * @param {KWin::Window} w Instancia de la ventana naciente.
 */
function processNewWindow(w) {
  if (!w || w.deleted || !isManageable(w)) {
    return;
  }

  bindWindow(w);
  syncWindowDelta(w);
}

