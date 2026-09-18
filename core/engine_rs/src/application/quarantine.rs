//! # Administrador de Cuarentena y Estabilización Wayland (`quarantine`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Gestiona los temporizadores de estabilización asíncrona para ventanas de clientes
//! Wayland (CSD, navegadores Gecko/Chromium, aplicaciones Electron, Steam, Java/JetBrains).
//! Evita ráfagas de cálculo prematuro de dimensiones y parpadeo geométrico.

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
    /// Ventana de arranque estándar o ligera (80 ms).
    Standard,
    /// Navegador web con CSD (Firefox, Zen, Chrome, Brave) (140 ms).
    Browser,
    /// Aplicación pesada (Electron, Discord, Slack, JetBrains, Steam) (220 ms).
    Heavy,
}

impl QuarantinePolicy {
    /// Determina la política de cuarentena según la clase de la ventana.
    pub fn from_class(class_name: &str) -> Self {
        let cls = class_name.to_lowercase();
        if cls.is_empty() {
            return Self::Standard;
        }

        // Navegadores web conocidos con negociación compleja de superficies Wayland
        if cls.contains("firefox")
            || cls.contains("zen")
            || cls.contains("chrome")
            || cls.contains("chromium")
            || cls.contains("brave")
            || cls.contains("floorp")
            || cls.contains("librewolf")
            || cls.contains("vivaldi")
            || cls.contains("opera")
            || cls.contains("edge")
        {
            return Self::Browser;
        }

        // Aplicaciones complejas / Electron / JVM / Juegos
        if cls.contains("electron")
            || cls.contains("code")
            || cls.contains("vscodium")
            || cls.contains("cursor")
            || cls.contains("discord")
            || cls.contains("slack")
            || cls.contains("steam")
            || cls.contains("spotify")
            || cls.contains("obsidian")
            || cls.contains("thunderbird")
            || cls.contains("postman")
            || cls.contains("idea")
            || cls.contains("clion")
            || cls.contains("pycharm")
            || cls.contains("java")
        {
            return Self::Heavy;
        }

        Self::Standard
    }

    /// Retorna la duración en milisegundos para esta política.
    pub fn duration(&self) -> Duration {
        match self {
            QuarantinePolicy::Standard => Duration::from_millis(80),
            QuarantinePolicy::Browser => Duration::from_millis(140),
            QuarantinePolicy::Heavy => Duration::from_millis(220),
        }
    }
}

/// Gestor concurrente de cuarentena que despacha eventos de liberación al expirar el tiempo.
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
    pub async fn schedule_release(&self, window_id: String, class_name: &str) {
        let mut guard = self.scheduled.lock().await;
        if guard.contains(&window_id) {
            return;
        }
        guard.insert(window_id.clone());
        drop(guard);

        let policy = QuarantinePolicy::from_class(class_name);
        let duration = policy.duration();
        let target_id = window_id.clone();
        let tx = self.tx.clone();
        let scheduled_ref = self.scheduled.clone();

        tokio::spawn(async move {
            tokio::time::sleep(duration).await;
            
            // Remover del registro
            {
                let mut guard = scheduled_ref.lock().await;
                guard.remove(&target_id);
            }

            info!(
                "[QUARANTINE-MGR] Tiempo cumplido ({:?}) para ventana '{}' ({:?})",
                duration, target_id, policy
            );

            let _ = tx
                .send(RavenMessage::Compositor(
                    CompositorEvent::ReleaseQuarantine(target_id),
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
