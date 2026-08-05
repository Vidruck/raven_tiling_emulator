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

  // 2. Solicitar clases de cuarentena personalizadas del daemon
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
  } catch (e) {
    Logger.warn("initDBusBridge", "No se pudo obtener clases de cuarentena del daemon");
  }

  // 3. Solicitar reglas de ventanas del daemon
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
        } catch (e) {
          Logger.warn("initDBusBridge", "Error parseando reglas de ventana");
        }
      }
    );
  } catch (e) {
    Logger.warn("initDBusBridge", "No se pudo obtener reglas de ventana del daemon");
  }

  // 4. Enlazar ventanas existentes al puente
  var existingWindows = workspace.windowList();
  for (var i = 0; i < existingWindows.length; i++) {
    processNewWindow(existingWindows[i]);
  }

  // 5. Hook: nueva ventana agregada
  workspace.windowAdded.connect(function (w) {
    processNewWindow(w);
  });

  // 6. Hook: ventana eliminada / cerrada
  workspace.windowRemoved.connect(function (w) {
    requestStateSync();
  });

  // 7. Hook: cambio de ventana activa → reportar al daemon para foco
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
    } catch (e) {
      Logger.error("activeWindowChanged", "Fallo al reportar ventana activa", e);
    }
  });

  // 8. Hook: cambio de escritorio virtual activo
  workspace.currentDesktopChanged.connect(function () {
    requestStateSync();
  });

  // 9. Notificar al daemon que el puente está operativo
  try {
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "bridgeReady"
    );
  } catch (e) {
    Logger.warn("initDBusBridge", "No se pudo notificar bridgeReady al daemon");
  }

  // 10. Primera sincronización completa de estado
  requestStateSync();
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
