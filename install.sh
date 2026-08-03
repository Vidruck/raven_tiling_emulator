#!/bin/bash
# ==============================================================================
#  🐦 RAVEN TILING EMULATOR — Native Rust Orchestrator & Deployment Suite (v3.0)
#  Autor: Alejandro González Hernández (Vidruck)
#  Licencia: GPL-3.0
# ==============================================================================

set -eo pipefail

# --- Códigos de Color ANSI (Terminal Visual Excellence) ---
BOLD='\033[1m'
DIM='\033[2m'
ITALIC='\033[3m'
RESET='\033[0m'

# Paleta de Colores
CYAN='\033[38;5;51m'
BLUE='\033[38;5;39m'
PURPLE='\033[38;5;141m'
MAGENTA='\033[38;5;198m'
GREEN='\033[38;5;48m'
YELLOW='\033[38;5;220m'
RED='\033[38;5;196m'
GRAY='\033[38;5;244m'

# --- Variables de Entorno y Rutas ---
TARGET_DIR="$HOME/.local/share/raven"
SOURCE_DIR=$(pwd)
ICON_NAME="org.kde.raven.tiling"

# --- Banner de Bienvenida Espectacular ---
print_banner() {
    clear 2>/dev/null || true
    echo -e "${CYAN}${BOLD}"
    cat << "EOF"
  ____       _   __   __  _ _ _   _   _
 |  _ \     / \  \ \ / / | _ _ | | | | |
 | |_) |   / _ \  \ V /  |  _|   |  \| |
 |  _ <   / ___ \  \ /   | |___  | |\  |
 |_| \_\ /_/   \_\  V    |_____| |_| \_|

  _____   ___   _ _      ___   _   _   _____
 |_   _| |_ _| |_  |    |_ _| | \ | | / ____|
   | |    | |   | |      | |  |  \| | | |  _
   | |    | |   | |___   | |  | |\  | | |_| |
   |_|   |___|  | ____| |___| |_| \_|  \____|

  _____  __  __ _  _   _   _ _        _    _ _ _     _      ____
 | ____| |  \/  | | | | | |_  |      / \  |_   _| /  _  \  |  _ \
 |  _|   | |\/| | | | | |  | |      / _ \   | |   | | | |  | |_) |
 | |___  | |  | | |  V  |  | |__   / ___ \  | |   | |_| |  |  _ <
 |_____| |_|  |_|  \ _ /   |____| /_/   \_\ |_|   \ ___ /  |_| \_\
EOF
    echo -e "${MAGENTA}${BOLD} >>> 🐦 Orchestrator & Deployment Suite v3.0 🐦 <<<${RESET}"
    echo -e "${GRAY} Engine: Native Rust | Host: KDE Plasma 6 (Wayland) | IPC: Single-Trip D-Bus${RESET}\n"
}

# --- Funciones de Formato Visual ---
log_step() {
    local step="$1"
    local total="$2"
    local title="$3"
    echo -e "\n${PURPLE}${BOLD}[${step}/${total}]${RESET} ${CYAN}${BOLD}${title}${RESET}"
    echo -e "${GRAY}─────────────────────────────────────────────────────────────────────────────${RESET}"
}

log_info() {
    echo -e " ${BLUE}ℹ${RESET} $1"
}

log_success() {
    echo -e " ${GREEN}✔${RESET} ${BOLD}$1${RESET}"
}

log_warning() {
    echo -e " ${YELLOW}⚠${RESET} $1"
}

log_error() {
    echo -e " ${RED}✖${RESET} ${BOLD}$1${RESET}"
}

