//! # Pruebas de Ventanas Rebeldes y Resiliencia Geométrica (`rebellious_windows_tests`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Verifica la sanitización defensiva ante ventanas que exigen tamaños mínimos mayores
//! al monitor, ráfagas de nacimiento en cuarentena y geometrías corruptas/negativas.

mod common;

use std::sync::Arc;
use tokio::sync::Mutex;

use raven_engine::domain::geometry::{Rect, WindowNode};

#[tokio::test]
async fn test_rebellious_window_eviction() {
    let mut controller = common::default_controller();
    let workspaces = common::single_workspace("default||default", 1000, 1000);

    let mut windows = Vec::new();
    for i in 0..4 {
        let mut win = common::make_test_window(&format!("win-{i}"), "default||default", "default", "");
        win.geometry = Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        win.min_w = if i == 2 { 2000 } else { 0 };
        win.min_h = 0;
        windows.push(win);
    }

    let commands = controller
        .handle_state_change(workspaces, windows)
        .expect("handle_state_change no debe fallar ante min_w excesivo");

    let win2_command = commands.iter().find(|cmd| match cmd {
        raven_engine::domain::action::RavenAction::MoveWindow {
            window_id, width, ..
        } => window_id == "win-2" && *width <= 1000,
        _ => false,
    });

    assert!(
        win2_command.is_some(),
        "Rebellious window should be safely clamped within screen bounds"
    );
}

#[tokio::test]
async fn test_rebellious_window_flood() {
    let controller = Arc::new(Mutex::new(common::default_controller()));
    let output = "eDP-1";
    let desktop = "Desk1";
    let workspace_id = format!("{desktop}||{output}");
    let workspaces = common::single_workspace(&workspace_id, 1920, 1080);

    // 1. Simular rebelde enviando geometrías basura rápidamente (tormenta de nacimiento)
    for i in 0..100 {
        let mut rebel = common::make_test_window("rebel-1", &workspace_id, output, desktop);
        rebel.geometry = Rect {
            x: i,
            y: i,
            width: 800 + i,
            height: 600 + i,
        };
        rebel.min_w = 500;
        rebel.min_h = 500;
        rebel.strict_birth = true;
        rebel.is_quarantined = true;

        let mut guard = controller.lock().await;
        let actions = guard
            .handle_state_change(workspaces.clone(), vec![rebel])
            .unwrap();
        assert!(!actions.is_empty());
    }

    // 2. Llega una segunda aplicación normal y la tormenta cesa
    let mut rebel_settled = common::make_test_window("rebel-1", &workspace_id, output, desktop);
    rebel_settled.min_w = 500;
    rebel_settled.min_h = 500;

    let mut good_app = common::make_test_window("good-app", &workspace_id, output, desktop);
    good_app.geometry = Rect {
        x: 0,
        y: 0,
        width: 200,
        height: 200,
    };

    let mut guard = controller.lock().await;
    let actions = guard
        .handle_state_change(workspaces, vec![rebel_settled, good_app])
        .unwrap();

    let mut move_count = 0;
    for action in actions {
        if let raven_core::action::RavenAction::MoveWindow { width, .. } = action {
            assert!(width > 800 && width < 1000);
            move_count += 1;
        }
    }

    assert_eq!(move_count, 2);
}

#[test]
fn test_corrupted_negative_geometry_sanitization() {
    let win = WindowNode::new(
        "corrupt-1".to_string(),
        "DP-1||1".to_string(),
        "DP-1".to_string(),
        vec!["1".to_string()],
        false,
        false,
        false,
        Rect::new(-50, -20, -800, 0),
        -300,
        -150,
        false,
        false,
        false,
    );

    assert!(win.geometry.width >= 1);
    assert!(win.geometry.height >= 1);
    assert_eq!(win.min_w, 0);
    assert_eq!(win.min_h, 0);
}

#[tokio::test]
async fn test_two_browsers_enforce_50_50_split_without_70_30_distortion() {
    for layout_name in [
        "raven",
        "strict_dwindle",
        "inverted_strict_dwindle",
        "tall",
        "divisor",
    ] {
        let config = raven_core::config::RavenConfig {
            default_gaps: 10,
            master_ratio: 0.5,
            nmaster: 1,
            layout_type: layout_name.to_string(),
            ..Default::default()
        };
        let engine = raven_engine::application::engine::TilingEngine::new(config);
        let mut controller = raven_engine::application::controller::RavenController::new(engine);

        let ws = "DP-1||desk_1";
        let workspaces = common::single_workspace(ws, 1920, 1080);

        let mut browser_a = common::make_test_window("browser-1", ws, "DP-1", "desk_1");
        browser_a.resource_class = "firefox".to_string();
        browser_a.min_w = 1350; // Simula navegador reportando 70% de la pantalla como min_w
        browser_a.min_h = 800;

        let mut browser_b = common::make_test_window("browser-2", ws, "DP-1", "desk_1");
        browser_b.resource_class = "brave-browser".to_string();
        browser_b.min_w = 1350;
        browser_b.min_h = 800;

        let actions = controller
            .handle_state_change(workspaces, vec![browser_a, browser_b])
            .expect("El cálculo de layout para dos navegadores no debe fallar");

        let mut widths = Vec::new();
        for act in actions {
            if let raven_core::action::RavenAction::MoveWindow { width, .. } = act {
                widths.push(width);
            }
        }

        assert_eq!(
            widths.len(),
            2,
            "Se esperaban 2 acciones MoveWindow en el algoritmo {layout_name}"
        );
        let diff = (widths[0] - widths[1]).abs();
        assert!(
            diff <= 2 && widths[0] >= 940 && widths[0] <= 960,
            "El algoritmo {layout_name} distorsionó el 50/50 de dos navegadores a {:?} (diff={diff})",
            widths
        );
    }
}


