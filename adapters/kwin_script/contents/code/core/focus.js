/**
 * @fileoverview Algoritmo de navegación y foco direccional nativo con resalte visual.
 */

/**
 * Destaca visualmente una ventana usando el Outline de KWin.
 *
 * @param {KWin::Window} w - Ventana a destacar.
 */
function highlightWindow(w) {
  try {
    if (!w) return;
    if (workspace.showOutline) {
      workspace.showOutline(w.frameGeometry);
      setKWinTimeout(function() {
        if (workspace.hideOutline) workspace.hideOutline();
      }, 200);
    }
  } catch (e) { }
}

/**
 * Foco direccional nativo utilizando geometría de KWin.
 *
 * @param {number} dx - Dirección en el eje X (-1 izquierda, 1 derecha, 0 neutro).
 * @param {number} dy - Dirección en el eje Y (-1 arriba, 1 abajo, 0 neutro).
 */
function focusDirection(dx, dy) {
  try {
    const activeWin = workspace.activeWindow;
    if (!activeWin) return;

    const actRect = getRectGeometry(activeWin.frameGeometry);
    const cx = actRect.x + actRect.w / 2;
    const cy = actRect.y + actRect.h / 2;

    let bestWin = null;
    let bestDist = Infinity;

    const windows = workspace.windowList();
    for (let i = 0; i < windows.length; i++) {
      const w = windows[i];
      if (w === activeWin || !isManageable(w) || w.minimized) continue;

      const desktopMatch = isSameDesktop(w, activeWin);
      if (!desktopMatch && (!w.onAllDesktops && !activeWin.onAllDesktops)) continue;

      const r = getRectGeometry(w.frameGeometry);
      const wx = r.x + r.w / 2;
      const wy = r.y + r.h / 2;

      // Filtrado según dirección solicitada
      if (dx > 0 && wx <= cx) continue;
      if (dx < 0 && wx >= cx) continue;
      if (dy > 0 && wy <= cy) continue;
      if (dy < 0 && wy >= cy) continue;

      const dist = Math.pow(wx - cx, 2) + Math.pow(wy - cy, 2);
      if (dist < bestDist) {
        bestDist = dist;
        bestWin = w;
      }
    }

    if (bestWin) {
      workspace.activeWindow = bestWin;
      highlightWindow(bestWin);
    }
  } catch (e) {
    Logger.error("focusDirection", "Error enfocando dirección " + dx + "," + dy, e);
  }
}
