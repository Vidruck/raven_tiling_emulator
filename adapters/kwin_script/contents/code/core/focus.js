/**
 * @file focus.js
 * @brief Resalte visual interactivo mediante el sistema de contorno (Outline) nativo de KWin.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/**
 * @brief Destaca momentáneamente una ventana proyectando el marco de contorno (Outline) del compositor.
 *
 * Utilizado al conmutar foco mediante atajos (`Meta+J` / `Meta+K`) para dar retroalimentación visual inmediata.
 * El contorno se desvanece automáticamente tras 200 ms.
 *
 * @param {KWin::Window} w Instancia de la ventana a destacar.
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

