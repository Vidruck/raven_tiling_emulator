use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

use crate::application::engine::TilingEngine;
use crate::domain::action::RavenAction;
use crate::domain::error::RavenError;
use crate::domain::geometry::{Rect, WindowNode};
use crate::domain::saturation::{calculate_screen_capacity, SaturationState};

/// Rastreador de oscilación rápida (flapping) de ventana.
///
/// Evita que ventanas problemáticas entren en bucles infinitos de actualización
/// debido a conflictos entre su tamaño mínimo y el algoritmo de mosaico (tiling).
struct FlapTracker {
    /// Marca de tiempo del último cambio detectado (last toggle time).
    last_toggle_time: u64,
    /// Conteo acumulado de eventos de oscilación detectados en cascada (toggle count).
    toggle_count: u64,
    /// Indica si la ventana está bajo penalización o en cuarentena (penalized).
    is_penalized: bool,
    /// Última geometría rectangular (rect) conocida para comparar cambios reales.
    last_rect: Option<Rect>,
    /// Último estado de minimización conocido (minimized).
    last_minimized: bool,
}

/// Orquestador principal de la lógica de Raven - v2.9 Master-Stack con soporte de intercambio de ventanas.
///
/// Administra el ciclo de vida del motor de mosaico (tiling engine), coordina
/// la sincronización de estados del compositor y detecta situaciones de inestabilidad.
pub struct RavenController {
    /// Motor de mosaico central de la aplicación.
    engine: TilingEngine,
    /// Registro histórico del último diseño (layout) calculado.
    last_known_layout: HashMap<String, Rect>,
    /// Registro de oscilaciones rápidas (flapping) por ventana.
    flap_registry: HashMap<String, FlapTracker>,
    /// Identificador de la ventana activa enfocada (focused window).
    pub active_window_id: Option<String>,
    /// Cantidad de ventanas activas en el último cambio de estado.
    last_active_window_count: usize,
}

impl RavenController {
    /// Crea una nueva instancia de `RavenController`.
    ///
    /// # Parámetros
    /// * `engine` - Instancia del motor de mosaico (tiling engine) a utilizar.
    pub fn new(mut engine: TilingEngine) -> Self {
        engine.window_history = Self::load_window_history();
        RavenController {
            engine,
            last_known_layout: HashMap::new(),
            flap_registry: HashMap::new(),
            active_window_id: None,
            last_active_window_count: 0,
        }
    }

    /// Retorna una referencia a la configuración actual del motor.
    pub fn get_config(&self) -> &crate::infrastructure::config::RavenConfig {
        &self.engine.config
    }

    /// Restablece todo el estado interno y registros temporales del controlador.
    pub fn reset_state(&mut self) {
        self.last_known_layout.clear();
        self.flap_registry.clear();
        self.engine.current_workspaces.clear();
        self.engine.current_windows.clear();
        self.active_window_id = None;
        self.last_active_window_count = 0;
    }

