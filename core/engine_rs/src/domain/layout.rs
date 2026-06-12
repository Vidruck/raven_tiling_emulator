//! # Algoritmos de Layout - Versión 2.8 (Dwindle BSP con Reutilización Semántica)
//!
//! Este submódulo contiene la lógica principal para el cálculo de la disposición
//! de las ventanas en mosaico (tiling). Implementa un árbol de división binaria adaptativo (BSP)
//! sensible a la proporción del contenedor (Aspect-Ratio Aware).

use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Aplica un espaciado (gap) interno a un rectángulo.
#[inline(always)]
fn apply_gaps(rect: &Rect, gap: i32) -> Rect {
    Rect {
        x: rect.x + gap,
        y: rect.y + gap,
        width: std::cmp::max(1, rect.width - (2 * gap)),
        height: std::cmp::max(1, rect.height - (2 * gap)),
    }
}

/// Calcula la disposición en Master-Stack dinámico con redimensionamiento asimétrico según foco.
///
/// Comienza con un diseño 1 x (C - 1). Si hay 5 o más ventanas, pasa a una composición 2 x 3,
/// encajando la ventana más antigua en el área maestra junto con la ventana activa.
pub trait LayoutStrategy: Send + Sync {
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        nmaster: usize,
        master_ratio: f32,
        default_gaps: i32,
        active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>);
}

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
    let mut evicted_windows = Vec::new();

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

    let mut current_ordered = ordered_windows.clone();

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

        // Calcular restricciones de tamaño mínimo de cada grupo
        let left_min_w = left_group.iter().map(|w| std::cmp::max(w.min_w, 100)).max().unwrap_or(0);
        let right_min_w = right_group.iter().map(|w| std::cmp::max(w.min_w, 100)).max().unwrap_or(0);
        let center_min_w = center_group.iter().map(|w| std::cmp::max(w.min_w, 150)).max().unwrap_or(0);

        let left_min_h = left_group.iter().map(|w| std::cmp::max(w.min_h, 80)).max().unwrap_or(0);
        let right_min_h = right_group.iter().map(|w| std::cmp::max(w.min_h, 80)).max().unwrap_or(0);
        let center_min_h = center_group.iter().map(|w| std::cmp::max(w.min_h, 120)).max().unwrap_or(0);
        let bottom_min_h = bottom_group.iter().map(|w| std::cmp::max(w.min_h, 80)).max().unwrap_or(0);

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
            } else {
                // Si la pantalla es físicamente muy pequeña para el centro, desalojamos la ventana con mayor restricción o la última
                let mut victim_idx = current_ordered.len() - 1;
                let mut max_constraint = 0;
                for (i, w) in current_ordered.iter().enumerate() {
                    let constraint = std::cmp::max(w.min_w, w.min_h);
                    if constraint > max_constraint && constraint > 300 {
                        max_constraint = constraint;
                        victim_idx = i;
                    }
                }
                let evicted_win = current_ordered.remove(victim_idx);
                evicted_windows.push(evicted_win.window_id);
                continue;
            }
        }

        // Si los paneles laterales quedan por debajo de su ancho mínimo permitido tras la compresión
        if (!left_group.is_empty() && sidebar_width < left_min_w)
            || (!right_group.is_empty() && sidebar_width < right_min_w)
        {
            let mut victim_idx = current_ordered.len() - 1;
            let mut max_constraint = 0;
            for (i, w) in current_ordered.iter().enumerate() {
                let constraint = std::cmp::max(w.min_w, w.min_h);
                if constraint > max_constraint && constraint > 300 {
                    max_constraint = constraint;
                    victim_idx = i;
                }
            }
            let evicted_win = current_ordered.remove(victim_idx);
            evicted_windows.push(evicted_win.window_id);
            continue;
        }

        // Calcular alturas de ranura para cada sección
        let left_slot_h = if !left_group.is_empty() { container.height / left_group.len() as i32 } else { 0 };
        let right_slot_h = if !right_group.is_empty() { container.height / right_group.len() as i32 } else { 0 };
        let center_slot_h = if !center_group.is_empty() { (container.height - bottom_height) / center_group.len() as i32 } else { 0 };

        // Si alguna subdivisión viola su altura mínima útil, desalojamos la ventana menos prioritaria y recalculamos
        if (!left_group.is_empty() && left_slot_h < left_min_h)
            || (!right_group.is_empty() && right_slot_h < right_min_h)
            || (!center_group.is_empty() && center_slot_h < center_min_h)
        {
            let mut victim_idx = current_ordered.len() - 1;
            let mut max_constraint = 0;
            for (i, w) in current_ordered.iter().enumerate() {
                let constraint = std::cmp::max(w.min_w, w.min_h);
                if constraint > max_constraint && constraint > 300 {
                    max_constraint = constraint;
                    victim_idx = i;
                }
            }
            let evicted_win = current_ordered.remove(victim_idx);
            evicted_windows.push(evicted_win.window_id);
            continue;
        }

        // Posicionar Left Sidebar
        if !left_group.is_empty() {
            let h_slot = container.height / left_group.len() as i32;
            for (i, win) in left_group.iter().enumerate() {
                let rect = Rect {
                    x: container.x,
                    y: container.y + (i as i32 * h_slot),
                    width: sidebar_width,
                    height: if i == left_group.len() - 1 { container.height - (i as i32 * h_slot) } else { h_slot },
                };
                layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
            }
        }

        // Posicionar Right Sidebar
        if !right_group.is_empty() {
            let h_slot = container.height / right_group.len() as i32;
            for (i, win) in right_group.iter().enumerate() {
                let rect = Rect {
                    x: container.x + container.width - sidebar_width,
                    y: container.y + (i as i32 * h_slot),
                    width: sidebar_width,
                    height: if i == right_group.len() - 1 { container.height - (i as i32 * h_slot) } else { h_slot },
                };
                layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
            }
        }

        // Posicionar Center (con soporte para múltiples sub-ventanas si N > 5)
        if !center_group.is_empty() {
            let main_h = container.height - bottom_height;
            let h_slot = main_h / center_group.len() as i32;
            for (i, win) in center_group.iter().enumerate() {
                let rect = Rect {
                    x: container.x + sidebar_width,
                    y: container.y + (i as i32 * h_slot),
                    width: center_width,
                    height: if i == center_group.len() - 1 { main_h - (i as i32 * h_slot) } else { h_slot },
                };
                layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
            }
        }

        // Posicionar Bottom Panels
        if !bottom_group.is_empty() {
            let w_slot = center_width / bottom_group.len() as i32;
            for (i, win) in bottom_group.iter().enumerate() {
                let rect = Rect {
                    x: container.x + sidebar_width + (i as i32 * w_slot),
                    y: container.y + container.height - bottom_height,
                    width: if i == bottom_group.len() - 1 { center_width - (i as i32 * w_slot) } else { w_slot },
                    height: bottom_height,
                };
                layout_map.insert(win.window_id.clone(), apply_gaps(&rect, half_g));
            }
        }

        break;
    }

    (layout_map, evicted_windows)
    }
}

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
            let master_w = (container.width as f32 * master_ratio) as i32;
            let stack_w = container.width - master_w;

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

