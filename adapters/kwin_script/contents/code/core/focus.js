/**
 * @file focus.js
 * @brief Gestión de retroalimentación de foco delegada al efecto nativo C++ (kwin4_effect_raven).
 * @author Alejandro González Hernández (Vidruck)
 * @version 5.1
 */

/**
 * @brief Limpia cualquier contorno residual del compositor sin instanciar temporizadores.
 *
 * La proyección visual de bordes activos e inactivos se realiza por hardware en `BorderManager` (C++).
 *
 * @param {KWin::Window} w Instancia de la ventana enfocada.
 */
function highlightWindow(w) {
  try {
    if (!w) return;
    if (workspace.hideOutline) {
      workspace.hideOutline();
    }
  } catch (e) { }
}


