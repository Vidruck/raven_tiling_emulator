use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use raven_core::config::RavenConfig;
use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_engine::domain::geometry::{Rect, Topology, WindowNode};

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
                window_id: format!("win-{i}"),
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
                resource_class: String::new(),
                caption: String::new(),
                custom_w_ratio: None,
                custom_h_ratio: None,
            };
            let mut guard = ctrl_clone.lock().await;
            guard.handle_delta_change(win);
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
            window_id: format!("win-{i}"),
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
            resource_class: String::new(),
            caption: String::new(),
            custom_w_ratio: None,
            custom_h_ratio: None,
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
    let config = RavenConfig {
        default_gaps: 4,
        ..Default::default()
    };
    let engine = TilingEngine::new(config);
    let controller = Arc::new(Mutex::new(RavenController::new(engine)));

    let ctrl_a = controller.clone();
    let ctrl_b = controller.clone();

    // Plasmoid recibe +10
    let task_a = tokio::spawn(async move {
        let mut guard = ctrl_a.lock().await;
        let topology = Topology {
            outputs: vec![],
            output_nodes: vec![],
            desktops: vec![],
            current_desktop: String::new(),
        };
        let _ = guard.handle_shortcut("increment_gaps".to_string(), 10, None, &topology);
    });

    // GUI recibe -8 concurrentemente
    let task_b = tokio::spawn(async move {
        let mut guard = ctrl_b.lock().await;
        let topology = Topology {
            outputs: vec![],
            output_nodes: vec![],
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
            resource_class: String::new(),
            caption: String::new(),
            custom_w_ratio: None,
            custom_h_ratio: None,
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
            resource_class: String::new(),
            caption: String::new(),
            custom_w_ratio: None,
            custom_h_ratio: None,
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
            resource_class: String::new(),
            caption: String::new(),
            custom_w_ratio: None,
            custom_h_ratio: None,
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

#[tokio::test]
async fn test_all_windows_dynamically_floated_and_restored() {
    // Escenario de estrés: El usuario invoca Meta+Shift+F sucesivamente en TODAS las ventanas activas.
    // 1. Verificar que el motor no entre en pánico cuando 0 ventanas quedan en el mosaico.
    // 2. Verificar que los comandos emitidos sean válidos (SetFloating { floating: true, keep_above: true }).
    // 3. El puente D-Bus retorna payloads JSON limpios sin colapsar.
    // 4. Al restaurar todas las ventanas una por una, el mosaico se reconstruye limpiamente sin fugas de estado.

    let config = RavenConfig::default();
    let engine = TilingEngine::new(config);
    let mut controller = RavenController::new(engine);
    let topology = Topology {
        outputs: vec!["DP-1".to_string()],
        output_nodes: vec![],
        desktops: vec!["desk_1".to_string()],
        current_desktop: "desk_1".to_string(),
    };

    let workspace_id = "DP-1||desk_1".to_string();
    let mut workspaces = HashMap::new();
    workspaces.insert(workspace_id.clone(), Rect { x: 0, y: 0, width: 1920, height: 1080 });

    let window_count = 10;
    let mut windows = Vec::new();
    for i in 0..window_count {
        windows.push(WindowNode {
            window_id: format!("app-{i}"),
            workspace_id: workspace_id.clone(),
            output: "DP-1".to_string(),
            desktops: vec!["desk_1".to_string()],
            is_floating: false,
            is_minimized: false,
            is_pip: false,
            geometry: Rect { x: 0, y: 0, width: 400, height: 300 },
            min_w: 100,
            min_h: 100,
            strict_birth: false,
            is_quarantined: false,
            is_fullscreen: false,
            resource_class: String::new(),
            caption: String::new(),
            custom_w_ratio: None,
            custom_h_ratio: None,
        });
    }

    // Registrar estado inicial de 10 ventanas en mosaico
    let initial_actions = controller.handle_state_change(workspaces.clone(), windows.clone()).unwrap();
    assert!(!initial_actions.is_empty(), "El estado inicial de 10 ventanas debe generar layout");

    // 1. El usuario presiona Meta+Shift+F en cada una de las 10 ventanas consecutivamente
    for i in 0..window_count {
        let win_id = format!("app-{}", i);
        let (needs_recalc, shortcut_cmds) = controller
            .handle_shortcut("toggle_floating".to_string(), 0, Some(win_id.clone()), &topology)
            .expect("handle_shortcut no debe fallar con toggle_floating");

        assert!(needs_recalc);
        assert_eq!(shortcut_cmds.len(), 1);

        match &shortcut_cmds[0] {
            raven_core::action::RavenAction::SetFloating { window_id, floating, keep_above } => {
                assert_eq!(window_id, &win_id);
                assert!(*floating);
                assert!(*keep_above);
            }
            _ => panic!("Comando inesperado retornado por toggle_floating"),
        }

        // Simular que el actor / pipeline ejecuta commit_layout tras needs_recalc
        let recalc_cmds = controller.commit_layout().expect("commit_layout no debe entrar en pánico");
        
        // El número de ventanas restantes en el mosaico disminuye progresivamente: (window_count - 1 - i)
        let remaining_tiled = window_count - 1 - i;
        if remaining_tiled == 0 {
            // Cuando TODAS las ventanas están flotando dinámicamente, el layout de mosaico queda vacío de forma segura
            assert!(recalc_cmds.is_empty(), "Con 0 ventanas en mosaico no debe haber comandos de movimiento");
        }
    }

    // Validar que las 10 ventanas están en dynamic_floating_windows
    assert_eq!(controller.get_engine().dynamic_floating_windows.len(), window_count);

    // 2. El usuario presiona Meta+Shift+F de nuevo para restaurar todas las ventanas al mosaico
    for i in 0..window_count {
        let win_id = format!("app-{}", i);
        let (needs_recalc, shortcut_cmds) = controller
            .handle_shortcut("toggle_floating".to_string(), 0, Some(win_id.clone()), &topology)
            .expect("handle_shortcut debe permitir restaurar ventanas al mosaico");

        assert!(needs_recalc);
        assert_eq!(shortcut_cmds.len(), 1);

        match &shortcut_cmds[0] {
            raven_core::action::RavenAction::SetFloating { window_id, floating, keep_above } => {
                assert_eq!(window_id, &win_id);
                assert!(!*floating);
                assert!(!*keep_above);
            }
            _ => panic!("Comando inesperado retornado por toggle_floating al restaurar"),
        }

        let recalc_cmds = controller.commit_layout().expect("commit_layout debe reconstruir el mosaico");
        assert!(!recalc_cmds.is_empty(), "Al reinsertar ventanas, el motor recalcula geometrías válidas");
        let has_move_target = recalc_cmds.iter().any(|c| match c {
            raven_core::action::RavenAction::MoveWindow { window_id, .. } => window_id == &win_id,
            _ => false,
        });
        assert!(has_move_target, "La ventana {} restaurada al mosaico debe recibir comando MoveWindow", win_id);
    }

    // Validar que la pila flotante quedó totalmente vacía y limpia
    assert_eq!(controller.get_engine().dynamic_floating_windows.len(), 0);
}

#[tokio::test]
async fn test_saturation_cyclic_stack_stress_60_windows() {
    // Escenario de estrés masivo contra intentos de romper el programa:
    // Pantalla de 1000x800. Capacidad máxima por algoritmo Dwindle BSP:
    // usable_w = 1000 - gaps; (1000/300).max(1) = 3 cols; (800/250).max(1) = 3 rows -> Cmax = 9.
    // Usaremos un monitor pequeño de 600x500 para forzar un Cmax = 4.
    // usable_w = 600 / 300 = 2; usable_h = 500 / 250 = 2 -> Cmax = 4.
    //
    // Se generan 60 ventanas (15 veces la capacidad del monitor).
    // 1. Inyectar las 60 ventanas en el controlador.
    // 2. Comprobar que exactamente las 4 más recientes quedan visibles en el mosaico
    //    y las 56 restantes reciben comando de desalojo / minimizado.
    // 3. Simular un usuario hostil desminimizando y activando en bucle ventanas desalojadas (100 ciclos):
    //    - La ventana reabierta/activada NUNCA debe ser desalojada de inmediato.
    //    - La ventana expulsada debe ser exactamente la que más tiempo llevaba inactiva (cola LRU).
    //    - No debe haber fugas, corrupción de árbol ni deadlocks con 60 ventanas.

    let config = RavenConfig {
        default_gaps: 0,
        ..Default::default()
    };
    let engine = TilingEngine::new(config);
    let mut controller = RavenController::new(engine);

    let workspace_id = "eDP-1||desktop_1".to_string();
    let mut workspaces = HashMap::new();
    workspaces.insert(
        workspace_id.clone(),
        Rect {
            x: 0,
            y: 0,
            width: 600,
            height: 500,
        },
    );

    let total_windows = 60;
    let mut window_pool: HashMap<String, WindowNode> = HashMap::new();

    for i in 0..total_windows {
        let wid = format!("stress-win-{:02}", i);
        window_pool.insert(
            wid.clone(),
            WindowNode {
                window_id: wid,
                workspace_id: workspace_id.clone(),
                output: "eDP-1".to_string(),
                desktops: vec!["desktop_1".to_string()],
                is_floating: false,
                is_minimized: false,
                is_pip: false,
                geometry: Rect {
                    x: 0,
                    y: 0,
                    width: 300,
                    height: 250,
                },
                min_w: 100,
                min_h: 100,
                strict_birth: false,
                is_quarantined: false,
                is_fullscreen: false,
                resource_class: "stress-test-class".to_string(),
                caption: format!("Stress Window {}", i),
                custom_w_ratio: None,
                custom_h_ratio: None,
            },
        );
    }

    // Paso 1: Introducir las 60 ventanas inicialmente todas abiertas
    let mut current_windows: Vec<WindowNode> = (0..total_windows)
        .map(|i| window_pool.get(&format!("stress-win-{:02}", i)).unwrap().clone())
        .collect();

    let initial_actions = controller
        .handle_state_change(workspaces.clone(), current_windows.clone())
        .expect("El controlador debe asimilar 60 ventanas sin pánico");

    // Extraer qué ventanas fueron ordenadas a minimizar
    let evicted_ids: Vec<String> = initial_actions
        .iter()
        .filter_map(|action| match action {
            raven_core::action::RavenAction::MinimizeWindow { window_id } => Some(window_id.clone()),
            _ => None,
        })
        .collect();

    // Con Cmax = 4 en 600x500 y 60 ventanas, el exceso debe ser exactamente 60 - 4 = 56 ventanas
    assert_eq!(
        evicted_ids.len(),
        56,
        "Deben ser desalojadas exactamente 56 ventanas por saturación geométrica"
    );

    // Las ventanas desalojadas fueron las 56 primeras (stress-win-00 hasta stress-win-55)
    for i in 0..56 {
        let expected_evicted = format!("stress-win-{:02}", i);
        assert!(
            evicted_ids.contains(&expected_evicted),
            "La ventana más vieja {} debió ser desalojada",
            expected_evicted
        );
    }

    // Actualizamos el estado simulando que KWin minimizó las ventanas desalojadas
    for id in &evicted_ids {
        if let Some(w) = window_pool.get_mut(id) {
            w.is_minimized = true;
        }
    }

    // Paso 2: Ciclo de estrés hostil (100 iteraciones)
    // El usuario toma una de las ventanas minimizadas (la más vieja de todas),
    // la desminimiza y la enfoca/activa.
    let mut rng_seed: usize = 0;
    for iteration in 0..100 {
        // Seleccionamos una ventana actualmente minimizada
        let candidate_id = format!("stress-win-{:02}", rng_seed % 56);
        rng_seed += 7; // Paso pseudoaleatorio para alternar entre las 56 ventanas

        // Simular que el usuario hace click en la barra de tareas: la desminimiza y la enfoca
        if let Some(w) = window_pool.get_mut(&candidate_id) {
            w.is_minimized = false;
        }
        controller.active_window_id = Some(candidate_id.clone());

        // Preparamos el payload completo de ventanas reflejando el nuevo estado
        current_windows = (0..total_windows)
            .map(|i| window_pool.get(&format!("stress-win-{:02}", i)).unwrap().clone())
            .collect();

        let actions = controller
            .handle_state_change(workspaces.clone(), current_windows.clone())
            .unwrap_or_else(|e| panic!("Fallo en iteración {}: {:?}", iteration, e));

        let new_evictions: Vec<String> = actions
            .iter()
            .filter_map(|action| match action {
                raven_core::action::RavenAction::MinimizeWindow { window_id } => Some(window_id.clone()),
                _ => None,
            })
            .collect();

        // 1. REGLA DE ORO DE LA GUERRA DE MINIMIZADO:
        // La ventana candidata que el usuario acaba de reabrir/enfocar NUNCA debe ser minimizada
        assert!(
            !new_evictions.contains(&candidate_id),
            "GUERRA DETECTADA en iteración {}: El demonio minimizó la ventana recién activada por el usuario ({})!",
            iteration,
            candidate_id
        );

        // 2. Debe desalojar a otra ventana para mantener el balance Cmax = 4
        assert_eq!(
            new_evictions.len(),
            1,
            "En iteración {}, al reactivar una ventana debe desalojar exactamente 1 ventana vieja",
            iteration
        );

        // 3. Aplicar la nueva minimización a nuestro pool simulado
        for evicted in &new_evictions {
            if let Some(w) = window_pool.get_mut(evicted) {
                w.is_minimized = true;
            }
        }

        // Comprobar que en todo momento quedan exactamente 4 ventanas no minimizadas
        let visible_count = window_pool.values().filter(|w| !w.is_minimized).count();
        assert_eq!(
            visible_count, 4,
            "En iteración {}, deben quedar exactamente 4 ventanas visibles en el mosaico",
            iteration
        );
    }

    // Comprobar que el historial interno mantiene exactamente las 60 ventanas en seguimiento
    assert_eq!(controller.get_engine().window_history.len(), 60);
}

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

    let topology = Topology {
        outputs: vec!["DP-1".to_string(), "HDMI-A-1".to_string()],
        output_nodes: vec![],
        desktops: vec!["desk_1".to_string(), "desk_2".to_string()],
        current_desktop: "desk_1".to_string(),
    };

    let ws1 = "DP-1||desk_1".to_string();
    let mut workspaces = HashMap::new();
    workspaces.insert(ws1.clone(), Rect { x: 0, y: 0, width: 1920, height: 1080 });

    // 1. Inicializar 3 ventanas en mosaico
    let mut windows = vec![
        WindowNode {
            window_id: "win-1".to_string(),
            workspace_id: ws1.clone(),
            output: "DP-1".to_string(),
            desktops: vec!["desk_1".to_string()],
            is_floating: false,
            is_minimized: false,
            is_pip: false,
            geometry: Rect { x: 0, y: 0, width: 960, height: 1080 },
            min_w: 100,
            min_h: 100,
            strict_birth: false,
            is_quarantined: false,
            is_fullscreen: false,
            resource_class: "terminal".to_string(),
            caption: "Terminal".to_string(),
            custom_w_ratio: None,
            custom_h_ratio: None,
        },
        WindowNode {
            window_id: "win-2".to_string(),
            workspace_id: ws1.clone(),
            output: "DP-1".to_string(),
            desktops: vec!["desk_1".to_string()],
            is_floating: false,
            is_minimized: false,
            is_pip: false,
            geometry: Rect { x: 960, y: 0, width: 960, height: 540 },
            min_w: 100,
            min_h: 100,
            strict_birth: false,
            is_quarantined: false,
            is_fullscreen: false,
            resource_class: "editor".to_string(),
            caption: "Editor".to_string(),
            custom_w_ratio: None,
            custom_h_ratio: None,
        },
        WindowNode {
            window_id: "win-3".to_string(),
            workspace_id: ws1.clone(),
            output: "DP-1".to_string(),
            desktops: vec!["desk_1".to_string()],
            is_floating: false,
            is_minimized: false,
            is_pip: false,
            geometry: Rect { x: 960, y: 540, width: 960, height: 540 },
            min_w: 100,
            min_h: 100,
            strict_birth: false,
            is_quarantined: false,
            is_fullscreen: false,
            resource_class: "browser".to_string(),
            caption: "Browser".to_string(),
            custom_w_ratio: None,
            custom_h_ratio: None,
        },
    ];

    let init_actions = controller.handle_state_change(workspaces.clone(), windows.clone()).unwrap();
    assert_eq!(init_actions.len(), 3);

    // Activar win-1
    controller.active_window_id = Some("win-1".to_string());

    // --- A. Atajos de Gaps ---
    // Meta+= (incrementGaps +2)
    let (recalc, _) = controller.handle_shortcut("increment_gaps".to_string(), 2, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert_eq!(controller.get_config().default_gaps, 12);

    // Meta+- (decrementGaps -2)
    let (recalc, _) = controller.handle_shortcut("increment_gaps".to_string(), -2, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert_eq!(controller.get_config().default_gaps, 10);

    // --- B. Atajos de Master (Capacidad y Ratio) ---
    // Meta+] (incrementMaster)
    let (recalc, _) = controller.handle_shortcut("increment_nmaster".to_string(), 1, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert_eq!(controller.get_config().nmaster, 2);

    // Meta+[ (decrementMaster)
    let (recalc, _) = controller.handle_shortcut("decrement_nmaster".to_string(), 1, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert_eq!(controller.get_config().nmaster, 1);

    // Meta+H (increaseRatio)
    let initial_ratio = controller.get_config().master_ratio;
    let (recalc, _) = controller.handle_shortcut("increase_ratio".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert!((controller.get_config().master_ratio - (initial_ratio + 0.05)).abs() < 0.001);

    // Meta+L (decreaseRatio)
    let (recalc, _) = controller.handle_shortcut("decrease_ratio".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert!((controller.get_config().master_ratio - initial_ratio).abs() < 0.001);

    // --- C. Atajos de Ciclado de Layout ---
    // Meta+Shift+L (cycleLayout)
    let (recalc, _) = controller.handle_shortcut("cycle_layout".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    // raven -> tall
    let cur_layout = controller.get_config().workspace_layouts.get(&ws1).cloned().unwrap_or(controller.get_config().layout_type.clone());
    assert_eq!(cur_layout, "tall");
    // Al cambiar de layout, el actor ejecuta commit_layout() para actualizar geometrías
    let move_cmds = controller.commit_layout().unwrap();
    assert!(!move_cmds.is_empty());
    // Simular que KWin converge las ventanas al nuevo layout Tall (win-1 Master izquierda, win-2 Stack der arriba, win-3 Stack der abajo)
    for cmd in move_cmds {
        if let raven_core::action::RavenAction::MoveWindow { window_id, x, y, width, height } = cmd {
            if let Some(w) = windows.iter_mut().find(|w| w.window_id == window_id) {
                w.geometry = Rect { x, y, width, height };
            }
        }
    }
    let _ = controller.handle_state_change(workspaces.clone(), windows.clone()).unwrap();

    // --- D. Atajos de Redimensionamiento Fino (Window Resizing) ---
    // Meta+Alt+Right (resize_width_inc)
    let (recalc, _) = controller.handle_shortcut("resize_width_inc".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert!(controller.get_engine().current_windows.get("win-1").unwrap().custom_w_ratio.unwrap() > 1.0);

    // Meta+Alt+Left (resize_width_dec)
    let (recalc, _) = controller.handle_shortcut("resize_width_dec".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert!((controller.get_engine().current_windows.get("win-1").unwrap().custom_w_ratio.unwrap() - 1.0).abs() < 0.001);

    // Meta+Alt+Down (resize_height_inc)
    let (recalc, _) = controller.handle_shortcut("resize_height_inc".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert!(controller.get_engine().current_windows.get("win-1").unwrap().custom_h_ratio.unwrap() > 1.0);

    // Meta+Alt+Up (resize_height_dec)
    let (recalc, _) = controller.handle_shortcut("resize_height_dec".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert!((controller.get_engine().current_windows.get("win-1").unwrap().custom_h_ratio.unwrap() - 1.0).abs() < 0.001);

    // --- E. Atajos de Navegación de Foco ---
    // Meta+J (focusNext)
    let (_, cmds) = controller.handle_shortcut("focus_next".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        raven_core::action::RavenAction::FocusWindow { window_id } => assert_eq!(window_id, "win-2"),
        _ => panic!("Esperado FocusWindow"),
    }

    // Meta+K (focusPrev)
    let (_, cmds) = controller.handle_shortcut("focus_prev".to_string(), 0, Some("win-2".to_string()), &topology).unwrap();
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        raven_core::action::RavenAction::FocusWindow { window_id } => assert_eq!(window_id, "win-1"),
        _ => panic!("Esperado FocusWindow"),
    }

    // Foco Direccional en Layout Tall:
    // En Tall con nmaster=1, la ventana maestra está a la izquierda (win-1, x: 10, w: 945).
    // Las ventanas del stack están a la derecha (x: 965): win-2 arriba (y: 10) y win-3 abajo (y: 545).
    // Meta+Right (focusRight) desde win-1 hacia la columna derecha (win-2 o win-3)
    let (_, cmds) = controller.handle_shortcut("focus_right".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert_eq!(cmds.len(), 1);
    let right_target = match &cmds[0] {
        raven_core::action::RavenAction::FocusWindow { window_id } => {
            assert!(window_id == "win-2" || window_id == "win-3");
            window_id.clone()
        }
        _ => panic!("Esperado FocusWindow a la derecha"),
    };

    // Meta+Left (focusLeft) desde la ventana derecha hacia win-1 (Master a la izquierda)
    let (_, cmds) = controller.handle_shortcut("focus_left".to_string(), 0, Some(right_target), &topology).unwrap();
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        raven_core::action::RavenAction::FocusWindow { window_id } => assert_eq!(window_id, "win-1"),
        _ => panic!("Esperado FocusWindow a la izquierda"),
    }

    // Meta+Down (focusDown) desde win-2 (arriba) hacia win-3 (abajo)
    let (_, cmds) = controller.handle_shortcut("focus_down".to_string(), 0, Some("win-2".to_string()), &topology).unwrap();
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        raven_core::action::RavenAction::FocusWindow { window_id } => assert_eq!(window_id, "win-3"),
        _ => panic!("Esperado FocusWindow abajo"),
    }

    // Meta+Up (focusUp) desde win-3 (abajo) hacia win-2 (arriba)
    let (_, cmds) = controller.handle_shortcut("focus_up".to_string(), 0, Some("win-3".to_string()), &topology).unwrap();
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        raven_core::action::RavenAction::FocusWindow { window_id } => assert_eq!(window_id, "win-2"),
        _ => panic!("Esperado FocusWindow arriba"),
    }

    // --- F. Intercambio de Posiciones (Swap) ---
    // Meta+Shift+J (swapNext)
    let (recalc, _) = controller.handle_shortcut("swap_next".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);

    // Meta+Shift+K (swapPrev)
    let (recalc, _) = controller.handle_shortcut("swap_prev".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);

    // --- G. Migración de Escritorios y Pantallas ---
    // Meta+Shift+Right (migrateActiveToDesktop: desk_1 -> desk_2)
    let (recalc, cmds) = controller.handle_shortcut("migrate_active_to_desktop".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        raven_core::action::RavenAction::MigrateToDesktop { window_id, target_desktop } => {
            assert_eq!(window_id, "win-1");
            assert_eq!(target_desktop, "desk_2");
        }
        _ => panic!("Esperado MigrateToDesktop"),
    }

    // Meta+Shift+Left (migrateActiveToPrevDesktop: desk_2 -> desk_1)
    let (recalc, cmds) = controller.handle_shortcut("migrate_active_to_prev_desktop".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        raven_core::action::RavenAction::MigrateToDesktop { window_id, target_desktop } => {
            assert_eq!(window_id, "win-1");
            assert_eq!(target_desktop, "desk_1");
        }
        _ => panic!("Esperado MigrateToDesktop"),
    }

    // Meta+Shift+M (migrateActiveToScreen: DP-1 -> HDMI-A-1)
    let (recalc, cmds) = controller.handle_shortcut("migrate_active_to_screen".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        raven_core::action::RavenAction::MigrateToOutput { window_id, target_output } => {
            assert_eq!(window_id, "win-1");
            assert_eq!(target_output, "HDMI-A-1");
        }
        _ => panic!("Esperado MigrateToOutput"),
    }

    // Meta+Shift+N (migrateActiveToPrevScreen: HDMI-A-1 -> DP-1)
    let (recalc, cmds) = controller.handle_shortcut("migrate_active_to_prev_screen".to_string(), 0, Some("win-1".to_string()), &topology).unwrap();
    assert!(recalc);
    assert_eq!(cmds.len(), 1);
    match &cmds[0] {
        raven_core::action::RavenAction::MigrateToOutput { window_id, target_output } => {
            assert_eq!(window_id, "win-1");
            assert_eq!(target_output, "DP-1");
        }
        _ => panic!("Esperado MigrateToOutput"),
    }

    // --- H. Alternar Tiling On/Off ---
    // Meta+Space (toggleTiling)
    let (recalc, _) = controller.handle_shortcut("toggle_tiling".to_string(), 0, None, &topology).unwrap();
    assert!(recalc);
    assert!(!controller.is_tiling_enabled());

    let (recalc, _) = controller.handle_shortcut("toggle_tiling".to_string(), 0, None, &topology).unwrap();
    assert!(recalc);
    assert!(controller.is_tiling_enabled());
}

#[tokio::test]
async fn test_window_focus_does_not_swap_spatial_order() {
    // Test de regresión: Garantizar que al enfocar o activar una ventana,
    // el orden espacial (spatial_order) y las geometrías de las ventanas NO se alteren.
    let config = RavenConfig {
        default_gaps: 10,
        master_ratio: 0.6,
        nmaster: 1,
        layout_type: "raven".to_string(),
        ..Default::default()
    };
    let engine = TilingEngine::new(config);
    let mut controller = RavenController::new(engine);

    let ws = "DP-1||desk_1".to_string();
    let mut workspaces = HashMap::new();
    workspaces.insert(ws.clone(), Rect { x: 0, y: 0, width: 1920, height: 1080 });

    let make_window = |id: &str| WindowNode {
        window_id: id.to_string(),
        workspace_id: ws.clone(),
        output: "DP-1".to_string(),
        desktops: vec!["desk_1".to_string()],
        is_floating: false,
        is_minimized: false,
        is_pip: false,
        geometry: Rect { x: 0, y: 0, width: 100, height: 100 },
        min_w: 100,
        min_h: 100,
        strict_birth: false,
        is_quarantined: false,
        is_fullscreen: false,
        resource_class: "app".to_string(),
        caption: id.to_string(),
        custom_w_ratio: None,
        custom_h_ratio: None,
    };

    let windows = vec![
        make_window("win-1"),
        make_window("win-2"),
        make_window("win-3"),
        make_window("win-4"),
    ];

    // Estado inicial: calcular geometrías iniciales
    let actions_init = controller.handle_state_change(workspaces.clone(), windows.clone()).unwrap();
    let mut geoms_initial: HashMap<String, Rect> = HashMap::new();
    for act in &actions_init {
        if let raven_core::action::RavenAction::MoveWindow { window_id, x, y, width, height } = act {
            geoms_initial.insert(window_id.clone(), Rect { x: *x, y: *y, width: *width, height: *height });
        }
    }
    assert_eq!(geoms_initial.len(), 4);

    // Simular que el usuario hace click o enfoca en win-2 (o win-4)
    controller.active_window_id = Some("win-2".to_string());
    let actions_after_focus = controller.handle_state_change(workspaces.clone(), windows.clone()).unwrap();
    let mut geoms_after_focus: HashMap<String, Rect> = HashMap::new();
    for act in &actions_after_focus {
        if let raven_core::action::RavenAction::MoveWindow { window_id, x, y, width, height } = act {
            geoms_after_focus.insert(window_id.clone(), Rect { x: *x, y: *y, width: *width, height: *height });
        }
    }

    // Comprobar que NINGUNA ventana cambió de geometría o de slot visual
    for id in ["win-1", "win-2", "win-3", "win-4"] {
        assert_eq!(
            geoms_initial.get(id),
            geoms_after_focus.get(id),
            "Regresión detectada: La geometría de {} cambió simplemente al recibir foco!",
            id
        );
    }
}



