use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use tracing::debug;
use zbus::interface;

use crate::application::controller::RavenController;
use crate::domain::action::RavenAction;
use crate::domain::error::RavenError;
use crate::domain::geometry::{Rect, WindowNode};

/// Representa la geometría de una pantalla en la estructura de serialización de KWin.
#[derive(Debug, Deserialize)]
pub struct KWinScreen {
    /// Posición en X.
    pub x: i32,
    /// Posición en Y.
    pub y: i32,
    /// Ancho en píxeles.
    pub w: i32,
    /// Alto en píxeles.
    pub h: i32,
}

/// Representa el estado de una ventana enviado por el puente de KWin.
#[derive(Debug, Deserialize)]
pub struct KWinWindow {
    /// Identificador único de la ventana.
    pub id: String,
    /// Identificador del área de trabajo (workspace ID) actual.
    #[serde(default)]
    pub ws: String,
    /// Nombre de la salida (output) física a la que pertenece.
    #[serde(default)]
    pub output: String,
    /// Listado de identificadores de escritorios virtuales.
    #[serde(default)]
    pub desktops: Vec<String>,
    /// Indica si la ventana está marcada como flotante (floating).
    #[serde(default)]
    pub f: bool,
    /// Indica si la ventana está minimizada.
    #[serde(default)]
    pub m: bool,
    /// Indica si la ventana está en modo Picture-in-Picture o keepAbove.
    #[serde(default)]
    pub p: bool,
    /// Coordenada horizontal actual.
    pub x: i32,
    /// Coordenada vertical actual.
    pub y: i32,
    /// Ancho en píxeles.
    pub w: i32,
    /// Alto en píxeles.
    pub h: i32,
    /// Ancho mínimo admitido por la ventana.
    #[serde(default)]
    pub min_w: i32,
    /// Alto mínimo admitido por la ventana.
    #[serde(default)]
    pub min_h: i32,
    /// Indica si la ventana requiere retroalimentación (feedback) de sincronización inmediata tras su creación.
    #[serde(default)]
    pub sb: bool,
}

/// Representa el estado global de salidas y escritorios virtuales en KWin.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct KWinTopology {
    /// Listado de nombres de salidas (outputs) físicas de pantalla.
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Listado de identificadores de escritorios virtuales activos.
    #[serde(default)]
    pub desktops: Vec<String>,
}

/// Carga de datos (payload) completa con el estado del compositor KWin.
#[derive(Debug, Deserialize)]
pub struct KWinPayload {
    /// Ventanas activas rastreadas por el puente.
    #[serde(default)]
    pub windows: Vec<KWinWindow>,
    /// Mapeo de áreas de trabajo útiles y sus geometrías.
    #[serde(default)]
    pub screens: HashMap<String, KWinScreen>,
    /// Topología actual de salidas y escritorios de KWin.
    #[serde(default)]
    pub topology: KWinTopology,
}

/// Deserializa y normaliza una cadena de texto JSON a las entidades del dominio de Raven.
///
/// # Parámetros
/// * `payload_json` - Cadena JSON enviada por el puente JavaScript de KWin.
///
/// # Retorno
/// Tupla que contiene el mapa de geometrías de las áreas de trabajo, la lista de ventanas normalizadas y la topología actual.
fn parse_payload(
    payload_json: &str,
) -> Result<(HashMap<String, Rect>, Vec<WindowNode>, KWinTopology), RavenError> {
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
        ));
    }
    Ok((workspaces, windows, payload.topology))
}

