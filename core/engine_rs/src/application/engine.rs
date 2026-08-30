use std::collections::{HashMap, HashSet, VecDeque};

use crate::domain::error::RavenError;
use crate::domain::geometry::{Rect, WindowNode};
use crate::domain::layout::calculate_global_topology;
use crate::infrastructure::config::RavenConfig;

/// El núcleo lógico del motor de mosaico (Tiling Engine).
///
/// Mantiene la configuración activa, el estado global de habilitación y
/// el historial cronológico de ventanas utilizado en la evicción (eviction) FIFO.
pub struct TilingEngine {
    /// Configuración activa del gestor de ventanas Raven.
    pub config: RavenConfig,
    /// Bandera que indica si la disposición de ventanas en mosaico (tiling) está activada.
    pub is_tiling_enabled: bool,
    /// Historial cronológico de ventanas utilizado para la evicción (eviction) FIFO.
    pub window_history: VecDeque<String>,
    /// Pila dinámica de identificadores de ventanas en modo flotante temporal (Quick Peek).
    pub dynamic_floating_windows: HashSet<String>,
    /// Mapa de las áreas de trabajo (workspaces) activas y sus geometrías útiles.
    pub current_workspaces: HashMap<String, Rect>,
    /// Mapa de todas las ventanas (windows) actualmente rastreadas por el motor.
    pub current_windows: HashMap<String, WindowNode>,
}

impl TilingEngine {
    /// Crea una nueva instancia de `TilingEngine` a partir de un objeto de configuración.
    ///
    /// # Parámetros
    /// * `config` - Configuración inicial de Raven.
    pub fn new(config: RavenConfig) -> Self {
        TilingEngine {
            is_tiling_enabled: config.tiling_enabled_on_startup,
            config,
            window_history: VecDeque::new(),
            dynamic_floating_windows: HashSet::new(),
            current_workspaces: HashMap::new(),
            current_windows: HashMap::new(),
        }
    }

    /// Alterna el estado operativo de activación del motor de mosaico.
    ///
    /// # Retorno
    /// El nuevo estado de habilitación del motor de mosaico.
    pub fn toggle_tiling(&mut self) -> bool {
        self.is_tiling_enabled = !self.is_tiling_enabled;
        self.is_tiling_enabled
    }

