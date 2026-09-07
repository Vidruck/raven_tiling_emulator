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
pub use service::{KWinBridgeMessage, KWinDbusService};

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::info;
use zbus::{Connection, ConnectionBuilder};

use raven_core::action::RavenAction;
use raven_core::backend::{BackendError, CompositorBackend, CompositorEvent};
use raven_core::geometry::{Rect, WindowNode};

/// Adaptador e intermediario concreto del compositor KWin y KDE Plasma.
pub struct KWinBackend {
    /// Nombre identificador del backend.
    name: &'static str,
    /// Conexión activa al bus D-Bus de sesión de KWin (mantiene vivo el registro D-Bus).
    connection: Option<Arc<Connection>>,
}

impl Default for KWinBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl KWinBackend {
    /// Crea una nueva instancia del backend intermediario de KWin.
    pub fn new() -> Self {
        Self {
            name: "kwin",
            connection: None,
        }
    }

    /// Inicia el servicio D-Bus de KWin (`org.kde.raven.Daemon`) e intermedia los mensajes hacia el canal receptor de Raven.
    ///
    /// Este método desacopla al binario principal de los detalles de sesión de D-Bus y `ConnectionBuilder`.
    pub async fn start_bridge<M>(
        &mut self,
        target_tx: mpsc::Sender<M>,
    ) -> Result<(), BackendError>
    where
        M: From<KWinBridgeMessage> + Send + 'static,
    {
        info!("[KWIN-BACKEND] Levantando servicio D-Bus intermediario para KWin / Plasma...");

        // Capacidad ampliada a 256 para absorber ráfagas extremas de sincronización interactiva
        let (bridge_tx, mut bridge_rx) = mpsc::channel(256);

        // Forwarder intermediario: retransmite los mensajes del bridge al actor principal
        tokio::spawn(async move {
            while let Some(msg) = bridge_rx.recv().await {
                if target_tx.send(M::from(msg)).await.is_err() {
                    break;
                }
            }
        });

        let dbus_service = KWinDbusService { tx: bridge_tx };

        let conn = ConnectionBuilder::session()
            .map_err(|e| BackendError::ConnectionFailed(e.to_string()))?
            .name("org.kde.raven.Daemon")
            .map_err(|e| BackendError::ConnectionFailed(e.to_string()))?
            .serve_at("/Events", dbus_service)
            .map_err(|e| BackendError::ConnectionFailed(e.to_string()))?
            .build()
            .await
            .map_err(|e| BackendError::ConnectionFailed(e.to_string()))?;

        self.connection = Some(Arc::new(conn));
        info!("[KWIN-BACKEND] Servicio org.kde.raven.Daemon registrado exitosamente como intermediario.");

        Ok(())
    }

    /// Retorna una referencia a la conexión D-Bus activa, si el bridge ha sido iniciado.
    pub fn connection(&self) -> Option<Arc<Connection>> {
        self.connection.clone()
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
        info!("[KWIN-BACKEND] Escucha de eventos de compositor KWin iniciada.");
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
