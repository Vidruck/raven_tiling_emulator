#!/bin/bash
# ==============================================================================
#  🐦 RAVEN TILING EMULATOR — Interactive TUI Management Suite (v3.4)
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

draw_box() {
    local title="$1"
    shift
    local lines=("$@")
    local max_len=${#title}
    
    for line in "${lines[@]}"; do
        # Remover códigos ANSI para calcular longitud real
        local clean_line=$(echo -e "$line" | sed 's/\x1b\[[0-9;]*m//g')
        if [ ${#clean_line} -gt $max_len ]; then
            max_len=${#clean_line}
        fi
    done
    
    local width=$((max_len + 4))
    
    # Borde superior
    echo -e "${PRIMARY}╭$(printf '─%.0s' $(seq 1 $width))╮${RESET}"
    if [ -n "$title" ]; then
        echo -e "${PRIMARY}│ ${ACCENT}${BOLD}$title${RESET}$(printf ' %.0s' $(seq 1 $((width - ${#title} - 1))))${PRIMARY}│${RESET}"
        echo -e "${PRIMARY}├$(printf '─%.0s' $(seq 1 $width))┤${RESET}"
    fi
    
    # Contenido
    for line in "${lines[@]}"; do
        local clean_line=$(echo -e "$line" | sed 's/\x1b\[[0-9;]*m//g')
        local pad=$((width - ${#clean_line} - 2))
        echo -e "${PRIMARY}│${RESET} $line$(printf ' %.0s' $(seq 1 $pad))${PRIMARY}│${RESET}"
    done
    
    # Borde inferior
    echo -e "${PRIMARY}╰$(printf '─%.0s' $(seq 1 $width))╯${RESET}"
}

print_header() {
    clear 2>/dev/null || true
    echo -e "${SECONDARY}${BOLD}"
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
    echo -e "${PRIMARY}${BOLD} 🐦 Raven Tiling Emulator v3.0 — Suite de Gestión Interactiva 🐦${RESET}"
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

# --- Lógica de Instalación ---
do_install() {
    print_header
    echo -e "${PRIMARY}${BOLD}🚀 Iniciando Instalación Completa de Raven Tiling Emulator...${RESET}\n"

    # 1. Comprobación de Requisitos
    log_step "1" "7" "Validación de Entorno y Sistema"
    if [[ "$XDG_CURRENT_DESKTOP" =~ KDE|Plasma ]] || [[ "$XDG_SESSION_DESKTOP" =~ KDE|Plasma ]] || [ "$KDE_FULL_SESSION" = "true" ]; then
        log_success "Entorno KDE Plasma detectado"
    else
        log_warning "No se detectó un entorno KDE Plasma activo. Asegúrate de estar corriendo Plasma 6."
    fi

    if ! command -v cargo >/dev/null 2>&1; then
        log_error "Cargo (Rust) no está instalado. Por favor instala Rust: https://rustup.rs"
        exit 1
    fi
    log_success "Compilador Rust (Cargo) disponible"

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
    
    if [ -f "$SOURCE_DIR/icon/org.kde.raven.tiling.svg" ]; then
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
        local qml_paths=(
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
        local user_qml_path="$HOME/.local/lib/qt6/qml:$HOME/.local/lib64/qt6/qml:$HOME/.local/share/qml"
        systemctl --user set-environment QML2_IMPORT_PATH="$user_qml_path:$QML2_IMPORT_PATH" 2>/dev/null || true
        systemctl --user set-environment QML_IMPORT_PATH="$user_qml_path:$QML_IMPORT_PATH" 2>/dev/null || true
    fi
    log_success "Módulos QML de Raven Hub sincronizados"

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
After=graphical-session.target

[Service]
ExecStart=$TARGET_DIR/bin/raven_engine
WorkingDirectory=$TARGET_DIR
Restart=always
RestartSec=3

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
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenToggleTiling" "Meta+Space,none,Raven: Alternar Mosaico (On/Off)"
        kwriteconfig6 --file "$SHORTCUTS_FILE" --group "kwin" --key "RavenToggleFloating" "Meta+Shift+F,none,Raven: Alternar Ventana Flotante Dinámica"
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

# --- Lógica de Reconstrucción del Bundle KWin ---
do_rebuild_kwin() {
    print_header
    echo -e "${PRIMARY}${BOLD}🎨 Reconstruyendo Bundle KWin (main.js)...${RESET}\n"
    if [ -f "$SOURCE_DIR/build_kwin_bundle.sh" ]; then
        bash "$SOURCE_DIR/build_kwin_bundle.sh"
        log_info "Actualizando paquete de KWin..."
        kpackagetool6 --type=KWin/Script -u "$SOURCE_DIR/adapters/kwin_script/" >/dev/null 2>&1 || true
        log_success "Script de KWin actualizado en Plasma 6"
    else
        log_error "No se encontró build_kwin_bundle.sh"
    fi
}

# --- Lógica de Recompilación Rápida ---
do_quick_rebuild() {
    print_header
    echo -e "${PRIMARY}${BOLD}🔄 Recompilando Componentes de Raven...${RESET}\n"
    cargo build --release --manifest-path "$SOURCE_DIR/Cargo.toml"
    systemctl --user stop raven.service 2>/dev/null || true
    mkdir -p "$TARGET_DIR/bin"
    install -m 755 "$SOURCE_DIR/target/release/raven_engine" "$TARGET_DIR/bin/raven_engine"
    install -m 755 "$SOURCE_DIR/target/release/raven_gui" "$TARGET_DIR/bin/raven_gui"
    log_success "Binarios de Rust actualizados en $TARGET_DIR/bin/"
    
    do_rebuild_kwin
    
    systemctl --user restart raven.service 2>/dev/null || true
    log_success "Servicio raven.service reiniciado"
}

# --- Lógica de Desinstalación ---
do_uninstall() {
    print_header
    echo -e "${ERROR}${BOLD}🗑️ Iniciando Desinstalación de Raven Tiling Emulator...${RESET}\n"

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

    log_step "2" "5" "Removiendo Adaptadores KDE (KWin Script, Plasmoide & Módulos QML)"
    kpackagetool6 --type=KWin/Script --remove "$KWIN_SCRIPT_ID" >/dev/null 2>&1 || true
    kpackagetool6 --type=Plasma/Applet --remove "$PLASMOID_ID" >/dev/null 2>&1 || true
    rm -rf "$HOME/.local/share/kwin/scripts/$KWIN_SCRIPT_ID"
    rm -rf "$HOME/.local/share/plasma/plasmoids/$PLASMOID_ID"
    rm -rf "$HOME/.local/lib/qt6/qml/org/kde/plasma/ravenlauncher" 2>/dev/null || true
    rm -rf "$HOME/.local/lib64/qt6/qml/org/kde/plasma/ravenlauncher" 2>/dev/null || true
    rm -rf "$HOME/.local/share/qml/org/kde/plasma/ravenlauncher" 2>/dev/null || true
    log_success "Adaptadores y módulos QML de Plasma 6 removidos"

    log_step "3" "5" "Limpiando Accesos Directos, Enlaces e Iconos"
    rm -f "$HOME/.local/share/applications/raven.desktop"
    rm -f "$HOME/.local/share/icons/hicolor/scalable/apps/${ICON_NAME}.svg"
    rm -f "$HOME/.local/bin/raven_engine"
    rm -f "$HOME/.local/bin/raven_gui"
    update-desktop-database "$HOME/.local/share/applications/" 2>/dev/null || true
    kbuildsycoca6 --noincremental > /dev/null 2>&1 || true
    log_success "Archivos de escritorio, iconos y enlaces en ~/.local/bin/ limpios"

    log_step "4" "5" "Eliminando Binarios y Caché Local"
    if [ -d "$TARGET_DIR" ]; then
        rm -rf "$TARGET_DIR"
        log_success "Directorio $TARGET_DIR eliminado"
    fi
    if [ -d "$HOME/.cache/raven" ]; then
        rm -rf "$HOME/.cache/raven"
        log_success "Caché e historial local (~/.cache/raven/) eliminados"
    fi

    log_step "5" "5" "Purgando Artefactos de Compilación"
    if command -v cargo >/dev/null 2>&1 && [ -f "$SOURCE_DIR/Cargo.toml" ]; then
        cargo clean --manifest-path "$SOURCE_DIR/Cargo.toml" >/dev/null 2>&1 || true
    else
        rm -rf "$SOURCE_DIR/target"
    fi
    log_success "Artefactos de compilación limpios"

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
        echo -e " ${SUCCESS}●${RESET} ${BOLD}Adaptador KWin Script:${RESET}            ${SUCCESS}Instalado${RESET}"
    else
        echo -e " ${WARNING}●${RESET} ${BOLD}Adaptador KWin Script:${RESET}            ${WARNING}No instalado${RESET}"
    fi

    # Plasmoid
    if [ -d "$HOME/.local/share/plasma/plasmoids/$PLASMOID_ID" ]; then
        echo -e " ${SUCCESS}●${RESET} ${BOLD}Plasmoide de Panel:${RESET}               ${SUCCESS}Instalado${RESET}"
    else
        echo -e " ${WARNING}●${RESET} ${BOLD}Plasmoide de Panel:${RESET}               ${WARNING}No instalado${RESET}"
    fi

    # Binarios
    if [ -f "$TARGET_DIR/bin/raven_engine" ]; then
        echo -e " ${SUCCESS}●${RESET} ${BOLD}Binario raven_engine:${RESET}             ${SUCCESS}Presente en $TARGET_DIR/bin/${RESET}"
    else
        echo -e " ${ERROR}●${RESET} ${BOLD}Binario raven_engine:${RESET}             ${ERROR}Ausente${RESET}"
    fi

    echo ""
}

# --- Manejo de Argumentos CLI Directos ---
case "$1" in
    --install|-i)
        do_install
        exit 0
        ;;
    --uninstall|-u)
        do_uninstall
        exit 0
        ;;
    --rebuild|-r)
        do_quick_rebuild
        exit 0
        ;;
    --bundle|-b)
        do_rebuild_kwin
        exit 0
        ;;
    --shortcuts|-k)
        do_inject_shortcuts
        exit 0
        ;;
    --status|-s)
        do_status
        exit 0
        ;;
esac

# --- Menú Interactivo TUI ---
show_menu() {
    print_header
    draw_box "SELECCIONA UNA OPCIÓN" \
        "${ACCENT}${BOLD}[1]${RESET} 🚀 Instalación Completa ${MUTED}(Compilar + Desplegar + Inyectar Atajos + Iniciar)${RESET}" \
        "${ACCENT}${BOLD}[2]${RESET} ⌨️ Inyectar Atajos de Teclado en Plasma ${MUTED}(Sincronizar kglobalshortcutsrc)${RESET}" \
        "${ACCENT}${BOLD}[3]${RESET} 🔄 Recompilación Rápida ${MUTED}(Rebuild Cargo + Bundle KWin + Restart)${RESET}" \
        "${ACCENT}${BOLD}[4]${RESET} 🎨 Reconstruir Bundle KWin ${MUTED}(build_kwin_bundle.sh + Update)${RESET}" \
        "${ACCENT}${BOLD}[5]${RESET} 📊 Ver Estado del Sistema ${MUTED}(Systemd / KWin / Plasmoid / Files)${RESET}" \
        "${ACCENT}${BOLD}[6]${RESET} 🗑️ Desinstalar Raven ${MUTED}(Remover adaptadores, servicios y binarios)${RESET}" \
        "${ACCENT}${BOLD}[7]${RESET} ❌ Salir"
    echo ""
    read -p " Ingrese opción [1-7]: " opt
    case "$opt" in
        1) do_install ;;
        2) do_inject_shortcuts ;;
        3) do_quick_rebuild ;;
        4) do_rebuild_kwin ;;
        5) do_status ;;
        6) do_uninstall ;;
        7) echo -e "\n${PRIMARY}¡Hasta luego! 🐦${RESET}\n"; exit 0 ;;
        *) echo -e "\n${ERROR}Opción inválida.${RESET}"; sleep 1; show_menu ;;
    esac
}

show_menu
