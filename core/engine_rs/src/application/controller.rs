use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::application::engine::TilingEngine;
use crate::domain::action::RavenAction;
use crate::domain::error::RavenError;
use crate::domain::geometry::{Rect, WindowNode};

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

/// Registra una geometría de ventana que fue ordenada aplicar de forma explícita.
struct CommandedGeometry {
    /// Rectángulo de geometría de la ventana ordenado.
    #[allow(dead_code)]
    rect: Rect,
    /// Marca de tiempo del momento en que se emitió el comando (timestamp).
    #[allow(dead_code)]
    timestamp: u64,
}

/// Orquestador principal de la lógica de Raven - v2.8 Master-Stack con soporte de intercambio de ventanas.
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
    /// Historial de geometrías explícitamente ordenadas por el motor.
    commanded_geometries: HashMap<String, CommandedGeometry>,
    /// Migraciones de ventanas que se encuentran en tránsito asíncrono.
    #[allow(dead_code)]
    pending_migrations: HashMap<String, String>,
    /// Ordenamiento de visibilidad de las ventanas de mosaico activas.
    #[allow(dead_code)]
    visible_windows_order: Vec<String>,
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
    pub fn new(engine: TilingEngine) -> Self {
        RavenController {
            engine,
            last_known_layout: HashMap::new(),
            flap_registry: HashMap::new(),
            commanded_geometries: HashMap::new(),
            pending_migrations: HashMap::new(),
            visible_windows_order: Vec::new(),
            active_window_id: None,
            last_active_window_count: 0,
        }
    }

    /// Restablece todo el estado interno y registros temporales del controlador.
    pub fn reset_state(&mut self) {
        self.last_known_layout.clear();
        self.flap_registry.clear();
        self.commanded_geometries.clear();
        self.pending_migrations.clear();
        self.visible_windows_order.clear();
        self.engine.current_workspaces.clear();
        self.engine.current_windows.clear();
        self.active_window_id = None;
        self.last_active_window_count = 0;
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

        // Comprobamos si la ventana cambió verdaderamente de geometría (geometry) o de estado de minimización (minimized)
        let has_changed = match tracker.last_rect {
            Some(r) => r != win.geometry || tracker.last_minimized != win.is_minimized,
            None => true,
        };

        // Guardamos el estado para futuras comparaciones
        tracker.last_rect = Some(win.geometry);
        tracker.last_minimized = win.is_minimized;

        if tracker.is_penalized {
            if now - tracker.last_toggle_time > 400 {
                tracker.is_penalized = false;
                tracker.toggle_count = 0;
            } else {
                return true;
            }
        }

        // Solo incrementamos el contador de oscilación (toggle count) si hubo un cambio real
        if has_changed {
            if now - tracker.last_toggle_time < 400 {
                tracker.toggle_count += 1;
                if tracker.toggle_count > 8 {
                    println!(
                        "[DEFENSA] Cortocircuito de oscilación rápida (flapping) activo para: {}.",
                        win.window_id
                    );
                    tracker.is_penalized = true;
                    tracker.last_toggle_time = now;
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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        self.engine.current_workspaces = workspaces.clone();
        self.engine.current_windows = windows
            .iter()
            .map(|w| (w.window_id.clone(), w.clone()))
            .collect();
        self.engine.update_history(&windows);

        let mut healthy_windows = Vec::new();
        for win in windows {
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
        // Si el número de ventanas cambia (se agregó o eliminó de la composición),
        // reiniciamos el ratio maestro (master ratio) a 0.5 (50-50) para evitar desorden.
        if active_count != self.last_active_window_count {
            self.engine.config.master_ratio = 0.5;
            self.last_active_window_count = active_count;
        }

        windows.sort_by_key(|w| {
            let is_strict = w.min_w > 0 || w.min_h > 0;
            let pos = self
                .engine
                .window_history
                .iter()
                .position(|id| id == &w.window_id)
                .unwrap_or(usize::MAX);
            (!is_strict, std::cmp::Reverse(pos))
        });

        let (new_layout, evicted_windows) = self.engine.calculate_from_payload(
            workspaces.clone(),
            windows.clone(),
            self.active_window_id.clone(),
        )?;
        let mut commands = Vec::new();

        for (wid, rect) in &new_layout {
            let needs_move = match self.last_known_layout.get(wid) {
                Some(old_rect) => old_rect != rect,
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

            self.commanded_geometries.insert(
                wid.clone(),
                CommandedGeometry {
                    rect: *rect,
                    timestamp: now,
                },
            );
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
                        println!(
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



                println!(
                    "[TOPOLOGY] Desalojo BSP sin escape para {}. Minimizando.",
                    evicted_id
                );
                commands.push(RavenAction::MinimizeWindow {
                    window_id: evicted_id.clone(),
                });
            }
        }

        self.last_known_layout = new_layout;
        Ok(commands)
    }

    /// Procesa de forma incremental la actualización de geometría o estado de una sola ventana.
    ///
    /// # Parámetros
    /// * `win` - Nodo de ventana con los cambios recientes.
    ///
    /// # Retorno
    /// Vector de comandos resultantes de evaluar el cambio en el motor.
    pub fn handle_delta_change(&mut self, win: WindowNode) -> Result<Vec<RavenAction>, RavenError> {
        self.engine
            .current_windows
            .insert(win.window_id.clone(), win);
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
    /// * `windows` - Listado de ventanas reportadas por el cliente D-Bus.
    /// * `_workspaces` - Mapa de geometrías de las áreas de trabajo.
    /// * `active_window_id` - Identificador de la ventana activa al momento del atajo.
    ///
    /// # Retorno
    /// Una tupla que contiene si se requiere recálculo y la lista de comandos a despachar.
    pub fn handle_shortcut(
        &mut self,
        action: String,
        payload: i32,
        windows: Vec<WindowNode>,
        _workspaces: HashMap<String, Rect>,
        active_window_id: Option<String>,
    ) -> Result<(bool, Vec<RavenAction>), RavenError> {
        self.active_window_id = active_window_id.clone();
        self.engine.update_history(&windows);
        let mut needs_recalc = false;
        let mut commands = Vec::new();

        match action.as_str() {
            "toggle_tiling" => {
                self.engine.toggle_tiling();
                needs_recalc = true;
            }
            "increment_gaps" => {
                self.engine.config.default_gaps =
                    std::cmp::max(0, self.engine.config.default_gaps + payload);
                needs_recalc = true;
            }
            "increase_ratio" => {
                self.engine.config.master_ratio =
                    f32::min(0.85, self.engine.config.master_ratio + 0.05);
                needs_recalc = true;
            }
            "decrease_ratio" => {
                self.engine.config.master_ratio =
                    f32::max(0.15, self.engine.config.master_ratio - 0.05);
                needs_recalc = true;
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
                        if is_desktop {
                            let mut desktops = Vec::new();
                            for key in _workspaces.keys() {
                                let mut parts = key.split("||");
                                if let (Some(out), Some(desk)) = (parts.next(), parts.next()) {
                                    if out == win_node.output {
                                        let desk_str = desk.to_string();
                                        if !desktops.contains(&desk_str) {
                                            desktops.push(desk_str);
                                        }
                                    }
                                }
                            }
                            let current_desk =
                                win_node.desktops.first().cloned().unwrap_or_default();
                            if let Some(target_desk) = desktops.iter().find(|&d| d != &current_desk)
                            {
                                commands.push(RavenAction::MigrateToDesktop {
                                    window_id: wid.clone(),
                                    target_desktop: target_desk.clone(),
                                });
                            }
                        } else {
                            let mut outputs = Vec::new();
                            for key in _workspaces.keys() {
                                if let Some(out) = key.split("||").next() {
                                    let out_str = out.to_string();
                                    if !outputs.contains(&out_str) {
                                        outputs.push(out_str);
                                    }
                                }
                            }
                            if let Some(target_out) =
                                outputs.iter().find(|&o| o != &win_node.output)
                            {
                                commands.push(RavenAction::MigrateToOutput {
                                    window_id: wid.clone(),
                                    target_output: target_out.clone(),
                                });
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        Ok((needs_recalc, commands))
    }
}
