/**
 * @file dbus_bridge.js
 * @brief Orquestador de comunicación D-Bus bidireccional entre el compositor KWin y el demonio Rust.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/** @type {QTimer|null} Temporizador de anti-rebote (debounce) para agrupar ráfagas de eventos de ventana. */
var _debounceTimer = null;

/**
 * @brief Solicita una sincronización global de estado con amortiguación temporal (debouncing de 50 ms).
 *
 * Agrupa múltiples eventos simultáneos de KWin (ej. cambio de escritorio + foco) en un único payload D-Bus.
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
    Logger.error("requestStateSync", "Fallo al solicitar sincronización de estado", e);
    try {
      syncState();
    } catch (err) { }
  }
}

/**
 * @brief Extrae la topología completa de pantallas, escritorios y ventanas activas y la despacha al demonio Rust vía D-Bus.
 *
 * Invoca el método `syncStateAndUpdateLayout` en `org.kde.raven.Events` y aplica de inmediato los comandos geométricos devueltos.
 */
function syncState() {
  const windows = workspace.windowList();
  const winState = [];
  const screens = {};

  const outs = workspace.screens || [];
  const desks = workspace.desktops || [];
  const currentDesk = workspace.currentDesktop;

  try {
    for (let o = 0; o < outs.length; o++) {
      const output = outs[o];
      const outName = output ? output.name : "default";

      if (desks && desks.length > 0) {
        for (let d = 0; d < desks.length; d++) {
          const desktop = desks[d];
          const deskId = desktop ? desktop.id.toString() : "default_desk";
          const wsId = outName + "||" + deskId;
          screens[wsId] = getSafeScreenGeometry(output, desktop);
        }
      } else {
        const deskId = currentDesk ? currentDesk.id.toString() : "default_desk";
        const wsId = outName + "||" + deskId;
        screens[wsId] = getSafeScreenGeometry(output, currentDesk);
      }
    }
  } catch (e) {
    Logger.error("syncState", "Error iterando topología de pantallas", e);
  }

  for (let i = 0; i < windows.length; i++) {
    const w = windows[i];
    try {
      if (!isManageable(w) || w.__raven_quarantined) {
        continue;
      }
      const safeId = getSafeWindowId(w);
      if (!safeId) {
        continue;
      }

      const output = w.output || workspace.activeOutput;
      const outName = output ? output.name : "default";

      const deskIds = [];
      if (w.desktops) {
        for (let d = 0; d < w.desktops.length; d++) {
          deskIds.push(w.desktops[d].id.toString());
        }
      }

      const strCap = w.caption ? w.caption.toString() : "";
      const strClass = w.resourceClass ? w.resourceClass.toString() : "";
      const geom = getRectGeometry(w.frameGeometry);

      winState.push({
        id: safeId,
        desktops: deskIds,
        output: outName,
        f: isFloating(w),
        m: Boolean(w.minimized),
        p: false,
        x: geom.x,
        y: geom.y,
        w: geom.w,
        h: geom.h,
        min_w: w.minSize ? Math.round(w.minSize.width) : 0,
        min_h: w.minSize ? Math.round(w.minSize.height) : 0,
        sb: Boolean(w.__raven_strict_birth),
        iq: Boolean(w.__raven_quarantined),
        fs: Boolean(w.fullScreen),
        cls: strClass,
        cap: strCap,
      });
    } catch (e) {
      Logger.error("syncState", "Error extrayendo geometría/estado de ventana", e);
    }
  }

  const masterOutputs = [];
  for (let o = 0; o < outs.length; o++) {
    if (outs[o] && outs[o].name) {
      masterOutputs.push(outs[o].name.toString());
    }
  }

  const masterDesktops = [];
  for (let d = 0; d < desks.length; d++) {
    if (desks[d] && desks[d].id) {
      masterDesktops.push(desks[d].id.toString());
    }
  }

  const payload = {
    windows: winState,
    screens: screens,
    topology: {
      outputs: masterOutputs,
      desktops: masterDesktops,
      current_desktop: currentDesk ? currentDesk.id.toString() : "",
    },
  };

  try {
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "syncStateAndUpdateLayout",
      JSON.stringify(payload),
      function (response) {
        if (response && response !== "[]") {
          applyCommands(response);
        }
      }
    );
  } catch (e) {
    Logger.error("syncState", "D-Bus Drop: Fallo enviando payload", e);
  }
}

