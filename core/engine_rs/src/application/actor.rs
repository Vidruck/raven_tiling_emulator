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
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::info;

use crate::application::controller::RavenController;
use crate::application::quarantine::QuarantineManager;
use crate::domain::geometry::{Rect, Topology, WindowNode};
use raven_backend_kwin::{parse_payload, service::KWinBridgeMessage, KWinWindow};
use raven_core::backend::{CompositorBackend, CompositorEvent};
use raven_core::ports::NotificationPort;

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
    backend: Arc<dyn CompositorBackend>,
    notifier: Arc<dyn NotificationPort>,
    quarantine_manager: QuarantineManager,
    rx: mpsc::Receiver<RavenMessage>,
}

impl RavenControllerActor {
    pub fn new(
        controller: RavenController,
        backend: Arc<dyn CompositorBackend>,
        notifier: Arc<dyn NotificationPort>,
        rx: mpsc::Receiver<RavenMessage>,
        tx: mpsc::Sender<RavenMessage>,
    ) -> Self {
        let quarantine_manager = QuarantineManager::new(tx);
        Self {
            controller,
            active_window_id: None,
            last_payload_json: String::from("{}"),
            current_topology: Topology::default(),
            backend,
            notifier,
            quarantine_manager,
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
                            engine
                                .dynamic_floating_windows
                                .retain(|id| id != &window_id);
                            engine
                                .maximized_windows
                                .retain(|id| id != &window_id);
                            let _ = self.controller.commit_layout();
                        }
                        CompositorEvent::WindowStateChanged {
                            window_id,
                            is_minimized,
                            is_fullscreen,
                            is_floating,
                            geometry,
                        } => {
                            if let Some(win) = self
                                .controller
                                .get_engine_mut()
                                .current_windows
                                .get_mut(&window_id)
                            {
                                if let Some(m) = is_minimized {
                                    win.is_minimized = m;
                                }
                                if let Some(f) = is_fullscreen {
                                    win.is_fullscreen = f;
                                }
                                if let Some(fl) = is_floating {
                                    win.is_floating = fl;
                                }
                                if let Some(geom) = geometry {
                                    win.geometry = geom;
                                }
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
                            info!("[WAYLAND] Topología actualizada recibida directamente de Wayland: {} salidas descubiertas", topology.output_nodes.len());
                            let engine = self.controller.get_engine_mut();
                            for node in &topology.output_nodes {
                                // Para cada pantalla detectada por Wayland, mapear o actualizar su geometría nativa
                                let ws_prefix = format!("{}||", node.name);
                                for (ws_id, rect) in engine.current_workspaces.iter_mut() {
                                    if ws_id.starts_with(&ws_prefix) {
                                        *rect = node.rect;
                                    }
                                }
                                // Si no existía para default, asegurarla
                                let default_ws = format!("{}||default", node.name);
                                engine
                                    .current_workspaces
                                    .entry(default_ws)
                                    .or_insert(node.rect);
                            }
                            self.current_topology = topology;
                            let _ = self.controller.commit_layout();
                        }
                        CompositorEvent::ReleaseQuarantine(window_id) => {
                            // Extraemos resource_class + resource_name y modificamos banderas en un
                            // bloque de scope limitado para liberar el borrow mutable antes de llamar
                            // a métodos que también requieren `&mut self.controller`.
                            let class_info_opt = {
                                if let Some(win) = self.controller.get_engine_mut().current_windows.get_mut(&window_id) {
                                    win.is_quarantined = false;
                                    win.strict_birth = false;
                                    let is_min = win.is_minimized;
                                    Some((win.resource_class.clone(), win.resource_name.clone(), is_min))
                                } else {
                                    None
                                }
                            };

                            if let Some((resource_class, resource_name, is_min)) = class_info_opt {
                                self.controller.clear_window_flapping(&window_id);
                                info!("[ACTOR] Cuarentena liberada (Fase 1) para '{}' [cls='{}' exe='{}']. Agendando rectificación (Fase 2).",
                                    window_id, resource_class, resource_name);
                                let mut commands = Vec::new();
                                // Solo emitir comando de liberación / animación de nacimiento si la ventana NO está minimizada
                                if !is_min {
                                    commands.push(raven_core::action::RavenAction::ReleaseQuarantine {
                                        window_id: window_id.clone(),
                                    });
                                }
                                if let Ok(cmds) = self.controller.commit_layout() {
                                    commands.extend(cmds);
                                }
                                if !commands.is_empty() {
                                    let backend = self.backend.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = backend.apply_actions(commands).await {
                                            tracing::error!("[ACTOR] Error al aplicar acciones de liberación de cuarentena: {}", e);
                                        }
                                    });
                                }
                                // --- Fase 2: Agendar verificación de rectificación solo si no está minimizada ---
                                if !is_min {
                                    self.quarantine_manager
                                        .schedule_rectification(window_id.clone(), &resource_class, &resource_name)
                                        .await;
                                }
                            }
                        }
                        CompositorEvent::RectifyWindow(window_id) => {
                            // --- Modelo de Sospecha Activa — Rectificación Forzada (Fase 2) ---
                            //
                            // Rust NO confía en la geometría reportada por el bridge KWin.
                            // La fuente de verdad son los árboles internos de Rust (last_known_layout
                            // y current_windows). Se determinan dos niveles de sospecha:
                            //
                            // [ALTA SOSPECHA] Ventana marcada por KWin como sospechosa (is_suspicious=true):
                            //   flood de señales durante Timer-0 o sin clase WM.
                            //   → Rectificación incondicional: re-calcular layout completo y
                            //     re-enviar TODOS los comandos de ese workspace como RectifyWindow.
                            //
                            // [SOSPECHA NORMAL] Ventana con target registrado en last_known_layout:
                            //   → Re-enviar directamente el target calculado sin comparar con
                            //     win.geometry del bridge (fuente no confiable post-CSD).

                            let win_opt = self
                                .controller
                                .get_engine()
                                .current_windows
                                .get(&window_id)
                                .cloned();

                            match win_opt {
                                None => {
                                    info!("[SOSPECHA-ACTIVA] Ventana '{}' ya no existe. Rectificación omitida.", window_id);
                                }
                                Some(win) if win.is_minimized => {
                                    info!("[SOSPECHA-ACTIVA] Ventana '{}' está minimizada. Rectificación omitida.", window_id);
                                }
                                Some(win) => {
                                    let is_suspicious = win.is_suspicious || win.resource_class.is_empty();
                                    if is_suspicious {
                                        // ALTA SOSPECHA: re-calcular layout completo como RectifyWindow
                                        tracing::warn!(
                                            "[SOSPECHA-ACTIVA] Ventana SOSPECHOSA '{}' → rectificación incondicional de workspace completo.",
                                            window_id
                                        );
                                        if let Ok(rectify_cmds) = self.controller.commit_layout_as_rectify(None) {
                                            if !rectify_cmds.is_empty() {
                                                let backend = self.backend.clone();
                                                tokio::spawn(async move {
                                                    if let Err(e) = backend.apply_actions(rectify_cmds).await {
                                                        tracing::error!("[SOSPECHA-ACTIVA] Error en rectificación de workspace: {}", e);
                                                    }
                                                });
                                            }
                                        }
                                    } else {
                                        // SOSPECHA NORMAL: re-enviar target desde last_known_layout
                                        // sin comparar contra win.geometry del bridge.
                                        if let Ok(rectify_cmds) = self.controller.commit_layout_as_rectify(Some(&window_id)) {
                                            if !rectify_cmds.is_empty() {
                                                let wid_log = window_id.clone();
                                                info!(
                                                    "[SOSPECHA-ACTIVA] Re-enviando geometría calculada para '{}' (sospecha normal, {} cmds).",
                                                    wid_log, rectify_cmds.len()
                                                );
                                                let backend = self.backend.clone();
                                                tokio::spawn(async move {
                                                    if let Err(e) = backend.apply_actions(rectify_cmds).await {
                                                        tracing::error!("[SOSPECHA-ACTIVA] Error re-enviando para '{}': {}", wid_log, e);
                                                    }
                                                });
                                            } else {
                                                info!("[SOSPECHA-ACTIVA] Ventana '{}' no necesita rectificación (no en layout activo).", window_id);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                RavenMessage::KWinBridge(kwin_msg) => match kwin_msg {
                    KWinBridgeMessage::SyncState {
                        payload_json,
                        reply,
                    } => {
                        self.last_payload_json = payload_json.clone();
                        let (mut workspaces, mut windows, topology) = parse_payload(&payload_json)
                            .unwrap_or_else(|_| (HashMap::new(), Vec::new(), Topology::default()));

                        // Si KWin no envía geometrías de pantalla o vienen incompletas,
                        // las complementamos automáticamente con la topología autoritativa de Wayland
                        if !self.current_topology.output_nodes.is_empty() {
                            let desks = if !topology.desktops.is_empty() {
                                topology.desktops.clone()
                            } else {
                                vec!["default_desk".to_string()]
                            };

                            for node in &self.current_topology.output_nodes {
                                for desk in &desks {
                                    let ws_id = format!("{}||{}", node.name, desk);
                                    workspaces.entry(ws_id).or_insert(node.rect);
                                }
                            }
                        }

                        // Preservar la bandera de flotación dinámica y maximización si Rust o KWin las gestionan
                        for win in &mut windows {
                            if win.is_maximized {
                                self.controller.get_engine_mut().maximized_windows.insert(win.window_id.clone());
                            }

                            if self
                                .controller
                                .get_engine()
                                .maximized_windows
                                .contains(&win.window_id)
                            {
                                win.is_maximized = true;
                                win.is_floating = true;
                            }

                            if self
                                .controller
                                .get_engine()
                                .dynamic_floating_windows
                                .contains(&win.window_id)
                            {
                                win.is_floating = true;
                            }
                            
                            // Blindaje de Idempotencia: Solo agendar cuarentena si la ventana NO existe
                            // en el registro activo de Rust o si Rust también la tiene en cuarentena.
                            // Si Rust ya la liberó previamente (is_quarantined=false y strict_birth=false),
                            // ignorar la bandera residual enviada por KWin para no disparar animaciones cíclicas.
                            let already_released = self
                                .controller
                                .get_engine()
                                .current_windows
                                .get(&win.window_id)
                                .map(|known| !known.is_quarantined && !known.strict_birth)
                                .unwrap_or(false);

                            if !already_released && (win.is_quarantined || win.strict_birth) {
                                self.quarantine_manager
                                    .schedule_release(win.window_id.clone(), &win.resource_class, &win.resource_name)
                                    .await;
                            } else if already_released {
                                // Asegurar que el nodo entrante no reactive banderas obsoletas
                                win.is_quarantined = false;
                                win.strict_birth = false;
                            }
                        }

                        if !topology.outputs.is_empty() || !topology.desktops.is_empty() {
                            self.current_topology.outputs = topology.outputs;
                            self.current_topology.desktops = topology.desktops;
                            self.current_topology.current_desktop = topology.current_desktop;
                        }
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
                                let out_name = if !win.output.is_empty() {
                                    win.output.as_str()
                                } else {
                                    "default"
                                };
                                let desk_name = win
                                    .desktops
                                    .first()
                                    .map(|d| d.as_str())
                                    .unwrap_or("default_desk");
                                format!("{}||{}", out_name, desk_name)
                            };

                            let is_max = win.max || self.controller.get_engine().maximized_windows.contains(&win.id);
                            if win.max {
                                self.controller.get_engine_mut().maximized_windows.insert(win.id.clone());
                            } else if !win.max && !win.f {
                                self.controller.get_engine_mut().maximized_windows.remove(&win.id);
                            }

                            let is_dynamic_float = self
                                .controller
                                .get_engine()
                                .dynamic_floating_windows
                                .contains(&win.id);
                            let mut win_node = WindowNode::new(
                                win.id,
                                ws_id,
                                win.output,
                                win.desktops,
                                win.f || is_dynamic_float || is_max,
                                win.m,
                                win.p,
                                Rect::new(win.x, win.y, win.w, win.h),
                                win.min_w,
                                win.min_h,
                                win.sb,
                                win.iq,
                                win.fs,
                            )
                            .with_maximized(is_max)
                            .with_class_and_caption(win.cls, win.cls_name, win.sus, win.cap);

                            let already_released = self
                                .controller
                                .get_engine()
                                .current_windows
                                .get(&win_node.window_id)
                                .map(|known| !known.is_quarantined && !known.strict_birth)
                                .unwrap_or(false);

                            if !already_released && (win_node.is_quarantined || win_node.strict_birth) {
                                self.quarantine_manager
                                    .schedule_release(win_node.window_id.clone(), &win_node.resource_class, &win_node.resource_name)
                                    .await;
                            } else if already_released {
                                win_node.is_quarantined = false;
                                win_node.strict_birth = false;
                            }

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
                    KWinBridgeMessage::DispatchShortcut {
                        action,
                        payload,
                        payload_str,
                        reply,
                    } => {
                        let effective_active_id = payload_str
                            .filter(|s| !s.trim().is_empty())
                            .or_else(|| self.active_window_id.clone());

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
                        let res =
                            serde_json::to_string(&self.controller.get_config().quarantine_classes)
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
                            let current_idx = topo
                                .desktops
                                .iter()
                                .position(|d| d == &topo.current_desktop)
                                .unwrap_or(0);
                            let prev_idx = if current_idx == 0 {
                                total - 1
                            } else {
                                current_idx - 1
                            };
                            let next_idx = (current_idx + 1) % total;
                            format!(
                                "{} | Escritorio {} | {}",
                                prev_idx + 1,
                                current_idx + 1,
                                next_idx + 1
                            )
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
                        let current_ws = self
                            .active_window_id
                            .as_ref()
                            .and_then(|wid| {
                                self.controller
                                    .get_engine()
                                    .current_windows
                                    .get(wid)
                                    .map(|w| w.workspace_id.clone())
                            })
                            .or_else(|| {
                                self.controller
                                    .get_engine()
                                    .current_windows
                                    .values()
                                    .next()
                                    .map(|w| w.workspace_id.clone())
                            })
                            .or_else(|| {
                                let out = self
                                    .current_topology
                                    .outputs
                                    .first()
                                    .map(|s| s.as_str())
                                    .unwrap_or("default");
                                let desk = if !self.current_topology.current_desktop.is_empty() {
                                    self.current_topology.current_desktop.as_str()
                                } else {
                                    "default_desk"
                                };
                                Some(format!("{}||{}", out, desk))
                            });

                        self.controller.get_engine_mut().config.layout_type = layout_name.clone();
                        if let Some(ws_id) = current_ws {
                            self.controller
                                .get_engine_mut()
                                .config
                                .workspace_layouts
                                .insert(ws_id, layout_name.clone());
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
                        self.notifier.notify_osd(
                            "Disposición de Ventanas",
                            &format!("Layout: {}", readable_name),
                        );

                        let mut commands = Vec::new();
                        if let Ok(cmds) = self.controller.commit_layout() {
                            commands = cmds;
                        }
                        let _ = reply.send(commands);
                    }
                    KWinBridgeMessage::CommandAppliedState { window_id, x, y, width, height } => {
                        // Auditoría de geometría aplicada: comparar con target calculado
                        if let Some(target) = self.controller.get_target_rect_for_window(&window_id) {
                            let matches = (target.x as i32 - x).abs() <= 2
                                && (target.y as i32 - y).abs() <= 2
                                && (target.width as i32 - width).abs() <= 2
                                && (target.height as i32 - height).abs() <= 2;

                            if !matches {
                                tracing::info!(
                                    "[AUDIT] Geometría divergente en '{}' (Esperado: {}x{}+{}+{}, Aplicado: {}x{}+{}+{}). Rectificando...",
                                    window_id, target.width, target.height, target.x, target.y, width, height, x, y
                                );
                                if let Ok(rectify_cmds) = self.controller.commit_layout_as_rectify(Some(&window_id)) {
                                    if !rectify_cmds.is_empty() {
                                        let backend = self.backend.clone();
                                        tokio::spawn(async move {
                                            let _ = backend.apply_actions(rectify_cmds).await;
                                        });
                                    }
                                }
                            } else {
                                tracing::debug!("[AUDIT] Geometría verificada correctamente para '{}'.", window_id);
                            }
                        }
                    }
                },
            }
        }
    }
}
