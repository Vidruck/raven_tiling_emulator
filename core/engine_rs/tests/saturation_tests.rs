//! # Pruebas de Saturación y Pila Cíclica LRU (`saturation_tests`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Valida el comportamiento del motor bajo ráfagas masivas de ventanas (`10,000` deltas)
//! y el desalojo determinista por saturación geométrica (`60` ventanas sobre capacidad `Cmax = 4`).

mod common;

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use raven_core::config::RavenConfig;
use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_engine::domain::geometry::{Rect, WindowNode};

#[tokio::test]
async fn test_saturation_flood() {
    let controller = Arc::new(Mutex::new(common::default_controller()));

    let mut tasks = Vec::with_capacity(10_000);
    for i in 0..10_000 {
        let ctrl_clone = controller.clone();
        tasks.push(tokio::spawn(async move {
            let mut win = common::make_test_window(&format!("win-{i}"), "default||default", "default", "");
            win.min_w = 0;
            win.min_h = 0;
            let mut guard = ctrl_clone.lock().await;
            guard.handle_delta_change(win);
        }));
    }

    for task in tasks {
        let _ = task.await;
    }

    let mut guard = controller.lock().await;
    let _ = guard.commit_layout();
    assert!(guard.is_tiling_enabled());
}

#[tokio::test]
async fn test_saturation_cyclic_stack_stress_60_windows() {
    let config = RavenConfig {
        default_gaps: 0,
        ..Default::default()
    };
    let engine = TilingEngine::new(config);
    let mut controller = RavenController::new(engine);

    let workspace_id = "eDP-1||desktop_1";
    let workspaces = common::single_workspace(workspace_id, 600, 500);

    let total_windows = 60;
    let mut window_pool: HashMap<String, WindowNode> = HashMap::new();

    for i in 0..total_windows {
        let wid = format!("stress-win-{i:02}");
        let mut win = common::make_test_window(&wid, workspace_id, "eDP-1", "desktop_1");
        win.geometry = Rect {
            x: 0,
            y: 0,
            width: 300,
            height: 250,
        };
        win.resource_class = "stress-test-class".to_string();
        win.caption = format!("Stress Window {i}");
        window_pool.insert(wid, win);
    }

    // Paso 1: Introducir las 60 ventanas inicialmente todas abiertas
    let mut current_windows: Vec<WindowNode> = (0..total_windows)
        .map(|i| {
            window_pool
                .get(&format!("stress-win-{i:02}"))
                .unwrap()
                .clone()
        })
        .collect();

    let initial_actions = controller
        .handle_state_change(workspaces.clone(), current_windows.clone())
        .expect("El controlador debe asimilar 60 ventanas sin pánico");

    let evicted_ids: Vec<String> = initial_actions
        .iter()
        .filter_map(|action| match action {
            raven_core::action::RavenAction::MinimizeWindow { window_id } => {
                Some(window_id.clone())
            }
            _ => None,
        })
        .collect();

    // Con Cmax = 4 en 600x500 y 60 ventanas, el exceso debe ser exactamente 60 - 4 = 56 ventanas
    assert_eq!(
        evicted_ids.len(),
        56,
        "Deben ser desalojadas exactamente 56 ventanas por saturación geométrica"
    );

    for i in 0..56 {
        let expected_evicted = format!("stress-win-{i:02}");
        assert!(
            evicted_ids.contains(&expected_evicted),
            "La ventana más vieja {expected_evicted} debió ser desalojada"
        );
    }

    for id in &evicted_ids {
        if let Some(w) = window_pool.get_mut(id) {
            w.is_minimized = true;
        }
    }

    // Paso 2: Ciclo de estrés hostil (100 iteraciones)
    let mut rng_seed: usize = 0;
    for iteration in 0..100 {
        let candidate_id = format!("stress-win-{:02}", rng_seed % 56);
        rng_seed += 7;

        if let Some(w) = window_pool.get_mut(&candidate_id) {
            w.is_minimized = false;
        }
        controller.active_window_id = Some(candidate_id.clone());

        current_windows = (0..total_windows)
            .map(|i| {
                window_pool
                    .get(&format!("stress-win-{i:02}"))
                    .unwrap()
                    .clone()
            })
            .collect();

        let actions = controller
            .handle_state_change(workspaces.clone(), current_windows.clone())
            .unwrap_or_else(|e| panic!("Fallo en iteración {iteration}: {e:?}"));

        let new_evictions: Vec<String> = actions
            .iter()
            .filter_map(|action| match action {
                raven_core::action::RavenAction::MinimizeWindow { window_id } => {
                    Some(window_id.clone())
                }
                _ => None,
            })
            .collect();

        assert!(
            !new_evictions.contains(&candidate_id),
            "GUERRA DETECTADA en iteración {iteration}: El demonio minimizó la ventana recién activada por el usuario ({candidate_id})!"
        );

        assert_eq!(
            new_evictions.len(),
            1,
            "En iteración {iteration}, al reactivar una ventana debe desalojar exactamente 1 ventana vieja"
        );

        for evicted in &new_evictions {
            if let Some(w) = window_pool.get_mut(evicted) {
                w.is_minimized = true;
            }
        }

        let visible_count = window_pool.values().filter(|w| !w.is_minimized).count();
        assert_eq!(
            visible_count, 4,
            "En iteración {iteration}, deben quedar exactamente 4 ventanas visibles en el mosaico"
        );
    }

    assert_eq!(controller.get_engine().window_history.len(), 60);
}

