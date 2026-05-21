use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use zbus::interface;

use crate::application::controller::RavenController;
use crate::domain::action::RavenAction;
use crate::domain::error::RavenError;
use crate::domain::geometry::{Rect, WindowNode};

#[derive(Debug, Deserialize)]
pub struct KWinScreen {
    pub x: i32, pub y: i32, pub w: i32, pub h: i32,
}

#[derive(Debug, Deserialize)]
pub struct KWinWindow {
    pub id: String,
    #[serde(default)] pub ws: String,
    #[serde(default)] pub output: String,
    #[serde(default)] pub desktops: Vec<String>,
    #[serde(default)] pub f: bool,
    #[serde(default)] pub m: bool,
    #[serde(default)] pub p: bool,
    pub x: i32, pub y: i32, pub w: i32, pub h: i32,
    #[serde(default)] pub min_w: i32,
    #[serde(default)] pub min_h: i32,
    #[serde(default)] pub sb: bool,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct KWinTopology {
    #[serde(default)] pub outputs: Vec<String>,
    #[serde(default)] pub desktops: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct KWinPayload {
    #[serde(default)] pub windows: Vec<KWinWindow>,
    #[serde(default)] pub screens: HashMap<String, KWinScreen>,
    #[serde(default)] pub topology: KWinTopology,
}

fn parse_payload(payload_json: &str) -> Result<(HashMap<String, Rect>, Vec<WindowNode>, KWinTopology), RavenError> {
    if payload_json.is_empty() || payload_json == "{}" {
        return Ok((HashMap::new(), Vec::new(), KWinTopology::default()));
    }
    let payload: KWinPayload = serde_json::from_str(payload_json)
        .map_err(|e| RavenError::ValidationError(format!("Payload KWin inválido: {}", e)))?;

    let mut workspaces = HashMap::new();
    for (ws_id, screen) in payload.screens {
        workspaces.insert(ws_id, Rect::new(screen.x, screen.y, screen.w, screen.h));
    }

    let mut windows = Vec::with_capacity(payload.windows.len());
    for win in payload.windows {
        windows.push(WindowNode::new(
            win.id, win.ws, win.output, win.desktops, win.f, win.m, win.p,
            Rect::new(win.x, win.y, win.w, win.h), win.min_w, win.min_h, win.sb,
        ));
    }
    Ok((workspaces, windows, payload.topology))
}

#[derive(Debug, Serialize, Clone)]
pub struct TilingCommand {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub window_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")] pub target_ws: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub direction: Option<String>,
}

impl From<RavenAction> for TilingCommand {
    fn from(action: RavenAction) -> Self {
        match action {
            RavenAction::MoveWindow { window_id, x, y, width, height } => TilingCommand {
                action: "move".to_string(), window_id: Some(window_id), x: Some(x), y: Some(y), width: Some(width), height: Some(height), target_ws: None, direction: None,
            },
            RavenAction::FocusWindow { window_id } => TilingCommand {
                action: "focus".to_string(), window_id: Some(window_id), x: None, y: None, width: None, height: None, target_ws: None, direction: None,
            },
            RavenAction::MigrateToOutput { window_id, target_output } => TilingCommand {
                action: "migrate_to_output".to_string(), window_id: Some(window_id), target_ws: Some(target_output), x: None, y: None, width: None, height: None, direction: None,
            },
            RavenAction::MigrateToDesktop { window_id, target_desktop } => TilingCommand {
                action: "migrate_to_desktop".to_string(), window_id: Some(window_id), target_ws: Some(target_desktop), x: None, y: None, width: None, height: None, direction: None,
            },
            RavenAction::MinimizeWindow { window_id } => TilingCommand {
                action: "minimize".to_string(), window_id: Some(window_id), x: None, y: None, width: None, height: None, target_ws: None, direction: None,
            },
            RavenAction::UnminimizeWindow { window_id } => TilingCommand {
                action: "unminimize".to_string(), window_id: Some(window_id), x: None, y: None, width: None, height: None, target_ws: None, direction: None,
            },
            RavenAction::RequestFeedback { window_id } => TilingCommand {
                action: "request_feedback".to_string(), window_id: Some(window_id), x: None, y: None, width: None, height: None, target_ws: None, direction: None,
            },
        }
    }
}

