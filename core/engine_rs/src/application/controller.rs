//! # Controlador de Aplicación y Mitigador de Inestabilidad (`RavenController`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Orquesta las mutaciones de estado, transiciones de layout, navegación de foco,
//! intercambio de ventanas (*swap*), ajustes interactivos de márgenes/ratios y detección
//! de bucles infinitos de oscilación geométrica (*FlapTracker*).

use std::collections::HashMap;
use tracing::{info, warn};

use crate::application::engine::TilingEngine;
use crate::domain::action::RavenAction;
use crate::domain::error::RavenError;
use crate::domain::geometry::{Rect, WindowNode};
use crate::domain::saturation::{calculate_screen_capacity, SaturationState};

use crate::domain::flap_guard::FlapGuard;

/// Orquestador principal de la lógica de Raven Hub v3.4 con soporte de pila compartida y mitigación de saturación.
pub struct RavenController {
    /// Motor de mosaico central de la aplicación.
    engine: TilingEngine,
    /// Registro histórico del último diseño (layout) calculado.
    last_known_layout: HashMap<String, Rect>,
    /// Guardia de detección y mitigación de oscilaciones rápidas (flapping).
    flap_guard: FlapGuard,
    /// Identificador de la ventana activa enfocada (focused window).
    pub active_window_id: Option<String>,
    /// Cantidad de ventanas activas en el último cambio de estado.
    last_active_window_count: usize,
    /// Puerto secundario para persistencia del historial de ventanas.
    history_storage: Box<dyn raven_core::ports::HistoryStorage>,
    /// Puerto secundario para notificaciones OSD del sistema.
    notifier: Box<dyn raven_core::ports::NotificationPort>,
}

impl RavenController {
    /// Crea una nueva instancia de `RavenController` con los adaptadores por defecto.
    pub fn new(engine: TilingEngine) -> Self {
        Self::with_ports(
            engine,
            Box::new(crate::infrastructure::adapters::FileHistoryStorage::new()),
            Box::new(crate::infrastructure::adapters::NotifySendNotifier),
        )
    }

    /// Crea una nueva instancia de `RavenController` inyectando puertos secundarios personalizados.
    pub fn with_ports(
        mut engine: TilingEngine,
        history_storage: Box<dyn raven_core::ports::HistoryStorage>,
        notifier: Box<dyn raven_core::ports::NotificationPort>,
    ) -> Self {
        engine.window_history = history_storage.load_history();
        RavenController {
            engine,
            last_known_layout: HashMap::new(),
            flap_guard: FlapGuard::new(),
            active_window_id: None,
            last_active_window_count: 0,
            history_storage,
            notifier,
        }
    }

    /// Retorna una referencia a la configuración actual del motor.
    pub fn get_config(&self) -> &crate::infrastructure::config::RavenConfig {
        &self.engine.config
    }

    /// Restablece todo el estado interno y registros temporales del controlador.
    pub fn reset_state(&mut self) {
        self.last_known_layout.clear();
        self.flap_guard.clear();
        self.engine.current_workspaces.clear();
        self.engine.current_windows.clear();
        self.active_window_id = None;
        self.last_active_window_count = 0;
    }

    /// Despacha una notificación OSD utilizando el puerto de notificación inyectado.
    fn send_osd_notification(&self, title: &str, body: &str) {
        self.notifier.notify_osd(title, body);
    }

    /// Determina si el motor de mosaico (tiling engine) está operativo.
    pub fn is_tiling_enabled(&self) -> bool {
        self.engine.is_tiling_enabled
    }

    pub fn get_engine(&self) -> &crate::application::engine::TilingEngine {
        &self.engine
    }

    pub fn get_engine_mut(&mut self) -> &mut crate::application::engine::TilingEngine {
        &mut self.engine
    }

    /// Comprueba si una ventana está oscilando rápidamente (flapping) y aplica penalizaciones.
    fn is_window_flapping(&mut self, win: &WindowNode) -> bool {
        if win.strict_birth
            || win.is_quarantined
            || self
                .engine
                .dynamic_floating_windows
                .contains(&win.window_id)
        {
            return false; // Las apps en cuarentena, en nacimiento o en Quick Peek tienen pase libre
        }
        self.flap_guard.is_window_flapping(win)
    }

    /// Elimina el registro de oscilación para una ventana (ej. tras estabilización de nacimiento).
    pub fn clear_window_flapping(&mut self, window_id: &str) {
        self.flap_guard.remove(window_id);
    }

    /// Retorna la geometría objetivo calculada por el último layout para una ventana específica.
    ///
    /// Devuelve `None` si la ventana aún no tiene una geometría confirmada en caché
    /// (ej. primera vez que se procesa o si no pasó la verificación de convergencia).
    /// Se utiliza en el modelo de rectificación para comparar la geometría física reportada
    /// por KWin con la geometría objetivo que Rust calculó y comandó previamente.
    pub fn get_target_rect_for_window(&self, window_id: &str) -> Option<Rect> {
        self.last_known_layout.get(window_id).copied()
    }

