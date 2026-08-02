/**
 * @fileoverview Pool estático de timers reutilizables para minimizar la carga del Garbage Collector en QJSEngine.
 */

var TIMER_POOL_SIZE = 10;
var _timer_pool = [];
var _timer_pool_ready = false;

/**
 * Inicializa el pool de timers estáticos preasignados.
 * Debe llamarse una sola vez durante init().
 */
function initTimerPool() {
  try {
    for (var i = 0; i < TIMER_POOL_SIZE; i++) {
      var t = new QTimer();
      t.singleShot = true;
      var slot = { timer: t, busy: false, callback: null };
      (function (s) {
        s.timer.timeout.connect(function () {
          s.busy = false;
          try { if (s.callback) s.callback(); } catch (e) { }
          s.callback = null;
        });
      })(slot);
      _timer_pool.push(slot);
    }
    _timer_pool_ready = true;
  } catch (e) {
    Logger.error("initTimerPool", "Error inicializando pool de timers", e);
    _timer_pool_ready = false;
  }
}

/**
 * Ejecuta un callback después de un retardo de tiempo especificado en milisegundos,
 * reutilizando un QTimer estático del pool si está disponible.
 *
 * @param {Function} callback - Función que se ejecutará al vencer el temporizador.
 * @param {number} delayMs - Tiempo de espera en milisegundos.
 * @returns {Object|null} El slot de timer asignado o null en caso de fallo.
 */
function setKWinTimeout(callback, delayMs) {
  if (_timer_pool_ready && _timer_pool.length > 0) {
    for (var i = 0; i < _timer_pool.length; i++) {
      var slot = _timer_pool[i];
      if (!slot.busy) {
        slot.busy = true;
        slot.callback = callback;
        slot.timer.interval = delayMs;
        slot.timer.start();
        return slot;
      }
    }
  }

  // Fallback a asignación dinámica
  try {
    var fallbackTimer = new QTimer();
    fallbackTimer.singleShot = true;
    fallbackTimer.interval = delayMs;
    fallbackTimer.timeout.connect(function () {
      try { callback(); } catch (e) { }
    });
    fallbackTimer.start();
    return fallbackTimer;
  } catch (e) {
    Logger.error("setKWinTimeout", "Error en fallback de temporizador", e);
    return null;
  }
}
