/**
 * @fileoverview Puente de Raven (Raven Bridge) para KDE Plasma 6 (Wayland).
 * Proporciona la integración entre el compositor de ventanas KWin y el
 * motor de mosaico (tiling engine) nativo en Rust a través de D-Bus.
 * 
 * @author Alejandro González Hernández (Vidruck)
 */

var _debounceTimer = null;
var _is_listening = false;
var _watchdog_timer = null;
var _active_timers = [];
var _quarantine_classes = ["firefox", "electron", "zen-browser", "code", "spotify", "floorp", "chrome"];

try {
    _debounceTimer = new QTimer();
    _debounceTimer.interval = 50;
    _debounceTimer.singleShot = true;
    _debounceTimer.timeout.connect(syncState);
} catch (e) {
    print("[Raven Bridge] Error inicializando timer global: " + e);
}

/**
 * Obtiene de forma segura el identificador único (ID) de una ventana.
 * 
 * @param {KWin::Window} w - Objeto de ventana de KWin.
 * @returns {string|null} Identificador único en formato cadena de texto (string) o null si es inválido.
 */
function getSafeWindowId(w) {
    try {
        if (!w || !w.internalId) {
            return null;
        }
        return w.internalId.toString();
    } catch (e) {
        return null;
    }
}

/**
 * Obtiene el identificador único del área de trabajo (workspace ID) para una ventana.
 * Combina el nombre de la salida (output) y el identificador del escritorio virtual (virtual desktop ID).
 * 
 * @param {KWin::Window} window - Objeto de ventana de KWin.
 * @returns {string} Identificador único del área de trabajo en formato "salida||escritorio".
 */
function getWorkspaceId(window) {
    try {
        if (!window || window.deleted) {
            return "default||default_desk";
        }
        var output = window.output || workspace.activeOutput;
        var outName = output ? output.name : "default";
        var desktopId = (window.desktops && window.desktops.length > 0) ?
            window.desktops[0].id.toString() :
            (workspace.currentDesktop ? workspace.currentDesktop.id.toString() : "default_desk");
        return outName + "||" + desktopId;
    } catch (e) {
        return "default||default_desk";
    }
}

/**
 * Determina si una ventana es gestionable (manageable) por el motor de mosaico (tiling engine).
 * 
 * @param {KWin::Window} w - Objeto de ventana de KWin.
 * @returns {boolean} Verdadero si la ventana debe ser gestionada; de lo contrario, falso.
 */
function isManageable(w) {
    try {
        if (!w || w.deleted || !w.managed) {
            return false;
        }
        if (w.popupWindow || w.tooltip || w.onScreenDisplay || w.notification || w.specialWindow) {
            return false;
        }
        if (w.desktopWindow || w.dock || w.splash || w.skipTaskbar || w.skipPager) {
            return false;
        }

        var strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
        if (strClass.indexOf("spectacle") !== -1 && w.fullScreen) {
            return false;
        }
        if (!w.normalWindow && !w.dialog && !w.utility) {
            return false;
        }

        return true;
    } catch (e) {
        return false;
    }
}

/**
 * Determina si una ventana debe comportarse como flotante (floating).
 * 
 * @param {KWin::Window} w - Objeto de ventana de KWin.
 * @returns {boolean} Verdadero si es flotante; de lo contrario, falso.
 */
function isFloating(w) {
    try {
        if (!w || w.deleted) {
            return true;
        }
        if (w.dialog || w.utility || w.specialWindow || w.modal || w.transientFor) {
            return true;
        }
        if (w.maximizeMode == 3 || w.fullScreen) {
            return true;
        }

        var strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
        var strCap = w.caption ? w.caption.toString().toLowerCase() : "";

        var isPip = strCap.indexOf("picture-in-picture") !== -1 || strCap.indexOf("pip") !== -1 || w.keepAbove;
        if (isPip && !w.keepAbove) {
            w.keepAbove = true;
        }

        var isRaven = strClass.indexOf("raven") !== -1 || strCap.indexOf("raven control center") !== -1;
        var isSpectacle = strClass.indexOf("spectacle") !== -1;
        var isKlipper = strClass.indexOf("klipper") !== -1 || strClass.indexOf("plasma.clipboard") !== -1;
        var isVirtPopup = (strClass.indexOf("qemu") !== -1 || strClass.indexOf("virt-manager") !== -1) && !w.normalWindow;

        return Boolean(isPip || isSpectacle || isKlipper || isVirtPopup || isRaven);
    } catch (e) {
        return true;
    }
}

