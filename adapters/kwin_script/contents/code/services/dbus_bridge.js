/**
 * @file dbus_bridge.js
 * @brief Orquestador D-Bus ultraligero y libre de temporizadores entre KWin y el demonio Raven en Rust.
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.1
 */

var _isApplyingCommands = false;
var _lastSyncedPayloadJson = "";
var _lastDeltaMap = {};

/**
 * @brief Solicita una sincronización global de estado de forma inmediata (sin QTimers en KWin).
 * La coalescencia de ráfagas y estabilización se ejecuta de forma asíncrona en el Actor de Rust.
 */
function requestStateSync(force) {
  if (_isApplyingCommands && !force) return;
  try {
    syncState(Boolean(force));
  } catch (err) { }
}

/**
 * @brief Limpia el caché de deltas para una ventana cerrada garantizando memoria acotada.
 */
function clearWindowDeltaCache(safeId) {
  if (safeId && _lastDeltaMap[safeId] !== undefined) {
    delete _lastDeltaMap[safeId];
  }
}

/**
 * @brief Extrae la topología y lista de ventanas y la despacha a Rust si hubo cambios reales.
 */
function syncState(force) {
  const windows = workspace.windowList();
  const winState = [];
  const activeIds = {};
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
      if (safeId) {
        activeIds[safeId] = true;
        winState.push(buildWindowState(w, safeId));
      }
    } catch (e) { }
  }

  // Podar entradas obsoletas en _lastDeltaMap para garantizar memoria O(N) estricta
  const cachedKeys = Object.keys(_lastDeltaMap);
  if (cachedKeys.length > winState.length + 16) {
    for (let k = 0; k < cachedKeys.length; k++) {
      if (!activeIds[cachedKeys[k]]) {
        delete _lastDeltaMap[cachedKeys[k]];
      }
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
    const payloadJson = JSON.stringify(payload);
    if (!force && payloadJson === _lastSyncedPayloadJson) {
      return;
    }
    _lastSyncedPayloadJson = payloadJson;

    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "syncStateAndUpdateLayout",
      payloadJson,
      function (response) {
        if (response && response !== "[]") applyCommands(response);
      }
    );
  } catch (e) {
    Logger.error("syncState", "Fallo enviando payload D-Bus", e);
  }
}

/**
 * @brief Sincroniza incrementalmente el estado de una ventana individual con deduplicación.
 */
