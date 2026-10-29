/**
 * @file index.js
 * @brief Punto de entrada modular y orquestador del puente Raven (Raven Bridge) en KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.1
 */

//@include "utils/logger.js"
//@include "utils/geometry.js"
//@include "core/window_utils.js"
//@include "core/quarantine.js"
//@include "core/focus.js"
//@include "services/dbus_bridge.js"

/**
 * @brief Inicializa el ciclo de vida del puente D-Bus y enlaza las señales del compositor KWin.
 *
 * Secuencia de arranque (Libre de QTimers):
 * 1. Registra los atajos de teclado de KWin.
 * 2. Enlaza todas las ventanas gestionables ya presentes en el espacio de trabajo (`bindWindow`).
 * 3. Registra ganchos de eventos del compositor (`windowAdded`, `windowRemoved`, `windowActivated`, `currentDesktopChanged`).
 * 4. Suscribe la señal D-Bus `tilingCommandsPending` y notifica `bridgeReady` al demonio Rust.
 * 5. Ejecuta la primera sincronización de estado global (`requestStateSync`).
 */
function initDBusBridge() {
  // 1. Registrar atajos de teclado de KWin
  try {
    if (typeof registerRavenShortcuts === "function") {
      registerRavenShortcuts();
    }
  } catch (eShortcuts) {
    Logger.error("Main", "Error registrando atajos", eShortcuts);
  }

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
    // Blindaje: notificar cierre inmediato al motor Rust o sincronizar si era subventana/diálogo
    if (w) {
      var safeId = getSafeWindowId(w);
      if (safeId) {
        clearWindowDeltaCache(safeId);
      }
      if (safeId && w.__raven_bound) {
        try {
          callDBus(
            "org.kde.raven.Daemon",
            "/Events",
            "org.kde.raven.Events",
            "windowClosed",
            safeId,
            function (response) {
              if (response && response !== "[]") applyCommands(response);
            }
          );
          return;
        } catch (e) {}
      }
      if (w.__raven_bound || w.normalWindow || w.dialog || w.utility) {
        requestStateSync(false);
      }
    }
  });

  if (workspace.windowActivated) {
    workspace.windowActivated.connect(function (aw) {
      if (!aw || aw.deleted) {
        return;
      }
      // Si la ventana activada es un diálogo o subventana transitoria, asociar foco a su ventana madre
      var targetWin = (aw.transientFor != null) ? aw.transientFor : aw;
      if (!targetWin || targetWin.deleted || !isManageable(targetWin) || targetWin.popupWindow) {
        return;
      }
      var awId = getSafeWindowId(targetWin);
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
  }

  workspace.currentDesktopChanged.connect(function () {
    requestStateSync(false);
  });

  // Escuchar cambios en el área útil de trabajo (ej. aparición, redimensionamiento o movimiento del panel de Plasma)
  if (workspace.clientAreaChanged) {
    workspace.clientAreaChanged.connect(function () {
      requestStateSync(false);
    });
  }

  // Escuchar cambios o reconfiguración de pantallas
  if (workspace.screensChanged) {
    workspace.screensChanged.connect(function () {
      requestStateSync(false);
    });
  }

  // 4. Suscribir señal asíncrona y notificar arranque directamente (sin temporizadores en el compositor)
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

  try {
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "bridgeReady"
    );
  } catch (e) { }

  // 5. Sincronización inicial única y limpia
  requestStateSync(true);

  // 6. Informar foco activo inicial si existe una ventana gestionable enfocada
  try {
    if (workspace.activeWindow && !workspace.activeWindow.deleted && isManageable(workspace.activeWindow)) {
      var initAwId = getSafeWindowId(workspace.activeWindow);
      if (initAwId) {
        callDBus(
          "org.kde.raven.Daemon",
          "/Events",
          "org.kde.raven.Events",
          "windowActivated",
          initAwId
        );
      }
    }
  } catch (eInitFocus) { }
}

// Registro e inicialización de ciclo de vida en el motor de scripting de KWin
try {
  Logger.info("Main", "Inicializando el puente de Raven Tiling Emulator v5.1 (Zero-Timers)");
  initDBusBridge();
  Logger.info("Main", "Puente inicializado exitosamente");
} catch (e) {
  Logger.error("Main", "Error crítico al inicializar el puente", e);
}