/**
 * Solicita de forma asíncrona la sincronización de estado (state sync) con filtrado de rebotes (debouncing).
 */
function requestStateSync() {
    try {
        if (!_debounceTimer) {
            _debounceTimer = new QTimer();
            _debounceTimer.interval = 50;
            _debounceTimer.singleShot = true;
            _debounceTimer.timeout.connect(syncState);
        }
        if (_debounceTimer.active) {
            _debounceTimer.stop();
        }
        _debounceTimer.start();
    } catch (e) {
        print("[Raven] Error en requestStateSync: " + e);
        try {
            syncState();
        } catch (err) {}
    }
}

/**
 * Normaliza y obtiene un objeto de geometría en coordenadas enteras de pantalla a partir de un Rect.
 * 
 * @param {QtRect} rect - Estructura de geometría nativa de Qt.
 * @returns {Object} Objeto con las propiedades normalizadas {x, y, w, h}.
 */
function getRectGeometry(rect) {
    if (!rect) {
        return { x: 0, y: 0, w: 1920, h: 1080 };
    }
    var x = typeof rect.x === "function" ? rect.x() : (rect.x !== undefined ? rect.x : 0);
    var y = typeof rect.y === "function" ? rect.y() : (rect.y !== undefined ? rect.y : 0);
    var w = typeof rect.width === "function" ? rect.width() : (rect.width !== undefined ? rect.width : (typeof rect.w === "function" ? rect.w() : (rect.w !== undefined ? rect.w : 1920)));
    var h = typeof rect.height === "function" ? rect.height() : (rect.height !== undefined ? rect.height : (typeof rect.h === "function" ? rect.h() : (rect.h !== undefined ? rect.h : 1080)));
    return { x: Math.round(x), y: Math.round(y), w: Math.round(w), h: Math.round(h) };
}

/**
 * Obtiene de forma segura el área útil de la pantalla (screen geometry) para un escritorio virtual y salida dados.
 * 
 * @param {KWin::Output} output - Salida física de pantalla.
 * @param {KWin::VirtualDesktop} desktop - Escritorio virtual.
 * @returns {Object} Geometría útil del área de trabajo.
 */
function getSafeScreenGeometry(output, desktop) {
    if (!output) {
        return { x: 0, y: 0, w: 1920, h: 1080 };
    }
    try {
        var area = workspace.clientArea(0, output, desktop);
        if (area && area.width > 0 && area.height > 0) {
            return getRectGeometry(area);
        }
    } catch (e) {}
    try {
        if (output.geometry) {
            return getRectGeometry(output.geometry);
        }
    } catch (e) {}
    return { x: 0, y: 0, w: 1920, h: 1080 };
}

/**
 * Sincroniza el estado completo del compositor enviándolo al demonio (daemon) de Rust vía D-Bus.
 */
function syncState() {
    var windows = workspace.windowList();
    var winState = [];
    var screens = {};

    var outs = workspace.screens || [];
    var desks = workspace.desktops || [];
    var currentDesk = workspace.currentDesktop;

    try {
        for (var o = 0; o < outs.length; o++) {
            var output = outs[o];
            var outName = output ? output.name : "default";

            var placementUsableGeometry = getSafeScreenGeometry(output, currentDesk);

            if (desks && desks.length > 0) {
                for (var d = 0; d < desks.length; d++) {
                    var desktop = desks[d];
                    var deskId = desktop ? desktop.id.toString() : "default_desk";
                    var wsId = outName + "||" + deskId;
                    screens[wsId] = placementUsableGeometry;
                }
            } else {
                var deskId = currentDesk ? currentDesk.id.toString() : "default_desk";
                var wsId = outName + "||" + deskId;
                screens[wsId] = placementUsableGeometry;
            }
        }
    } catch (e) {
        print("[Raven] Error topología pantallas: " + e);
    }

    for (var i = 0; i < windows.length; i++) {
        var w = windows[i];
        try {
            if (!isManageable(w) || w.__raven_quarantined) {
                continue;
            }
            var safeId = getSafeWindowId(w);
            if (!safeId) {
                continue;
            }

            var output = w.output || workspace.activeOutput;
            var outName = output ? output.name : "default";

            var deskIds = [];
            if (w.desktops) {
                for (var d = 0; d < w.desktops.length; d++) {
                    deskIds.push(w.desktops[d].id.toString());
                }
            }

            var wsId = getWorkspaceId(w);
            var geom = getRectGeometry(w.frameGeometry);

            winState.push({
                id: safeId,
                ws: wsId,
                desktops: deskIds,
                output: outName,
                f: isFloating(w),
                m: Boolean(w.minimized),
                p: Boolean(w.keepAbove),
                x: geom.x,
                y: geom.y,
                w: geom.w,
                h: geom.h,
                min_w: w.minSize ? Math.round(w.minSize.width) : 0,
                min_h: w.minSize ? Math.round(w.minSize.height) : 0,
                sb: Boolean(w.__raven_strict_birth)
            });
        } catch (e) {
            print("[Raven] Error mapeando ventana: " + e);
        }
    }

    var masterOutputs = [];
    for (var o = 0; o < outs.length; o++) {
        if (outs[o] && outs[o].name) {
            masterOutputs.push(outs[o].name.toString());
        }
    }

    var masterDesktops = [];
    for (var d = 0; d < desks.length; d++) {
        if (desks[d] && desks[d].id) {
            masterDesktops.push(desks[d].id.toString());
        }
    }

    var payload = {
        windows: winState,
        screens: screens,
        topology: {
            outputs: masterOutputs,
            desktops: masterDesktops
        }
    };

    try {
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", "syncState", JSON.stringify(payload));
    } catch (e) {
        print("[Raven Bridge] D-bus Drop: " + e);
    }
}

