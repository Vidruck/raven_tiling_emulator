use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WindowRule {
    pub class: String,
    pub action: String,
    #[serde(default)]
    pub pip: bool,
}

/// Estructura que define la configuración del gestor de ventanas Raven.
///
/// Contiene parámetros ajustables para el comportamiento del mosaico (tiling), gaps,
/// y configuraciones de ventanas flotantes (PiP).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RavenConfig {
    /// Espacio (gap) por defecto entre ventanas en píxeles.
    pub default_gaps: i32,
    /// Indica si el motor de mosaico (tiling engine) debe habilitarse automáticamente al iniciar.
    pub tiling_enabled_on_startup: bool,
    /// Número óptimo de ventanas en el área de mosaico Dwindle BSP antes de desalojar (eviction).
    pub nmaster: usize,
    /// Proporción del área de corte asimétrico (0.0 a 1.0) para la espiral (dwindle).
    pub master_ratio: f32,
    /// Posición por defecto para ventanas Picture-in-Picture (PiP). Valores: top-left, top-right, bottom-left, bottom-right.
    pub pip_position: String,
    /// Altura del panel persistente del shell en píxeles (usado para calcular el límite de saturación de pantalla).
    pub panel_height: i32,
    /// Preset de layout activo. Valores: dense, aesthetic, functional, balanced, simple.
    pub layout_preset: String,
    /// Clases de ventanas que no deben ser cacheadas inmediatamente al ser creadas (CSD issues, navegadores, etc).
    #[serde(default = "default_quarantine")]
    pub quarantine_classes: Vec<String>,
    /// Reglas dinámicas por clase de aplicación.
    #[serde(default)]
    pub window_rules: Vec<WindowRule>,
}

fn default_quarantine() -> Vec<String> {
    vec![
        "firefox".into(),
        "electron".into(),
        "zen-browser".into(),
        "code".into(),
        "spotify".into(),
        "floorp".into(),
        "chrome".into(),
    ]
}

impl RavenConfig {
    /// Crea una nueva instancia de `RavenConfig` con valores predeterminados.
    ///
    /// # Retorno
    /// Una instancia de `RavenConfig` con la configuración base.
    pub fn new() -> Self {
        RavenConfig {
            default_gaps: 8,
            tiling_enabled_on_startup: true,
            nmaster: 1,
            master_ratio: 0.5,
            // FIX BUG-02: normalizado a minúsculas para que coincida con el match en layout.rs
            pip_position: String::from("bottom-right"),
            panel_height: 30,
            layout_preset: String::from("balanced"),
            quarantine_classes: default_quarantine(),
            window_rules: vec![],
        }
    }

    /// Carga la configuración desde el archivo JSON en el sistema de archivos.
    ///
    /// Busca el archivo en `~/.config/raven/raven.json`. Si no existe o hay
    /// un error en el formato, retorna la configuración por defecto.
    ///
    /// # Retorno
    /// La configuración cargada o los valores por defecto en caso de fallo.
    pub fn load() -> Self {
        let home = env::var("HOME").unwrap_or_else(|_| String::from("~"));
        let mut path = PathBuf::from(home);
        path.push(".config");
        path.push("raven");
        path.push("raven.json");

        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(mut config) = serde_json::from_str::<RavenConfig>(&content) {
                config.nmaster = std::cmp::max(1, config.nmaster);
                config.master_ratio = config.master_ratio.clamp(0.1, 0.9);
                config.default_gaps = std::cmp::max(0, config.default_gaps);
                // Normalizar pip_position a minúsculas para garantizar compatibilidad
                config.pip_position = config.pip_position.to_lowercase();
                config.panel_height = config.panel_height.clamp(20, 120);

                info!("[CONFIG] Preferencias cargadas con éxito desde disco.");
                return config;
            } else {
                warn!("[CONFIG] Error de formato JSON. Usando Fallback nativo.");
            }
        } else {
            info!(
                "[CONFIG] No se encontró archivo de configuración. Usando Fallback nativo."
            );
        }
        RavenConfig::new()
    }
}