/// Comando de redimensionamiento, movimiento o foco serializado para el puente de KWin.
#[derive(Debug, Serialize, Clone)]
pub struct TilingCommand {
    /// Acción a ejecutar (p. ej., "move", "focus", "minimize").
    pub action: String,
    /// Identificador único de la ventana objetivo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
    /// Coordenada horizontal de destino.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    /// Coordenada vertical de destino.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    /// Ancho final en píxeles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    /// Alto final en píxeles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    /// Identificador del área de trabajo destino para migraciones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ws: Option<String>,
    /// Dirección del comando.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

impl From<RavenAction> for TilingCommand {
    /// Convierte una acción lógica de dominio `RavenAction` a un comando físico `TilingCommand`.
    fn from(action: RavenAction) -> Self {
        match action {
            RavenAction::MoveWindow {
                window_id,
                x,
                y,
                width,
                height,
            } => TilingCommand {
                action: "move".to_string(),
                window_id: Some(window_id),
                x: Some(x),
                y: Some(y),
                width: Some(width),
                height: Some(height),
                target_ws: None,
                direction: None,
            },
            RavenAction::FocusWindow { window_id } => TilingCommand {
                action: "focus".to_string(),
                window_id: Some(window_id),
                x: None,
                y: None,
                width: None,
                height: None,
                target_ws: None,
                direction: None,
            },
            RavenAction::MigrateToOutput {
                window_id,
                target_output,
            } => TilingCommand {
                action: "migrate_to_output".to_string(),
                window_id: Some(window_id),
                target_ws: Some(target_output),
                x: None,
                y: None,
                width: None,
                height: None,
                direction: None,
            },
            RavenAction::MigrateToDesktop {
                window_id,
                target_desktop,
            } => TilingCommand {
                action: "migrate_to_desktop".to_string(),
                window_id: Some(window_id),
                target_ws: Some(target_desktop),
                x: None,
                y: None,
                width: None,
                height: None,
                direction: None,
            },
            RavenAction::MinimizeWindow { window_id } => TilingCommand {
                action: "minimize".to_string(),
                window_id: Some(window_id),
                x: None,
                y: None,
                width: None,
                height: None,
                target_ws: None,
                direction: None,
            },
            RavenAction::UnminimizeWindow { window_id } => TilingCommand {
                action: "unminimize".to_string(),
                window_id: Some(window_id),
                x: None,
                y: None,
                width: None,
                height: None,
                target_ws: None,
                direction: None,
            },
            RavenAction::RequestFeedback { window_id } => TilingCommand {
                action: "request_feedback".to_string(),
                window_id: Some(window_id),
                x: None,
                y: None,
                width: None,
                height: None,
                target_ws: None,
                direction: None,
            },
            RavenAction::SaturationWarning { cmax, active } => TilingCommand {
                action: "saturation_warning".to_string(),
                window_id: None,
                x: Some(cmax as i32),
                y: Some(active as i32),
                width: None,
                height: None,
                target_ws: None,
                direction: None,
            },
        }
    }
}

/// Implementación del demonio (daemon) del bus D-Bus de Raven.
///
/// Actúa como canal de comunicación interactivo e incremental entre KWin y el motor de mosaico.
pub struct RavenDBusService {
    /// Instancia protegida del orquestador de lógica del motor de Raven.
    pub controller: Arc<Mutex<RavenController>>,
    /// Cola (queue) de comandos calculados pendientes de ser recogidos por KWin.
    pub pending_commands: Arc<Mutex<Vec<TilingCommand>>>,
    /// Identificador único de la ventana actualmente enfocada en el sistema.
    pub active_window_id: Arc<Mutex<Option<String>>>,
    /// Último payload en formato JSON recibido para optimización de atajos.
    pub last_payload_json: Arc<Mutex<String>>,
    /// Topología física de pantallas y escritorios virtuales en tiempo real.
    pub current_topology: Arc<Mutex<KWinTopology>>,
    /// Manejador de hilos asíncronos del runtime de Tokio.
    pub tokio_handle: Handle,
}

#[interface(name = "org.kde.raven.Events")]
impl RavenDBusService {
    /// Recibe y procesa el estado global de las ventanas y del compositor KWin de forma asíncrona.
    ///
    /// Cuenta con un cortocircuito (circuit breaker) que descarta paquetes si hay saturación.
    #[zbus(name = "syncState")]
    async fn sync_state(&self, payload_json: String) {
        if payload_json.len() > 5 * 1024 * 1024 {
            return;
        }

        static LAST_SYNC: AtomicU64 = AtomicU64::new(0);
        static CIRCUIT_BREAKER: AtomicU64 = AtomicU64::new(0);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        if now < CIRCUIT_BREAKER.load(Ordering::Relaxed)
            || now - LAST_SYNC.load(Ordering::Relaxed) < 32
        {
            if let Ok(mut last_payload) = self.last_payload_json.try_lock() {
                *last_payload = payload_json;
            }
            return;
        }
        LAST_SYNC.store(now, Ordering::Relaxed);

        let controller_clone = Arc::clone(&self.controller);
        let pending_clone = Arc::clone(&self.pending_commands);
        let payload_clone = Arc::clone(&self.last_payload_json);
        let topology_clone = Arc::clone(&self.current_topology);

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
                let dbus_commands: Vec<TilingCommand> =
                    commands.into_iter().map(Into::into).collect();

                // --- Canal Push-Based (v2.8) ---
                // Intentar invocar receiveCommands() directamente en el bridge JS.
                // Si el método está disponible en org.kde.kwin, el bridge lo recibe
                // de forma inmediata sin esperar el próximo ciclo de polling.
                // Si falla (bridge no expuesto o script no cargado), los comandos
                // se encolan en pending_commands para ser recogidos por el fallback.
                let push_ok = if !dbus_commands.is_empty() {
                    if let Ok(json) = serde_json::to_string(&dbus_commands) {
                        // La llamada se hace fire-and-forget; un error no es fatal.
                        match zbus::Connection::session().await {
                            Ok(conn) => {
                                let result = conn
                                    .call_method(
                                        Some("org.kde.kwin.Script"),
                                        "/Scripting",
                                        Some("org.kde.kwin.Script"),
                                        "receiveCommands",
                                        &json,
                                    )
                                    .await;
                                result.is_ok()
                            }
                            Err(_) => false,
                        }
                    } else {
                        false
                    }
                } else {
                    true // sin comandos que empujar, no necesita fallback
                };

                if !push_ok {
                    // Fallback: encolar para que getPendingCommands() los entregue
                    debug!("[PUSH] Canal push no disponible, usando cola de fallback.");
                    queue.extend(dbus_commands);
                }
            }
        });
    }

    /// Sincroniza de forma incremental (delta sync) el cambio de geometría o estado de una única ventana.
    #[zbus(name = "syncWindowDelta")]
    async fn sync_window_delta(&self, delta_json: String) {
        let controller_clone = Arc::clone(&self.controller);
        let pending_clone = Arc::clone(&self.pending_commands);

        self.tokio_handle.spawn(async move {
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
                );
                let mut ctrl = controller_clone.lock().await;
                if let Ok(commands) = ctrl.handle_delta_change(win_node) {
                    let mut queue = pending_clone.lock().await;
                    let dbus_commands: Vec<TilingCommand> =
                        commands.into_iter().map(Into::into).collect();
                    queue.extend(dbus_commands);
                }
            }
        });
    }

    /// Notifica que el puente de JavaScript se ha restablecido y está listo.
    #[zbus(name = "bridgeReady")]
    async fn bridge_ready(&self) {
        self.pending_commands.lock().await.clear();
        self.last_payload_json.lock().await.clear();
        self.controller.lock().await.reset_state();
    }

    /// Retorna los comandos pendientes acumulados en la cola y los elimina.
    #[zbus(name = "getPendingCommands")]
    async fn get_pending_commands(&self) -> String {
        let mut queue = self.pending_commands.lock().await;
        if queue.is_empty() {
            return String::from("[]");
        }

        let cmds = queue.drain(..).collect::<Vec<_>>();
        serde_json::to_string(&cmds).unwrap_or_else(|_| String::from("[]"))
    }

    /// Registra el identificador de la ventana activa enfocada en KWin.
    #[zbus(name = "windowActivated")]
    async fn window_activated(&self, window_id: String) {
        let val = if window_id.trim().is_empty() {
            None
        } else {
            Some(window_id.clone())
        };
        *self.active_window_id.lock().await = val.clone();
        self.controller.lock().await.active_window_id = val;
    }

    /// Alterna el estado operativo de activación del motor de mosaico.
    #[zbus(name = "toggleTiling")]
    async fn toggle_tiling(&self) {
        self.dispatch_shortcut("toggle_tiling", 0).await;
    }

    /// Incrementa o decrementa la separación (gaps) entre las ventanas.
    #[zbus(name = "incrementGaps")]
    async fn increment_gaps(&self, amount: i32) {
        self.dispatch_shortcut("increment_gaps", amount).await;
    }

    /// Incrementa el límite óptimo de ventanas activas en la composición foveal.
    #[zbus(name = "incrementMaster")]
    async fn increment_master(&self) {
        self.dispatch_shortcut("increment_nmaster", 1).await;
    }

    /// Decrementa el límite óptimo de ventanas activas en la composición foveal.
    #[zbus(name = "decrementMaster")]
    async fn decrement_master(&self) {
        self.dispatch_shortcut("decrement_nmaster", 1).await;
    }

    /// Aumenta el ratio de división (split ratio) asimétrica de la espiral BSP.
    #[zbus(name = "increaseRatio")]
    async fn increase_ratio(&self) {
        self.dispatch_shortcut("increase_ratio", 0).await;
    }

    /// Disminuye el ratio de división (split ratio) asimétrica de la espiral BSP.
    #[zbus(name = "decreaseRatio")]
    async fn decrease_ratio(&self) {
        self.dispatch_shortcut("decrease_ratio", 0).await;
    }

    /// Envía el foco a la ventana siguiente del mosaico.
    #[zbus(name = "focusNext")]
    async fn focus_next(&self) {
        self.dispatch_shortcut("focus_next", 0).await;
    }

    /// Retorna la lista de clases en cuarentena configuradas.
    #[zbus(name = "getQuarantineClasses")]
    async fn get_quarantine_classes(&self) -> String {
        let controller = self.controller.lock().await;
        serde_json::to_string(&controller.get_config().quarantine_classes)
            .unwrap_or_else(|_| String::from("[]"))
    }

    /// Retorna la lista de reglas de ventanas configuradas.
    #[zbus(name = "getWindowRules")]
    async fn get_window_rules(&self) -> String {
        let controller = self.controller.lock().await;
        serde_json::to_string(&controller.get_config().window_rules)
            .unwrap_or_else(|_| String::from("[]"))
    }

    /// Envía el foco a la ventana anterior del mosaico.
    #[zbus(name = "focusPrev")]
    async fn focus_prev(&self) {
        self.dispatch_shortcut("focus_prev", 0).await;
    }

    /// Intercambia la ventana activa con la siguiente en la pila.
    #[zbus(name = "swapNext")]
    async fn swap_next(&self) {
        self.dispatch_shortcut("swap_next", 0).await;
    }

    /// Intercambia la ventana activa con la anterior en la pila.
    #[zbus(name = "swapPrev")]
    async fn swap_prev(&self) {
        self.dispatch_shortcut("swap_prev", 0).await;
    }

    /// Migra la ventana activa al monitor siguiente.
    #[zbus(name = "migrateActiveToScreen")]
    async fn migrate_active_to_screen(&self) {
        self.dispatch_shortcut("migrate_active_to_screen", 0).await;
    }

    /// Migra la ventana activa al monitor anterior.
    #[zbus(name = "migrateActiveToPrevScreen")]
    async fn migrate_active_to_prev_screen(&self) {
        self.dispatch_shortcut("migrate_active_to_prev_screen", 0)
            .await;
    }

    /// Migra la ventana activa al escritorio virtual siguiente.
    #[zbus(name = "migrateActiveToDesktop")]
    async fn migrate_active_to_desktop(&self) {
        self.dispatch_shortcut("migrate_active_to_desktop", 0).await;
    }

    /// Migra la ventana activa al escritorio virtual anterior.
    #[zbus(name = "migrateActiveToPrevDesktop")]
    async fn migrate_active_to_prev_desktop(&self) {
        self.dispatch_shortcut("migrate_active_to_prev_desktop", 0)
            .await;
    }

    /// Retorna si el motor de mosaico (tiling) está activado.
    #[zbus(name = "getTilingState")]
    async fn get_tiling_state(&self) -> bool {
        self.controller.lock().await.is_tiling_enabled()
    }

    /// Retorna el número actual de monitores o salidas físicas activas.
    #[zbus(name = "getMonitorCount")]
    async fn get_monitor_count(&self) -> i32 {
        let topo = self.current_topology.lock().await;
        if !topo.outputs.is_empty() {
            topo.outputs.len() as i32
        } else {
            1
        }
    }

    /// Retorna el número actual de escritorios virtuales activos.
    #[zbus(name = "getDesktopCount")]
    async fn get_desktop_count(&self) -> i32 {
        let topo = self.current_topology.lock().await;
        if !topo.desktops.is_empty() {
            topo.desktops.len() as i32
        } else {
            1
        }
    }
}