/**
 * Sincroniza de forma incremental el cambio de geometría o estado (delta sync) de una única ventana.
 * 
 * @param {KWin::Window} w - Objeto de ventana modificado.
 */
function syncWindowDelta(w) {
    try {
        if (!w || w.deleted || !isManageable(w) || w.__raven_quarantined) {
            return;
        }
        var safeId = getSafeWindowId(w);
        if (!safeId) {
            return;
        }

        var geom = getRectGeometry(w.frameGeometry);
        var deskIds = [];
        if (w.desktops) {
            for (var d = 0; d < w.desktops.length; d++) {
                deskIds.push(w.desktops[d].id.toString());
            }
        }

        var deltaPayload = {
            id: safeId,
            ws: getWorkspaceId(w),
            output: (w.output ? w.output.name : "default"),
            desktops: deskIds,
            f: isFloating(w),
            m: Boolean(w.minimized),
            p: Boolean(w.keepAbove),
            x: geom.x,
            y: geom.y,
            w: geom.w,
            h: geom.h,
            min_w: w.minSize ? Math.round(w.minSize.width) : 0,
            min_h: w.minSize ? Math.round(w.minSize.height) : 0,
            sb: Boolean(w.__raven_strict_birth)
        };
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", "syncWindowDelta", JSON.stringify(deltaPayload));
    } catch (e) {
        print("[Raven] Error Delta Sync: " + e);
    }
}

/**
 * Migra nativamente una ventana a una pantalla (output) o escritorio virtual específico.
 * 
 * @param {KWin::Window} win - Objeto de ventana.
 * @param {string|null} target_output_name - Nombre de la salida destino o null.
 * @param {string|null} target_desktop_id - Identificador del escritorio virtual destino o null.
 */
function migrateWindow(win, target_output_name, target_desktop_id) {
    if (!win || win.deleted) {
        return;
    }
    try {
        if (target_output_name) {
            var outputs = workspace.screens || [];
            for (var i = 0; i < outputs.length; i++) {
                if (outputs[i].name === target_output_name) {
                    workspace.sendClientToScreen(win, outputs[i]);
                    break;
                }
            }
        }
        if (target_desktop_id) {
            var desktops = workspace.desktops || [];
            for (var j = 0; j < desktops.length; j++) {
                if (desktops[j].id.toString() === target_desktop_id) {
                    win.desktops = [desktops[j]];
                    break;
                }
            }
        }
    } catch (e) {
        print("[Raven] Fallo en migración nativa: " + e);
    }
}

/**
 * Procesa y aplica los comandos JSON recibidos desde el demonio (daemon) de Rust.
 * 
 * @param {string} commandsJson - Carga de comandos serializada en JSON.
 */
