#!/bin/bash
# ==============================================================================
#  🐦 RAVEN TILING EMULATOR — Interactive TUI Management Suite (v4.5.5)
#  Autor: Alejandro González Hernández (Vidruck)
#  Licencia: GPL-3.0
# ==============================================================================

set -eo pipefail

# --- Configuración de Entorno y Rutas ---
TARGET_DIR="$HOME/.local/share/raven"
SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ICON_NAME="org.kde.raven.tiling"
KWIN_SCRIPT_ID="org.kde.raven.bridge"
PLASMOID_ID="org.kde.plasma.ravenlauncher"

# --- Estilos Visuales & Paleta ---
BOLD='\033[1m'
DIM='\033[2m'
ITALIC='\033[3m'
RESET='\033[0m'

PRIMARY='\033[38;5;141m'   # Purple/Violet
SECONDARY='\033[38;5;39m'  # Cyan Electric
ACCENT='\033[38;5;213m'    # Magenta Pink
SUCCESS='\033[38;5;48m'     # Mint Green
WARNING='\033[38;5;220m'   # Gold Yellow
ERROR='\033[38;5;196m'     # Bright Red
MUTED='\033[38;5;244m'     # Cool Gray
BG_CARD='\033[48;5;235m'   # Dark Background Card

# --- Componentes UI ---

# Función para calcular el ancho visual de una cadena (manejando ANSI y emojis de doble ancho)
get_visual_length() {
    local str="$1"
    python3 -c '
import sys, re, unicodedata
s = sys.argv[1]
s_clean = re.sub(r"(\\033|\x1b|\\e)\[[0-9;]*[a-zA-Z]", "", s)
w = 0
for ch in s_clean:
    if ord(ch) in (0xFE0E, 0xFE0F): # variation selectors
        continue
    eaw = unicodedata.east_asian_width(ch)
    if eaw in ("W", "F") or (unicodedata.category(ch) in ("So", "Sk") and ord(ch) > 0x1000):
        w += 2
    else:
        w += 1
print(w)
' "$str" 2>/dev/null || {
        local clean=$(echo -e "$str" | sed 's/\x1b\[[0-9;]*m//g')
        echo ${#clean}
    }
}

draw_box() {
    local title="$1"
    shift
    local lines=("$@")
    local max_len=$(get_visual_length "$title")
    
    for line in "${lines[@]}"; do
        local len=$(get_visual_length "$line")
        if [ "$len" -gt "$max_len" ]; then
            max_len=$len
        fi
    done
    
    local width=$((max_len + 4))
    
    # Borde superior
    echo -e "${PRIMARY}╭$(printf '─%.0s' $(seq 1 $width))╮${RESET}"
    if [ -n "$title" ]; then
        local title_len=$(get_visual_length "$title")
        local title_pad=$((width - title_len - 1))
        echo -e "${PRIMARY}│ ${ACCENT}${BOLD}$title${RESET}$(printf ' %.0s' $(seq 1 $title_pad))${PRIMARY}│${RESET}"
        echo -e "${PRIMARY}├$(printf '─%.0s' $(seq 1 $width))┤${RESET}"
    fi
    
    # Contenido
    for line in "${lines[@]}"; do
        local line_len=$(get_visual_length "$line")
        local pad=$((width - line_len - 1))
        if [ "$pad" -lt 0 ]; then pad=0; fi
        echo -e "${PRIMARY}│${RESET} $line$(printf ' %.0s' $(seq 1 $pad))${PRIMARY}│${RESET}"
    done
    
    # Borde inferior
    echo -e "${PRIMARY}╰$(printf '─%.0s' $(seq 1 $width))╯${RESET}"
}

print_header() {
    clear 2>/dev/null || true
    echo -e "${SECONDARY}${BOLD}"
    cat << "EOF"
  ██████╗   █████╗  ██╗   ██╗ ███████╗ ███╗   ██╗
  ██╔══██╗ ██╔══██╗ ██║   ██║ ██╔════╝ ████╗  ██║
  ██████╔╝ ███████║ ██║   ██║ █████╗   ██╔██╗ ██║
  ██╔══██╗ ██╔══██║ ╚██╗ ██╔╝ ██╔══╝   ██║╚██╗██║
  ██║  ██║ ██║  ██║  ╚████╔╝  ███████╗ ██║ ╚████║
  ╚═╝  ╚═╝ ╚═╝  ╚═╝   ╚═══╝   ╚══════╝ ╚═╝  ╚═══╝
EOF
    echo -e "${ACCENT}${BOLD}            🐦  — Tiling Emulator —  🐦${RESET}"
    echo -e "${PRIMARY}${BOLD}               - Menu de Instalación -${RESET}"
    echo -e "${MUTED} Engine: Native Rust | Host: KDE Plasma 6 (Wayland) | IPC: Single-Trip D-Bus${RESET}\n"
}

log_info() {
    echo -e " ${SECONDARY}ℹ${RESET} $1"
}

log_success() {
    echo -e " ${SUCCESS}✔${RESET} ${BOLD}$1${RESET}"
}

log_warning() {
    echo -e " ${WARNING}⚠${RESET} $1"
}

log_error() {
    echo -e " ${ERROR}✖${RESET} ${BOLD}$1${RESET}"
}

log_step() {
    local step="$1"
    local total="$2"
    local title="$3"
    echo -e "\n${PRIMARY}${BOLD}[$step/$total]${RESET} ${SECONDARY}${BOLD}$title${RESET}"
    echo -e "${MUTED}───────────────────────────────────────────────────────────────────────────────${RESET}"
}

# --- Detección de Requisitos del Sistema y Multiarch ---
check_system_prerequisites() {
    log_step "1" "7" "Validación de Entorno y Sistema"

    # Detección de distribución
    local distro_id=""
    local distro_name="Linux Desconocido"
    if [ -f /etc/os-release ]; then
        distro_id=$(grep -E '^ID=' /etc/os-release | cut -d= -f2 | tr -d '"')
        distro_name=$(grep -E '^PRETTY_NAME=' /etc/os-release | cut -d= -f2 | tr -d '"')
    fi
    log_info "Distribución detectada: ${distro_name}"

    # Verificación de KDE Plasma
    if [[ "$XDG_CURRENT_DESKTOP" =~ KDE|Plasma ]] || [[ "$XDG_SESSION_DESKTOP" =~ KDE|Plasma ]] || [ "$KDE_FULL_SESSION" = "true" ]; then
        local plasma_ver
        plasma_ver=$(plasmashell --version 2>/dev/null | awk '{print $2}' || true)
        if [ -n "$plasma_ver" ]; then
            log_success "Entorno KDE Plasma detectado (v${plasma_ver})"
            if [[ "$plasma_ver" =~ ^5\. ]]; then
                log_error "Estás ejecutando KDE Plasma 5 (${plasma_ver}). Raven v4.6+ requiere KDE Plasma 6 sobre Wayland."
                log_warning "Distribuciones compatibles con Plasma 6: Ubuntu 24.10+, Ubuntu 26.04 LTS, Debian Trixie/Sid, Fedora 40+, Arch Linux."
                exit 1
            fi
        else
            log_success "Entorno KDE Plasma detectado"
        fi
    else
        log_warning "No se detectó un entorno KDE Plasma activo. Asegúrate de estar corriendo Plasma 6 sobre Wayland."
    fi

    # Verificación de Cargo / Rust
    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Cargo (Rust) no está instalado."
        case "$distro_id" in
            ubuntu|debian)
                log_info "Instálalo en Debian / Ubuntu con: sudo apt install cargo rustc (o vía https://rustup.rs)"
                ;;
            arch|manjaro)
                log_info "Instálalo en Arch Linux con: sudo pacman -S rust"
                ;;
            fedora)
                log_info "Instálalo en Fedora con: sudo dnf install cargo"
                ;;
            *)
                log_info "Instala Rust oficialmente desde: https://rustup.rs"
                ;;
        esac
        exit 1
    fi
    log_success "Compilador Rust (Cargo $(cargo --version 2>/dev/null | awk '{print $2}')) disponible"

    # Verificación de herramientas de compilación
    local missing_tools=()
    if ! command -v cmake >/dev/null 2>&1; then missing_tools+=("cmake"); fi
    if ! command -v kpackagetool6 >/dev/null 2>&1; then missing_tools+=("kpackagetool6"); fi

    if [ ${#missing_tools[@]} -gt 0 ]; then
        log_warning "Herramientas faltantes detectadas: ${missing_tools[*]}"
        case "$distro_id" in
            ubuntu|debian)
                log_info "Para instalar las dependencias necesarias en Debian / Ubuntu ejecuta:"
                echo -e "   ${ACCENT}sudo apt update && sudo apt install build-essential cmake extra-cmake-modules kwin-dev libplasma-dev qt6-base-dev qt6-declarative-dev libkf6coreaddons-dev libkf6config-dev libkf6windowsystem-dev libkdecoration3-dev${RESET}"
                ;;
            arch|manjaro)
                log_info "Para instalar las dependencias necesarias en Arch Linux ejecuta:"
                echo -e "   ${ACCENT}sudo pacman -S --needed base-devel cmake extra-cmake-modules kwin qt6-base qt6-declarative${RESET}"
                ;;
            fedora)
                log_info "Para instalar las dependencias necesarias en Fedora ejecuta:"
                echo -e "   ${ACCENT}sudo dnf install cmake extra-cmake-modules kwin-devel qt6-qtbase-devel qt6-qtdeclarative-devel kf6-kcoreaddons-devel${RESET}"
                ;;
        esac
    fi
}