function syncWindowDelta(w) {
  try {
    if (!w || w.deleted || w.minimized || !isManageable(w)) return;
    const safeId = getSafeWindowId(w);
    if (!safeId) return;

    const deltaJson = JSON.stringify(buildWindowState(w, safeId));
    if (_lastDeltaMap[safeId] === deltaJson) {
      return;
    }
    _lastDeltaMap[safeId] = deltaJson;

    callDBus(
      "org.kde.raven.Daemon",
      "/Events",
      "org.kde.raven.Events",
      "syncWindowDelta",
      deltaJson,
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
 * @brief Comprueba si una ventana se encuentra actualmente en redimensionamiento interactivo con el ratón.
 */
function isWindowResizing(w) {
  if (!w || w.deleted) return false;
  return Boolean(w.resize || w.interactiveResize || w.__was_resizing);
}

/**
 * @brief Comprueba si una ventana se encuentra actualmente en arrastre interactivo con el ratón.
 */
function isWindowMoving(w) {
  if (!w || w.deleted) return false;
  return Boolean(w.move || w.interactiveMove || w.__was_moving);
}

/**
 * @brief Aplica las acciones calculadas por Rust en KWin sin necesidad de temporizadores.
 *
 * @param commandsJson Cadena JSON con el lote de acciones generadas por el demonio Rust.
 * @param activeResizingId Identificador de la ventana actualmente sujeta por el cursor (si aplica)
 *                         para blindarla contra sobreescrituras mientras las ventanas vecinas reaccionan.
 */
function applyCommands(commandsJson, activeResizingId) {
  if (!commandsJson) return;
  var needsSyncAfterBatch = false;
  var forceSyncAfterBatch = false;
  _isApplyingCommands = true;
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
        needsSyncAfterBatch = true;
        forceSyncAfterBatch = true;
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
          if (w.__raven_dynamic_float) {
            delete w.__raven_target_geom;
          }
          break;

        case "set_keep_above":
          w.keepAbove = Boolean(cmd.keep_above);
          break;

        case "set_maximize":
          w.__raven_mutating = true;
          try {
            const shouldMax = Boolean(cmd.maximized !== undefined ? cmd.maximized : cmd.floating);
            if (shouldMax) {
              delete w.__raven_target_geom;
            }
            w.setMaximize(shouldMax, shouldMax);
            needsSyncAfterBatch = true;
          } catch (eMax) {
          } finally {
            w.__raven_mutating = false;
          }
          break;

        case "move":
        case "rectify_window":
          const safeId = getSafeWindowId(w);
          // BLINDAJE ANTI-JALONEO: Si esta ventana está siendo manipulada directamente por el usuario
          // con el ratón, NUNCA sobreescribir su geometría para evitar la lucha contra el cursor.
          // Las ventanas vecinas sí recibirán su nuevo tamaño y posición para reaccionar en tiempo real.
          if (
            isWindowResizing(w) ||
            isWindowMoving(w) ||
            (activeResizingId && safeId === activeResizingId) ||
            w.minimized ||
            w.fullScreen
          ) {
            break;
          }
          if (w.maximized || (w.maximizeMode !== undefined && w.maximizeMode === 3)) break;
          if (!w.normalWindow || w.transient || w.transientFor != null || w.modal || w.dialog || w.utility) break;
          if (hasActiveTransientOrModalChild(w, windows)) break;

          // Si la ventana tiene maximización parcial (ej. Teams con MAXIMIZED_VERT)
          // o anclaje de mosaico nativo de Plasma (QuickTile / Tile), desmarcarlos
          // para que Raven controle la geometría sin interferencias del compositor
          if (w.maximizeMode !== undefined && w.maximizeMode !== 0) {
            try {
              w.setMaximize(false, false);
            } catch (eUnmax) { }
          }
          if (w.tile) {
            try {
              w.tile = null;
            } catch (eTile) { }
          }
          if (w.quickTileMode !== undefined && w.quickTileMode !== 0) {
            try {
              w.quickTileMode = 0;
            } catch (eQt) { }
          }

          const targetGeom = {
            x: Math.round(cmd.x),
            y: Math.round(cmd.y),
            width: Math.round(cmd.width),
            height: Math.round(cmd.height),
          };

          w.__raven_target_geom = targetGeom;
          w.__raven_mutating = true;
          try {
            w.frameGeometry = targetGeom;
          } finally {
            w.__raven_mutating = false;
          }
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
              needsSyncAfterBatch = true;
            } catch (eMin) {
            } finally {
              w.__raven_mutating = false;
            }

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
          }
          break;

        case "unminimize":
          if (w.minimized) {
            w.__raven_mutating = true;
            try {
              w.minimized = false;
            } catch (eUnmin) {
            } finally {
              w.__raven_mutating = false;
            }
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
          try {
            delete w.__raven_target_geom;
            migrateWindow(w, cmd.target_ws, null);
            needsSyncAfterBatch = true;
          } finally {
            w.__raven_mutating = false;
          }
          break;

        case "migrate_to_desktop":
          w.__raven_mutating = true;
          try {
            delete w.__raven_target_geom;
            migrateWindow(w, null, cmd.target_ws);
            needsSyncAfterBatch = true;
          } finally {
            w.__raven_mutating = false;
          }
          break;
      }
    }
  } catch (e) {
    Logger.error("applyCommands", "Error procesando comandos", e);
  } finally {
    _isApplyingCommands = false;
  }

  if (needsSyncAfterBatch) {
    requestStateSync(forceSyncAfterBatch);
  }
}

/**
 * @brief Determina el hueco (slot) geométrico del algoritmo activo bajo el punto de arrastre `(dropX, dropY)`.
 *
 * Evalúa todas las ventanas gestionables del escritorio actual usando sus geometrías de mosaico
 * objetivo (`__raven_target_geom` o `__raven_pre_drag_geom`) para dibujar en tiempo real el contorno
 * del algoritmo activo mediante `workspace.showOutline`.
 */
