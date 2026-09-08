//! # Modelo de Actores de Concurrencia (`RavenControllerActor`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Implementa el patrón Actor sobre canales asíncronos Tokio (`mpsc` y `oneshot`)
//! para garantizar el acceso secuencial, seguro y libre de condiciones de carrera (*race conditions*)
//! sobre el estado mutable del motor de ventanas.

use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;

use crate::application::controller::RavenController;
use raven_backend_kwin::{parse_payload, KWinWindow, service::KWinBridgeMessage};
use crate::domain::geometry::{Rect, Topology, WindowNode};
use raven_core::backend::CompositorEvent;

/// Mensajes que el demonio recibe en su bucle de eventos (compatibilidad KWin + eventos universales de compositor).
pub enum RavenMessage {
    /// Mensaje originado desde el bridge D-Bus de KWin.
    KWinBridge(KWinBridgeMessage),
    /// Evento de compositor universal recibido desde la abstracción de backend.
    Compositor(CompositorEvent),
}

impl From<KWinBridgeMessage> for RavenMessage {
    fn from(msg: KWinBridgeMessage) -> Self {
        RavenMessage::KWinBridge(msg)
    }
}

/// Actor principal que posee (owns) el orquestador de lógica del motor de Raven.
pub struct RavenControllerActor {
    controller: RavenController,
    active_window_id: Option<String>,
    last_payload_json: String,
    current_topology: Topology,
    rx: mpsc::Receiver<RavenMessage>,
}

impl RavenControllerActor {
    pub fn new(controller: RavenController, rx: mpsc::Receiver<RavenMessage>) -> Self {
        Self {
            controller,
            active_window_id: None,
            last_payload_json: String::from("{}"),
            current_topology: Topology::default(),
            rx,
        }
    }