# --- Despliegue Robusto Multi-Distribución de Efectos KWin (Arch, Debian, Ubuntu, Fedora) ---
install_kwin_effect_binary() {
    local EFFECT_SO="$1"
    if [ ! -f "$EFFECT_SO" ]; then
        log_error "No se encontró el binario del efecto: $EFFECT_SO"
        return 1
    fi

    log_info "Instalando binario del efecto KWin de forma segura (instalación atómica)..."

    # ── Blindaje anti-crash Plasma 6 ──────────────────────────────────────────
    # Desactivar temporalmente el efecto en KWin vía D-Bus antes de reemplazarlo
    qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.unloadEffect kwin4_effect_raven >/dev/null 2>&1 || true
    kwriteconfig6 --file kwinrc --group Plugins --key kwin4_effect_ravenEnabled true 2>/dev/null || true

    local arch_triplet="$(uname -m)-linux-gnu"
    local qtpaths_dir="$(qtpaths6 --plugin-dir 2>/dev/null || qtpaths --plugin-dir 2>/dev/null || kf6-config --qt-plugins 2>/dev/null || true)"

    local CANDIDATE_DIRS=()
    [ -n "$qtpaths_dir" ] && CANDIDATE_DIRS+=("$qtpaths_dir/kwin/effects/plugins")
    CANDIDATE_DIRS+=(
        "/usr/lib/${arch_triplet}/qt6/plugins/kwin/effects/plugins"
        "/usr/lib/x86_64-linux-gnu/qt6/plugins/kwin/effects/plugins"
        "/usr/lib/aarch64-linux-gnu/qt6/plugins/kwin/effects/plugins"
        "/usr/lib64/qt6/plugins/kwin/effects/plugins"
        "/usr/lib/qt6/plugins/kwin/effects/plugins"
    )

    local installed_system=false
    for dir in "${CANDIDATE_DIRS[@]}"; do
        local parent_dir="$(dirname "$dir")"
        if [ -d "$dir" ] || [ -d "$parent_dir" ]; then
            if sudo mkdir -p "$dir" 2>/dev/null && sudo install -m 755 "$EFFECT_SO" "$dir/kwin4_effect_raven.so" 2>/dev/null; then
                installed_system=true
                log_success "Plugin de efecto instalado en el sistema ($dir/kwin4_effect_raven.so)"
                break
            fi
        fi
    done

    # Si se instaló en el sistema, remover copias locales para evitar discrepancias
    if [ "$installed_system" = true ]; then
        rm -f "$HOME/.local/lib/${arch_triplet}/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so" 2>/dev/null || true
        rm -f "$HOME/.local/lib64/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so" 2>/dev/null || true
        rm -f "$HOME/.local/lib/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so" 2>/dev/null || true
    else
        log_warning "No se pudo instalar con sudo en directorios de sistema. Instalando en ~/.local..."
        local LOCAL_DIRS=(
            "$HOME/.local/lib/${arch_triplet}/qt6/plugins/kwin/effects/plugins"
            "$HOME/.local/lib64/qt6/plugins/kwin/effects/plugins"
            "$HOME/.local/lib/qt6/plugins/kwin/effects/plugins"
        )
        for ldir in "${LOCAL_DIRS[@]}"; do
            mkdir -p "$ldir" 2>/dev/null || true
            install -m 755 "$EFFECT_SO" "$ldir/kwin4_effect_raven.so" 2>/dev/null || true
        done
        log_success "Plugin de efecto instalado localmente en ~/.local"
    fi

    # Recargar el efecto dinámicamente en KWin
    log_info "Recargando efecto en KWin..."
    qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.loadEffect kwin4_effect_raven >/dev/null 2>&1 || true
    log_success "Efecto Nativo de KWin (kwin4_effect_raven) desplegado y habilitado"
    return 0
}