function findTargetSlotGeometry(draggedWin, dropX, dropY) {
  try {
    const windows = workspace.windowList();
    const currentDesk = workspace.currentDesktop;
    const currentDeskId = currentDesk && currentDesk.id ? currentDesk.id.toString() : "";

    let bestSlot = null;
    let bestDistSq = Number.MAX_VALUE;

    for (let i = 0; i < windows.length; i++) {
      const cand = windows[i];
      if (!cand || cand.deleted || cand.minimized || cand.hidden || !isManageable(cand)) continue;
      if (isWindowNativeFloating(cand)) continue;

      if (currentDeskId && cand.desktops && cand.desktops.length > 0) {
        let onDesk = false;
        for (let d = 0; d < cand.desktops.length; d++) {
          if (cand.desktops[d] && cand.desktops[d].id.toString() === currentDeskId) {
            onDesk = true;
            break;
          }
        }
        if (!onDesk) continue;
      }

      let slot = null;
      if (cand === draggedWin) {
        slot = cand.__raven_pre_drag_geom || cand.__raven_target_geom;
      } else {
        slot = cand.__raven_target_geom;
      }
      if (!slot) {
        const cg = getRectGeometry(cand.frameGeometry);
        slot = { x: cg.x, y: cg.y, width: cg.w, height: cg.h };
      }

      if (
        dropX >= slot.x &&
        dropX <= slot.x + slot.width &&
        dropY >= slot.y &&
        dropY <= slot.y + slot.height
      ) {
        return slot;
      }

      const cx = slot.x + slot.width / 2;
      const cy = slot.y + slot.height / 2;
      const distSq = (dropX - cx) * (dropX - cx) + (dropY - cy) * (dropY - cy);
      if (distSq < bestDistSq) {
        bestDistSq = distSq;
        bestSlot = slot;
      }
    }

    if (bestSlot) {
      return bestSlot;
    }

    // Si se arrastra hacia un monitor vacío, proyectar el área útil completa de ese monitor
    const outs = workspace.screens || [];
    for (let o = 0; o < outs.length; o++) {
      const out = outs[o];
      if (!out) continue;
      const area = getSafeScreenGeometry(out, currentDesk);
      if (
        dropX >= area.x &&
        dropX <= area.x + area.width &&
        dropY >= area.y &&
        dropY <= area.y + area.height
      ) {
        return area;
      }
    }
  } catch (eSlot) { }
  return null;
}

/**
 * @brief Finaliza una operación interactiva de arrastre o redimensionado con el ratón.
 *
 * Oculta el contorno visual de previsualización, anula cualquier intento de mosaico nativo de Plasma
 * y notifica al motor Rust las nuevas proporciones (`userResizedWindow`) o el punto de caída
 * para intercambio de ranuras en tiempo real (`userMovedWindow`).
 */
function finishInteractiveMoveResize(w) {
  if (!w || w.deleted) return;
  const wasMoving = Boolean(w.__was_moving || isWindowMoving(w));
  const wasResizing = Boolean(w.__was_resizing || isWindowResizing(w));
  w.__was_interacting = false;
  w.__was_moving = false;
  w.__was_resizing = false;
  delete w.__last_resize_step;
  delete w.__last_outline_key;
  delete w.__raven_pre_drag_geom;
  delete w.__raven_target_geom;

  try {
    if (typeof workspace.hideOutline === "function") {
      workspace.hideOutline();
    }
  } catch (eHide) { }

  if (!isWindowNativeFloating(w)) {
    if (w.tile) {
      w.__raven_mutating = true;
      try { w.tile = null; } catch (eTile) { }
      w.__raven_mutating = false;
    }
    if (w.quickTileMode !== undefined && w.quickTileMode !== 0) {
      w.__raven_mutating = true;
      try { w.quickTileMode = 0; } catch (eQt) { }
      w.__raven_mutating = false;
    }
  }

  const safeId = getSafeWindowId(w);
  const g = getRectGeometry(w.frameGeometry);

  if (wasResizing && safeId && !isWindowNativeFloating(w)) {
    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "userResizedWindow",
        safeId,
        Math.round(g.x),
        Math.round(g.y),
        Math.round(g.w),
        Math.round(g.h),
        function (response) {
          if (response && response !== "[]") applyCommands(response, safeId);
        }
      );
      return;
    } catch (eResFin) { }
  }

  if (wasMoving && safeId && !isWindowNativeFloating(w)) {
    let dropX = g.x + g.w / 2;
    let dropY = g.y + g.h / 2;
    try {
      if (workspace.cursorPos && workspace.cursorPos.x !== undefined && workspace.cursorPos.y !== undefined) {
        dropX = workspace.cursorPos.x;
        dropY = workspace.cursorPos.y;
      }
    } catch (eCur) { }

    try {
      callDBus(
        "org.kde.raven.Daemon",
        "/Events",
        "org.kde.raven.Events",
        "userMovedWindow",
        safeId,
        Math.round(dropX),
        Math.round(dropY),
        function (response) {
          if (response && response !== "[]") applyCommands(response, safeId);
        }
      );
      return;
    } catch (eMovFin) { }
  }

  requestStateSync(true);
}

