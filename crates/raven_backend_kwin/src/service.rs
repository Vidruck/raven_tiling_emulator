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

/// Mensajes enviados desde el servicio D-Bus hacia el actor del motor.
pub enum KWinBridgeMessage {
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

/// Servicio D-Bus de Raven para interactuar con KWin y Plasma 6.
pub struct KWinDbusService {
    pub tx: mpsc::Sender<KWinBridgeMessage>,
}

impl KWinDbusService {
    async fn dispatch_shortcut(&self, signal_ctxt: &zbus::object_server::SignalContext<'_>, action: &str, payload: i32) -> String {
        self.dispatch_shortcut_with_str(signal_ctxt, action, payload, None).await
    }

    async fn dispatch_shortcut_with_str(
        &self,
        signal_ctxt: &zbus::object_server::SignalContext<'_>,
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

        let response = if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        };

        if response != "[]" {
            let _ = Self::tiling_commands_pending(signal_ctxt, &response).await;
        }

        response
    }
}

#[interface(name = "org.kde.raven.Events")]
impl KWinDbusService {
    /// Señal emitida a KWin cuando se generan comandos de mosaico asíncronos.
    #[zbus(signal)]
    pub async fn tiling_commands_pending(signal_ctxt: &zbus::object_server::SignalContext<'_>, commands_json: &str) -> zbus::Result<()>;

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
        let msg = KWinBridgeMessage::SyncWindowDelta {
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
        let _ = self.tx.send(KWinBridgeMessage::BridgeReady).await;
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

    /// Alterna el modo flotante temporal (Quick Peek) para la ventana activa o la especificada.
    #[zbus(name = "toggleFloating")]
    async fn toggle_floating(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>, window_id: String) -> String {
        let wid = if window_id.trim().is_empty() {
            None
        } else {
            Some(window_id)
        };
        self.dispatch_shortcut_with_str(&signal_ctxt, "toggle_floating", 0, wid).await
    }

    /// Alterna el estado operativo de activación del motor de mosaico.
    #[zbus(name = "toggleTiling")]
    async fn toggle_tiling(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "toggle_tiling", 0).await
    }

    /// Incrementa o decrementa la separación (gaps) entre las ventanas.
    #[zbus(name = "incrementGaps")]
    async fn increment_gaps(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>, amount: i32) -> String {
        self.dispatch_shortcut(&signal_ctxt, "increment_gaps", amount).await
    }

    /// Incrementa el límite óptimo de ventanas activas en la composición foveal.
    #[zbus(name = "incrementMaster")]
    async fn increment_master(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "increment_nmaster", 1).await
    }

    /// Decrementa el límite óptimo de ventanas activas en la composición foveal.
    #[zbus(name = "decrementMaster")]
    async fn decrement_master(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "decrement_nmaster", 1).await
    }

    /// Aumenta el ratio de división (split ratio) asimétrica de la espiral BSP.
    #[zbus(name = "increaseRatio")]
    async fn increase_ratio(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "increase_ratio", 0).await
    }

    /// Disminuye el ratio de división (split ratio) asimétrica de la espiral BSP.
    #[zbus(name = "decreaseRatio")]
    async fn decrease_ratio(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "decrease_ratio", 0).await
    }

    /// Envía el foco a la ventana siguiente del mosaico.
    #[zbus(name = "focusNext")]
    async fn focus_next(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "focus_next", 0).await
    }

    /// Envía el foco a la ventana anterior del mosaico.
    #[zbus(name = "focusPrev")]
    async fn focus_prev(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "focus_prev", 0).await
    }

    /// Envía el foco a la ventana a la izquierda en la topología.
    #[zbus(name = "focusLeft")]
    async fn focus_left(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "focus_left", 0).await
    }

    /// Envía el foco a la ventana a la derecha en la topología.
    #[zbus(name = "focusRight")]
    async fn focus_right(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "focus_right", 0).await
    }

    /// Envía el foco a la ventana superior en la topología.
    #[zbus(name = "focusUp")]
    async fn focus_up(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "focus_up", 0).await
    }

    /// Envía el foco a la ventana inferior en la topología.
    #[zbus(name = "focusDown")]
    async fn focus_down(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "focus_down", 0).await
    }

    /// Intercambia la ventana activa con la siguiente en la pila.
    #[zbus(name = "swapNext")]
    async fn swap_next(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "swap_next", 0).await
    }

    /// Intercambia la ventana activa con la anterior en la pila.
    #[zbus(name = "swapPrev")]
    async fn swap_prev(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "swap_prev", 0).await
    }

    /// Migra la ventana activa al monitor siguiente.
    #[zbus(name = "migrateActiveToScreen")]
    async fn migrate_active_to_screen(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "migrate_active_to_screen", 0).await
    }

    /// Migra la ventana activa al monitor anterior.
    #[zbus(name = "migrateActiveToPrevScreen")]
    async fn migrate_active_to_prev_screen(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "migrate_active_to_prev_screen", 0).await
    }

    /// Migra la ventana activa al escritorio virtual siguiente.
    #[zbus(name = "migrateActiveToDesktop")]
    async fn migrate_active_to_desktop(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "migrate_active_to_desktop", 0).await
    }

    /// Migra la ventana activa al escritorio virtual anterior.
    #[zbus(name = "migrateActiveToPrevDesktop")]
    async fn migrate_active_to_prev_desktop(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "migrate_active_to_prev_desktop", 0).await
    }

    /// Cicla al siguiente layout (estrategia de tiling).
    #[zbus(name = "cycleLayout")]
    async fn cycle_layout(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>) -> String {
        self.dispatch_shortcut(&signal_ctxt, "cycle_layout", 0).await
    }

    /// Asigna directamente el algoritmo de mosaico para el área de trabajo activa.
    #[zbus(name = "setLayoutForCurrentWorkspace")]
    async fn set_layout_for_current_workspace(&self, #[zbus(signal_context)] signal_ctxt: zbus::object_server::SignalContext<'_>, layout_name: String) -> String {
        let (reply_tx, reply_rx) = oneshot::channel();
        let msg = KWinBridgeMessage::SetLayoutForCurrentWorkspace {
            layout_name,
            reply: reply_tx,
        };
        let response = if self.tx.send(msg).await.is_ok() {
            reply_rx.await.unwrap_or_else(|_| String::from("[]"))
        } else {
            String::from("[]")
        };

        if response != "[]" {
            let _ = Self::tiling_commands_pending(&signal_ctxt, &response).await;
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