function applyCommands(commandsJson) {
    if (!commandsJson) {
        return;
    }
    try {
        var cmds = JSON.parse(commandsJson);
        var windows = workspace.windowList();

        for (var i = 0; i < cmds.length; i++) {
            var cmd = cmds[i];
            if (cmd.action === "request_sync") {
                requestStateSync();
                continue;
            }

            for (var j = 0; j < windows.length; j++) {
                var w = windows[j];
                if (getSafeWindowId(w) === cmd.window_id) {
                    if (!w || w.deleted) {
                        break;
                    }

                    if (cmd.action === "move") {
                        try {
                            if (w.maximizeMode === 3 || w.interactiveMove || w.interactiveResize || w.__raven_ui_migrating) {
                                break;
                            }
                            w.__raven_mutating = true;
                            w.frameGeometry = {
                                x: Math.round(cmd.x),
                                y: Math.round(cmd.y),
                                width: Math.round(cmd.width),
                                height: Math.round(cmd.height)
                            };

                            (function(capturedWindow) {
                                setKWinTimeout(function() {
                                    if (capturedWindow && !capturedWindow.deleted) {
                                        capturedWindow.__raven_mutating = false;
                                    }
                                }, 400);
                            })(w);
                        } catch (e) {}
                    } else if (cmd.action === "focus") {
                        workspace.activeWindow = w;
                    } else if (cmd.action === "request_feedback") {
                        if (w.__raven_strict_birth) {
                            w.__raven_strict_birth = false;

                            (function(cw) {
                                setKWinTimeout(function() {
                                    if (cw && !cw.deleted) {
                                        requestStateSync();
                                    }
                                }, 480);
                            })(w);
                        }
                    } else if (cmd.action === "minimize") {
                        w.__raven_mutating = true;
                        w.minimized = true;
                        (function(cw) {
                            setKWinTimeout(function() {
                                if (cw && !cw.deleted) {
                                    cw.__raven_mutating = false;
                                    requestStateSync();
                                }
                            }, 100);
                        })(w);
                    } else if (cmd.action === "unminimize") {
                        w.__raven_mutating = true;
                        w.minimized = false;
                        (function(cw) {
                            setKWinTimeout(function() {
                                if (cw && !cw.deleted) {
                                    cw.__raven_mutating = false;
                                    requestStateSync();
                                }
                            }, 100);
                        })(w);
                    } else if (cmd.action === "migrate_to_output") {
                        w.__raven_mutating = true;
                        migrateWindow(w, cmd.target_ws, null);
                        (function(cw) {
                            setKWinTimeout(function() {
                                if (cw && !cw.deleted) {
                                    cw.__raven_mutating = false;
                                    requestStateSync();
                                }
                            }, 150);
                        })(w);
                    } else if (cmd.action === "migrate_to_desktop") {
                        w.__raven_mutating = true;
                        migrateWindow(w, null, cmd.target_ws);
                        (function(cw) {
                            setKWinTimeout(function() {
                                if (cw && !cw.deleted) {
                                    cw.__raven_mutating = false;
                                    requestStateSync();
                                }
                            }, 150);
                        })(w);
                    }
                    break;
                }
            }
        }
    } catch (e) {
        print("[Raven Bridge] Error applyCommands: " + e);
    }
}

/**
 * Crea y registra un temporizador (timer) de disparo único (single shot) para ejecutar una retrollamada (callback).
 * 
 * @param {function} callback - Función de retrollamada a ejecutar al completarse el tiempo.
 * @param {number} ms - Tiempo de espera en milisegundos.
 * @returns {QTimer|null} Instancia del temporizador creado o null si falló la inicialización.
 */
function setKWinTimeout(callback, ms) {
    try {
        var timer = new QTimer();
        timer.interval = ms;
        timer.singleShot = true;
        _active_timers.push(timer);
        timer.timeout.connect(function() {
            try {
                callback();
            } catch (e) {} finally {
                try {
                    timer.stop();
                } catch (err) {}
                var idx = _active_timers.indexOf(timer);
                if (idx !== -1) {
                    _active_timers.splice(idx, 1);
                }
            }
        });
        timer.start();
        return timer;
    } catch (e) {
        try {
            callback();
        } catch (err) {}
        return null;
    }
}

/**
 * Inicia el proceso de escucha asíncrona de comandos pendientes desde el demonio (daemon) mediante D-Bus.
 * Cuenta con un temporizador supervisor (watchdog timer) para recuperarse de posibles bloqueos.
 */
function listenForCommands() {
    if (_is_listening) {
        return;
    }
    _is_listening = true;
    if (_watchdog_timer) {
        try {
            _watchdog_timer.stop();
        } catch (e) {}
    }
    _watchdog_timer = setKWinTimeout(function() {
        _is_listening = false;
        listenForCommands();
    }, 6000);

    try {
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", "getPendingCommands", function(response) {
            if (_watchdog_timer) {
                try {
                    _watchdog_timer.stop();
                } catch (e) {}
            }
            _is_listening = false;

            if (response && response !== "[]") {
                applyCommands(response);
                setKWinTimeout(listenForCommands, 30);
            } else {
                setKWinTimeout(listenForCommands, 350);
            }
        });
    } catch (e) {
        _is_listening = false;
        setKWinTimeout(listenForCommands, 1000);
    }
}

