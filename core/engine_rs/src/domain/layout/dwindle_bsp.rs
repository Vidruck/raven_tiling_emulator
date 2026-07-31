use super::{apply_gaps, distribute_sizes, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

pub struct DwindleBSPStrategy;

impl LayoutStrategy for DwindleBSPStrategy {
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        _nmaster: usize,
        master_ratio: f32,
        default_gaps: i32,
        _active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
    let mut layout_map = HashMap::new();
    let evicted_windows = Vec::new();

    let total_area = screen_rect.width * screen_rect.height;
    if total_area <= 0 || screen_rect.width <= 0 || screen_rect.height <= 0 {
        return (layout_map, evicted_windows);
    }

    let active_windows: Vec<WindowNode> = windows
        .iter()
        .filter(|w| !w.is_floating && !w.is_minimized)
        .cloned()
        .collect();

    if active_windows.is_empty() {
        return (layout_map, evicted_windows);
    }

    // Mantener un orden espacial estable e inalterado por cambios de foco.
    let ordered_windows = active_windows.clone();

    let half_g = default_gaps / 2;

    let container = Rect {
        x: screen_rect.x + half_g,
        y: screen_rect.y + half_g,
        width: std::cmp::max(1, screen_rect.width - default_gaps),
        height: std::cmp::max(1, screen_rect.height - default_gaps),
    };

    let current_ordered = ordered_windows.clone();

    loop {
        if current_ordered.is_empty() {
            break;
        }

        let mut left_group = Vec::new();
        let mut right_group = Vec::new();
        let mut bottom_group = Vec::new();
        let mut center_group = Vec::new();

        for (idx, win) in current_ordered.iter().enumerate() {
            if idx == 0 {
                center_group.push(win.clone());
            } else if idx == 1 {
                left_group.push(win.clone());
            } else if idx == 2 {
                right_group.push(win.clone());
            } else if idx == 3 {
                bottom_group.push(win.clone());
            } else if idx == 4 {
                bottom_group.push(win.clone());
            } else {
                // i >= 5: Subdivisión jerárquica (Laterales primero, luego Centro)
                if idx % 3 == 2 {
                    left_group.push(win.clone());
                } else if idx % 3 == 0 {
                    right_group.push(win.clone());
                } else {
                    center_group.push(win.clone());
                }
            }
        }

        // Calcular restricciones de tamaño mínimo de cada grupo (defensivamente limitados al 35% del contenedor)
        let max_allowed_min_w = (container.width as f32 * 0.35) as i32;
        let max_allowed_min_h = (container.height as f32 * 0.35) as i32;

        let left_min_w = left_group.iter().map(|w| w.min_w.min(max_allowed_min_w)).max().unwrap_or(0);
        let right_min_w = right_group.iter().map(|w| w.min_w.min(max_allowed_min_w)).max().unwrap_or(0);
        let center_min_w = center_group.iter().map(|w| w.min_w.min(max_allowed_min_w)).max().unwrap_or(0);

        let _left_min_h = left_group.iter().map(|w| w.min_h.min(max_allowed_min_h)).max().unwrap_or(0);
        let _right_min_h = right_group.iter().map(|w| w.min_h.min(max_allowed_min_h)).max().unwrap_or(0);
        let _center_min_h = center_group.iter().map(|w| w.min_h.min(max_allowed_min_h)).max().unwrap_or(0);
        let bottom_min_h = bottom_group.iter().map(|w| w.min_h.min(max_allowed_min_h)).max().unwrap_or(0);

        let central_ratio = master_ratio.clamp(0.35, 0.85);
        let bottom_ratio = 0.30f32;

        let mut bottom_height = if !bottom_group.is_empty() {
            let bh = ((container.height as f32 * bottom_ratio).round()) as i32;
            std::cmp::max(bh, bottom_min_h)
        } else {
            0
        };

        // Medida defensiva: evitar que la altura del bottom supere el 50% de la pantalla útil
        if bottom_height > container.height / 2 {
            bottom_height = container.height / 2;
        }

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

        // Negociación defensiva: si el ancho restante para el centro es menor que su mínimo
        if center_width < center_min_w {
            let needed_for_sidebars = container.width - center_min_w;
            if needed_for_sidebars >= 0 {
                sidebar_width = needed_for_sidebars / (if !right_group.is_empty() { 2 } else { 1 });
                center_width = container.width - (if !right_group.is_empty() { 2 * sidebar_width } else { sidebar_width });
            }
        }

        // Eliminada la validación estricta de ancho para paneles laterales para evitar desalojos y cuelgues del layout

        // Calcular alturas de ranura para cada sección
        let _left_slot_h = if !left_group.is_empty() { container.height / left_group.len() as i32 } else { 0 };
        let _right_slot_h = if !right_group.is_empty() { container.height / right_group.len() as i32 } else { 0 };
        let _center_slot_h = if !center_group.is_empty() { (container.height - bottom_height) / center_group.len() as i32 } else { 0 };

        // Eliminada la validación estricta de altura para evitar que aplicaciones como IntelliJ o Zen detengan el layout

        // Posicionar Left Sidebar
        if !left_group.is_empty() {
            let mins: Vec<i32> = left_group.iter().map(|w| std::cmp::max(w.min_h, 80)).collect();
            let heights = distribute_sizes(container.height, &mins);
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

        // Posicionar Right Sidebar
        if !right_group.is_empty() {
            let mins: Vec<i32> = right_group.iter().map(|w| std::cmp::max(w.min_h, 80)).collect();
            let heights = distribute_sizes(container.height, &mins);
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

        // Posicionar Center (con soporte para múltiples sub-ventanas si N > 5)
        if !center_group.is_empty() {
            let main_h = container.height - bottom_height;
            let mins: Vec<i32> = center_group.iter().map(|w| std::cmp::max(w.min_h, 120)).collect();
            let heights = distribute_sizes(main_h, &mins);
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

        // Posicionar Bottom Panels
        if !bottom_group.is_empty() {
            let mins: Vec<i32> = bottom_group.iter().map(|w| std::cmp::max(w.min_w, 100)).collect();
            let widths = distribute_sizes(center_width, &mins);
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

        break;
    }

    (layout_map, evicted_windows)
    }
}

