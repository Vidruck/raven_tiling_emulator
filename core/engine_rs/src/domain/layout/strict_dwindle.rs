use super::{apply_gaps, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

pub struct StrictDwindleStrategy;

impl LayoutStrategy for StrictDwindleStrategy {
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

        let active_windows: Vec<WindowNode> = windows
            .iter()
            .filter(|w| !w.is_floating && !w.is_minimized)
            .cloned()
            .collect();

        if active_windows.is_empty() {
            return (layout_map, evicted_windows);
        }

        let half_g = default_gaps / 2;
        let mut container = Rect {
            x: screen_rect.x + half_g,
            y: screen_rect.y + half_g,
            width: std::cmp::max(1, screen_rect.width - default_gaps),
            height: std::cmp::max(1, screen_rect.height - default_gaps),
        };

        let mut split_horizontal = true;
        let count = active_windows.len();

        for (i, win) in active_windows.iter().enumerate() {
            if i == count - 1 {
                layout_map.insert(win.window_id.clone(), apply_gaps(&container, half_g));
                break;
            }

            let mut curr = container.clone();
            if split_horizontal {
                let w = (container.width as f32 * master_ratio) as i32;
                curr.width = w;
                container.x += w;
                container.width -= w;
            } else {
                let h = (container.height as f32 * master_ratio) as i32;
                curr.height = h;
                container.y += h;
                container.height -= h;
            }
            
            layout_map.insert(win.window_id.clone(), apply_gaps(&curr, half_g));
            split_horizontal = !split_horizontal;
        }

        (layout_map, evicted_windows)
    }
}

