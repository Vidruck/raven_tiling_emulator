//! # Algoritmo de Disposición Principal: Dwindle BSP Adaptativo (Aspect-Ratio Aware)
//!
//! Implementa la arquitectura de cuadrícula insignia de Raven Engine:
//! - Distribuye dinámicamente las ventanas entre un Área Central (Master), Paneles Laterales (Sidebars)
//!   y Paneles Inferiores (Bottom Panels).
//! - Adapta proporciones defensivas y distribuye espacios sobrantes respetando restricciones geométricas.

use super::{apply_gaps, distribute_weighted_sizes, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Estrategia principal "Dwindle BSP": disposición adaptativa multitramo.
pub struct DwindleBSPStrategy;

impl LayoutStrategy for DwindleBSPStrategy {
    /// Calcula la disposición espacial en mosaico para el workspace activo.
    ///
    /// # Parámetros
    /// - `windows`: Arreglo de nodos de ventana a organizar.
    /// - `screen_rect`: Geometría utilizable de la pantalla.
    /// - `_nmaster`: No utilizado (la distribución central es adaptativa).
    /// - `master_ratio`: Relación de tamaño del área central frente a los laterales (ej. 0.60).
    /// - `default_gaps`: Espaciado interno en píxeles.
    /// - `_active_window_id`: Identificador de la ventana con foco.
    ///
    /// # Retorno
    /// Tupla con la tabla hash de geometrías calculadas y el vector de evicción.
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        _nmaster: usize,
        master_ratio: f32,
        default_gaps: i32,
        _active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        // 1. Validar que la pantalla tenga un área válida para trabajar
        let total_area = screen_rect.width * screen_rect.height;
        if total_area <= 0 || screen_rect.width <= 0 || screen_rect.height <= 0 {
            return (HashMap::new(), Vec::new());
        }

        // 2. Filtrar únicamente ventanas en mosaico (excluye flotantes y minimizadas)
        let active_windows: Vec<&WindowNode> = windows
            .iter()
            .filter(|w| !w.is_floating && !w.is_minimized)
            .collect();

        if active_windows.is_empty() {
            return (HashMap::new(), Vec::new());
        }

        let mut layout_map = HashMap::with_capacity(active_windows.len());
        let evicted_windows = Vec::new();

        // 3. Crear el marco del contenedor descontando el espaciado (gap) periférico
        let half_g = default_gaps / 2;
        let container = Rect {
            x: screen_rect.x + half_g,
            y: screen_rect.y + half_g,
            width: std::cmp::max(1, screen_rect.width - default_gaps),
            height: std::cmp::max(1, screen_rect.height - default_gaps),
        };

        let current_ordered = active_windows;

        if !current_ordered.is_empty() {
            // 4. Clasificar ventanas en 4 zonas dinámicas: Centro, Izquierda, Derecha e Inferior
            let total_count = current_ordered.len();
            let mut left_group = Vec::with_capacity(total_count);
            let mut right_group = Vec::with_capacity(total_count);
            let mut bottom_group = Vec::with_capacity(total_count);
            let mut center_group = Vec::with_capacity(total_count);

            if total_count == 3 {
                // Caso optimizado v3.2: 3 ventanas (Master Superior + Dúo Inferior)
                center_group.push(current_ordered[0]); // Ventana 1: Master Superior
                bottom_group.push(current_ordered[1]); // Ventana 2: Panel Inferior Izquierdo
                bottom_group.push(current_ordered[2]); // Ventana 3: Panel Inferior Derecho
            } else {
                for (idx, &win) in current_ordered.iter().enumerate() {
                    if idx == 0 {
                        center_group.push(win); // Ventana 1: Área Central
                    } else if idx == 1 {
                        left_group.push(win); // Ventana 2: Sidebar Izquierdo
                    } else if idx == 2 {
                        right_group.push(win); // Ventana 3: Sidebar Derecho
                    } else if idx == 3 {
                        bottom_group.push(win); // Ventana 4: Panel Inferior Izquierdo
                    } else if idx == 4 {
                        bottom_group.push(win); // Ventana 5: Panel Inferior Derecho
                    } else {
                        // idx >= 5: Subdivisión jerárquica cíclica (Laterales primero, luego Centro)
                        if idx % 3 == 2 {
                            left_group.push(win);
                        } else if idx % 3 == 0 {
                            right_group.push(win);
                        } else {
                            center_group.push(win);
                        }
                    }
                }
            }

            // 5. Acotar requerimientos de tamaño mínimo defensivamente al 35% de la dimensión útil
            let max_allowed_min_w = (container.width as f32 * 0.35) as i32;
            let max_allowed_min_h = (container.height as f32 * 0.35) as i32;

            let left_min_w = left_group.iter().map(|w| w.min_w.min(max_allowed_min_w)).max().unwrap_or(0);
            let right_min_w = right_group.iter().map(|w| w.min_w.min(max_allowed_min_w)).max().unwrap_or(0);
            let center_min_w = center_group.iter().map(|w| w.min_w.min(max_allowed_min_w)).max().unwrap_or(0);

            let bottom_min_h = bottom_group.iter().map(|w| w.min_h.min(max_allowed_min_h)).max().unwrap_or(0);

            // 6. Calcular proporciones y alturas de paneles
            let central_ratio = master_ratio.clamp(0.35, 0.85);
            let bottom_ratio = if left_group.is_empty() && right_group.is_empty() && !bottom_group.is_empty() {
                1.0 - central_ratio
            } else {
                0.30f32
            };

            let mut bottom_height = if !bottom_group.is_empty() {
                let bh = ((container.height as f32 * bottom_ratio).round()) as i32;
                std::cmp::max(bh, bottom_min_h)
            } else {
                0
            };

            // Garantía defensiva: La altura del panel inferior no debe superar el 65% de la pantalla
            if bottom_height > (container.height as f32 * 0.65) as i32 {
                bottom_height = (container.height as f32 * 0.65) as i32;
            }

            // 7. Calcular anchos de los paneles laterales y del área central
            let mut sidebar_width = if !left_group.is_empty() && !right_group.is_empty() {
                let sw = ((container.width as f32 * (1.0 - central_ratio) / 2.0).round()) as i32;
                std::cmp::max(sw, std::cmp::max(left_min_w, right_min_w))
            } else if !left_group.is_empty() {
                let sw = ((container.width as f32 * (1.0 - central_ratio)).round()) as i32;
                std::cmp::max(sw, left_min_w)
            } else {
                0
            };

            let total_sidebars_width = if !right_group.is_empty() {
                2 * sidebar_width
            } else if !left_group.is_empty() {
                sidebar_width
            } else {
                0
            };

            let mut center_width = container.width - total_sidebars_width;

            // Ajuste dinámico: Si el espacio asignado al centro cae por debajo de su requerimiento mínimo
            if center_width < center_min_w {
                let needed_for_sidebars = container.width - center_min_w;
                if needed_for_sidebars >= 0 {
                    sidebar_width = needed_for_sidebars / (if !right_group.is_empty() { 2 } else { 1 });
                    center_width = container.width - (if !right_group.is_empty() { 2 * sidebar_width } else { sidebar_width });
                }
            }

            // 8. Posicionar ventanas del Sidebar Izquierdo (Left Group)
            if !left_group.is_empty() {
                let mins: Vec<i32> = left_group.iter().map(|w| std::cmp::max(w.min_h, 80)).collect();
                let weights: Vec<Option<f32>> = left_group.iter().map(|w| w.custom_h_ratio).collect();
                let heights = distribute_weighted_sizes(container.height, &mins, &weights);
                let mut current_y = container.y;
                for (i, win) in left_group.iter().enumerate() {
                    let rect = Rect {
                        x: container.x,
                        y: current_y,
                        width: sidebar_width,
                        height: heights[i],
                    };
                    layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
                    current_y += heights[i];
                }
            }

            // 9. Posicionar ventanas del Sidebar Derecho (Right Group)
            if !right_group.is_empty() {
                let mins: Vec<i32> = right_group.iter().map(|w| std::cmp::max(w.min_h, 80)).collect();
                let weights: Vec<Option<f32>> = right_group.iter().map(|w| w.custom_h_ratio).collect();
                let heights = distribute_weighted_sizes(container.height, &mins, &weights);
                let mut current_y = container.y;
                for (i, win) in right_group.iter().enumerate() {
                    let rect = Rect {
                        x: container.x + container.width - sidebar_width,
                        y: current_y,
                        width: sidebar_width,
                        height: heights[i],
                    };
                    layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
                    current_y += heights[i];
                }
            }

            // 10. Posicionar ventanas del Área Central (Center Group)
            if !center_group.is_empty() {
                let main_h = container.height - bottom_height;
                let mins: Vec<i32> = center_group.iter().map(|w| std::cmp::max(w.min_h, 120)).collect();
                let weights: Vec<Option<f32>> = center_group.iter().map(|w| w.custom_h_ratio).collect();
                let heights = distribute_weighted_sizes(main_h, &mins, &weights);
                let mut current_y = container.y;
                for (i, win) in center_group.iter().enumerate() {
                    let rect = Rect {
                        x: container.x + sidebar_width,
                        y: current_y,
                        width: center_width,
                        height: heights[i],
                    };
                    layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
                    current_y += heights[i];
                }
            }

            // 11. Posicionar ventanas del Panel Inferior (Bottom Group)
            if !bottom_group.is_empty() {
                let mins: Vec<i32> = bottom_group.iter().map(|w| std::cmp::max(w.min_w, 100)).collect();
                let weights: Vec<Option<f32>> = bottom_group.iter().map(|w| w.custom_w_ratio).collect();
                let widths = distribute_weighted_sizes(center_width, &mins, &weights);
                let mut current_x = container.x + sidebar_width;
                for (i, win) in bottom_group.iter().enumerate() {
                    let rect = Rect {
                        x: current_x,
                        y: container.y + container.height - bottom_height,
                        width: widths[i],
                        height: bottom_height,
                    };
                    layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
                    current_x += widths[i];
                }
            }
        }

        (layout_map, evicted_windows)
    }
}
