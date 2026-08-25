/**
 * @fileoverview Punto de entrada base (Plantilla) para el puente Raven (Raven Bridge) en KDE Plasma 6.
 * Este archivo agrupa y carga los submódulos modulares a través de compilación por node o despliegue.
 */

//@include "utils/logger.js"
//@include "utils/timer_pool.js"
//@include "utils/geometry.js"
//@include "core/window_utils.js"
//@include "core/quarantine.js"
//@include "core/focus.js"
//@include "services/dbus_bridge.js"
//@include "services/shortcuts.js"

/**
 * Inicializa y registra los atajos de teclado globales de Raven en KWin.
 */
function initShortcuts() {
  registerRavenShortcuts();
}

/**
 * Inicializa el puente D-Bus completo, incluyendo:
 * - Pool de timers estáticos
 * - Clases de cuarentena del daemon
 * - Reglas de ventanas del daemon
 * - Hooks de ciclo de vida de ventanas (windowAdded, windowRemoved, etc.)
 * - Seguimiento de ventana activa
 * - Notificación de bridge listo al daemon Rust
 * - Primera sincronización completa
 */
function initDBusBridge() {
  // 1. Inicializar pool de timers estáticos
  initTimerPool();

  // 2. Enlazar ventanas existentes al puente (sin disparar syncs masivos)
  var existingWindows = workspace.windowList();
  for (var i = 0; i < existingWindows.length; i++) {
    var w = existingWindows[i];
    if (w && !w.deleted && isManageable(w)) {
      bindWindow(w);
    }
  }

  // 3. Hooks de ciclo de vida de ventanas
  workspace.windowAdded.connect(function (w) {
    processNewWindow(w);
  });

  workspace.windowRemoved.connect(function (w) {
    requestStateSync();
  });

  workspace.activeWindowChanged.connect(function () {
    var aw = workspace.activeWindow;
    var awId = aw ? getSafeWindowId(aw) : "";
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "windowActivated",
        awId || ""
      );
    } catch (e) { }
  });

  workspace.currentDesktopChanged.connect(function () {
    requestStateSync();
  });

  // 4. Solicitar configuración del daemon y notificar arranque de forma diferida (50ms)
  setKWinTimeout(function () {
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "bridgeReady"
      );
    } catch (e) { }

    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "getQuarantineClasses",
        function (res) {
          updateQuarantineClasses(res);
        }
      );
    } catch (e) { }

    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "getWindowRules",
        function (res) {
          try {
            if (res) {
              _window_rules = JSON.parse(res);
            }
          } catch (e) { }
        }
      );
    } catch (e) { }

    // Sincronización inicial única y limpia tras levantar el entorno
    requestStateSync();
  }, 100);
}

// Registro inicial de ciclo de vida
try {
  Logger.info("Main", "Inicializando el puente de Raven Tiling Emulator v3.0");
  initShortcuts();
  initDBusBridge();
  Logger.info("Main", "Puente inicializado exitosamente");
} catch (e) {
  Logger.error("Main", "Error crítico al inicializar el puente", e);
}
