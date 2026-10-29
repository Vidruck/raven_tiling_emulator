//! # Pruebas de Estabilidad Espacial y Foco (`spatial_stability_tests`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Garantiza que al enfocar o activar diferentes ventanas en el mosaico,
//! el orden espacial (`spatial_order`) y las geometrías asignadas permanezcan inmutables.

mod common;

use std::collections::HashMap;
use raven_core::config::RavenConfig;
use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_engine::domain::geometry::Rect;

#[tokio::test]
async fn test_window_focus_does_not_swap_spatial_order() {
    let config = RavenConfig {
        default_gaps: 10,
        master_ratio: 0.6,
        nmaster: 1,
        layout_type: "raven".to_string(),
        ..Default::default()
    };
    let engine = TilingEngine::new(config.clone());
    let mut controller = RavenController::new(engine);

    let ws = "DP-1||desk_1";
    let workspaces = common::single_workspace(ws, 1920, 1080);

    let windows = vec![
        common::make_test_window("win-1", ws, "DP-1", "desk_1"),
        common::make_test_window("win-2", ws, "DP-1", "desk_1"),
        common::make_test_window("win-3", ws, "DP-1", "desk_1"),
        common::make_test_window("win-4", ws, "DP-1", "desk_1"),
    ];

    let actions_init = controller
        .handle_state_change(workspaces.clone(), windows.clone())
        .unwrap();
    let mut geoms_initial: HashMap<String, Rect> = HashMap::new();
    for act in &actions_init {
        if let raven_core::action::RavenAction::MoveWindow {
            window_id,
            x,
            y,
            width,
            height,
        } = act
        {
            geoms_initial.insert(
                window_id.clone(),
                Rect {
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                },
            );
        }
    }
    assert_eq!(geoms_initial.len(), 4);

    controller.active_window_id = Some("win-2".to_string());
    let actions_after_focus = controller
        .handle_state_change(workspaces, windows)
        .unwrap();
    let mut geoms_after_focus: HashMap<String, Rect> = HashMap::new();
    for act in &actions_after_focus {
        if let raven_core::action::RavenAction::MoveWindow {
            window_id,
            x,
            y,
            width,
            height,
        } = act
        {
            geoms_after_focus.insert(
                window_id.clone(),
                Rect {
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                },
            );
        }
    }

    for id in ["win-1", "win-2", "win-3", "win-4"] {
        assert_eq!(
            geoms_initial.get(id),
            geoms_after_focus.get(id),
            "Regresión detectada: La geometría de {id} cambió simplemente al recibir foco!"
        );
    }
}

#[tokio::test]
async fn test_interactive_mouse_resize_70_30_to_50_50_and_60_40() {
    let config = RavenConfig {
        default_gaps: 10,
        master_ratio: 0.7,
        nmaster: 1,
        layout_type: "raven".to_string(),
        ..Default::default()
    };
    let engine = TilingEngine::new(config.clone());
    let mut controller = RavenController::new(engine);

    let ws = "DP-1||desk_1";
    let workspaces = common::single_workspace(ws, 1920, 1080);

    let windows = vec![
        common::make_test_window("win-1", ws, "DP-1", "desk_1"),
        common::make_test_window("win-2", ws, "DP-1", "desk_1"),
    ];

    // 1. Disposición inicial 70/30
    let _ = controller.handle_state_change(workspaces, windows).unwrap();

    // 2. El usuario arrastra el borde de win-1 con el ratón hacia la mitad de la pantalla (~945px -> 50/50)
    let actions_50 = controller
        .handle_user_resize("win-1", Rect::new(10, 10, 945, 1060))
        .unwrap();
    let mut widths_50 = HashMap::new();
    for act in &actions_50 {
        if let raven_core::action::RavenAction::MoveWindow {
            window_id, width, ..
        } = act
        {
            widths_50.insert(window_id.clone(), *width);
        }
    }
    assert!(
        (widths_50["win-1"] - widths_50["win-2"]).abs() <= 4,
        "Tras redimensionar a ~50/50, ambas ventanas deben ser iguales, pero fueron: {:?}",
        widths_50
    );

    // 3. El usuario ahora arrastra win-1 para hacer una disposición 60/40 (~1134px)
    let actions_60 = controller
        .handle_user_resize("win-1", Rect::new(10, 10, 1134, 1060))
        .unwrap();
    let mut widths_60 = HashMap::new();
    for act in &actions_60 {
        if let raven_core::action::RavenAction::MoveWindow {
            window_id, width, ..
        } = act
        {
            widths_60.insert(window_id.clone(), *width);
        }
    }
    assert!(
        widths_60["win-1"] > widths_60["win-2"] + 300,
        "Tras redimensionar a ~60/40, win-1 debe dominar la proporción 60/40, pero fueron: {:?}",
        widths_60
    );
}