/**
 * @brief Sincroniza de forma incremental el cambio de geometría o estado (delta sync) de una única ventana.
 *
 * Utilizado tras el redimensionamiento o movimiento manual de una ventana por el usuario para actualizar el modelo espacial de Rust.
 *
 * @param {KWin::Window} w Instancia de la ventana modificada.
 */
function syncWindowDelta(w) {
  try {
    if (!w || w.deleted || !isManageable(w) || w.__raven_quarantined) {
      return;
    }

    const safeId = getSafeWindowId(w);
    if (!safeId) {
      return;
    }

    const geom = getRectGeometry(w.frameGeometry);
    const deskIds = [];
    if (w.desktops) {
      for (let d = 0; d < w.desktops.length; d++) {
        deskIds.push(w.desktops[d].id.toString());
      }
    }

    const deltaPayload = {
      id: safeId,
      desktops: deskIds,
      output: w.output ? w.output.name : "default",
      f: isFloating(w),
      m: Boolean(w.minimized),
      p: false,
      x: geom.x,
      y: geom.y,
      w: geom.w,
      h: geom.h,
      min_w: w.minSize ? Math.round(w.minSize.width) : 0,
      min_h: w.minSize ? Math.round(w.minSize.height) : 0,
      sb: Boolean(w.__raven_strict_birth),
      iq: Boolean(w.__raven_quarantined),
      fs: Boolean(w.fullScreen),
      cls: w.resourceClass ? w.resourceClass.toString() : "",
      cap: w.caption ? w.caption.toString() : "",
    };
    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "syncWindowDelta",
      JSON.stringify(deltaPayload),
      function (response) {
        if (response && response !== "[]") {
          applyCommands(response);
        }
      }
    );
  } catch (e) {
    Logger.error("syncWindowDelta", "Fallo en sincronización incremental", e);
  }
}

/**
 * @brief Migra nativamente una ventana a un monitor o escritorio virtual especificado.
 *
 * @param {KWin::Window} win Instancia de la ventana a desplazar.
 * @param {string|null} target_output_name Nombre del monitor destino o null si permanece en el mismo.
 * @param {string|null} target_desktop_id Identificador del escritorio virtual destino o null.
 */
function migrateWindow(win, target_output_name, target_desktop_id) {
  if (!win || win.deleted) {
    return;
  }
  try {
    if (target_output_name) {
      const outputs = workspace.screens || [];
      for (let i = 0; i < outputs.length; i++) {
        const out = outputs[i];
        if (out && out.name === target_output_name) {
          try {
            if (typeof workspace.sendClientToScreen === "function") {
              workspace.sendClientToScreen(win, out);
            }
          } catch (errScreen) {
            Logger.debug("migrateWindow", "workspace.sendClientToScreen fallback: " + errScreen);
          }

          try {
            if (out.geometry && win.frameGeometry) {
              const geom = out.geometry;
              const curW = win.frameGeometry.width || 800;
              const curH = win.frameGeometry.height || 600;
              const newX = geom.x + Math.max(0, Math.floor((geom.width - curW) / 2));
              const newY = geom.y + Math.max(0, Math.floor((geom.height - curH) / 2));
              win.frameGeometry = {
                x: newX,
                y: newY,
                width: Math.min(curW, geom.width),
                height: Math.min(curH, geom.height)
              };
            }
          } catch (errGeom) {
            Logger.debug("migrateWindow", "win.frameGeometry relocation fallback: " + errGeom);
          }

          try {
            win.output = out;
          } catch (errOut) {}
          break;
        }
      }
    }
    if (target_desktop_id) {
      const desktops = workspace.desktops || [];
      for (let j = 0; j < desktops.length; j++) {
        if (desktops[j] && desktops[j].id.toString() === target_desktop_id) {
          win.desktops = [desktops[j]];
          break;
        }
      }
    }
  } catch (e) {
    Logger.error("migrateWindow", "Fallo al migrar ventana", e);
  }
}

/**
 * @brief Procesa y aplica en KWin la lista atómica de comandos JSON calculados por el demonio Rust.
 *
 * Ejecuta en dos pasadas:
 * 1. Mutaciones de estado y flotación (`set_floating`, `keepAbove`).
 * 2. Transformaciones geométricas (`move`, `focus`, `minimize`, `unminimize`, `migrate_to_output`).
 * Incorpora banderas `__raven_mutating` para suprimir ciclos recursivos de feedback con KWin.
 *
 * @param {string} commandsJson Cadena JSON con el vector de RavenAction devuelto por el demonio.
 */
