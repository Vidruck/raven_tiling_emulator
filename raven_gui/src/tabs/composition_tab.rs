//! # Pestaña de Reglas de Ventana y Cuarentena CSD (`composition_tab.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Administra la creación interactiva de reglas de exclusión por clase WM (`WindowRule`),
//! anclaje permanente a modo Picture-in-Picture (PiP) y configuración de clases en cuarentena.

use eframe::egui;
use raven_core::config::RavenConfig;

/// Renderiza la vista de administración de reglas de ventana y cuarentenas CSD.
pub fn show(config: &mut RavenConfig, ui: &mut egui::Ui, accent: egui::Color32) {
    ui.heading(egui::RichText::new("🛡️ Reglas de Ventanas & Cuarentena").strong().size(18.0).color(accent));
    ui.label(egui::RichText::new("Gestiona Aplicaciones problematicas o que quieras definir como flotantes/Pip's.").weak());
    ui.add_space(12.0);

    ui.add_space(12.0);

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.heading(egui::RichText::new("🛡️ Reglas para Exclusión de Aplicaciones").strong().size(14.0));
            ui.label(egui::RichText::new("Aquí puedes definir reglas para que ciertas aplicaciones que quieras mantener como flotantes o en modo PiP.").weak().size(11.0));
            
            ui.label(egui::RichText::new("Clase WM: Identifica una aplicación.").weak().size(11.0));
            ui.label(egui::RichText::new("Acción: Acción a realizar (float or pip).").weak().size(11.0));
            ui.add_space(6.0);

            let mut to_delete = None;
            for (idx, rule) in config.window_rules.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label("Clase WM:");
                    ui.text_edit_singleline(&mut rule.class);
                    ui.label("Acción:");
                    ui.text_edit_singleline(&mut rule.action);
                    ui.checkbox(&mut rule.pip, "PiP");
                    if ui.button("🗑").clicked() {
                        to_delete = Some(idx);
                    }
                });
                ui.add_space(4.0);
            }

            if let Some(idx) = to_delete {
                config.window_rules.remove(idx);
            }

            ui.add_space(6.0);
            if ui.button("➕ Añadir Regla").clicked() {
                config.window_rules.push(raven_core::config::WindowRule {
                    class: "nueva_app".to_string(),
                    action: "float".to_string(),
                    pip: false,
                });
            }
        });
    });

    ui.add_space(12.0);

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.heading(egui::RichText::new("☣️ Lista de Cuarentena").strong().size(14.0));
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Coloca aqui el nombre de aquella aplicación que cuando nazca rompa la composición o que fuera de lugar.").weak().size(11.0));
            ui.add_space(6.0);

            let mut to_delete = None;
            for (idx, class_name) in config.quarantine_classes.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(class_name);
                    if ui.button("🗑 Quitar").clicked() {
                        to_delete = Some(idx);
                    }
                });
                ui.add_space(4.0);
            }

            if let Some(idx) = to_delete {
                config.quarantine_classes.remove(idx);
            }

            ui.add_space(6.0);
            if ui.button("➕ Añadir a la Lista").clicked() {
                config.quarantine_classes.push("ejemplo_app".to_string());
            }
        });
    });
}
