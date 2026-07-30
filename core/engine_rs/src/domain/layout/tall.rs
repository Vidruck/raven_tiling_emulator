use super::{apply_gaps, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

pub struct TallStrategy;

impl LayoutStrategy for TallStrategy {
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        nmaster: usize,
        master_ratio: f32,
        default_gaps: i32,
        _active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        let mut layout_map = HashMap::new();
        let evicted_windows = Vec::new();

        let active_windows: Vec<WindowNode> = windows
            .iter()
            .filter(|w| !w.is_floating && !w.is_minimized)
            .cloned()
            .collect();

        if active_windows.is_empty() {
            return (layout_map, evicted_windows);
        }

        let half_g = default_gaps / 2;
        let container = Rect {
            x: screen_rect.x + half_g,
            y: screen_rect.y + half_g,
            width: std::cmp::max(1, screen_rect.width - default_gaps),
            height: std::cmp::max(1, screen_rect.height - default_gaps),
        };

        if active_windows.len() <= nmaster {
            let w_slot = container.width / active_windows.len() as i32;
            for (i, win) in active_windows.iter().enumerate() {
                let rect = Rect {
                    x: container.x + (i as i32 * w_slot),
                    y: container.y,
                    width: if i == active_windows.len() - 1 { container.width - (i as i32 * w_slot) } else { w_slot },
                    height: container.height,
                };
                layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
            }
        } else {
            let mut master_w = (container.width as f32 * master_ratio) as i32;

            // Restricción dinámica: Adaptarse a ventanas como Preferencias de KDE o Zen con minSize muy grande
            let mut max_master_min_w = 0;
            for i in 0..nmaster {
                if active_windows[i].min_w > max_master_min_w {
                    max_master_min_w = active_windows[i].min_w;
                }
            }
            if master_w < max_master_min_w {
                master_w = max_master_min_w;
            }

            let stack_count = active_windows.len() - nmaster;
            let mut stack_w = container.width - master_w;

            let mut max_stack_min_w = 0;
            for i in 0..stack_count {
                let win = &active_windows[nmaster + i];
                if win.min_w > max_stack_min_w {
                    max_stack_min_w = win.min_w;
                }
            }
            if stack_w < max_stack_min_w {
                stack_w = max_stack_min_w;
                // Si la pila requiere más espacio del sobrante, empujar el master hacia la izquierda
                master_w = std::cmp::max(1, container.width - stack_w);
            }

            let master_h_slot = container.height / nmaster as i32;
            for i in 0..nmaster {
                let rect = Rect {
                    x: container.x,
                    y: container.y + (i as i32 * master_h_slot),
                    width: master_w,
                    height: if i == nmaster - 1 { container.height - (i as i32 * master_h_slot) } else { master_h_slot },
                };
                layout_map.insert(active_windows[i].window_id.clone(), apply_gaps(&rect, half_g));
            }

            let stack_count = active_windows.len() - nmaster;
            let stack_h_slot = container.height / stack_count as i32;
            for i in 0..stack_count {
                let win = &active_windows[nmaster + i];
                let rect = Rect {
                    x: container.x + master_w,
                    y: container.y + (i as i32 * stack_h_slot),
                    width: stack_w,
                    height: if i == stack_count - 1 { container.height - (i as i32 * stack_h_slot) } else { stack_h_slot },
                };
                layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
            }
        }

        (layout_map, evicted_windows)
    }
}

