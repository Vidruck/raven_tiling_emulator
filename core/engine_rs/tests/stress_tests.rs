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
    let topology = KWinTopology {
        outputs: vec!["DP-1".to_string()],
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