    fn load_window_history() -> std::collections::VecDeque<String> {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("~"));
        let mut path = std::path::PathBuf::from(home);
        path.push(".cache");
        path.push("raven");
        path.push("history.json");

        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(history) = serde_json::from_str(&content) {
                return history;
            }
        }
        std::collections::VecDeque::new()
    }

    fn persist_window_history(history: std::collections::VecDeque<String>) {
        tokio::spawn(async move {
            let home = std::env::var("HOME").unwrap_or_else(|_| String::from("~"));
            let mut path = std::path::PathBuf::from(home);
            path.push(".cache");
            path.push("raven");
            let _ = tokio::fs::create_dir_all(&path).await;
            path.push("history.json");
            
            if let Ok(json) = serde_json::to_string(&history) {
                let _ = tokio::fs::write(path, json).await;
            }
        });
    }

    /// Determina si el motor de mosaico (tiling engine) está operativo.
    pub fn is_tiling_enabled(&self) -> bool {
        self.engine.is_tiling_enabled
    }

    /// Comprueba si una ventana está oscilando rápidamente (flapping) y aplica penalizaciones.
    ///
    /// # Parámetros
    /// * `win` - Nodo de ventana (window node) a evaluar.
    ///
    /// # Retorno
    /// Verdadero (true) si la ventana está penalizada u oscilando; falso (false) de lo contrario.
    fn is_window_flapping(&mut self, win: &WindowNode) -> bool {
        if win.strict_birth {
            return false; // Las apps rebeldes en cuarentena tienen pase libre para intentar acatar la orden
        }
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let tracker = self
            .flap_registry
            .entry(win.window_id.clone())
            .or_insert(FlapTracker {
                last_toggle_time: now,
                toggle_count: 0,
                is_penalized: false,
                last_rect: None,
                last_minimized: win.is_minimized,
            });

        if win.is_minimized != tracker.last_minimized {
            tracker.last_minimized = win.is_minimized;
            tracker.toggle_count = 0;
            return false;
        }

        if tracker.is_penalized {
            if now - tracker.last_toggle_time > 8000 {
                tracker.is_penalized = false;
                tracker.toggle_count = 0;
                warn!(
                    "[Controller] Ventana {} liberada de penalización.",
                    win.window_id
                );
            } else {
                return true;
            }
        }

        let is_jumping = match tracker.last_rect {
            Some(old_r) => {
                let dx = (old_r.x - win.geometry.x).abs();
                let dy = (old_r.y - win.geometry.y).abs();
                let dw = (old_r.width - win.geometry.width).abs();
                let dh = (old_r.height - win.geometry.height).abs();
                dx > 10 || dy > 10 || dw > 10 || dh > 10
            }
            None => false,
        };

        tracker.last_rect = Some(win.geometry);

        if is_jumping {
            if now - tracker.last_toggle_time < 300 {
                tracker.toggle_count += 1;
                if tracker.toggle_count >= 5 {
                    tracker.is_penalized = true;
                    warn!(
                        "[Controller] Ventana {} penalizada por oscilación (flap detectado).",
                        win.window_id
                    );
                    return true;
                }
            } else {
                tracker.toggle_count = 1;
            }
            tracker.last_toggle_time = now;
        }

        false
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

        let history_changed = self.engine.update_history(&windows);
        if history_changed {
            Self::persist_window_history(self.engine.window_history.clone());
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

        windows.sort_by_key(|w| {
            self.engine
                .window_history
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
                if win_node.geometry != *rect {
                    win_rect_differs = true;
                }
            }

            let needs_move = match self.last_known_layout.get(wid) {
                Some(old_rect) => old_rect != rect || win_rect_differs,
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

                if let Some(win_node) = windows.iter().find(|w| &w.window_id == wid) {
                    if win_node.strict_birth {
                        commands.push(RavenAction::RequestFeedback {
                            window_id: wid.clone(),
                        });
                    }
                }
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
                        ws_id, ws_active, cap.cmax
                    );
                }
                SaturationState::Fluid => {}
            }
        }

        self.last_known_layout = new_layout;
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
        let workspaces = self.engine.current_workspaces.clone();
        let windows: Vec<WindowNode> = self.engine.current_windows.values().cloned().collect();
        self.handle_state_change(workspaces, windows)
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
        topology: &crate::infrastructure::dbus::KWinTopology,
    ) -> Result<(bool, Vec<RavenAction>), RavenError> {
        self.active_window_id = active_window_id.clone();
        
        let windows: Vec<WindowNode> = self.engine.current_windows.values().cloned().collect();
        self.engine.update_history(&windows);
        let mut needs_recalc = false;
        let mut config_changed = false;
        let mut commands = Vec::new();

        match action.as_str() {
            "toggle_tiling" => {
                self.engine.toggle_tiling();
                needs_recalc = true;
            }
            "cycle_layout" => {
                self.engine.config.layout_type = match self.engine.config.layout_type.as_str() {
                    "dwindle" => "tall".to_string(),
                    "tall" => "monocle".to_string(),
                    "monocle" => "dwindle".to_string(),
                    _ => "dwindle".to_string(),
                };
                info!("[CONTROLLER] Layout cambiado a: {}", self.engine.config.layout_type);
                needs_recalc = true;
                config_changed = true;
            }
            "increment_gaps" => {
                self.engine.config.default_gaps =
                    std::cmp::max(0, self.engine.config.default_gaps + _payload);
                needs_recalc = true;
                config_changed = true;
            }
            // BUG-01: handlers de nmaster ahora implementados correctamente
            "increment_nmaster" => {
                self.engine.config.nmaster =
                    (self.engine.config.nmaster + 1).min(8);
                needs_recalc = true;
                config_changed = true;
            }
            "decrement_nmaster" => {
                self.engine.config.nmaster =
                    self.engine.config.nmaster.saturating_sub(1).max(1);
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
            "swap_next" | "swap_prev" => {
                let mut active_windows: Vec<_> = windows
                    .into_iter()
                    .filter(|w| !w.is_floating && !w.is_minimized && !w.is_pip)
                    .collect();

                if active_windows.len() > 1 {
                    active_windows.sort_by_key(|w| {
                        let is_strict = w.min_w > 0 || w.min_h > 0;
                        let pos = self
                            .engine
                            .window_history
                            .iter()
                            .position(|id| id == &w.window_id)
                            .unwrap_or(usize::MAX);
                        (!is_strict, std::cmp::Reverse(pos))
                    });

                    if let Some(ref active_id) = active_window_id {
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

                            let pos_active = self
                                .engine
                                .window_history
                                .iter()
                                .position(|id| id == active_id);
                            let pos_target = self
                                .engine
                                .window_history
                                .iter()
                                .position(|id| id == target_id);

                            if let (Some(p_act), Some(p_tar)) = (pos_active, pos_target) {
                                self.engine.window_history.swap(p_act, p_tar);
                                needs_recalc = true;
                            }
                        }
                    }
                }
            }
            "focus_next" | "focus_prev" => {
                let active_windows: Vec<_> = windows
                    .into_iter()
                    .filter(|w| !w.is_floating && !w.is_minimized && !w.is_pip)
                    .collect();

                if !active_windows.is_empty() {
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
            "migrate_active_to_screen"
            | "migrate_active_to_desktop"
            | "migrate_active_to_prev_screen"
            | "migrate_active_to_prev_desktop" => {
                if let Some(ref wid) = active_window_id {
                    if let Some(win_node) = windows.iter().find(|w| &w.window_id == wid) {
                        let is_desktop = action.contains("desktop");
                        let is_prev = action.contains("prev");
                        if is_desktop {
                            let desktops = &topology.desktops;
                            let current_desk = win_node.desktops.first().cloned().unwrap_or_default();
                            
                            if let Some(current_idx) = desktops.iter().position(|d| d == &current_desk) {
                                let target_idx = if is_prev {
                                    if current_idx == 0 { desktops.len() - 1 } else { current_idx - 1 }
                                } else {
                                    (current_idx + 1) % desktops.len()
                                };
                                
                                if let Some(target_desk) = desktops.get(target_idx) {
                                    commands.push(RavenAction::MigrateToDesktop {
                                        window_id: wid.clone(),
                                        target_desktop: target_desk.clone(),
                                    });
                                }
                            }
                        } else {
                            let outputs = &topology.outputs;
                            let current_out = &win_node.output;
                            
                            if let Some(current_idx) = outputs.iter().position(|o| o == current_out) {
                                let target_idx = if is_prev {
                                    if current_idx == 0 { outputs.len() - 1 } else { current_idx - 1 }
                                } else {
                                    (current_idx + 1) % outputs.len()
                                };
                                
                                if let Some(target_out) = outputs.get(target_idx) {
                                    commands.push(RavenAction::MigrateToOutput {
                                        window_id: wid.clone(),
                                        target_output: target_out.clone(),
                                    });
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
