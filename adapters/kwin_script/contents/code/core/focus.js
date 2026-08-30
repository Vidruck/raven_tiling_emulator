/**
 * @fileoverview Resalte visual con el sistema de Outline de KWin.
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

