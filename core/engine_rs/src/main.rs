use std::error::Error;
use tracing::info;
use zbus::ConnectionBuilder;

use raven_engine::application::controller::RavenController;
use raven_engine::application::engine::TilingEngine;
use raven_core::config::RavenConfig;

use raven_engine::infrastructure::dbus::RavenDBusService;

/// Punto de entrada principal del demonio (daemon) Raven Tiling Emulator.
///
/// Inicializa las capas de configuración, dominio e infraestructura, y registra
/// el servicio en el bus de sesión de D-Bus para comenzar la orquestación.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Inicializar tracing: nivel configurable via RUST_LOG (e.g. RUST_LOG=debug)
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .compact()
        .init();

    info!("🐦 Iniciando Raven Tiling Emulator (Motor Nativo Rust v2.9)...");

    let app_config = RavenConfig::load();
    let engine = TilingEngine::new(app_config);
    let controller = RavenController::new(engine);

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let actor = raven_engine::application::actor::RavenControllerActor::new(controller, rx);
    
    // Iniciar el actor en un hilo en background
    tokio::spawn(actor.run());

    let dbus_service = RavenDBusService { tx };

    info!("[DBUS] Registrando servicio org.kde.raven.Daemon...");

    let _connection = ConnectionBuilder::session()?
        .name("org.kde.raven.Daemon")?
        .serve_at("/Events", dbus_service)?
        .build()
        .await?;

    info!("\u{2705} Raven está operando con éxito. Topología registrada.");

    std::future::pending::<()>().await;

    Ok(())
}
