/**
 * @file dbus_bridge.js
 * @brief Orquestador D-Bus ultraligero entre KWin y el demonio Raven en Rust.
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.0
 */

var _debounceTimer = null;

/**
 * @brief Solicita una sincronización global de estado agrupando eventos en 20ms.
 */
function requestStateSync() {
  try {
    if (!_debounceTimer) {
      _debounceTimer = new QTimer();
      _debounceTimer.interval = 20;
      _debounceTimer.singleShot = true;
      _debounceTimer.timeout.connect(syncState);
    }
    if (_debounceTimer.active) _debounceTimer.stop();
    _debounceTimer.start();
  } catch (e) {
    try { syncState(); } catch (err) { }
  }
}

/**
 * @brief Extrae la topología y lista de ventanas y la despacha a Rust.
 */
function syncState() {
  const windows = workspace.windowList();
  const winState = [];
  const screens = {};
  const outs = workspace.screens || [];
  const desks = workspace.desktops || [];
  const currentDesk = workspace.currentDesktop;
  const masterOutputs = [];
  const masterDesktops = [];

  try {
    for (let o = 0; o < outs.length; o++) {
      const out = outs[o];
      if (out && out.name) {
        masterOutputs.push(out.name.toString());
        if (desks.length > 0) {
          for (let d = 0; d < desks.length; d++) {
            const desk = desks[d];
            if (desk && desk.id) {
              screens[out.name + "||" + desk.id.toString()] = getSafeScreenGeometry(out, desk);
            }
          }
        } else {
          const deskId = currentDesk ? currentDesk.id.toString() : "default_desk";
          screens[out.name + "||" + deskId] = getSafeScreenGeometry(out, currentDesk);
        }
      }
    }
    for (let d = 0; d < desks.length; d++) {
      if (desks[d] && desks[d].id) masterDesktops.push(desks[d].id.toString());
    }
  } catch (e) { }

  for (let i = 0; i < windows.length; i++) {
    const w = windows[i];
    try {
      if (!isManageable(w)) continue;
      const safeId = getSafeWindowId(w);
      if (safeId) winState.push(buildWindowState(w, safeId));
    } catch (e) { }
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
        if (response && response !== "[]") applyCommands(response);
      }
    );
  } catch (e) {
    Logger.error("syncState", "Fallo enviando payload D-Bus", e);
  }
}

/**
 * @brief Sincroniza incrementalmente el estado de una ventana individual.
 */
function syncWindowDelta(w) {
  try {
    if (!w || w.deleted || w.minimized || !isManageable(w)) return;
    const safeId = getSafeWindowId(w);
    if (!safeId) return;

    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "syncWindowDelta",
      JSON.stringify(buildWindowState(w, safeId)),
      function (response) {
        if (response && response !== "[]") applyCommands(response);
      }
    );
  } catch (e) {
    Logger.error("syncWindowDelta", "Fallo en delta sync", e);
  }
}

/**
 * @brief Migra una ventana a un monitor o escritorio virtual.
 */
