/**
 * @file shortcuts.js
 * @brief Registro de atajos de teclado globales en KWin (KDE Plasma 6).
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.0
 */

function registerRavenShortcuts() {
  function dispatchToRaven(actionStr, arg) {
    try {
      if (arg !== undefined) {
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", actionStr, arg, function (res) {
          if (res && res !== "[]") applyCommands(res);
        });
      } else {
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", actionStr, function (res) {
          if (res && res !== "[]") applyCommands(res);
        });
      }
    } catch (e) {
      Logger.error("Shortcuts", "Fallo al enviar atajo D-Bus: " + actionStr, e);
    }
  }

  // Gestión de estado y flotación
  registerShortcut("RavenToggleTiling", "Raven: Alternar Mosaico (On/Off)", "Meta+Space", function () {
    try {
      callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", "toggleTiling", function (response) {
        if (response && response !== "[]") applyCommands(response);
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", "getTilingState", function (stateRes) {
          if (stateRes === "true" || stateRes === true) syncState();
        });
      });
    } catch (e) {
      Logger.error("Shortcuts", "Fallo en RavenToggleTiling: " + e);
    }
  });

  registerShortcut("RavenToggleFloating", "Raven: Alternar Ventana Flotante Dinámica", "Meta+Shift+F", function () {
    const aw = workspace.activeWindow;
    dispatchToRaven("toggleFloating", aw ? getSafeWindowId(aw) || "" : "");
  });

  // Navegación y foco
  registerShortcut("RavenFocusNext", "Raven: Siguiente Ventana", "Meta+J", function () { dispatchToRaven("focusNext"); });
  registerShortcut("RavenFocusPrev", "Raven: Ventana Anterior", "Meta+K", function () { dispatchToRaven("focusPrev"); });
  registerShortcut("RavenFocusLeft", "Raven: Foco Izquierda", "Meta+Left", function () { dispatchToRaven("focusLeft"); });
  registerShortcut("RavenFocusRight", "Raven: Foco Derecha", "Meta+Right", function () { dispatchToRaven("focusRight"); });
  registerShortcut("RavenFocusUp", "Raven: Foco Arriba", "Meta+Up", function () { dispatchToRaven("focusUp"); });
  registerShortcut("RavenFocusDown", "Raven: Foco Abajo", "Meta+Down", function () { dispatchToRaven("focusDown"); });

  // Intercambio y ratios
  registerShortcut("RavenSwapNext", "Raven: Intercambiar Siguiente", "Meta+Shift+J", function () { dispatchToRaven("swapNext"); });
  registerShortcut("RavenSwapPrev", "Raven: Intercambiar Anterior", "Meta+Shift+K", function () { dispatchToRaven("swapPrev"); });
  registerShortcut("RavenIncreaseRatio", "Raven: Expandir Master", "Meta+H", function () { dispatchToRaven("increaseRatio"); });
  registerShortcut("RavenDecreaseRatio", "Raven: Contraer Master", "Meta+L", function () { dispatchToRaven("decreaseRatio"); });

  // Migración entre monitores y escritorios
  registerShortcut("RavenMigrateMonitor", "Raven: Enviar a Monitor Siguiente", "Meta+Shift+M", function () { dispatchToRaven("migrateActiveToScreen"); });
  registerShortcut("RavenMigratePrevMonitor", "Raven: Enviar a Monitor Anterior", "Meta+Shift+N", function () { dispatchToRaven("migrateActiveToPrevScreen"); });
  registerShortcut("RavenMigrateDesktop", "Raven: Enviar a Escritorio Siguiente", "Meta+Shift+Right", function () { dispatchToRaven("migrateActiveToDesktop"); });
  registerShortcut("RavenMigratePrevDesktop", "Raven: Enviar a Escritorio Anterior", "Meta+Shift+Left", function () { dispatchToRaven("migrateActiveToPrevDesktop"); });

  // Márgenes, capacidad y layouts
  registerShortcut("RavenIncrementGaps", "Raven: Incrementar Gaps", "Meta+=", function () { dispatchToRaven("incrementGaps", 2); });
  registerShortcut("RavenDecrementGaps", "Raven: Decrementar Gaps", "Meta+-", function () { dispatchToRaven("incrementGaps", -2); });
  registerShortcut("RavenIncrementMaster", "Raven: Incrementar Capacidad Master", "Meta+]", function () { dispatchToRaven("incrementMaster"); });
  registerShortcut("RavenDecrementMaster", "Raven: Decrementar Capacidad Master", "Meta+[", function () { dispatchToRaven("decrementMaster"); });
  registerShortcut("RavenCycleLayout", "Raven: Ciclar Algoritmo de Disposición", "Meta+Shift+L", function() {
    dispatchToRaven("cycleLayout");
    if (workspace.activeWindow) highlightWindow(workspace.activeWindow);
  });

  // Redimensionamiento fino por ventana
  registerShortcut("RavenResizeWidthInc", "Raven: Aumentar Ancho de Ventana", "Meta+Alt+Right", function () { dispatchToRaven("resize_width_inc"); });
  registerShortcut("RavenResizeWidthDec", "Raven: Reducir Ancho de Ventana", "Meta+Alt+Left", function () { dispatchToRaven("resize_width_dec"); });
  registerShortcut("RavenResizeHeightInc", "Raven: Aumentar Alto de Ventana", "Meta+Alt+Down", function () { dispatchToRaven("resize_height_inc"); });
  registerShortcut("RavenResizeHeightDec", "Raven: Reducir Alto de Ventana", "Meta+Alt+Up", function () { dispatchToRaven("resize_height_dec"); });
}
