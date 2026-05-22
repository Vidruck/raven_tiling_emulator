use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;
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
    println!("🐦 Iniciando Raven Tiling Emulator (Motor Nativo Rust v2.7)...");

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

    println!("[DBUS] Registrando servicio org.kde.raven.Daemon...");

    let _connection = ConnectionBuilder::session()?
        .name("org.kde.raven.Daemon")?
        .serve_at("/Events", dbus_service)?
        .build()
        .await?;

    println!("✅ Raven está operando con éxito. Topología registrada.");

    std::future::pending::<()>().await;

    Ok(())
}
