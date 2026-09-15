//! # Utilidad de Inspección Wayland (Probe)
//!
//! Herramienta de diagnóstico por CLI para verificar la ingestión de eventos nativos 
//! directamente desde el socket de Wayland (`$WAYLAND_DISPLAY`).
//! 
//! Inicializa el adaptador `WaylandBackend` de forma aislada, sin levantar el motor completo,
//! e imprime en la salida estándar los eventos traducidos (`CompositorEvent`) durante 5 segundos.
//! Es fundamental para depurar la topología de monitores y el comportamiento del compositor anfitrión.

use std::time::Duration;
use tokio::sync::mpsc;
use tracing::Level;
use tracing_subscriber::FmtSubscriber;

use raven_backend_wayland::WaylandBackend;
use raven_core::backend::{CompositorBackend, CompositorEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!("============================================================");
    println!("  🐦 RAVEN WAYLAND PROTOCOL PROBE & INSPECTOR (v4.0.0)");
    println!("============================================================");

    let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "NO DEFINIDO".to_string());
    println!("[INFO] Variable WAYLAND_DISPLAY detectada: {}", wayland_display);

    let backend = WaylandBackend::new();
    let (event_tx, mut event_rx) = mpsc::channel(100);

    println!("[INFO] Iniciando WaylandBackend::start_listener()...");
    if let Err(e) = backend.start_listener(event_tx).await {
        eprintln!("[ERROR] Fallo al iniciar el listener de Wayland: {}", e);
        return Ok(());
    }

    println!("[OK] Listener activo. Esperando y analizando eventos durante 5 segundos...");

    let timeout = tokio::time::sleep(Duration::from_secs(5));
    tokio::pin!(timeout);

    let mut event_count = 0;

    loop {
        tokio::select! {
            Some(event) = event_rx.recv() => {
                event_count += 1;
                match event {
                    CompositorEvent::TopologyChanged(topo) => {
                        println!("🖥️ [EVENTO #{} - TopologyChanged]", event_count);
                        println!("   Outputs detectados: {:?}", topo.outputs);
                        for node in &topo.output_nodes {
                            println!("   -> Monitor: {} | Rect: {:?}", node.name, node.rect);
                        }
                    }
                    CompositorEvent::WindowDiscovered(win) => {
                        println!("🪟 [EVENTO #{} - WindowDiscovered]", event_count);
                        println!("   ID: {}", win.window_id);
                        println!("   Clase / AppId: {}", win.resource_class);
                        println!("   Título: {}", win.caption);
                        println!("   Workspace/Monitor: {}", win.workspace_id);
                        println!("   Fullscreen: {} | Minimized: {}", win.is_fullscreen, win.is_minimized);
                    }
                    CompositorEvent::WindowFocused(Some(id)) => {
                        println!("🎯 [EVENTO #{} - WindowFocused]", event_count);
                        println!("   Foco activo en ventana ID: {}", id);
                    }
                    CompositorEvent::WindowClosed(id) => {
                        println!("❌ [EVENTO #{} - WindowClosed]", event_count);
                        println!("   Ventana destruida ID: {}", id);
                    }
                    other => {
                        println!("📦 [EVENTO #{}] {:?}", event_count, other);
                    }
                }
                println!("------------------------------------------------------------");
            }
            _ = &mut timeout => {
                println!("\n[INFO] Prueba de sondeo completada. Total de eventos recibidos: {}", event_count);
                break;
            }
        }
    }

    let (initial_workspaces, _) = backend.query_initial_state().await?;
    println!("\n[INFO] Estado de pantallas reportado por query_initial_state():");
    for (ws, rect) in initial_workspaces {
        println!("   -> Workspace {}: {:?}", ws, rect);
    }

    println!("============================================================");
    Ok(())
}