#[tokio::test]
async fn test_interactive_mouse_resize_neighbor_reactivity() {
    let config = RavenConfig {
        default_gaps: 10,
        master_ratio: 0.5,
        nmaster: 1,
        layout_type: "raven".to_string(),
        ..Default::default()
    };
    let engine = TilingEngine::new(config.clone());
    let mut controller = RavenController::new(engine);

    let ws = "DP-1||desk_1";
    let workspaces = common::single_workspace(ws, 1920, 1080);

    let windows = vec![
        common::make_test_window("win-left", ws, "DP-1", "desk_1"),
        common::make_test_window("win-right", ws, "DP-1", "desk_1"),
    ];

    let _ = controller.handle_state_change(workspaces, windows).unwrap();

    // Redimensionar la ventana izquierda reduciéndola a 500px
    let actions = controller
        .handle_user_resize("win-left", Rect::new(10, 10, 500, 1060))
        .unwrap();

    let mut right_geom: Option<Rect> = None;
    for act in actions {
        if let raven_core::action::RavenAction::MoveWindow {
            window_id,
            x,
            y,
            width,
            height,
        } = act
        {
            if window_id == "win-right" {
                right_geom = Some(Rect {
                    x,
                    y,
                    width,
                    height,
                });
            }
        }
    }

    assert!(
        right_geom.is_some(),
        "La ventana vecina win-right DEBE recibir un comando MoveWindow reactivo"
    );
    let rg = right_geom.unwrap();
    // La ventana vecina debe haber crecido reactivamente para ocupar el espacio liberado (1390px)
    assert!(
        rg.width >= 1300,
        "win-right no reaccionó apropiadamente: {:?}",
        rg
    );
    assert_eq!(
        rg.width + 500 + 3 * config.default_gaps,
        1920,
        "La suma de ambas ventanas más los gaps debe completar el monitor: {:?}",
        rg
    );
}

#[tokio::test]
async fn test_interactive_mouse_drag_to_swap_windows() {
    let config = RavenConfig {
        default_gaps: 10,
        master_ratio: 0.5,
        nmaster: 1,
        layout_type: "strict_dwindle".to_string(),
        ..Default::default()
    };
    let engine = TilingEngine::new(config.clone());
    let mut controller = RavenController::new(engine);

    let ws = "DP-1||desk_1";
    let workspaces = common::single_workspace(ws, 1920, 1080);

    let windows = vec![
        common::make_test_window("win-left", ws, "DP-1", "desk_1"),
        common::make_test_window("win-right", ws, "DP-1", "desk_1"),
    ];

    let actions_init = controller.handle_state_change(workspaces, windows).unwrap();
    let mut initial_x = HashMap::new();
    for act in actions_init {
        if let raven_core::action::RavenAction::MoveWindow { window_id, x, .. } = act {
            initial_x.insert(window_id, x);
        }
    }
    assert!(initial_x["win-left"] < initial_x["win-right"]);

    // El usuario arrastra win-left con el ratón y lo suelta sobre la ranura derecha (x=1400, y=500)
    let actions_swap = controller
        .handle_user_move("win-left", 1400, 500)
        .unwrap();
    let mut swapped_x = HashMap::new();
    for act in actions_swap {
        if let raven_core::action::RavenAction::MoveWindow { window_id, x, .. } = act {
            swapped_x.insert(window_id, x);
        }
    }
    assert!(
        swapped_x["win-right"] < swapped_x["win-left"],
        "Las ventanas deberían haber intercambiado sus ranuras tras el arrastre interactivo"
    );
}


