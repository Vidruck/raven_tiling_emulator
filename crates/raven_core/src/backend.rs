//! # Abstracción de Puertos y Eventos de Compositor (`backend`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 4.0  
//! **Licencia:** GPL-3.0  
//!
//! Este módulo define los contratos universales (`traits`) y tipos de datos
//! agnósticos requeridos para que cualquier compositor o sistema de ventanas
//! (KWin, wlroots, etc.) interactúe con el motor de Raven Tiling.

use std::collections::HashMap;
use std::fmt;
use serde::{Deserialize, Serialize};
use crate::action::RavenAction;
use crate::geometry::{Rect, WindowNode};

/// Errores estandarizados producidos por los adaptadores de compositor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BackendError {
    /// Error al establecer la conexión con el bus o socket del compositor.
    ConnectionFailed(String),
    /// Error al enviar o serializar una acción hacia el compositor.
    ActionDispatchFailed(String),
    /// Error en la consulta de introspección o lectura de estado inicial.
    QueryFailed(String),
    /// Compositor no compatible o servicio no disponible en el entorno actual.
    NotSupported(String),
    /// Error genérico de I/O o comunicación asíncrona.
    Other(String),
}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendError::ConnectionFailed(msg) => write!(f, "Fallo de conexión al compositor: {}", msg),
            BackendError::ActionDispatchFailed(msg) => write!(f, "Fallo al despachar acción al compositor: {}", msg),
            BackendError::QueryFailed(msg) => write!(f, "Fallo en consulta al compositor: {}", msg),
            BackendError::NotSupported(msg) => write!(f, "Compositor no soportado: {}", msg),
            BackendError::Other(msg) => write!(f, "Error en backend de compositor: {}", msg),
        }
    }
}

impl std::error::Error for BackendError {}

/// Eventos normalizados de ventanas emitidos por cualquier compositor hacia el motor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompositorEvent {
    /// Se ha descubierto o abierto una nueva ventana en el sistema.
    WindowDiscovered(WindowNode),
    /// Se ha cerrado o destruido una ventana.
    WindowClosed(String),
    /// Cambio atómico en las propiedades o geometría de una ventana existente.
    WindowStateChanged {
        window_id: String,
        is_minimized: Option<bool>,
        is_fullscreen: Option<bool>,
        is_floating: Option<bool>,
        geometry: Option<Rect>,
    },
    /// Foco de entrada cambiado hacia una ventana específica.
    WindowFocused(Option<String>),
    /// Cambio o reorganización en la topología de monitores o escritorios.
    TopologyChanged(crate::geometry::Topology),
}

/// Contrato universal de abstracción para backends de compositores de ventanas.
///
/// Cualquier integración (KWin Wayland, wlroots, Hyprland IPC, etc.) debe implementar
/// este rasgo para alimentar el bucle reactivo de `RavenControllerActor`.
#[async_trait::async_trait]
pub trait CompositorBackend: Send + Sync {
    /// Nombre identificador del backend (ej: `"kwin"`, `"wayland-wlr"`, `"hyprland"`).
    fn name(&self) -> &'static str;

    /// Inicia la escucha asíncrona de eventos del compositor y los transmite por el canal.
    async fn start_listener(
        &self,
        event_tx: tokio::sync::mpsc::Sender<CompositorEvent>,
    ) -> Result<(), BackendError>;

    /// Aplica físicamente una secuencia de acciones de reacomodo o foco sobre el compositor.
    async fn apply_actions(
        &self,
        actions: Vec<RavenAction>,
    ) -> Result<(), BackendError>;

    /// Consulta inicial para descubrir las pantallas y ventanas existentes al arrancar el motor.
    async fn query_initial_state(
        &self,
    ) -> Result<(HashMap<String, Rect>, Vec<WindowNode>), BackendError>;
}
