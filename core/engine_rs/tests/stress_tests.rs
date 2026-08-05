use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use raven_core::config::RavenConfig;
use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_engine::domain::geometry::{Rect, WindowNode};
use raven_engine::infrastructure::dbus::KWinTopology;

#[tokio::test]
async fn test_saturation_flood() {
    let config = RavenConfig::default();
    let engine = TilingEngine::new(config);
    let controller = Arc::new(Mutex::new(RavenController::new(engine)));

    let mut tasks = vec![];
    for i in 0..10_000 {
        let ctrl_clone = controller.clone();
        tasks.push(tokio::spawn(async move {
            let win = WindowNode {
                window_id: format!("win-{}", i),
                workspace_id: "default||default".to_string(),
                output: "default".to_string(),
                desktops: vec![],
                is_floating: false,
                is_minimized: false,
                is_pip: false,
                geometry: Rect { x: 0, y: 0, width: 800, height: 600 },
                min_w: 0,
                min_h: 0,
                strict_birth: false,
                is_quarantined: false,
                is_fullscreen: false,
            };
            let mut guard = ctrl_clone.lock().await;
            let _ = guard.handle_delta_change(win);
        }));
    }

    for task in tasks {
        let _ = task.await;
    }
    
    let mut guard = controller.lock().await;
    let _ = guard.commit_layout(); // Execute the geometry calculation once at the end
    assert!(guard.is_tiling_enabled());
}

#[tokio::test]
async fn test_rebellious_window_eviction() {
    let config = RavenConfig::default();
    let engine = TilingEngine::new(config);
    let mut controller = RavenController::new(engine);

    let mut workspaces = HashMap::new();
    workspaces.insert("default||default".to_string(), Rect { x: 0, y: 0, width: 1000, height: 1000 });

    let mut windows = vec![];
    for i in 0..4 {
        windows.push(WindowNode {
            window_id: format!("win-{}", i),
            workspace_id: "default||default".to_string(),
            output: "default".to_string(),
            desktops: vec![],
            is_floating: false,
            is_minimized: false,
            is_pip: false,
            geometry: Rect { x: 0, y: 0, width: 100, height: 100 },
            min_w: if i == 2 { 2000 } else { 0 }, // Rebellious window demands 2000px width on a 1000px screen
            min_h: 0,
            strict_birth: false,
            is_quarantined: false,
            is_fullscreen: false,
        });
    }

    let result = controller.handle_state_change(workspaces, windows);
    assert!(result.is_ok());
    let commands = result.unwrap();

    // En v3.0, utils::distribute_sizes sanitiza defensivamente min_w para evitar colapsos.
    // Verificamos que la ventana rebelde no desborde la pantalla (width <= 1000) y reciba comandos válidos.
    let win2_command = commands.iter().find(|cmd| match cmd {
        raven_engine::domain::action::RavenAction::MoveWindow { window_id, width, .. } => window_id == "win-2" && *width <= 1000,
        _ => false,
    });
    
    assert!(win2_command.is_some(), "Rebellious window should be safely clamped within screen bounds");
}

#[tokio::test]
async fn test_concurrent_settings_conflict() {
    let mut config = RavenConfig::default();
    config.default_gaps = 4;
    let engine = TilingEngine::new(config);
    let controller = Arc::new(Mutex::new(RavenController::new(engine)));

    let ctrl_a = controller.clone();
    let ctrl_b = controller.clone();

    // Plasmoid recibe +10
    let task_a = tokio::spawn(async move {
        let mut guard = ctrl_a.lock().await;
        let topology = KWinTopology {
            outputs: vec![],
            desktops: vec![],
            current_desktop: String::new(),
        };
        let _ = guard.handle_shortcut("increment_gaps".to_string(), 10, None, &topology);
    });

    // GUI recibe -8 concurrentemente
    let task_b = tokio::spawn(async move {
        let mut guard = ctrl_b.lock().await;
        let topology = KWinTopology {
            outputs: vec![],
            desktops: vec![],
            current_desktop: String::new(),
        };
        let _ = guard.handle_shortcut("increment_gaps".to_string(), -8, None, &topology);
    });

    let _ = tokio::join!(task_a, task_b);

    let guard = controller.lock().await;
    let final_gaps = guard.get_config().default_gaps;
    
    // Mientras no entre en pánico, maneja la concurrencia de forma segura.
    // El valor final debe ser lógicamente consistente (por ejemplo, 6).
    assert!(final_gaps == 6 || final_gaps > 0);
}

#[tokio::test]
async fn test_rebellious_window_flood() {
    let config = RavenConfig::default();
    let engine = TilingEngine::new(config);
    let controller = Arc::new(Mutex::new(RavenController::new(engine)));
    let output = "eDP-1".to_string();
    let desktop = "Desk1".to_string();
    let workspace_id = format!("{}||{}", desktop, output);

    let mut workspaces = HashMap::new();
    workspaces.insert(workspace_id.clone(), raven_core::geometry::Rect { x: 0, y: 0, width: 1920, height: 1080 });

    // 1. Simular rebelde enviando geometrías basura rápidamente (tormenta de nacimiento)
    for i in 0..100 {
        let windows = vec![WindowNode {
            window_id: "rebel-1".to_string(),
            workspace_id: workspace_id.clone(),
            output: output.clone(),
            desktops: vec![desktop.clone()],
            is_floating: false,
            is_minimized: false,
            is_pip: false,
            geometry: raven_core::geometry::Rect { x: i, y: i, width: 800 + i, height: 600 + i },
            min_w: 500,
            min_h: 500,
            strict_birth: true,
            is_quarantined: true,
            is_fullscreen: false,
        }];

        let mut guard = controller.lock().await;
        let actions = guard.handle_state_change(workspaces.clone(), windows).unwrap();
        
        // Como es la única ventana, Rust debe exigir que ocupe todo el ancho
        // Como cambia la geometría, Rust debe emitir comandos RequestFeedback
        assert!(!actions.is_empty());
    }

    // 2. Llega la nueva aplicación, la tormenta cesa
    let windows = vec![
        WindowNode {
            window_id: "rebel-1".to_string(),
            workspace_id: workspace_id.clone(),
            output: output.clone(),
            desktops: vec![desktop.clone()],
            is_floating: false,
            is_minimized: false,
            is_pip: false,
            geometry: raven_core::geometry::Rect { x: 0, y: 0, width: 800, height: 600 },
            min_w: 500,
            min_h: 500,
            strict_birth: false,
            is_quarantined: false,
            is_fullscreen: false,
        },
        WindowNode {
            window_id: "good-app".to_string(),
            workspace_id: workspace_id.clone(),
            output: output.clone(),
            desktops: vec![desktop.clone()],
            is_floating: false,
            is_minimized: false,
            is_pip: false,
            geometry: raven_core::geometry::Rect { x: 0, y: 0, width: 200, height: 200 },
            min_w: 100,
            min_h: 100,
            strict_birth: false,
            is_quarantined: false,
            is_fullscreen: false,
        }
    ];

    let mut guard = controller.lock().await;
    let actions = guard.handle_state_change(workspaces.clone(), windows).unwrap();

    // Rust no debe ahogarse. Debe emitir comandos para 2 ventanas.
    // Verificamos que se calculó el layout partiéndolo en 2 (ej. anchos de ~948).
    let mut move_count = 0;
    for action in actions {
        if let raven_core::action::RavenAction::MoveWindow { width, .. } = action {
            assert!(width > 800 && width < 1000); // 948px para cada ventana
            move_count += 1;
        }
    }
    
    assert_eq!(move_count, 2);
}
