//! # Capa de Servicios y Persistencia del Sistema (`services.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Controla el ciclo de vida del servicio `raven.service` vía `systemctl --user`,
//! regeneración atómica de configuración JSON y emisión de señales de recarga.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use raven_core::config::RavenConfig;

/// Gestor de persistencia y control de demonio systemd.
pub struct ServiceManager {
    pub config_path: PathBuf,
    pub is_active_cache: bool,
    pub last_active_check: Option<Instant>,
}

impl ServiceManager {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            is_active_cache: false,
            last_active_check: None,
        }
    }

    pub fn is_service_active(&mut self) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_active_check {
            if now.duration_since(last) < Duration::from_secs(2) {
                return self.is_active_cache;
            }
        }
        
        let active = Command::new("systemctl")
            .args(["--user", "is-active", "raven.service"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
            .unwrap_or(false);
            
        self.is_active_cache = active;
        self.last_active_check = Some(now);
        active
    }

    pub fn start_service(&mut self) -> String {
        let ok = Command::new("systemctl")
            .args(["--user", "start", "raven.service"])
            .status()
            .is_ok();
        self.last_active_check = None;
        if ok {
            "✅ Motor Raven iniciado con éxito.".to_string()
        } else {
            "❌ Error al iniciar el servicio.".to_string()
        }
    }

    pub fn stop_service(&mut self) -> String {
        let ok = Command::new("systemctl")
            .args(["--user", "stop", "raven.service"])
            .status()
            .is_ok();
        self.last_active_check = None;
        if ok {
            "🛑 Motor Raven apagado con éxito.".to_string()
        } else {
            "❌ Error al detener el servicio.".to_string()
        }
    }

    pub fn save_and_restart(&mut self, config: &RavenConfig) -> String {
        if let Some(parent) = self.config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(config) {
            if fs::write(&self.config_path, json).is_ok() {
                if self.is_service_active() {
                    let ok = Command::new("systemctl")
                        .args(["--user", "restart", "raven.service"])
                        .status()
                        .is_ok();
                    if ok {
                        "✅ Configuración guardada y servicio reiniciado.".to_string()
                    } else {
                        "❌ Error al reiniciar el servicio.".to_string()
                    }
                } else {
                    "✅ Configuración guardada correctamente.".to_string()
                }
            } else {
                "❌ Error al guardar la configuración.".to_string()
            }
        } else {
            "❌ Error al serializar la configuración.".to_string()
        }
    }

    pub fn get_recent_logs(&self) -> Vec<String> {
        let output = Command::new("journalctl")
            .args(["--user", "-u", "raven.service", "-n", "5", "--no-pager"])
            .output();

        match output {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout);
                text.lines()
                    .map(|l| l.to_string())
                    .filter(|l| !l.trim().is_empty())
                    .collect()
            }
            Err(_) => vec!["[Error al consultar journalctl]".to_string()],
        }
    }
}
