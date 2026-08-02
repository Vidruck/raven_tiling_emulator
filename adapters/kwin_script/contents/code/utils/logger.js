/**
 * @fileoverview Modulo de Logger para el puente Raven en KWin (Plasma 6).
 */

var Logger = {
  // Activa esto a true si necesitas depurar localmente. Se deja en false por eficiencia.
  debug_enabled: false,

  info: function (ctx, msg) {
    print("[RAVEN] [INFO] [" + ctx + "] " + msg);
  },
  warn: function (ctx, msg) {
    print("[RAVEN] [WARN] [" + ctx + "] " + msg);
  },
  error: function (ctx, msg, err) {
    var trace = err ? " | Trace: " + err : "";
    print("[RAVEN] [ERROR] [" + ctx + "] " + msg + trace);
  },
  debug: function (ctx, msg) {
    if (this.debug_enabled) {
      print("[RAVEN] [DEBUG] [" + ctx + "] " + msg);
    }
  }
};
