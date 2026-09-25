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
pub mod effect;
pub mod parser;
pub mod quarantine;
pub mod service;

pub use commands::TilingCommand;
pub use effect::{RavenEffectClient, WindowAnimation};
pub use parser::{parse_payload, KWinPayload, KWinScreen, KWinTopology, KWinWindow};
pub use quarantine::{KWinQuarantineManager, QuarantineCategory, StartupMode};
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
    /// Cliente D-Bus para el efecto gráfico nativo de KWin (kwin4_effect_raven).
    effect_client: RavenEffectClient,
    /// Administrador nativo de cuarentena, filtrado y temporizadores de estabilización.
    quarantine_manager: KWinQuarantineManager,
    /// Registro de últimas geometrías conocidas por ventana para interpolación dinámica de movimientos y estiramiento.
    last_known_geometries: Arc<tokio::sync::RwLock<HashMap<String, Rect>>>,
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
            effect_client: RavenEffectClient::new(),
            quarantine_manager: KWinQuarantineManager::new(),
            last_known_geometries: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Retorna una referencia al cliente del efecto nativo de KWin.
    pub fn effect_client(&self) -> &RavenEffectClient {
        &self.effect_client
    }

    /// Retorna una referencia al administrador de cuarentena y filtrado.
    pub fn quarantine_manager(&self) -> &KWinQuarantineManager {
        &self.quarantine_manager
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
            *guard = Some(arc_conn.clone());
        }
        self.effect_client.set_connection(arc_conn).await;
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
        let last_known_geoms_clone = self.last_known_geometries.clone();

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
                                win.id.clone(),
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
                            ).with_class_and_caption(win.cls, win.cls_name, win.sus, win.cap);

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
                    KWinBridgeMessage::CommandAppliedState { window_id, x, y, width, height } => {
                        let mut geom_guard = last_known_geoms_clone.write().await;
                        geom_guard.insert(window_id, Rect::new(x, y, width, height));
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

        let mut animations = Vec::new();
        {
            let mut geom_guard = self.last_known_geometries.write().await;
            for action in &actions {
                match action {
                    RavenAction::ReleaseQuarantine { window_id } => {
                        // 250ms de animación de nacimiento con zoom y elastic
                        self.effect_client.animate_birth(window_id, 250).await;
                    }
                    RavenAction::MoveWindow { window_id, x, y, width, height } |
                    RavenAction::RectifyWindow { window_id, x, y, width, height } => {
                        let target_rect = Rect::new(*x, *y, *width, *height);
                        if let Some(prev_rect) = geom_guard.get(window_id).copied() {
                            if prev_rect != target_rect {
                                animations.push(WindowAnimation {
                                    id: window_id.clone(),
                                    from: [prev_rect.x as f64, prev_rect.y as f64, prev_rect.width as f64, prev_rect.height as f64],
                                    to: [target_rect.x as f64, target_rect.y as f64, target_rect.width as f64, target_rect.height as f64],
                                    duration_ms: 150,
                                    easing: "EaseOutCubic".to_string(),
                                });
                            }
                        }
                        geom_guard.insert(window_id.clone(), target_rect);
                    }
                    _ => {}
                }
            }
        }

        // Si hay animaciones de movimiento o redimensionamiento (estiramiento), despachar al efecto C++
        if !animations.is_empty() {
            self.effect_client.animate_batch(animations).await;
        }

        // Las órdenes D-Bus (C++ effect) y KWin Script se disparan concurrentemente.
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
