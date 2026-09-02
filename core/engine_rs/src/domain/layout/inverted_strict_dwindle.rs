//! # Algoritmo de Espiral Recursiva Invertida (`InvertedStrictDwindleStrategy`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Implementa una partición espiral espejada geométricamente: la ventana principal
//! toma el panel derecho y las subsecuentes se subdividen de forma recursiva hacia
//! el cuadrante izquierdo con protecciones de contorno `min_rem`.

use super::{apply_gaps, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Estrategia de layout "Inverted Strict Dwindle": primera ventana a la derecha, espiral cerrando a la izquierda.
pub struct InvertedStrictDwindleStrategy;

impl LayoutStrategy for InvertedStrictDwindleStrategy {
    fn predict_capacity(&self, screen_rect: Rect, default_gaps: i32) -> usize {
        let usable_w = std::cmp::max(1, screen_rect.width - default_gaps);
        let usable_h = std::cmp::max(1, screen_rect.height - default_gaps);
        // Cada paso divide por 2 alternando W y H. La profundidad máxima estable es cuando el lado < 200px
        let max_w_splits = (usable_w as f32 / 240.0).log2().max(1.0) as usize;
        let max_h_splits = (usable_h as f32 / 200.0).log2().max(1.0) as usize;
        std::cmp::max(3, (max_w_splits + max_h_splits + 1).min(6))
    }
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        _nmaster: usize,
        master_ratio: f32,
        default_gaps: i32,
        _active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        // 1. Filtrar ventanas activas en mosaico
        let active_windows: Vec<&WindowNode> = windows
            .iter()
            .filter(|w| !w.is_floating && !w.is_minimized)
            .collect();

        if active_windows.is_empty() {
            return (HashMap::new(), Vec::new());
        }

        let mut layout_map = HashMap::with_capacity(active_windows.len());
        let evicted_windows = Vec::new();

        // 2. Establecer el rectángulo del contenedor con márgenes (gaps)
        let half_g = default_gaps / 2;
        let mut container = Rect {
            x: screen_rect.x + half_g,
            y: screen_rect.y + half_g,
            width: std::cmp::max(1, screen_rect.width - default_gaps),
            height: std::cmp::max(1, screen_rect.height - default_gaps),
        };

        // Estado de dirección de partición:
        // 0: Derecha (la ventana ocupa el lado derecho, el contenedor remanente queda a la izquierda)
        // 1: Abajo (la ventana ocupa la parte inferior, el contenedor remanente queda arriba)
        let count = active_windows.len();

        for (i, win) in active_windows.iter().enumerate() {
            if i == count - 1 {
                layout_map.insert(win.window_id.clone(), apply_gaps(&container, half_g));
                break;
            }

            let mut curr = container;

            match i % 4 {
                0 => {
                    // Partición vertical: la ventana toma el bloque DERECHO de tamaño master_ratio
                    let win_min_w = std::cmp::max(win.min_w, 120);
                    let min_rem = 120;
                    let max_allowed = std::cmp::max(1, container.width - min_rem);
                    let raw_w = (container.width as f32 * master_ratio) as i32;
                    let main_w = raw_w.clamp(std::cmp::min(win_min_w, max_allowed), max_allowed);
                    let rem_w = std::cmp::max(1, container.width - main_w);
                    
                    curr.x = container.x + rem_w;
                    curr.width = main_w;

                    // El contenedor remanente se queda en el lado IZQUIERDO
                    container.width = rem_w;
                }
                1 => {
                    // Partición horizontal: la ventana toma el bloque INFERIOR de tamaño master_ratio
                    let win_min_h = std::cmp::max(win.min_h, 100);
                    let min_rem = 100;
                    let max_allowed = std::cmp::max(1, container.height - min_rem);
                    let raw_h = (container.height as f32 * master_ratio) as i32;
                    let main_h = raw_h.clamp(std::cmp::min(win_min_h, max_allowed), max_allowed);
                    let rem_h = std::cmp::max(1, container.height - main_h);
                    
                    curr.y = container.y + rem_h;
                    curr.height = main_h;

                    // El contenedor remanente se queda en la parte SUPERIOR
                    container.height = rem_h;
                }
                2 => {
                    // Partición vertical: la ventana toma el bloque DERECHO del área remanente de tamaño master_ratio
                    let win_min_w = std::cmp::max(win.min_w, 120);
                    let min_rem = 120;
                    let max_allowed = std::cmp::max(1, container.width - min_rem);
                    let raw_w = (container.width as f32 * master_ratio) as i32;
                    let main_w = raw_w.clamp(std::cmp::min(win_min_w, max_allowed), max_allowed);
                    let rem_w = std::cmp::max(1, container.width - main_w);
                    
                    curr.x = container.x + rem_w;
                    curr.width = main_w;

                    // El contenedor remanente queda en la esquina SUPERIOR IZQUIERDA
                    container.width = rem_w;
                }
                _ => {
                    // Partición horizontal: la ventana toma el bloque SUPERIOR de tamaño master_ratio
                    let win_min_h = std::cmp::max(win.min_h, 100);
                    let min_rem = 100;
                    let max_allowed = std::cmp::max(1, container.height - min_rem);
                    let raw_h = (container.height as f32 * master_ratio) as i32;
                    let main_h = raw_h.clamp(std::cmp::min(win_min_h, max_allowed), max_allowed);
                    
                    curr.height = main_h;

                    // El contenedor remanente se desplaza hacia ABAJO
                    container.y += main_h;
                    container.height = std::cmp::max(1, container.height - main_h);
                }
            }

            layout_map.insert(win.window_id.clone(), apply_gaps(&curr, half_g));
        }

        (layout_map, evicted_windows)
    }
}
