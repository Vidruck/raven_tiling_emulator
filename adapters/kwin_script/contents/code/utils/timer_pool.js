/**
 * @file timer_pool.js
 * @brief Pool estático de temporizadores reutilizables para optimización del recolector de basura (GC) en QJSEngine.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/** @type {number} Capacidad máxima de temporizadores preasignados en el pool. */
var TIMER_POOL_SIZE = 10;

/** @type {Array<{timer: QTimer, busy: boolean, callback: Function|null}>} Lista de slots de temporizadores. */
var _timer_pool = [];

/** @type {boolean} Indica si el pool ha completado su inicialización. */
var _timer_pool_ready = false;

/**
 * @brief Inicializa el pool de temporizadores estáticos preasignados.
 *
 * Crea instancias persistentes de QTimer para evitar la creación y destrucción constante
 * de objetos durante eventos masivos de redimensionamiento o movimiento.
 * Debe invocarse una única vez durante el ciclo de arranque `initDBusBridge()`.
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
 * @brief Ejecuta un callback después de un retardo temporal en milisegundos.
 *
 * Reutiliza un slot libre de QTimer del pool estático. Si todos los slots están ocupados,
 * recurre a una asignación dinámica de reserva (*fallback*).
 *
 * @param {Function} callback Función a ejecutar al expirar el tiempo.
 * @param {number} delayMs Tiempo de espera en milisegundos.
 * @returns {Object|QTimer|null} Referencia al temporizador asignado o null en caso de fallo crítico.
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

  // Fallback a asignación dinámica si el pool se encuentra saturado
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
