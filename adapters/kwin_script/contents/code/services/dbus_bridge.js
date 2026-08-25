/**
 * @fileoverview Servicios de comunicación D-Bus para sincronización entre KWin y el demonio Rust.
 */

var _debounceTimer = null;

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
    Logger.error("requestStateSync", "Fallo al solicitar sincronización de estado", e);
    try {
      syncState();
    } catch (err) { }
  }
}

/**
 * Sincroniza el estado completo del compositor enviándolo al demonio (daemon) de Rust vía D-Bus.
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

      const strCap = w.caption ? w.caption.toString().toLowerCase() : "";
      const isPipWindow = PIP_CAPTION_REGEX.test(strCap);
      const wsId = getWorkspaceId(w);
      const geom = getRectGeometry(w.frameGeometry);

      winState.push({
        id: safeId,
        ws: wsId,
        desktops: deskIds,
        output: outName,
        f: isFloating(w),
        m: Boolean(w.minimized),
        p: isPipWindow,
        x: geom.x,
        y: geom.y,
        w: geom.w,
        h: geom.h,
        min_w: w.minSize ? Math.round(w.minSize.width) : 0,
        min_h: w.minSize ? Math.round(w.minSize.height) : 0,
        sb: Boolean(w.__raven_strict_birth),
        iq: Boolean(w.__raven_quarantined),
        fs: Boolean(w.fullScreen),
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
 * Sincroniza de forma incremental el cambio de geometría o estado (delta sync) de una única ventana.
 *
 * @param {KWin::Window} w - Objeto de ventana modificado.
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
      ws: getWorkspaceId(w),
      desktops: deskIds,
      output: w.output ? w.output.name : "default",
      f: isFloating(w),
      m: Boolean(w.minimized),
      p: Boolean(w.keepAbove),
      x: geom.x,
      y: geom.y,
      w: geom.w,
      h: geom.h,
      min_w: w.minSize ? Math.round(w.minSize.width) : 0,
      min_h: w.minSize ? Math.round(w.minSize.height) : 0,
      sb: Boolean(w.__raven_strict_birth),
      iq: Boolean(w.__raven_quarantined),
      fs: Boolean(w.fullScreen),
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
      const outputs = workspace.screens || [];
      for (let i = 0; i < outputs.length; i++) {
        if (outputs[i].name === target_output_name) {
          workspace.sendClientToScreen(win, outputs[i]);
          break;
        }
      }
    }
    if (target_desktop_id) {
      const desktops = workspace.desktops || [];
      for (let j = 0; j < desktops.length; j++) {
        if (desktops[j].id.toString() === target_desktop_id) {
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
 * Procesa y aplica los comandos JSON recibidos desde el demonio (daemon) de Rust.
 *
 * @param {string} commandsJson - Carga de comandos serializada en JSON.
 */
function applyCommands(commandsJson) {
  if (!commandsJson) {
    return;
  }
  try {
    const cmds = JSON.parse(commandsJson);
    const windows = workspace.windowList();

    for (let i = 0; i < cmds.length; i++) {
      const cmd = cmds[i];
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
                }, 400);
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
          } else if (cmd.action === "saturation_warning") {
            Logger.warn("Saturation", "Pantalla cerca de saturación: " + cmd.active + "/" + cmd.cmax + " ventanas");
          } else if (cmd.action === "set_floating") {
            try {
              w.__raven_dynamic_float = Boolean(cmd.floating);
              w.keepAbove = Boolean(cmd.keep_above);
            } catch (e) {
              Logger.error("applyCommands", "Error asignando estado flotante dinámico", e);
            }
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
