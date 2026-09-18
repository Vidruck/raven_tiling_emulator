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
    /// Navegador web con CSD (Firefox, Zen, Chrome, Brave).
    Browser,
    /// Aplicación pesada (Electron, Discord, Slack, JetBrains, Steam).
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

    /// Retorna la duración de cuarentena (Fase 1) según la política.
    pub fn duration(&self) -> Duration {
        match self {
            QuarantinePolicy::Standard => Duration::from_millis(60),
            QuarantinePolicy::Browser => Duration::from_millis(90),
            QuarantinePolicy::Heavy => Duration::from_millis(120),
        }
    }

    /// Retorna el retraso adicional para la verificación de rectificación post-cuarentena (Fase 2).
    ///
    /// Este intervalo ocurre *después* de que el timer de cuarentena expira, para dar tiempo
    /// al compositor (KWin/Wayland) y a la aplicación de procesar la orden de posicionamiento.
    /// Si la ventana ignoró o sobreescribió la geometría calculada, Rust re-enviará el comando.
    pub fn rectification_delay(&self) -> Duration {
        match self {
            QuarantinePolicy::Standard => Duration::from_millis(80),
            QuarantinePolicy::Browser => Duration::from_millis(150),
            QuarantinePolicy::Heavy => Duration::from_millis(200),
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
    /// Espera el `rectification_delay` de la política y luego despacha
    /// `CompositorEvent::RectifyWindow` para que el actor verifique si la entidad asumió
    /// las medidas calculadas por Rust. Si la geometría física no coincide con el objetivo,
    /// el actor re-envía `MoveWindow` forzosamente, resolviendo el problema de navegadores
    /// y apps CSD que sobreescriben su posición con valores de sesión anterior.
    ///
    /// # Parámetros
    /// * `window_id`  – Identificador de la ventana a verificar.
    /// * `class_name` – Clase WM de la ventana (para determinar la política de demora).
    pub async fn schedule_rectification(&self, window_id: String, class_name: &str) {
        let policy = QuarantinePolicy::from_class(class_name);
        let delay = policy.rectification_delay();
        let target_id = window_id.clone();
        let tx = self.tx.clone();

        tokio::spawn(async move {
            tokio::time::sleep(delay).await;

            info!(
                "[RECTIF-MGR] Disparando verificación de rectificación (Fase 2, {:?}) para ventana '{}'",
                delay, target_id
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
