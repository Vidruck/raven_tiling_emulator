//! # Algoritmos de Layout - Versión 2.7 (Dwindle BSP con Reutilización Semántica)
//!
//! Este submódulo contiene la lógica principal para el cálculo de la disposición
//! de las ventanas en mosaico (tiling). Implementa un árbol de división binaria adaptativo (BSP)
//! sensible a la proporción del contenedor (Aspect-Ratio Aware).

use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Calcula dinámicamente el área mínima permitida basándose en la resolución de la pantalla.
fn calculate_dynamic_min_area(screen_width: i32, screen_height: i32) -> i32 {
    let total_pixels = screen_width as f64 * screen_height as f64;

    let percentage = if total_pixels <= 1_100_000.0 {
        0.12 // Pantallas muy pequeñas (720p/HD): 12%
    } else if total_pixels <= 2_500_000.0 {
        0.08 // Pantallas laptop/normales (1080p): 8%
    } else if total_pixels <= 5_000_000.0 {
        0.05 // Monitores grandes (1440p): 5%
    } else {
        0.03 // Monitores gigantes (4K+): 3%
    };

    (total_pixels * percentage) as i32
}

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

/// Calcula la disposición en espiral (dwindle) con partición binaria de espacio (BSP)
/// reutilizando semánticamente las variables clásicas.
///
/// # Argumentos Mapeados:
/// * `nmaster` - Utilizado como el Techo Dinámico de Ventanas cómodas en pantalla antes de desalojar (eviction).
/// * `master_ratio` - Utilizado como el ratio de división (split ratio) binario asimétrico.
/// * `active_window_id` - Identificador opcional de la ventana en foco activo para aplicar el redimensionamiento asimétrico.
pub fn calculate_master_stack(
    windows: Vec<WindowNode>,
    screen_rect: Rect,
    nmaster: usize,
    master_ratio: f32,
    default_gaps: i32,
    active_window_id: Option<String>,
) -> (HashMap<String, Rect>, Vec<String>) {
    let mut layout_map = HashMap::new();
    let mut evicted_windows = Vec::new();

    let total_area = screen_rect.width * screen_rect.height;
    if total_area <= 0 || screen_rect.width <= 0 || screen_rect.height <= 0 {
        return (layout_map, evicted_windows);
    }

    let active_windows: Vec<WindowNode> = windows
        .into_iter()
        .filter(|w| !w.is_floating && !w.is_minimized)
        .collect();

    let count = active_windows.len();
    if count == 0 {
        return (layout_map, evicted_windows);
    }

    let half_g = default_gaps / 2;
    let min_allowed_area = calculate_dynamic_min_area(screen_rect.width, screen_rect.height);

    let max_comfortable_windows = if nmaster <= 1 { 5 } else { nmaster };

    let mut current_active = active_windows.clone();

    loop {
        if current_active.is_empty() {
            break;
        }

        let windows_to_place = std::cmp::min(current_active.len(), max_comfortable_windows);
        let mut temp_layout = HashMap::new();
        let mut failed_window_ids = Vec::new();

        if windows_to_place == 1 {
            let win = &current_active[0];
            let final_rect = apply_gaps(&screen_rect, default_gaps);
            let allowed_min_w = std::cmp::min(win.min_w, 300);
            let allowed_min_h = std::cmp::min(win.min_h, 250);

            if final_rect.width * final_rect.height >= min_allowed_area
                && final_rect.width >= allowed_min_w
                && final_rect.height >= allowed_min_h
            {
                temp_layout.insert(win.window_id.clone(), final_rect);
            } else {
                failed_window_ids.push(win.window_id.clone());
            }
        } else {
            let mut current_container = Rect {
                x: screen_rect.x + half_g,
                y: screen_rect.y + half_g,
                width: screen_rect.width - default_gaps,
                height: screen_rect.height - default_gaps,
            };

            let safe_split_ratio = master_ratio.clamp(0.20, 0.80);

            for (i, win) in current_active.iter().take(windows_to_place).enumerate() {
                let final_rect = if i == windows_to_place - 1 {
                    apply_gaps(&current_container, half_g)
                } else {
                    let mut tile_rect = current_container;

                    let is_active_involved = if let Some(ref active_id) = active_window_id {
                        win.window_id == *active_id
                            || (i == windows_to_place - 2
                                && current_active[i + 1].window_id == *active_id)
                    } else {
                        false
                    };

                    let split_ratio = if is_active_involved {
                        safe_split_ratio
                    } else {
                        0.5
                    };

                    if current_container.width > current_container.height {
                        let split_width = (current_container.width as f32 * split_ratio) as i32;
                        tile_rect.width = split_width;

                        current_container.x += split_width;
                        current_container.width -= split_width;
                    } else {
                        let split_height = (current_container.height as f32 * split_ratio) as i32;
                        tile_rect.height = split_height;

                        current_container.y += split_height;
                        current_container.height -= split_height;
                    }

                    apply_gaps(&tile_rect, half_g)
                };

                let allowed_min_w = std::cmp::min(win.min_w, 300);
                let allowed_min_h = std::cmp::min(win.min_h, 250);

                if final_rect.width * final_rect.height >= min_allowed_area
                    && final_rect.width >= allowed_min_w
                    && final_rect.height >= allowed_min_h
                {
                    temp_layout.insert(win.window_id.clone(), final_rect);
                } else {
                    failed_window_ids.push(win.window_id.clone());
                }
            }
        }

        if !failed_window_ids.is_empty() {
            current_active.retain(|w| !failed_window_ids.contains(&w.window_id));
        } else {
            layout_map = temp_layout;
            break;
        }
    }

    for win in &active_windows {
        if !layout_map.contains_key(&win.window_id) {
            evicted_windows.push(win.window_id.clone());
        }
    }

    (layout_map, evicted_windows)
}