/**
 * @brief Gestiona el paso interactivo (arrastre o redimensionado) con el ratón.
 */
function handleInteractiveStep(w, geom) {
  if (!w || w.deleted || isWindowNativeFloating(w)) return;
  const g = getRectGeometry(geom || w.frameGeometry);

  if (isWindowResizing(w)) {
    w.__was_interacting = true;
    w.__was_resizing = true;
    delete w.__raven_target_geom;

    if (!_isApplyingCommands) {
      const lastStep = w.__last_resize_step;
      if (
        !lastStep ||
        Math.abs(g.w - lastStep.w) >= 8 ||
        Math.abs(g.h - lastStep.h) >= 8 ||
        Math.abs(g.x - lastStep.x) >= 8 ||
        Math.abs(g.y - lastStep.y) >= 8
      ) {
        w.__last_resize_step = { x: g.x, y: g.y, w: g.w, h: g.h };
        const safeId = getSafeWindowId(w);
        if (safeId) {
          try {
            callDBus(
              "org.kde.raven.Daemon",
              "/Events",
              "org.kde.raven.Events",
              "userResizedWindow",
              safeId,
              Math.round(g.x),
              Math.round(g.y),
              Math.round(g.w),
              Math.round(g.h),
              function (response) {
                if (response && response !== "[]") {
                  applyCommands(response, safeId);
                }
              }
            );
          } catch (eLiveRes) { }
        }
      }
    }
    return;
  }

  if (isWindowMoving(w)) {
    if (!w.__was_moving) {
      w.__was_moving = true;
      if (w.__raven_target_geom) {
        w.__raven_pre_drag_geom = {
          x: w.__raven_target_geom.x,
          y: w.__raven_target_geom.y,
          width: w.__raven_target_geom.width,
          height: w.__raven_target_geom.height,
        };
      } else {
        w.__raven_pre_drag_geom = {
          x: g.x,
          y: g.y,
          width: g.w,
          height: g.h,
        };
      }
    }
    w.__was_interacting = true;
    delete w.__raven_target_geom;

    // Dibujar el contorno de la ranura destino del algoritmo activo en tiempo real
    if (typeof workspace.showOutline === "function") {
      let dropX = g.x + g.w / 2;
      let dropY = g.y + g.h / 2;
      try {
        if (workspace.cursorPos && workspace.cursorPos.x !== undefined && workspace.cursorPos.y !== undefined) {
          dropX = workspace.cursorPos.x;
          dropY = workspace.cursorPos.y;
        }
      } catch (eCur) { }

      const slot = findTargetSlotGeometry(w, dropX, dropY);
      if (slot) {
        const key = slot.x + ":" + slot.y + ":" + slot.width + ":" + slot.height;
        if (w.__last_outline_key !== key) {
          w.__last_outline_key = key;
          try {
            workspace.showOutline(slot);
          } catch (eOut) { }
        }
      }
    }
  }
}

/**
 * @brief Enlaza reactivamente los eventos del ciclo de vida e interacción de una ventana.
 */
