//! # Utilidades y Constructores Compartidos de Pruebas (`common`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Proporciona constructores fluidos y escenarios base para las pruebas modulares
//! de integración del motor `raven_engine`.

#![allow(dead_code)]

use std::collections::HashMap;
use raven_core::config::RavenConfig;
use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_engine::domain::geometry::{Rect, Topology, WindowNode};

/// Crea un `RavenController` inicializado con la configuración por defecto.
pub fn default_controller() -> RavenController {
    let config = RavenConfig::default();
    let engine = TilingEngine::new(config);
    RavenController::new(engine)
}

/// Crea un mapa de áreas de trabajo con un único monitor/escritorio de dimensiones dadas.
pub fn single_workspace(ws_id: &str, width: i32, height: i32) -> HashMap<String, Rect> {
    let mut workspaces = HashMap::new();
    workspaces.insert(
        ws_id.to_string(),
        Rect {
            x: 0,
            y: 0,
            width,
            height,
        },
    );
    workspaces
}

/// Crea un nodo de ventana (`WindowNode`) estándar de prueba.
pub fn make_test_window(id: &str, ws_id: &str, output: &str, desktop: &str) -> WindowNode {
    WindowNode {
        window_id: id.to_string(),
        workspace_id: ws_id.to_string(),
        output: output.to_string(),
        desktops: if desktop.is_empty() {
            vec![]
        } else {
            vec![desktop.to_string()]
        },
        is_floating: false,
        is_minimized: false,
        is_pip: false,
        geometry: Rect {
            x: 0,
            y: 0,
            width: 800,
            height: 600,
        },
        min_w: 100,
        min_h: 100,
        strict_birth: false,
        is_quarantined: false,
        is_fullscreen: false,
        is_maximized: false,
        resource_class: "app".to_string(),
        resource_name: String::new(),
        is_suspicious: false,
        caption: id.to_string(),
        custom_w_ratio: None,
        custom_h_ratio: None,
    }
}

/// Crea una topología dual de prueba (2 monitores, 2 escritorios virtuales).
pub fn dual_topology() -> Topology {
    Topology {
        outputs: vec!["DP-1".to_string(), "HDMI-A-1".to_string()],
        output_nodes: vec![],
        desktops: vec!["desk_1".to_string(), "desk_2".to_string()],
        current_desktop: "desk_1".to_string(),
    }
}