/**
 * Enlaza (binds) los eventos principales de una ventana a las funciones de sincronización del puente de Raven.
 * 
 * @param {KWin::Window} w - Objeto de ventana.
 */
function bindWindow(w) {
    try {
        if (!isManageable(w) || w.__raven_bound) {
            return;
        }
        w.__raven_bound = true;

        w.minimizedChanged.connect(function() {
            if (w && !w.deleted && !w.__raven_mutating && !w.interactiveMove && !w.interactiveResize) {
                requestStateSync();
            }
        });

        w.outputChanged.connect(function() {
            if (!w || w.deleted) {
                return;
            }
            if (w.__raven_mutating) {
                return;
            }

            if (!w.interactiveMove && !w.interactiveResize) {
                w.__raven_ui_migrating = true;
                (function(cw) {
                    setKWinTimeout(function() {
                        if (cw && !cw.deleted) {
                            cw.__raven_ui_migrating = false;
                        }
                    }, 250);
                })(w);
            }
            requestStateSync();
        });

        w.desktopsChanged.connect(function() {
            if (!w || w.deleted) {
                return;
            }
            if (w.__raven_mutating) {
                return;
            }

            if (!w.interactiveMove && !w.interactiveResize) {
                w.__raven_ui_migrating = true;
                (function(cw) {
                    setKWinTimeout(function() {
                        if (cw && !cw.deleted) {
                            cw.__raven_ui_migrating = false;
                        }
                    }, 250);
                })(w);
            }
            requestStateSync();
        });

        w.frameGeometryChanged.connect(function() {
            if (!w || w.deleted) {
                return;
            }

            if (w.__raven_quarantined && w.__raven_stab_timer) {
                w.__raven_stab_timer.stop();
                w.__raven_stab_timer.start();
                return;
            }

            if (w.interactiveMove || w.interactiveResize) {
                w.__was_interacting = true;
                return;
            }
            if (w.__was_interacting && !w.interactiveMove && !w.interactiveResize) {
                w.__was_interacting = false;
                requestStateSync();
                return;
            }
            if (w.__raven_mutating || w.__raven_ui_migrating) {
                return;
            }

            syncWindowDelta(w);
        });

        if (w.interactiveMoveResizeFinished !== undefined) {
            w.interactiveMoveResizeFinished.connect(function() {
                if (w && !w.deleted) {
                    w.__was_interacting = false;
                    requestStateSync();
                }
            });
        }
    } catch (e) {
        print("[Raven] Error bindWindow: " + e);
    }
}

/**
 * Inicializa el script puente de Raven conectando los listeners de KWin y disparando la sincronización inicial.
 */
function init() {
    print("[Raven Bridge] Inicializando v2.7...");

    var initialWindows = workspace.windowList();
    for (var i = 0; i < initialWindows.length; i++) {
        bindWindow(initialWindows[i]);
    }

    workspace.windowAdded.connect(function(w) {
        if (!isManageable(w)) {
            return;
        }
        var strClass = w.resourceClass ? w.resourceClass.toString().toLowerCase() : "";
        var needsQuarantine = false;

        for (var i = 0; i < _quarantine_classes.length; i++) {
            if (strClass.indexOf(_quarantine_classes[i]) !== -1) {
                needsQuarantine = true;
                break;
            }
        }

        if (needsQuarantine) {
            w.__raven_quarantined = true;
            bindWindow(w);

            var stabTimer = new QTimer();
            stabTimer.interval = 220;
            stabTimer.singleShot = true;
            w.__raven_stab_timer = stabTimer;

            stabTimer.timeout.connect(function() {
                if (w && !w.deleted) {
                    w.__raven_quarantined = false;
                    w.__raven_strict_birth = true;
                    w.__raven_stab_timer = null;
                    requestStateSync();
                }
                stabTimer.destroy();
            });
            stabTimer.start();
        } else {
            bindWindow(w);
            requestStateSync();
        }
    });

    workspace.windowRemoved.connect(function() {
        requestStateSync();
    });
    workspace.windowActivated.connect(function(w) {
        if (w && isManageable(w)) {
            var id = getSafeWindowId(w);
            if (id) {
                callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", "windowActivated", id, function() {});
            }
        }
    });

    try {
        callDBus("org.kde.raven.Daemon", "/Events", "org.kde.raven.Events", "bridgeReady", function() {});
    } catch (e) {}

    requestStateSync();
    listenForCommands();
}

try {
    init();
} catch (e) {
    print("[Raven Bridge] Error crítico: " + e);
}
