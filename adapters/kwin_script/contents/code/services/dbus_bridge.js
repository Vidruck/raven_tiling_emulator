/**
 * @file dbus_bridge.js
 * @brief Orquestador D-Bus ultraligero entre KWin y el demonio Raven en Rust.
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.0
 */

var _debounceTimer = null;

/**
 * @brief Solicita una sincronización global de estado agrupando eventos en 40ms.
 */
function requestStateSync() {
  try {
    if (!_debounceTimer) {
      _debounceTimer = new QTimer();
      _debounceTimer.interval = 40;
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
        const deskId = currentDesk ? currentDesk.id.toString() : "default_desk";
        screens[out.name + "||" + deskId] = getSafeScreenGeometry(out, currentDesk);
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
    if (!w || w.deleted || !isManageable(w)) return;
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
          try { if (typeof workspace.sendClientToScreen === "function") workspace.sendClientToScreen(win, outputs[i]); } catch (err) {}
          try { win.output = outputs[i]; } catch (errOut) {}
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

        case "set_maximize":
          w.__raven_mutating = true;
          try {
            const shouldMax = Boolean(cmd.maximized !== undefined ? cmd.maximized : cmd.floating);
            w.setMaximize(shouldMax, shouldMax);
          } catch (eMax) {}
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
          if (w.maximized || (w.maximizeMode !== undefined && w.maximizeMode !== 0)) break;
          if (w.transientChildren && w.transientChildren.length > 0) break;

          const targetGeom = {
            x: Math.round(cmd.x),
            y: Math.round(cmd.y),
            width: Math.round(cmd.width),
            height: Math.round(cmd.height),
          };

          const curFg = w.frameGeometry;
          if (cmd.action === "move" && curFg &&
              Math.round(curFg.x) === targetGeom.x &&
              Math.round(curFg.y) === targetGeom.y &&
              Math.round(curFg.width) === targetGeom.width &&
              Math.round(curFg.height) === targetGeom.height) {
            break;
          }

          w.__raven_mutating = true;
          w.frameGeometry = targetGeom;

          // Auditoría de geometría aplicada
          try {
            const fgApplied = w.frameGeometry || targetGeom;
            callDBus(
              "org.kde.raven.Daemon",
              "/Events",
              "org.kde.raven.Events",
              "commandAppliedState",
              cmd.window_id,
              Math.round(fgApplied.x),
              Math.round(fgApplied.y),
              Math.round(fgApplied.width),
              Math.round(fgApplied.height)
            );
          } catch (eAudit) {}

          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) cw.__raven_mutating = false;
            }, 60);
          })(w);
          break;

        case "focus":
          if (workspace.activeWindow !== w && (!workspace.activeWindow || !workspace.activeWindow.popupWindow)) {
            workspace.activeWindow = w;
          }
          break;

        case "minimize":
          w.__raven_mutating = true;
          w.minimized = true;
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 60);
          })(w);
          break;

        case "unminimize":
          w.__raven_mutating = true;
          w.minimized = false;
          (function (cw) {
            setKWinTimeout(function () {
              if (cw && !cw.deleted) {
                cw.__raven_mutating = false;
                requestStateSync();
              }
            }, 60);
          })(w);
          break;

        case "close":
          try {
            if (typeof w.closeWindow === "function") {
              w.closeWindow();
            }
          } catch (eClose) {}
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

        case "release_quarantine":
        case "request_feedback":
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

    if (w.captionChanged !== undefined) {
      w.captionChanged.connect(function () {
        if (w && !w.deleted && !w.__raven_mutating) {
          const cap = w.caption ? w.caption.toString().toLowerCase() : "";
          if (PIP_CAPTION_REGEX.test(cap) || FLOATING_CAPTION_REGEX.test(cap)) requestStateSync();
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
      if (!w || w.deleted) return;
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
      if (w.transientChildren && w.transientChildren.length > 0) return;

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
    f: isFloating(w),
    m: Boolean(w.minimized),
    p: false,
    x: geom.x,
    y: geom.y,
    w: geom.w,
    h: geom.h,
    min_w: w.minSize ? Math.round(w.minSize.width) : 0,
    min_h: w.minSize ? Math.round(w.minSize.height) : 0,
    sb: false,
    iq: false,
    fs: Boolean(w.fullScreen),
    max: Boolean(w.maximized || (w.maximizeMode !== undefined && w.maximizeMode !== 0)),
    sus: false,
    cls: w.resourceClass ? w.resourceClass.toString() : "",
    cls_name: w.resourceName ? w.resourceName.toString() : "",
    cap: w.caption ? w.caption.toString() : "",
  };
}
