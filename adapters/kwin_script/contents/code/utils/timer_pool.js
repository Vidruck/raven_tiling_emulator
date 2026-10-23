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

/** @type {Array<{at: number, callback: Function}>} Cola de callbacks pendientes para ejecución diferida */
var _deferred_queue = [];
var _queue_timer = null;

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
          var cb = s.callback;
          s.callback = null;
          try { if (cb) cb(); } catch (e) { }
          processDeferredQueue();
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

function processDeferredQueue() {
  if (!_timer_pool_ready || _deferred_queue.length === 0) return;
  var now = Date.now();
  for (var i = 0; i < _deferred_queue.length; i++) {
    var item = _deferred_queue[i];
    if (item.at <= now) {
      _deferred_queue.splice(i, 1);
      i--;
      try { item.callback(); } catch (e) {}
    } else {
      var remaining = Math.max(0, item.at - now);
      for (var s = 0; s < _timer_pool.length; s++) {
        if (!_timer_pool[s].busy) {
          _deferred_queue.splice(i, 1);
          i--;
          _timer_pool[s].busy = true;
          _timer_pool[s].callback = item.callback;
          _timer_pool[s].timer.interval = remaining;
          _timer_pool[s].timer.start();
          break;
        }
      }
    }
  }
}

/**
 * @brief Ejecuta un callback después de un retardo temporal en milisegundos.
 *
 * Utiliza un slot preasignado del pool de temporizadores. Si el pool está temporalmente saturado,
 * agenda el callback en una cola de vencimiento procesada por un timer maestro persistente,
 * eliminando fugas de memoria y allocations continuas en QJSEngine.
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

  // Si los slots fijos están saturados, agendar en la cola persistente sin instanciar QTimers huérfanos
  _deferred_queue.push({ at: Date.now() + delayMs, callback: callback });
  if (!_queue_timer) {
    _queue_timer = new QTimer();
    _queue_timer.interval = 16;
    _queue_timer.timeout.connect(function () {
      processDeferredQueue();
      if (_deferred_queue.length === 0 && _queue_timer && _queue_timer.active) {
        _queue_timer.stop();
      }
    });
  }
  if (!_queue_timer.active) {
    _queue_timer.start();
  }
  return null;
}

