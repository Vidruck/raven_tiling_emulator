// ============================================================================
// RAVEN GUI — PESTAÑA DE ALGORITMOS Y PRESETS (tabs/layouts_tab.rs)
// ============================================================================

use eframe::egui;
use raven_core::config::RavenConfig;
use crate::kde_theme::KdePalette;
use crate::models::PRESETS;
use crate::components::layout_preview::draw_layout_preview;

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
