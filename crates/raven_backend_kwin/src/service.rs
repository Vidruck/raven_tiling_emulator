//! # Servicio D-Bus para KWin (`service`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 4.0  
//! **Licencia:** GPL-3.0  
//!
//! Implementa la interfaz D-Bus `org.kde.raven.Events` que comunica con el script KWin
//! y el plasmoide, enlazando hacia el actor de concurrencia mediante canales Tokio.

use tokio::sync::{mpsc, oneshot};
use zbus::interface;
use zbus::object_server::SignalEmitter;

use raven_core::action::RavenAction;
use crate::commands::TilingCommand;

/// Mensajes enviados desde el servicio D-Bus hacia el actor del motor.
pub enum KWinBridgeMessage {
    SyncState {
        payload_json: String,
        reply: oneshot::Sender<Vec<RavenAction>>,
    },
    SyncWindowDelta {
        delta_json: String,
        reply: oneshot::Sender<Vec<RavenAction>>,
    },
    DispatchShortcut {
        action: String,
        payload: i32,
        payload_str: Option<String>,
        reply: oneshot::Sender<Vec<RavenAction>>,
    },
    BridgeReady,
    WindowClosed {
        window_id: String,
        reply: oneshot::Sender<Vec<RavenAction>>,
    },
    WindowActivated {
        window_id: Option<String>,
    },
    CommandAppliedState {
        window_id: String,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
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
        reply: oneshot::Sender<Vec<RavenAction>>,
    },
}

/// Convierte una lista de acciones de Raven en la representación JSON de `TilingCommand` esperada por KWin.
pub fn actions_to_kwin_json(actions: Vec<RavenAction>) -> String {
    let dbus_commands: Vec<TilingCommand> = actions.into_iter().map(Into::into).collect();
    serde_json::to_string(&dbus_commands).unwrap_or_else(|_| String::from("[]"))
}

/// Servicio D-Bus de Raven para interactuar con KWin y Plasma 6.
pub struct KWinDbusService {
    pub tx: mpsc::Sender<KWinBridgeMessage>,
}

impl KWinDbusService {
    async fn dispatch_shortcut(&self, emitter: &SignalEmitter<'_>, action: &str, payload: i32) -> String {
        self.dispatch_shortcut_with_str(emitter, action, payload, None).await
    }

    async fn dispatch_shortcut_with_str(
        &self,
        emitter: &SignalEmitter<'_>,
        action: &str,
        payload: i32,
        payload_str: Option<String>,
    ) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = KWinBridgeMessage::DispatchShortcut {
            action: action.to_string(),
            payload,
            payload_str,
            reply: reply_tx,
        };

        let actions = if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_default()
        } else {
            Vec::new()
        };

        let response = actions_to_kwin_json(actions);

        if response != "[]" {
            let _ = Self::tiling_commands_pending(emitter, &response).await;
        }

        response
    }
}

#[interface(name = "org.kde.raven.Events")]
impl KWinDbusService {
    /// Señal emitida a KWin cuando se generan comandos de mosaico asíncronos.
    #[zbus(signal)]
    pub async fn tiling_commands_pending(emitter: &SignalEmitter<'_>, commands_json: &str) -> zbus::Result<()>;

    /// Recibe y procesa el estado global de las ventanas, retornando inmediatamente los comandos geométricos.
    #[zbus(name = "syncStateAndUpdateLayout")]
    async fn sync_state_and_update_layout(&self, payload_json: String) -> String {
        if payload_json.len() > 5 * 1024 * 1024 {
            return String::from("[]");
        }

        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = KWinBridgeMessage::SyncState {
            payload_json,
            reply: reply_tx,
        };

        let actions = if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_default()
        } else {
            Vec::new()
        };

        actions_to_kwin_json(actions)
    }

    /// Sincroniza de forma incremental (delta sync) el cambio de geometría o estado de una única ventana.
    #[zbus(name = "syncWindowDelta")]
    async fn sync_window_delta(&self, delta_json: String) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = KWinBridgeMessage::SyncWindowDelta {
            delta_json,
            reply: reply_tx,
        };

