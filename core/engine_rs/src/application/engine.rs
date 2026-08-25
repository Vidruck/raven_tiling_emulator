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

        // Marcar dinámicamente como flotantes las ventanas en la pila Quick Peek y apagar PiP para ellas
        let effective_windows: Vec<WindowNode> = windows
            .iter()
            .map(|w| {
                if self.dynamic_floating_windows.contains(&w.window_id) {
                    let mut cloned = w.clone();
                    cloned.is_floating = true;
                    cloned.is_pip = false; // Jamás tratar como PiP una ventana de la pila flotante de Rust
                    cloned
                } else {
                    w.clone()
                }
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

        if initial_len != self.window_history.len() || initial_order != self.window_history {
            true
        } else {
            false
        }
    }
}