function migrateWindow(win, target_output_name, target_desktop_id) {
  if (!win || win.deleted) return;
  try {
    if (target_output_name) {
      const outputs = workspace.screens || [];
      for (let i = 0; i < outputs.length; i++) {
        if (outputs[i] && outputs[i].name === target_output_name) {
          try { if (typeof workspace.sendClientToScreen === "function") workspace.sendClientToScreen(win, outputs[i]); } catch (err) { }
          try { win.output = outputs[i]; } catch (errOut) { }
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
  } catch (e) { }
}

/**
 * @brief Comprueba si una ventana madre tiene actualmente un diálogo, confirmación o hija transitoria abierta.
 */
function hasActiveTransientOrModalChild(parentWin, allWindows) {
  if (!parentWin || parentWin.deleted) return false;
  const list = allWindows || workspace.windowList();
  for (let i = 0; i < list.length; i++) {
    const child = list[i];
    if (!child || child === parentWin || child.deleted || child.minimized || child.hidden) continue;
    if (child.transientFor === parentWin) {
      return true;
    }
    if (child.modal || child.dialog || child.transient) {
      if (child.transientFor === parentWin) return true;
      try {
        if (typeof child.mainWindows === "function") {
          const mains = child.mainWindows();
          if (mains && mains.indexOf(parentWin) !== -1) return true;
        } else if (child.mainWindows && child.mainWindows.indexOf && child.mainWindows.indexOf(parentWin) !== -1) {
          return true;
        }
      } catch (eMains) { }
    }
  }
  return false;
}

/**
 * @brief Aplica las acciones calculadas por Rust en KWin.
 */
function applyCommands(commandsJson) {
  if (!commandsJson) return;
  try {
    const cmds = JSON.parse(commandsJson);
    if (!cmds || !cmds.length) return;

    const windows = workspace.windowList();
    const winMap = {};
    for (let i = 0; i < windows.length; i++) {
      const w = windows[i];
      const id = getSafeWindowId(w);
      if (id) winMap[id] = w;
    }

    for (let i = 0; i < cmds.length; i++) {
      const cmd = cmds[i];
      if (cmd.action === "request_sync") {
        requestStateSync();
        continue;
      }
      if (cmd.action === "saturation_warning") {
        Logger.warn("Saturation", "Saturación: " + cmd.active + "/" + cmd.cmax);
        continue;
      }

      const w = cmd.window_id ? winMap[cmd.window_id] : null;
      if (!w || w.deleted) continue;

      switch (cmd.action) {
        case "set_floating":
          w.__raven_dynamic_float = Boolean(cmd.floating);
          w.keepAbove = Boolean(cmd.keep_above);
          break;

        case "set_keep_above":
          w.keepAbove = Boolean(cmd.keep_above);
          break;

        case "set_maximize":
          w.__raven_mutating = true;
          try {
            const shouldMax = Boolean(cmd.maximized !== undefined ? cmd.maximized : cmd.floating);
            w.setMaximize(shouldMax, shouldMax);
          } catch (eMax) { }
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 60);
          })(w);
          break;

        case "move":
        case "rectify_window":
          if (w.minimized || w.interactiveMove || w.interactiveResize || w.fullScreen) break;
          if (w.maximized || (w.maximizeMode !== undefined && w.maximizeMode === 3)) break;
          if (!w.normalWindow || w.transient || w.transientFor != null || w.modal || w.dialog || w.utility) break;
          if (hasActiveTransientOrModalChild(w, windows)) break;

          // Si la ventana tiene maximización parcial (ej. Teams con MAXIMIZED_VERT),
          // desmarcarla en KWin para que acepte libremente la nueva geometría
          if (w.maximizeMode !== undefined && w.maximizeMode !== 0) {
            try {
              w.setMaximize(false, false);
            } catch (eUnmax) { }
          }

          const targetGeom = {
            x: Math.round(cmd.x),
            y: Math.round(cmd.y),
            width: Math.round(cmd.width),
            height: Math.round(cmd.height),
          };

          w.__raven_mutating = true;
          w.frameGeometry = targetGeom;

          // Auditoría y verificación suave de convergencia:
          // Si el cliente Wayland CSD (Gecko, GTK, Electron) confirma su superficie inicial
          // milisegundos después y se queda en el centro, re-aplicamos targetGeom de inmediato.
          (function (cw, tgt, wid) {
            setKWinTimeout(function () {
              if (!cw || cw.deleted) return;
              if (cw.minimized) {
                cw.__raven_mutating = false;
                return;
              }
              try {
                var cur = cw.frameGeometry;
                if (cur && (Math.abs(cur.x - tgt.x) > 2 || Math.abs(cur.y - tgt.y) > 2 ||
                  Math.abs(cur.width - tgt.width) > 2 || Math.abs(cur.height - tgt.height) > 2)) {
                  cw.frameGeometry = tgt;
                }
              } catch (eReapply) { }

              try {
                var fgApplied = cw.frameGeometry || tgt;
                callDBus(
                  "org.kde.raven.Daemon",
                  "/Events",
                  "org.kde.raven.Events",
                  "commandAppliedState",
                  wid,
                  Math.round(fgApplied.x),
                  Math.round(fgApplied.y),
                  Math.round(fgApplied.width),
                  Math.round(fgApplied.height),
                  function (response) {
                    if (response && response !== "[]") {
                      applyCommands(response);
                    }
                  }
                );
              } catch (eAudit) { }

              cw.__raven_mutating = false;
            }, 60);
          })(w, targetGeom, cmd.window_id);
          break;

        case "focus":
          if (!w.minimized && workspace.activeWindow !== w &&
            (!workspace.activeWindow || (!workspace.activeWindow.popupWindow && !workspace.activeWindow.modal && !workspace.activeWindow.dialog && !workspace.activeWindow.transient && workspace.activeWindow.transientFor == null)) &&
            !hasActiveTransientOrModalChild(w, windows)) {
            workspace.activeWindow = w;
          }
          break;

        case "minimize":
          if (!w.minimized) {
            w.__raven_mutating = true;
            try {
              w.minimized = true;
            } catch (eMin) { }

            try {
              if (workspace.activeWindow === w) {
                var nextWin = null;
                for (var mi = 0; mi < windows.length; mi++) {
                  var cand = windows[mi];
                  if (cand && cand !== w && !cand.deleted && !cand.minimized && !cand.hidden && isManageable(cand)) {
                    nextWin = cand;
                    break;
                  }
                }
                if (nextWin) {
                  workspace.activeWindow = nextWin;
                }
              }
            } catch (eFoc) { }

            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                  requestStateSync();
                }
              }, 50);
            })(w);
          }
          break;

        case "unminimize":
          if (w.minimized) {
            w.__raven_mutating = true;
            try {
              w.minimized = false;
            } catch (eUnmin) { }
            (function (cw) {
              setKWinTimeout(function () {
                if (cw && !cw.deleted) {
                  cw.__raven_mutating = false;
                }
              }, 300);
            })(w);
          }
          break;

        case "close":
          try {
            if (typeof w.closeWindow === "function") {
              w.closeWindow();
            }
          } catch (eClose) { }
          break;

        case "migrate_to_output":
          w.__raven_mutating = true;
          migrateWindow(w, cmd.target_ws, null);
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 40);
          })(w);
          break;

        case "migrate_to_desktop":
          w.__raven_mutating = true;
          migrateWindow(w, null, cmd.target_ws);
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 40);
          })(w);
          break;
      }
    }
  } catch (e) {
    Logger.error("applyCommands", "Error procesando comandos", e);
  }
}