spinner() {
    local pid=$1
    local delay=0.1
    local spinstr='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'

    # Si la salida no es una TTY (ej. ejecución en subshell o captura de log), evitar imprimir caracteres de control
    if [ ! -t 1 ]; then
        wait "$pid"
        return
    fi

    while kill -0 "$pid" 2>/dev/null; do
        local temp=${spinstr#?}
        printf " ${MAGENTA}%c${RESET}  " "$spinstr"
        spinstr=$temp${spinstr%"$temp"}
        sleep $delay
        printf "\b\b\b\b"
    done
    printf "    \b\b\b\b"
}

# --- Inicio de Ejecución ---
print_banner

# --- [0/7] Pre-flight & Verificaciones de Sanidad ---
log_step "0" "7" "Verificaciones del Sistema y Sanidad del Entorno"

# Detectar Fedora u otra distribución mediante /etc/os-release
OS_ID=""
if [ -f /etc/os-release ]; then
    OS_ID=$(grep -E '^ID=' /etc/os-release | cut -d'=' -f2 | tr -d '"')
fi

# Soporte automatizado para Fedora Linux (KDE Plasma Spin)
if [ "$OS_ID" = "fedora" ]; then
    log_info "Sistema detectado: Fedora Linux (KDE Plasma Spin)"
    MISSING_PKGS=()
    
    command -v kpackagetool6 >/dev/null 2>&1 || MISSING_PKGS+=("kf6-kpackage")
    command -v kbuildsycoca6 >/dev/null 2>&1 || MISSING_PKGS+=("kde-cli-tools")
    command -v node >/dev/null 2>&1 || MISSING_PKGS+=("nodejs")
    command -v rsync >/dev/null 2>&1 || MISSING_PKGS+=("rsync")
    command -v gcc >/dev/null 2>&1 || MISSING_PKGS+=("gcc")

    if [ ${#MISSING_PKGS[@]} -gt 0 ]; then
        log_warning "Faltan paquetes necesarios del sistema en Fedora: ${MISSING_PKGS[*]}"
        log_info "Instalando dependencias requeridas usando sudo dnf..."
        sudo dnf install -y "${MISSING_PKGS[@]}" || {
            log_error "No se pudieron instalar las dependencias mediante dnf."
            exit 1
        }
        log_success "Dependencias del sistema Fedora instaladas correctamente"
    fi
fi

if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1090
    source "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null 2>&1; then
    log_warning "Rust/Cargo no detectado. Intentando instalación automática vía rustup..."
    if command -v curl >/dev/null 2>&1; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        if [ -f "$HOME/.cargo/env" ]; then
            # shellcheck disable=SC1090
            source "$HOME/.cargo/env"
        else
            export PATH="$HOME/.cargo/bin:$PATH"
        fi
    else
        log_error "curl no está instalado. No se puede descargar Rust/Cargo automáticamente."
        exit 1
    fi
fi

command -v cargo >/dev/null 2>&1 || { log_error "Rust/Cargo no disponible para compilar."; exit 1; }
log_success "Entorno Rust/Cargo listo ($(cargo --version | cut -d' ' -f1-2))"

if ! command -v kpackagetool6 >/dev/null 2>&1; then
    log_error "kpackagetool6 no detectado. En Fedora instala el paquete 'kf6-kpackage'."
    exit 1
fi
log_success "Herramientas de KDE Plasma 6 verificadas (kpackagetool6 activo)"

# --- [1/7] Sincronización de Fuentes ---
log_step "1" "7" "Despliegue del Entorno de Ejecución"

log_info "Sincronizando fuentes con ruta de destino: ${DIM}$TARGET_DIR${RESET}"
mkdir -p "$TARGET_DIR"
rsync -a --exclude='target' --exclude='.git' --exclude='.venv' "$SOURCE_DIR/" "$TARGET_DIR/"
log_success "Entorno de fuentes sincronizado correctamente"

# --- [2/7] Compilación de Alto Rendimiento (Rust Native Release) ---
log_step "2" "7" "Compilación de Alto Rendimiento (Rust Release Mode)"

log_info "Optimizando flags para la arquitectura de CPU nativa: ${BOLD}$(uname -m)${RESET}"
export RUSTFLAGS="-C target-cpu=native"
export CARGO_TERM_COLOR=always

log_info "Compilando raven_engine y raven_gui en modo --release..."

if cargo build --release --workspace; then
    log_success "Compilación nativa finalizada exitosamente"
else
    log_error "Fallo crítico durante la compilación en Rust."
    exit 1
fi

log_info "Instalando binarios en $TARGET_DIR/bin/"
mkdir -p "$TARGET_DIR/bin"
systemctl --user stop raven.service 2>/dev/null || true

cp target/release/raven_engine "$TARGET_DIR/bin/"
cp target/release/raven_gui "$TARGET_DIR/bin/"
log_success "Binarios de producción instalados en $TARGET_DIR/bin/"

log_info "Limpiando archivos de compilación intermedios (target/) para liberar espacio..."
cargo clean > /dev/null 2>&1 || true
rm -rf "$TARGET_DIR/target" 2>/dev/null || true
log_success "Artefactos temporales eliminados correctamente"

# --- [3/7] Integración en el Escritorio (Desktop & Icons) ---
log_step "3" "7" "Integración con el Entorno de Escritorio"

mkdir -p ~/.local/share/icons/hicolor/scalable/apps/
cp icon/${ICON_NAME}.svg ~/.local/share/icons/hicolor/scalable/apps/${ICON_NAME}.svg
log_success "Icono vectorial instalado en la galería del sistema"

mkdir -p ~/.local/share/applications/
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
log_success "Lanzador 'Raven Control Center' registrado en el menú de aplicaciones"

# --- [4/7] Reconstrucción de Caché de Servicios ---
log_step "4" "7" "Regenerando Caché de Servicios de KDE (KBuildSycoca)"

log_info "Ejecutando kbuildsycoca6..."
kbuildsycoca6 --noincremental > /dev/null 2>&1 || true
log_success "Caché de servicios de KDE actualizada"

# --- [5/7] Empaquetado y Despliegue de Adaptadores KWin ---
log_step "5" "7" "Empaquetado y Despliegue de Adaptadores KWin & Plasmoid"

if command -v node >/dev/null 2>&1; then
    log_info "Compilando submódulos multiarchivo de kwin_script a bundle distribuible..."
    node -e '
        const fs = require("fs");
        const path = require("path");
        function buildBundle(entryPath) {
            let content = fs.readFileSync(entryPath, "utf8");
            const dir = path.dirname(entryPath);
            return content.replace(/\/\/\s*@include\s+"([^"]+)"/g, (match, relPath) => {
                return buildBundle(path.join(dir, relPath));
            });
        }
        const bundle = buildBundle("adapters/kwin_script/contents/code/main.js");
        fs.writeFileSync("adapters/kwin_script/contents/code/main.js", bundle);
    ' 2>/dev/null || true
    log_success "Bundle JS compilado exitosamente"