impl RavenDBusService {
    /// Despacha de forma asíncrona una acción de atajo de teclado, recalculando el layout si es necesario.
    async fn dispatch_shortcut(&self, action: &str, payload: i32) {
        let payload_json = self.last_payload_json.lock().await.clone();
        if payload_json.is_empty() && action != "toggle_tiling" {
            return;
        }

        let active_id = self.active_window_id.lock().await.clone();
        let (workspaces, parsed_windows, _topology) = parse_payload(&payload_json)
            .unwrap_or_else(|_| (HashMap::new(), Vec::new(), KWinTopology::default()));

        let mut ctrl = self.controller.lock().await;
        if let Ok((needs_recalc, cmds)) = ctrl.handle_shortcut(
            action.to_string(),
            payload,
            parsed_windows,
            workspaces,
            active_id,
        ) {
            let mut queue = self.pending_commands.lock().await;
            if queue.len() > 200 {
                queue.clear();
            }
            let dbus_commands: Vec<TilingCommand> = cmds.into_iter().map(Into::into).collect();
            queue.extend(dbus_commands);

            if needs_recalc {
                if let Ok((workspaces, windows, _topology)) = parse_payload(&payload_json) {
                    if let Ok(recalc_cmds) = ctrl.handle_state_change(workspaces, windows) {
                        let recalc_dbus_cmds: Vec<TilingCommand> =
                            recalc_cmds.into_iter().map(Into::into).collect();
                        queue.extend(recalc_dbus_cmds);
                    }
                }
            }
        }
    }
}
