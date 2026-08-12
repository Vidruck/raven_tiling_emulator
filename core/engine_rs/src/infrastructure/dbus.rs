use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use zbus::interface;

use crate::application::actor::RavenMessage;
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
    /// Indica si la ventana se encuentra en cuarentena de estabilización (Gecko/CSD).
    #[serde(default)]
    pub iq: bool,
    /// Indica si la ventana está en modo pantalla completa nativa.
    #[serde(default)]
    pub fs: bool,
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
    /// Identificador del escritorio actual activo.
    #[serde(default)]
    pub current_desktop: String,
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
pub fn parse_payload(
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
            win.iq,
            win.fs,
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
    pub tx: mpsc::Sender<RavenMessage>,
}

#[interface(name = "org.kde.raven.Events")]
impl RavenDBusService {
    /// Recibe y procesa el estado global de las ventanas, retornando inmediatamente los comandos geométricos.
    ///
    /// Este método implementa la arquitectura Single-Trip, reduciendo la latencia y eliminando el polling.
    #[zbus(name = "syncStateAndUpdateLayout")]
    async fn sync_state_and_update_layout(&self, payload_json: String) -> String {
        if payload_json.len() > 5 * 1024 * 1024 {
            return String::from("[]");
        }

        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = RavenMessage::SyncState {
            payload_json,
            reply: reply_tx,
        };

        if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        }
    }

    /// Sincroniza de forma incremental (delta sync) el cambio de geometría o estado de una única ventana.
    #[zbus(name = "syncWindowDelta")]
    async fn sync_window_delta(&self, delta_json: String) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = RavenMessage::SyncWindowDelta {
            delta_json,
            reply: reply_tx,
        };

        if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        }
    }

    /// Notifica que el puente de JavaScript se ha restablecido y está listo.
    #[zbus(name = "bridgeReady")]
    async fn bridge_ready(&self) {
        let _ = self.tx.send(RavenMessage::BridgeReady).await;
    }

    /// Registra el identificador de la ventana activa enfocada en KWin.
    #[zbus(name = "windowActivated")]
    async fn window_activated(&self, window_id: String) {
        let val = if window_id.trim().is_empty() {
            None
        } else {
            Some(window_id.clone())
        };
        let _ = self.tx.send(RavenMessage::WindowActivated { window_id: val }).await;
    }

    /// Alterna el estado operativo de activación del motor de mosaico.
    #[zbus(name = "toggleTiling")]
    async fn toggle_tiling(&self) -> String {
        self.dispatch_shortcut("toggle_tiling", 0).await
    }

    /// Incrementa o decrementa la separación (gaps) entre las ventanas.
    #[zbus(name = "incrementGaps")]
    async fn increment_gaps(&self, amount: i32) -> String {
        self.dispatch_shortcut("increment_gaps", amount).await
    }

    /// Incrementa el límite óptimo de ventanas activas en la composición foveal.
    #[zbus(name = "incrementMaster")]
    async fn increment_master(&self) -> String {
        self.dispatch_shortcut("increment_nmaster", 1).await
    }

    /// Decrementa el límite óptimo de ventanas activas en la composición foveal.
    #[zbus(name = "decrementMaster")]
    async fn decrement_master(&self) -> String {
        self.dispatch_shortcut("decrement_nmaster", 1).await
    }

    /// Aumenta el ratio de división (split ratio) asimétrica de la espiral BSP.
    #[zbus(name = "increaseRatio")]
    async fn increase_ratio(&self) -> String {
        self.dispatch_shortcut("increase_ratio", 0).await
    }

    /// Disminuye el ratio de división (split ratio) asimétrica de la espiral BSP.
    #[zbus(name = "decreaseRatio")]
    async fn decrease_ratio(&self) -> String {
        self.dispatch_shortcut("decrease_ratio", 0).await
    }

    /// Envía el foco a la ventana siguiente del mosaico.
    #[zbus(name = "focusNext")]
    async fn focus_next(&self) -> String {
        self.dispatch_shortcut("focus_next", 0).await
    }

    /// Retorna la lista de clases en cuarentena configuradas.
    #[zbus(name = "getQuarantineClasses")]
    async fn get_quarantine_classes(&self) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(RavenMessage::GetQuarantineClasses { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        }
    }

    /// Retorna la lista de reglas de ventanas configuradas.
    #[zbus(name = "getWindowRules")]
    async fn get_window_rules(&self) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(RavenMessage::GetWindowRules { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        }
    }

    /// Envía el foco a la ventana anterior del mosaico.
    #[zbus(name = "focusPrev")]
    async fn focus_prev(&self) -> String {
        self.dispatch_shortcut("focus_prev", 0).await
    }

    /// Intercambia la ventana activa con la siguiente en la pila.
    #[zbus(name = "swapNext")]
    async fn swap_next(&self) -> String {
        self.dispatch_shortcut("swap_next", 0).await
    }

    /// Intercambia la ventana activa con la anterior en la pila.
    #[zbus(name = "swapPrev")]
    async fn swap_prev(&self) -> String {
        self.dispatch_shortcut("swap_prev", 0).await
    }

    /// Migra la ventana activa al monitor siguiente.
    #[zbus(name = "migrateActiveToScreen")]
    async fn migrate_active_to_screen(&self) -> String {
        self.dispatch_shortcut("migrate_active_to_screen", 0).await
    }

    /// Migra la ventana activa al monitor anterior.
    #[zbus(name = "migrateActiveToPrevScreen")]
    async fn migrate_active_to_prev_screen(&self) -> String {
        self.dispatch_shortcut("migrate_active_to_prev_screen", 0).await
    }

    /// Migra la ventana activa al escritorio virtual siguiente.
    #[zbus(name = "migrateActiveToDesktop")]
    async fn migrate_active_to_desktop(&self) -> String {
        self.dispatch_shortcut("migrate_active_to_desktop", 0).await
    }

    /// Migra la ventana activa al escritorio virtual anterior.
    #[zbus(name = "migrateActiveToPrevDesktop")]
    async fn migrate_active_to_prev_desktop(&self) -> String {
        self.dispatch_shortcut("migrate_active_to_prev_desktop", 0).await
    }

    /// Cicla al siguiente layout (estrategia de tiling).
    #[zbus(name = "cycleLayout")]
    async fn cycle_layout(&self) -> String {
        self.dispatch_shortcut("cycle_layout", 0).await
    }

    /// Asigna directamente el algoritmo de mosaico para el área de trabajo activa.
    #[zbus(name = "setLayoutForCurrentWorkspace")]
    async fn set_layout_for_current_workspace(&self, layout_name: String) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = RavenMessage::SetLayoutForCurrentWorkspace {
            layout_name,
            reply: reply_tx,
        };
        if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        }
    }

    #[zbus(name = "getTilingState")]
    async fn get_tiling_state(&self) -> bool {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(RavenMessage::GetTilingState { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or(true)
        } else {
            true
        }
    }

    /// Retorna el número actual de monitores o salidas físicas activas.
    #[zbus(name = "getMonitorCount")]
    async fn get_monitor_count(&self) -> i32 {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(RavenMessage::GetMonitorCount { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or(1)
        } else {
            1
        }
    }

    /// Retorna el estado formateado de los escritorios virtuales.
    #[zbus(name = "getDesktopStatus")]
    async fn get_desktop_status(&self) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(RavenMessage::GetDesktopStatus { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("1 | Escritorio 1 | 1"))
        } else {
            String::from("1 | Escritorio 1 | 1")
        }
    }
}

