//! # Algoritmo de Columnas Equitativas (`DivisorStrategy`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Implementa una partición horizontal donde la pantalla se divide en $N$ columnas
//! paralelas de ancho equitativo, ocupando la altura total disponible.

use super::{apply_gaps, distribute_weighted_sizes, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Estrategia de layout "Divisor": asigna anchos idénticos a todas las ventanas en paralelo.
pub struct DivisorStrategy;

impl LayoutStrategy for DivisorStrategy {
    /// Calcula las geometrías de las ventanas dividiendo la pantalla en columnas verticales equitativas.
    ///
    /// # Parámetros
    /// - `windows`: Lista de nodos de ventana presentes en el workspace.
    /// - `screen_rect`: Área total de la pantalla o espacio útil asignado.
    /// - `_nmaster`: No utilizado en esta estrategia (sin área principal reservada).
    /// - `_master_ratio`: No utilizado en esta estrategia (división equitativa $1/N$).
    /// - `default_gaps`: Espaciado interno en píxeles entre ventanas y bordes.
    /// - `_active_window_id`: Identificador de la ventana enfocada.
    ///
    /// # Retorno
    /// Una tupla `(HashMap<WindowId, Rect>, Vec<EvictedWindowId>)` con la posición de cada ventana.
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        _nmaster: usize,
        _master_ratio: f32,
        default_gaps: i32,
        _active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        // 1. Filtrar solo ventanas que participan en el tiling (excluye flotantes y minimizadas)
        let active_windows: Vec<&WindowNode> = windows
            .iter()
            .filter(|w| !w.is_floating && !w.is_minimized)
            .collect();

        // Si no hay ventanas activas, retornar mapa vacío
        if active_windows.is_empty() {
            return (HashMap::new(), Vec::new());
        }

        let mut layout_map = HashMap::with_capacity(active_windows.len());
        let evicted_windows = Vec::new();

        // 2. Calcular el contenedor de trabajo considerando el espaciado interno (gap)
        let half_g = default_gaps / 2;
        let container = Rect {
            x: screen_rect.x + half_g,
            y: screen_rect.y + half_g,
            width: std::cmp::max(1, screen_rect.width - default_gaps),
            height: std::cmp::max(1, screen_rect.height - default_gaps),
        };

        // 3. Dividir el ancho del contenedor respetando requerimientos mínimos y ratios horizontales
        let mins: Vec<i32> = active_windows.iter().map(|w| std::cmp::max(w.min_w, 80)).collect();
        let weights: Vec<Option<f32>> = active_windows.iter().map(|w| w.custom_w_ratio).collect();
        let widths = distribute_weighted_sizes(container.width, &mins, &weights);

        // 4. Asignar la geometría a cada ventana de izquierda a derecha
        let mut current_x = container.x;
        for (i, win) in active_windows.iter().enumerate() {
            let rect = Rect {
                x: current_x,
                y: container.y,
                width: widths[i],
                height: container.height,
            };

            // Aplicar espaciado (gaps) y guardar el resultado
            layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
            current_x += widths[i];
        }

        (layout_map, evicted_windows)
    }
}