/// Calcula la topología global para todas las áreas de trabajo (workspaces) activas.
///
/// Distribuye las ventanas no flotantes en sus respectivas geometrías de área de trabajo
/// utilizando particiones binarias de espacio (BSP) y calcula las posiciones exactas de
/// las ventanas flotantes con Picture-in-Picture (PiP).
pub fn calculate_global_topology(
    windows: Vec<WindowNode>,
    workspaces: HashMap<String, Rect>,
    nmaster: usize,
    master_ratio: f32,
    default_gaps: i32,
    pip_position: &str,
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
                .push(win);
        }
    }

    for (ws_id, ws_windows) in windows_by_ws {
        if let Some(screen_rect) = workspaces.get(&ws_id) {
            let (ws_layout, ws_evicted) = calculate_master_stack(
                ws_windows.clone(),
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

    fn mock_window_with_min(id: &str, min_w: i32, min_h: i32) -> WindowNode {
        WindowNode::new(
            id.to_string(),
            "ws_1".to_string(),
            "DP-1".to_string(),
            vec!["desk_1".to_string()],
            false,
            false,
            false,
            Rect::new(0, 0, 0, 0),
            min_w,
            min_h,
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

        // Caso sin foco: división simétrica 50/50 (gaps = 0 para facilitar matemáticas)
        let (layout_no_focus, _) =
            calculate_master_stack(windows.clone(), Rect::new(0, 0, 1200, 1000), 2, 0.6, 0, None);
        let r1_nf = layout_no_focus.get("win_1").unwrap();
        let r2_nf = layout_no_focus.get("win_2").unwrap();
        assert_eq!(r1_nf.width, 600);
        assert_eq!(r2_nf.width, 600);

        // Caso con foco en win_1: usa ratio 0.6 (60%)
        let (layout_focus_1, _) =
            calculate_master_stack(windows.clone(), Rect::new(0, 0, 1200, 1000), 2, 0.6, 0, Some("win_1".to_string()));
        let r1_f1 = layout_focus_1.get("win_1").unwrap();
        let r2_f1 = layout_focus_1.get("win_2").unwrap();
        assert_eq!(r1_f1.width, 720);
        assert_eq!(r2_f1.width, 480);

        // Caso con foco en win_2: usa ratio 0.6 (por implicar a la última ventana)
        let (layout_focus_2, _) =
            calculate_master_stack(windows.clone(), Rect::new(0, 0, 1200, 1000), 2, 0.6, 0, Some("win_2".to_string()));
        let r1_f2 = layout_focus_2.get("win_2").unwrap();
        let r2_f2 = layout_focus_2.get("win_1").unwrap();
        assert_eq!(r1_f2.width, 480); // win_2 toma el resto (1 - 0.6 = 0.4)
        assert_eq!(r2_f2.width, 720); // win_1 toma la división principal (0.6)
    }

    #[test]
    fn test_eviccion_recalculo_sin_huecos() {
        // En pantalla de 500x500, con 3 ventanas, win_2 tiene un tamaño mínimo de 400 (se acota a 300)
        // Pero al repartirse en 3 ventanas, win_2 obtendría un slot de 250 de ancho.
        // Dado que 250 < 300, win_2 no cabrá y será desalojada.
        let windows = vec![
            mock_window("win_1"),
            mock_window_with_min("win_2", 400, 0),
            mock_window("win_3"),
        ];

        let (layout, evicted) =
            calculate_master_stack(windows, Rect::new(0, 0, 500, 500), 5, 0.5, 0, None);

        // win_2 debe ser desalojada
        assert!(evicted.contains(&"win_2".to_string()));
        // Deben quedar win_1 y win_3 organizadas de forma limpia en el espacio disponible
        assert_eq!(layout.len(), 2);
        assert!(layout.contains_key("win_1"));
        assert!(layout.contains_key("win_3"));

        let r1 = layout.get("win_1").unwrap();
        let r3 = layout.get("win_3").unwrap();
        // Se recalculó la composición para 2 ventanas (cada una toma el 100% del ancho y 50% del alto = 250 de alto)
        assert_eq!(r1.width, 500);
        assert_eq!(r1.height, 250);
        assert_eq!(r3.width, 500);
        assert_eq!(r3.height, 250);
    }

    #[test]
    fn test_acotamiento_tamano_minimo() {
        // win_1 tiene un min_w de 500, pero como se acota a 300, debería caber en un slot de 400
        let windows = vec![
            mock_window_with_min("win_1", 500, 0),
            mock_window("win_2"),
        ];

        // Pantalla de 800 de ancho, split simétrico da slots de 400
        let (layout, evicted) =
            calculate_master_stack(windows, Rect::new(0, 0, 800, 1000), 2, 0.5, 0, None);

        // Sin el acotamiento de 300, win_1 (min_w = 500) habría sido desalojada de su slot de 400.
        // Con el acotamiento a 300, 400 >= 300, por lo que cabe y no es desalojada.
        assert!(evicted.is_empty());
        assert_eq!(layout.len(), 2);
        assert!(layout.contains_key("win_1"));
        assert!(layout.contains_key("win_2"));
    }
}
