//! # Pestaña de Gestión del Servicio Systemd (`service_tab.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Proporciona controles interactivos para iniciar, detener, reiniciar y consultar
//! el estado en vivo del servicio `raven.service` administrado por systemd en la sesión de usuario.

use eframe::egui;
use raven_core::config::RavenConfig;
use crate::services::ServiceManager;

/// Renderiza la vista de gestión del servicio de usuario `raven.service`.
pub fn show(
    config: &mut RavenConfig,
    ui: &mut egui::Ui,
    accent: egui::Color32,
    service_mgr: &mut ServiceManager,
    status_msg: &mut String,
) {
    ui.heading(egui::RichText::new("⚙️ Gestión del Servicio Nativo (raven.service)").strong().size(18.0).color(accent));
    ui.label(egui::RichText::new("Control del demonio ejecutable de Rust administrado por systemd --user.").weak());
    ui.add_space(12.0);

    let is_active = service_mgr.is_service_active();

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.heading(egui::RichText::new("Estado del Servicio").strong().size(14.0));
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Estado Actual:").strong());
                if is_active {
                    ui.colored_label(egui::Color32::from_rgb(76, 201, 240), "● Activo (Ejecutándose)");
                } else {
                    ui.colored_label(egui::Color32::from_rgb(247, 37, 133), "● Inactivo (Apagado)");
                }
            });
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.add_sized([140.0, 32.0], egui::Button::new("▶ Encender")).clicked() {
                    *status_msg = service_mgr.start_service();
                }
                ui.add_space(10.0);
                if ui.add_sized([140.0, 32.0], egui::Button::new("⏹ Apagar")).clicked() {
                    *status_msg = service_mgr.stop_service();
                }
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);

            ui.checkbox(
                &mut config.tiling_enabled_on_startup,
                "Arrancar Raven automáticamente al iniciar la sesión de usuario (Systemd User Unit)",
            );
        });
    });

    ui.add_space(12.0);

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.heading(egui::RichText::new("📜 Mini Depurador").strong().size(14.0));
            ui.label(egui::RichText::new("Permite visualizar los ultimos 5 logs del servicio.").weak().size(11.0));
            ui.add_space(6.0);

            let logs = service_mgr.get_recent_logs();

            egui::Frame::none()
                .fill(egui::Color32::from_black_alpha(40))
                .rounding(8.0)
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    if logs.is_empty() {
                        ui.label(egui::RichText::new("Sin registros recientes en journalctl.").weak().size(11.0));
                    } else {
                        for line in logs {
                            ui.label(egui::RichText::new(line).monospace().size(10.5));
                        }
                    }
                });
        });
    });
}
