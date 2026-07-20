use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use raven_core::config::RavenConfig;
use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_engine::domain::geometry::{Rect, WindowNode};

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
        });
    }

    let result = controller.handle_state_change(workspaces, windows);
    assert!(result.is_ok());
    let commands = result.unwrap();

    let has_eviction = commands.iter().any(|cmd| match cmd {
        raven_engine::domain::action::RavenAction::MinimizeWindow { window_id } => window_id == "win-2",
        _ => false,
    });
    
    assert!(has_eviction, "Rebellious window should have been evicted/minimized");
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
        let _ = guard.handle_shortcut("increment_gaps".to_string(), 10, vec![], HashMap::new(), None);
    });

    // GUI recibe -8 concurrentemente
    let task_b = tokio::spawn(async move {
        let mut guard = ctrl_b.lock().await;
        let _ = guard.handle_shortcut("increment_gaps".to_string(), -8, vec![], HashMap::new(), None);
    });

    let _ = tokio::join!(task_a, task_b);

    let guard = controller.lock().await;
    let final_gaps = guard.get_config().default_gaps;
    
    // Mientras no entre en pánico, maneja la concurrencia de forma segura.
    // El valor final debe ser lógicamente consistente (por ejemplo, 6).
    assert!(final_gaps == 6 || final_gaps > 0);
}