pub struct RavenDBusService {
    pub controller: Arc<Mutex<RavenController>>,
    pub pending_commands: Arc<Mutex<Vec<TilingCommand>>>,
    pub active_window_id: Arc<Mutex<Option<String>>>,
    pub last_payload_json: Arc<Mutex<String>>,
    pub current_topology: Arc<Mutex<KWinTopology>>,
    pub tokio_handle: Handle,
}

#[interface(name = "org.kde.raven.Events")]
impl RavenDBusService {
    #[zbus(name = "syncState")]
    async fn sync_state(&self, payload_json: String) {
        if payload_json.len() > 5 * 1024 * 1024 { return; }

        static LAST_SYNC: AtomicU64 = AtomicU64::new(0);
        static CIRCUIT_BREAKER: AtomicU64 = AtomicU64::new(0);
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);

        if now < CIRCUIT_BREAKER.load(Ordering::Relaxed) || now - LAST_SYNC.load(Ordering::Relaxed) < 32 {
            if let Ok(mut last_payload) = self.last_payload_json.try_lock() { *last_payload = payload_json; }
            return;
        }
        LAST_SYNC.store(now, Ordering::Relaxed);

        let controller_clone = Arc::clone(&self.controller);
        let pending_clone = Arc::clone(&self.pending_commands);
        let payload_clone = Arc::clone(&self.last_payload_json);
        let topology_clone = Arc::clone(&self.current_topology);