function bindWindow(w) {
  try {
    if (!isManageable(w) || w.__raven_bound) return;
    w.__raven_bound = true;

    const onStateChange = function () {
      if (w && !w.deleted && !w.__raven_mutating && !_isApplyingCommands && !isWindowMoving(w) && !isWindowResizing(w)) {
        requestStateSync(false);
      }
    };

    w.minimizedChanged.connect(onStateChange);
    w.maximizedChanged.connect(onStateChange);
    if (w.fullScreenChanged !== undefined) w.fullScreenChanged.connect(onStateChange);
    if (w.transientChanged !== undefined) {
      w.transientChanged.connect(function () {
        delete w.__raven_manageable;
        requestStateSync(false);
      });
    }
    if (w.modalChanged !== undefined) {
      w.modalChanged.connect(function () {
        delete w.__raven_manageable;
        requestStateSync(false);
      });
    }

    // Suprimir el mosaico nativo de Plasma (QuickTiling / Custom Tiles) en ventanas gestionadas por Raven
    const suppressNativeTile = function () {
      if (!w || w.deleted || w.__raven_mutating || isWindowNativeFloating(w)) return;
      if (w.tile) {
        w.__raven_mutating = true;
        try { w.tile = null; } catch (eTile) { }
        w.__raven_mutating = false;
      }
      if (w.quickTileMode !== undefined && w.quickTileMode !== 0) {
        w.__raven_mutating = true;
        try { w.quickTileMode = 0; } catch (eQt) { }
        w.__raven_mutating = false;
      }
    };
    if (w.tileChanged !== undefined) {
      w.tileChanged.connect(suppressNativeTile);
    }
    if (w.quickTileModeChanged !== undefined) {
      w.quickTileModeChanged.connect(suppressNativeTile);
    }

    if (w.captionChanged !== undefined) {
      w.captionChanged.connect(function () {
        if (!w || w.deleted || w.__raven_mutating || _isApplyingCommands) return;
        // Solo actuar si la ventana conmuta al modo Picture-in-Picture
        if (isPipWindow(w) && !w.__raven_pip_init) {
          w.__raven_pip_init = true;
          w.keepAbove = true;
          requestStateSync(false);
          return;
        }
        // Los cambios de título habituales (navegador, terminal, música)
        // no alteran la geometría del mosaico ni requieren saturar D-Bus.
      });
    }

    const onOutputOrDesktop = function () {
      if (!w || w.deleted || w.__raven_mutating || _isApplyingCommands) return;
      delete w.__raven_target_geom;
      requestStateSync(false);
    };

    w.outputChanged.connect(onOutputOrDesktop);
    w.desktopsChanged.connect(onOutputOrDesktop);

    if (w.moveResizedChanged !== undefined) {
      w.moveResizedChanged.connect(function () {
        if (!w || w.deleted) return;
        const resizing = isWindowResizing(w);
        const moving = isWindowMoving(w);
        if (resizing) {
          w.__was_resizing = true;
          w.__was_interacting = true;
          delete w.__raven_target_geom;
        } else if (moving) {
          w.__was_moving = true;
          w.__was_interacting = true;
          delete w.__raven_target_geom;
        } else if (w.__was_interacting) {
          finishInteractiveMoveResize(w);
        }
      });
    }

    if (w.interactiveMoveResizeStarted !== undefined) {
      w.interactiveMoveResizeStarted.connect(function () {
        if (!w || w.deleted) return;
        delete w.__raven_target_geom;
        if (isWindowResizing(w)) w.__was_resizing = true;
        if (isWindowMoving(w)) w.__was_moving = true;
        w.__was_interacting = true;
      });
    }

    if (w.interactiveMoveResizeStepped !== undefined) {
      w.interactiveMoveResizeStepped.connect(function (stepGeom) {
        if (!w || w.deleted) return;
        handleInteractiveStep(w, stepGeom);
      });
    }

    w.frameGeometryChanged.connect(function () {
      if (!w || w.deleted || w.minimized) return;

      const resizing = isWindowResizing(w);
      const moving = isWindowMoving(w);

      if (resizing || moving) {
        handleInteractiveStep(w, w.frameGeometry);
        return;
      }

      if (w.__was_interacting && !resizing && !moving) {
        finishInteractiveMoveResize(w);
        return;
      }

      if (w.__raven_mutating || _isApplyingCommands) return;
      if (hasActiveTransientOrModalChild(w, null)) return;

      // Supresión de eco determinista libre de temporizadores:
      // Si el cambio de geometría reportado por el cliente Wayland converge con
      // el objetivo comandado por Raven (tolerancia <= 4px pos, <= 12px dim),
      // se ignora el eco sin disparar llamadas D-Bus innecesarias.
      if (w.__raven_target_geom) {
        const g = getRectGeometry(w.frameGeometry);
        const tg = w.__raven_target_geom;
        if (
          Math.abs(g.x - tg.x) <= 4 &&
          Math.abs(g.y - tg.y) <= 4 &&
          Math.abs(g.w - tg.width) <= 12 &&
          Math.abs(g.h - tg.height) <= 12
        ) {
          return;
        }
      }

      syncWindowDelta(w);
    });

    if (w.interactiveMoveResizeFinished !== undefined) {
      w.interactiveMoveResizeFinished.connect(function () {
        if (w && !w.deleted) {
          finishInteractiveMoveResize(w);
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