    pub async fn run(mut self) {
        info!("🎭 Actor Model (P3) Inicializado con Canal Bounded (Capacidad: 256)");

        while let Some(msg) = self.rx.recv().await {
            match msg {
                RavenMessage::Compositor(event) => {
                    match event {
                        CompositorEvent::WindowDiscovered(window) => {
                            self.controller.handle_delta_change(window);
                            let _ = self.controller.commit_layout();
                        }
                        CompositorEvent::WindowClosed(window_id) => {
                            let engine = self.controller.get_engine_mut();
                            engine.current_windows.remove(&window_id);
                            engine.window_history.retain(|id| id != &window_id);
                            engine.dynamic_floating_windows.retain(|id| id != &window_id);
                            let _ = self.controller.commit_layout();
                        }
                        CompositorEvent::WindowStateChanged { window_id, is_minimized, is_fullscreen, is_floating, geometry } => {
                            if let Some(win) = self.controller.get_engine_mut().current_windows.get_mut(&window_id) {
                                if let Some(m) = is_minimized { win.is_minimized = m; }
                                if let Some(f) = is_fullscreen { win.is_fullscreen = f; }
                                if let Some(fl) = is_floating { win.is_floating = fl; }
                                if let Some(geom) = geometry { win.geometry = geom; }
                            }
                            let _ = self.controller.commit_layout();
                        }
                        CompositorEvent::WindowFocused(wid) => {
                            self.active_window_id = wid.clone();
                            self.controller.active_window_id = wid.clone();
                            if let Some(ref id) = wid {
                                self.controller.get_engine_mut().promote_to_recent(id);
                            }
                        }
                        CompositorEvent::TopologyChanged(topology) => {
                            self.current_topology = topology;
                        }
                    }
                }
                RavenMessage::KWinBridge(kwin_msg) => match kwin_msg {
                    KWinBridgeMessage::SyncState { payload_json, reply } => {
                        self.last_payload_json = payload_json.clone();
                        let (workspaces, mut windows, topology) = parse_payload(&payload_json)
                            .unwrap_or_else(|_| (HashMap::new(), Vec::new(), Topology::default()));
                        
                        // Preservar la bandera de flotación dinámica si Rust ya mantiene la ventana en Quick Peek
                        for win in &mut windows {
                            if self.controller.get_engine().dynamic_floating_windows.contains(&win.window_id) {
                                win.is_floating = true;
                            }
                        }

                        self.current_topology = topology.clone();
                        self.controller.active_window_id = self.active_window_id.clone();
                        
                        let mut all_commands = Vec::new();
                        if let Ok(cmds) = self.controller.handle_state_change(workspaces, windows) {
                            all_commands.extend(cmds);
                        }
                        
                        let _ = reply.send(all_commands);
                    }
                    KWinBridgeMessage::SyncWindowDelta { delta_json, reply } => {
                        let mut commands = Vec::new();
                        if let Ok(win) = serde_json::from_str::<KWinWindow>(&delta_json) {
                            let ws_id = if !win.ws.is_empty() {
                                win.ws
                            } else {
                                let out_name = if !win.output.is_empty() { win.output.as_str() } else { "default" };
                                let desk_name = win.desktops.first().map(|d| d.as_str()).unwrap_or("default_desk");
                                format!("{}||{}", out_name, desk_name)
                            };

                            let is_dynamic_float = self.controller.get_engine().dynamic_floating_windows.contains(&win.id);
                            let win_node = WindowNode::new(
                                win.id,
                                ws_id,
                                win.output,
                                win.desktops,
                                win.f || is_dynamic_float,
                                win.m,
                                win.p,
                                Rect::new(win.x, win.y, win.w, win.h),
                                win.min_w,
                                win.min_h,
                                win.sb,
                                win.iq,
                                win.fs,
                            )
                            .with_class_and_caption(win.cls, win.cap);
                            
                            let is_tiled = !win_node.is_floating && !win_node.is_minimized;
                            let wid = win_node.window_id.clone();
                            self.controller.handle_delta_change(win_node);
                            
                            if is_tiled {
                                self.active_window_id = Some(wid.clone());
                                self.controller.active_window_id = Some(wid.clone());
                                self.controller.get_engine_mut().promote_to_recent(&wid);
                            }

                            if let Ok(recalc_cmds) = self.controller.commit_layout() {
                                commands.extend(recalc_cmds);
                            }
                        }
                        let _ = reply.send(commands);
                    }
                    KWinBridgeMessage::DispatchShortcut { action, payload, payload_str, reply } => {
                        let effective_active_id = payload_str.filter(|s| !s.trim().is_empty()).or_else(|| self.active_window_id.clone());
                        
                        if let Some(ref id) = effective_active_id {
                            self.active_window_id = Some(id.clone());
                            self.controller.active_window_id = Some(id.clone());
                            self.controller.get_engine_mut().promote_to_recent(id);
                        }

                        let mut all_commands = Vec::new();
                        if let Ok((needs_recalc, cmds)) = self.controller.handle_shortcut(
                            action,
                            payload,
                            effective_active_id,
                            &self.current_topology,
                        ) {
                            all_commands.extend(cmds);
                            
                            if needs_recalc {
                                if let Ok(recalc_cmds) = self.controller.commit_layout() {
                                    all_commands.extend(recalc_cmds);
                                }
                            }
                        }
                        let _ = reply.send(all_commands);
                    }
                    KWinBridgeMessage::BridgeReady => {
                        self.last_payload_json.clear();
                        self.controller.reset_state();
                    }
                    KWinBridgeMessage::WindowActivated { window_id } => {
                        if let Some(ref id) = window_id {
                            if !id.trim().is_empty() {
                                self.active_window_id = window_id.clone();
                                self.controller.active_window_id = window_id.clone();
                                // Promover de inmediato en el historial cíclico (LRU) como la ventana más reciente
                                self.controller.get_engine_mut().promote_to_recent(id);
                            }
                        }
                    }
                    KWinBridgeMessage::GetQuarantineClasses { reply } => {
                        let res = serde_json::to_string(&self.controller.get_config().quarantine_classes)
                            .unwrap_or_else(|_| String::from("[]"));
                        let _ = reply.send(res);
                    }
                    KWinBridgeMessage::GetWindowRules { reply } => {
                        let res = serde_json::to_string(&self.controller.get_config().window_rules)
                            .unwrap_or_else(|_| String::from("[]"));
                        let _ = reply.send(res);
                    }
                    KWinBridgeMessage::GetDesktopStatus { reply } => {
                        let topo = &self.current_topology;
                        let res = if topo.desktops.is_empty() {
                            String::from("1 | Escritorio 1 | 1")
                        } else {
                            let total = topo.desktops.len();
                            let current_idx = topo.desktops.iter().position(|d| d == &topo.current_desktop).unwrap_or(0);
                            let prev_idx = if current_idx == 0 { total - 1 } else { current_idx - 1 };
                            let next_idx = (current_idx + 1) % total;
                            format!("{} | Escritorio {} | {}", prev_idx + 1, current_idx + 1, next_idx + 1)
                        };
                        let _ = reply.send(res);
                    }
                    KWinBridgeMessage::GetTilingState { reply } => {
                        let _ = reply.send(self.controller.is_tiling_enabled());
                    }
                    KWinBridgeMessage::GetMonitorCount { reply } => {
                        let count = if !self.current_topology.outputs.is_empty() {
                            self.current_topology.outputs.len() as i32
                        } else {
                            1
                        };
                        let _ = reply.send(count);
                    }
                    KWinBridgeMessage::SetLayoutForCurrentWorkspace { layout_name, reply } => {
                        let current_ws = self.active_window_id.as_ref().and_then(|wid| {
                            self.controller.get_engine().current_windows.get(wid).map(|w| w.workspace_id.clone())
                        }).or_else(|| {
                            self.controller.get_engine().current_windows.values().next().map(|w| w.workspace_id.clone())
                        }).or_else(|| {
                            let out = self.current_topology.outputs.first().map(|s| s.as_str()).unwrap_or("default");
                            let desk = if !self.current_topology.current_desktop.is_empty() {
                                self.current_topology.current_desktop.as_str()
                            } else {
                                "default_desk"
                            };
                            Some(format!("{}||{}", out, desk))
                        });

                        self.controller.get_engine_mut().config.layout_type = layout_name.clone();
                        if let Some(ws_id) = current_ws {
                            self.controller.get_engine_mut().config.workspace_layouts.insert(ws_id, layout_name.clone());
                        }

                        if let Err(e) = self.controller.get_engine().config.save() {
                            tracing::warn!("[ACTOR] Error al persistir configuración tras cambio de layout: {}", e);
                        }

                        let readable_name = match layout_name.as_str() {
                            "tall" => "Tall (Columna)",
                            "monocle" => "Monocle (Monocromático)",
                            "strict_dwindle" => "Strict Dwindle (Espiral)",
                            "inverted_strict_dwindle" => "Inverted Dwindle (Espiral Invertida)",
                            "divisor" => "Divisor (Cuadrícula)",
                            _ => "Raven BSP (Foveal)",
                        };
                        tokio::spawn(async move {
                            let _ = tokio::process::Command::new("notify-send")
                                .arg("-a")
                                .arg("Raven Tiling")
                                .arg("-t")
                                .arg("1200")
                                .arg("-h")
                                .arg("string:x-canonical-private-synchronous:raven-osd")
                                .arg("Disposición de Ventanas")
                                .arg(format!("Layout: {}", readable_name))
                                .output()
                                .await;
                        });

                        let mut commands = Vec::new();
                        if let Ok(cmds) = self.controller.commit_layout() {
                            commands = cmds;
                        }
                        let _ = reply.send(commands);
                    }
                },
            }
        }
    }
}
