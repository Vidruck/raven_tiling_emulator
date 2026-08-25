//! # Orquestador de Topología Global y Soporte PiP
//!
//! Coordina la distribución de ventanas a través de múltiples workspaces y pantallas,
//! ejecutando la estrategia de layout correspondiente por workspace y superponiendo
//! las ventanas flotantes Picture-in-Picture (PiP) en sus esquinas configuradas.

use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

use super::strategy::get_strategy;

/// Calcula la topología global para todas las áreas de trabajo (workspaces) activas.
///
/// 1. Agrupa la lista global de ventanas según el `workspace_id` al que pertenecen.
/// 2. Invoca la estrategia de layout seleccionada (`Tall`, `Dwindle`, `Monocle`, etc.) para cada monitor.
/// 3. Superpone y posiciona las ventanas de reproducción "Pantalla en Pantalla" (PiP)
///    en la esquina solicitada (`top-left`, `top-right`, `bottom-left`, `bottom-right`)
///    evitando el solapamiento exacto mediante offsets de apilamiento vertical.
///
/// # Parámetros
/// - `windows`: Colección completa de nodos de ventana gestionados por el motor.
/// - `workspaces`: Mapa que vincula cada `workspace_id` con la geometría `Rect` de su monitor.
/// - `nmaster`: Cantidad de ventanas maestras configuradas.
/// - `master_ratio`: Relación de división de área maestra frente a secundaria.
/// - `default_gaps`: Espaciado interno en píxeles.
/// - `pip_position`: Ubicación deseada para las ventanas PiP (`"top-right"`, `"bottom-left"`, etc.).
/// - `layout_type`: Identificador de la estrategia de layout activa.
/// - `active_window_id`: Identificador opcional de la ventana enfocado en el sistema.
///
/// # Retorno
/// Tupla `(HashMap<WindowId, Rect>, Vec<EvictedWindowId>)` con las geometrías de todas las ventanas.
pub fn calculate_global_topology(
    windows: &[WindowNode],
    workspaces: &HashMap<String, Rect>,
    nmaster: usize,
    master_ratio: f32,
    default_gaps: i32,
    pip_position: &str,
    layout_type: &str,
    workspace_layouts: &HashMap<String, String>,
    active_window_id: Option<String>,
    pip_size_ratio: f32,
) -> (HashMap<String, Rect>, Vec<String>) {
    let mut global_layout = HashMap::new();
    let mut global_evicted = Vec::new();
    let mut windows_by_ws: HashMap<String, Vec<WindowNode>> = HashMap::new();

    // 1. Agrupar ventanas no flotantes (o PiP / Fullscreen) por workspace
    for win in windows {
        if !win.is_floating || win.is_pip || win.is_fullscreen {
            windows_by_ws
                .entry(win.workspace_id.clone())
                .or_insert_with(Vec::new)
                .push(win.clone());
        }
    }

    // 2. Procesar cada workspace con la estrategia elegida y superponer PiP / FullScreen
    for (ws_id, ws_windows) in windows_by_ws {
        let screen_rect_opt = workspaces.get(&ws_id).copied().or_else(|| {
            let output_prefix = ws_id.split("||").next()?;
            workspaces.iter().find(|(k, _)| k.starts_with(output_prefix)).map(|(_, r)| *r)
        });

        if let Some(screen_rect) = screen_rect_opt {
            // Filtrar ventanas no-fullscreen y no-pip para el mosaico de fondo
            let tiling_windows: Vec<WindowNode> = ws_windows
                .iter()
                .filter(|w| !w.is_fullscreen && !w.is_pip && !w.is_floating)
                .cloned()
                .collect();

            // Consultar layout específico del workspace o usar el global por defecto
            let current_layout_type = workspace_layouts
                .get(&ws_id)
                .map(|s| s.as_str())
                .unwrap_or(layout_type);

            // Instanciar estrategia según el nombre del layout para el fondo
            let strategy = get_strategy(current_layout_type);
            let (ws_layout, ws_evicted) = strategy.calculate(
                &tiling_windows,
                screen_rect,
                nmaster,
                master_ratio,
                default_gaps,
                active_window_id.clone(),
            );
            global_layout.extend(ws_layout);
            global_evicted.extend(ws_evicted);

            // Si hay ventanas en modo pantalla completa nativo, asignarles el área completa
            for win in &ws_windows {
                if win.is_fullscreen && !win.is_minimized {
                    global_layout.insert(win.window_id.clone(), screen_rect);
                }
            }

            // 3. Dimensionar y superponer ventanas Picture-in-Picture (PiP)
            let pip_w = (screen_rect.width as f32 * pip_size_ratio) as i32;
            let pip_h = (pip_w as f32 * 0.56) as i32; // Relación de aspecto ~16:9
            let pip_gap = default_gaps + 10;
            let mut pip_index = 0;

            for win in ws_windows {
                if win.is_pip && !win.is_minimized {
                    let final_pip_w = std::cmp::max(pip_w, win.min_w);
                    let final_pip_h = std::cmp::max(pip_h, win.min_h);

                    // Offset de apilamiento vertical si hay múltiples reproductores PiP
                    let offset_y = pip_index * (final_pip_h + pip_gap);

                    let mut x = screen_rect.x + pip_gap;
                    let mut y = screen_rect.y + pip_gap;

                    // Posicionar según la esquina seleccionada
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
                        "top" => {
                            x = screen_rect.x + (screen_rect.width - final_pip_w) / 2;
                            y += offset_y; // Apilar hacia abajo
                        }
                        "bottom" => {
                            x = screen_rect.x + (screen_rect.width - final_pip_w) / 2;
                            y = screen_rect.y + screen_rect.height - final_pip_h - pip_gap;
                            y -= offset_y; // Apilar hacia arriba
                        }
                        "left" => {
                            y = screen_rect.y + (screen_rect.height - final_pip_h) / 2;
                            y += offset_y; // Apilar hacia abajo
                        }
                        "right" => {
                            x = screen_rect.x + screen_rect.width - final_pip_w - pip_gap;
                            y = screen_rect.y + (screen_rect.height - final_pip_h) / 2;
                            y += offset_y; // Apilar hacia abajo
                        }
                        _ => {
                            // "top-left" por defecto
                            y += offset_y; // Apilar hacia abajo
                        }
                    }

                    // Prevenir que la ventana salga de los límites verticales visibles
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
    use crate::domain::layout::strategy::LayoutStrategy;
    use crate::domain::layout::DwindleBSPStrategy;

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
            false,
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

        assert!(evicted.is_empty());
        assert_eq!(layout.len(), 6);

        let r2 = layout.get("win_2").unwrap();
        let r6 = layout.get("win_6").unwrap();

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

        let strategy = DwindleBSPStrategy;
        let (layout, evicted) =
            strategy.calculate(&windows, Rect::new(0, 0, 1000, 600), 1, 0.6, 0, None);

        assert!(evicted.is_empty());
        assert_eq!(layout.len(), 5);

        let r1 = layout.get("win_1").unwrap();
        let r2 = layout.get("win_2").unwrap();
        let r3 = layout.get("win_3").unwrap();
        let r4 = layout.get("win_4").unwrap();
        let r5 = layout.get("win_5").unwrap();

        assert_eq!(r2.x, 0);
        assert_eq!(r2.width, 200);
        assert_eq!(r2.height, 600);

        assert_eq!(r3.x, 800);
        assert_eq!(r3.width, 200);
        assert_eq!(r3.height, 600);

        assert_eq!(r1.x, 200);
        assert_eq!(r1.width, 600);
        assert_eq!(r1.height, 420);

        assert_eq!(r4.x, 200);
        assert_eq!(r4.width, 300);
        assert_eq!(r4.height, 180);

        assert_eq!(r5.x, 500);
        assert_eq!(r5.width, 300);
        assert_eq!(r5.height, 180);
    }

    #[test]
    fn test_raven_base_3_windows_layout() {
        let windows = vec![
            mock_window("win_1"),
            mock_window("win_2"),
            mock_window("win_3"),
        ];

        let strategy = DwindleBSPStrategy;
        // Pantalla típica 1920x1080, ratio 0.65 (65% altura master superior, 35% inferior)
        let (layout, evicted) =
            strategy.calculate(&windows, Rect::new(0, 0, 1920, 1080), 1, 0.65, 0, None);

        assert!(evicted.is_empty());
        assert_eq!(layout.len(), 3);

        let r1 = layout.get("win_1").unwrap();
        let r2 = layout.get("win_2").unwrap();
        let r3 = layout.get("win_3").unwrap();

        // Ventana 1: Master superior completo en ancho
        assert_eq!(r1.x, 0);
        assert_eq!(r1.y, 0);
        assert_eq!(r1.width, 1920);
        assert_eq!(r1.height, 702); // 1080 - 378 (35% de 1080)

        // Ventana 2: Inferior izquierda
        assert_eq!(r2.x, 0);
        assert_eq!(r2.y, 702);
        assert_eq!(r2.width, 960);
        assert_eq!(r2.height, 378);

        // Ventana 3: Inferior derecha
        assert_eq!(r3.x, 960);
        assert_eq!(r3.y, 702);
        assert_eq!(r3.width, 960);
        assert_eq!(r3.height, 378);
    }

    #[test]
    fn test_dynamic_floating_stack_peek() {
        use crate::application::engine::TilingEngine;
        use crate::infrastructure::config::RavenConfig;

        let mut engine = TilingEngine::new(RavenConfig::default());
        let mut workspaces = HashMap::new();
        workspaces.insert("ws_1".to_string(), Rect::new(0, 0, 1920, 1080));

        let windows = vec![
            mock_window("ide_window"),
            mock_window("browser_window"),
            mock_window("image_viewer"),
        ];

        // 1. Con las 3 ventanas en tiling (Raven-base v3.2: 1 arriba, 2 abajo)
        let (layout_3, _) = engine
            .calculate_from_payload(&workspaces, &windows, None)
            .unwrap();
        assert_eq!(layout_3.len(), 3);
        assert!(layout_3.contains_key("image_viewer"));

        // 2. Activar modo flotante dinámico (Quick Peek) para image_viewer
        engine.dynamic_floating_windows.insert("image_viewer".to_string());

        let (layout_dyn_float, _) = engine
            .calculate_from_payload(&workspaces, &windows, None)
            .unwrap();
        // image_viewer ahora flota y no ocupa espacio en el mosaico
        assert_eq!(layout_dyn_float.len(), 2);
        assert!(!layout_dyn_float.contains_key("image_viewer"));
        assert!(layout_dyn_float.contains_key("ide_window"));
        assert!(layout_dyn_float.contains_key("browser_window"));

        // 3. Desactivar modo flotante dinámico (vuelve a la normalidad como nuevo)
        engine.dynamic_floating_windows.remove("image_viewer");

        let (layout_restored, _) = engine
            .calculate_from_payload(&workspaces, &windows, None)
            .unwrap();
        assert_eq!(layout_restored.len(), 3);
        assert!(layout_restored.contains_key("image_viewer"));
    }

    #[test]
    fn test_pip_detection_and_dynamic_float_precedence() {
        use crate::application::engine::TilingEngine;
        use crate::infrastructure::config::{RavenConfig, WindowRule};

        let mut config = RavenConfig::default();
        config.window_rules.push(WindowRule {
            class: "vlc".to_string(),
            action: "pip".to_string(),
            pip: true,
        });

        let mut engine = TilingEngine::new(config);
        let mut workspaces = HashMap::new();
        workspaces.insert("ws_1".to_string(), Rect::new(0, 0, 1920, 1080));

        let win_normal = mock_window("editor").with_class_and_caption("code".to_string(), "Editor".to_string());
        let win_pip_by_title = mock_window("firefox_pip").with_class_and_caption("firefox".to_string(), "Picture-in-Picture".to_string());
        let win_pip_by_rule = mock_window("vlc_video").with_class_and_caption("vlc".to_string(), "Movie.mp4".to_string());

        let windows = vec![win_normal, win_pip_by_title, win_pip_by_rule];

        // 1. Rust clasifica automáticamente a firefox y vlc como PiP sin saturar el mosaico
        let (layout, _) = engine.calculate_from_payload(&workspaces, &windows, None).unwrap();
        assert_eq!(layout.len(), 3);
        
        // Editor ocupa la pantalla de mosaico completa (1920x1080 menos gaps)
        let r_editor = layout.get("editor").unwrap();
        assert!(r_editor.width > 1500);

        // Firefox y VLC se posicionan como PiP de tamaño reducido
        let r_firefox = layout.get("firefox_pip").unwrap();
        assert!(r_firefox.width < 600); // 22% PiP scale

        // 2. Si el usuario fuerza a VLC con Quick Peek (Meta+Shift+F), Rust anula su modo PiP
        engine.dynamic_floating_windows.insert("vlc_video".to_string());
        let (layout_override, _) = engine.calculate_from_payload(&workspaces, &windows, None).unwrap();
        
        // vlc_video sale de la grilla de mosaico y de la lista de layouts forzados
        assert!(!layout_override.contains_key("vlc_video"));
        assert!(layout_override.contains_key("editor"));
        assert!(layout_override.contains_key("firefox_pip"));
    }
}
