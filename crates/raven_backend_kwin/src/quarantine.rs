//! # Administrador de Cuarentena y Filtrado KWin (`quarantine`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 4.1  
//! **Licencia:** GPL-3.0  
//!
//! Implementa la lógica de filtrado de ventanas gestionables/flotantes,
//! detección de arranque en frío (*Cold Start*) vs. caliente (*Warm Start*),
//! mitigación de ráfagas (*flood*) y temporizadores de estabilización en Rust.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tracing::info;

use raven_core::backend::CompositorEvent;
use raven_core::config::WindowRule;
use crate::parser::KWinWindow;

/// Categoría de estabilización de la aplicación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuarantineCategory {
    /// Ventanas estándar o ligeras (GTK/Qt simples, terminales).
    Standard,
    /// Navegadores web con CSD (Firefox, Zen, Chrome, Brave, etc.).
    Browser,
    /// Aplicaciones pesadas con ciclo de vida asíncrono (Electron, Discord, Slack, JetBrains, Steam).
    Heavy,
}

impl QuarantineCategory {
    /// Determina la categoría de una ventana a partir de su ejecutable y clase WM.
    pub fn from_window(resource_class: &str, resource_name: &str) -> Self {
        let cls = resource_class.to_lowercase();
        let name = resource_name.to_lowercase();

        let browser_executables = [
            "zen", "zen-browser", "firefox", "firefox-esr", "navigator",
            "librewolf", "floorp", "waterfox", "icecat", "chrome",
            "google-chrome", "chromium", "chromium-browser", "brave-browser",
            "brave", "vivaldi", "vivaldi-stable", "opera", "msedge",
            "microsoft-edge", "epiphany", "falkon", "midori", "qutebrowser", "min",
        ];

        if browser_executables.iter().any(|&exe| {
            name == exe || name.starts_with(exe) || cls == exe || cls.starts_with(exe)
        }) || cls.contains("browser") {
            return Self::Browser;
        }

        let heavy_executables = [
            "code", "code-oss", "vscodium", "cursor", "discord",
            "discordcanary", "slack", "steam", "spotify", "obsidian",
            "thunderbird", "postman", "idea", "idea64", "clion",
            "clion64", "pycharm", "pycharm64", "datagrip", "goland",
            "rider", "webstorm", "java", "teams", "signal",
            "telegram-desktop", "notion-app", "figma-linux", "gimp",
            "inkscape", "blender", "krita",
        ];

        if heavy_executables.iter().any(|&exe| {
            name == exe || name.starts_with(exe) || cls == exe || cls.starts_with(exe)
        }) || cls.contains("electron")
            || name.contains("electron")
            || cls.contains("java")
            || name.contains("java")
        {
            return Self::Heavy;
        }

        Self::Standard
    }
}

/// Estado del arranque para seleccionar temporizadores de estabilización.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupMode {
    /// Primer arranque de la aplicación/clase en la sesión.
    Cold,
    /// Instancia secundaria o ventana subsecuente de una clase ya inicializada.
    Warm,
}

/// Gestor nativo de cuarentena, filtrado y auditoría para KWin.
#[derive(Clone)]
pub struct KWinQuarantineManager {
    seen_classes: Arc<Mutex<HashSet<String>>>,
    active_quarantines: Arc<Mutex<HashSet<String>>>,
    last_geometry_seen: Arc<Mutex<std::collections::HashMap<String, (i32, i32, i32, i32)>>>,
}

