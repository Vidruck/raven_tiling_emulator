// ============================================================================
// RAVEN TILING EMULATOR — CENTRO DE CONTROL GRÁFICO (raven_gui)
// ============================================================================
// Interfaz nativa desarrollada en Rust usando `egui` y `eframe`. Proporciona un
// panel de configuración moderno con estética Material You / GNOME Adwaita.
// Arquitectura modular multiarchivo limpia y mantenible.
// ============================================================================

use eframe::egui;

mod app;
mod components;
mod kde_theme;
mod models;
mod services;
mod tabs;

use app::RavenGuiApp;

/// Punto de entrada principal del Centro de Control de Raven.
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([940.0, 680.0])
            .with_min_inner_size([880.0, 620.0])
            .with_title("Raven Tiling Emulator — Control Center"),
        ..Default::default()
    };

    eframe::run_native(
        "Raven Config",
        options,
        Box::new(|cc| Box::new(RavenGuiApp::new(cc))),
    )
}
