//! # Estrategia de Disposición Tall (Master + Stack Vertical)
//!
//! Implementa la distribución clásica "Tall": la pantalla se divide verticalmente en dos columnas.
//! - La columna izquierda contiene las ventanas maestras (`nmaster`).
//! - La columna derecha contiene el apilamiento vertical (stack) con las demás ventanas.

use super::{apply_gaps, distribute_weighted_sizes, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Estrategia de layout "Tall": columna maestra a la izquierda y pila vertical a la derecha.
pub struct TallStrategy;

impl LayoutStrategy for TallStrategy {
    /// Calcula las posiciones de las ventanas en columnas Master y Stack.
    ///
    /// # Parámetros
    /// - `windows`: Lista de ventanas no flotantes ni minimizadas.
    /// - `screen_rect`: Área útil de la pantalla.
    /// - `nmaster`: Número de ventanas reservadas para el área principal (Master).
    /// - `master_ratio`: Proporción de ancho asignada al área principal (ej. 0.50 o 0.60).
    /// - `default_gaps`: Espaciado interno en píxeles.
    /// - `_active_window_id`: Identificador de la ventana con foco.
    ///
    /// # Retorno
    /// Tupla `(HashMap<WindowId, Rect>, Vec<EvictedWindowId>)`.
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        nmaster: usize,
        master_ratio: f32,
        default_gaps: i32,
        _active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        // 1. Filtrar ventanas activas en el mosaico
        let active_windows: Vec<&WindowNode> = windows
            .iter()
            .filter(|w| !w.is_floating && !w.is_minimized)
            .collect();

        if active_windows.is_empty() {
            return (HashMap::new(), Vec::new());
        }

        let mut layout_map = HashMap::with_capacity(active_windows.len());
        let evicted_windows = Vec::new();

        // 2. Definir el contenedor principal respetando gaps periféricos
        let half_g = default_gaps / 2;
        let container = Rect {
            x: screen_rect.x + half_g,
            y: screen_rect.y + half_g,
            width: std::cmp::max(1, screen_rect.width - default_gaps),
            height: std::cmp::max(1, screen_rect.height - default_gaps),
        };

        // 3. Caso A: Si el número de ventanas es menor o igual a nmaster, todas ocupan el área completa en columnas
        if active_windows.len() <= nmaster {
            let w_slot = container.width / active_windows.len() as i32;
            for (i, win) in active_windows.iter().enumerate() {
                let rect = Rect {
                    x: container.x + (i as i32 * w_slot),
                    y: container.y,
                    width: if i == active_windows.len() - 1 {
                        container.width - (i as i32 * w_slot)
                    } else {
                        w_slot
                    },
                    height: container.height,
                };
                layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
            }
        } else {
            // 4. Caso B: Hay ventanas suficientes para crear la columna Master (izq) y la columna Stack (der)
            
            // Límite defensivo: Ninguna ventana puede exigir un min_w mayor al 40% del contenedor útil
            let max_allowed_min_w = (container.width as f32 * 0.40) as i32;
            let mut master_w = (container.width as f32 * master_ratio) as i32;

            // Restricción dinámicas para el área Master
            let mut max_master_min_w = 0;
            for win in active_windows.iter().take(nmaster) {
                let clamped_min = win.min_w.min(max_allowed_min_w);
                if clamped_min > max_master_min_w {
                    max_master_min_w = clamped_min;
                }
            }
            if master_w < max_master_min_w {
                master_w = max_master_min_w;
            }

            // Restricciones dinámicas para la columna Stack
            let stack_count = active_windows.len() - nmaster;
            let mut stack_w = container.width - master_w;

            let mut max_stack_min_w = 0;
            for i in 0..stack_count {
                let win = &active_windows[nmaster + i];
                let clamped_min = win.min_w.min(max_allowed_min_w);
                if clamped_min > max_stack_min_w {
                    max_stack_min_w = clamped_min;
                }
            }
            if stack_w < max_stack_min_w {
                stack_w = max_stack_min_w;
                master_w = std::cmp::max(1, container.width - stack_w);
            }

            // Garantía defensiva: La columna secundaria (stack) conserva siempre un mínimo del 20% del contenedor
            let min_stack_w = (container.width as f32 * 0.20) as i32;
            if stack_w < min_stack_w {
                stack_w = min_stack_w;
                master_w = std::cmp::max(1, container.width - stack_w);
            }

            // 5. Apilar ventanas de la columna Master a la izquierda
            let master_wins = &active_windows[..nmaster];
            let master_mins: Vec<i32> = master_wins.iter().map(|w| std::cmp::max(w.min_h, 80)).collect();
            let master_weights: Vec<Option<f32>> = master_wins.iter().map(|w| w.custom_h_ratio).collect();
            let master_heights = distribute_weighted_sizes(container.height, &master_mins, &master_weights);
            let mut current_y = container.y;
            for (i, win) in master_wins.iter().enumerate() {
                let rect = Rect {
                    x: container.x,
                    y: current_y,
                    width: master_w,
                    height: master_heights[i],
                };
                layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
                current_y += master_heights[i];
            }

            // 6. Apilar ventanas de la columna Stack a la derecha
            let stack_wins = &active_windows[nmaster..];
            let stack_mins: Vec<i32> = stack_wins.iter().map(|w| std::cmp::max(w.min_h, 80)).collect();
            let stack_weights: Vec<Option<f32>> = stack_wins.iter().map(|w| w.custom_h_ratio).collect();
            let stack_heights = distribute_weighted_sizes(container.height, &stack_mins, &stack_weights);
            let mut current_y = container.y;
            for (i, win) in stack_wins.iter().enumerate() {
                let rect = Rect {
                    x: container.x + master_w,
                    y: current_y,
                    width: stack_w,
                    height: stack_heights[i],
                };
                layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
                current_y += stack_heights[i];
            }
        }

        (layout_map, evicted_windows)
    }
}
