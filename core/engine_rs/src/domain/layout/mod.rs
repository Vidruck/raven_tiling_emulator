//! # Algoritmos de Layout - Versión 2.9
//!
//! Este submódulo contiene la lógica principal para el cálculo de la disposición
//! de las ventanas en mosaico (tiling). Implementa un árbol de división binaria adaptativo (BSP)
//! sensible a la proporción del contenedor (Aspect-Ratio Aware).

use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Aplica un espaciado (gap) interno a un rectángulo.
#[inline(always)]
pub(crate) fn apply_gaps(rect: &Rect, gap: i32) -> Rect {
    Rect {
        x: rect.x + gap,
        y: rect.y + gap,
        width: std::cmp::max(1, rect.width - (2 * gap)),
        height: std::cmp::max(1, rect.height - (2 * gap)),
    }
}

/// Distribuye un total de espacio lineal entre N elementos respetando sus tamaños mínimos.
pub(crate) fn distribute_sizes(total: i32, minimums: &[i32]) -> Vec<i32> {
    let n = minimums.len();
    if n == 0 { return vec![]; }

    // Medida defensiva global: evitar que la suma de mínimos consuma todo el espacio útil
    let max_min_per_item = std::cmp::max(10, total / n as i32);
    let sanitized_mins: Vec<i32> = minimums.iter().map(|&m| m.min(max_min_per_item)).collect();

    let mut sizes = vec![total / n as i32; n];
    sizes[n - 1] += total % n as i32;

    let mut unresolved = true;
    while unresolved {
        unresolved = false;
        let mut deficit = 0;
        let mut flexible_count = 0;

        for i in 0..n {
            if sizes[i] < sanitized_mins[i] {
                deficit += sanitized_mins[i] - sizes[i];
                sizes[i] = sanitized_mins[i];
                unresolved = true;
            } else if sizes[i] > sanitized_mins[i] {
                flexible_count += 1;
            }
        }

        if deficit > 0 && flexible_count > 0 {
            let deduction = deficit / flexible_count;
            let mut remainder = deficit % flexible_count;
            for i in 0..n {
                if sizes[i] > sanitized_mins[i] {
                    let mut take = deduction;
                    if remainder > 0 { take += 1; remainder -= 1; }
                    let actual_take = std::cmp::min(take, sizes[i] - sanitized_mins[i]);
                    sizes[i] -= actual_take;
                }
            }
        } else if deficit > 0 {
            break;
        }
    }
    sizes
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


pub mod dwindle_bsp;
pub mod tall;
pub mod monocle;
pub mod strict_dwindle;
pub mod divisor;

pub use dwindle_bsp::DwindleBSPStrategy;
pub use tall::TallStrategy;
pub use monocle::MonocleStrategy;
pub use strict_dwindle::StrictDwindleStrategy;
pub use divisor::DivisorStrategy;

pub fn get_strategy(layout_type: &str) -> Box<dyn LayoutStrategy> {
    match layout_type {
        "tall" => Box::new(TallStrategy),
        "monocle" => Box::new(MonocleStrategy),
        "strict_dwindle" => Box::new(StrictDwindleStrategy),
        "divisor" => Box::new(DivisorStrategy),
        "raven" | _ => Box::new(DwindleBSPStrategy),
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
            let mut pip_index = 0;

            for win in ws_windows {
                if win.is_pip && !win.is_minimized {
                    let final_pip_w = std::cmp::max(pip_w, win.min_w);
                    let final_pip_h = std::cmp::max(pip_h, win.min_h);

                    // Calculamos el offset de apilamiento para evitar solapamiento exacto
                    let offset_y = pip_index * (final_pip_h + pip_gap);

                    let mut x = screen_rect.x + pip_gap;
                    let mut y = screen_rect.y + pip_gap;

                    match pip_position.trim() {
                        "top-right" => {
                            x = screen_rect.x + screen_rect.width - final_pip_w - pip_gap;
                            y += offset_y; // Apilar hacia abajo
                        }
                        "bottom-left" => {
                            y = screen_rect.y + screen_rect.height - final_pip_h - pip_gap;
                            y -= offset_y; // Apilar hacia arriba
                        }
                        "bottom-right" => {
                            x = screen_rect.x + screen_rect.width - final_pip_w - pip_gap;
                            y = screen_rect.y + screen_rect.height - final_pip_h - pip_gap;
                            y -= offset_y; // Apilar hacia arriba
                        }
                        _ => {
                            // "top-left" por defecto
                            y += offset_y; // Apilar hacia abajo
                        }
                    }

                    // Prevenir que se salgan de la pantalla verticalmente
                    if y < screen_rect.y + pip_gap {
                        y = screen_rect.y + pip_gap;
                    }
                    if y + final_pip_h > screen_rect.y + screen_rect.height - pip_gap {
                        y = screen_rect.y + screen_rect.height - final_pip_h - pip_gap;
                    }

                    let pip_rect = Rect::new(x, y, final_pip_w, final_pip_h);
                    global_layout.insert(win.window_id.clone(), pip_rect);
                    pip_index += 1;
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
        let strategy = DwindleBSPStrategy;
        let (layout, evicted) =
            strategy.calculate(&[], Rect::new(0, 0, 1920, 1080), 1, 0.5, 10, None);
        assert!(layout.is_empty());
        assert!(evicted.is_empty());
    }

    #[test]
    fn test_ventana_unica_respeta_gaps() {
        let windows = vec![mock_window("win_1")];
        let strategy = DwindleBSPStrategy;
        let (layout, evicted) =
            strategy.calculate(&windows, Rect::new(0, 0, 1920, 1080), 1, 0.5, 20, None);
        assert!(evicted.is_empty());
        let rect = layout.get("win_1").unwrap();
        assert_eq!(rect.x, 20);
        assert_eq!(rect.width, 1880);
    }

    #[test]
    fn test_redimensionamiento_por_foco() {
        let windows = vec![mock_window("win_1"), mock_window("win_2")];
        let strategy = DwindleBSPStrategy;

        // Caso sin foco: win_1 es Central (derecha) y win_2 es Sidebar Izq (izquierda)
        // gaps = 0, master_ratio = 0.6 (central_ratio = 0.6, sidebar_ratio = 0.4)
        // sidebar_width = 1200 * 0.4 = 480.
        // Central: x = 480, width = 720.
        // Sidebar: x = 0, width = 480.
        let (layout_no_focus, _) = strategy.calculate(
            &windows,
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
        let (layout_focus_2, _) = strategy.calculate(
            &windows,
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

        let strategy = DwindleBSPStrategy;
        let (layout, evicted) =
            strategy.calculate(&windows, Rect::new(0, 0, 1000, 1000), 5, 0.6, 0, None);

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
        let strategy = DwindleBSPStrategy;
        let (layout, evicted) =
            strategy.calculate(&windows, Rect::new(0, 0, 1000, 600), 1, 0.6, 0, None);

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