impl Default for KWinQuarantineManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KWinQuarantineManager {
    /// Crea una nueva instancia de `KWinQuarantineManager`.
    pub fn new() -> Self {
        Self {
            seen_classes: Arc::new(Mutex::new(HashSet::new())),
            active_quarantines: Arc::new(Mutex::new(HashSet::new())),
            last_geometry_seen: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Evalúa si la apertura de esta ventana representa un arranque en frío (*Cold Start*) o en caliente (*Warm Start*).
    pub async fn evaluate_startup_mode(&self, resource_class: &str, resource_name: &str) -> StartupMode {
        let key = if !resource_name.is_empty() {
            resource_name.to_lowercase()
        } else {
            resource_class.to_lowercase()
        };

        if key.is_empty() {
            return StartupMode::Cold;
        }

        let mut seen = self.seen_classes.lock().await;
        if seen.contains(&key) {
            StartupMode::Warm
        } else {
            seen.insert(key);
            StartupMode::Cold
        }
    }

    /// Retorna el tiempo de temporizador calibrado para estabilización.
    pub fn get_stabilization_delay(&self, mode: StartupMode, category: QuarantineCategory) -> Duration {
        match (mode, category) {
            // Cold Start: tiempo suficiente pero ágil para configurar Wayland / CSD
            (StartupMode::Cold, QuarantineCategory::Standard) => Duration::from_millis(60),
            (StartupMode::Cold, QuarantineCategory::Browser) => Duration::from_millis(75),
            (StartupMode::Cold, QuarantineCategory::Heavy) => Duration::from_millis(85),

            // Warm Start: ultra rápido porque los recursos de renderizado ya están vivos
            (StartupMode::Warm, QuarantineCategory::Standard) => Duration::from_millis(20),
            (StartupMode::Warm, QuarantineCategory::Browser) => Duration::from_millis(25),
            (StartupMode::Warm, QuarantineCategory::Heavy) => Duration::from_millis(30),
        }
    }

    /// Retorna el retraso calibrado para la verificación de rectificación post-layout.
    pub fn get_rectification_delay(&self) -> Duration {
        Duration::from_millis(45)
    }

    /// Agenda la estabilización de una ventana y emite `ReleaseQuarantine` y `RectifyWindow` al expirar.
    pub async fn schedule_quarantine(
        &self,
        window_id: String,
        resource_class: String,
        resource_name: String,
        event_tx: mpsc::Sender<CompositorEvent>,
    ) {
        let mut active = self.active_quarantines.lock().await;
        if active.contains(&window_id) {
            return;
        }
        active.insert(window_id.clone());
        drop(active);

        let startup_mode = self.evaluate_startup_mode(&resource_class, &resource_name).await;
        let category = QuarantineCategory::from_window(&resource_class, &resource_name);
        let stab_delay = self.get_stabilization_delay(startup_mode, category);
        let rect_delay = self.get_rectification_delay();

        let target_id = window_id.clone();
        let active_ref = self.active_quarantines.clone();

        tokio::spawn(async move {
            tokio::time::sleep(stab_delay).await;

            info!(
                "[KWIN-QUARANTINE] Estabilización ({:?}, {:?}, {:?}) finalizada para '{}'. Despachando ReleaseQuarantine.",
                startup_mode, category, stab_delay, target_id
            );

            // Fase 1: Liberar cuarentena y permitir layout
            let _ = event_tx.send(CompositorEvent::ReleaseQuarantine(target_id.clone())).await;

            // Fase 2: Rectificación rápida
            tokio::time::sleep(rect_delay).await;

            {
                let mut active = active_ref.lock().await;
                active.remove(&target_id);
            }

            info!(
                "[KWIN-QUARANTINE] Auditando rectificación ({:?}) para '{}'.",
                rect_delay, target_id
            );
            let _ = event_tx.send(CompositorEvent::RectifyWindow(target_id)).await;
        });
    }

    /// Filtra eventos de ruido redundantes si la geometría no ha cambiado significativamente.
    pub async fn filter_flood(&self, window_id: &str, x: i32, y: i32, w: i32, h: i32) -> bool {
        let mut geom_map = self.last_geometry_seen.lock().await;
        if let Some(&(lx, ly, lw, lh)) = geom_map.get(window_id) {
            if lx == x && ly == y && lw == w && lh == h {
                return true; // Es flood idéntico, suprimir
            }
        }
        geom_map.insert(window_id.to_string(), (x, y, w, h));
        false
    }

    /// Evalúa si una ventana recibida en crudo desde KWin es gestionable en mosaico.
    pub fn is_manageable(&self, win: &KWinWindow) -> bool {
        if win.w <= 0 || win.h <= 0 {
            return false;
        }

        let cls_lower = win.cls.to_lowercase();
        if cls_lower.contains("spectacle") && win.fs {
            return false;
        }

        // Ignorar ventanas flotantes conocidas o utilidades fijas
        if cls_lower.contains("kcolorchooser")
            || cls_lower.contains("colorpicker")
            || cls_lower.contains("klipper")
            || cls_lower.contains("polkit")
            || cls_lower.contains("pinentry")
            || cls_lower.contains("kdialog")
            || cls_lower.contains("raven_gui")
            || cls_lower.contains("raven-gui")
        {
            return false;
        }

        true
    }

    /// Evalúa si una ventana debe ser tratada como flotante libre.
    pub fn is_floating(&self, win: &KWinWindow, rules: &[WindowRule]) -> bool {
        if win.f {
            return true;
        }

        let cls_lower = win.cls.to_lowercase();
        let cap_lower = win.cap.to_lowercase();

        // 1. Reglas de usuario
        for rule in rules {
            if !rule.class.is_empty() && cls_lower.contains(&rule.class.to_lowercase()) {
                if rule.action == "float" || rule.pip {
                    return true;
                }
            }
        }

        // 2. Detección PiP por título
        if cap_lower.contains("picture-in-picture")
            || cap_lower.contains("picture in picture")
            || cap_lower.contains("pantalla en pantalla")
            || cap_lower.contains("imagen en imagen")
            || cap_lower == "pip"
        {
            return true;
        }

        // 3. Ventana de tamaño fijo rígido (min == max) o micro widgets
        if win.min_w > 0 && win.min_h > 0 && win.w < 380 && win.h < 320 {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cold_vs_warm_startup() {
        let mgr = KWinQuarantineManager::new();

        // Primera ventana de Zen Browser -> Cold Start
        let mode1 = mgr.evaluate_startup_mode("zen-browser", "zen").await;
        assert_eq!(mode1, StartupMode::Cold);

        // Segunda ventana de Zen Browser -> Warm Start
        let mode2 = mgr.evaluate_startup_mode("zen-browser", "zen").await;
        assert_eq!(mode2, StartupMode::Warm);

        // Ventana de terminal -> Cold Start
        let mode3 = mgr.evaluate_startup_mode("kitty", "kitty").await;
        assert_eq!(mode3, StartupMode::Cold);
    }

    #[test]
    fn test_quarantine_categories_and_delays() {
        let mgr = KWinQuarantineManager::new();

        let cat_browser = QuarantineCategory::from_window("zen-browser", "zen");
        assert_eq!(cat_browser, QuarantineCategory::Browser);

        let cat_heavy = QuarantineCategory::from_window("code", "code");
        assert_eq!(cat_heavy, QuarantineCategory::Heavy);

        let cat_std = QuarantineCategory::from_window("kitty", "kitty");
        assert_eq!(cat_std, QuarantineCategory::Standard);

        // Validar temporizadores reducidos
        assert_eq!(mgr.get_stabilization_delay(StartupMode::Cold, QuarantineCategory::Browser), Duration::from_millis(75));
        assert_eq!(mgr.get_stabilization_delay(StartupMode::Warm, QuarantineCategory::Browser), Duration::from_millis(25));
        assert_eq!(mgr.get_stabilization_delay(StartupMode::Warm, QuarantineCategory::Standard), Duration::from_millis(20));
    }

    #[tokio::test]
    async fn test_filter_flood_identical_geometry() {
        let mgr = KWinQuarantineManager::new();
        let wid = "0x12345";

        assert!(!mgr.filter_flood(wid, 100, 100, 800, 600).await);
        // Misma geometría inmediata -> detectado como flood
        assert!(mgr.filter_flood(wid, 100, 100, 800, 600).await);
        // Nueva geometría -> no es flood
        assert!(!mgr.filter_flood(wid, 120, 100, 800, 600).await);
    }
}
