//! # Backend Wayland Nativo para Raven Tiling (`raven_backend_wayland`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)
//! **Versión:** 4.0.0
//! **Licencia:** GPL-3.0
//!
//! Este crate implementa la conexión directa al socket Wayland del compositor
//! (`WAYLAND_DISPLAY`) mediante `wayland-client` y protocolos estándar como
//! `wl_output` y `zwlr_foreign_toplevel_management_v1`.
//!
//! Cumple el contrato universal [`CompositorBackend`](raven_core::backend::CompositorBackend).

pub mod state;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use wayland_client::Connection;

use raven_core::action::RavenAction;
use raven_core::backend::{BackendError, CompositorBackend, CompositorEvent};
use raven_core::geometry::{Rect, WindowNode};

use crate::state::WaylandState;

/// Adaptador de backend para compositores Wayland nativos.
#[derive(Clone)]
pub struct WaylandBackend {
    name: &'static str,
    state: Arc<Mutex<WaylandState>>,
}

impl Default for WaylandBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WaylandBackend {
    /// Crea una nueva instancia de `WaylandBackend`.
    pub fn new() -> Self {
        Self {
            name: "wayland",
            state: Arc::new(Mutex::new(WaylandState::new(None))),
        }
    }
}

#[async_trait]
impl CompositorBackend for WaylandBackend {
    fn name(&self) -> &'static str {
        self.name
    }

    /// Inicia el bucle de eventos asíncrono escuchando el socket Wayland.
    async fn start_listener(
        &self,
        event_tx: mpsc::Sender<CompositorEvent>,
    ) -> Result<(), BackendError> {
        info!("[WAYLAND-BACKEND] Conectando al compositor Wayland nativo...");

        let conn = Connection::connect_to_env()
            .map_err(|e| BackendError::ConnectionFailed(format!("Error conectando a Wayland socket: {}", e)))?;

        let (state_tx, mut state_rx) = mpsc::channel(128);

        // Forwarder task de eventos normalizados hacia el engine
        tokio::spawn(async move {
            while let Some(ev) = state_rx.recv().await {
                if event_tx.send(ev).await.is_err() {
                    break;
                }
            }
        });

        let state_clone = self.state.clone();

        // Lanzar hilo dedicado para la cola de eventos síncrona de Wayland
        std::thread::Builder::new()
            .name("raven-wayland-event-loop".to_string())
            .spawn(move || {
                let mut event_queue = conn.new_event_queue();
                let qh = event_queue.handle();

                let display = conn.display();
                let _registry = display.get_registry(&qh, ());

                let mut local_state = WaylandState::new(Some(state_tx));

                // Realizar roundtrip inicial para descubrir outputs y toplevel managers
                if let Err(e) = event_queue.roundtrip(&mut local_state) {
                    error!("[WAYLAND-BACKEND] Fallo en roundtrip inicial: {}", e);
                    return;
                }

                info!("[WAYLAND-BACKEND] Roundtrip inicial exitoso. Iniciando despacho continuo...");

                loop {
                    if let Err(e) = event_queue.blocking_dispatch(&mut local_state) {
                        warn!("[WAYLAND-BACKEND] Conexión Wayland cerrada o error en dispatch: {}", e);
                        break;
                    }

                    // Sincronizar estado compartido para query_initial_state
                    if let Ok(mut lock) = state_clone.lock() {
                        lock.outputs = local_state.outputs.clone();
                    }
                }
            })
            .map_err(|e| BackendError::Other(format!("Fallo iniciando hilo Wayland: {}", e)))?;

        Ok(())
    }

    /// Aplica acciones hacia el compositor (foco, activación, etc.).
    async fn apply_actions(
        &self,
        actions: Vec<RavenAction>,
    ) -> Result<(), BackendError> {
        for action in actions {
            match action {
                RavenAction::FocusWindow { window_id } => {
                    info!("[WAYLAND-BACKEND] Solicitud de foco para ventana: {}", window_id);
                    // Los toplevel handles pueden invocar handle.set_activated()
                }
                RavenAction::MinimizeWindow { window_id } => {
                    info!("[WAYLAND-BACKEND] Solicitud de minimizar ventana: {}", window_id);
                }
                RavenAction::UnminimizeWindow { window_id } => {
                    info!("[WAYLAND-BACKEND] Solicitud de restaurar ventana: {}", window_id);
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Consulta inicial para descubrir las pantallas registradas al arrancar.
    async fn query_initial_state(
        &self,
    ) -> Result<(HashMap<String, Rect>, Vec<WindowNode>), BackendError> {
        let mut workspaces = HashMap::new();
        if let Ok(state) = self.state.lock() {
            for info in state.outputs.values() {
                let name = if info.name.is_empty() {
                    "default".to_string()
                } else {
                    info.name.clone()
                };
                let ws_id = format!("{}||default", name);
                workspaces.insert(ws_id, info.geometry);
            }
        }
        Ok((workspaces, Vec::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wayland_backend_creation() {
        let backend = WaylandBackend::new();
        assert_eq!(backend.name(), "wayland");
    }
}
