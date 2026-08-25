use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tracing::info;

use crate::application::controller::RavenController;
use crate::infrastructure::dbus::{parse_payload, KWinTopology, TilingCommand, KWinWindow};
use crate::domain::geometry::{Rect, WindowNode};

/// Mensajes que el demonio D-Bus envía al Actor Central.
pub enum RavenMessage {
    SyncState {
        payload_json: String,
        reply: oneshot::Sender<String>,
    },
    SyncWindowDelta {
        delta_json: String,
        reply: oneshot::Sender<String>,
    },
    DispatchShortcut {
        action: String,
        payload: i32,
        payload_str: Option<String>,
        reply: oneshot::Sender<String>,
    },
    BridgeReady,
    WindowActivated {
        window_id: Option<String>,
    },
    GetQuarantineClasses {
        reply: oneshot::Sender<String>,
    },
    GetWindowRules {
        reply: oneshot::Sender<String>,
    },
    GetDesktopStatus {
        reply: oneshot::Sender<String>,
    },
    GetTilingState {
        reply: oneshot::Sender<bool>,
    },
    GetMonitorCount {
        reply: oneshot::Sender<i32>,
    },
    SetLayoutForCurrentWorkspace {
        layout_name: String,
        reply: oneshot::Sender<String>,
    },
}

/// Actor principal que posee (owns) el orquestador de lógica del motor de Raven.
pub struct RavenControllerActor {
    controller: RavenController,
    active_window_id: Option<String>,
    last_payload_json: String,
    current_topology: KWinTopology,
    rx: mpsc::Receiver<RavenMessage>,
}

impl RavenControllerActor {
    pub fn new(controller: RavenController, rx: mpsc::Receiver<RavenMessage>) -> Self {
        Self {
            controller,
            active_window_id: None,
            last_payload_json: String::from("{}"),
            current_topology: KWinTopology::default(),
            rx,
        }
    }

    pub async fn run(mut self) {
        info!("🎭 Actor Model (P3) Inicializado con Canal Bounded (Capacidad: 100)");

        while let Some(msg) = self.rx.recv().await {
            match msg {
                RavenMessage::SyncState { payload_json, reply } => {
                    self.last_payload_json = payload_json.clone();
                    let (workspaces, windows, topology) = parse_payload(&payload_json)
                        .unwrap_or_else(|_| (HashMap::new(), Vec::new(), KWinTopology::default()));
                    
                    self.current_topology = topology.clone();
                    self.controller.active_window_id = self.active_window_id.clone();
                    
                    let mut all_commands = Vec::new();
                    if let Ok(cmds) = self.controller.handle_state_change(workspaces, windows) {
                        all_commands.extend(cmds);
                    }
                    
                    let dbus_commands: Vec<TilingCommand> = all_commands.into_iter().map(Into::into).collect();
                    let response = serde_json::to_string(&dbus_commands).unwrap_or_else(|_| String::from("[]"));
                    let _ = reply.send(response);
                }
                RavenMessage::SyncWindowDelta { delta_json, reply } => {
                    let mut response = String::from("[]");
                    if let Ok(win) = serde_json::from_str::<KWinWindow>(&delta_json) {
                        let win_node = WindowNode::new(
                            win.id,
                            win.ws,
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
                        )
                        .with_class_and_caption(win.cls, win.cap);
                        
                        self.controller.handle_delta_change(win_node);
                        if let Ok(commands) = self.controller.commit_layout() {
                            let dbus_commands: Vec<TilingCommand> =
                                commands.into_iter().map(Into::into).collect();
                            response = serde_json::to_string(&dbus_commands).unwrap_or_else(|_| String::from("[]"));
                        }
                    }
                    let _ = reply.send(response);
                }
                RavenMessage::DispatchShortcut { action, payload, payload_str, reply } => {
                    let effective_active_id = payload_str.filter(|s| !s.trim().is_empty()).or_else(|| self.active_window_id.clone());
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
                    let dbus_commands: Vec<TilingCommand> = all_commands.into_iter().map(Into::into).collect();
                    let response = serde_json::to_string(&dbus_commands).unwrap_or_else(|_| String::from("[]"));
                    
                    let _ = reply.send(response);
                }
                RavenMessage::BridgeReady => {
                    self.last_payload_json.clear();
                    self.controller.reset_state();
                }
                RavenMessage::WindowActivated { window_id } => {
                    if let Some(ref id) = window_id {
                        if !id.trim().is_empty() {
                            self.active_window_id = window_id.clone();
                            self.controller.active_window_id = window_id;
                        }
                    }
                }
                RavenMessage::GetQuarantineClasses { reply } => {
                    let res = serde_json::to_string(&self.controller.get_config().quarantine_classes)
                        .unwrap_or_else(|_| String::from("[]"));
                    let _ = reply.send(res);
                }
                RavenMessage::GetWindowRules { reply } => {
                    let res = serde_json::to_string(&self.controller.get_config().window_rules)
                        .unwrap_or_else(|_| String::from("[]"));
                    let _ = reply.send(res);
                }
                RavenMessage::GetDesktopStatus { reply } => {
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
                RavenMessage::GetTilingState { reply } => {
                    let _ = reply.send(self.controller.is_tiling_enabled());
                }
                RavenMessage::GetMonitorCount { reply } => {
                    let count = if !self.current_topology.outputs.is_empty() {
                        self.current_topology.outputs.len() as i32
                    } else {
                        1
                    };
                    let _ = reply.send(count);
                }
                RavenMessage::SetLayoutForCurrentWorkspace { layout_name, reply } => {
                    let current_ws = self.active_window_id.as_ref().and_then(|wid| {
                        self.controller.get_engine().current_windows.get(wid).map(|w| w.workspace_id.clone())
                    });

                    if let Some(ws_id) = current_ws {
                        self.controller.get_engine_mut().config.workspace_layouts.insert(ws_id, layout_name);
                    } else {
                        self.controller.get_engine_mut().config.layout_type = layout_name;
                    }

                    let mut response = String::from("[]");
                    if let Ok(commands) = self.controller.commit_layout() {
                        let dbus_commands: Vec<TilingCommand> = commands.into_iter().map(Into::into).collect();
                        response = serde_json::to_string(&dbus_commands).unwrap_or_else(|_| String::from("[]"));
                    }
                    let _ = reply.send(response);
                }
            }
        }
    }
}
