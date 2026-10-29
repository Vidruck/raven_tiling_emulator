//! # Pruebas de Atajos de Teclado y Navegación (`shortcuts_tests`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Verifica la ejecución de todos los atajos de teclado de Raven:
//! gaps, capacidad master, ratios, ciclado de layouts, redimensionamiento fino,
//! navegación direccional/cíclica de foco, swaps y migración entre pantallas/escritorios.

mod common;

use raven_core::config::RavenConfig;
use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_engine::domain::geometry::Rect;

#[tokio::test]
async fn test_all_keyboard_shortcuts_execution_and_effects() {
    let config = RavenConfig {
        default_gaps: 10,
        master_ratio: 0.5,
        nmaster: 1,
        layout_type: "raven".to_string(),
        ..Default::default()
    };
    let engine = TilingEngine::new(config);
    let mut controller = RavenController::new(engine);

    let topology = common::dual_topology();
    let ws1 = "DP-1||desk_1";
    let workspaces = common::single_workspace(ws1, 1920, 1080);

    let mut w1 = common::make_test_window("win-1", ws1, "DP-1", "desk_1");
    w1.geometry = Rect { x: 0, y: 0, width: 960, height: 1080 };
    w1.resource_class = "terminal".to_string();

    let mut w2 = common::make_test_window("win-2", ws1, "DP-1", "desk_1");
    w2.geometry = Rect { x: 960, y: 0, width: 960, height: 540 };
    w2.resource_class = "editor".to_string();

    let mut w3 = common::make_test_window("win-3", ws1, "DP-1", "desk_1");
    w3.geometry = Rect { x: 960, y: 540, width: 960, height: 540 };
    w3.resource_class = "browser".to_string();

    let mut windows = vec![w1, w2, w3];

    let init_actions = controller
        .handle_state_change(workspaces.clone(), windows.clone())
        .unwrap();
    assert_eq!(init_actions.len(), 3);

    controller.active_window_id = Some("win-1".to_string());

    // --- A. Atajos de Gaps ---
    let (recalc, _) = controller
        .handle_shortcut("increment_gaps".to_string(), 2, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert_eq!(controller.get_config().default_gaps, 12);

    let (recalc, _) = controller
        .handle_shortcut("increment_gaps".to_string(), -2, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert_eq!(controller.get_config().default_gaps, 10);

    // --- B. Atajos de Master (Capacidad y Ratio) ---
    let (recalc, _) = controller
        .handle_shortcut("increment_nmaster".to_string(), 1, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert_eq!(controller.get_config().nmaster, 2);

    let (recalc, _) = controller
        .handle_shortcut("decrement_nmaster".to_string(), 1, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert_eq!(controller.get_config().nmaster, 1);

    let initial_ratio = controller.get_config().master_ratio;
    let (recalc, _) = controller
        .handle_shortcut("increase_ratio".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert!((controller.get_config().master_ratio - (initial_ratio + 0.05)).abs() < 0.001);

    let (recalc, _) = controller
        .handle_shortcut("decrease_ratio".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert!((controller.get_config().master_ratio - initial_ratio).abs() < 0.001);

    // --- C. Atajos de Ciclado de Layout ---
    let (recalc, _) = controller
        .handle_shortcut("cycle_layout".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    let cur_layout = controller
        .get_config()
        .workspace_layouts
        .get(ws1)
        .cloned()
        .unwrap_or_else(|| controller.get_config().layout_type.clone());
    assert_eq!(cur_layout, "tall");

    let move_cmds = controller.commit_layout().unwrap();
    assert!(!move_cmds.is_empty());
    for cmd in move_cmds {
        if let raven_core::action::RavenAction::MoveWindow {
            window_id,
            x,
            y,
            width,
            height,
        } = cmd
        {
            if let Some(w) = windows.iter_mut().find(|w| w.window_id == window_id) {
                w.geometry = Rect { x, y, width, height };
            }
        }
    }
    let _ = controller
        .handle_state_change(workspaces, windows)
        .unwrap();

    // --- D. Atajos de Redimensionamiento Fino ---
    let (recalc, _) = controller
        .handle_shortcut("resize_width_inc".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert!(
        controller
            .get_engine()
            .current_windows
            .get("win-1")
            .unwrap()
            .custom_w_ratio
            .unwrap()
            > 1.0
    );

    let (recalc, _) = controller
        .handle_shortcut("resize_width_dec".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert!(
        (controller
            .get_engine()
            .current_windows
            .get("win-1")
            .unwrap()
            .custom_w_ratio
            .unwrap()
            - 1.0)
            .abs()
            < 0.001
    );

    let (recalc, _) = controller
        .handle_shortcut("resize_height_inc".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert!(
        controller
            .get_engine()
            .current_windows
            .get("win-1")
            .unwrap()
            .custom_h_ratio
            .unwrap()
            > 1.0
    );

    let (recalc, _) = controller
        .handle_shortcut("resize_height_dec".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);
    assert!(
        (controller
            .get_engine()
            .current_windows
            .get("win-1")
            .unwrap()
            .custom_h_ratio
            .unwrap()
            - 1.0)
            .abs()
            < 0.001
    );

    // --- E. Atajos de Navegación de Foco ---
    let (_, cmds) = controller
        .handle_shortcut("focus_next".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(
        cmds[0],
        raven_core::action::RavenAction::FocusWindow {
            window_id: "win-2".to_string()
        }
    );

    let (_, cmds) = controller
        .handle_shortcut("focus_prev".to_string(), 0, Some("win-2".to_string()), &topology)
        .unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(
        cmds[0],
        raven_core::action::RavenAction::FocusWindow {
            window_id: "win-1".to_string()
        }
    );

    let (_, cmds) = controller
        .handle_shortcut("focus_right".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert_eq!(cmds.len(), 1);
    let right_target = match &cmds[0] {
        raven_core::action::RavenAction::FocusWindow { window_id } => {
            assert!(window_id == "win-2" || window_id == "win-3");
            window_id.clone()
        }
        _ => panic!("Esperado FocusWindow a la derecha"),
    };

    let (_, cmds) = controller
        .handle_shortcut("focus_left".to_string(), 0, Some(right_target), &topology)
        .unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(
        cmds[0],
        raven_core::action::RavenAction::FocusWindow {
            window_id: "win-1".to_string()
        }
    );

    let (_, cmds) = controller
        .handle_shortcut("focus_down".to_string(), 0, Some("win-2".to_string()), &topology)
        .unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(
        cmds[0],
        raven_core::action::RavenAction::FocusWindow {
            window_id: "win-3".to_string()
        }
    );

    let (_, cmds) = controller
        .handle_shortcut("focus_up".to_string(), 0, Some("win-3".to_string()), &topology)
        .unwrap();
    assert_eq!(cmds.len(), 1);
    assert_eq!(
        cmds[0],
        raven_core::action::RavenAction::FocusWindow {
            window_id: "win-2".to_string()
        }
    );

    // --- F. Intercambio de Posiciones (Swap) ---
    let (recalc, _) = controller
        .handle_shortcut("swap_next".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);

    let (recalc, _) = controller
        .handle_shortcut("swap_prev".to_string(), 0, Some("win-1".to_string()), &topology)
        .unwrap();
    assert!(recalc);

    // --- G. Migración de Escritorios y Pantallas ---
    let (recalc, cmds) = controller
        .handle_shortcut(
            "migrate_active_to_desktop".to_string(),
            0,
            Some("win-1".to_string()),
            &topology,
        )
        .unwrap();
    assert!(recalc);
    assert_eq!(
        cmds,
        vec![raven_core::action::RavenAction::MigrateToDesktop {
            window_id: "win-1".to_string(),
            target_desktop: "desk_2".to_string(),
        }]
    );

    let (recalc, cmds) = controller
        .handle_shortcut(
            "migrate_active_to_prev_desktop".to_string(),
            0,
            Some("win-1".to_string()),
            &topology,
        )
        .unwrap();
    assert!(recalc);
    assert_eq!(
        cmds,
        vec![raven_core::action::RavenAction::MigrateToDesktop {
            window_id: "win-1".to_string(),
            target_desktop: "desk_1".to_string(),
        }]
    );

    let (recalc, cmds) = controller
        .handle_shortcut(
            "migrate_active_to_screen".to_string(),
            0,
            Some("win-1".to_string()),
            &topology,
        )
        .unwrap();
    assert!(recalc);
    assert_eq!(
        cmds,
        vec![raven_core::action::RavenAction::MigrateToOutput {
            window_id: "win-1".to_string(),
            target_output: "HDMI-A-1".to_string(),
        }]
    );

    let (recalc, cmds) = controller
        .handle_shortcut(
            "migrate_active_to_prev_screen".to_string(),
            0,
            Some("win-1".to_string()),
            &topology,
        )
        .unwrap();
    assert!(recalc);
    assert_eq!(
        cmds,
        vec![raven_core::action::RavenAction::MigrateToOutput {
            window_id: "win-1".to_string(),
            target_output: "DP-1".to_string(),
        }]
    );

    // --- H. Alternar Tiling On/Off ---
    let (recalc, _) = controller
        .handle_shortcut("toggle_tiling".to_string(), 0, None, &topology)
        .unwrap();
    assert!(recalc);
    assert!(!controller.is_tiling_enabled());

    let (recalc, _) = controller
        .handle_shortcut("toggle_tiling".to_string(), 0, None, &topology)
        .unwrap();
    assert!(recalc);
    assert!(controller.is_tiling_enabled());

    // --- I. Atajos de Minimizar y Cerrar Ventana ---
    let (recalc, cmds) = controller
        .handle_shortcut(
            "minimize_active".to_string(),
            0,
            Some("win-1".to_string()),
            &topology,
        )
        .unwrap();
    assert!(recalc);
    assert!(cmds.iter().any(|cmd| matches!(
        cmd,
        raven_core::action::RavenAction::MinimizeWindow { window_id } if window_id == "win-1"
    )));
    assert!(
        controller
            .get_engine()
            .current_windows
            .get("win-1")
            .unwrap()
            .is_minimized
    );
    let recalc_cmds = controller.commit_layout().unwrap();
    assert!(
        !recalc_cmds.iter().any(|cmd| matches!(
            cmd,
            raven_core::action::RavenAction::MoveWindow { window_id, .. } if window_id == "win-1"
        )),
        "commit_layout no debe emitir MoveWindow para una ventana recién minimizada"
    );

    let (recalc, cmds) = controller
        .handle_shortcut(
            "close_active".to_string(),
            0,
            Some("win-2".to_string()),
            &topology,
        )
        .unwrap();
    assert!(!recalc);
    assert_eq!(
        cmds,
        vec![raven_core::action::RavenAction::CloseWindow {
            window_id: "win-2".to_string()
        }]
    );
}

