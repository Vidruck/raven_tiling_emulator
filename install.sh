#!/bin/bash
# Raven Tiling Emulator - Native Rust Orchestrator
# Autor: Alejandro González Hernández (Vidruck)

TARGET_DIR="$HOME/.local/share/raven"
SOURCE_DIR=$(pwd)
ICON_NAME="org.kde.raven.tiling"

echo "🐦 Iniciando orquestación de Raven..."

# Verificaciones de sanidad
if [ -f "$HOME/.cargo/env" ]; then
    source "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "⚠️ Rust/Cargo no detectado. Descargando e instalando mediante rustup..."
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        else
            export PATH="$HOME/.cargo/bin:$PATH"
        fi
    else
        echo >&2 "❌ Error: curl no está instalado. No se puede descargar Rust/Cargo automáticamente."
        exit 1;
    fi
fi

# Volver a comprobar por seguridad
command -v cargo >/dev/null 2>&1 || { echo >&2 "❌ Error: Rust/Cargo no disponible para compilar."; exit 1; }
command -v kpackagetool6 >/dev/null 2>&1 || { echo >&2 "❌ Error: kpackagetool6 no detectado. ¿Estás en Plasma 6?"; exit 1; }

# [1/7] Despliegue de código y sincronización
if [ "$SOURCE_DIR" != "$TARGET_DIR" ]; then
    echo "[1/7] Desplegando en entorno de ejecución ($TARGET_DIR)..."
    mkdir -p "$TARGET_DIR"
    rsync -a --exclude='target' --exclude='.git' --exclude='.venv' "$SOURCE_DIR/" "$TARGET_DIR/"
    cd "$TARGET_DIR" || exit
else
    echo "[1/7] Ejecutando desde directorio de destino. Verificando actualizaciones..."
    if [ -d ".git" ]; then
        git pull origin main || echo "⚠️ Sincronización fallida. Manteniendo versión local."
    fi
fi

# [2/7] Compilación de Alto Rendimiento
echo "[2/7] Compilando componentes nativos (Release)..."
# Optimizamos para la arquitectura del procesador local
export RUSTFLAGS="-C target-cpu=native"
cargo build --release --workspace

# Asegurar persistencia de binarios
mkdir -p "$TARGET_DIR/bin"
# Detener el servicio activo para evitar error de archivo ocupado ("Text file busy")
systemctl --user stop raven.service 2>/dev/null || true
cp target/release/raven_engine "$TARGET_DIR/bin/"
cp target/release/raven_gui "$TARGET_DIR/bin/"

# Limpieza profunda de archivos residuales para reducir peso de instalación
echo "Limpiando artefactos de compilación residuales..."
cargo clean

if [ "$SOURCE_DIR" != "$TARGET_DIR" ]; then
    cd "$SOURCE_DIR" || exit
    cargo clean
    cd "$TARGET_DIR" || exit
fi

# [3/7] Integración con el Entorno de Escritorio
echo "[3/7] Configurando iconos y lanzador de aplicaciones..."
mkdir -p ~/.local/share/icons/hicolor/scalable/apps/
cp icon/${ICON_NAME}.svg ~/.local/share/icons/hicolor/scalable/apps/${ICON_NAME}.svg

cat <<EOF > ~/.local/share/applications/raven.desktop
[Desktop Entry]
Version=1.0
Type=Application
Name=Raven Control Center
GenericName=Gestor de Mosaico (Preferencias)
Comment=Configura el comportamiento del motor nativo Raven
Exec=$TARGET_DIR/bin/raven_gui
Icon=${ICON_NAME}
Terminal=false
Categories=Settings;DesktopSettings;
Keywords=tiling;raven;kde;plasma;
StartupNotify=true
EOF

# [4/7] Sincronización de Servicios KDE
echo "[4/7] Regenerando caché de servicios de KDE..."
kbuildsycoca6 --noincremental > /dev/null 2>&1

# [5/7] Instalación de Extensiones KWin
echo "[5/7] Instalando adaptadores de KWin y Plasmoids..."
kpackagetool6 --type=KWin/Script -i adapters/kwin_script/ 2>/dev/null || kpackagetool6 --type=KWin/Script -u adapters/kwin_script/
kpackagetool6 --type=Plasma/Applet -i adapters/plasmoid/ 2>/dev/null || kpackagetool6 --type=Plasma/Applet -u adapters/plasmoid/

# [6/7] Automatización del Daemon (Systemd)
echo "[6/7] Configurando servicio nativo systemd..."
mkdir -p ~/.config/systemd/user/
cat <<EOF > ~/.config/systemd/user/raven.service
[Unit]
Description=Raven Tiling Emulator Daemon (Native Rust)
After=graphical-session.target

[Service]
ExecStart=$TARGET_DIR/bin/raven_engine
WorkingDirectory=$TARGET_DIR
Restart=always
RestartSec=3
# Optimizaciones de prioridad para el motor de tiling
CPUSchedulingPolicy=rr
CPUSchedulingPriority=50
OOMScoreAdjust=-200

[Install]
WantedBy=graphical-session.target
EOF

mkdir -p ~/.local/share/dbus-1/services/
cat <<EOF > ~/.local/share/dbus-1/services/org.kde.raven.Daemon.service
[D-BUS Service]
Name=org.kde.raven.Daemon
Exec=/usr/bin/systemctl --user start raven.service
SystemdService=raven.service
EOF

# [7/7] Activación del Ecosistema
echo "[7/7] Reiniciando servicios de Raven..."
systemctl --user daemon-reload
systemctl --user enable --now raven.service

# Lanzar el Centro de Bienvenida de Raven en segundo plano
if [ -x "$TARGET_DIR/bin/raven_gui" ]; then
    "$TARGET_DIR/bin/raven_gui" &
fi

echo "✅ Raven Tiling Emulator instalado y operando con éxito. ¡Huélum!"
echo "En caso de no notar cambios o activacion instantanea, favor de reiniciar el puente en Guiones de KWin."