//! # Pruebas de Flotación Dinámica Quick Peek (`floating_tests`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Verifica la transición masiva de ventanas entre el árbol de mosaico y la pila
//! flotante dinámica (`Meta+Shift+F`), garantizando reconstrucción limpia y sin fugas.

mod common;

use raven_engine::domain::geometry::{Rect, Topology};

#[tokio::test]
async fn test_all_windows_dynamically_floated_and_restored() {
    let mut controller = common::default_controller();
    let topology = Topology {
        outputs: vec!["DP-1".to_string()],
        output_nodes: vec![],
        desktops: vec!["desk_1".to_string()],
        current_desktop: "desk_1".to_string(),
    };

    let workspace_id = "DP-1||desk_1";
    let workspaces = common::single_workspace(workspace_id, 1920, 1080);

    let window_count = 10;
    let mut windows = Vec::with_capacity(window_count);
    for i in 0..window_count {
        let mut win = common::make_test_window(&format!("app-{i}"), workspace_id, "DP-1", "desk_1");
        win.geometry = Rect {
            x: 0,
            y: 0,
            width: 400,
            height: 300,
        };
        windows.push(win);
    }

    let initial_actions = controller
        .handle_state_change(workspaces, windows)
        .unwrap();
    assert!(
        !initial_actions.is_empty(),
        "El estado inicial de 10 ventanas debe generar layout"
    );

    // 1. Conmutar a flotante dinámico las 10 ventanas consecutivamente
    for i in 0..window_count {
        let win_id = format!("app-{i}");
        let (needs_recalc, shortcut_cmds) = controller
            .handle_shortcut(
                "toggle_floating".to_string(),
                0,
                Some(win_id.clone()),
                &topology,
            )
            .expect("handle_shortcut no debe fallar con toggle_floating");

        assert!(needs_recalc);
        assert_eq!(shortcut_cmds.len(), 2);

        match &shortcut_cmds[0] {
            raven_core::action::RavenAction::SetFloating {
                window_id,
                floating,
                keep_above,
            } => {
                assert_eq!(window_id, &win_id);
                assert!(*floating);
                assert!(*keep_above);
            }
            _ => panic!("Comando inesperado retornado por toggle_floating"),
        }

        match &shortcut_cmds[1] {
            raven_core::action::RavenAction::MoveWindow { window_id, .. } => {
                assert_eq!(window_id, &win_id);
            }
            _ => panic!("Comando de reescalamiento flotante esperado"),
        }

        let recalc_cmds = controller
            .commit_layout()
            .expect("commit_layout no debe entrar en pánico");

        let remaining_tiled = window_count - 1 - i;
        if remaining_tiled == 0 {
            assert!(
                recalc_cmds.is_empty(),
                "Con 0 ventanas en mosaico no debe haber comandos de movimiento"
            );
        }
    }

    assert_eq!(
        controller.get_engine().dynamic_floating_windows.len(),
        window_count
    );

    // 2. Restaurar todas las ventanas al mosaico
    for i in 0..window_count {
        let win_id = format!("app-{i}");
        let (needs_recalc, shortcut_cmds) = controller
            .handle_shortcut(
                "toggle_floating".to_string(),
                0,
                Some(win_id.clone()),
                &topology,
            )
            .expect("handle_shortcut debe permitir restaurar ventanas al mosaico");

        assert!(needs_recalc);
        assert_eq!(shortcut_cmds.len(), 1);

        match &shortcut_cmds[0] {
            raven_core::action::RavenAction::SetFloating {
                window_id,
                floating,
                keep_above,
            } => {
                assert_eq!(window_id, &win_id);
                assert!(!*floating);
                assert!(!*keep_above);
            }
            _ => panic!("Comando inesperado retornado por toggle_floating al restaurar"),
        }

        let recalc_cmds = controller
            .commit_layout()
            .expect("commit_layout debe reconstruir el mosaico");
        assert!(
            !recalc_cmds.is_empty(),
            "Al reinsertar ventanas, el motor recalcula geometrías válidas"
        );
        let has_move_target = recalc_cmds.iter().any(|c| match c {
            raven_core::action::RavenAction::MoveWindow { window_id, .. } => window_id == &win_id,
            _ => false,
        });
        assert!(
            has_move_target,
            "La ventana {win_id} restaurada al mosaico debe recibir comando MoveWindow"
        );
    }

    assert_eq!(controller.get_engine().dynamic_floating_windows.len(), 0);
}

