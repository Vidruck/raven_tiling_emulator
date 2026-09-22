//! # Administrador de Cuarentena y Estabilización Wayland (`quarantine`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Gestiona los temporizadores de estabilización asíncrona para ventanas de clientes
//! Wayland (CSD, navegadores Gecko/Chromium, aplicaciones Electron, Steam, Java/JetBrains).
//! Evita ráfagas de cálculo prematuro de dimensiones y parpadeo geométrico.
//!
//! ## Modelo Bifásico de Rectificación
//!
//! Implementa un modelo bifásico para garantizar que las ventanas asuman las medidas calculadas:
//!
//! **Fase 1 – Liberación** (`ReleaseQuarantine`): expira el periodo de cuarentena, se libera
//! la bandera del compositor y se recalcula el layout enviando las órdenes de posicionamiento.
//!
//! **Fase 2 – Rectificación** (`RectifyWindow`): tras un intervalo adicional, verifica que la
//! ventana asumió las medidas comandadas. Si la geometría física no coincide con el objetivo
//! calculado, Rust re-emite las órdenes de posicionamiento y solicita retroalimentación.
//! Esto resuelve el problema de navegadores y aplicaciones CSD que sobreescriben su geometría
//! con valores de sesión anterior tras el arranque.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::info;

use crate::application::actor::RavenMessage;
use raven_core::backend::CompositorEvent;

/// Categoría y duración de estabilización para ventanas en cuarentena.
#[derive(Debug, Clone, Copy)]
pub enum QuarantinePolicy {
    /// Ventana de arranque estándar o ligera.
    Standard,
    /// Navegador web con CSD (Firefox, Zen, Chrome, Brave…).
    Browser,
    /// Aplicación pesada (Electron, Discord, Slack, JetBrains, Steam…).
    Heavy,
}

impl QuarantinePolicy {
    /// Determina la política de cuarentena a partir de la clase WM y el nombre del ejecutable.
    ///
    /// Usa **dos vectores de detección** complementarios para mayor robustez:
    ///
    /// 1. **`resource_name`** (primera parte de WM_CLASS, nombre del ejecutable) — comparación
    ///    exacta o por prefijo. Es el identificador más estable porque viene del binario real del
    ///    proceso y no cambia con temas, locales ni renombrados de aplicación.
    ///    Ejemplo: `zen`, `firefox`, `chrome`, `discord`.
    ///
    /// 2. **`resource_class`** (segunda parte de WM_CLASS, clase de la app) — búsqueda por
    ///    fragmento como fallback. Cubre apps menos conocidas o con nombres variantes.
    ///    Ejemplo: `zen-browser`, `Chromium`, `com.electron.app`.
    ///
    /// # Parámetros
    /// * `resource_class` – Clase WM del proceso (ej. `"zen-browser"`, `"firefox"`).
    /// * `resource_name`  – Nombre del ejecutable (ej. `"zen"`, `"Navigator"`, `"chrome"`).
    pub fn from_class(resource_class: &str, resource_name: &str) -> Self {
        let cls  = resource_class.to_lowercase();
        let name = resource_name.to_lowercase();

        // --- Navegadores por nombre de ejecutable exacto (WM_CLASS instancia) ---
        // Zen Browser ejecutable: "zen" (resourceName) / "zen-browser" (resourceClass)
        // Firefox/Librewolf:      "navigator" (resourceName en Wayland) / "firefox" (resourceClass)
        // Chrome/Chromium:        "chrome" / "chromium" / "google-chrome"
        // Brave:                  "brave-browser" / "brave"
        // Edge:                   "msedge"
        let browser_executables: &[&str] = &[
            "zen",
            "zen-browser",
            "firefox",
            "firefox-esr",
            "navigator",      // Firefox en modo Wayland usa "Navigator" como resourceName
            "librewolf",
            "floorp",
            "waterfox",
            "icecat",
            "chrome",
            "google-chrome",
            "chromium",
            "chromium-browser",
            "brave-browser",
            "brave",
            "vivaldi",
            "vivaldi-stable",
            "opera",
            "msedge",
            "microsoft-edge",
            "epiphany",
            "falkon",
            "midori",
            "qutebrowser",
            "min",
        ];

        // Comparación exacta primero (más preciso), luego prefijo (ej "zen" en "zen-snapshot")
        let is_browser = browser_executables.iter().any(|&exe| {
            name == exe || name.starts_with(exe) || cls == exe || cls.starts_with(exe)
        }) || cls.contains("browser");

        if is_browser {
            return Self::Browser;
        }

        // --- Apps pesadas por nombre de ejecutable exacto ---
        let heavy_executables: &[&str] = &[
            "code",
            "code-oss",
            "vscodium",
            "cursor",
            "discord",
            "discordcanary",
            "slack",
            "steam",
            "spotify",
            "obsidian",
            "thunderbird",
            "postman",
            "idea",
            "idea64",
            "clion",
            "clion64",
            "pycharm",
            "pycharm64",
            "datagrip",
            "goland",
            "rider",
            "webstorm",
            "java",
            "teams",
            "signal",
            "telegram-desktop",
            "notion-app",
            "figma-linux",
            "gimp",
            "inkscape",
            "blender",
            "krita",
        ];

        let is_heavy = heavy_executables.iter().any(|&exe| {
            name == exe || name.starts_with(exe) || cls == exe || cls.starts_with(exe)
        }) || cls.contains("electron")
            || name.contains("electron")
            || cls.contains("java")
            || name.contains("java");

        if is_heavy {
            return Self::Heavy;
        }

        Self::Standard
    }

