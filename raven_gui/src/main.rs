//! # Centro de Control Gráfico de Raven (`raven_gui`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Interfaz nativa desarrollada en Rust usando `egui` y `eframe`. Proporciona un
//! panel de configuración avanzado con estética glassmorphic / Material You adaptado a KDE Plasma 6.

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