# --- Lógica de Instalación ---
do_install() {
    print_header
    echo -e "${PRIMARY}${BOLD}🚀 Iniciando Instalación Completa de Raven Tiling Emulator...${RESET}\n"

    # 1. Comprobación de Requisitos y Plataforma
    check_system_prerequisites

    # 2. Compilación del Motor Rust
    log_step "2" "7" "Compilación del Motor Nativo Rust (Release)"
    log_info "Compilando raven_engine y raven_gui en modo --release..."
    
    if cargo build --release --manifest-path "$SOURCE_DIR/Cargo.toml" 2>&1 | while read -r line; do
        if [[ "$line" =~ Compiling[[:space:]]+([^[:space:]]+) ]]; then
            echo -ne "\r\033[K ${SECONDARY}⚙${RESET} ${MUTED}Compilando crate:${RESET} ${PRIMARY}${BOLD}${BASH_REMATCH[1]}${RESET}"
        elif [[ "$line" =~ Finished ]]; then
            echo -ne "\r\033[K"
        fi
    done; then
        echo -ne "\r\033[K"
        log_success "Binarios de Rust (raven_engine y raven_gui) compilados con éxito"
    else
        echo -ne "\r\033[K"
        log_error "Fallo en la compilación de Rust. Ejecuta 'cargo build --release' para ver los errores."
        exit 1
    fi

    # 3. Creación de Estructura Destino
    log_step "3" "7" "Despliegue de Binarios en Entorno Local"
    systemctl --user stop raven.service 2>/dev/null || true
    mkdir -p "$TARGET_DIR/bin"
    mkdir -p "$HOME/.local/bin"
    install -m 755 "$SOURCE_DIR/target/release/raven_engine" "$TARGET_DIR/bin/raven_engine"
    install -m 755 "$SOURCE_DIR/target/release/raven_gui" "$TARGET_DIR/bin/raven_gui"
    ln -sf "$TARGET_DIR/bin/raven_engine" "$HOME/.local/bin/raven_engine"
    ln -sf "$TARGET_DIR/bin/raven_gui" "$HOME/.local/bin/raven_gui"
    log_success "Ejecutables instalados en $TARGET_DIR/bin/ y enlazados en ~/.local/bin/"

    # 4. Iconos y Lanzadores Desktop
    log_step "4" "7" "Integración de Escritorio e Iconografía"
    mkdir -p "$HOME/.local/share/icons/hicolor/scalable/apps"
    mkdir -p "$HOME/.local/share/applications"
    
    if [ -f "$SOURCE_DIR/assets/icon_app/org.kde.raven.tiling.svg" ]; then
        cp "$SOURCE_DIR/assets/icon_app/org.kde.raven.tiling.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/${ICON_NAME}.svg"
    elif [ -f "$SOURCE_DIR/icon/org.kde.raven.tiling.svg" ]; then
        cp "$SOURCE_DIR/icon/org.kde.raven.tiling.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/${ICON_NAME}.svg"
    fi

    cat <<EOF > "$HOME/.local/share/applications/raven.desktop"
[Desktop Entry]
Name=Raven Control Center
Comment=Configuración Nativa para Raven Tiling Emulator
Exec=$TARGET_DIR/bin/raven_gui
Icon=$ICON_NAME
Terminal=false
Type=Application
Categories=Utility;Settings;Qt;KDE;
EOF
    chmod +x "$HOME/.local/share/applications/raven.desktop"
    update-desktop-database "$HOME/.local/share/applications/" 2>/dev/null || true
    log_success "Lanzadores e icono registrados en el sistema"

    # 5. Generación de Bundle JS y Registro de Adaptadores KWin / Plasmoid
    log_step "5" "7" "Empaquetado JS, C++ y Despliegue de Adaptadores KDE"
    if [ -f "$SOURCE_DIR/build_kwin_bundle.sh" ]; then
        log_info "Ensamblando bundle monolítico de KWin (build_kwin_bundle.sh)..."
        bash "$SOURCE_DIR/build_kwin_bundle.sh" >/dev/null 2>&1
        log_success "Bundle main.js ensamblado exitosamente"
    fi

    log_info "Configurando proyecto C++ / Qt 6 con CMake..."
    cmake -B "$SOURCE_DIR/adapters/plasmoid/build" -S "$SOURCE_DIR/adapters/plasmoid" -DCMAKE_BUILD_TYPE=Release >/dev/null 2>&1 || true
    
    log_info "Compilando Plugin Nativo C++ / QML de Raven Hub (CMake Build)..."
    local num_cores=$(nproc 2>/dev/null || echo 2)
    if cmake --build "$SOURCE_DIR/adapters/plasmoid/build" -j"$num_cores" 2>&1 | while read -r line; do
        if [[ "$line" =~ \[([0-9]+)%\][[:space:]]+Building[[:space:]]+CXX[[:space:]]+object[[:space:]]+(.*) ]]; then
            local pct="${BASH_REMATCH[1]}"
            local obj="${BASH_REMATCH[2]##*/}"
            echo -ne "\r\033[K ${SECONDARY}⚙${RESET} ${MUTED}[${pct}%] Compilando C++:${RESET} ${ACCENT}${BOLD}${obj}${RESET}"
        elif [[ "$line" =~ \[([0-9]+)%\][[:space:]]+Linking[[:space:]]+CXX[[:space:]]+shared[[:space:]]+module[[:space:]]+(.*) ]]; then
            local pct="${BASH_REMATCH[1]}"
            local mod="${BASH_REMATCH[2]##*/}"
            echo -ne "\r\033[K ${SUCCESS}🔗${RESET} ${MUTED}[${pct}%] Enlazando módulo:${RESET} ${PRIMARY}${BOLD}${mod}${RESET}"
        fi
    done; then
        echo -ne "\r\033[K"
        cmake --install "$SOURCE_DIR/adapters/plasmoid/build" --prefix "$HOME/.local" >/dev/null 2>&1 || true
        log_success "Plugin C++ / QML compilado e instalado con éxito"
    else
        echo -ne "\r\033[K"
        log_error "Fallo en la compilación de C++. Ejecuta 'cmake --build build' para depurar."
        exit 1
    fi

    # Desplegar módulo en rutas locales de QML
    local build_qml_dir="$SOURCE_DIR/adapters/plasmoid/build/plugin/org/kde/plasma/ravenlauncher/plugin"
    if [ -d "$build_qml_dir" ]; then
        local arch_triplet="$(uname -m)-linux-gnu"
        local qml_paths=(
            "$HOME/.local/lib/${arch_triplet}/qml/org/kde/plasma/ravenlauncher/plugin"
            "$HOME/.local/lib/${arch_triplet}/qt6/qml/org/kde/plasma/ravenlauncher/plugin"
            "$HOME/.local/lib/qt6/qml/org/kde/plasma/ravenlauncher/plugin"
            "$HOME/.local/lib64/qt6/qml/org/kde/plasma/ravenlauncher/plugin"
            "$HOME/.local/share/qml/org/kde/plasma/ravenlauncher/plugin"
            "$SOURCE_DIR/adapters/plasmoid/package/contents/ui/org/kde/plasma/ravenlauncher/plugin"
        )
        cp -a "$SOURCE_DIR/adapters/plasmoid/plugin/RavenTheme.qml" "$build_qml_dir/" 2>/dev/null || true
        for p in "${qml_paths[@]}"; do
            mkdir -p "$p"
            cp -a "$build_qml_dir"/* "$p/" 2>/dev/null || true
        done
        local user_qml_path="$HOME/.local/lib/${arch_triplet}/qml:$HOME/.local/lib/${arch_triplet}/qt6/qml:$HOME/.local/lib/qt6/qml:$HOME/.local/lib64/qt6/qml:$HOME/.local/share/qml"
        systemctl --user set-environment QML2_IMPORT_PATH="$user_qml_path:$QML2_IMPORT_PATH" 2>/dev/null || true
        systemctl --user set-environment QML_IMPORT_PATH="$user_qml_path:$QML_IMPORT_PATH" 2>/dev/null || true
    fi
    log_success "Módulos QML de Raven Hub sincronizados"

    # Compilación y Despliegue del Efecto C++ Nativo de KWin (adapters/kwin_effect)
    if [ -d "$SOURCE_DIR/adapters/kwin_effect" ]; then
        log_info "Configurando y compilando Efecto Nativo de KWin (adapters/kwin_effect)..."
        cmake -B "$SOURCE_DIR/adapters/kwin_effect/build" -S "$SOURCE_DIR/adapters/kwin_effect" -DCMAKE_BUILD_TYPE=Release >/dev/null 2>&1 || true
        if cmake --build "$SOURCE_DIR/adapters/kwin_effect/build" -j"$num_cores" >/dev/null 2>&1; then
            local EFFECT_SO="$SOURCE_DIR/adapters/kwin_effect/build/kwin/effects/plugins/kwin4_effect_raven.so"
            if [ ! -f "$EFFECT_SO" ] && [ -f "$SOURCE_DIR/adapters/kwin_effect/build/kwin4_effect_raven.so" ]; then
                EFFECT_SO="$SOURCE_DIR/adapters/kwin_effect/build/kwin4_effect_raven.so"
            fi
            if [ -f "$EFFECT_SO" ]; then
                install_kwin_effect_binary "$EFFECT_SO"
            fi
        else
            log_warning "No se pudo compilar el Efecto Nativo de KWin C++ (fallback grácil a animaciones estándar)"
        fi
    fi


    log_info "Instalando script de KWin '$KWIN_SCRIPT_ID'..."
    kpackagetool6 --type=KWin/Script -i "$SOURCE_DIR/adapters/kwin_script/" >/dev/null 2>&1 || \
    kpackagetool6 --type=KWin/Script -u "$SOURCE_DIR/adapters/kwin_script/" >/dev/null 2>&1
    log_success "Script de KWin registrado en Plasma 6"

    log_info "Instalando Plasmoide para el panel '$PLASMOID_ID'..."
    # Eliminar applet viejo toggle si existiera para evitar duplicados
    kpackagetool6 --type=Plasma/Applet -r "org.kde.raven.toggle" >/dev/null 2>&1 || true
    rm -rf "$HOME/.local/share/plasma/plasmoids/org.kde.raven.toggle" 2>/dev/null || true

    kpackagetool6 --type=Plasma/Applet -i "$SOURCE_DIR/adapters/plasmoid/package" >/dev/null 2>&1 || \
    kpackagetool6 --type=Plasma/Applet -u "$SOURCE_DIR/adapters/plasmoid/package" >/dev/null 2>&1
    log_success "Plasmoide de Plasma 6 ($PLASMOID_ID) instalado/actualizado"

    do_inject_shortcuts

    # 6. Servicio Systemd y D-Bus
    log_step "6" "7" "Configuración del Servicio Nativo Systemd"
    mkdir -p "$HOME/.config/systemd/user/"
    cat <<EOF > "$HOME/.config/systemd/user/raven.service"
[Unit]
Description=Raven Tiling Emulator Daemon (Native Rust)
PartOf=graphical-session.target
After=graphical-session.target
StartLimitIntervalSec=30
StartLimitBurst=5

[Service]
Type=dbus
BusName=org.kde.raven.Daemon
ExecStart=$TARGET_DIR/bin/raven_engine
WorkingDirectory=$TARGET_DIR
Restart=on-failure
RestartSec=1
Slice=session.slice
Nice=2
CPUWeight=100
MemoryHigh=96M
MemoryMax=192M
TasksMax=64
NoNewPrivileges=yes

[Install]
WantedBy=graphical-session.target
EOF

    mkdir -p "$HOME/.local/share/dbus-1/services/"
    cat <<EOF > "$HOME/.local/share/dbus-1/services/org.kde.raven.Daemon.service"
[D-BUS Service]
Name=org.kde.raven.Daemon
Exec=/usr/bin/systemctl --user start raven.service
SystemdService=raven.service
EOF
    log_success "Servicios raven.service y D-Bus activadores configurados"

    # 7. Activación
    log_step "7" "7" "Activación del Servicio"
    systemctl --user daemon-reload
    systemctl --user enable --now raven.service >/dev/null 2>&1 || true
    log_success "Demonio raven_engine activo"

    echo ""
    draw_box "🎉 ¡INSTALACIÓN COMPLETADA EXITOSAMENTE!" \
        "${SUCCESS}${BOLD}• Motor Nativo Rust:${RESET}  Activo (raven_engine via Systemd)" \
        "${SUCCESS}${BOLD}• Centro de Control:${RESET}  Desplegado (Raven GUI - egui)" \
        "${SUCCESS}${BOLD}• Puente KWin:${RESET}       Empaquetado e Instalado en Plasma 6" \
        "${SUCCESS}${BOLD}• Plasmoide:${RESET}         Disponible en el panel" \
        "" \
        "${WARNING}💡 Nota:${RESET} Recuerda activar 'Raven Bridge' en:" \
        "${MUTED}   Preferencias del Sistema -> Scripts de KWin${RESET}"
    echo ""
}

# --- Lógica de Inyección de Atajos Globales de KWin ---
do_inject_shortcuts() {
    print_header
    echo -e "${PRIMARY}${BOLD}⌨️ Inyectando Atajos de Teclado Globales en KDE Plasma (kglobalshortcutsrc)...${RESET}\n"

    local SHORTCUTS_FILE="$HOME/.config/kglobalshortcutsrc"
    
    if [ ! -f "$SHORTCUTS_FILE" ]; then
        mkdir -p "$(dirname "$SHORTCUTS_FILE")"
        touch "$SHORTCUTS_FILE"
    fi

    log_info "Limpiando conflictos con scripts de mosaico antiguos (Polonium, Krohnkite, KZones)..."
    # Limpieza defensiva en kglobalshortcutsrc de asignaciones heredadas
    if command -v kwriteconfig6 >/dev/null 2>&1; then
        # Liberar combinaciones en conflictos si pertenecían a scripts anteriores desinstalados
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "KZones: Snap all windows" "none,none,KZones: Snap all windows" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "PoloniumRetileWindow" "none,none,Polonium: Retile Window" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "PoloniumInsertAbove" "none,none,Polonium: Insert Above" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "PoloniumInsertBelow" "none,none,Polonium: Insert Below" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "PoloniumInsertLeft" "none,none,Polonium: Insert Left" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "KrohnkiteFloatAll" "none,none,Krohnkite: Toggle Float All" 2>/dev/null || true
    fi

    log_info "Configurando combinación de teclas predeterminadas para Raven en [kwin]..."

    if command -v kwriteconfig6 >/dev/null 2>&1; then
        # Desvincular combinaciones duplicadas en acciones nativas de KWin / Plasma para evitar colisiones dobles
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Window Minimize" "Meta+PgDown,Meta+PgDown,Minimizar la ventana" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Window Maximize" "Meta+PgUp,Meta+PgUp,Maximizar la ventana" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Window Close" "Alt+F4,Alt+F4,Cerrar la ventana" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Window Quick Tile Left" "none,Meta+Left,Situar la ventana en mosaico a la izquierda" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Window Quick Tile Right" "none,Meta+Right,Situar la ventana en mosaico a la derecha" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Window Quick Tile Top" "none,Meta+Up,Situar la ventana en mosaico en la parte superior" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Window Quick Tile Bottom" "none,Meta+Down,Situar la ventana en mosaico en la parte inferior" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Switch Window Left" "none,Meta+Alt+Left,Cambiar a la ventana a la izquierda" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Switch Window Right" "none,Meta+Alt+Right,Cambiar a la ventana a la derecha" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Switch Window Up" "none,Meta+Alt+Up,Cambiar a la ventana superior" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Switch Window Down" "none,Meta+Alt+Down,Cambiar a la ventana anterior" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Window to Next Screen" "none,Meta+Shift+Right,Mover ventana a la pantalla siguiente" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "Window to Previous Screen" "none,Meta+Shift+Left,Mover ventana a la pantalla anterior" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "view_zoom_in" "Meta++,Meta++\tMeta+=,Ampliar" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "view_zoom_out" "none,Meta+-,Reducir" 2>/dev/null || true
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "ksmserver" --key "Lock Session" "Screensaver,Screensaver\tMeta+L,Bloquear la sesión" 2>/dev/null || true

        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenToggleTiling" "Meta+Space,none,Raven: Alternar Mosaico (On/Off)"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenToggleFloating" "Meta+Shift+F,none,Raven: Alternar Ventana Flotante Dinámica"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenToggleMaximize" "Meta+F,none,Raven: Alternar Maximizar Ventana"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenMinimizeActive" "Meta+X,none,Raven: Minimizar Ventana"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenCloseActive" "Meta+Q,none,Raven: Cerrar Ventana"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenFocusNext" "Meta+J,none,Raven: Siguiente Ventana"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenFocusPrev" "Meta+K,none,Raven: Ventana Anterior"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenFocusLeft" "Meta+Left,none,Raven: Foco Izquierda"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenFocusRight" "Meta+Right,none,Raven: Foco Derecha"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenFocusUp" "Meta+Up,none,Raven: Foco Arriba"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenFocusDown" "Meta+Down,none,Raven: Foco Abajo"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenSwapNext" "Meta+Shift+J,none,Raven: Intercambiar Siguiente"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenSwapPrev" "Meta+Shift+K,none,Raven: Intercambiar Anterior"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenIncreaseRatio" "Meta+H,none,Raven: Expandir Master"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenDecreaseRatio" "Meta+L,none,Raven: Contraer Master"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenIncrementMaster" "Meta+],none,Raven: Incrementar Master"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenDecrementMaster" "Meta+[,none,Raven: Decrementar Master"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenIncrementGaps" "Meta+=,none,Raven: Incrementar Gaps"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenDecrementGaps" "Meta+-,none,Raven: Decrementar Gaps"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenCycleLayout" "Meta+Shift+L,none,Raven: Ciclar Layout"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenMigrateMonitor" "Meta+Shift+M,none,Raven: Enviar a Otro Monitor"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenMigratePrevMonitor" "Meta+Shift+N,none,Raven: Enviar a Monitor Anterior"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenMigrateDesktop" "Meta+Shift+Right,none,Raven: Enviar a Escritorio Siguiente"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenMigratePrevDesktop" "Meta+Shift+Left,none,Raven: Enviar a Escritorio Anterior"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenResizeWidthInc" "Meta+Alt+Right,none,Raven: Aumentar Ancho de Ventana"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenResizeWidthDec" "Meta+Alt+Left,none,Raven: Reducir Ancho de Ventana"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenResizeHeightInc" "Meta+Alt+Down,none,Raven: Aumentar Alto de Ventana"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenResizeHeightDec" "Meta+Alt+Up,none,Raven: Reducir Alto de Ventana"
    fi

    # Notificar y recargar la caché de atajos de KDE Plasma 6
    log_info "Notificando al demonio de atajos kglobalaccel de Plasma 6..."
    qdbus6 org.kde.kglobalaccel /kglobalaccel reloadConfig >/dev/null 2>&1 || \
    dbus-send --type=method_call --dest=org.kde.kglobalaccel /kglobalaccel org.kde.kglobalaccel.reloadConfig >/dev/null 2>&1 || true

    log_success "Atajos globales inyectados e integrados limpiamente en KDE Plasma 6"
}

# --- Lógica de Compilación y Despliegue de Componentes ---

# 1. Compilación e Instalación del Motor y GUI en Rust
do_build_engine() {
    print_header
    echo -e "${PRIMARY}${BOLD}🦀 Compilando Motor y GUI Nativo Rust (Release)...${RESET}\n"

    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Cargo (Rust) no está instalado. Por favor instala Rust: https://rustup.rs"
        exit 1
    fi

    log_info "Compilando raven_engine y raven_gui en modo --release..."
    if cargo build --release --manifest-path "$SOURCE_DIR/Cargo.toml" 2>&1 | while read -r line; do
        if [[ "$line" =~ Compiling[[:space:]]+([^[:space:]]+) ]]; then
            echo -ne "\r\033[K ${SECONDARY}⚙${RESET} ${MUTED}Compilando crate:${RESET} ${PRIMARY}${BOLD}${BASH_REMATCH[1]}${RESET}"
        elif [[ "$line" =~ Finished ]]; then
            echo -ne "\r\033[K"
        fi
    done; then
        echo -ne "\r\033[K"
        log_success "Binarios de Rust compilados con éxito"
    else
        echo -ne "\r\033[K"
        log_error "Fallo en la compilación de Rust. Ejecuta 'cargo build --release' para ver los errores."
        exit 1
    fi

    # Detener servicio antes de instalar binarios
    if systemctl --user is-active --quiet raven.service 2>/dev/null; then
        log_info "Deteniendo raven.service para reemplazo atómico..."
        systemctl --user stop raven.service 2>/dev/null || true
    fi

    mkdir -p "$TARGET_DIR/bin"
    mkdir -p "$HOME/.local/bin"

    # Instalación atómica (crea un nuevo inodo evitando corrupción de memoria de ejecutables en uso)
    install -m 755 "$SOURCE_DIR/target/release/raven_engine" "$TARGET_DIR/bin/raven_engine"
    install -m 755 "$SOURCE_DIR/target/release/raven_gui" "$TARGET_DIR/bin/raven_gui"
    ln -sf "$TARGET_DIR/bin/raven_engine" "$HOME/.local/bin/raven_engine"
    ln -sf "$TARGET_DIR/bin/raven_gui" "$HOME/.local/bin/raven_gui"
    log_success "Binarios instalados atómicamente en $TARGET_DIR/bin/ y enlazados en ~/.local/bin/"

    # Reiniciar servicio si está configurado
    if [ -f "$HOME/.config/systemd/user/raven.service" ]; then
        systemctl --user daemon-reload 2>/dev/null || true
        systemctl --user restart raven.service 2>/dev/null || true
        log_success "Servicio raven.service reiniciado"
    fi
}

# 2. Reconstrucción del Bundle KWin Script (JavaScript)
do_rebuild_kwin() {
    print_header
    echo -e "${PRIMARY}${BOLD}📜 Reconstruyendo Bundle KWin Script (main.js)...${RESET}\n"

    if [ -f "$SOURCE_DIR/build_kwin_bundle.sh" ]; then
        bash "$SOURCE_DIR/build_kwin_bundle.sh"
        log_info "Actualizando paquete KWin Script..."
        kpackagetool6 --type=KWin/Script -u "$SOURCE_DIR/adapters/kwin_script/" >/dev/null 2>&1 || \
        kpackagetool6 --type=KWin/Script -i "$SOURCE_DIR/adapters/kwin_script/" >/dev/null 2>&1 || true
        
        # Notificar a KWin para recargar la configuración de scripts en caliente
        qdbus6 org.kde.KWin /KWin reconfigure >/dev/null 2>&1 || true
        log_success "Script de KWin ('$KWIN_SCRIPT_ID') actualizado y recargado en Plasma 6"
    else
        log_error "No se encontró build_kwin_bundle.sh en $SOURCE_DIR"
        exit 1
    fi
}

# 3. Compilación e Instalación del Efecto C++ Nativo de KWin
do_build_kwin_effect() {
    print_header
    echo -e "${PRIMARY}${BOLD}🎬 Compilando e Instalando Efecto Nativo KWin C++ (adapters/kwin_effect)...${RESET}\n"

    if [ ! -d "$SOURCE_DIR/adapters/kwin_effect" ]; then
        log_error "No se encontró el directorio $SOURCE_DIR/adapters/kwin_effect"
        exit 1
    fi

    local num_cores=$(nproc 2>/dev/null || echo 2)
    log_info "Configurando CMake para kwin4_effect_raven..."
    cmake -B "$SOURCE_DIR/adapters/kwin_effect/build" -S "$SOURCE_DIR/adapters/kwin_effect" -DCMAKE_BUILD_TYPE=Release >/dev/null 2>&1 || true

    log_info "Compilando módulo C++..."
    if cmake --build "$SOURCE_DIR/adapters/kwin_effect/build" -j"$num_cores" 2>&1 | while read -r line; do
        if [[ "$line" =~ \[([0-9]+)%\][[:space:]]+Building[[:space:]]+CXX[[:space:]]+object[[:space:]]+(.*) ]]; then
            local pct="${BASH_REMATCH[1]}"
            local obj="${BASH_REMATCH[2]##*/}"
            echo -ne "\r\033[K ${SECONDARY}⚙${RESET} ${MUTED}[${pct}%] Compilando Efecto:${RESET} ${ACCENT}${BOLD}${obj}${RESET}"
        elif [[ "$line" =~ \[([0-9]+)%\][[:space:]]+Linking[[:space:]]+CXX[[:space:]]+shared[[:space:]]+module[[:space:]]+(.*) ]]; then
            local pct="${BASH_REMATCH[1]}"
            local mod="${BASH_REMATCH[2]##*/}"
            echo -ne "\r\033[K ${SUCCESS}🔗${RESET} ${MUTED}[${pct}%] Enlazando Efecto:${RESET} ${PRIMARY}${BOLD}${mod}${RESET}"
        fi
    done; then
        echo -ne "\r\033[K"
        log_success "Efecto C++ compilado exitosamente"
    else
        echo -ne "\r\033[K"
        log_error "Fallo en la compilación de C++. Ejecuta 'cmake --build build' en adapters/kwin_effect para depurar."
        exit 1
    fi

    local EFFECT_SO="$SOURCE_DIR/adapters/kwin_effect/build/kwin/effects/plugins/kwin4_effect_raven.so"
    if [ ! -f "$EFFECT_SO" ] && [ -f "$SOURCE_DIR/adapters/kwin_effect/build/kwin4_effect_raven.so" ]; then
        EFFECT_SO="$SOURCE_DIR/adapters/kwin_effect/build/kwin4_effect_raven.so"
    fi

    if [ ! -f "$EFFECT_SO" ]; then
        log_error "No se encontró el archivo binario generado kwin4_effect_raven.so"
        exit 1
    fi

    install_kwin_effect_binary "$EFFECT_SO"
}

