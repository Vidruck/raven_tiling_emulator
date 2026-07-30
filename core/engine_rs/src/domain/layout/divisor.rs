use super::{apply_gaps, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

pub struct DivisorStrategy;

impl LayoutStrategy for DivisorStrategy {
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        _nmaster: usize,
        _master_ratio: f32,
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

        let num_windows = active_windows.len() as i32;
        let w_slot = container.width / num_windows;

        for (i, win) in active_windows.iter().enumerate() {
            let i = i as i32;
            let rect = Rect {
                x: container.x + (i * w_slot),
                y: container.y,
                width: if i == num_windows - 1 { container.width - (i * w_slot) } else { w_slot },
                height: container.height,
            };
            layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
        }

        (layout_map, evicted_windows)
    }
}

