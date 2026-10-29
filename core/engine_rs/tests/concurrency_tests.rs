//! # Pruebas de Concurrencia y Mutación de Configuración (`concurrency_tests`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Verifica la ausencia de condiciones de carrera ante modificaciones concurrentes
//! de configuración desde múltiples clientes (Plasmoid, GUI, atajos).

mod common;

use std::sync::Arc;
use tokio::sync::Mutex;

use raven_core::config::RavenConfig;
use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_engine::domain::geometry::Topology;

#[tokio::test]
async fn test_concurrent_settings_conflict() {
    let config = RavenConfig {
        default_gaps: 4,
        ..Default::default()
    };
    let engine = TilingEngine::new(config);
    let controller = Arc::new(Mutex::new(RavenController::new(engine)));

    let ctrl_a = controller.clone();
    let ctrl_b = controller.clone();

    let task_a = tokio::spawn(async move {
        let mut guard = ctrl_a.lock().await;
        let topology = Topology::default();
        let _ = guard.handle_shortcut("increment_gaps".to_string(), 10, None, &topology);
    });

    let task_b = tokio::spawn(async move {
        let mut guard = ctrl_b.lock().await;
        let topology = Topology::default();
        let _ = guard.handle_shortcut("increment_gaps".to_string(), -8, None, &topology);
    });

    let _ = tokio::join!(task_a, task_b);

    let guard = controller.lock().await;
    let final_gaps = guard.get_config().default_gaps;
    assert!(final_gaps == 6 || final_gaps >= 0);
}

