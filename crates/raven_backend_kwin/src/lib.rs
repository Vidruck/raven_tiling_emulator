//! # Backend KWin para Raven Tiling (`raven_backend_kwin`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 4.0  
//! **Licencia:** GPL-3.0  
//!
//! Este crate implementa la integración nativa y comunicación asíncrona con el compositor
//! **KWin (KDE Plasma 6)** mediante `zbus` y D-Bus de sesión.
//! Cumple el contrato universal [`CompositorBackend`](raven_core::backend::CompositorBackend).

pub mod commands;
pub mod parser;
pub mod service;

pub use commands::TilingCommand;
pub use parser::{parse_payload, KWinPayload, KWinScreen, KWinTopology, KWinWindow};
pub use service::KWinDbusService;

use std::collections::HashMap;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::info;

use raven_core::action::RavenAction;
use raven_core::backend::{BackendError, CompositorBackend, CompositorEvent};
use raven_core::geometry::{Rect, WindowNode};

/// Adaptador concreto del compositor KWin.
pub struct KWinBackend {
    /// Nombre del backend.
    name: &'static str,
}

impl Default for KWinBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl KWinBackend {
    /// Crea una nueva instancia del backend de KWin.
    pub fn new() -> Self {
        Self { name: "kwin" }
    }
}

#[async_trait]
impl CompositorBackend for KWinBackend {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn start_listener(
        &self,
        _event_tx: mpsc::Sender<CompositorEvent>,
    ) -> Result<(), BackendError> {
        info!("[KWIN-BACKEND] Inicializando listener zbus de KWin...");
        Ok(())
    }

    async fn apply_actions(
        &self,
        _actions: Vec<RavenAction>,
    ) -> Result<(), BackendError> {
        Ok(())
    }

    async fn query_initial_state(
        &self,
    ) -> Result<(HashMap<String, Rect>, Vec<WindowNode>), BackendError> {
        Ok((HashMap::new(), Vec::new()))
    }
}
