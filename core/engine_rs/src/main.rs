use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;
use zbus::ConnectionBuilder;

use raven_core::application::controller::RavenController;
use raven_core::application::engine::TilingEngine;
use raven_core::infrastructure::config::RavenConfig;
use raven_core::infrastructure::dbus::{KWinTopology, RavenDBusService};

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

    info!("\u{1f426} Iniciando Raven Tiling Emulator (Motor Nativo Rust v2.8)...");

    let app_config = RavenConfig::load();
    let engine = TilingEngine::new(app_config);

    let controller = Arc::new(Mutex::new(RavenController::new(engine)));

    let tokio_handle = tokio::runtime::Handle::current();
    let dbus_service = RavenDBusService {
        controller,
        pending_commands: Arc::new(Mutex::new(Vec::new())),
        active_window_id: Arc::new(Mutex::new(None)),
        last_payload_json: Arc::new(Mutex::new(String::from("{}"))),
        current_topology: Arc::new(Mutex::new(KWinTopology::default())),
        tokio_handle,
    };

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