function applyCommands(commandsJson) {
  if (!commandsJson) {
    return;
  }
  try {
    const cmds = JSON.parse(commandsJson);
    const windows = workspace.windowList();

    // Pasada 1: Comandos de mutación de estado/flags (set_floating, etc.)
    for (let i = 0; i < cmds.length; i++) {
      const cmd = cmds[i];
      if (cmd.action === "set_floating") {
        for (let j = 0; j < windows.length; j++) {
          const w = windows[j];
          if (getSafeWindowId(w) === cmd.window_id) {
            try {
              const wasFloating = Boolean(w.__raven_dynamic_float);
              w.__raven_dynamic_float = Boolean(cmd.floating);
              w.keepAbove = Boolean(cmd.keep_above);

              // Feedback visual táctil: si pasa a flotante, aplicar un ligero desajuste (nudge)
              if (cmd.floating && !wasFloating && !w.fullScreen && w.maximizeMode === 0) {
                w.__raven_mutating = true;
                const fg = w.frameGeometry;
                w.frameGeometry = {
                  x: fg.x + 24,
                  y: fg.y + 24,
                  width: Math.max(300, Math.round(fg.width * 0.95)),
                  height: Math.max(200, Math.round(fg.height * 0.95))
                };
                (function (cw) {
                  setKWinTimeout(function () {
                    if (cw && !cw.deleted) {
                      cw.__raven_mutating = false;
                    }
                  }, 150);
                })(w);
              }
            } catch (e) {
              Logger.error("applyCommands", "Error asignando estado flotante dinámico", e);
            }
            break;
          }
        }
      }
    }

    // Pasada 2: Comandos de posicionamiento, foco y migración
    for (let i = 0; i < cmds.length; i++) {
      const cmd = cmds[i];
      if (cmd.action === "set_floating") {
        continue;
      }
      if (cmd.action === "request_sync") {
        requestStateSync();
        continue;
      }

      if (cmd.action === "saturation_warning") {
        Logger.info("Raven UI", "Saturation warning received from core");
        try {
          if (workspace.showOutline) {
            const activeWin = workspace.activeWindow;
            if (activeWin) {
              workspace.showOutline(activeWin.frameGeometry);
            } else {
              const ca = workspace.clientArea(0, 0, workspace.currentDesktop);
              workspace.showOutline({
                x: ca.x, y: ca.y, width: ca.width, height: ca.height
              });
            }
            setKWinTimeout(function() {
              if (workspace.hideOutline) workspace.hideOutline();
            }, 300);
          }
        } catch(e) {}
        continue;
      }

      for (let j = 0; j < windows.length; j++) {
        const w = windows[j];
        if (getSafeWindowId(w) === cmd.window_id) {
          if (!w || w.deleted) {
            break;
          }

          if (cmd.action === "move") {
            try {
              if (
                w.interactiveMove ||
                w.interactiveResize ||
                w.__raven_ui_migrating
              ) {
                break;
              }

              if (w.maximizeMode !== 0 || w.fullScreen) {
                break;
              }

              w.__raven_mutating = true;
              const targetGeom = {
                x: Math.round(cmd.x),
                y: Math.round(cmd.y),
                width: Math.round(cmd.width),
                height: Math.round(cmd.height),
              };

              w.frameGeometry = targetGeom;

              (function (capturedWindow) {
                setKWinTimeout(function () {
                  if (capturedWindow && !capturedWindow.deleted) {
                    capturedWindow.__raven_mutating = false;
                  }
                }, 120);
              })(w);
            } catch (e) { }
          } else if (cmd.action === "focus") {
            workspace.activeWindow = w;
          } else if (cmd.action === "request_feedback") {
            if (w.__raven_strict_birth) {
              w.__raven_strict_birth = false;

              (function (cw) {
                setKWinTimeout(function () {
                  if (cw && !cw.deleted) {
                    requestStateSync();
                  }
                }, 480);
              })(w);
            }
          } else if (cmd.action === "minimize") {
            w.__raven_mutating = true;
            w.minimized = true;
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                  requestStateSync();
                }
              }, 100);
            })(w);
          } else if (cmd.action === "unminimize") {
            w.__raven_mutating = true;
            w.minimized = false;
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                  requestStateSync();
                }
              }, 100);
            })(w);
          } else if (cmd.action === "migrate_to_output") {
            w.__raven_mutating = true;
            migrateWindow(w, cmd.target_ws, null);
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                  requestStateSync();
                }
              }, 150);
            })(w);
          } else if (cmd.action === "migrate_to_desktop") {
            w.__raven_mutating = true;
            migrateWindow(w, null, cmd.target_ws);
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                  requestStateSync();
                }
              }, 150);
            })(w);
          } else if (cmd.action === "saturation_warning") {
            Logger.warn("Saturation", "Pantalla cerca de saturación: " + cmd.active + "/" + cmd.cmax + " ventanas");
          }
          break;
        }
      }
    }
  } catch (e) {
    Logger.error("applyCommands", "Fallo crítico aplicando comandos del daemon. Payload: " + commandsJson, e);
  }
}