    /// Procesa una actualización completa de estado del compositor y calcula los nuevos comandos.
    ///
    /// # Parámetros
    /// * `workspaces` - Mapa actualizado de áreas de trabajo (workspaces) y sus dimensiones.
    /// * `windows` - Vector de ventanas (windows) activas con sus propiedades actuales.
    ///
    /// # Retorno
    /// Un vector de acciones `RavenAction` a despachar en el compositor, o error de dominio.
    pub fn handle_state_change(
        &mut self,
        workspaces: HashMap<String, Rect>,
        windows: Vec<WindowNode>,
    ) -> Result<Vec<RavenAction>, RavenError> {
        let mut history_changed = self.engine.update_history(&windows);

        // Si hay una ventana activa especificada, promoverla como MRU en el historial cíclico
        if let Some(ref act_id) = self.active_window_id {
            if !act_id.is_empty()
                && windows
                    .iter()
                    .any(|w| &w.window_id == act_id && !w.is_floating)
                && self.engine.promote_to_recent(act_id)
            {
                history_changed = true;
            }
        }

        if history_changed {
            self.history_storage.save_history(self.engine.window_history.clone());
        }

        let mut healthy_windows = Vec::new();
        for win in windows.into_iter() {
            if !self.is_window_flapping(&win) {
                healthy_windows.push(win);
            }
        }
        let mut windows = healthy_windows;

        // Determinamos la cantidad de ventanas activas (que no floten ni estén minimizadas)
        let active_count = windows
            .iter()
            .filter(|w| !w.is_floating && !w.is_minimized)
            .count();
        // Actualizamos el tracking del número de ventanas activas.
        if active_count != self.last_active_window_count {
            self.last_active_window_count = active_count;
        }

        // Para preservar la estabilidad espacial del layout (evitando swaps visuales al enfocar o interactuar),
        // las ventanas se ordenan por `spatial_order` indexado en O(1) vía HashMap para evitar O(N^2).
        let spatial_indices: HashMap<&str, usize> = self
            .engine
            .spatial_order
            .iter()
            .enumerate()
            .map(|(idx, id)| (id.as_str(), idx))
            .collect();

        windows.sort_by_key(|w| {
            spatial_indices
                .get(w.window_id.as_str())
                .copied()
                .unwrap_or(usize::MAX)
        });

        let (new_layout, evicted_windows) = self.engine.calculate_from_payload(
            &workspaces,
            &windows,
            self.active_window_id.clone(),
        )?;
        let previous_layout = std::mem::take(&mut self.last_known_layout);
        // Almacenar siempre la geometría objetivo calculada por el motor como fuente de verdad
        self.last_known_layout = new_layout.clone();
        let mut commands = Vec::new();

        for (wid, rect) in &new_layout {
            let mut win_rect_differs = false;
            if let Some(win_node) = windows.iter().find(|w| &w.window_id == wid) {
                let dx = (win_node.geometry.x - rect.x).abs();
                let dy = (win_node.geometry.y - rect.y).abs();
                let dw = (win_node.geometry.width - rect.width).abs();
                let dh = (win_node.geometry.height - rect.height).abs();
                
                // Tolerancia adaptativa:
                // Posición <= 4px; Dimensiones <= 12px (admite incrementos de celda de terminales y márgenes CSD)
                if dx > 4 || dy > 4 || dw > 12 || dh > 12 {
                    win_rect_differs = true;
                }
            }

            let is_quarantined_or_strict = windows
                .iter()
                .find(|w| &w.window_id == wid)
                .map(|w| w.is_quarantined || w.strict_birth)
                .unwrap_or(false);

            let needs_move = match previous_layout.get(wid) {
                Some(old_rect) => old_rect != rect || win_rect_differs || is_quarantined_or_strict,
                None => true,
            };

            if needs_move {
                commands.push(RavenAction::MoveWindow {
                    window_id: wid.clone(),
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                });
            }
        }

        for evicted_id in &evicted_windows {
            if let Some(win_node) = windows.iter().find(|w| &w.window_id == evicted_id) {
                let mut outputs = Vec::new();
                for key in workspaces.keys() {
                    if let Some(out) = key.split("||").next() {
                        let out_str = out.to_string();
                        if !outputs.contains(&out_str) {
                            outputs.push(out_str);
                        }
                    }
                }

                if outputs.len() > 1 && outputs.iter().any(|o| o != &win_node.output) {
                    if let Some(target_out) = outputs.iter().find(|&o| o != &win_node.output) {
                        info!(
                            "[TOPOLOGY] Desalojo BSP: Enviando {} al monitor {}",
                            evicted_id, target_out
                        );
                        commands.push(RavenAction::MigrateToOutput {
                            window_id: evicted_id.clone(),
                            target_output: target_out.clone(),
                        });
                        continue;
                    }
                }

                info!(
                    "[TOPOLOGY] Desalojo BSP sin escape para {}. Minimizando.",
                    evicted_id
                );
                commands.push(RavenAction::MinimizeWindow {
                    window_id: evicted_id.clone(),
                });
            }
        }

        // --- Motor de Composición Predictiva ---
        // Calcular el estado de saturación por cada workspace activo
        for (ws_id, ws_rect) in &workspaces {
            let ws_active = windows
                .iter()
                .filter(|w| !w.is_floating && !w.is_minimized && w.workspace_id == *ws_id)
                .count();

            if ws_active == 0 {
                continue;
            }

            let cap = calculate_screen_capacity(
                ws_rect.width,
                ws_rect.height,
                self.engine.config.default_gaps,
                300, // min_w funcional
                250, // min_h funcional
                ws_active,
            );

            match cap.state {
                SaturationState::PreSaturation | SaturationState::Saturated => {
                    commands.push(RavenAction::SaturationWarning {
                        cmax: cap.cmax,
                        active: ws_active,
                    });
                }
                SaturationState::Overloaded => {
                    // Ya manejado por evicted_windows arriba
                    tracing::warn!(
                        "[SATURACIÓN] Pantalla {} sobrecargada: {} ventanas / Cmax={}",
                        ws_id,
                        ws_active,
                        cap.cmax
                    );
                }
                SaturationState::Fluid => {}
            }
        }

        self.engine.current_workspaces = workspaces.clone();

        // ── Fusión defensiva de current_windows ──────────────────────────────────────
        // NO reemplazamos completamente current_windows a ciegas si el payload viene vacío de workspaces.
        // Pero para los workspaces reportados (definidos por el mapa `workspaces` y los workspaces de las ventanas),
        // eliminamos inmediatamente las ventanas que ya no existan en KWin.
        {
            let payload_window_ids: std::collections::HashSet<String> =
                windows.iter().map(|w| w.window_id.clone()).collect();
            let mut payload_workspace_ids: std::collections::HashSet<String> =
                workspaces.keys().cloned().collect();
            for w in &windows {
                payload_workspace_ids.insert(w.workspace_id.clone());
            }

            // Eliminar solo las ventanas que pertenecen a workspaces reportados y ya no están
            self.engine.current_windows.retain(|id, existing_win| {
                if payload_workspace_ids.contains(&existing_win.workspace_id) {
                    let keep = payload_window_ids.contains(id);
                    if !keep {
                        self.flap_guard.remove(id);
                    }
                    return keep;
                }
                true
            });

            // Insertar/actualizar todas las ventanas del payload preservando ratios personalizados previos
            for mut win in windows {
                if let Some(existing) = self.engine.current_windows.get(&win.window_id) {
                    if win.custom_w_ratio.is_none() {
                        win.custom_w_ratio = existing.custom_w_ratio;
                    }
                    if win.custom_h_ratio.is_none() {
                        win.custom_h_ratio = existing.custom_h_ratio;
                    }
                }
                self.engine.current_windows.insert(win.window_id.clone(), win);
            }
        }

        Ok(commands)
    }

    /// Procesa de forma incremental la actualización de estado de una sola ventana.
    ///
    /// # Parámetros
    /// * `win` - Nodo de ventana con los cambios recientes.
    ///
    /// Solo actualiza el estado interno (diccionario). Para generar comandos y redibujar, debe llamarse explícitamente a `commit_layout()`.
    pub fn handle_delta_change(&mut self, mut win: WindowNode) {
        if let Some(existing) = self.engine.current_windows.get(&win.window_id) {
            if win.custom_w_ratio.is_none() {
                win.custom_w_ratio = existing.custom_w_ratio;
            }
            if win.custom_h_ratio.is_none() {
                win.custom_h_ratio = existing.custom_h_ratio;
            }
        }
        self.engine
            .current_windows
            .insert(win.window_id.clone(), win);
    }

    /// Compromete el estado de las ventanas actuales y calcula el diseño (layout) geométrico definitivo.
    ///
    /// # Retorno
    /// Vector de comandos resultantes de evaluar el cambio en el motor.
    pub fn commit_layout(&mut self) -> Result<Vec<RavenAction>, RavenError> {
        let workspaces = self.engine.current_workspaces.clone();
        let windows: Vec<WindowNode> = self.engine.current_windows.values().cloned().collect();
        self.handle_state_change(workspaces, windows)
    }

