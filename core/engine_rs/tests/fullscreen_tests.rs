use std::collections::HashMap;
use raven_core::config::RavenConfig;
use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_engine::domain::geometry::{Rect, WindowNode};

/// Helper para crear nodos de ventana de prueba rápidamente
fn create_test_window(
    id: &str,
    workspace_id: &str,
    output: &str,
    is_floating: bool,
) -> WindowNode {
    WindowNode {
        window_id: id.to_string(),
        workspace_id: workspace_id.to_string(),
        output: output.to_string(),
        desktops: vec!["1".to_string()],
        is_floating,
        is_minimized: false,
        is_pip: false,
        geometry: Rect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        },
        min_w: 200,
        min_h: 200,
        strict_birth: false,
        is_quarantined: false,
        is_fullscreen: false,
        resource_class: String::new(),
        caption: String::new(),
        custom_w_ratio: None,
        custom_h_ratio: None,
    }
}

#[tokio::test]
async fn test_fullscreen_direct_transition_evicts_from_tiling_background() {
    let config = RavenConfig::default();
    let engine = TilingEngine::new(config);
    let mut controller = RavenController::new(engine);

    let mut workspaces = HashMap::new();
    let screen = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    workspaces.insert("ws1||HDMI-A-1".to_string(), screen);

    // 1. Estado inicial con 2 ventanas en mosaico (Navegador y Terminal)
    let nav_window = create_test_window("browser-1", "ws1||HDMI-A-1", "HDMI-A-1", false);
    let term_window = create_test_window("term-1", "ws1||HDMI-A-1", "HDMI-A-1", false);

    let actions = controller
        .handle_state_change(workspaces.clone(), vec![nav_window.clone(), term_window.clone()])
        .expect("Sincronización inicial exitosa");

    // Deben calcularse 2 comandos de movimiento dividiendo la pantalla
    assert_eq!(actions.len(), 2);

    // 2. Transición Directa a Fullscreen (sin maximizar previamente): el navegador pasa a is_floating = true
    let nav_fs = create_test_window("browser-1", "ws1||HDMI-A-1", "HDMI-A-1", true);

    let fs_actions = controller
        .handle_state_change(workspaces.clone(), vec![nav_fs.clone(), term_window.clone()])
        .expect("Sincronización en fullscreen exitosa");

    // La ventana restante (terminal) debe ocupar el 100% de la pantalla de fondo
    if let Some(raven_core::action::RavenAction::MoveWindow { window_id, width, height, .. }) = fs_actions.first() {
        assert_eq!(window_id, "term-1");
        assert!(width > &900 && height > &900, "Terminal debe ocupar el espacio vacante");
    }

    // 3. Salida de Fullscreen: el navegador regresa a is_floating = false
    let restored_actions = controller
        .handle_state_change(workspaces, vec![nav_window, term_window])
        .expect("Restauración de mosaico exitosa");

    // Se debe restaurar el mosaico de 2 ventanas
    assert_eq!(restored_actions.len(), 2);
}

#[tokio::test]
async fn test_simultaneous_fullscreen_across_multiple_monitors() {
    let config = RavenConfig::default();
    let engine = TilingEngine::new(config);
    let mut controller = RavenController::new(engine);

    // 3 Monitores independientes
    let mut workspaces = HashMap::new();
    let screen1 = Rect { x: 0, y: 0, width: 1920, height: 1080 };
    let screen2 = Rect { x: 1920, y: 0, width: 2560, height: 1440 };
    let screen3 = Rect { x: 4480, y: 0, width: 1920, height: 1080 };

    workspaces.insert("ws-mon1||HDMI-A-1".to_string(), screen1);
    workspaces.insert("ws-mon2||DP-1".to_string(), screen2);
    workspaces.insert("ws-mon3||DP-2".to_string(), screen3);

    // Crear 2 ventanas por monitor (Total 6 ventanas)
    let w1_1 = create_test_window("win-m1-1", "ws-mon1||HDMI-A-1", "HDMI-A-1", false);
    let w1_2 = create_test_window("win-m1-2", "ws-mon1||HDMI-A-1", "HDMI-A-1", false);

    let w2_1 = create_test_window("win-m2-1", "ws-mon2||DP-1", "DP-1", false);
    let w2_2 = create_test_window("win-m2-2", "ws-mon2||DP-1", "DP-1", false);

    let w3_1 = create_test_window("win-m3-1", "ws-mon3||DP-2", "DP-2", false);
    let w3_2 = create_test_window("win-m3-2", "ws-mon3||DP-2", "DP-2", false);

    let all_windows = vec![w1_1.clone(), w1_2.clone(), w2_1.clone(), w2_2.clone(), w3_1.clone(), w3_2.clone()];

    let initial_actions = controller
        .handle_state_change(workspaces.clone(), all_windows)
        .expect("Inicializar 3 monitores");
    assert_eq!(initial_actions.len(), 6);

    // Simular que win-m1-1 en HDMI-A-1 y win-m2-1 en DP-1 entran a FULLSCREEN en paralelo
    let w1_1_fs = create_test_window("win-m1-1", "ws-mon1||HDMI-A-1", "HDMI-A-1", true);
    let w2_1_fs = create_test_window("win-m2-1", "ws-mon2||DP-1", "DP-1", true);

    let fs_multimon_windows = vec![w1_1_fs, w1_2.clone(), w2_1_fs, w2_2.clone(), w3_1.clone(), w3_2.clone()];

    let fs_actions = controller
        .handle_state_change(workspaces.clone(), fs_multimon_windows)
        .expect("Manejo de múltiples fullscreen simultáneos en distintos monitores");

    // Verificar que win-m1-2 (monitor 1) ocupe la pantalla 1 completa
    // y win-m2-2 (monitor 2) ocupe la pantalla 2 completa sin colisionar
    for action in fs_actions {
        if let raven_core::action::RavenAction::MoveWindow { window_id, x, width, .. } = action {
            if window_id == "win-m1-2" {
                assert!(x <= 16, "Monitor 1 debe ubicarse con respecto a x=0");
                assert!(width > 1800, "Monitor 1 debe expandirse a cerca de 1920");
            }
            if window_id == "win-m2-2" {
                assert!((1920..=1936).contains(&x), "Monitor 2 debe ubicarse con respecto a x=1920");
                assert!(width > 2400, "Monitor 2 debe expandirse a cerca de 2560");
            }
        }
    }
}