        let actions = if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_default()
        } else {
            Vec::new()
        };

        actions_to_kwin_json(actions)
    }

    /// Notifica que el puente de JavaScript se ha restablecido y está listo.
    #[zbus(name = "bridgeReady")]
    async fn bridge_ready(&self) {
        let _ = self.tx.send(KWinBridgeMessage::BridgeReady).await;
    }

    /// Notifica que una ventana ha sido cerrada y destruida en KWin, eliminándola inmediatamente de la memoria del motor.
    #[zbus(name = "windowClosed")]
    async fn window_closed(&self, window_id: String) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = KWinBridgeMessage::WindowClosed {
            window_id,
            reply: reply_tx,
        };

        let actions = if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_default()
        } else {
            Vec::new()
        };

        actions_to_kwin_json(actions)
    }

    /// Registra el identificador de la ventana activa enfocada en KWin.
    #[zbus(name = "windowActivated")]
    async fn window_activated(&self, window_id: String) {
        let val = if window_id.trim().is_empty() {
            None
        } else {
            Some(window_id.clone())
        };
        let _ = self.tx.send(KWinBridgeMessage::WindowActivated { window_id: val }).await;
    }

    /// Reporta la geometría física asumida por una ventana tras un comando de movimiento para auditoría.
    #[zbus(name = "commandAppliedState")]
    async fn command_applied_state(&self, window_id: String, x: i32, y: i32, width: i32, height: i32) {
        let _ = self.tx.send(KWinBridgeMessage::CommandAppliedState {
            window_id,
            x,
            y,
            width,
            height,
        }).await;
    }

    /// Alterna el modo flotante temporal (Quick Peek) para la ventana activa o la especificada.
    #[zbus(name = "toggleFloating")]
    async fn toggle_floating(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, window_id: String) -> String {
        let wid = if window_id.trim().is_empty() {
            None
        } else {
            Some(window_id)
        };
        self.dispatch_shortcut_with_str(&emitter, "toggle_floating", 0, wid).await
    }

    /// Alterna el estado maximizado para la ventana activa o la especificada.
    #[zbus(name = "toggleMaximize")]
    async fn toggle_maximize(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, window_id: String) -> String {
        let wid = if window_id.trim().is_empty() {
            None
        } else {
            Some(window_id)
        };
        self.dispatch_shortcut_with_str(&emitter, "toggle_maximize", 0, wid).await
    }

    /// Minimiza la ventana activa o la especificada.
    #[zbus(name = "minimizeActive")]
    async fn minimize_active(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, window_id: String) -> String {
        let wid = if window_id.trim().is_empty() {
            None
        } else {
            Some(window_id)
        };
        self.dispatch_shortcut_with_str(&emitter, "minimize_active", 0, wid).await
    }

    /// Cierra la ventana activa o la especificada.
    #[zbus(name = "closeActive")]
    async fn close_active(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, window_id: String) -> String {
        let wid = if window_id.trim().is_empty() {
            None
        } else {
            Some(window_id)
        };
        self.dispatch_shortcut_with_str(&emitter, "close_active", 0, wid).await
    }

    /// Alterna el estado operativo de activación del motor de mosaico.
    #[zbus(name = "toggleTiling")]
    async fn toggle_tiling(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "toggle_tiling", 0).await
    }

    /// Incrementa o decrementa la separación (gaps) entre las ventanas.
    #[zbus(name = "incrementGaps")]
    async fn increment_gaps(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, amount: i32) -> String {
        self.dispatch_shortcut(&emitter, "increment_gaps", amount).await
    }

    /// Incrementa el límite óptimo de ventanas activas en la composición foveal.
    #[zbus(name = "incrementMaster")]
    async fn increment_master(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "increment_nmaster", 1).await
    }

    /// Decrementa el límite óptimo de ventanas activas en la composición foveal.
    #[zbus(name = "decrementMaster")]
    async fn decrement_master(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "decrement_nmaster", 1).await
    }

    /// Aumenta el ratio de división (split ratio) asimétrica de la espiral BSP.
    #[zbus(name = "increaseRatio")]
    async fn increase_ratio(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "increase_ratio", 0).await
    }

    /// Disminuye el ratio de división (split ratio) asimétrica de la espiral BSP.
    #[zbus(name = "decreaseRatio")]
    async fn decrease_ratio(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "decrease_ratio", 0).await
    }

    /// Envía el foco a la ventana siguiente del mosaico.
    #[zbus(name = "focusNext")]
    async fn focus_next(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "focus_next", 0).await
    }

    /// Envía el foco a la ventana anterior del mosaico.
    #[zbus(name = "focusPrev")]
    async fn focus_prev(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "focus_prev", 0).await
    }

    /// Envía el foco a la ventana a la izquierda en la topología.
    #[zbus(name = "focusLeft")]
    async fn focus_left(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "focus_left", 0).await
    }

    /// Envía el foco a la ventana a la derecha en la topología.
    #[zbus(name = "focusRight")]
    async fn focus_right(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "focus_right", 0).await
    }

    /// Envía el foco a la ventana superior en la topología.
    #[zbus(name = "focusUp")]
    async fn focus_up(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "focus_up", 0).await
    }

    /// Envía el foco a la ventana inferior en la topología.
    #[zbus(name = "focusDown")]
    async fn focus_down(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "focus_down", 0).await
    }

    /// Intercambia la ventana activa con la siguiente en la pila.
    #[zbus(name = "swapNext")]
    async fn swap_next(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "swap_next", 0).await
    }

    /// Intercambia la ventana activa con la anterior en la pila.
    #[zbus(name = "swapPrev")]
    async fn swap_prev(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "swap_prev", 0).await
    }

    /// Migra la ventana activa al monitor siguiente.
    #[zbus(name = "migrateActiveToScreen")]
    async fn migrate_active_to_screen(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "migrate_active_to_screen", 0).await
    }

    /// Migra la ventana activa al monitor anterior.
    #[zbus(name = "migrateActiveToPrevScreen")]
    async fn migrate_active_to_prev_screen(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "migrate_active_to_prev_screen", 0).await
    }

    /// Migra la ventana activa al escritorio virtual siguiente.
    #[zbus(name = "migrateActiveToDesktop")]
    async fn migrate_active_to_desktop(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "migrate_active_to_desktop", 0).await
    }

    /// Migra la ventana activa al escritorio virtual anterior.
    #[zbus(name = "migrateActiveToPrevDesktop")]
    async fn migrate_active_to_prev_desktop(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "migrate_active_to_prev_desktop", 0).await
    }

    /// Cicla al siguiente layout (estrategia de tiling).
    #[zbus(name = "cycleLayout")]
    async fn cycle_layout(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "cycle_layout", 0).await
    }

    /// Incrementa el ancho de la ventana activa.
    #[zbus(name = "resize_width_inc")]
    async fn resize_width_inc(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "resize_width_inc", 0).await
    }

    /// Reduce el ancho de la ventana activa.
    #[zbus(name = "resize_width_dec")]
    async fn resize_width_dec(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "resize_width_dec", 0).await
    }

    /// Incrementa el alto de la ventana activa.
    #[zbus(name = "resize_height_inc")]
    async fn resize_height_inc(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "resize_height_inc", 0).await
    }

    /// Reduce el alto de la ventana activa.
    #[zbus(name = "resize_height_dec")]
    async fn resize_height_dec(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>) -> String {
        self.dispatch_shortcut(&emitter, "resize_height_dec", 0).await
    }

    /// Asigna directamente el algoritmo de mosaico para el área de trabajo activa.
    #[zbus(name = "setLayoutForCurrentWorkspace")]
    async fn set_layout_for_current_workspace(&self, #[zbus(signal_emitter)] emitter: SignalEmitter<'_>, layout_name: String) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = KWinBridgeMessage::SetLayoutForCurrentWorkspace {
            layout_name,
            reply: reply_tx,
        };
        let actions = if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_default()
        } else {
            Vec::new()
        };

        let response = actions_to_kwin_json(actions);

        if response != "[]" {
            let _ = Self::tiling_commands_pending(&emitter, &response).await;
        }

        response
    }

    #[zbus(name = "getTilingState")]
    async fn get_tiling_state(&self) -> bool {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(KWinBridgeMessage::GetTilingState { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or(true)
        } else {
            true
        }
    }

    /// Retorna el número actual de monitores o salidas físicas activas.
    #[zbus(name = "getMonitorCount")]
    async fn get_monitor_count(&self) -> i32 {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(KWinBridgeMessage::GetMonitorCount { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or(1)
        } else {
            1
        }
    }

    /// Retorna el estado formateado de los escritorios virtuales.
    #[zbus(name = "getDesktopStatus")]
    async fn get_desktop_status(&self) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(KWinBridgeMessage::GetDesktopStatus { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("1 | Escritorio 1 | 1"))
        } else {
            String::from("1 | Escritorio 1 | 1")
        }
    }

    /// Retorna la lista de clases en cuarentena configuradas.
    #[zbus(name = "getQuarantineClasses")]
    async fn get_quarantine_classes(&self) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(KWinBridgeMessage::GetQuarantineClasses { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        }
    }

    /// Retorna la lista de reglas de ventanas configuradas.
    #[zbus(name = "getWindowRules")]
    async fn get_window_rules(&self) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        if self.tx.send(KWinBridgeMessage::GetWindowRules { reply: reply_tx }).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        }
    }
}