    /// Calcula la nueva disposición de ventanas basándose en el estado del dominio.
    ///
    /// Ejecuta el algoritmo de topología global para determinar las nuevas
    /// posiciones y dimensiones de cada ventana según la configuración actual.
    ///
    /// # Parámetros
    /// * `workspaces` - Mapa de áreas de trabajo (workspaces) disponibles.
    /// * `windows` - Listado de nodos de ventana (windows) a organizar.
    ///
    /// # Retorno
    /// Un `Result` con una tupla que contiene el mapa de geometrías calculadas y la lista de ventanas desalojadas (evicted), o un error de dominio.
    pub fn calculate_from_payload(
        &self,
        workspaces: &HashMap<String, Rect>,
        windows: &[WindowNode],
        active_window_id: Option<String>,
    ) -> Result<(HashMap<String, Rect>, Vec<String>), RavenError> {
        if !self.is_tiling_enabled || windows.is_empty() {
            return Ok((HashMap::new(), Vec::new()));
        }

        // Evaluación y Arbitraje de Ventanas en Rust (Jerarquía de Precedencia Estricta)
        let effective_windows: Vec<WindowNode> = windows
            .iter()
            .map(|w| {
                let mut cloned = w.clone();
                let class_lower = cloned.resource_class.to_lowercase();
                let caption_lower = cloned.caption.to_lowercase();

                // 1. PRIORIDAD MÁXIMA: Pila Flotante Dinámica (Quick Peek)
                if self.dynamic_floating_windows.contains(&cloned.window_id) {
                    cloned.is_floating = true;
                    cloned.is_pip = false;
                    tracing::debug!(
                        "[RUST ENGINE] Ventana {} ({}) fijada en Quick Peek Flotante",
                        cloned.window_id,
                        cloned.resource_class
                    );
                    return cloned;
                }

                // 2. Reglas de Usuario configuradas en GUI
                for rule in &self.config.window_rules {
                    if !rule.class.is_empty() && class_lower.contains(&rule.class.to_lowercase()) {
                        if rule.pip {
                            cloned.is_pip = true;
                            cloned.is_floating = false;
                            tracing::info!(
                                "[RUST RULE] Ventana {} ({}) catalogada como PiP por regla de usuario",
                                cloned.window_id,
                                cloned.resource_class
                            );
                        } else if rule.action == "float" {
                            cloned.is_floating = true;
                            cloned.is_pip = false;
                            tracing::info!(
                                "[RUST RULE] Ventana {} ({}) catalogada como Flotante por regla de usuario",
                                cloned.window_id,
                                cloned.resource_class
                            );
                        }
                        return cloned;
                    }
                }

                // 3. Heurística Nativa de Detección PiP por Título (Multilenguaje)
                let is_pip_caption = caption_lower.contains("picture-in-picture")
                    || caption_lower.contains("picture in picture")
                    || caption_lower.contains("imagen en imagen")
                    || caption_lower.contains("pantalla en pantalla")
                    || caption_lower.contains("reproductor en miniatura")
                    || caption_lower.contains("incrustation")
                    || caption_lower.contains("bild-in-bild")
                    || caption_lower.contains("imagem em imagem")
                    || caption_lower == "pip";

                if is_pip_caption {
                    cloned.is_pip = true;
                    cloned.is_floating = false;
                    tracing::info!(
                        "[RUST DETECT] Ventana {} ({}) detectada como PiP por título '{}'",
                        cloned.window_id,
                        cloned.resource_class,
                        cloned.caption
                    );
                    return cloned;
                }

                // 4. Heurística Nativa de Herramientas y Micro-Widgets Flotantes por Clase o Título
                if !class_lower.is_empty() || !caption_lower.is_empty() {
                    let is_known_float_tool = class_lower.contains("colorchooser")
                        || class_lower.contains("colorpicker")
                        || class_lower.contains("gcolor")
                        || class_lower.contains("eyedropper")
                        || class_lower.contains("spectacle")
                        || class_lower.contains("klipper")
                        || class_lower.contains("polkit")
                        || class_lower.contains("pinentry")
                        || class_lower.contains("zenity")
                        || class_lower.contains("kdialog")
                        || class_lower == "raven_gui"
                        || class_lower == "raven-gui"
                        || class_lower == "raven config"
                        || caption_lower.contains("color picker")
                        || caption_lower.contains("selector de color")
                        || caption_lower.contains("mini player")
                        || caption_lower.contains("miniplayer")
                        || caption_lower.contains("zuno widget")
                        || caption_lower.contains("now playing widget")
                        || caption_lower.contains("raven control center")
                        || caption_lower.contains("raven tiling emulator — control center");

                    if is_known_float_tool {
                        cloned.is_floating = true;
                        cloned.is_pip = false;
                        return cloned;
                    }
                }

                // 5. Heurística por micro-dimensiones con clase no vacía (ej. widgets dedicados de Zuno)
                if !class_lower.is_empty() && (class_lower.contains("zuno") || class_lower.contains("electron") || class_lower.contains("widget")) {
                    if cloned.geometry.width > 0 && cloned.geometry.height > 0 && cloned.geometry.width < 380 && cloned.geometry.height < 320 {
                        cloned.is_floating = true;
                        cloned.is_pip = false;
                        return cloned;
                    }
                }

                cloned
            })
            .collect();

        let config_clone = self.config.clone();

        let (layout_map, evicted_windows) = calculate_global_topology(
            &effective_windows,
            workspaces,
            config_clone.nmaster,
            config_clone.master_ratio,
            config_clone.default_gaps,
            &config_clone.pip_position,
            &config_clone.layout_type,
            &config_clone.workspace_layouts,
            active_window_id,
            config_clone.pip_size_ratio,
        );
        Ok((layout_map, evicted_windows))
    }

    /// Sincroniza el historial cronológico interno con el estado del compositor.
    ///
    /// Elimina de la memoria aquellas ventanas destruidas o cerradas en el compositor
    /// y registra las nuevas ventanas visibles (no flotantes) al final de la cola (queue) FIFO.
    ///
    /// # Parámetros
    /// * `current_windows` - Estado actual de ventanas activas reportadas por el compositor.
    pub fn update_history(&mut self, current_windows: &[WindowNode]) -> bool {
        let initial_len = self.window_history.len();
        let initial_order = self.window_history.clone();

        self.window_history
            .retain(|id| current_windows.iter().any(|w| &w.window_id == id));

        // Limpiar de la pila flotante aquellas ventanas que hayan sido cerradas
        self.dynamic_floating_windows
            .retain(|id| current_windows.iter().any(|w| &w.window_id == id));

        for win in current_windows {
            let is_dyn_float = self.dynamic_floating_windows.contains(&win.window_id);
            if !self.window_history.contains(&win.window_id) && !win.is_floating && !is_dyn_float {
                self.window_history.push_back(win.window_id.clone());
            }
        }

        initial_len != self.window_history.len() || initial_order != self.window_history
    }
}