pub struct MonocleStrategy;

impl LayoutStrategy for MonocleStrategy {
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
        let mut evicted_windows = Vec::new();

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

        for win in active_windows {
            layout_map.insert(win.window_id.clone(), apply_gaps(&container, half_g));
        }

        (layout_map, evicted_windows)
    }
}

pub fn get_strategy(layout_type: &str) -> Box<dyn LayoutStrategy> {
    match layout_type {
        "tall" => Box::new(TallStrategy),
        "monocle" => Box::new(MonocleStrategy),
        "dwindle" | _ => Box::new(DwindleBSPStrategy),
    }
}

/// Calcula la topología global para todas las áreas de trabajo (workspaces) activas.
///
/// Distribuye las ventanas no flotantes en sus respectivas geometrías de área de trabajo
/// utilizando particiones binarias de espacio (BSP) y calcula las posiciones exactas de
/// las ventanas flotantes con Picture-in-Picture (PiP).
pub fn calculate_global_topology(
    windows: &[WindowNode],
    workspaces: &HashMap<String, Rect>,
    nmaster: usize,
    master_ratio: f32,
    default_gaps: i32,
    pip_position: &str,
    layout_type: &str,
    active_window_id: Option<String>,
) -> (HashMap<String, Rect>, Vec<String>) {
    let mut global_layout = HashMap::new();
    let mut global_evicted = Vec::new();
    let mut windows_by_ws: HashMap<String, Vec<WindowNode>> = HashMap::new();

    for win in windows {
        if !win.is_floating || win.is_pip {
            windows_by_ws
                .entry(win.workspace_id.clone())
                .or_insert_with(Vec::new)
                .push(win.clone());
        }
    }

    for (ws_id, ws_windows) in windows_by_ws {
        if let Some(screen_rect) = workspaces.get(&ws_id) {
            let strategy = get_strategy(layout_type);
            let (ws_layout, ws_evicted) = strategy.calculate(
                &ws_windows,
                *screen_rect,
                nmaster,
                master_ratio,
                default_gaps,
                active_window_id.clone(),
            );
            global_layout.extend(ws_layout);
            global_evicted.extend(ws_evicted);

            let pip_w = (screen_rect.width as f32 * 0.22) as i32;
            let pip_h = (pip_w as f32 * 0.56) as i32;
            let pip_gap = default_gaps + 10;

            for win in ws_windows {
                if win.is_pip && !win.is_minimized {
                    let mut x = screen_rect.x + pip_gap;
                    let mut y = screen_rect.y + pip_gap;

                    match pip_position {
                        "top-right" => {
                            x = screen_rect.x + screen_rect.width - pip_w - pip_gap;
                        }
                        "bottom-left" => {
                            y = screen_rect.y + screen_rect.height - pip_h - pip_gap;
                        }
                        "bottom-right" => {
                            x = screen_rect.x + screen_rect.width - pip_w - pip_gap;
                            y = screen_rect.y + screen_rect.height - pip_h - pip_gap;
                        }
                        _ => {}
                    }

                    let pip_rect = Rect::new(x, y, pip_w, pip_h);
                    global_layout.insert(win.window_id.clone(), pip_rect);
                }
            }
        }
    }
    (global_layout, global_evicted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_window(id: &str) -> WindowNode {
        WindowNode::new(
            id.to_string(),
            "ws_1".to_string(),
            "DP-1".to_string(),
            vec!["desk_1".to_string()],
            false,
            false,
            false,
            Rect::new(0, 0, 0, 0),
            0,
            0,
            false,
        )
    }


    #[test]
    fn test_calculo_vacio_retorna_limpio() {
        let (layout, evicted) =
            calculate_master_stack(vec![], Rect::new(0, 0, 1920, 1080), 1, 0.5, 10, None);
        assert!(layout.is_empty());
        assert!(evicted.is_empty());
    }

    #[test]
    fn test_ventana_unica_respeta_gaps() {
        let windows = vec![mock_window("win_1")];
        let (layout, evicted) =
            calculate_master_stack(windows, Rect::new(0, 0, 1920, 1080), 1, 0.5, 20, None);
        assert!(evicted.is_empty());
        let rect = layout.get("win_1").unwrap();
        assert_eq!(rect.x, 20);
        assert_eq!(rect.width, 1880);
    }

    #[test]
    fn test_redimensionamiento_por_foco() {
        let windows = vec![mock_window("win_1"), mock_window("win_2")];

        // Caso sin foco: win_1 es Central (derecha) y win_2 es Sidebar Izq (izquierda)
        // gaps = 0, master_ratio = 0.6 (central_ratio = 0.6, sidebar_ratio = 0.4)
        // sidebar_width = 1200 * 0.4 = 480.
        // Central: x = 480, width = 720.
        // Sidebar: x = 0, width = 480.
        let (layout_no_focus, _) = calculate_master_stack(
            windows.clone(),
            Rect::new(0, 0, 1200, 1000),
            2,
            0.6,
            0,
            None,
        );
        let r1_nf = layout_no_focus.get("win_1").unwrap();
        let r2_nf = layout_no_focus.get("win_2").unwrap();
        assert_eq!(r1_nf.x, 480);
        assert_eq!(r1_nf.width, 720);
        assert_eq!(r2_nf.x, 0);
        assert_eq!(r2_nf.width, 480);

        // Caso con foco en win_2: win_2 NO debe moverse del lateral ni hacer swap automático
        let (layout_focus_2, _) = calculate_master_stack(
            windows.clone(),
            Rect::new(0, 0, 1200, 1000),
            2,
            0.6,
            0,
            Some("win_2".to_string()),
        );
        let r1_f2 = layout_focus_2.get("win_2").unwrap();
        let r2_f2 = layout_focus_2.get("win_1").unwrap();
        // win_2 debe seguir estando en la columna izquierda (sidebar) y win_1 en la derecha (centro)
        assert_eq!(r1_f2.x, 0);
        assert_eq!(r1_f2.width, 480);
        assert_eq!(r2_f2.x, 480);
        assert_eq!(r2_f2.width, 720);
    }

    #[test]
    fn test_subdivision_panels_laterales() {
        let windows = vec![
            mock_window("win_1"),
            mock_window("win_2"),
            mock_window("win_3"),
            mock_window("win_4"),
            mock_window("win_5"),
            mock_window("win_6"),
        ];

        let (layout, evicted) =
            calculate_master_stack(windows, Rect::new(0, 0, 1000, 1000), 5, 0.6, 0, None);

        // Con 6 ventanas, win_6 debe acomodarse subdividiendo el lateral izquierdo verticalmente. No hay evicción.
        assert!(evicted.is_empty());
        assert_eq!(layout.len(), 6);

        let r2 = layout.get("win_2").unwrap(); // Sidebar Izq (Sup)
        let r6 = layout.get("win_6").unwrap(); // Sidebar Izq (Inf)

        // Ambos laterales izq deben estar en x = 0, tener el mismo ancho de 200, y h = 500.
        assert_eq!(r2.x, 0);
        assert_eq!(r2.width, 200);
        assert_eq!(r2.height, 500);

        assert_eq!(r6.x, 0);
        assert_eq!(r6.width, 200);
        assert_eq!(r6.height, 500);
    }

    #[test]
    fn test_vscode_layout_5_windows() {
        let windows = vec![
            mock_window("win_1"),
            mock_window("win_2"),
            mock_window("win_3"),
            mock_window("win_4"),
            mock_window("win_5"),
        ];

        // Pantalla de 1000 de ancho y 600 de alto (gaps = 0, master_ratio = 0.6)
        // sidebar_width = (1000 * 0.4) / 2 = 200
        // bottom_height = 600 * 0.3 = 180
        let (layout, evicted) =
            calculate_master_stack(windows, Rect::new(0, 0, 1000, 600), 1, 0.6, 0, None);

        assert!(evicted.is_empty());
        assert_eq!(layout.len(), 5);

        let r1 = layout.get("win_1").unwrap(); // Central
        let r2 = layout.get("win_2").unwrap(); // Sidebar Izq
        let r3 = layout.get("win_3").unwrap(); // Sidebar Der
        let r4 = layout.get("win_4").unwrap(); // Bottom Panel 1 (izq)
        let r5 = layout.get("win_5").unwrap(); // Bottom Panel 2 (der)

        // Sidebar Izq: x = 0, y = 0, w = 200, h = 600
        assert_eq!(r2.x, 0);
        assert_eq!(r2.width, 200);
        assert_eq!(r2.height, 600);

        // Sidebar Der: x = 800, y = 0, w = 200, h = 600
        assert_eq!(r3.x, 800);
        assert_eq!(r3.width, 200);
        assert_eq!(r3.height, 600);

        // Central: x = 200, y = 0, w = 600, h = 420 (600 - 180)
        assert_eq!(r1.x, 200);
        assert_eq!(r1.width, 600);
        assert_eq!(r1.height, 420);

        // Bottom 1: x = 200, y = 420, w = 300, h = 180
        assert_eq!(r4.x, 200);
        assert_eq!(r4.width, 300);
        assert_eq!(r4.height, 180);

        // Bottom 2: x = 500, y = 420, w = 300, h = 180
        assert_eq!(r5.x, 500);
        assert_eq!(r5.width, 300);
        assert_eq!(r5.height, 180);
    }
}
