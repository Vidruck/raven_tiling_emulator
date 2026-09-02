/**
 * @file logger.js
 * @brief Módulo de registro estructurado y diagnóstico para el puente de KWin (Plasma 6).
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/**
 * @namespace Logger
 * @brief Objeto estático de registro de mensajes para el motor de scripts de KWin.
 *
 * Emite trazas estandarizadas visualizables mediante `journalctl -f | grep RAVEN`.
 */
var Logger = {
  /** @type {boolean} Bandera de depuración detallada. Desactivada por defecto para máxima eficiencia. */
  debug_enabled: false,

  /**
   * @brief Emite un mensaje informativo de operación estándar.
   * @param {string} ctx Contexto o nombre de la función emisora.
   * @param {string} msg Mensaje descriptivo.
   */
  info: function (ctx, msg) {
    print("[RAVEN] [INFO] [" + ctx + "] " + msg);
  },

  /**
   * @brief Emite una advertencia de anomalía recuperable o estado inconsistente.
   * @param {string} ctx Contexto de ejecución.
   * @param {string} msg Mensaje de advertencia.
   */
  warn: function (ctx, msg) {
    print("[RAVEN] [WARN] [" + ctx + "] " + msg);
  },

  /**
   * @brief Registra un error crítico junto con la traza de excepción asociada.
   * @param {string} ctx Contexto del error.
   * @param {string} msg Mensaje del error.
   * @param {Error|string} [err] Objeto de error o traza.
   */
  error: function (ctx, msg, err) {
    var trace = err ? " | Trace: " + err : "";
    print("[RAVEN] [ERROR] [" + ctx + "] " + msg + trace);
  },

  /**
   * @brief Emite mensajes de depuración sólo si debug_enabled es true.
   * @param {string} ctx Contexto.
   * @param {string} msg Mensaje de diagnóstico.
   */
  debug: function (ctx, msg) {
    if (this.debug_enabled) {
      print("[RAVEN] [DEBUG] [" + ctx + "] " + msg);
    }
  }
};