    /// Modelo de **Sospecha Activa** — Rectificación Forzada (Fase 2 del Timer de Cuarentena).
    ///
    /// Re-calcula el layout completo desde los árboles internos de Rust y transforma todos los
    /// comandos `MoveWindow` en `RectifyWindow`. La diferencia clave con `commit_layout()` es que:
    ///
    /// - **No confía en la geometría reportada por el bridge KWin** (`win.geometry`).
    ///   La fuente de verdad son los árboles calculados internamente por Rust.
    /// - **Re-envía el layout incondicionalmente** para todas las ventanas tileadas.
    ///   No compara contra la geometría actual del compositor.
    /// - Los comandos `RectifyWindow` **bypass** la guardia anti-redundancia de KWin.
    ///
    /// Esto resuelve el caso donde un navegador (Zen, Firefox) o app CSD restaura su sesión
    /// anterior **después** de que Rust ya envió la posición correcta.
    ///
    /// # Parámetros
    /// * `target_window_id` – Si `Some`, solo rectifica esa ventana. Si `None`, rectifica todas
    ///   las ventanas tileadas del workspace activo.
    ///
    /// # Retorno
    /// Vector de comandos `RectifyWindow` a aplicar inmediatamente.
    pub fn commit_layout_as_rectify(
        &mut self,
        target_window_id: Option<&str>,
    ) -> Result<Vec<RavenAction>, RavenError> {
        // Re-calcular el layout desde los árboles internos (fuente de verdad Rust)
        let workspaces = self.engine.current_workspaces.clone();
        let windows: Vec<WindowNode> = self.engine.current_windows.values().cloned().collect();

        // Invocar handle_state_change que calcula el layout y actualiza last_known_layout
        let layout_cmds = self.handle_state_change(workspaces, windows)?;

        // Transformar MoveWindow → RectifyWindow, filtrando por target si se especifica
        let mut rectify_cmds: Vec<RavenAction> = layout_cmds
            .into_iter()
            .filter_map(|cmd| match cmd {
                RavenAction::MoveWindow { ref window_id, x, y, width, height } => {
                    // Si target_window_id es Some, filtrar solo esa ventana
                    if let Some(target) = target_window_id {
                        if window_id.as_str() != target {
                            return None;
                        }
                    }
                    Some(RavenAction::RectifyWindow {
                        window_id: window_id.clone(),
                        x,
                        y,
                        width,
                        height,
                    })
                }
                // Otros comandos (SetFloating, Focus, etc.) se preservan tal cual
                other => {
                    if target_window_id.is_none() {
                        Some(other)
                    } else {
                        None
                    }
                }
            })
            .collect();

        // Si se especificó un target_window_id y handle_state_change no emitió MoveWindow
        // (por ejemplo, porque el rect en last_known_layout coincidía con la última corrida previa),
        // recuperamos el target rect de last_known_layout para garantizar la emisión de RectifyWindow
        // únicamente para la ventana afectada sin tocar el resto.
        if let Some(target) = target_window_id {
            let has_target_rectify = rectify_cmds.iter().any(|cmd| match cmd {
                RavenAction::RectifyWindow { window_id, .. } => window_id.as_str() == target,
                _ => false,
            });

            if !has_target_rectify {
                if let Some(target_rect) = self.last_known_layout.get(target).copied() {
                    rectify_cmds.push(RavenAction::RectifyWindow {
                        window_id: target.to_string(),
                        x: target_rect.x,
                        y: target_rect.y,
                        width: target_rect.width,
                        height: target_rect.height,
                    });
                }
            }
        }

        Ok(rectify_cmds)
    }

