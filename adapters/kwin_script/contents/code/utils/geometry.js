/**
 * @file geometry.js
 * @brief Funciones auxiliares de cálculo y normalización geométrica de pantallas y ventanas en KWin.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

/**
 * @brief Normaliza un rectángulo nativo de Qt/KWin a un objeto de coordenadas enteras estándar.
 *
 * Resuelve polimorfismos entre métodos de acceso `x()`/`width()` y propiedades directas `x`/`width`.
 *
 * @param {QtRect|Object} rect Estructura geométrica provista por el compositor.
 * @returns {{x: number, y: number, w: number, h: number}} Objeto normalizado con enteros redondeados.
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

function getSafeScreenGeometry(output, desktop) {
  if (!output) {
    return { x: 0, y: 0, w: 1920, h: 1080 };
  }

  // 1. Intentar clientArea con PlacementArea / MaximizeArea
  // En KWin 6 (QJSEngine), clientArea(ClientAreaOption, Output, VirtualDesktop)
  // KWin.PlacementArea = 0 (área de colocación libre excluyendo paneles)
  // KWin.MaximizeArea = 1 (área máxima para ventanas maximizadas excluyendo paneles y struts)
  var optionsToTry = [];
  if (typeof KWin !== "undefined") {
    if (KWin.PlacementArea !== undefined) optionsToTry.push(KWin.PlacementArea);
    if (KWin.MaximizeArea !== undefined) optionsToTry.push(KWin.MaximizeArea);
  }
  optionsToTry.push(0);
  optionsToTry.push(1);

  for (var i = 0; i < optionsToTry.length; i++) {
    try {
      var opt = optionsToTry[i];
      var area = workspace.clientArea(opt, output, desktop);
      if (area) {
        var geom = getRectGeometry(area);
        if (geom.w > 0 && geom.h > 0) {
          return geom;
        }
      }
    } catch (e) { }
  }

  // 2. Fallback a geometry del output físico si clientArea aún no está disponible
  try {
    if (output.geometry) {
      var outGeom = getRectGeometry(output.geometry);
      if (outGeom.w > 0 && outGeom.h > 0) {
        return outGeom;
      }
    }
  } catch (e) { }

  return { x: 0, y: 0, w: 1920, h: 1080 };
}
