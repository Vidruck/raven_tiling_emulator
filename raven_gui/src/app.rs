// ============================================================================
// RAVEN GUI — ESTRUCTURA Y CICLO DE VIDA DE LA APLICACIÓN (app.rs)
// ============================================================================

use eframe::egui;
use raven_core::config::RavenConfig;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use notify::{Watcher, RecursiveMode};

use crate::models::NavTab;
use crate::kde_theme::KdePalette;
use crate::services::ServiceManager;
use crate::tabs;

pub struct RavenGuiApp {
    pub config: RavenConfig,
    pub service_mgr: ServiceManager,
    pub status_msg: String,
    pub kde_palette: KdePalette,
    pub active_tab: NavTab,
    pub _watcher: Option<notify::RecommendedWatcher>,
    pub rx_theme: Option<Receiver<notify::Result<notify::Event>>>,
}

impl RavenGuiApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
        let config_path = PathBuf::from(format!("{}/.config/raven/raven.json", home));

        let config = if let Ok(content) = fs::read_to_string(&config_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            RavenConfig::default()
        };

        let mut kde_palette = KdePalette::default();
        if let Some(p) = KdePalette::read_from_system() {
            kde_palette = p;
        }

        let (tx, rx) = channel();
        let mut watcher = None;
        
        if let Ok(mut w) = notify::recommended_watcher(tx) {
            let kdeglobals_path = PathBuf::from(env::var("HOME").unwrap_or_default()).join(".config/kdeglobals");
            if w.watch(&kdeglobals_path, RecursiveMode::NonRecursive).is_ok() {
                watcher = Some(w);
            }
        }

        Self {
            config,
            service_mgr: ServiceManager::new(config_path),
            status_msg: String::new(),
            kde_palette,
            active_tab: NavTab::Layouts,
            _watcher: watcher,
            rx_theme: Some(rx),
        }
    }
}

impl eframe::App for RavenGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(ref rx) = self.rx_theme {
            if rx.try_recv().is_ok() {
                if let Some(p) = KdePalette::read_from_system() {
                    self.kde_palette = p;
                }
            }
        }

        let mut visuals = if self.kde_palette.is_dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        let primary_text = if self.kde_palette.is_dark {
            egui::Color32::from_rgb(245, 245, 250)
        } else {
            egui::Color32::from_rgb(20, 20, 25)
        };

        visuals.window_fill = self.kde_palette.window_bg;
        visuals.panel_fill = self.kde_palette.window_bg;
        visuals.widgets.noninteractive.bg_fill = self.kde_palette.window_bg;
        visuals.widgets.inactive.bg_fill = self.kde_palette.button_bg;
        visuals.widgets.hovered.bg_fill = self.kde_palette.selection_bg;
        visuals.widgets.active.bg_fill = self.kde_palette.selection_bg;
        visuals.override_text_color = Some(primary_text);
        visuals.selection.bg_fill = self.kde_palette.selection_bg;
        visuals.window_rounding = 14.0.into();
        ctx.set_visuals(visuals);

        let accent = self.kde_palette.selection_bg;

        // ── Panel Inferior de Acciones Principales ──
        egui::TopBottomPanel::bottom("footer_panel").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let btn = ui.add_sized(
                    [170.0, 34.0],
                    egui::Button::new(egui::RichText::new("💾 Guardar y Aplicar").strong().size(14.0)),
                );
                if btn.clicked() {
                    self.status_msg = self.service_mgr.save_and_restart(&self.config);
                }
                ui.add_space(14.0);
                ui.label(egui::RichText::new(&self.status_msg).size(13.0).strong());
            });
            ui.add_space(8.0);
        });

        // ── Panel Lateral Izquierdo ──
        egui::SidePanel::left("navigation_rail")
            .resizable(false)
            .default_width(210.0)
            .frame(
                egui::Frame::none()
                    .fill(self.kde_palette.window_bg)
                    .inner_margin(egui::Margin::same(12.0))
                    .outer_margin(egui::Margin::same(8.0))
                    .rounding(16.0)
                    .stroke(egui::Stroke::NONE)
            )
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.vertical_centered(|ui| {
                    ui.heading(
                        egui::RichText::new("🐦 Raven Tiling Emulator")
                            .size(18.0)
                            .strong()
                            .color(accent),
                    );
                    ui.label(egui::RichText::new("Centro de Control").weak().size(11.0));
                });

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(10.0);

                let nav_items = [
                    (NavTab::Layouts, "🎨 Composición"),
                    (NavTab::CompositionPip, "🛡️ Reglas para Apps"),
                    (NavTab::ShortcutsGuide, "⌨️ Manual de Atajos"),
                    (NavTab::EngineService, "⚙️ Estado del Servicio"),
                    (NavTab::About, "ℹ️ Acerca de"),
                ];

                for (tab, label) in nav_items {
                    let is_selected = self.active_tab == tab;
                    let text_color = if is_selected {
                        egui::Color32::WHITE
                    } else if self.kde_palette.is_dark {
                        egui::Color32::from_rgb(225, 225, 230)
                    } else {
                        egui::Color32::from_rgb(35, 35, 40)
                    };

                    let bg_color = if is_selected {
                        accent
                    } else {
                        egui::Color32::TRANSPARENT
                    };

                    let btn_rect = ui.allocate_space(egui::vec2(186.0, 38.0));
                    let response = ui.interact(btn_rect.1, ui.id().with(label), egui::Sense::click());

                    if response.hovered() && !is_selected {
                        ui.painter().rect_filled(btn_rect.1, 12.0, self.kde_palette.button_bg);
                    } else {
                        ui.painter().rect_filled(btn_rect.1, 12.0, bg_color);
                    }

                    ui.painter().text(
                        btn_rect.1.min + egui::vec2(16.0, 19.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        egui::FontId::proportional(13.5),
                        text_color,
                    );

                    if response.clicked() {
                        self.active_tab = tab;
                    }

                    ui.add_space(4.0);
                }
            });

        // ── Panel Central ──
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(self.kde_palette.window_bg)
                    .inner_margin(egui::Margin::same(16.0))
                    .outer_margin(egui::Margin::same(8.0))
                    .rounding(16.0)
                    .stroke(egui::Stroke::NONE)
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    match self.active_tab {
                        NavTab::Layouts => tabs::layouts_tab::show(&mut self.config, ui, accent, &self.kde_palette),
                        NavTab::CompositionPip => tabs::composition_tab::show(&mut self.config, ui, accent),
                        NavTab::EngineService => {
                            tabs::service_tab::show(
                                &mut self.config,
                                ui,
                                accent,
                                &mut self.service_mgr,
                                &mut self.status_msg,
                            );
                        }
                        NavTab::ShortcutsGuide => tabs::shortcuts_tab::show(ui, accent),
                        NavTab::About => tabs::about_tab::show(ui, accent),
                    }
                });
            });
    }
}
