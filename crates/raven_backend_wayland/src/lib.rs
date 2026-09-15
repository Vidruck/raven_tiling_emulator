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
/// 
/// Establece y mantiene una conexión directa con el compositor (e.g., KWin, Sway) 
/// utilizando el protocolo base de Wayland y extensiones como `wlr-foreign-toplevel-management`.
/// Su función principal es actuar como proveedor de topología y sensores de estado con latencia sub-milisegundo.
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
    ///
    /// Inicializa el estado compartido concurrente (`Arc<Mutex<WaylandState>>`) que almacenará
    /// y actualizará la información de salidas (monitores) y ventanas (toplevels) en tiempo real.
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

    /// Inicia el bucle de eventos escuchando el socket de Wayland.
    ///
    /// # Comportamiento
    /// Crea un hilo dedicado (`raven-wayland-event-loop`) que gestiona la cola de eventos
    /// de Wayland de forma bloqueante (`blocking_dispatch`). Los eventos procesados son
    /// traducidos a `CompositorEvent` y emitidos asíncronamente al motor a través del canal `event_tx`.
    async fn start_listener(
        &self,
        event_tx: mpsc::Sender<CompositorEvent>,
    ) -> Result<(), BackendError> {
        info!("[WAYLAND-BACKEND] Conectando al compositor Wayland nativo...");

        let conn = Connection::connect_to_env()
            .map_err(|e| BackendError::ConnectionFailed(format!("Error conectando a Wayland socket: {}", e)))?;

        let (state_tx, mut state_rx) = mpsc::channel(128);

        // Hilo asíncrono para reenviar eventos normalizados hacia el motor principal
        tokio::spawn(async move {
            while let Some(ev) = state_rx.recv().await {
                if event_tx.send(ev).await.is_err() {
                    break;
                }
            }
        });

        let state_clone = self.state.clone();

        // Hilo dedicado para la cola de eventos síncrona y continua de Wayland
        std::thread::Builder::new()
            .name("raven-wayland-event-loop".to_string())
            .spawn(move || {
                let mut event_queue = conn.new_event_queue();
                let qh = event_queue.handle();

                let display = conn.display();
                let _registry = display.get_registry(&qh, ());

                let mut local_state = WaylandState::new(Some(state_tx));

                // Roundtrip inicial: sincroniza y descubre outputs y toplevel managers activos
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

                    // Sincronizar estado concurrente para llamadas de consulta iniciales (query_initial_state)
                    if let Ok(mut lock) = state_clone.lock() {
                        lock.outputs = local_state.outputs.clone();
                    }
                }
            })
            .map_err(|e| BackendError::Other(format!("Fallo iniciando hilo Wayland: {}", e)))?;

        Ok(())
    }

    /// Aplica acciones emitidas por el motor hacia el compositor Wayland.
    ///
    /// *Nota:* En la arquitectura "Thin Bridge" actual para KDE Plasma, los cambios geométricos
    /// (movimiento/redimensionamiento) se delegan al script de KWin debido al modelo de seguridad.
    /// Este método atiende interacciones directas (como foco o minimización) mediante `toplevel_handle`.
    async fn apply_actions(
        &self,
        actions: Vec<RavenAction>,
    ) -> Result<(), BackendError> {
        for action in actions {
            match action {
                RavenAction::FocusWindow { window_id } => {
                    info!("[WAYLAND-BACKEND] Solicitud de foco para ventana: {}", window_id);
                    // TODO: Invocar handle.set_activated() sobre el toplevel_handle correspondiente
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

    /// Consulta inicial para descubrir las pantallas (workspaces virtuales) registradas al arrancar.
    ///
    /// Extrae las dimensiones nativas de los monitores descubiertos durante el `roundtrip` inicial.
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
