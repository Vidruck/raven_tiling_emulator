/**
 * @fileoverview Registro e inyección de accesos directos globales nativos de KWin para Raven.
 */

/**
 * Registra los atajos globales nativos de KWin para controlar a Raven.
 * Expone las acciones del gestor de ventanas al panel de preferencias del sistema.
 */
function registerRavenShortcuts() {
  function dispatchToRaven(actionStr) {
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        actionStr,
        function (response) {
          if (response && response !== "[]") {
            applyCommands(response);
          }
        }
      );
    } catch (e) {
      Logger.error("Shortcuts", "Fallo al enviar atajo D-Bus: " + actionStr, e);
    }
  }

  function dispatchToRavenArg(actionStr, arg) {
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        actionStr,
        arg,
        function (response) {
          if (response && response !== "[]") {
            applyCommands(response);
          }
        }
      );
    } catch (e) {
      Logger.error("Shortcuts", "Fallo al enviar atajo D-Bus con arg: " + actionStr, e);
    }
  }

  registerShortcut("RavenToggleTiling", "Raven: Alternar Mosaico (On/Off)", "Meta+Space", function () {
    dispatchToRaven("toggleTiling");
  });
  registerShortcut("RavenToggleFloating", "Raven: Alternar Ventana Flotante Dinámica (Quick Peek)", "Meta+Shift+F", function () {
    var aw = workspace.activeWindow;
    var awId = aw ? getSafeWindowId(aw) : "";
    dispatchToRavenArg("toggleFloating", awId);
  });
  registerShortcut("RavenFocusNext", "Raven: Siguiente Ventana", "Meta+J", function () {
    dispatchToRaven("focusNext");
  });
  registerShortcut("RavenFocusPrev", "Raven: Ventana Anterior", "Meta+K", function () {
    dispatchToRaven("focusPrev");
  });
  registerShortcut("RavenFocusLeft", "Raven: Foco Izquierda", "Meta+Left", function () {
    dispatchToRaven("focusLeft");
  });
  registerShortcut("RavenFocusRight", "Raven: Foco Derecha", "Meta+Right", function () {
    dispatchToRaven("focusRight");
  });
  registerShortcut("RavenFocusUp", "Raven: Foco Arriba", "Meta+Up", function () {
    dispatchToRaven("focusUp");
  });
  registerShortcut("RavenFocusDown", "Raven: Foco Abajo", "Meta+Down", function () {
    dispatchToRaven("focusDown");
  });
  registerShortcut("RavenSwapNext", "Raven: Intercambiar Siguiente", "Meta+Shift+J", function () {
    dispatchToRaven("swapNext");
  });
  registerShortcut("RavenSwapPrev", "Raven: Intercambiar Anterior", "Meta+Shift+K", function () {
    dispatchToRaven("swapPrev");
  });
  registerShortcut("RavenIncreaseRatio", "Raven: Expandir Master", "Meta+H", function () {
    dispatchToRaven("increaseRatio");
  });
  registerShortcut("RavenDecreaseRatio", "Raven: Contraer Master", "Meta+L", function () {
    dispatchToRaven("decreaseRatio");
  });
  registerShortcut("RavenMigrateMonitor", "Raven: Enviar a Otro Monitor", "Meta+Shift+M", function () {
    dispatchToRaven("migrateActiveToScreen");
  });

  // Shortcuts para uso desde Plasmoid / Externo
  registerShortcut("RavenIncrementGaps", "Raven: Incrementar Gaps", "Meta+=", function () {
    dispatchToRavenArg("incrementGaps", 2);
  });
  registerShortcut("RavenDecrementGaps", "Raven: Decrementar Gaps", "Meta+-", function () {
    dispatchToRavenArg("incrementGaps", -2);
  });
  registerShortcut("RavenIncrementMaster", "Raven: Incrementar Master", "Meta+]", function () {
    dispatchToRaven("incrementMaster");
  });
  registerShortcut("RavenDecrementMaster", "Raven: Decrementar Master", "Meta+[", function () {
    dispatchToRaven("decrementMaster");
  });
  registerShortcut("RavenMigratePrevMonitor", "Raven: Enviar a Monitor Anterior", "Meta+Shift+N", function () {
    dispatchToRaven("migrateActiveToPrevScreen");
  });
  registerShortcut("RavenCycleLayout", "Raven: Ciclar Layout", "Meta+Shift+L", function() {
    dispatchToRaven("cycleLayout");
    if (workspace.activeWindow) highlightWindow(workspace.activeWindow);
  });
  registerShortcut("RavenMigrateDesktop", "Raven: Enviar a Escritorio Siguiente", "Meta+Shift+Right", function () {
    dispatchToRaven("migrateActiveToDesktop");
  });
  registerShortcut("RavenMigratePrevDesktop", "Raven: Enviar a Escritorio Anterior", "Meta+Shift+Left", function () {
    dispatchToRaven("migrateActiveToPrevDesktop");
  });

  registerShortcut("RavenResizeWidthInc", "Raven: Aumentar Ancho de Ventana", "Meta+Alt+Right", function () {
    dispatchToRaven("resize_width_inc");
  });
  registerShortcut("RavenResizeWidthDec", "Raven: Reducir Ancho de Ventana", "Meta+Alt+Left", function () {
    dispatchToRaven("resize_width_dec");
  });
  registerShortcut("RavenResizeHeightInc", "Raven: Aumentar Alto de Ventana", "Meta+Alt+Down", function () {
    dispatchToRaven("resize_height_inc");
  });
  registerShortcut("RavenResizeHeightDec", "Raven: Reducir Alto de Ventana", "Meta+Alt+Up", function () {
    dispatchToRaven("resize_height_dec");
  });
}
