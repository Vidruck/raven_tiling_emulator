//! # Pestaña de Algoritmos y Presets de Mosaico (`layouts_tab.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Proporciona la interfaz interactiva para selección de algoritmos (Raven, Tall, Monocle,
//! Fibonacci Dwindle, Inverted Dwindle, Divisor) y carga de presets estilísticos predefinidos.

use eframe::egui;
use raven_core::config::RavenConfig;
use crate::kde_theme::KdePalette;
use crate::models::PRESETS;
use crate::components::layout_preview::draw_layout_preview;
use std::fs;
use std::path::PathBuf;

/// Escanea el directorio de configuración buscando layouts de usuario en formato `.lua`.
fn get_custom_layouts() -> Vec<String> {
    let mut layouts = Vec::new();
    let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/home/vidruck".to_string());
    let layouts_dir = PathBuf::from(home_dir).join(".config/raven/layouts");

    if let Ok(entries) = fs::read_dir(layouts_dir) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    let path = entry.path();
                    if path.extension().is_some_and(|ext| ext == "lua") {
                        if let Some(name) = path.file_stem().and_then(|n| n.to_str()) {
                            layouts.push(name.to_string());
                        }
                    }
                }
            }
        }
    }
    layouts.sort();
    layouts
}

/// Renderiza la vista de selección de algoritmos y presets de composición.
pub fn show(config: &mut RavenConfig, ui: &mut egui::Ui, accent: egui::Color32, palette: &KdePalette) {
    ui.heading(egui::RichText::new("🎨 Composición").strong().size(18.0).color(accent));
    ui.label(egui::RichText::new("Personaliza la disposición geométrica, márgenes y anclaje PiP en tiempo real.").weak());
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Cargar Preset Rápido:").strong());
        for p in PRESETS {
            if ui.button(p.display).clicked() {
                config.layout_type = p.layout_type.to_string();
                config.default_gaps = p.gaps;
                config.master_ratio = p.ratio;
            }
        }
    });

    ui.add_space(12.0);
    ui.separator();
    ui.add_space(10.0);

    ui.columns(2, |cols| {
        cols[0].vertical(|ui| {
            ui.heading(egui::RichText::new("Algoritmo de Composición").strong().size(14.0));
            ui.add_space(6.0);

            for p in PRESETS {
                let is_selected = config.layout_type == p.layout_type;
                ui.radio_value(&mut config.layout_type, p.layout_type.to_string(), egui::RichText::new(p.display).strong());
                ui.label(egui::RichText::new(p.desc).weak().size(11.0));
                ui.add_space(4.0);
                if is_selected {
                    ui.label(egui::RichText::new("✔ Orden de posicionamiento en el algoritmo").color(accent).size(11.0).strong());
                }
                ui.add_space(8.0);
            }

            // [NUEVO] Sección de Layouts Personalizados (Lua)
            ui.add_space(8.0);
            ui.heading(egui::RichText::new("Scripts Lua").strong().size(13.5));
            ui.add_space(6.0);

            let custom_layouts = get_custom_layouts();
            if custom_layouts.is_empty() {
                ui.label(egui::RichText::new("No hay layouts personalizados.").weak().italics().size(11.0));
            } else {
                for layout_name in custom_layouts {
                    let is_selected = config.layout_type == layout_name;
                    ui.radio_value(&mut config.layout_type, layout_name.clone(), egui::RichText::new(&layout_name).strong());
                    if is_selected {
                        ui.label(egui::RichText::new("✔ Script Lua activo").color(accent).size(11.0).strong());
                    }
                    ui.add_space(4.0);
                }
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("➕ Importar Script Lua...").on_hover_text("Selecciona un archivo .lua para usar como algoritmo de tiling").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Scripts Lua", &["lua"])
                        .set_directory(std::env::var("HOME").unwrap_or_else(|_| "/".to_string()))
                        .pick_file()
                    {
                        if let Ok(content) = fs::read_to_string(&path) {
                            // Validar que el script contenga una función de retorno o sintaxis básica
                            let trimmed = content.trim();
                            if !trimmed.contains("function") && !trimmed.starts_with("return") {
                                // Advertencia: el archivo podría no ser un layout válido
                                eprintln!("Advertencia: El script Lua no parece definir una función de layout.");
                            }
                            let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/home/vidruck".to_string());
                            let target_dir = PathBuf::from(home_dir).join(".config/raven/layouts");
                            let _ = fs::create_dir_all(&target_dir);
                            
                            if let Some(file_name) = path.file_name() {
                                let target_path = target_dir.join(file_name);
                                let _ = fs::copy(&path, target_path);
                            }
                        }
                    }
                }
            });

            ui.add_space(8.0);
            // 📖 Manual interactivo y plantilla para desarrolladores
            ui.collapsing("📖 Manual y Plantilla de Algoritmos Lua", |ui| {
                ui.label(
                    egui::RichText::new(
                        "Raven ejecuta layouts Lua en un entorno seguro (sandbox sin acceso a red ni disco). \
                        El script debe retornar una función pura que reciba (screen, windows, config) y devuelva una tabla asociativa con la geometría de cada ventana:"
                    )
                    .size(11.0)
                    .weak()
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("• screen: { x, y, w, h }\n• windows: lista de { id, min_w, min_h, class, is_active }\n• config: { gaps, master_ratio, nmaster, active_id }")
                        .size(10.5)
                        .monospace()
                        .color(accent)
                );
                ui.add_space(6.0);

                let template_code = r#"return function(screen, windows, config)
    local layout = {}
    local n = #windows
    if n == 0 then return layout end

    local gaps = config.gaps or 8
    local master_ratio = config.master_ratio or 0.55
    local master_w = math.floor((screen.w - gaps * 3) * master_ratio)
    local stack_w = screen.w - master_w - gaps * 3

    -- Ventana 1: Master
    local first_id = type(windows[1]) == "table" and windows[1].id or windows[1]
    layout[first_id] = {
        x = screen.x + gaps,
        y = screen.y + gaps,
        w = (n == 1) and (screen.w - gaps * 2) or master_w,
        h = screen.h - gaps * 2
    }

    -- Ventanas restantes en Stack lateral
    if n > 1 then
        local stack_count = n - 1
        local slot_h = math.floor((screen.h - gaps * (stack_count + 1)) / stack_count)
        for i = 2, n do
            local win_id = type(windows[i]) == "table" and windows[i].id or windows[i]
            local idx = i - 2
            layout[win_id] = {
                x = screen.x + master_w + gaps * 2,
                y = screen.y + gaps + idx * (slot_h + gaps),
                w = stack_w,
                h = slot_h
            }
        end
    end

    return layout
end"#;

                ui.label(egui::RichText::new("Plantilla de referencia:").strong().size(11.0));
                ui.add_space(2.0);
                egui::ScrollArea::vertical()
                    .max_height(140.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut template_code.to_string())
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(10)
                                .lock_focus(true)
                                .interactive(false)
                        );
                    });

                ui.add_space(4.0);
                if ui.button("📋 Copiar Plantilla al Portapapeles").clicked() {
                    ui.output_mut(|o| o.copied_text = template_code.to_string());
                }
                ui.add_space(4.0);
            });

            ui.add_space(10.0);
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.heading(egui::RichText::new("Geometría de Mosaico").strong().size(13.5));
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label("Gaps:");
                        ui.add(egui::Slider::new(&mut config.default_gaps, 0..=30).suffix(" px"));
                    });
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Ratio:");
                        ui.add(egui::Slider::new(&mut config.master_ratio, 0.2..=0.8).fixed_decimals(2));
                    });
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("nmaster:");
                        ui.add(egui::Slider::new(&mut config.nmaster, 1..=4));
                    });
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Escala PiP:");
                        ui.add(egui::Slider::new(&mut config.pip_size_ratio, 0.10..=0.50).fixed_decimals(2));
                    });
                });
            });
        });

        cols[1].vertical(|ui| {
            ui.heading(egui::RichText::new("Previsualización Gráfica").strong().size(14.0));
            ui.add_space(6.0);

            draw_layout_preview(ui, &config.layout_type, config.master_ratio, config.default_gaps, &mut config.pip_position, config.pip_size_ratio, palette);

            ui.add_space(10.0);
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.label(egui::RichText::new("💡 Instrucciones de uso de PiP:").strong().size(12.0));
                    ui.label(
                        egui::RichText::new(
                            "Haz clic en alguna de las 4 esquinas del panel de previsualización para mover al instante la ventana Picture-in-Picture en esa dirección."
                        )
                        .weak()
                        .size(11.0),
                    );
                    ui.add_space(6.0);
                    ui.label(format!("• Anclaje PiP Activo: {}", config.pip_position));
                });
            });
        });
    });
}
