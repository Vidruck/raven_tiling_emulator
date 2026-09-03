/**
 * @file index.js
 * @brief Punto de entrada modular y orquestador del puente Raven (Raven Bridge) en KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
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
 * @brief Registra los atajos de teclado globales en el gestor de accesos directos de KWin.
 */
function initShortcuts() {
  registerRavenShortcuts();
}

/**
 * @brief Inicializa el ciclo de vida del puente D-Bus y enlaza las señales del compositor KWin.
 *
 * Secuencia de arranque:
 * 1. Inicializa el pool estático de temporizadores (`initTimerPool`).
 * 2. Enlaza todas las ventanas gestionables ya presentes en el espacio de trabajo (`bindWindow`).
 * 3. Registra ganchos de eventos del compositor (`windowAdded`, `windowRemoved`, `activeWindowChanged`, `currentDesktopChanged`).
 * 4. Suscribe la señal D-Bus `tilingCommandsPending` y notifica `bridgeReady` al demonio Rust.
 * 5. Ejecuta la primera sincronización de estado global (`requestStateSync`).
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
    if (aw && !isManageable(aw)) {
      // Ignorar paneles, diálogos de escritorio y plasmoides para no perder el foco previo de apps
      return;
    }
    var awId = aw ? getSafeWindowId(aw) : "";
    if (awId) {
      try {
        callDBus(
          "org.kde.raven.Daemon",
          "/Events",
          "org.kde.raven.Events",
          "windowActivated",
          awId
        );
      } catch (e) { }
    }
  });

  workspace.currentDesktopChanged.connect(function () {
    requestStateSync();
  });

  // 4. Solicitar configuración del daemon y notificar arranque de forma diferida (100ms)
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

    try {
      registerDBusSignal(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "tilingCommandsPending",
        function (commandsJson) {
          if (commandsJson && commandsJson !== "[]") {
            applyCommands(commandsJson);
          }
        }
      );
    } catch (e) { }

    // Sincronización inicial única y limpia tras levantar el entorno
    requestStateSync();
  }, 100);
}

// Registro e inicialización de ciclo de vida en el motor de scripting de KWin
try {
  Logger.info("Main", "Inicializando el puente de Raven Tiling Emulator v3.4");
  initShortcuts();
  initDBusBridge();
  Logger.info("Main", "Puente inicializado exitosamente");
} catch (e) {
  Logger.error("Main", "Error crítico al inicializar el puente", e);
}