# 4. Compilación e Instalación del Plasmoide / Raven Hub
do_build_plasmoid() {
    print_header
    echo -e "${PRIMARY}${BOLD}🧩 Compilando e Instalando Plasmoide / Raven Hub (adapters/plasmoid)...${RESET}\n"

    if [ ! -d "$SOURCE_DIR/adapters/plasmoid" ]; then
        log_error "No se encontró el directorio $SOURCE_DIR/adapters/plasmoid"
        exit 1
    fi

    local num_cores=$(nproc 2>/dev/null || echo 2)
    log_info "Configurando proyecto C++ / Qt 6 con CMake..."
    cmake -B "$SOURCE_DIR/adapters/plasmoid/build" -S "$SOURCE_DIR/adapters/plasmoid" -DCMAKE_BUILD_TYPE=Release >/dev/null 2>&1 || true

    log_info "Compilando Plugin Nativo C++ / QML de Raven Hub..."
    if cmake --build "$SOURCE_DIR/adapters/plasmoid/build" -j"$num_cores" 2>&1 | while read -r line; do
        if [[ "$line" =~ \[([0-9]+)%\][[:space:]]+Building[[:space:]]+CXX[[:space:]]+object[[:space:]]+(.*) ]]; then
            local pct="${BASH_REMATCH[1]}"
            local obj="${BASH_REMATCH[2]##*/}"
            echo -ne "\r\033[K ${SECONDARY}⚙${RESET} ${MUTED}[${pct}%] Compilando Plasmoide:${RESET} ${ACCENT}${BOLD}${obj}${RESET}"
        elif [[ "$line" =~ \[([0-9]+)%\][[:space:]]+Linking[[:space:]]+CXX[[:space:]]+shared[[:space:]]+module[[:space:]]+(.*) ]]; then
            local pct="${BASH_REMATCH[1]}"
            local mod="${BASH_REMATCH[2]##*/}"
            echo -ne "\r\033[K ${SUCCESS}🔗${RESET} ${MUTED}[${pct}%] Enlazando Plasmoide:${RESET} ${PRIMARY}${BOLD}${mod}${RESET}"
        fi
    done; then
        echo -ne "\r\033[K"
        cmake --install "$SOURCE_DIR/adapters/plasmoid/build" --prefix "$HOME/.local" >/dev/null 2>&1 || true
        log_success "Plugin C++ / QML compilado e instalado"
    else
        echo -ne "\r\033[K"
        log_error "Fallo en la compilación del Plasmoide. Ejecuta 'cmake --build build' en adapters/plasmoid para depurar."
        exit 1
    fi

    # Desplegar módulo en rutas locales de QML
    local build_qml_dir="$SOURCE_DIR/adapters/plasmoid/build/plugin/org/kde/plasma/ravenlauncher/plugin"
    if [ -d "$build_qml_dir" ]; then
        local arch_triplet="$(uname -m)-linux-gnu"
        local qml_paths=(
            "$HOME/.local/lib/${arch_triplet}/qml/org/kde/plasma/ravenlauncher/plugin"
            "$HOME/.local/lib/${arch_triplet}/qt6/qml/org/kde/plasma/ravenlauncher/plugin"
            "$HOME/.local/lib/qt6/qml/org/kde/plasma/ravenlauncher/plugin"
            "$HOME/.local/lib64/qt6/qml/org/kde/plasma/ravenlauncher/plugin"
            "$HOME/.local/share/qml/org/kde/plasma/ravenlauncher/plugin"
            "$SOURCE_DIR/adapters/plasmoid/package/contents/ui/org/kde/plasma/ravenlauncher/plugin"
        )
        cp -a "$SOURCE_DIR/adapters/plasmoid/plugin/RavenTheme.qml" "$build_qml_dir/" 2>/dev/null || true
        for p in "${qml_paths[@]}"; do
            mkdir -p "$p"
            cp -a "$build_qml_dir"/* "$p/" 2>/dev/null || true
        done
        local user_qml_path="$HOME/.local/lib/${arch_triplet}/qml:$HOME/.local/lib/${arch_triplet}/qt6/qml:$HOME/.local/lib/qt6/qml:$HOME/.local/lib64/qt6/qml:$HOME/.local/share/qml"
        systemctl --user set-environment QML2_IMPORT_PATH="$user_qml_path:$QML2_IMPORT_PATH" 2>/dev/null || true
        systemctl --user set-environment QML_IMPORT_PATH="$user_qml_path:$QML_IMPORT_PATH" 2>/dev/null || true
    fi

    log_info "Actualizando paquete de Plasmoide en Plasma 6..."
    kpackagetool6 --type=Plasma/Applet -r "org.kde.raven.toggle" >/dev/null 2>&1 || true
    rm -rf "$HOME/.local/share/plasma/plasmoids/org.kde.raven.toggle" 2>/dev/null || true

    kpackagetool6 --type=Plasma/Applet -u "$SOURCE_DIR/adapters/plasmoid/package" >/dev/null 2>&1 || \
    kpackagetool6 --type=Plasma/Applet -i "$SOURCE_DIR/adapters/plasmoid/package" >/dev/null 2>&1 || true
    log_success "Plasmoide de Plasma 6 ($PLASMOID_ID) actualizado e instalado"
}

# --- Lógica de Instalación Completa ---
do_install() {
    print_header
    echo -e "${PRIMARY}${BOLD}🚀 Iniciando Instalación Completa de Raven Tiling Emulator...${RESET}\n"

    # 1. Validación de Entorno
    log_step "1" "7" "Validación de Entorno y Sistema"
    if [[ "$XDG_CURRENT_DESKTOP" =~ KDE|Plasma ]] || [[ "$XDG_SESSION_DESKTOP" =~ KDE|Plasma ]] || [ "$KDE_FULL_SESSION" = "true" ]; then
        log_success "Entorno KDE Plasma detectado"
    else
        log_warning "No se detectó un entorno KDE Plasma activo. Asegúrate de estar corriendo Plasma 6."
    fi

    # 2. Compilación del Motor Rust
    log_step "2" "7" "Compilación e Instalación del Motor Rust"
    do_build_engine

    # 3. Iconos y Lanzadores Desktop
    log_step "3" "7" "Integración de Escritorio e Iconografía"
    mkdir -p "$HOME/.local/share/icons/hicolor/scalable/apps"
    mkdir -p "$HOME/.local/share/applications"

    if [ -f "$SOURCE_DIR/assets/icon_app/org.kde.raven.tiling.svg" ]; then
        cp "$SOURCE_DIR/assets/icon_app/org.kde.raven.tiling.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/${ICON_NAME}.svg"
    elif [ -f "$SOURCE_DIR/icon/org.kde.raven.tiling.svg" ]; then
        cp "$SOURCE_DIR/icon/org.kde.raven.tiling.svg" "$HOME/.local/share/icons/hicolor/scalable/apps/${ICON_NAME}.svg"
    fi

    cat <<EOF > "$HOME/.local/share/applications/raven.desktop"
[Desktop Entry]
Name=Raven Control Center
Comment=Configuración Nativa para Raven Tiling Emulator
Exec=$TARGET_DIR/bin/raven_gui
Icon=$ICON_NAME
Terminal=false
Type=Application
Categories=Utility;Settings;Qt;KDE;
EOF
    chmod +x "$HOME/.local/share/applications/raven.desktop"
    update-desktop-database "$HOME/.local/share/applications/" 2>/dev/null || true
    log_success "Lanzadores e icono registrados en el sistema"

    # 4. KWin Script Bundle
    log_step "4" "7" "Empaquetado y Registro de KWin Script"
    do_rebuild_kwin

    # 5. Efecto C++ Nativo de KWin
    log_step "5" "7" "Compilación y Despliegue de Efecto Nativo KWin C++"
    do_build_kwin_effect

    # 6. Plasmoide / Raven Hub
    log_step "6" "7" "Compilación y Despliegue del Plasmoide"
    do_build_plasmoid

    # 7. Atajos y Servicios Systemd
    log_step "7" "7" "Configuración de Atajos y Demonio Systemd"
    do_inject_shortcuts

    mkdir -p "$HOME/.config/systemd/user/"
    cat <<EOF > "$HOME/.config/systemd/user/raven.service"
[Unit]
Description=Raven Tiling Emulator Daemon (Native Rust)
PartOf=graphical-session.target
After=graphical-session.target
StartLimitIntervalSec=30
StartLimitBurst=5

[Service]
Type=dbus
BusName=org.kde.raven.Daemon
ExecStart=$TARGET_DIR/bin/raven_engine
WorkingDirectory=$TARGET_DIR
Restart=on-failure
RestartSec=1
Slice=session.slice
Nice=2
CPUWeight=100
MemoryHigh=96M
MemoryMax=192M
TasksMax=64
NoNewPrivileges=yes

[Install]
WantedBy=graphical-session.target
EOF

    mkdir -p "$HOME/.local/share/dbus-1/services/"
    cat <<EOF > "$HOME/.local/share/dbus-1/services/org.kde.raven.Daemon.service"
[D-BUS Service]
Name=org.kde.raven.Daemon
Exec=/usr/bin/systemctl --user start raven.service
SystemdService=raven.service
EOF
    systemctl --user daemon-reload
    systemctl --user enable --now raven.service >/dev/null 2>&1 || true
    log_success "Servicios raven.service y D-Bus activadores configurados y activos"

    echo ""
    draw_box "🎉 ¡INSTALACIÓN COMPLETADA EXITOSAMENTE!" \
        "${SUCCESS}${BOLD}• Motor Nativo Rust:${RESET}  Activo (raven_engine via Systemd)" \
        "${SUCCESS}${BOLD}• Centro de Control:${RESET}  Desplegado (Raven GUI - egui)" \
        "${SUCCESS}${BOLD}• Puente KWin:${RESET}       Empaquetado e Instalado en Plasma 6" \
        "${SUCCESS}${BOLD}• Efecto C++ KWin:${RESET}   Compilado e Instalado (kwin4_effect_raven)" \
        "${SUCCESS}${BOLD}• Plasmoide:${RESET}         Disponible en el panel" \
        "" \
        "${WARNING}💡 Nota:${RESET} Recuerda activar 'Raven Bridge' en:" \
        "${MUTED}   Preferencias del Sistema -> Scripts de KWin${RESET}"
    echo ""
}

# --- Lógica de Desinstalación Limpia ---
do_uninstall() {
    print_header
    echo -e "${ERROR}${BOLD}🗑️ Iniciando Desinstalación Completa de Raven Tiling Emulator...${RESET}\n"

    log_step "1" "5" "Deteniendo Servicio Nativo Systemd"
    if systemctl --user is-active --quiet raven.service 2>/dev/null; then
        systemctl --user stop raven.service
    fi

    if [ -f "$HOME/.config/systemd/user/raven.service" ]; then
        systemctl --user disable raven.service 2>/dev/null || true
        rm -f "$HOME/.config/systemd/user/raven.service"
        rm -f "$HOME/.local/share/dbus-1/services/org.kde.raven.Daemon.service"
        systemctl --user daemon-reload 2>/dev/null || true
        systemctl --user reset-failed 2>/dev/null || true
        log_success "Servicio Systemd y D-Bus eliminados"
    fi

    log_step "2" "5" "Removiendo Adaptadores KDE (KWin Script, Efecto C++, Plasmoide & QML)"
    # Descargar efecto de KWin dinámicamente si está cargado
    qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.unloadEffect kwin4_effect_raven >/dev/null 2>&1 || true

    kpackagetool6 --type=KWin/Script --remove "$KWIN_SCRIPT_ID" >/dev/null 2>&1 || true
    kpackagetool6 --type=Plasma/Applet --remove "$PLASMOID_ID" >/dev/null 2>&1 || true
    rm -rf "$HOME/.local/share/kwin/scripts/$KWIN_SCRIPT_ID"
    rm -rf "$HOME/.local/share/plasma/plasmoids/$PLASMOID_ID"
    rm -f "$HOME/.local/lib64/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so"
    rm -f "$HOME/.local/lib/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so"
    sudo rm -f "/usr/lib64/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so" "/usr/lib/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so" 2>/dev/null || true
    kwriteconfig6 --file kwinrc --group Plugins --key kwin4_effect_ravenEnabled false 2>/dev/null || true
    rm -rf "$HOME/.local/lib/qt6/qml/org/kde/plasma/ravenlauncher" 2>/dev/null || true
    rm -rf "$HOME/.local/lib64/qt6/qml/org/kde/plasma/ravenlauncher" 2>/dev/null || true
    rm -rf "$HOME/.local/share/qml/org/kde/plasma/ravenlauncher" 2>/dev/null || true
    qdbus6 org.kde.KWin /KWin reconfigure >/dev/null 2>&1 || true
    log_success "Adaptadores, efecto C++ y módulos QML de Plasma 6 removidos"

    log_step "3" "5" "Limpiando Accesos Directos, Enlaces e Iconos"
    rm -f "$HOME/.local/share/applications/raven.desktop"
    rm -f "$HOME/.local/share/icons/hicolor/scalable/apps/${ICON_NAME}.svg"
    rm -f "$HOME/.local/bin/raven_engine"
    rm -f "$HOME/.local/bin/raven_gui"
    update-desktop-database "$HOME/.local/share/applications/" 2>/dev/null || true
    kbuildsycoca6 --noincremental > /dev/null 2>&1 || true
    log_success "Archivos de escritorio, iconos y enlaces en ~/.local/bin/ limpios"

    log_step "4" "5" "Eliminando Binarios y Caché Local del Sistema"
    if [ -d "$TARGET_DIR" ]; then
        rm -rf "$TARGET_DIR"
        log_success "Directorio $TARGET_DIR eliminado"
    fi
    if [ -d "$HOME/.cache/raven" ]; then
        rm -rf "$HOME/.cache/raven"
        log_success "Caché e historial local (~/.cache/raven/) eliminados"
    fi

    log_step "5" "5" "Purgando Artefactos de Compilación Locales (Clean Caches)"
    if command -v cargo >/dev/null 2>&1 && [ -f "$SOURCE_DIR/Cargo.toml" ]; then
        cargo clean --manifest-path "$SOURCE_DIR/Cargo.toml" >/dev/null 2>&1 || true
    else
        rm -rf "$SOURCE_DIR/target"
    fi
    rm -rf "$SOURCE_DIR/adapters/kwin_effect/build" "$SOURCE_DIR/adapters/plasmoid/build" 2>/dev/null || true
    rm -f "$SOURCE_DIR/adapters/kwin_script/contents/code/main.js" 2>/dev/null || true
    log_success "Artefactos de compilación en el repositorio purgados completamente"

    echo ""
    read -p "❓ ¿Deseas eliminar también los archivos de configuración (~/.config/raven)? (s/N): " confirm
    if [[ $confirm == [sS] ]]; then
        rm -rf "$HOME/.config/raven/"
        log_success "Configuración borrada (~/.config/raven/)"
    fi

    echo ""
    draw_box "🧹 DESINSTALACIÓN COMPLETADA" \
        "${SUCCESS}Raven y todos sus componentes fueron removidos con éxito.${RESET}" \
        "${MUTED}¡Gracias por haber probado Raven Tiling Emulator! 🐦${RESET}"
    echo ""
}

# --- Estado del Sistema ---
do_status() {
    print_header
    echo -e "${PRIMARY}${BOLD}📊 Estado del Ecosistema Raven${RESET}\n"

    # Daemon Systemd
    if systemctl --user is-active --quiet raven.service 2>/dev/null; then
        echo -e " ${SUCCESS}●${RESET} ${BOLD}Servicio Systemd (raven.service):${RESET} ${SUCCESS}Activo (Running)${RESET}"
    else
        echo -e " ${ERROR}●${RESET} ${BOLD}Servicio Systemd (raven.service):${RESET} ${ERROR}Inactivo / Detenido${RESET}"
    fi

    # KWin Script
    if [ -d "$HOME/.local/share/kwin/scripts/$KWIN_SCRIPT_ID" ]; then
        echo -e " ${SUCCESS}●${RESET} ${BOLD}Adaptador KWin Script:${RESET}            ${SUCCESS}Instalado ($KWIN_SCRIPT_ID)${RESET}"
    else
        echo -e " ${WARNING}●${RESET} ${BOLD}Adaptador KWin Script:${RESET}            ${WARNING}No instalado${RESET}"
    fi

    # KWin Effect
    local effect_status="${WARNING}No instalado${RESET}"
    if [ -f "/usr/lib64/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so" ] || \
       [ -f "/usr/lib/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so" ]; then
        effect_status="${SUCCESS}Instalado en Sistema (/usr/lib...)${RESET}"
    elif [ -f "$HOME/.local/lib64/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so" ] || \
         [ -f "$HOME/.local/lib/qt6/plugins/kwin/effects/plugins/kwin4_effect_raven.so" ]; then
        effect_status="${SUCCESS}Instalado Localmente (~/.local/...)${RESET}"
    fi
    echo -e " ${SUCCESS}●${RESET} ${BOLD}Efecto Nativo KWin C++:${RESET}          $effect_status"

    # Verificar si está cargado en el compositor
    if qdbus6 org.kde.KWin /Effects org.kde.kwin.Effects.isEffectLoaded kwin4_effect_raven 2>/dev/null | grep -q "true"; then
        echo -e "   ${MUTED}↳ Estado en compositor:${RESET} ${SUCCESS}Cargado y Activo en KWin${RESET}"
    else
        echo -e "   ${MUTED}↳ Estado en compositor:${RESET} ${MUTED}No cargado en este momento${RESET}"
    fi

    # Plasmoid
    if [ -d "$HOME/.local/share/plasma/plasmoids/$PLASMOID_ID" ]; then
        echo -e " ${SUCCESS}●${RESET} ${BOLD}Plasmoide de Panel:${RESET}               ${SUCCESS}Instalado ($PLASMOID_ID)${RESET}"
    else
        echo -e " ${WARNING}●${RESET} ${BOLD}Plasmoide de Panel:${RESET}               ${WARNING}No instalado${RESET}"
    fi

    # Binarios
    if [ -f "$TARGET_DIR/bin/raven_engine" ] && [ -f "$TARGET_DIR/bin/raven_gui" ]; then
        echo -e " ${SUCCESS}●${RESET} ${BOLD}Binarios Raven:${RESET}                   ${SUCCESS}Presentes en $TARGET_DIR/bin/${RESET}"
    else
        echo -e " ${ERROR}●${RESET} ${BOLD}Binarios Raven:${RESET}                   ${ERROR}Ausentes o incompletos${RESET}"
    fi

    echo ""
}

# --- Manejo de Argumentos CLI Directos ---
case "$1" in
    --install|-i)
        do_install
        exit 0
        ;;
    --shortcuts|-k)
        do_inject_shortcuts
        exit 0
        ;;
    --rebuild-engine|--rebuild|-r)
        do_build_engine
        exit 0
        ;;
    --bundle|-b)
        do_rebuild_kwin
        exit 0
        ;;
    --effect|-e)
        do_build_kwin_effect
        exit 0
        ;;
    --plasmoid|-p)
        do_build_plasmoid
        exit 0
        ;;
    --status|-s)
        do_status
        exit 0
        ;;
    --uninstall|-u)
        do_uninstall
        exit 0
        ;;
esac

# --- Menú Interactivo TUI ---
show_menu() {
    print_header
    draw_box "SELECCIONA UNA OPCIÓN" \
        "${ACCENT}${BOLD}[1]${RESET} 🚀 Instalación Completa ${MUTED}(Todos los componentes: Rust + KWin + Efecto + Plasmoide)${RESET}" \
        "${ACCENT}${BOLD}[2]${RESET} ⌨️ Inyectar Atajos de Teclado en Plasma ${MUTED}(Sincronizar kglobalshortcutsrc)${RESET}" \
        "${ACCENT}${BOLD}[3]${RESET} 🦀 Recompilar Motor y GUI Rust ${MUTED}(raven_engine & raven_gui + restart raven.service)${RESET}" \
        "${ACCENT}${BOLD}[4]${RESET} 📜 Reconstruir Bundle KWin Script ${MUTED}(build_kwin_bundle.sh + kpackagetool6 + reconfigure)${RESET}" \
        "${ACCENT}${BOLD}[5]${RESET} 🎬 Compilar e Instalar Efecto Nativo KWin ${MUTED}(kwin4_effect_raven C++ + recarga dinámica)${RESET}" \
        "${ACCENT}${BOLD}[6]${RESET} 🧩 Compilar e Instalar Plasmoide / Raven Hub ${MUTED}(adapters/plasmoid C++/QML)${RESET}" \
        "${ACCENT}${BOLD}[7]${RESET} 📊 Ver Estado del Sistema ${MUTED}(Systemd / KWin / Efecto / Plasmoid / Binarios)${RESET}" \
        "${ACCENT}${BOLD}[8]${RESET} 🗑️ Desinstalación Completa ${MUTED}(Remover adaptadores, servicios, binarios y build caches)${RESET}" \
        "${ACCENT}${BOLD}[9]${RESET} ❌ Salir"
    echo ""
    read -p " Ingrese opción [1-9]: " opt
    case "$opt" in
        1) do_install ;;
        2) do_inject_shortcuts ;;
        3) do_build_engine ;;
        4) do_rebuild_kwin ;;
        5) do_build_kwin_effect ;;
        6) do_build_plasmoid ;;
        7) do_status ;;
        8) do_uninstall ;;
        9) echo -e "\n${PRIMARY}¡Hasta luego! 🐦${RESET}\n"; exit 0 ;;
        *) echo -e "\n${ERROR}Opción inválida.${RESET}"; sleep 1; show_menu ;;
    esac
}

show_menu

