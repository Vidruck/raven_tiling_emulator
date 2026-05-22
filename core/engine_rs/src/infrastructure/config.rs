use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

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
    /// Posición por defecto para ventanas Picture-in-Picture (PiP).
    pub pip_position: String,
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
            pip_position: String::from("Bottom-right"),
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

                println!("[RUST_CONFIG] Preferencias cargadas con éxito desde disco.");
                return config;
            } else {
                println!("[RUST_CONFIG] Error de formato JSON. Usando Fallback nativo.");
            }
        } else {
            println!(
                "[RUST_CONFIG] No se encontró archivo de configuración. Usando Fallback nativo."
            );
        }
        RavenConfig::new()
    }
}