impl RavenDBusService {
    /// Despacha de forma asíncrona una acción de atajo de teclado, recalculando el layout si es necesario.
    async fn dispatch_shortcut(&self, action: &str, payload: i32) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = RavenMessage::DispatchShortcut {
            action: action.to_string(),
            payload,
            reply: reply_tx,
        };

        if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_payload_empty() {
        let (workspaces, windows, topology) = parse_payload("{}").unwrap();
        assert!(workspaces.is_empty());
        assert!(windows.is_empty());
        assert!(topology.outputs.is_empty());
        assert!(topology.desktops.is_empty());
    }

    #[test]
    fn test_parse_payload_valid() {
        let json = r#"{
            "screens": {
                "ws1": {"x": 0, "y": 0, "w": 1920, "h": 1080}
            },
            "windows": [
                {
                    "id": "win1",
                    "ws": "ws1",
                    "output": "DP-1",
                    "desktops": ["desktop-1"],
                    "f": false,
                    "m": false,
                    "p": false,
                    "x": 10,
                    "y": 10,
                    "w": 500,
                    "h": 400,
                    "min_w": 0,
                    "min_h": 0,
                    "sb": false
                }
            ],
            "topology": {
                "outputs": ["DP-1"],
                "desktops": ["desktop-1"]
            }
        }"#;

        let (workspaces, windows, topology) = parse_payload(json).unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces.get("ws1").unwrap().width, 1920);

        assert_eq!(windows.len(), 1);
        let win = &windows[0];
        assert_eq!(win.window_id, "win1");
        assert_eq!(win.geometry.width, 500);

        assert_eq!(topology.outputs.len(), 1);
        assert_eq!(topology.outputs[0], "DP-1");
    }
}