/**
 * @brief Enlaza reactivamente los eventos del ciclo de vida de una ventana.
 */
function bindWindow(w) {
  try {
    if (!isManageable(w) || w.__raven_bound) return;
    w.__raven_bound = true;

    const onStateChange = function () {
      if (w && !w.deleted && !w.__raven_mutating && !w.interactiveMove && !w.interactiveResize) {
        requestStateSync();
      }
    };

    w.minimizedChanged.connect(onStateChange);
    w.maximizedChanged.connect(onStateChange);
    if (w.fullScreenChanged !== undefined) w.fullScreenChanged.connect(onStateChange);
    if (w.transientChanged !== undefined) {
      w.transientChanged.connect(function () {
        delete w.__raven_manageable;
        requestStateSync();
      });
    }
    if (w.modalChanged !== undefined) {
      w.modalChanged.connect(function () {
        delete w.__raven_manageable;
        requestStateSync();
      });
    }

    if (w.captionChanged !== undefined) {
      w.captionChanged.connect(function () {
        delete w.__raven_manageable;
        if (w && !w.deleted && !w.__raven_mutating) {
          syncWindowDelta(w);
        }
      });
    }

    const onOutputOrDesktop = function () {
      if (!w || w.deleted || w.__raven_mutating) return;
      if (!w.interactiveMove && !w.interactiveResize) {
        w.__raven_ui_migrating = true;
        (function (cw) {
          setKWinTimeout(function () {
            if (cw && !cw.deleted) cw.__raven_ui_migrating = false;
          }, 60);
        })(w);
      }
      requestStateSync();
    };

    w.outputChanged.connect(onOutputOrDesktop);
    w.desktopsChanged.connect(onOutputOrDesktop);

    w.frameGeometryChanged.connect(function () {
      if (!w || w.deleted || w.minimized) return;
      if (w.interactiveMove || w.interactiveResize) {
        w.__was_interacting = true;
        return;
      }
      if (w.__was_interacting && !w.interactiveMove && !w.interactiveResize) {
        w.__was_interacting = false;
        requestStateSync();
        return;
      }
      if (w.__raven_mutating || w.__raven_ui_migrating) return;
      if (hasActiveTransientOrModalChild(w, null)) return;

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
    Logger.error("bindWindow", "Error enlazando ventana", e);
  }
}

/**
 * @brief Extrae y normaliza el estado espacial de una ventana.
 */
function buildWindowState(w, safeId) {
  const geom = getRectGeometry(w.frameGeometry);
  const deskIds = [];
  if (w.desktops) {
    for (let d = 0; d < w.desktops.length; d++) deskIds.push(w.desktops[d].id.toString());
  }
  const output = w.output || workspace.activeOutput;
  return {
    id: safeId,
    desktops: deskIds,
    output: output ? output.name : "default",
    f: isWindowNativeFloating(w),
    m: Boolean(w.minimized),
    p: Boolean(w.keepAbove),
    x: geom.x,
    y: geom.y,
    w: geom.w,
    h: geom.h,
    min_w: w.minSize ? Math.round(w.minSize.width) : 0,
    min_h: w.minSize ? Math.round(w.minSize.height) : 0,
    fs: Boolean(w.fullScreen),
    max: Boolean(w.maximized || (w.maximizeMode !== undefined && w.maximizeMode === 3)),
    cls: w.resourceClass ? w.resourceClass.toString() : "",
    cls_name: w.resourceName ? w.resourceName.toString() : "",
    cap: w.caption ? w.caption.toString() : "",
  };
}