    /// Maneja las solicitudes de atajos de teclado (shortcuts) invocados desde la UI o el compositor.
    ///
    /// Permite alterar el estado operativo del motor, los gaps, cambiar el foco o migrar ventanas.
    ///
    /// # Parámetros
    /// * `action` - Identificador textual del atajo a ejecutar.
    /// * `payload` - Entero con argumento opcional de peso (p. ej., valor delta de gaps).
    pub fn handle_shortcut(
        &mut self,
        action: String,
        _payload: i32,
        active_window_id: Option<String>,
        topology: &crate::domain::geometry::Topology,
    ) -> Result<(bool, Vec<RavenAction>), RavenError> {
        self.active_window_id = active_window_id.clone();

        let windows: Vec<WindowNode> = self.engine.current_windows.values().cloned().collect();
        self.engine.update_history(&windows);
        let mut needs_recalc = false;
        let mut config_changed = false;
        let mut commands = Vec::new();

        match action.as_str() {
            // Alternar estado maximizado
            "toggle_maximize" => {
                let target_wid = self.active_window_id.clone().or_else(|| {
                    self.engine.window_history.back().cloned().or_else(|| {
                        windows
                            .iter()
                            .find(|w| !w.is_minimized)
                            .map(|w| w.window_id.clone())
                    })
                });

                if let Some(wid) = target_wid {
                    self.flap_guard.remove(&wid);
                    self.last_known_layout.remove(&wid);
                    if self.engine.maximized_windows.contains(&wid) {
                        self.engine.maximized_windows.remove(&wid);
                        if let Some(win) = self.engine.current_windows.get_mut(&wid) {
                            win.is_maximized = false;
                            win.is_floating = false;
                        }
                        info!(
                            "[CONTROLLER] Ventana {} desmaximizada -> devuelta a mosaico",
                            wid
                        );
                        commands.push(RavenAction::SetMaximize {
                            window_id: wid,
                            maximized: false,
                        });
                        self.send_osd_notification("Ventana", "Modo: Mosaico (Tiling)");
                    } else {
                        self.engine.maximized_windows.insert(wid.clone());
                        if let Some(win) = self.engine.current_windows.get_mut(&wid) {
                            win.is_maximized = true;
                            win.is_floating = true;
                        }
                        info!(
                            "[CONTROLLER] Ventana {} añadida a estado Maximizado",
                            wid
                        );
                        commands.push(RavenAction::SetMaximize {
                            window_id: wid,
                            maximized: true,
                        });
                        self.send_osd_notification("Ventana", "Modo: Maximizado");
                    }
                    needs_recalc = true;
                }
            }
            "minimize_active" | "minimize_window" => {
                let target_wid = self.active_window_id.clone().or_else(|| {
                    self.engine.window_history.back().cloned().or_else(|| {
                        windows
                            .iter()
                            .find(|w| !w.is_minimized)
                            .map(|w| w.window_id.clone())
                    })
                });

                if let Some(wid) = target_wid {
                    self.flap_guard.remove(&wid);
                    self.last_known_layout.remove(&wid);
                    let target_ws = self
                        .engine
                        .current_windows
                        .get(&wid)
                        .map(|w| w.workspace_id.clone());
                    self.engine.minimized_windows.insert(wid.clone());
                    if let Some(win) = self.engine.current_windows.get_mut(&wid) {
                        win.is_minimized = true;
                    }

                    let next_focus_id = self
                        .engine
                        .window_history
                        .iter()
                        .rev()
                        .find(|&id| {
                            id != &wid
                                && self.engine.current_windows.get(id).is_some_and(|w| {
                                    !w.is_minimized
                                        && !w.is_floating
                                        && !w.is_pip
                                        && (target_ws.is_none()
                                            || Some(&w.workspace_id) == target_ws.as_ref())
                                })
                        })
                        .cloned()
                        .or_else(|| {
                            self.engine
                                .current_windows
                                .values()
                                .find(|w| {
                                    w.window_id != wid
                                        && !w.is_minimized
                                        && !w.is_floating
                                        && !w.is_pip
                                        && (target_ws.is_none()
                                            || Some(&w.workspace_id) == target_ws.as_ref())
                                })
                                .map(|w| w.window_id.clone())
                        });

                    self.active_window_id = next_focus_id.clone();

                    commands.push(RavenAction::MinimizeWindow {
                        window_id: wid.clone(),
                    });
                    if let Some(ref next_id) = next_focus_id {
                        commands.push(RavenAction::FocusWindow {
                            window_id: next_id.clone(),
                        });
                    }
                    self.send_osd_notification("Ventana", "Minimizar");
                    needs_recalc = true;
                }
            }
            "close_active" | "close_window" => {
                let target_wid = self.active_window_id.clone().or_else(|| {
                    self.engine.window_history.back().cloned().or_else(|| {
                        windows
                            .iter()
                            .find(|w| !w.is_minimized)
                            .map(|w| w.window_id.clone())
                    })
                });

                if let Some(wid) = target_wid {
                    commands.push(RavenAction::CloseWindow {
                        window_id: wid.clone(),
                    });
                    self.send_osd_notification("Ventana", "Cerrar");
                }
            }
            // Alternar estado flotante dinámico (Quick Peek)
            "toggle_floating" => {
                // Se prioriza la ventana activa explícita provista por KWin o el foco rastreado;
                // en su defecto, recurre al historial reciente de ventanas no minimizadas.
                let target_wid = self.active_window_id.clone().or_else(|| {
                    self.engine.window_history.back().cloned().or_else(|| {
                        windows
                            .iter()
                            .find(|w| !w.is_minimized)
                            .map(|w| w.window_id.clone())
                    })
                });

                if let Some(wid) = target_wid {
                    self.flap_guard.remove(&wid);
                    self.last_known_layout.remove(&wid);
                    if self.engine.dynamic_floating_windows.contains(&wid) {
                        // Caso A: La ventana ya está en Quick Peek -> Devolverla al layout de mosaico (Tiling)
                        self.engine.dynamic_floating_windows.remove(&wid);
                        if let Some(win) = self.engine.current_windows.get_mut(&wid) {
                            win.is_floating = false;
                        }
                        info!(
                            "[CONTROLLER] Ventana {} devuelta a la pila de mosaico (Tiling)",
                            wid
                        );
                        commands.push(RavenAction::SetFloating {
                            window_id: wid,
                            floating: false,
                            keep_above: false,
                        });
                        self.send_osd_notification("Ventana", "Modo: Mosaico (Tiling)");
                    } else {
                        // Caso B: La ventana está en mosaico -> Convertirla en flotante temporal (Quick Peek)
                        self.engine.dynamic_floating_windows.insert(wid.clone());
                        let ws_id = self
                            .engine
                            .current_windows
                            .get(&wid)
                            .map(|w| w.workspace_id.clone())
                            .unwrap_or_else(|| "default||default_desk".to_string());
                        let ws_rect = self.engine.current_workspaces.get(&ws_id).copied();
                        let float_rect = crate::application::services::floating_service::calculate_dynamic_floating_rect(
                            ws_rect,
                            &ws_id,
                            topology,
                        );

                        if let Some(win) = self.engine.current_windows.get_mut(&wid) {
                            win.is_floating = true;
                            win.geometry = float_rect;
                        }
                        info!(
                            "[CONTROLLER] Ventana {} añadida a la pila flotante dinámica (Quick Peek, {}x{})",
                            wid, float_rect.width, float_rect.height
                        );
                        commands.push(RavenAction::SetFloating {
                            window_id: wid.clone(),
                            floating: true,
                            keep_above: true,
                        });
                        commands.push(RavenAction::MoveWindow {
                            window_id: wid,
                            x: float_rect.x,
                            y: float_rect.y,
                            width: float_rect.width,
                            height: float_rect.height,
                        });
                        self.send_osd_notification("Ventana", "Modo: Flotante (Quick Peek)");
                    }
                    needs_recalc = true;
                }
            }
            "toggle_tiling" => {
                let enabled = self.engine.toggle_tiling();
                self.last_known_layout.clear();
                if enabled {
                    self.send_osd_notification(
                        "Modo Mosaico",
                        "Activado (Reorganizando ventanas)",
                    );
                } else {
                    self.send_osd_notification("Modo Mosaico", "Desactivado (Modo Flotante)");
                    let mut offset = 0;
                    for win in windows.iter() {
                        if !win.is_minimized && !win.is_floating {
                            let ws_rect = self.engine.current_workspaces.get(&win.workspace_id).copied();
                            let base_rect = crate::application::services::floating_service::calculate_dynamic_floating_rect(
                                ws_rect,
                                &win.workspace_id,
                                topology,
                            );
                            let float_x = base_rect.x + offset;
                            let float_y = base_rect.y + offset;

                            commands.push(RavenAction::MoveWindow {
                                window_id: win.window_id.clone(),
                                x: float_x,
                                y: float_y,
                                width: base_rect.width,
                                height: base_rect.height,
                            });
                            offset = (offset + 24) % 120;
                        }
                    }
                }
                needs_recalc = true;
            }
            "cycle_layout" => {
                self.last_known_layout.clear();
                let current_ws = self
                    .active_window_id
                    .as_ref()
                    .and_then(|wid| {
                        self.engine
                            .current_windows
                            .get(wid)
                            .map(|w| w.workspace_id.clone())
                    })
                    .or_else(|| {
                        self.engine.window_history.back().and_then(|wid| {
                            self.engine
                                .current_windows
                                .get(wid)
                                .map(|w| w.workspace_id.clone())
                        })
                    })
                    .or_else(|| {
                        self.engine
                            .current_windows
                            .values()
                            .find(|w| !w.is_minimized && !w.is_floating)
                            .map(|w| w.workspace_id.clone())
                    });

                let next_layout_for = |current: &str| -> String {
                    match current {
                        "raven" => "tall",
                        "tall" => "monocle",
                        "monocle" => "strict_dwindle",
                        "strict_dwindle" => "inverted_strict_dwindle",
                        "inverted_strict_dwindle" => "divisor",
                        "divisor" => "raven",
                        _ => "raven",
                    }
                    .to_string()
                };

                let new_layout_name;
                if let Some(ws_id) = current_ws {
                    let output_name = ws_id.split("||").next().unwrap_or(&ws_id).to_string();
                    let current = self
                        .engine
                        .config
                        .workspace_layouts
                        .get(&ws_id)
                        .or_else(|| self.engine.config.workspace_layouts.get(&output_name))
                        .cloned()
                        .unwrap_or_else(|| self.engine.config.layout_type.clone());

                    let next = next_layout_for(&current);
                    new_layout_name = next.clone();
                    self.engine
                        .config
                        .workspace_layouts
                        .insert(ws_id.clone(), next.clone());
                    if self.engine.config.workspace_layouts.contains_key(&output_name) {
                        self.engine
                            .config
                            .workspace_layouts
                            .insert(output_name, next.clone());
                    }
                    self.engine.config.layout_type = next.clone();
                    info!(
                        "[CONTROLLER] Layout de Workspace {} cambiado a: {}",
                        ws_id, next
                    );
                } else {
                    let next = next_layout_for(&self.engine.config.layout_type);
                    self.engine.config.layout_type = next.clone();
                    for val in self.engine.config.workspace_layouts.values_mut() {
                        *val = next.clone();
                    }
                    new_layout_name = next;
                    info!(
                        "[CONTROLLER] Layout global cambiado a: {}",
                        self.engine.config.layout_type
                    );
                }

                let readable_name = match new_layout_name.as_str() {
                    "tall" => "Tall (Columna)",
                    "monocle" => "Monocle (Monocromático)",
                    "strict_dwindle" => "Strict Dwindle (Espiral)",
                    "inverted_strict_dwindle" => "Inverted Dwindle (Espiral Invertida)",
                    "divisor" => "Divisor (Cuadrícula)",
                    _ => "Raven BSP (Foveal)",
                };
                self.send_osd_notification(
                    "Disposición de Ventanas",
                    &format!("Layout: {}", readable_name),
                );

                needs_recalc = true;
                config_changed = true;
            }
            "increment_gaps" => {
                self.last_known_layout.clear();
                self.engine.config.default_gaps =
                    std::cmp::max(0, self.engine.config.default_gaps + _payload);
                let sign = if _payload >= 0 {
                    format!("+{}", _payload)
                } else {
                    format!("{}", _payload)
                };
                self.send_osd_notification(
                    "Márgenes de Ventana",
                    &format!(
                        "Gaps {} px (Total: {} px)",
                        sign, self.engine.config.default_gaps
                    ),
                );
                needs_recalc = true;
                config_changed = true;
            }
            // BUG-01: handlers de nmaster ahora implementados correctamente
            "increment_nmaster" => {
                self.last_known_layout.clear();
                self.engine.config.nmaster = (self.engine.config.nmaster + 1).min(8);
                needs_recalc = true;
                config_changed = true;
            }
            "decrement_nmaster" => {
                self.last_known_layout.clear();
                self.engine.config.nmaster = self.engine.config.nmaster.saturating_sub(1).max(1);
                needs_recalc = true;
                config_changed = true;
            }
            "increase_ratio" => {
                self.last_known_layout.clear();
                self.engine.config.master_ratio =
                    f32::min(0.85, self.engine.config.master_ratio + 0.05);
                needs_recalc = true;
                config_changed = true;
            }
            "decrease_ratio" => {
                self.last_known_layout.clear();
                self.engine.config.master_ratio =
                    f32::max(0.25, self.engine.config.master_ratio - 0.05);
                needs_recalc = true;
                config_changed = true;
            }
            "resize_width_inc" | "resize_width_dec" | "resize_height_inc" | "resize_height_dec" => {
                let target_wid = self.active_window_id.clone().or_else(|| {
                    self.engine.window_history.back().cloned()
                });
                if let Some(ref wid) = target_wid {
                    if let Some(win) = self.engine.current_windows.get_mut(wid) {
                        self.last_known_layout.clear();
                        let delta = 0.05f32;
                        match action.as_str() {
                            "resize_width_inc" => {
                                let cur = win.custom_w_ratio.unwrap_or(1.0);
                                win.custom_w_ratio = Some((cur + delta).min(3.0));
                                self.engine.config.master_ratio =
                                    (self.engine.config.master_ratio + delta).clamp(0.25, 0.85);
                            }
                            "resize_width_dec" => {
                                let cur = win.custom_w_ratio.unwrap_or(1.0);
                                win.custom_w_ratio = Some((cur - delta).max(0.2));
                                self.engine.config.master_ratio =
                                    (self.engine.config.master_ratio - delta).clamp(0.25, 0.85);
                            }
                            "resize_height_inc" => {
                                let cur = win.custom_h_ratio.unwrap_or(1.0);
                                win.custom_h_ratio = Some((cur + delta).min(3.0));
                            }
                            "resize_height_dec" => {
                                let cur = win.custom_h_ratio.unwrap_or(1.0);
                                win.custom_h_ratio = Some((cur - delta).max(0.2));
                            }
                            _ => {}
                        }
                        needs_recalc = true;
                    }
                }
            }
            "swap_next" | "swap_prev" => {
                self.last_known_layout.clear();
                self.send_osd_notification(
                    "Organización de Mosaico",
                    if action == "swap_next" {
                        "Intercambiar Ventana (Siguiente)"
                    } else {
                        "Intercambiar Ventana (Anterior)"
                    },
                );

                let effective_active_id = self.active_window_id.clone().or_else(|| {
                    self.engine
                        .window_history
                        .back()
                        .cloned()
                        .or_else(|| {
                            windows
                                .iter()
                                .find(|w| !w.is_floating && !w.is_minimized && !w.is_pip)
                                .map(|w| w.window_id.clone())
                        })
                });

                let active_ws = effective_active_id.as_ref().and_then(|aid| {
                    self.engine
                        .current_windows
                        .get(aid)
                        .map(|w| w.workspace_id.clone())
                });

                let mut active_windows: Vec<_> = windows
                    .into_iter()
                    .filter(|w| {
                        !w.is_floating
                            && !w.is_minimized
                            && !w.is_pip
                            && (active_ws.is_none() || Some(&w.workspace_id) == active_ws.as_ref())
                    })
                    .collect();

                if active_windows.len() > 1 {
                    active_windows.sort_by_key(|w| {
                        self.engine
                            .spatial_order
                            .iter()
                            .position(|id| id == &w.window_id)
                            .unwrap_or(usize::MAX)
                    });

                    if let Some(ref active_id) = effective_active_id {
                        if let Some(current_idx) = active_windows
                            .iter()
                            .position(|w| &w.window_id == active_id)
                        {
                            let step = if action == "swap_next" {
                                1
                            } else {
                                active_windows.len() - 1
                            };
                            let target_idx = (current_idx + step) % active_windows.len();
                            let target_id = &active_windows[target_idx].window_id;

                            let s_pos_active = self
                                .engine
                                .spatial_order
                                .iter()
                                .position(|id| id == active_id);
                            let s_pos_target = self
                                .engine
                                .spatial_order
                                .iter()
                                .position(|id| id == target_id);

                            if let (Some(p_act), Some(p_tar)) = (s_pos_active, s_pos_target) {
                                self.engine.spatial_order.swap(p_act, p_tar);
                                needs_recalc = true;
                            }

                            let h_pos_active = self
                                .engine
                                .window_history
                                .iter()
                                .position(|id| id == active_id);
                            let h_pos_target = self
                                .engine
                                .window_history
                                .iter()
                                .position(|id| id == target_id);

                            if let (Some(p_act), Some(p_tar)) = (h_pos_active, h_pos_target) {
                                self.engine.window_history.swap(p_act, p_tar);
                            }
                        }
                    }
                } else {
                    commands.push(RavenAction::RequestSync);
                }
            }
            "focus_next" | "focus_prev" => {
                let mut active_windows: Vec<_> = windows
                    .into_iter()
                    .filter(|w| !w.is_floating && !w.is_minimized && !w.is_pip)
                    .collect();

                if !active_windows.is_empty() {
                    active_windows.sort_by_key(|w| {
                        self.engine
                            .spatial_order
                            .iter()
                            .position(|id| id == &w.window_id)
                            .unwrap_or(usize::MAX)
                    });

                    let current_idx = active_windows
                        .iter()
                        .position(|w| Some(&w.window_id) == active_window_id.as_ref())
                        .unwrap_or(0);

                    let step = if action == "focus_next" {
                        1
                    } else {
                        active_windows.len() - 1
                    };
                    let next_idx = (current_idx + step) % active_windows.len();

                    commands.push(RavenAction::FocusWindow {
                        window_id: active_windows[next_idx].window_id.clone(),
                    });
                }
            }
            "focus_left" | "focus_right" | "focus_up" | "focus_down" => {
                let (dx, dy) = match action.as_str() {
                    "focus_left" => (-1, 0),
                    "focus_right" => (1, 0),
                    "focus_up" => (0, -1),
                    "focus_down" => (0, 1),
                    _ => (0, 0),
                };

                let effective_active_id = self.active_window_id.clone().or_else(|| {
                    self.engine.window_history.back().cloned().or_else(|| {
                        windows
                            .iter()
                            .find(|w| !w.is_floating && !w.is_minimized)
                            .map(|w| w.window_id.clone())
                    })
                });

                let fallback_map: HashMap<String, Rect>;
                let effective_layout = if self
                    .last_known_layout
                    .contains_key(effective_active_id.as_deref().unwrap_or(""))
                    && self.last_known_layout.len() > 1
                {
                    &self.last_known_layout
                } else {
                    // Fallback resiliente: usar geometrías directas de windows si last_known_layout aún no ha sido confirmado por KWin
                    fallback_map = windows
                        .iter()
                        .filter(|w| !w.is_floating && !w.is_minimized)
                        .map(|w| (w.window_id.clone(), w.geometry))
                        .collect();
                    &fallback_map
                };

                if let Some(ref act_id) = effective_active_id {
                    if let Some(target_id) = crate::domain::layout::topology::find_directional_focus(
                        act_id,
                        effective_layout,
                        dx,
                        dy,
                    ) {
                        commands.push(RavenAction::FocusWindow {
                            window_id: target_id,
                        });
                    }
                }
            }
            "migrate_active_to_screen"
            | "migrate_active_to_desktop"
            | "migrate_active_to_prev_screen"
            | "migrate_active_to_prev_desktop" => {
                let target_wid = self.active_window_id.clone().or_else(|| {
                    self.engine.window_history.back().cloned().or_else(|| {
                        windows
                            .iter()
                            .find(|w| !w.is_floating && !w.is_minimized)
                            .map(|w| w.window_id.clone())
                    })
                });

                if let Some(ref wid) = target_wid {
                    if let Some(win_node) = windows.iter().find(|w| &w.window_id == wid) {
                        let is_desktop = action.contains("desktop");
                        let is_prev = action.contains("prev");
                        if is_desktop {
                            let desktops = &topology.desktops;
                            let current_desk =
                                win_node.desktops.first().cloned().unwrap_or_default();

                            if let Some(current_idx) =
                                desktops.iter().position(|d| d == &current_desk)
                            {
                                let target_idx = if is_prev {
                                    if current_idx == 0 {
                                        desktops.len() - 1
                                    } else {
                                        current_idx - 1
                                    }
                                } else {
                                    (current_idx + 1) % desktops.len()
                                };

                                if let Some(target_desk) = desktops.get(target_idx) {
                                    commands.push(RavenAction::MigrateToDesktop {
                                        window_id: wid.clone(),
                                        target_desktop: target_desk.clone(),
                                    });
                                    self.flap_guard.remove(wid);
                                    self.last_known_layout.remove(wid);
                                    if let Some(target_w) = self.engine.current_windows.get_mut(wid)
                                    {
                                        target_w.desktops = vec![target_desk.clone()];
                                        target_w.workspace_id =
                                            format!("{}||{}", target_w.output, target_desk);
                                    }
                                    needs_recalc = true;
                                    self.send_osd_notification(
                                        "Espacio de Trabajo",
                                        &format!("Ventana enviada a escritorio {}", target_idx + 1),
                                    );
                                }
                            }
                        } else {
                            let outputs = &topology.outputs;
                            let current_out = &win_node.output;

                            if let Some(current_idx) = outputs.iter().position(|o| o == current_out)
                            {
                                let target_idx = if is_prev {
                                    if current_idx == 0 {
                                        outputs.len() - 1
                                    } else {
                                        current_idx - 1
                                    }
                                } else {
                                    (current_idx + 1) % outputs.len()
                                };

                                if let Some(target_out) = outputs.get(target_idx) {
                                    commands.push(RavenAction::MigrateToOutput {
                                        window_id: wid.clone(),
                                        target_output: target_out.clone(),
                                    });
                                    self.flap_guard.remove(wid);
                                    self.last_known_layout.remove(wid);
                                    if let Some(target_w) = self.engine.current_windows.get_mut(wid)
                                    {
                                        let desk =
                                            target_w.desktops.first().cloned().unwrap_or_default();
                                        target_w.output = target_out.clone();
                                        target_w.workspace_id = format!("{}||{}", target_out, desk);
                                    }
                                    needs_recalc = true;
                                    self.send_osd_notification(
                                        "Monitor",
                                        &format!("Ventana enviada a monitor {}", target_out),
                                    );
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        if config_changed {
            if let Err(e) = self.engine.config.save() {
                warn!("[CONTROLLER] Error al persistir configuración: {}", e);
            }
        }

        Ok((needs_recalc, commands))
    }

    /// Recarga en caliente la configuración del motor sin reiniciar el demonio ni perder el historial
    /// de ventanas, actualizando de inmediato la geometría de todas las ventanas activas.
    ///
    /// # Parámetros
    /// * `new_config` - Nueva estructura `RavenConfig` leída desde el disco.
    ///
    /// # Retorno
    /// Vector de comandos `RavenAction` para aplicar la nueva composición al instante.
    pub fn reload_config(
        &mut self,
        new_config: crate::infrastructure::config::RavenConfig,
    ) -> Result<Vec<RavenAction>, RavenError> {
        self.engine.config = new_config;
        for win in self.engine.current_windows.values_mut() {
            win.custom_w_ratio = None;
            win.custom_h_ratio = None;
        }
        self.last_known_layout.clear();
        self.flap_guard.clear();
        self.commit_layout()
    }

    /// Adapta en tiempo real los parámetros del algoritmo de composición (`master_ratio`,
    /// `custom_w_ratio`, `custom_h_ratio`) cuando el usuario redimensiona una ventana con el cursor.
    ///
    /// Incluye acoplamiento magnético suave en proporciones canónicas (`50/50`, `60/40`, `40/60`,
    /// `70/30`, `30/70`) para facilitar ajustes precisos sin que el demonio devuelva la ventana
    /// a su dimensión anterior.
    ///
    /// # Parámetros
    /// * `window_id` - Identificador de la ventana redimensionada por el usuario.
    /// * `new_rect` - Nueva geometría resultante de la interacción con el ratón.
    ///
    /// # Retorno
    /// Vector de acciones `RavenAction` para reacomodar las ventanas vecinas en tiempo real.
    pub fn handle_user_resize(
        &mut self,
        window_id: &str,
        new_rect: Rect,
    ) -> Result<Vec<RavenAction>, RavenError> {
        let (ws_id, is_floating, is_minimized, old_win_geom) =
            match self.engine.current_windows.get(window_id) {
                Some(w) => (w.workspace_id.clone(), w.is_floating, w.is_minimized, w.geometry),
                None => return Ok(Vec::new()),
            };

        if is_floating || is_minimized {
            if let Some(w) = self.engine.current_windows.get_mut(window_id) {
                w.geometry = new_rect;
            }
            return Ok(Vec::new());
        }

        let old_rect = self
            .last_known_layout
            .get(window_id)
            .copied()
            .unwrap_or(old_win_geom);
        let screen_rect = self
            .engine
            .current_workspaces
            .get(&ws_id)
            .or_else(|| {
                let out = ws_id.split("||").next().unwrap_or(&ws_id);
                self.engine.current_workspaces.get(out)
            })
            .copied()
            .unwrap_or(Rect::new(0, 0, 1920, 1080));

        let gaps = self.engine.config.default_gaps;
        let usable_w = (screen_rect.width - gaps).max(1) as f32;
        let usable_h = (screen_rect.height - gaps).max(1) as f32;

        // Ordenar las ventanas activas del workspace según spatial_order para identificar el rol de la ventana
        let mut ws_windows: Vec<String> = self
            .engine
            .current_windows
            .values()
            .filter(|w| {
                w.workspace_id == ws_id && !w.is_floating && !w.is_minimized && !w.is_pip
            })
            .map(|w| w.window_id.clone())
            .collect();

        ws_windows.sort_by_key(|id| {
            self.engine
                .spatial_order
                .iter()
                .position(|sid| sid == id)
                .unwrap_or(usize::MAX)
        });

        let total_count = ws_windows.len();
        let win_idx = ws_windows
            .iter()
            .position(|id| id == window_id)
            .unwrap_or(0);

        let output_name = ws_id.split("||").next().unwrap_or(&ws_id);
        let layout_type = self
            .engine
            .config
            .workspace_layouts
            .get(&ws_id)
            .or_else(|| self.engine.config.workspace_layouts.get(output_name))
            .cloned()
            .unwrap_or_else(|| self.engine.config.layout_type.clone());

        let snap_canonical_ratio = |r: f32| -> f32 {
            let clamped = r.clamp(0.20, 0.80);
            if (clamped - 0.50).abs() <= 0.025 {
                0.50
            } else if (clamped - 0.60).abs() <= 0.015 {
                0.60
            } else if (clamped - 0.40).abs() <= 0.015 {
                0.40
            } else if (clamped - 0.70).abs() <= 0.015 {
                0.70
            } else if (clamped - 0.30).abs() <= 0.015 {
                0.30
            } else {
                clamped
            }
        };

        let mut config_changed = false;
        let dw = (new_rect.width - old_rect.width).abs();
        let dh = (new_rect.height - old_rect.height).abs();

        if dw > 4 && total_count >= 2 {
            if total_count == 2 {
                // Para 2 ventanas en mosaico horizontal: calcular la relación espacial física
                // directa entre la ventana redimensionada y su vecina.
                let other_id = ws_windows.iter().find(|id| id.as_str() != window_id);
                let other_geom = other_id
                    .and_then(|oid| {
                        self.last_known_layout
                            .get(oid)
                            .or_else(|| self.engine.current_windows.get(oid).map(|w| &w.geometry))
                    })
                    .copied()
                    .unwrap_or(Rect::new(0, 0, 100, 100));

                let win_is_left = old_rect.x < other_geom.x;
                let raw_ratio_left = if win_is_left {
                    ((new_rect.width + gaps) as f32 / usable_w).clamp(0.15, 0.85)
                } else {
                    (1.0 - ((new_rect.width + gaps) as f32 / usable_w)).clamp(0.15, 0.85)
                };

                // Imantado magnético suave cerca de 50/50, manteniendo fluidez continua en el resto
                let ratio_left = if (raw_ratio_left - 0.50).abs() <= 0.020 {
                    0.50
                } else if (raw_ratio_left - 0.60).abs() <= 0.012 {
                    0.60
                } else if (raw_ratio_left - 0.40).abs() <= 0.012 {
                    0.40
                } else {
                    raw_ratio_left
                };

                match layout_type.as_str() {
                    "tall" | "strict_dwindle" => {
                        self.engine.config.master_ratio = ratio_left;
                        config_changed = true;
                    }
                    "inverted_strict_dwindle" => {
                        self.engine.config.master_ratio = 1.0 - ratio_left;
                        config_changed = true;
                    }
                    "divisor" => {
                        if let Some(w_curr) = self.engine.current_windows.get_mut(window_id) {
                            let r = if win_is_left { ratio_left } else { 1.0 - ratio_left };
                            w_curr.custom_w_ratio = Some((r / (1.0 - r).max(0.01)).clamp(0.20, 5.0));
                        }
                    }
                    _ => {
                        // Raven DwindleBSP: en 2 ventanas, la ventana izquierda tiene cuota (1 - master_ratio)
                        // y la central/derecha tiene master_ratio.
                        self.engine.config.master_ratio = (1.0 - ratio_left).clamp(0.15, 0.85);
                        config_changed = true;
                    }
                }
            } else {
                // 3 o más ventanas: Búsqueda geométrica de ventana hermana (sibling) adyacente en la misma fila o columna.
                let other_windows: Vec<(String, Rect)> = ws_windows
                    .iter()
                    .filter(|id| id.as_str() != window_id)
                    .filter_map(|id| {
                        let g = self
                            .last_known_layout
                            .get(id)
                            .or_else(|| self.engine.current_windows.get(id).map(|w| &w.geometry))
                            .copied()?;
                        Some((id.clone(), g))
                    })
                    .collect();

                let mut horizontal_sibling: Option<(String, Rect, bool)> = None;
                if dw > 4 {
                    for (oid, og) in &other_windows {
                        let same_row = (old_rect.y - og.y).abs() <= gaps + 10
                            && (old_rect.height - og.height).abs() <= gaps + 16;
                        if same_row {
                            let is_right = (og.x - (old_rect.x + old_rect.width)).abs() <= gaps * 2 + 14;
                            let is_left = ((og.x + og.width) - old_rect.x).abs() <= gaps * 2 + 14;
                            if is_right {
                                horizontal_sibling = Some((oid.clone(), *og, true));
                                break;
                            } else if is_left {
                                horizontal_sibling = Some((oid.clone(), *og, false));
                                break;
                            }
                        }
                    }
                }

                let mut vertical_sibling: Option<(String, Rect, bool)> = None;
                if dh > 4 {
                    for (oid, og) in &other_windows {
                        let same_col = (old_rect.x - og.x).abs() <= gaps + 10
                            && (old_rect.width - og.width).abs() <= gaps + 16;
                        if same_col {
                            let is_bottom = (og.y - (old_rect.y + old_rect.height)).abs() <= gaps * 2 + 14;
                            let is_top = ((og.y + og.height) - old_rect.y).abs() <= gaps * 2 + 14;
                            if is_bottom {
                                vertical_sibling = Some((oid.clone(), *og, true));
                                break;
                            } else if is_top {
                                vertical_sibling = Some((oid.clone(), *og, false));
                                break;
                            }
                        }
                    }
                }

                // 1. Si se detectó redimensionamiento entre dos hermanas horizontales que comparten fila:
                if let Some((sib_id, sib_geom, self_is_left)) = horizontal_sibling {
                    let total_shared_w = (old_rect.width + sib_geom.width).max(1);
                    let (new_left_w, new_right_w) = if self_is_left {
                        let clamped_self = new_rect.width.clamp(80, total_shared_w - 80);
                        (clamped_self, total_shared_w - clamped_self)
                    } else {
                        let clamped_self = new_rect.width.clamp(80, total_shared_w - 80);
                        (total_shared_w - clamped_self, clamped_self)
                    };

                    let ratio_l = (new_left_w as f32 / total_shared_w as f32).clamp(0.10, 0.90);
                    let ratio_r = (new_right_w as f32 / total_shared_w as f32).clamp(0.10, 0.90);

                    let (w_left_id, w_right_id) = if self_is_left {
                        (window_id.to_string(), sib_id)
                    } else {
                        (sib_id, window_id.to_string())
                    };

                    if let Some(wl) = self.engine.current_windows.get_mut(&w_left_id) {
                        wl.custom_w_ratio = Some(ratio_l);
                    }
                    if let Some(wr) = self.engine.current_windows.get_mut(&w_right_id) {
                        wr.custom_w_ratio = Some(ratio_r);
                    }
                    config_changed = true;
                } else if dw > 4 {
                    // Si no es un par de hermanas de fila, es la frontera entre Master y columnas
                    let frac_w = ((new_rect.width + gaps) as f32 / usable_w).clamp(0.15, 0.85);
                    match layout_type.as_str() {
                        "divisor" => {
                            if let Some(w) = self.engine.current_windows.get_mut(window_id) {
                                let scale_w =
                                    (new_rect.width as f32 / old_rect.width.max(1) as f32).clamp(0.20, 5.0);
                                let cur = w.custom_w_ratio.unwrap_or(1.0);
                                w.custom_w_ratio = Some((cur * scale_w).clamp(0.15, 5.0));
                            }
                        }
                        "tall" | "strict_dwindle" => {
                            let raw_ratio = if win_idx < self.engine.config.nmaster.max(1) {
                                frac_w
                            } else {
                                1.0 - frac_w
                            };
                            self.engine.config.master_ratio = snap_canonical_ratio(raw_ratio);
                            config_changed = true;
                        }
                        "inverted_strict_dwindle" => {
                            let raw_ratio = if win_idx == 0 { frac_w } else { 1.0 - frac_w };
                            self.engine.config.master_ratio = snap_canonical_ratio(raw_ratio);
                            config_changed = true;
                        }
                        _ => {
                            if win_idx == 0 {
                                self.engine.config.master_ratio = snap_canonical_ratio(frac_w);
                                config_changed = true;
                            } else {
                                let raw_ratio = (1.0 - (2.0 * frac_w)).clamp(0.20, 0.85);
                                self.engine.config.master_ratio = snap_canonical_ratio(raw_ratio);
                                config_changed = true;
                            }
                        }
                    }
                }

                // 2. Si se detectó redimensionamiento entre dos hermanas verticales que comparten columna:
                if let Some((sib_id, sib_geom, self_is_top)) = vertical_sibling {
                    let total_shared_h = (old_rect.height + sib_geom.height).max(1);
                    let (new_top_h, new_bottom_h) = if self_is_top {
                        let clamped_self = new_rect.height.clamp(80, total_shared_h - 80);
                        (clamped_self, total_shared_h - clamped_self)
                    } else {
                        let clamped_self = new_rect.height.clamp(80, total_shared_h - 80);
                        (total_shared_h - clamped_self, clamped_self)
                    };

                    let ratio_t = (new_top_h as f32 / total_shared_h as f32).clamp(0.10, 0.90);
                    let ratio_b = (new_bottom_h as f32 / total_shared_h as f32).clamp(0.10, 0.90);

                    let (w_top_id, w_bottom_id) = if self_is_top {
                        (window_id.to_string(), sib_id)
                    } else {
                        (sib_id, window_id.to_string())
                    };

                    if let Some(wt) = self.engine.current_windows.get_mut(&w_top_id) {
                        wt.custom_h_ratio = Some(ratio_t);
                    }
                    if let Some(wb) = self.engine.current_windows.get_mut(&w_bottom_id) {
                        wb.custom_h_ratio = Some(ratio_b);
                    }
                    config_changed = true;
                } else if dh > 4 {
                    // Si no es un par de hermanas de columna, es la frontera entre Master Superior y Panel Inferior
                    let frac_h = ((new_rect.height + gaps) as f32 / usable_h).clamp(0.15, 0.85);
                    let aspect_ratio = usable_w / usable_h.max(1.0);
                    if layout_type == "raven" && total_count == 3 && aspect_ratio >= 1.55 {
                        let raw_ratio = if win_idx == 0 { frac_h } else { 1.0 - frac_h };
                        self.engine.config.master_ratio = snap_canonical_ratio(raw_ratio);
                        config_changed = true;
                    } else if let Some(w) = self.engine.current_windows.get_mut(window_id) {
                        let scale_h =
                            (new_rect.height as f32 / old_rect.height.max(1) as f32).clamp(0.20, 5.0);
                        let cur = w.custom_h_ratio.unwrap_or(1.0);
                        w.custom_h_ratio = Some((cur * scale_h).clamp(0.15, 5.0));
                    }
                }
            }
        }

        if let Some(w) = self.engine.current_windows.get_mut(window_id) {
            w.geometry = new_rect;
        }
        self.flap_guard.remove(window_id);
        self.last_known_layout.clear();

        if config_changed {
            let _ = self.engine.config.save();
        }

        self.commit_layout()
    }

    /// Reubica o intercambia interactivamente una ventana en el mosaico activo cuando el usuario
    /// la arrastra y suelta con el cursor del ratón sobre otro slot o monitor.
    ///
    /// # Parámetros
    /// * `window_id` - Identificador de la ventana arrastrada por el usuario.
    /// * `drop_x` - Coordenada X global del punto de suelta (cursor o centro de ventana).
    /// * `drop_y` - Coordenada Y global del punto de suelta (cursor o centro de ventana).
    ///
    /// # Retorno
    /// Vector de acciones `RavenAction` para posicionar e intercambiar las ventanas involucradas.
    pub fn handle_user_move(
        &mut self,
        window_id: &str,
        drop_x: i32,
        drop_y: i32,
    ) -> Result<Vec<RavenAction>, RavenError> {
        let (mut current_ws_id, is_floating, is_minimized) =
            match self.engine.current_windows.get(window_id) {
                Some(w) => (w.workspace_id.clone(), w.is_floating, w.is_minimized),
                None => return Ok(Vec::new()),
            };

        if is_floating || is_minimized {
            return Ok(Vec::new());
        }

        let current_desk = current_ws_id
            .split("||")
            .nth(1)
            .unwrap_or("default_desk")
            .to_string();

        // Comprobar si el usuario arrastró la ventana hacia otro monitor físico
        let mut migrated_output: Option<String> = None;
        for (ws_key, ws_rect) in &self.engine.current_workspaces {
            let desk_part = ws_key.split("||").nth(1).unwrap_or("default_desk");
            if desk_part == current_desk
                && drop_x >= ws_rect.x
                && drop_x <= ws_rect.x + ws_rect.width
                && drop_y >= ws_rect.y
                && drop_y <= ws_rect.y + ws_rect.height
            {
                if ws_key != &current_ws_id {
                    let new_out = ws_key.split("||").next().unwrap_or("default").to_string();
                    migrated_output = Some(new_out);
                    current_ws_id = ws_key.clone();
                }
                break;
            }
        }

        if let Some(ref new_out) = migrated_output {
            if let Some(win) = self.engine.current_windows.get_mut(window_id) {
                win.output = new_out.clone();
                win.workspace_id = current_ws_id.clone();
            }
        }

        // Buscar la ventana objetivo en el workspace destino cuyo slot contenga (drop_x, drop_y)
        // o cuyo centro geométrico sea el más cercano al punto de suelta.
        let candidate_ids: Vec<String> = self
            .engine
            .current_windows
            .values()
            .filter(|w| {
                w.workspace_id == current_ws_id && !w.is_floating && !w.is_minimized && !w.is_pip
            })
            .map(|w| w.window_id.clone())
            .collect();

        let mut target_window_id: Option<String> = None;
        let mut best_dist_sq = i64::MAX;

        for cid in &candidate_ids {
            let slot_rect = self
                .last_known_layout
                .get(cid)
                .copied()
                .or_else(|| self.engine.current_windows.get(cid).map(|w| w.geometry));

            if let Some(r) = slot_rect {
                let contains = drop_x >= r.x
                    && drop_x <= r.x + r.width
                    && drop_y >= r.y
                    && drop_y <= r.y + r.height;
                if contains && cid != window_id {
                    target_window_id = Some(cid.clone());
                    break;
                }
                let cx = (r.x + r.width / 2) as i64;
                let cy = (r.y + r.height / 2) as i64;
                let dx = drop_x as i64 - cx;
                let dy = drop_y as i64 - cy;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < best_dist_sq {
                    best_dist_sq = dist_sq;
                    target_window_id = Some(cid.clone());
                }
            }
        }

        if let Some(ref target_id) = target_window_id {
            if target_id != window_id {
                let s_pos_a = self
                    .engine
                    .spatial_order
                    .iter()
                    .position(|id| id == window_id);
                let s_pos_b = self
                    .engine
                    .spatial_order
                    .iter()
                    .position(|id| id == target_id);
                if let (Some(pa), Some(pb)) = (s_pos_a, s_pos_b) {
                    self.engine.spatial_order.swap(pa, pb);
                }

                let h_pos_a = self
                    .engine
                    .window_history
                    .iter()
                    .position(|id| id == window_id);
                let h_pos_b = self
                    .engine
                    .window_history
                    .iter()
                    .position(|id| id == target_id);
                if let (Some(pa), Some(pb)) = (h_pos_a, h_pos_b) {
                    self.engine.window_history.swap(pa, pb);
                }
            }
        }

        self.flap_guard.remove(window_id);
        self.last_known_layout.clear();
        let mut cmds = Vec::new();
        if let Some(target_out) = migrated_output {
            cmds.push(RavenAction::MigrateToOutput {
                window_id: window_id.to_string(),
                target_output: target_out,
            });
        }
        cmds.extend(self.commit_layout()?);
        Ok(cmds)
    }
}
