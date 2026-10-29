#!/bin/bash
# build_kwin_bundle.sh
# Genera el archivo main.js (bundle monolítico) para KWin Script
# a partir de los módulos individuales en el directorio code/.
#
# Uso: bash build_kwin_bundle.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CODE_DIR="${SCRIPT_DIR}/adapters/kwin_script/contents/code"
OUTPUT="${CODE_DIR}/main.js"

echo "🐦 Raven Bridge Builder — Generando bundle main.js..."

# Orden de concatenación (resolviendo dependencias, libre de QTimers):
# 1. Logger (sin dependencias)
# 2. Geometry (sin dependencias internas)
# 3. Window Utils (depende de Logger)
# 4. Quarantine (depende de Window Utils)
# 5. Focus (depende de Geometry, Window Utils)
# 6. D-Bus Bridge (depende de todo lo anterior)
# 7. Shortcuts (depende de D-Bus Bridge)
# 8. Index (punto de entrada, inicialización)

cat \
  "${CODE_DIR}/utils/logger.js" \
  "${CODE_DIR}/utils/geometry.js" \
  "${CODE_DIR}/core/window_utils.js" \
  "${CODE_DIR}/core/quarantine.js" \
  "${CODE_DIR}/core/focus.js" \
  "${CODE_DIR}/services/dbus_bridge.js" \
  "${CODE_DIR}/services/shortcuts.js" \
  > "${OUTPUT}.tmp"

# Agregar las funciones de inicialización y el bloque de arranque desde index.js,
# excluyendo las directivas //@include que son solo para documentación
grep -v '^//@include ' "${CODE_DIR}/index.js" >> "${OUTPUT}.tmp"

mv "${OUTPUT}.tmp" "${OUTPUT}"

echo "✅ Bundle generado exitosamente: ${OUTPUT}"
echo "   Tamaño: $(wc -c < "${OUTPUT}") bytes, $(wc -l < "${OUTPUT}") líneas"