    /// Retorna la duración de cuarentena (Fase 1) según la política.
    ///
    /// Tiempos calibrados para ser lo más cortos posibles manteniendo estabilidad:
    /// - **Standard**: 40ms — apps GTK/Qt simples, tiempo mínimo para el primer frame.
    /// - **Browser**: 55ms — navegadores necesitan tiempo para negociación de superficie Wayland.
    /// - **Heavy**: 75ms — apps Electron/JVM con inicialización costosa.
    pub fn duration(&self) -> Duration {
        match self {
            QuarantinePolicy::Standard => Duration::from_millis(90),
            QuarantinePolicy::Browser  => Duration::from_millis(120),
            QuarantinePolicy::Heavy    => Duration::from_millis(150),
        }
    }

    /// Retorna el retraso adicional para la verificación de rectificación post-cuarentena (Fase 2).
    ///
    /// Este intervalo ocurre *después* de que el timer de cuarentena expira, para dar tiempo
    /// al compositor y a la app de procesar la orden de posicionamiento antes de verificar.
    /// - **Standard**: 45ms — suficiente para un round-trip DBus.
    /// - **Browser**: 90ms — los navegadores restauran sesión y sobreescriben geometría con retardo.
    /// - **Heavy**: 120ms — apps pesadas pueden tardar más en procesar el resize Wayland.
    pub fn rectification_delay(&self) -> Duration {
        match self {
            QuarantinePolicy::Standard => Duration::from_millis(60),
            QuarantinePolicy::Browser  => Duration::from_millis(60),
            QuarantinePolicy::Heavy    => Duration::from_millis(60),
        }
    }
}

/// Gestor concurrente de cuarentena que despacha eventos de liberación y rectificación.
///
/// Implementa el **Modelo Bifásico de Rectificación** de Raven:
/// 1. **Fase 1 – Liberación** (`ReleaseQuarantine`): expira el periodo de cuarentena.
/// 2. **Fase 2 – Rectificación** (`RectifyWindow`): verifica y corrige la geometría si fue ignorada.
#[derive(Clone)]
pub struct QuarantineManager {
    scheduled: Arc<tokio::sync::Mutex<HashSet<String>>>,
    tx: mpsc::Sender<RavenMessage>,
}

impl QuarantineManager {
    /// Crea una nueva instancia de `QuarantineManager`.
    pub fn new(tx: mpsc::Sender<RavenMessage>) -> Self {
        Self {
            scheduled: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
            tx,
        }
    }

    /// Agenda la liberación diferida de cuarentena para una ventana si no ha sido agendada previamente.
    ///
    /// Al expirar el timer de cuarentena, despacha `CompositorEvent::ReleaseQuarantine`.
    /// El actor que recibe ese evento debe agendar la fase de rectificación con
    /// `schedule_rectification` para implementar el modelo bifásico completo.
    ///
    /// # Parámetros
    /// * `window_id`      – Identificador de la ventana.
    /// * `resource_class` – Segunda parte de WM_CLASS (ej. `"zen-browser"`).
    /// * `resource_name`  – Primera parte de WM_CLASS / ejecutable (ej. `"zen"`, `"Navigator"`).
    pub async fn schedule_release(&self, window_id: String, resource_class: &str, resource_name: &str) {
        let mut guard = self.scheduled.lock().await;
        if guard.contains(&window_id) {
            return;
        }
        guard.insert(window_id.clone());
        drop(guard);

        let policy = QuarantinePolicy::from_class(resource_class, resource_name);
        let duration = policy.duration();
        let target_id = window_id.clone();
        let tx = self.tx.clone();
        let scheduled_ref = self.scheduled.clone();

        tokio::spawn(async move {
            tokio::time::sleep(duration).await;

            // Remover del registro de cuarentena activa
            {
                let mut guard = scheduled_ref.lock().await;
                guard.remove(&target_id);
            }

            info!(
                "[QUARANTINE-MGR] Tiempo cumplido ({:?}) para ventana '{}' ({:?}). Liberando cuarentena (Fase 1).",
                duration, target_id, policy
            );

            let _ = tx
                .send(RavenMessage::Compositor(
                    CompositorEvent::ReleaseQuarantine(target_id),
                ))
                .await;
        });
    }

    /// Agenda la verificación de rectificación post-cuarentena para una ventana (Fase 2).
    ///
    /// Debe llamarse **después** de que `ReleaseQuarantine` fue procesado por el actor y las
    /// órdenes de posicionamiento iniciales fueron enviadas al compositor.
    ///
    /// # Parámetros
    /// * `window_id`      – Identificador de la ventana a verificar.
    /// * `resource_class` – Segunda parte de WM_CLASS (para determinar la política de demora).
    /// * `resource_name`  – Nombre del ejecutable (primera parte WM_CLASS).
    pub async fn schedule_rectification(&self, window_id: String, resource_class: &str, resource_name: &str) {
        let policy = QuarantinePolicy::from_class(resource_class, resource_name);
        let delay = policy.rectification_delay();
        let target_id = window_id.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            tokio::time::sleep(delay).await;

            info!(
                "[RECTIF-MGR] Verificación rectificación (Fase 2, {:?}, {:?}) para ventana '{}'",
                delay, policy, target_id
            );

            let _ = tx
                .send(RavenMessage::Compositor(
                    CompositorEvent::RectifyWindow(target_id),
                ))
                .await;
        });
    }

    /// Limpia el registro de una ventana si se destruyó antes de expirar el timer.
    pub async fn cancel(&self, window_id: &str) {
        let mut guard = self.scheduled.lock().await;
        guard.remove(window_id);
    }
}
