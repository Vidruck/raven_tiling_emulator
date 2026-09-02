//! # Algoritmo de Espiral Recursiva Clásica (`StrictDwindleStrategy`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Implementa la división binaria áurea en espiral secuencial estricta (estilo Fibonacci/Bspwm).
//! Alterna cortes horizontales y verticales manteniendo un piso de seguridad `min_rem` para evitar
//! colapsos dimensionales en resoluciones reducidas.

use super::{apply_gaps, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Estrategia de layout "Strict Dwindle": partición binaria secuencial en espiral.
pub struct StrictDwindleStrategy;

impl LayoutStrategy for StrictDwindleStrategy {
    fn predict_capacity(&self, screen_rect: Rect, default_gaps: i32) -> usize {
        let usable_w = std::cmp::max(1, screen_rect.width - default_gaps);
        let usable_h = std::cmp::max(1, screen_rect.height - default_gaps);
        let max_w_splits = (usable_w as f32 / 240.0).log2().max(1.0) as usize;
        let max_h_splits = (usable_h as f32 / 200.0).log2().max(1.0) as usize;
        std::cmp::max(3, (max_w_splits + max_h_splits + 1).min(6))
    }
    /// Calcula las geometrías de las ventanas dividiendo sucesivamente el contenedor en mitades/ratios.
    ///
    /// # Parámetros
    /// - `windows`: Lista de ventanas no minimizadas ni flotantes.
    /// - `screen_rect`: Área utilizable del monitor.
    /// - `_nmaster`: No utilizado en Dwindle estricto.
    /// - `master_ratio`: Proporción de división entre la ventana actual y el espacio restante.
    /// - `default_gaps`: Espaciado entre bordes y ventanas.
    /// - `_active_window_id`: Identificador de la ventana activa.
    ///
    /// # Retorno
    /// Tupla con la tabla hash de geometrías resultantes y vector de evictadas (vacío).
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

        // Estado alternante: true = división vertical (ancho), false = división horizontal (alto)
        let mut split_horizontal = true;
        let count = active_windows.len();

        // 3. Iterar ventana por ventana creando el árbol espiral
        for (i, win) in active_windows.iter().enumerate() {
            // La última ventana de la lista toma todo el espacio restante disponible
            if i == count - 1 {
                layout_map.insert(win.window_id.clone(), apply_gaps(&container, half_g));
                break;
            }

            let mut curr = container;

            // División según la dirección alternante actual respetando requerimientos mínimos defensivos
            if split_horizontal {
                let win_min_w = std::cmp::max(win.min_w, 120);
                let min_rem = 120;
                let max_allowed = std::cmp::max(1, container.width - min_rem);
                let raw_w = (container.width as f32 * master_ratio) as i32;
                let w = raw_w.clamp(std::cmp::min(win_min_w, max_allowed), max_allowed);
                curr.width = w;
                container.x += w;
                container.width = std::cmp::max(1, container.width - w);
            } else {
                let win_min_h = std::cmp::max(win.min_h, 100);
                let min_rem = 100;
                let max_allowed = std::cmp::max(1, container.height - min_rem);
                let raw_h = (container.height as f32 * master_ratio) as i32;
                let h = raw_h.clamp(std::cmp::min(win_min_h, max_allowed), max_allowed);
                curr.height = h;
                container.y += h;
                container.height = std::cmp::max(1, container.height - h);
            }
            
            // Guardar la posición de la ventana actual recortando gaps
            layout_map.insert(win.window_id.clone(), apply_gaps(&curr, half_g));
            
            // Alternar orientación para la siguiente ventana
            split_horizontal = !split_horizontal;
        }

        (layout_map, evicted_windows)
    }
}
