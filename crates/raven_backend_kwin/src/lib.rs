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
pub use service::{actions_to_kwin_json, KWinBridgeMessage, KWinDbusService};

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
#[derive(Clone)]
pub struct KWinBackend {
    /// Nombre identificador del backend.
    name: &'static str,
    /// Conexión activa al bus D-Bus de sesión de KWin (mantiene vivo el registro D-Bus).
    connection: Arc<tokio::sync::RwLock<Option<Arc<Connection>>>>,
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
            connection: Arc::new(tokio::sync::RwLock::new(None)),
        }
    }

    /// Inicia el servicio D-Bus de KWin (`org.kde.raven.Daemon`) e intermedia los mensajes hacia el canal receptor de Raven.
    ///
    /// Este método desacopla al binario principal de los detalles de sesión de D-Bus y `ConnectionBuilder`.
    pub async fn start_bridge<M>(
        &self,
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

        let arc_conn = Arc::new(conn);
        {
            let mut guard = self.connection.write().await;
            *guard = Some(arc_conn);
        }
        info!("[KWIN-BACKEND] Servicio org.kde.raven.Daemon registrado exitosamente como intermediario.");

        Ok(())
    }

    /// Retorna una referencia a la conexión D-Bus activa, si el bridge ha sido iniciado.
    pub async fn connection(&self) -> Option<Arc<Connection>> {
        self.connection.read().await.clone()
    }
}

#[async_trait]
impl CompositorBackend for KWinBackend {
    fn name(&self) -> &'static str {
        self.name
    }

    /// Implementa `start_listener` traduciendo eventos recibidos desde el puente de KWin
    /// hacia eventos normalizados `CompositorEvent` en el canal agnóstico de compositores.
    async fn start_listener(
        &self,
        event_tx: mpsc::Sender<CompositorEvent>,
    ) -> Result<(), BackendError> {
        info!("[KWIN-BACKEND] Iniciando CompositorBackend::start_listener para KWin / Plasma...");

        let (bridge_tx, mut bridge_rx) = mpsc::channel(256);

        // Subtarea que traduce eventos del bridge D-Bus de KWin hacia CompositorEvent universales
        tokio::spawn(async move {
            while let Some(msg) = bridge_rx.recv().await {
                match msg {
                    KWinBridgeMessage::WindowActivated { window_id } => {
                        let _ = event_tx.send(CompositorEvent::WindowFocused(window_id)).await;
                    }
                    KWinBridgeMessage::SyncState { payload_json, reply } => {
                        if let Ok((workspaces, windows, topology)) = parse_payload(&payload_json) {
                            // Emitir cambio de topología universal
                            let _ = event_tx.send(CompositorEvent::TopologyChanged(topology)).await;

                            // Emitir ventanas descubiertas para sincronización universal
                            for win in windows {
                                let _ = event_tx.send(CompositorEvent::WindowDiscovered(win)).await;
                            }
                            let _ = workspaces;
                        }
                        // Responder con vector vacío si se usa como listener puro
                        let _ = reply.send(Vec::new());
                    }
                    KWinBridgeMessage::SyncWindowDelta { delta_json, reply } => {
                        if let Ok(win) = serde_json::from_str::<KWinWindow>(&delta_json) {
                            let ws_id = if !win.ws.is_empty() {
                                win.ws
                            } else {
                                let out_name = if !win.output.is_empty() { win.output.as_str() } else { "default" };
                                let desk_name = win.desktops.first().map(|d| d.as_str()).unwrap_or("default_desk");
                                format!("{}||{}", out_name, desk_name)
                            };

                            let win_node = WindowNode::new(
                                win.id,
                                ws_id,
                                win.output,
                                win.desktops,
                                win.f,
                                win.m,
                                win.p,
                                Rect::new(win.x, win.y, win.w, win.h),
                                win.min_w,
                                win.min_h,
                                win.sb,
                                win.iq,
                                win.fs,
                            ).with_class_and_caption(win.cls, win.cap);

                            let _ = event_tx.send(CompositorEvent::WindowDiscovered(win_node)).await;
                        }
                        let _ = reply.send(Vec::new());
                    }
                    KWinBridgeMessage::DispatchShortcut { reply, .. } => {
                        let _ = reply.send(Vec::new());
                    }
                    KWinBridgeMessage::SetLayoutForCurrentWorkspace { reply, .. } => {
                        let _ = reply.send(Vec::new());
                    }
                    KWinBridgeMessage::GetQuarantineClasses { reply } => {
                        let _ = reply.send(String::from("[]"));
                    }
                    KWinBridgeMessage::GetWindowRules { reply } => {
                        let _ = reply.send(String::from("[]"));
                    }
                    KWinBridgeMessage::GetDesktopStatus { reply } => {
                        let _ = reply.send(String::from("1 | Escritorio 1 | 1"));
                    }
                    KWinBridgeMessage::GetTilingState { reply } => {
                        let _ = reply.send(true);
                    }
                    KWinBridgeMessage::GetMonitorCount { reply } => {
                        let _ = reply.send(1);
                    }
                    KWinBridgeMessage::BridgeReady => {}
                }
            }
        });

        // Registrar servicio D-Bus si no está iniciado aún
        let is_registered = self.connection.read().await.is_some();
        if !is_registered {
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

            let mut guard = self.connection.write().await;
            *guard = Some(Arc::new(conn));
        }

        info!("[KWIN-BACKEND] Escucha de eventos de compositor KWin iniciada correctamente.");
        Ok(())
    }

    async fn apply_actions(
        &self,
        actions: Vec<RavenAction>,
    ) -> Result<(), BackendError> {
        if actions.is_empty() {
            return Ok(());
        }

        let conn_opt = self.connection.read().await.clone();
        if let Some(conn) = conn_opt {
            let json = actions_to_kwin_json(actions);
            if json != "[]" {
                conn.emit_signal(
                    Option::<&str>::None,
                    "/Events",
                    "org.kde.raven.Events",
                    "tilingCommandsPending",
                    &(&json,),
                )
                .await
                .map_err(|e| BackendError::ActionDispatchFailed(e.to_string()))?;
            }
        }
        Ok(())
    }

    async fn query_initial_state(
        &self,
    ) -> Result<(HashMap<String, Rect>, Vec<WindowNode>), BackendError> {
        Ok((HashMap::new(), Vec::new()))
    }
}
