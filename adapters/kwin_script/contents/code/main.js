/**
 * @fileoverview Punto de entrada principal para el puente de Raven (Raven Bridge) en KDE Plasma 6 (Wayland) — v3.0
 * Orquesta los componentes del adaptador modular (utils, core, services) e inicializa los listeners de KWin.
 *
 * @author Alejandro González Hernández (Vidruck)
 */

// --- Importación / inclusión de módulos en orden de dependencia ---
// 1. Utilidades base y logs
// @include "utils/logger.js"
// @include "utils/geometry.js"
// @include "utils/timer_pool.js"

// 2. Núcleo de evaluación de ventanas
// @include "core/window_utils.js"
// @include "core/quarantine.js"
// @include "core/focus.js"

// 3. Servicios D-Bus y Atajos
// @include "services/dbus_bridge.js"
// @include "services/shortcuts.js"

/**
 * Inicializa el script puente de Raven conectando los listeners de KWin y disparando la sincronización inicial.
 */
function init() {
  Logger.info("init", "Inicializando v3.0 (Push-Based Multi-archivo)...");

  // Inicializar pool de timers estáticos (debe ser lo primero)
  initTimerPool();

  // Inyectar los atajos nativos de KWin
  registerRavenShortcuts();

  var initialWindows = workspace.windowList();
  for (var i = 0; i < initialWindows.length; i++) {
    bindWindow(initialWindows[i]);
  }

  // Conectar señales globales
  workspace.windowAdded.connect(onWindowAdded);
  workspace.windowRemoved.connect(onWindowRemoved);
  workspace.currentDesktopChanged.connect(onDesktopChanged);
  workspace.windowActivated.connect(onWindowActivated);

  // Detección dinámica de monitores (hot-plug)
  if (workspace.outputAdded) {
    workspace.outputAdded.connect(requestStateSync);
  }
  if (workspace.outputRemoved) {
    workspace.outputRemoved.connect(requestStateSync);
  }

  // Detección de tiling nativo de Plasma 6 para evitar conflictos
  try {
    var output = workspace.activeScreen || workspace.activeOutput;
    if (workspace.tilingForScreen && workspace.tilingForScreen(output)) {
      Logger.warn("init", "Se detectó Tiling Nativo activado para la pantalla. Podrían ocurrir conflictos severos.");
    }
  } catch(e) {}

  // Atajos de bordes de pantalla para acciones rápidas
  try {
    if (workspace.registerScreenEdge) {
      // 0 = TopEdge en KWin
      workspace.registerScreenEdge(0, function() {
        dispatchToRaven("cycleLayout");
        if (workspace.activeWindow) highlightWindow(workspace.activeWindow);
      });
    }
  } catch(e) {}

  try {
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "bridgeReady",
      function () {
        // Al estar listo, pedimos las configuraciones dinámicas
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
              if (res) _window_rules = JSON.parse(res);
            }
          );
        } catch (e) { }
      },
    );
  } catch (e) { }

  requestStateSync();
}

// ---- Manejadores de eventos globales ----

function onWindowAdded(w) {
  if (!isManageable(w)) {
    return;
  }

  var strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
  if (strClass !== "") {
    processNewWindow(w);
  } else {
    var classChangedConn = function() {
      if (w && !w.deleted) {
        processNewWindow(w);
        try {
          w.windowClassChanged.disconnect(classChangedConn);
        } catch(e) {}
      }
    };
    w.windowClassChanged.connect(classChangedConn);

    setKWinTimeout(function () {
      if (w && !w.deleted) {
        try {
          w.windowClassChanged.disconnect(classChangedConn);
        } catch(e) {}
        processNewWindow(w);
      }
    }, 50);
  }
}

/** Manejador estático de evento 'windowRemoved'. */
function onWindowRemoved() {
  requestStateSync();
}

/** Manejador estático de evento 'currentDesktopChanged'. */
function onDesktopChanged() {
  requestStateSync();
}

/**
 * Manejador estático de evento 'windowActivated'.
 * @param {KWin::Window} w - Ventana activada.
 */
function onWindowActivated(w) {
  if (w && isManageable(w)) {
    var id = getSafeWindowId(w);
    if (id) {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "windowActivated",
        id,
        function () { },
      );
    }
  }
}

try {
  init();
} catch (e) {
  Logger.error("Global", "Error crítico inicializando el bridge", e);
}
