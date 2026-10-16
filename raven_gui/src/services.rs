//! # Capa de Servicios y Persistencia del Sistema (`services.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
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
    pub displays_cache: Vec<raven_core::geometry::OutputNode>,
    pub last_displays_check: Option<Instant>,
    pub desktops_cache: Vec<DesktopInfo>,
    pub last_desktops_check: Option<Instant>,
}

impl ServiceManager {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            is_active_cache: false,
            last_active_check: None,
            displays_cache: Vec::new(),
            last_displays_check: None,
            desktops_cache: Vec::new(),
            last_desktops_check: None,
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

        // Candado y Sanitización: Crear copia limpia sin claves vacías
        let mut clean_config = config.clone();
        clean_config.workspace_layouts.retain(|k, v| !k.trim().is_empty() && !v.trim().is_empty());

        if let Ok(json) = serde_json::to_string_pretty(&clean_config) {
            // Escritura atómica vía archivo temporal para prevenir lecturas parciales o corrupción
            let temp_path = self.config_path.with_extension("tmp");
            if fs::write(&temp_path, json).is_ok() && fs::rename(&temp_path, &self.config_path).is_ok() {
                // Notificar en caliente al efecto C++ de KWin directamente vía D-Bus (busctl)
                let _ = Command::new("busctl")
                    .args([
                        "--user",
                        "call",
                        "org.kde.kwin.RavenEffect",
                        "/Effects/Raven",
                        "org.kde.kwin.RavenEffect",
                        "UpdateBorderConfig",
                        "bdss",
                        if clean_config.borders_enabled { "true" } else { "false" },
                        &clean_config.border_width.to_string(),
                        &clean_config.active_border_color,
                        &clean_config.inactive_border_color,
                    ])
                    .output();

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
                "❌ Error al guardar la configuración en disco.".to_string()
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

    /// Obtiene las salidas físicas de pantalla detectadas por Wayland/KWin (usando kscreen-doctor o fallback con caché).
    pub fn get_displays(&mut self) -> Vec<raven_core::geometry::OutputNode> {
        use raven_core::geometry::{OutputNode, Rect};
        let now = Instant::now();
        if let Some(last) = self.last_displays_check {
            if now.duration_since(last) < Duration::from_secs(3) && !self.displays_cache.is_empty() {
                return self.displays_cache.clone();
            }
        }

        let mut outputs = Vec::new();

        if let Ok(out) = Command::new("kscreen-doctor").arg("-o").output() {
            let text = String::from_utf8_lossy(&out.stdout);
            let mut cur_name = String::new();
            let mut cur_rect = Rect::new(0, 0, 1920, 1080);
            let mut cur_scale = 1.0;
            let mut in_output = false;

            for line in text.lines() {
                let trimmed = line.trim();
                if line.starts_with("Output:") {
                    if in_output && !cur_name.is_empty() {
                        outputs.push(OutputNode::new(cur_name.clone(), cur_rect, cur_scale));
                    }
                    in_output = true;
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        cur_name = parts[2].to_string();
                    } else if parts.len() >= 2 {
                        cur_name = parts[1].to_string();
                    }
                    cur_rect = Rect::new(0, 0, 1920, 1080);
                    cur_scale = 1.0;
                } else if trimmed.starts_with("Geometry:") {
                    // Formato: Geometry: 0,0 1920x1200
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let pos: Vec<&str> = parts[1].split(',').collect();
                        let size: Vec<&str> = parts[2].split('x').collect();
                        let x = pos.first().and_then(|v| v.parse().ok()).unwrap_or(0);
                        let y = pos.get(1).and_then(|v| v.parse().ok()).unwrap_or(0);
                        let w = size.first().and_then(|v| v.parse().ok()).unwrap_or(1920);
                        let h = size.get(1).and_then(|v| v.parse().ok()).unwrap_or(1080);
                        cur_rect = Rect::new(x, y, w, h);
                    }
                } else if trimmed.starts_with("Scale:") {
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if let Some(val) = parts.get(1) {
                        cur_scale = val.parse().unwrap_or(1.0);
                    }
                }
            }
            if in_output && !cur_name.is_empty() {
                outputs.push(OutputNode::new(cur_name, cur_rect, cur_scale));
            }
        }

        if outputs.is_empty() {
            // Fallback estándar
            outputs.push(OutputNode::new(
                "eDP-1".to_string(),
                Rect::new(0, 0, 1920, 1080),
                1.0,
            ));
        }

        self.displays_cache = outputs.clone();
        self.last_displays_check = Some(now);
        outputs
    }
}

/// Representa un escritorio virtual con su nombre visible y su UUID interno de KWin.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DesktopInfo {
    pub id: String,
    pub name: String,
}

impl ServiceManager {
    /// Obtiene la lista de escritorios virtuales configurados en KDE Plasma con sus UUIDs reales (con caché).
    pub fn get_virtual_desktops(&mut self) -> Vec<DesktopInfo> {
        let now = Instant::now();
        if let Some(last) = self.last_desktops_check {
            if now.duration_since(last) < Duration::from_secs(3) && !self.desktops_cache.is_empty() {
                return self.desktops_cache.clone();
            }
        }

        let mut desktops = Vec::new();
        let count = if let Ok(out) = Command::new("kreadconfig6")
            .args(["--file", "kwinrc", "--group", "Desktops", "--key", "Number"])
            .output()
        {
            String::from_utf8_lossy(&out.stdout)
                .trim()
                .parse::<usize>()
                .unwrap_or(4)
        } else {
            4
        };

        for i in 1..=count {
            let id = if let Ok(out) = Command::new("kreadconfig6")
                .args([
                    "--file",
                    "kwinrc",
                    "--group",
                    "Desktops",
                    "--key",
                    &format!("Id_{}", i),
                ])
                .output()
            {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if s.is_empty() {
                    format!("desk_{}", i)
                } else {
                    s
                }
            } else {
                format!("desk_{}", i)
            };

            let name = if let Ok(out) = Command::new("kreadconfig6")
                .args([
                    "--file",
                    "kwinrc",
                    "--group",
                    "Desktops",
                    "--key",
                    &format!("Name_{}", i),
                ])
                .output()
            {
                let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if s.is_empty() {
                    format!("Escritorio {}", i)
                } else {
                    s
                }
            } else {
                format!("Escritorio {}", i)
            };

            desktops.push(DesktopInfo { id, name });
        }

        self.desktops_cache = desktops.clone();
        self.last_desktops_check = Some(now);
        desktops
    }
}
