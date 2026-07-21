pub use raven_core::config::{RavenConfig, WindowRule};
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

pub trait RavenConfigExt {
    fn load() -> Self;
}

impl RavenConfigExt for RavenConfig {
    fn load() -> Self {
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


                info!("[CONFIG] Preferencias cargadas con éxito desde disco.");
                return config;
            } else {
                warn!("[CONFIG] Error de formato JSON. Usando Fallback nativo.");
            }
        } else {
            info!("[CONFIG] No se encontró archivo de configuración. Usando Fallback nativo.");
        }
        RavenConfig::new()
    }
}
