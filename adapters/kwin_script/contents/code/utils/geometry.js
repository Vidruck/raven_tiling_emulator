/**
 * @fileoverview Funciones auxiliares de geometría y cálculo de coordenadas para KWin.
 */

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

  function getProp(obj, p1, p2, def) {
    if (typeof obj[p1] === "function") return obj[p1]();
    if (obj[p1] !== undefined) return obj[p1];
    if (typeof obj[p2] === "function") return obj[p2]();
    if (obj[p2] !== undefined) return obj[p2];
    return def;
  }

  return {
    x: Math.round(getProp(rect, "x", "x", 0)),
    y: Math.round(getProp(rect, "y", "y", 0)),
    w: Math.round(getProp(rect, "width", "w", 1920)),
    h: Math.round(getProp(rect, "height", "h", 1080)),
  };
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
  } catch (e) { }
  try {
    if (output.geometry) {
      return getRectGeometry(output.geometry);
    }
  } catch (e) { }
  return { x: 0, y: 0, w: 1920, h: 1080 };
}
