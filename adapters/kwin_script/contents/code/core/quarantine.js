/**
 * @file quarantine.js
 * @brief Estabilización temporal (cuarentena CSD) para ventanas de arranque asíncrono en Wayland.
 * @author Alejandro González Hernández (Vidruck)
 * @version 4.0 — Modelo de 3 Timers en cascada
 *
 * ## Modelo de 3 Timers en Cascada
 *
 * **Timer-0 (KWin, este archivo)**: 60–140ms según categoría.
 *   - `__raven_kwin_stabilizing = true` → `frameGeometryChanged` suprimido.
 *   - Flood de señales → reinicia timer y marca `__raven_suspicious`.
 *   - Al expirar → limpia bandera, llama `requestStateSync()`.
 *
 * **Timer-1 (Rust Fase 1)**: 40–75ms.
 *   - Rust recibe ventana con `iq=true` → `schedule_release`.
 *   - Al expirar: libera cuarentena, `commit_layout`, manda `MoveWindow`.
 *
 * **Timer-2 (Rust Fase 2 — Sospecha Activa)**: 45–120ms.
 *   - Rust re-calcula layout y re-envía todos los comandos como `RectifyWindow`.
 *   - NO confía en geometría del bridge; fuente de verdad = árboles internos de Rust.
 */

/** @type {number} Señales de geometría en Timer-0 antes de considerar flood. */
var QUARANTINE_FLOOD_THRESHOLD = 4;

/** @type {number} Máximo de reinicios de Timer-0 por flood antes de proceder. */
var QUARANTINE_MAX_RESTARTS = 2;

/**
 * @brief Evalúa y procesa la incorporación de una nueva ventana al sistema de mosaico.
 *
 * Implementa el Timer-0 del modelo de 3 timers en cascada.
 *
 * @param {KWin::Window} w Instancia de la ventana naciente.
 */
function processNewWindow(w) {
  if (!w || w.deleted || !isManageable(w)) {
    return;
  }

  var strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
  var strName  = w.resourceName  ? w.resourceName.toString().toLowerCase()  : "";

  // Necesita cuarentena si: sin clase/nombre (siempre sospechosa) o en la lista activa.
  var needsQuarantine = (strClass === "" || strName === "");

  if (!needsQuarantine && _quarantine_classes) {
    for (var i = 0; i < _quarantine_classes.length; i++) {
      if (strClass.indexOf(_quarantine_classes[i]) !== -1 ||
          strName.indexOf(_quarantine_classes[i]) !== -1) {
        needsQuarantine = true;
        break;
      }
    }
  }

  bindWindow(w);

  if (needsQuarantine) {
    w.__raven_quarantined      = true;   // Rust Timer-1: suprimir sync hasta liberación
    w.__raven_strict_birth     = true;   // Rust Timer-1: indicador de nacimiento estricto
    w.__raven_kwin_stabilizing = true;   // KWin Timer-0: suprimir frameGeometryChanged
    w.__raven_stab_timer       = null;
    w.__raven_flood_count      = 0;      // Contador de señales durante Timer-0
    w.__raven_timer0_restarts  = 0;      // Reinicios de Timer-0 por flood

    // Ventanas sin clase/nombre son doblemente sospechosas para Rust
    if (strClass === "" || strName === "") {
      w.__raven_suspicious = true;
    }

    _scheduleKWinTimer0(w);
  } else {
    requestStateSync();
  }
}

/**
 * @brief Agenda el Timer-0 de KWin para una ventana en cuarentena.
 *
 * Al expirar: limpia `__raven_kwin_stabilizing` y llama `requestStateSync()`,
 * lo que dispara los Timers 1 y 2 de Rust en cascada.
 * Si hubo flood de señales durante el timer, se reinicia (máx. QUARANTINE_MAX_RESTARTS veces)
 * y la ventana queda marcada como `__raven_suspicious` para sospecha activa en Rust.
 *
 * @param {KWin::Window} w Instancia de la ventana.
 */
function _scheduleKWinTimer0(w) {
  if (!w || w.deleted) return;

  var delay = getKWinQuarantineDelay(w);

  setKWinTimeout(function () {
    if (!w || w.deleted) return;

    var floodCount = w.__raven_flood_count || 0;
    var restarts   = w.__raven_timer0_restarts || 0;

    if (floodCount >= QUARANTINE_FLOOD_THRESHOLD && restarts < QUARANTINE_MAX_RESTARTS) {
      // Flood: reiniciar Timer-0 y elevar sospecha
      w.__raven_flood_count     = 0;
      w.__raven_timer0_restarts = restarts + 1;
      w.__raven_suspicious      = true;

      Logger.warn(
        "quarantine",
        "[TIMER-0] Flood (" + floodCount + " señales) para '" +
        (w.resourceClass || "?") + "'/''" + (w.resourceName || "?") +
        "'. Reiniciando (restart #" + (restarts + 1) + ") → SOSPECHOSA."
      );

      _scheduleKWinTimer0(w);
      return;
    }

    // Timer-0 completado: liberar estabilización KWin y notificar a Rust
    w.__raven_kwin_stabilizing = false;
    w.__raven_flood_count      = 0;

    Logger.info(
      "quarantine",
      "[TIMER-0] OK → notificando Rust para '" +
      (w.resourceClass || "?") + "'/''" + (w.resourceName || "?") +
      "'" + (w.__raven_suspicious ? " [SOSPECHOSA]" : "")
    );

    requestStateSync();

  }, delay);
}
