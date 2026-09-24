//! # Controlador de Aplicación y Mitigador de Inestabilidad (`RavenController`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
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
        // las ventanas se ordenan por `spatial_order`.
        // La evicción por saturación (si excede Cmax) utilizará independientemente `window_history` para desalojar las más antiguas.
        windows.sort_by_key(|w| {
            self.engine
                .spatial_order
                .iter()
                .position(|id| id == &w.window_id)
                .unwrap_or(usize::MAX)
        });

        let (new_layout, evicted_windows) = self.engine.calculate_from_payload(
            &workspaces,
            &windows,
            self.active_window_id.clone(),
        )?;
        let mut commands = Vec::new();

        for (wid, rect) in &new_layout {
            let mut win_rect_differs = false;
            if let Some(win_node) = windows.iter().find(|w| &w.window_id == wid) {
                let dx = (win_node.geometry.x - rect.x).abs();
                let dy = (win_node.geometry.y - rect.y).abs();
                let dw = (win_node.geometry.width - rect.width).abs();
                let dh = (win_node.geometry.height - rect.height).abs();
                
                if dx > 3 || dy > 3 || dw > 3 || dh > 3 {
                    win_rect_differs = true;
                }
            }

            let is_quarantined_or_strict = windows
                .iter()
                .find(|w| &w.window_id == wid)
                .map(|w| w.is_quarantined || w.strict_birth)
                .unwrap_or(false);

            let needs_move = match self.last_known_layout.get(wid) {
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

        // --- Closed-Loop Revalidation (Verificación de Bucle Cerrado) ---
        // Registramos en last_known_layout únicamente aquellas ventanas cuya geometría física reportada
        // por KWin ya converge con el objetivo calculado por Rust. Si una ventana no alcanzó su tamaño,
        // no se marca como 'confirmada' en caché, garantizando que el motor revalide y reenvíe la orden.
        let mut confirmed_layout = HashMap::new();
        for (wid, rect) in &new_layout {
            if let Some(win_node) = windows.iter().find(|w| &w.window_id == wid) {
                let dx = (win_node.geometry.x - rect.x).abs();
                let dy = (win_node.geometry.y - rect.y).abs();
                let dw = (win_node.geometry.width - rect.width).abs();
                let dh = (win_node.geometry.height - rect.height).abs();

                // Tolerancia de 2px para redondeos enteros de gaps
                if dx <= 2 && dy <= 2 && dw <= 2 && dh <= 2 {
                    confirmed_layout.insert(wid.clone(), *rect);
                }
            }
        }
        self.last_known_layout = confirmed_layout;

        self.engine.current_workspaces = workspaces;
        self.engine.current_windows = windows
            .into_iter()
            .map(|w| (w.window_id.clone(), w))
            .collect();
        Ok(commands)
    }

    /// Procesa de forma incremental la actualización de estado de una sola ventana.
    ///
    /// # Parámetros
    /// * `win` - Nodo de ventana con los cambios recientes.
    ///
    /// Solo actualiza el estado interno (diccionario). Para generar comandos y redibujar, debe llamarse explícitamente a `commit_layout()`.
    pub fn handle_delta_change(&mut self, win: WindowNode) {
        self.engine
            .current_windows
            .insert(win.window_id.clone(), win);
    }

    /// Compromete el estado de las ventanas actuales y calcula el diseño (layout) geométrico definitivo.
    ///
    /// # Retorno
    /// Vector de comandos resultantes de evaluar el cambio en el motor.
    pub fn commit_layout(&mut self) -> Result<Vec<RavenAction>, RavenError> {
        self.last_known_layout.clear();
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
        let rectify_cmds: Vec<RavenAction> = layout_cmds
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
                    commands.push(RavenAction::MinimizeWindow {
                        window_id: wid.clone(),
                    });
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
                    self.flap_guard.remove(&wid);
                    self.last_known_layout.remove(&wid);
                    commands.push(RavenAction::CloseWindow {
                        window_id: wid.clone(),
                    });
                    self.send_osd_notification("Ventana", "Cerrar");
                    needs_recalc = true;
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
                        let ws_rect = self
                            .engine
                            .current_workspaces
                            .get(&ws_id)
                            .copied()
                            .unwrap_or_else(|| Rect::new(0, 0, 1920, 1080));
                        let float_w = ((ws_rect.width as f32 * 0.60).round() as i32)
                            .max(640)
                            .min(ws_rect.width - 40);
                        let float_h = ((ws_rect.height as f32 * 0.60).round() as i32)
                            .max(480)
                            .min(ws_rect.height - 40);
                        let float_x = ws_rect.x + (ws_rect.width - float_w) / 2;
                        let float_y = ws_rect.y + (ws_rect.height - float_h) / 2;

                        if let Some(win) = self.engine.current_windows.get_mut(&wid) {
                            win.is_floating = true;
                            win.geometry = Rect::new(float_x, float_y, float_w, float_h);
                        }
                        info!(
                            "[CONTROLLER] Ventana {} añadida a la pila flotante dinámica (Quick Peek, {}x{})",
                            wid, float_w, float_h
                        );
                        commands.push(RavenAction::SetFloating {
                            window_id: wid.clone(),
                            floating: true,
                            keep_above: true,
                        });
                        commands.push(RavenAction::MoveWindow {
                            window_id: wid,
                            x: float_x,
                            y: float_y,
                            width: float_w,
                            height: float_h,
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
                            let ws_rect = self
                                .engine
                                .current_workspaces
                                .get(&win.workspace_id)
                                .copied()
                                .unwrap_or_else(|| Rect::new(0, 0, 1920, 1080));
                            let float_w = ((ws_rect.width as f32 * 0.60).round() as i32)
                                .max(640)
                                .min(ws_rect.width - 40);
                            let float_h = ((ws_rect.height as f32 * 0.60).round() as i32)
                                .max(480)
                                .min(ws_rect.height - 40);
                            let base_x = ws_rect.x + (ws_rect.width - float_w) / 2;
                            let base_y = ws_rect.y + (ws_rect.height - float_h) / 2;
                            let float_x = base_x + offset;
                            let float_y = base_y + offset;

                            commands.push(RavenAction::MoveWindow {
                                window_id: win.window_id.clone(),
                                x: float_x,
                                y: float_y,
                                width: float_w,
                                height: float_h,
                            });
                            offset = (offset + 24) % 120;
                        }
                    }
                }
                needs_recalc = true;
            }
            "cycle_layout" => {
                let current_ws = self.active_window_id.as_ref().and_then(|wid| {
                    self.engine
                        .current_windows
                        .get(wid)
                        .map(|w| w.workspace_id.clone())
                });

                let new_layout_name;
                if let Some(ws_id) = current_ws {
                    let current = self
                        .engine
                        .config
                        .workspace_layouts
                        .get(&ws_id)
                        .cloned()
                        .unwrap_or_else(|| self.engine.config.layout_type.clone());

                    let next = match current.as_str() {
                        "raven" => "tall",
                        "tall" => "monocle",
                        "monocle" => "strict_dwindle",
                        "strict_dwindle" => "inverted_strict_dwindle",
                        "inverted_strict_dwindle" => "divisor",
                        "divisor" => "raven",
                        _ => "raven",
                    }
                    .to_string();

                    new_layout_name = next.clone();
                    self.engine
                        .config
                        .workspace_layouts
                        .insert(ws_id.clone(), next.clone());
                    info!(
                        "[CONTROLLER] Layout de Workspace {} cambiado a: {}",
                        ws_id, next
                    );
                } else {
                    self.engine.config.layout_type = match self.engine.config.layout_type.as_str() {
                        "raven" => "tall".to_string(),
                        "tall" => "monocle".to_string(),
                        "monocle" => "strict_dwindle".to_string(),
                        "strict_dwindle" => "inverted_strict_dwindle".to_string(),
                        "inverted_strict_dwindle" => "divisor".to_string(),
                        "divisor" => "raven".to_string(),
                        _ => "raven".to_string(),
                    };
                    new_layout_name = self.engine.config.layout_type.clone();
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
                self.engine.config.nmaster = (self.engine.config.nmaster + 1).min(8);
                needs_recalc = true;
                config_changed = true;
            }
            "decrement_nmaster" => {
                self.engine.config.nmaster = self.engine.config.nmaster.saturating_sub(1).max(1);
                needs_recalc = true;
                config_changed = true;
            }
            "increase_ratio" => {
                self.engine.config.master_ratio =
                    f32::min(0.85, self.engine.config.master_ratio + 0.05);
                needs_recalc = true;
                config_changed = true;
            }
            "decrease_ratio" => {
                self.engine.config.master_ratio =
                    f32::max(0.30, self.engine.config.master_ratio - 0.05);
                needs_recalc = true;
                config_changed = true;
            }
            "resize_width_inc" | "resize_width_dec" | "resize_height_inc" | "resize_height_dec" => {
                if let Some(ref wid) = self.active_window_id {
                    if let Some(win) = self.engine.current_windows.get_mut(wid) {
                        let delta = 0.05f32;
                        match action.as_str() {
                            "resize_width_inc" => {
                                let cur = win.custom_w_ratio.unwrap_or(1.0);
                                win.custom_w_ratio = Some((cur + delta).min(3.0));
                            }
                            "resize_width_dec" => {
                                let cur = win.custom_w_ratio.unwrap_or(1.0);
                                win.custom_w_ratio = Some((cur - delta).max(0.2));
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
                self.send_osd_notification(
                    "Organización de Mosaico",
                    if action == "swap_next" {
                        "Intercambiar Ventana (Siguiente)"
                    } else {
                        "Intercambiar Ventana (Anterior)"
                    },
                );
                commands.push(RavenAction::RequestSync);

                let mut active_windows: Vec<_> = windows
                    .into_iter()
                    .filter(|w| !w.is_floating && !w.is_minimized && !w.is_pip)
                    .collect();

                if active_windows.len() > 1 {
                    active_windows.sort_by_key(|w| {
                        let is_strict = w.min_w > 0 || w.min_h > 0;
                        let pos = self
                            .engine
                            .spatial_order
                            .iter()
                            .position(|id| id == &w.window_id)
                            .unwrap_or(usize::MAX);
                        (!is_strict, pos)
                    });

                    let effective_active_id = self.active_window_id.clone().or_else(|| {
                        self.engine
                            .window_history
                            .back()
                            .cloned()
                            .or_else(|| active_windows.first().map(|w| w.window_id.clone()))
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
}
