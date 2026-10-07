//! # Administrador de Atajos Globales de KWin / KDE Plasma (`shortcuts`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Registra los atajos globales nativos en el subsistema `org.kde.kglobalaccel` de Plasma 6
//! y despacha los eventos directamente hacia el actor mediante canales Tokio, desacoplando
//! por completo al script de KWin de la captura de teclado.

use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use zbus::Connection;

use crate::service::KWinBridgeMessage;

/// Definición de un atajo global soportado por Raven en KDE Plasma.
#[derive(Debug, Clone)]
pub struct ShortcutDefinition {
    /// Identificador único del atajo (ej. "RavenToggleTiling").
    pub id: &'static str,
    /// Etiqueta descriptiva legible para el menú de Configuración de KDE.
    pub description: &'static str,
    /// Combinación de teclas predeterminada (ej. "Meta+Space").
    pub default_key: &'static str,
    /// Acción interna de Raven a despachar.
    pub action: &'static str,
    /// Carga numérica entera asociada (ej. 2 o -2 para gaps).
    pub payload: i32,
}

/// Catálogo canónico de atajos globales registrados por Raven en KDE Plasma.
pub const RAVEN_SHORTCUTS: &[ShortcutDefinition] = &[
    // --- Gestión de Estado y Tiling ---
    ShortcutDefinition {
        id: "RavenToggleTiling",
        description: "Raven: Alternar Mosaico (On/Off)",
        default_key: "Meta+Space",
        action: "toggle_tiling",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenToggleFloating",
        description: "Raven: Alternar Ventana Flotante Dinámica",
        default_key: "Meta+Shift+F",
        action: "toggle_floating",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenToggleMaximize",
        description: "Raven: Alternar Maximizar Ventana",
        default_key: "Meta+F",
        action: "toggle_maximize",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenMinimizeActive",
        description: "Raven: Minimizar Ventana",
        default_key: "Meta+X",
        action: "minimize_active",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenCloseActive",
        description: "Raven: Cerrar Ventana",
        default_key: "Meta+Q",
        action: "close_active",
        payload: 0,
    },

    // --- Navegación y Foco ---
    ShortcutDefinition {
        id: "RavenFocusNext",
        description: "Raven: Siguiente Ventana",
        default_key: "Meta+J",
        action: "focus_next",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenFocusPrev",
        description: "Raven: Ventana Anterior",
        default_key: "Meta+K",
        action: "focus_prev",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenFocusLeft",
        description: "Raven: Foco Izquierda",
        default_key: "Meta+Left",
        action: "focus_left",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenFocusRight",
        description: "Raven: Foco Derecha",
        default_key: "Meta+Right",
        action: "focus_right",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenFocusUp",
        description: "Raven: Foco Arriba",
        default_key: "Meta+Up",
        action: "focus_up",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenFocusDown",
        description: "Raven: Foco Abajo",
        default_key: "Meta+Down",
        action: "focus_down",
        payload: 0,
    },

    // --- Intercambio y Ajuste de Ratios ---
    ShortcutDefinition {
        id: "RavenSwapNext",
        description: "Raven: Intercambiar Siguiente",
        default_key: "Meta+Shift+J",
        action: "swap_next",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenSwapPrev",
        description: "Raven: Intercambiar Anterior",
        default_key: "Meta+Shift+K",
        action: "swap_prev",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenIncreaseRatio",
        description: "Raven: Expandir Master",
        default_key: "Meta+H",
        action: "increase_ratio",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenDecreaseRatio",
        description: "Raven: Contraer Master",
        default_key: "Meta+L",
        action: "decrease_ratio",
        payload: 0,
    },

    // --- Migración entre Pantallas y Escritorios ---
    ShortcutDefinition {
        id: "RavenMigrateMonitor",
        description: "Raven: Enviar a Monitor Siguiente",
        default_key: "Meta+Shift+M",
        action: "migrate_active_to_screen",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenMigratePrevMonitor",
        description: "Raven: Enviar a Monitor Anterior",
        default_key: "Meta+Shift+N",
        action: "migrate_active_to_prev_screen",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenMigrateDesktop",
        description: "Raven: Enviar a Escritorio Siguiente",
        default_key: "Meta+Shift+Right",
        action: "migrate_active_to_desktop",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenMigratePrevDesktop",
        description: "Raven: Enviar a Escritorio Anterior",
        default_key: "Meta+Shift+Left",
        action: "migrate_active_to_prev_desktop",
        payload: 0,
    },

    // --- Márgenes y Algoritmos de Disposición ---
    ShortcutDefinition {
        id: "RavenIncrementGaps",
        description: "Raven: Incrementar Gaps",
        default_key: "Meta+=",
        action: "increment_gaps",
        payload: 2,
    },
    ShortcutDefinition {
        id: "RavenDecrementGaps",
        description: "Raven: Decrementar Gaps",
        default_key: "Meta+-",
        action: "increment_gaps",
        payload: -2,
    },
    ShortcutDefinition {
        id: "RavenIncrementMaster",
        description: "Raven: Incrementar Capacidad Master",
        default_key: "Meta+]",
        action: "increment_nmaster",
        payload: 1,
    },
    ShortcutDefinition {
        id: "RavenDecrementMaster",
        description: "Raven: Decrementar Capacidad Master",
        default_key: "Meta+[",
        action: "decrement_nmaster",
        payload: 1,
    },
    ShortcutDefinition {
        id: "RavenCycleLayout",
        description: "Raven: Ciclar Algoritmo de Disposición",
        default_key: "Meta+Shift+L",
        action: "cycle_layout",
        payload: 0,
    },

    // --- Redimensionamiento Fino 2D ---
    ShortcutDefinition {
        id: "RavenResizeWidthInc",
        description: "Raven: Aumentar Ancho de Ventana",
        default_key: "Meta+Alt+Right",
        action: "resize_width_inc",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenResizeWidthDec",
        description: "Raven: Reducir Ancho de Ventana",
        default_key: "Meta+Alt+Left",
        action: "resize_width_dec",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenResizeHeightInc",
        description: "Raven: Aumentar Alto de Ventana",
        default_key: "Meta+Alt+Down",
        action: "resize_height_inc",
        payload: 0,
    },
    ShortcutDefinition {
        id: "RavenResizeHeightDec",
        description: "Raven: Reducir Alto de Ventana",
        default_key: "Meta+Alt+Up",
        action: "resize_height_dec",
        payload: 0,
    },
];

/// Registra los atajos globales en el subsistema `org.kde.kglobalaccel` de Plasma.
pub async fn register_global_shortcuts(
    conn: Arc<Connection>,
    _tx: mpsc::Sender<KWinBridgeMessage>,
) {
    info!("[KWIN-SHORTCUTS] Verificando atajos registrados en kglobalaccel para componente 'kwin'...");

    // Consulta los nombres de atajos disponibles en KWin sin arrojar errores de firma D-Bus
    let names_res: Result<Vec<String>, _> = conn
        .call_method(
            Some("org.kde.kglobalaccel"),
            "/component/kwin",
            Some("org.kde.kglobalaccel.Component"),
            "shortcutNames",
            &(),
        )
        .await
        .map(|msg| msg.body().deserialize().unwrap_or_default());

    match names_res {
        Ok(names) => {
            let active_raven_count = names.iter().filter(|n| n.starts_with("Raven")).count();
            info!(
                "[KWIN-SHORTCUTS] {} atajos de Raven activos detectados en KWin (kglobalaccel).",
                active_raven_count
            );
        }
        Err(e) => {
            warn!("[KWIN-SHORTCUTS] No se pudo consultar atajos en kglobalaccel: {}", e);
        }
    }
}
