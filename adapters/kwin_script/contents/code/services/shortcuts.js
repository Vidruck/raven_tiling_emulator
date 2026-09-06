/**
 * @file shortcuts.js
 * @brief Registro e integración de atajos de teclado globales nativos de KWin en KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/**
 * @brief Registra todos los atajos de teclado globales de Raven en el subsistema de accesos rápidos de KWin.
 *
 * Expone las acciones del gestor de mosaico en la sección "KWin" del panel de Preferencias del Sistema.
 * Cada combinación despacha un método D-Bus hacia `org.kde.raven.Daemon` y ejecuta inmediatamente los comandos retornados.
 */
function registerRavenShortcuts() {
  /**
   * @brief Envía una acción sin parámetros al demonio de Raven vía D-Bus.
   * @param {string} actionStr Nombre del método D-Bus.
   */
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

  /**
   * @brief Envía una acción con un argumento al demonio de Raven vía D-Bus.
   * @param {string} actionStr Nombre del método D-Bus.
   * @param {number|string} arg Valor del parámetro.
   */
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

  // ── GESTIÓN DE ESTADO Y FLOTACIÓN ──
  registerShortcut("RavenToggleTiling", "Raven: Alternar Mosaico (On/Off)", "Meta+Space", function () {
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "toggleTiling",
        function (response) {
          if (response && response !== "[]") {
            applyCommands(response);
          }
          // Verificar estado resultante para coordinar el comportamiento del puente
          callDBus(
            "org.kde.raven.Daemon",
            "/Events",
            "org.kde.raven.Events",
            "getTilingState",
            function (stateRes) {
              var isEnabled = (stateRes === "true" || stateRes === true);
              if (isEnabled) {
                // Modo activado: Sincronización forzada completa ("patada del puente a Rust")
                syncState();
              } else {
                // Modo desactivado: Desacomodo sutil (offset en cascada) para feedback visual claro
                var winList = workspace.windowList ? workspace.windowList() : [];
                var activeOut = workspace.activeOutput;
                var offset = 18;
                for (var i = 0; i < winList.length; i++) {
                  var w = winList[i];
                  if (w && !w.deleted && isManageable(w) && !w.minimized && !w.fullScreen && w.maximizeMode === 0) {
                    if (!activeOut || !w.output || w.output.name === activeOut.name) {
                      try {
                        w.__raven_mutating = true;
                        var geom = w.frameGeometry;
                        w.frameGeometry = {
                          x: geom.x + offset,
                          y: geom.y + offset,
                          width: geom.width,
                          height: geom.height
                        };
                        offset += 14;
                        (function (cw) {
                          setKWinTimeout(function () {
                            if (cw && !cw.deleted) {
                              cw.__raven_mutating = false;
                            }
                          }, 150);
                        })(w);
                      } catch (errGeom) {}
                    }
                  }
                }
              }
            }
          );
        }
      );
    } catch (e) {
      Logger.error("Shortcuts", "Fallo en RavenToggleTiling: " + e);
    }
  });
  registerShortcut("RavenToggleFloating", "Raven: Alternar Ventana Flotante Dinámica (Quick Peek)", "Meta+Shift+F", function () {
    var aw = workspace.activeWindow;
    var awId = aw ? getSafeWindowId(aw) : "";
    dispatchToRavenArg("toggleFloating", awId);
  });

  // ── NAVEGACIÓN Y FOCO VISUAL ──
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

  // ── INTERCAMBIO Y RATIOS DE COMPOSICIÓN ──
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

  // ── MIGRACIÓN ENTRE MONITORES Y ESCRITORIOS ──
  registerShortcut("RavenMigrateMonitor", "Raven: Enviar a Monitor Siguiente", "Meta+Shift+M", function () {
    dispatchToRaven("migrateActiveToScreen");
  });
  registerShortcut("RavenMigratePrevMonitor", "Raven: Enviar a Monitor Anterior", "Meta+Shift+N", function () {
    dispatchToRaven("migrateActiveToPrevScreen");
  });
  registerShortcut("RavenMigrateDesktop", "Raven: Enviar a Escritorio Siguiente", "Meta+Shift+Right", function () {
    dispatchToRaven("migrateActiveToDesktop");
  });
  registerShortcut("RavenMigratePrevDesktop", "Raven: Enviar a Escritorio Anterior", "Meta+Shift+Left", function () {
    dispatchToRaven("migrateActiveToPrevDesktop");
  });

  // ── MÁRGENES, CAPACIDAD Y CICLADO DE ALGORITMOS ──
  registerShortcut("RavenIncrementGaps", "Raven: Incrementar Gaps", "Meta+=", function () {
    dispatchToRavenArg("incrementGaps", 2);
  });
  registerShortcut("RavenDecrementGaps", "Raven: Decrementar Gaps", "Meta+-", function () {
    dispatchToRavenArg("incrementGaps", -2);
  });
  registerShortcut("RavenIncrementMaster", "Raven: Incrementar Capacidad Master", "Meta+]", function () {
    dispatchToRaven("incrementMaster");
  });
  registerShortcut("RavenDecrementMaster", "Raven: Decrementar Capacidad Master", "Meta+[", function () {
    dispatchToRaven("decrementMaster");
  });
  registerShortcut("RavenCycleLayout", "Raven: Ciclar Algoritmo de Disposición", "Meta+Shift+L", function() {
    dispatchToRaven("cycleLayout");
    if (workspace.activeWindow) highlightWindow(workspace.activeWindow);
  });

  // ── REDIMENSIONAMIENTO FINO POR VENTANA ──
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