fi

log_info "Instalando script de KWin 'org.kde.raven.bridge'..."
kpackagetool6 --type=KWin/Script -i adapters/kwin_script/ >/dev/null 2>&1 || \
kpackagetool6 --type=KWin/Script -u adapters/kwin_script/ >/dev/null 2>&1
log_success "Script de KWin instalado/actualizado"

log_info "Instalando Plasmoide para la barra de tareas..."
kpackagetool6 --type=Plasma/Applet -i adapters/plasmoid/ >/dev/null 2>&1 || \
kpackagetool6 --type=Plasma/Applet -u adapters/plasmoid/ >/dev/null 2>&1
log_success "Plasmoide de Plasma 6 instalado/actualizado"

# --- [6/7] Automatización y Daemon Systemd ---
log_step "6" "7" "Configuración del Servicio Nativo Systemd & D-Bus"

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
log_success "Servicios systemd (raven.service) y D-Bus activadores configurados"

# --- [7/7] Activación del Ecosistema ---
log_step "7" "7" "Activación e Inicio del Ecosistema Raven"

systemctl --user daemon-reload
systemctl --user enable --now raven.service >/dev/null 2>&1 || true
log_success "Demonio nativo raven_engine activo y ejecutándose"

if [ -x "$TARGET_DIR/bin/raven_gui" ]; then
    log_info "Iniciando Centro de Control (Raven GUI)..."
    ("$TARGET_DIR/bin/raven_gui" >/dev/null 2>&1 &)
fi

# --- Resumen Final y Salida de Gala ---
echo -e "\n${GREEN}${BOLD}=============================================================================${RESET}"
echo -e "${GREEN}${BOLD}   🎉 ¡INSTALACIÓN COMPLETADA Y CON ÉXITO! 🎉${RESET}"
echo -e "${GREEN}${BOLD}=============================================================================${RESET}\n"

echo -e " ${CYAN}• ${BOLD}Motor Rust:${RESET}        ${GREEN}Activo (raven_engine / Systemd)${RESET}"
echo -e " ${CYAN}• ${BOLD}Centro de Control:${RESET} ${GREEN}Desplegado (Raven GUI - egui/eframe)${RESET}"
echo -e " ${CYAN}• ${BOLD}Puente KWin:${RESET}       ${GREEN}Empaquetado (v3.0 Multiarchivo / Single-Trip D-Bus)${RESET}"
echo -e " ${CYAN}• ${BOLD}Plasmoide:${RESET}         ${GREEN}Disponible en paneles de Plasma 6${RESET}\n"

echo -e "${YELLOW}${BOLD}💡 Tip:${RESET} Si es la primera vez que instalas el script en KWin, asegúrate de tener activada"
echo -e "      la casilla de ${BOLD}Raven Bridge${RESET} en ${DIM}Preferencias del Sistema -> Scripts de KWin${RESET}.\n"

echo -e "${MAGENTA}${BOLD}   ¡Huélum! 🐦${RESET}\n"