        // Volvemos a procesar de forma asíncrona fuera del hilo de D-Bus de KWin
        self.tokio_handle.spawn(async move {
            *(payload_clone.lock().await) = payload_json.clone();
            let (workspaces, windows, topology) = match parse_payload(&payload_json) {
                Ok(p) => p,
                Err(_) => return,
            };
            *(topology_clone.lock().await) = topology;

            let mut ctrl = controller_clone.lock().await;
            if let Ok(commands) = ctrl.handle_state_change(workspaces, windows) {
                let mut queue = pending_clone.lock().await;
                if queue.len() > 150 {
                    queue.clear();
                    let future_time = now + 1000;
                    CIRCUIT_BREAKER.store(future_time, Ordering::Relaxed);
                    return;
                }
                let dbus_commands: Vec<TilingCommand> = commands.into_iter().map(Into::into).collect();
                queue.extend(dbus_commands);
            }
        });
    }

    #[zbus(name = "syncWindowDelta")]
    async fn sync_window_delta(&self, delta_json: String) {
        let controller_clone = Arc::clone(&self.controller);
        let pending_clone = Arc::clone(&self.pending_commands);

        self.tokio_handle.spawn(async move {
            if let Ok(win) = serde_json::from_str::<KWinWindow>(&delta_json) {
                let win_node = WindowNode::new(
                    win.id, win.ws, win.output, win.desktops, win.f, win.m, win.p,
                    Rect::new(win.x, win.y, win.w, win.h), win.min_w, win.min_h, win.sb,
                );
                let mut ctrl = controller_clone.lock().await;
                if let Ok(commands) = ctrl.handle_delta_change(win_node) {
                    let mut queue = pending_clone.lock().await;
                    let dbus_commands: Vec<TilingCommand> = commands.into_iter().map(Into::into).collect();
                    queue.extend(dbus_commands);
                }
            }
        });
    }

    #[zbus(name = "bridgeReady")]
    async fn bridge_ready(&self) {
        self.pending_commands.lock().await.clear();
        self.last_payload_json.lock().await.clear();
        self.controller.lock().await.reset_state();
    }

    #[zbus(name = "getPendingCommands")]
    async fn get_pending_commands(&self) -> String {
        let mut queue = self.pending_commands.lock().await;
        if queue.is_empty() {
            return String::from("[]");
        }

        let cmds = queue.drain(..).collect::<Vec<_>>();
        serde_json::to_string(&cmds).unwrap_or_else(|_| String::from("[]"))
    }

    #[zbus(name = "windowActivated")]
    async fn window_activated(&self, window_id: String) {
        if !window_id.trim().is_empty() { *self.active_window_id.lock().await = Some(window_id); }
    }

    // --- SHORTCUTS MANTENIDOS INTEGRALMENTE ---
    #[zbus(name = "toggleTiling")]
    async fn toggle_tiling(&self) { self.dispatch_shortcut("toggle_tiling", 0).await; }
    #[zbus(name = "incrementGaps")]
    async fn increment_gaps(&self, amount: i32) { self.dispatch_shortcut("increment_gaps", amount).await; }
    #[zbus(name = "incrementMaster")]
    async fn increment_master(&self) { self.dispatch_shortcut("increment_master", 0).await; }
    #[zbus(name = "decrementMaster")]
    async fn decrement_master(&self) { self.dispatch_shortcut("decrement_master", 0).await; }
    #[zbus(name = "increaseRatio")]
    async fn increase_ratio(&self) { self.dispatch_shortcut("increase_ratio", 0).await; }
    #[zbus(name = "decreaseRatio")]
    async fn decrease_ratio(&self) { self.dispatch_shortcut("decrease_ratio", 0).await; }
    #[zbus(name = "focusNext")]
    async fn focus_next(&self) { self.dispatch_shortcut("focus_next", 0).await; }
    #[zbus(name = "focusPrev")]
    async fn focus_prev(&self) { self.dispatch_shortcut("focus_prev", 0).await; }
    #[zbus(name = "migrateActiveToScreen")]
    async fn migrate_active_to_screen(&self) { self.dispatch_shortcut("migrate_active_to_screen", 0).await; }
    #[zbus(name = "migrateActiveToPrevScreen")]
    async fn migrate_active_to_prev_screen(&self) { self.dispatch_shortcut("migrate_active_to_prev_screen", 0).await; }
    #[zbus(name = "migrateActiveToDesktop")]
    async fn migrate_active_to_desktop(&self) { self.dispatch_shortcut("migrate_active_to_desktop", 0).await; }
    #[zbus(name = "migrateActiveToPrevDesktop")]
    async fn migrate_active_to_prev_desktop(&self) { self.dispatch_shortcut("migrate_active_to_prev_desktop", 0).await; }
    #[zbus(name = "getTilingState")]
    async fn get_tiling_state(&self) -> bool { self.controller.lock().await.is_tiling_enabled() }
    #[zbus(name = "getMonitorCount")]
    async fn get_monitor_count(&self) -> i32 {
        let topo = self.current_topology.lock().await;
        if !topo.outputs.is_empty() { topo.outputs.len() as i32 } else { 1 }
    }
    #[zbus(name = "getDesktopCount")]
    async fn get_desktop_count(&self) -> i32 {
        let topo = self.current_topology.lock().await;
        if !topo.desktops.is_empty() { topo.desktops.len() as i32 } else { 1 }
    }
}

impl RavenDBusService {
    async fn dispatch_shortcut(&self, action: &str, payload: i32) {
        let payload_json = self.last_payload_json.lock().await.clone();
        if payload_json.is_empty() && action != "toggle_tiling" { return; }

        let active_id = self.active_window_id.lock().await.clone();
        let (workspaces, parsed_windows, _topology) = parse_payload(&payload_json).unwrap_or_else(|_| (HashMap::new(), Vec::new(), KWinTopology::default()));

        let mut ctrl = self.controller.lock().await;
        if let Ok((needs_recalc, cmds)) = ctrl.handle_shortcut(action.to_string(), payload, parsed_windows, workspaces, active_id) {
            let mut queue = self.pending_commands.lock().await;
            if queue.len() > 200 { queue.clear(); }
            let dbus_commands: Vec<TilingCommand> = cmds.into_iter().map(Into::into).collect();
            queue.extend(dbus_commands);

            if needs_recalc {
                if let Ok((workspaces, windows, _topology)) = parse_payload(&payload_json) {
                    if let Ok(recalc_cmds) = ctrl.handle_state_change(workspaces, windows) {
                        let recalc_dbus_cmds: Vec<TilingCommand> = recalc_cmds.into_iter().map(Into::into).collect();
                        queue.extend(recalc_dbus_cmds);
                    }
                }
            }
        }
    }
}