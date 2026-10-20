//! # Pestaña de Reglas de Ventana y Cuarentena CSD (`composition_tab.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
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

    // ── Isleta de Configuración de Bordes y Contorno Activo/Inactivo ──
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.heading(egui::RichText::new("✨ Contorno de Foco y Resaltado de Ventanas").strong().size(14.0));
            ui.label(egui::RichText::new("Dibuja un marco delimitador nativo por hardware mediante el efecto KWin para indicar el foco de ventanas.").weak().size(11.0));
            ui.add_space(8.0);

            ui.checkbox(&mut config.borders_enabled, "Habilitar contornos de ventana");

            if config.borders_enabled {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Grosor del contorno (px):");
                    ui.add(egui::Slider::new(&mut config.border_width, 1.0..=6.0).step_by(0.5).suffix(" px"));
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label("Ventana activa (foco):");
                    let mut active_c = parse_hex_color(&config.active_border_color);
                    if ui.color_edit_button_srgba(&mut active_c).changed() {
                        config.active_border_color = format!("#{:02x}{:02x}{:02x}", active_c.r(), active_c.g(), active_c.b());
                    }
                    ui.text_edit_singleline(&mut config.active_border_color);
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Ventanas inactivas:");
                    let mut inactive_c = parse_hex_color(&config.inactive_border_color);
                    if ui.color_edit_button_srgba(&mut inactive_c).changed() {
                        config.inactive_border_color = format!("#{:02x}{:02x}{:02x}", inactive_c.r(), inactive_c.g(), inactive_c.b());
                    }
                    ui.text_edit_singleline(&mut config.inactive_border_color);
                });
            }
        });
    });

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

fn parse_hex_color(hex: &str) -> egui::Color32 {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() >= 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(61);
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(174);
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(233);
        let a = if clean.len() >= 8 {
            u8::from_str_radix(&clean[6..8], 16).unwrap_or(255)
        } else {
            255
        };
        egui::Color32::from_rgba_unmultiplied(r, g, b, a)
    } else {
        egui::Color32::from_rgb(61, 174, 233)
    }
}
