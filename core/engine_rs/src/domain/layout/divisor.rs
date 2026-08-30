//! # Estrategia de Disposición Divisor (Columnas Equitativas)
//!
//! Implementa una distribución horizontal donde la pantalla se divide en $N$ columnas
//! de ancho exactamente igual. Cada ventana ocupa una columna completa de arriba a abajo.

use super::{apply_gaps, LayoutStrategy};
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

        // 3. Dividir el ancho del contenedor en partes iguales entre el número de ventanas
        let num_windows = active_windows.len() as i32;
        let w_slot = container.width / num_windows;

        // 4. Asignar la geometría a cada ventana de izquierda a derecha
        for (i, win) in active_windows.iter().enumerate() {
            let i = i as i32;
            
            // La última ventana absorbe cualquier sobrante por división entera para no dejar huecos
            let slot_width = if i == num_windows - 1 {
                container.width - (i * w_slot)
            } else {
                w_slot
            };

            let rect = Rect {
                x: container.x + (i * w_slot),
                y: container.y,
                width: slot_width,
                height: container.height,
            };

            // Aplicar espaciado (gaps) y guardar el resultado
            layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
        }

        (layout_map, evicted_windows)
    }
}