/**
 * @brief Enlaza los eventos y señales reactivas del ciclo de vida de una ventana con el puente de Raven.
 *
 * Suscribe escuchadores para:
 * - Minimización y restauración (`minimizedChanged`).
 * - Maximización y desmaximización (`maximizedChanged`).
 * - Pantalla completa (`fullScreenChanged`).
 * - Cambio de título (`captionChanged` para refresco PiP).
 * - Migración de pantalla o escritorio virtual (`outputChanged`, `desktopsChanged`).
 * - Modificación geométrica interactiva (`frameGeometryChanged`, `interactiveMoveResizeFinished`).
 *
 * @param {KWin::Window} w Instancia de la ventana a enlazar.
 */
function bindWindow(w) {
  try {
    if (!isManageable(w) || w.__raven_bound) {
      return;
    }
    w.__raven_bound = true;

    w.minimizedChanged.connect(function () {
      if (
        w &&
        !w.deleted &&
        !w.__raven_mutating &&
        !w.interactiveMove &&
        !w.interactiveResize
      ) {
        requestStateSync();
      }
    });

    w.maximizedChanged.connect(function () {
      if (
        w &&
        !w.deleted &&
        !w.__raven_mutating &&
        !w.interactiveMove &&
        !w.interactiveResize
      ) {
        requestStateSync();
      }
    });

    if (w.fullScreenChanged !== undefined) {
      w.fullScreenChanged.connect(function () {
        if (
          w &&
          !w.deleted &&
          !w.__raven_mutating &&
          !w.interactiveMove &&
          !w.interactiveResize
        ) {
          requestStateSync();
        }
      });
    }

    if (w.captionChanged !== undefined) {
      w.captionChanged.connect(function () {
        if (
          w &&
          !w.deleted &&
          !w.__raven_mutating
        ) {
          requestStateSync();
        }
      });
    }

    w.outputChanged.connect(function () {
      if (!w || w.deleted || w.__raven_mutating) {
        return;
      }
      if (!w.interactiveMove && !w.interactiveResize) {
        w.__raven_ui_migrating = true;
        (function (cw) {
          setKWinTimeout(function () {
            if (cw && !cw.deleted) {
              cw.__raven_ui_migrating = false;
            }
          }, 250);
        })(w);
      }
      requestStateSync();
    });

    w.desktopsChanged.connect(function () {
      if (!w || w.deleted || w.__raven_mutating) {
        return;
      }
      if (!w.interactiveMove && !w.interactiveResize) {
        w.__raven_ui_migrating = true;
        (function (cw) {
          setKWinTimeout(function () {
            if (cw && !cw.deleted) {
              cw.__raven_ui_migrating = false;
            }
          }, 250);
        })(w);
      }
      requestStateSync();
    });

    w.frameGeometryChanged.connect(function () {
      if (!w || w.deleted) {
        return;
      }
      if (w.__raven_quarantined && w.__raven_stab_timer) {
        const t = w.__raven_stab_timer;
        if (t.timer) {
          t.timer.stop();
          t.timer.start();
        } else if (typeof t.stop === "function") {
          t.stop();
          t.start();
        }
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
      w.interactiveMoveResizeFinished.connect(function () {
        if (w && !w.deleted) {
          w.__was_interacting = false;
          requestStateSync();
        }
      });
    }
  } catch (e) {
    Logger.error("bindWindow", "Error enlazando eventos de ventana", e);
  }
}
