use eframe::egui;
use raven_core::config::{RavenConfig, WindowRule};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

mod kde_theme;
use kde_theme::KdePalette;
use notify::{Watcher, RecursiveMode};
use std::sync::mpsc::{channel, Receiver};

/// Preset de layout para la UI (espejo del dominio Rust).
struct PresetDef {
    name: &'static str,
    display: &'static str,
    desc: &'static str,
    layout_type: &'static str,
    gaps: i32,
    ratio: f32,
}

const PRESETS: &[PresetDef] = &[
    PresetDef { name: "raven",      display: "Raven (Base)",     desc: "Esquema dinámico y asimétrico para pantallas panorámicas.", layout_type: "raven", gaps: 6, ratio: 0.5 },
    PresetDef { name: "clasico",    display: "Clásico",          desc: "Esquema de panel maestro con pila secundaria.", layout_type: "tall", gaps: 8, ratio: 0.55 },
    PresetDef { name: "monoculo",   display: "Monóculo",         desc: "Modo maximizado de una sola ventana.", layout_type: "monocle", gaps: 0, ratio: 1.0 },
    PresetDef { name: "hyper",      display: "Flujo Avanzado",   desc: "Mosaico fractal estrictamente simétrico en espiral.", layout_type: "strict_dwindle", gaps: 8, ratio: 0.5 },
    PresetDef { name: "divisor",    display: "Divisor",          desc: "Disposición equitativa en columnas proporcionales.", layout_type: "divisor", gaps: 8, ratio: 0.5 },
];

/// Aplicación gráfica del Centro de Bienvenida (Welcome Center) de Raven.
struct RavenGuiApp {
    config: RavenConfig,
    config_path: PathBuf,
    status_msg: String,
    is_active_cache: bool,
    last_active_check: Option<Instant>,
    kde_palette: KdePalette,
    _watcher: Option<notify::RecommendedWatcher>,
    rx_theme: Option<Receiver<notify::Result<notify::Event>>>,
}

impl RavenGuiApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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
            config_path, 
            status_msg: String::new(),
            is_active_cache: false,
            last_active_check: None,
            kde_palette,
            _watcher: watcher,
            rx_theme: Some(rx),
        }
    }

    fn is_service_active(&mut self) -> bool {
        let now = Instant::now();
        if let Some(last) = self.last_active_check {
            if now.duration_since(last) < Duration::from_secs(2) {
                return self.is_active_cache;
            }
        }
        
        let active = Command::new("systemctl")
            .args(["--user", "is-active", "raven.service"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
            .unwrap_or(false);
            
        self.is_active_cache = active;
        self.last_active_check = Some(now);
        active
    }

    fn start_service(&mut self) {
        let ok = Command::new("systemctl")
            .args(["--user", "start", "raven.service"])
            .status()
            .is_ok();
        self.status_msg = if ok {
            "✅ Motor Raven iniciado con éxito.".to_string()
        } else {
            "❌ Error al iniciar el servicio.".to_string()
        };
        self.last_active_check = None; // Forzar rechequeo
    }

    fn stop_service(&mut self) {
        let ok = Command::new("systemctl")
            .args(["--user", "stop", "raven.service"])
            .status()
            .is_ok();
        self.status_msg = if ok {
            "🛑 Motor Raven apagado con éxito.".to_string()
        } else {
            "❌ Error al detener el servicio.".to_string()
        };
        self.last_active_check = None; // Forzar rechequeo
    }

    fn save_and_restart(&mut self) {
        if let Some(parent) = self.config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&self.config) {
            if fs::write(&self.config_path, json).is_ok() {
                if self.is_service_active() {
                    let ok = Command::new("systemctl")
                        .args(["--user", "restart", "raven.service"])
                        .status()
                        .is_ok();
                    self.status_msg = if ok {
                        "✅ Configuración guardada y servicio reiniciado.".to_string()
                    } else {
                        "❌ Error al reiniciar el servicio.".to_string()
                    };
                } else {
                    self.status_msg = "✅ Configuración guardada correctamente.".to_string();
                }
            } else {
                self.status_msg = "❌ Error al guardar la configuración.".to_string();
            }
        }
    }

    fn draw_layout_preview(ui: &mut egui::Ui, layout_type: &str, ratio: f32, gaps: i32, palette: &KdePalette) {
        let desired_size = egui::vec2(ui.available_width().min(320.0), 120.0);
        let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        let painter = ui.painter_at(rect);
        let bg = palette.window_bg;
        painter.rect_filled(rect, 6.0, bg);

        let gap_f = gaps as f32 * 0.5;
        let w = rect.width();
        let h = rect.height();

        let center_color = palette.selection_bg;
        let side_color   = palette.button_bg;
        let bot_color    = palette.view_bg;

        match layout_type {
            "tall" => {
                let master_w = w * ratio;
                let stack_w = w - master_w;
                let stack_h = h / 4.0;

                let master = egui::Rect::from_min_size(
                    rect.min + egui::vec2(gap_f, gap_f),
                    egui::vec2(master_w - gap_f * 2.0, h - gap_f * 2.0),
                );
                painter.rect_filled(master, 4.0, center_color);
                painter.text(master.center(), egui::Align2::CENTER_CENTER, "1 (foco)", egui::FontId::monospace(11.0), egui::Color32::WHITE);

                for i in 0..4 {
                    let r = egui::Rect::from_min_size(
                        rect.min + egui::vec2(master_w + gap_f, i as f32 * stack_h + gap_f),
                        egui::vec2(stack_w - gap_f * 2.0, stack_h - gap_f * 2.0),
                    );
                    painter.rect_filled(r, 4.0, side_color);
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, &(i + 2).to_string(), egui::FontId::monospace(10.0), egui::Color32::WHITE);
                }
            }
            "monocle" => {
                let r = egui::Rect::from_min_size(
                    rect.min + egui::vec2(gap_f, gap_f),
                    egui::vec2(w - gap_f * 2.0, h - gap_f * 2.0),
                );
                painter.rect_filled(r, 4.0, center_color);
                painter.text(r.center(), egui::Align2::CENTER_CENTER, "1 (maximizado)", egui::FontId::monospace(12.0), egui::Color32::WHITE);
            }
            "strict_dwindle" => {
                // Dibujamos espiral: 1 izquierda, 2 der-arriba, 3 der-abajo-izq, 4 der-abajo-der-arriba, 5 der-abajo-der-abajo
                let curr_x = rect.min.x;
                let curr_y = rect.min.y;
                
                // Win 1
                let w1_w = w * ratio;
                let r1 = egui::Rect::from_min_size(
                    egui::pos2(curr_x + gap_f, curr_y + gap_f),
                    egui::vec2(w1_w - gap_f * 2.0, h - gap_f * 2.0),
                );
                painter.rect_filled(r1, 4.0, center_color);
                painter.text(r1.center(), egui::Align2::CENTER_CENTER, "1 (foco)", egui::FontId::monospace(11.0), egui::Color32::WHITE);

                // Win 2
                let rem_w = w - w1_w;
                let w2_h = h * ratio;
                let r2 = egui::Rect::from_min_size(
                    egui::pos2(curr_x + w1_w + gap_f, curr_y + gap_f),
                    egui::vec2(rem_w - gap_f * 2.0, w2_h - gap_f * 2.0),
                );
                painter.rect_filled(r2, 4.0, side_color);
                painter.text(r2.center(), egui::Align2::CENTER_CENTER, "2", egui::FontId::monospace(11.0), egui::Color32::WHITE);

                // Win 3
                let rem_h = h - w2_h;
                let w3_w = rem_w * ratio;
                let r3 = egui::Rect::from_min_size(
                    egui::pos2(curr_x + w1_w + gap_f, curr_y + w2_h + gap_f),
                    egui::vec2(w3_w - gap_f * 2.0, rem_h - gap_f * 2.0),
                );
                painter.rect_filled(r3, 4.0, bot_color);
                painter.text(r3.center(), egui::Align2::CENTER_CENTER, "3", egui::FontId::monospace(10.0), egui::Color32::WHITE);

                // Win 4
                let rem_w2 = rem_w - w3_w;
                let w4_h = rem_h * ratio;
                let r4 = egui::Rect::from_min_size(
                    egui::pos2(curr_x + w1_w + w3_w + gap_f, curr_y + w2_h + gap_f),
                    egui::vec2(rem_w2 - gap_f * 2.0, w4_h - gap_f * 2.0),
                );
                painter.rect_filled(r4, 4.0, bot_color);
                painter.text(r4.center(), egui::Align2::CENTER_CENTER, "4", egui::FontId::monospace(9.0), egui::Color32::WHITE);

                // Win 5
                let rem_h2 = rem_h - w4_h;
                let r5 = egui::Rect::from_min_size(
                    egui::pos2(curr_x + w1_w + w3_w + gap_f, curr_y + w2_h + w4_h + gap_f),
                    egui::vec2(rem_w2 - gap_f * 2.0, rem_h2 - gap_f * 2.0),
                );
                painter.rect_filled(r5, 4.0, bot_color);
                painter.text(r5.center(), egui::Align2::CENTER_CENTER, "5", egui::FontId::monospace(9.0), egui::Color32::WHITE);
            }
            "divisor" => {
                let n = 5;
                let col_w = w / n as f32;
                for i in 0..n {
                    let r = egui::Rect::from_min_size(
                        rect.min + egui::vec2(i as f32 * col_w + gap_f, gap_f),
                        egui::vec2(col_w - gap_f * 2.0, h - gap_f * 2.0),
                    );
                    let color = if i == 0 { center_color } else { side_color };
                    let lbl = if i == 0 { "1" } else { "" };
                    painter.rect_filled(r, 4.0, color);
                    if i == 0 {
                        painter.text(r.center(), egui::Align2::CENTER_CENTER, lbl, egui::FontId::monospace(10.0), egui::Color32::WHITE);
                    } else {
                        painter.text(r.center(), egui::Align2::CENTER_CENTER, &(i+1).to_string(), egui::FontId::monospace(10.0), egui::Color32::WHITE);
                    }
                }
            }
            "raven" | _ => {
                let sidebar_w = w * (1.0 - ratio.clamp(0.35, 0.85)) / 2.0;
                let center_w  = w - 2.0 * sidebar_w;
                let bot_h     = h * 0.28;
                let top_h     = h - bot_h;

                let left = egui::Rect::from_min_size(
                    rect.min + egui::vec2(gap_f, gap_f),
                    egui::vec2(sidebar_w - gap_f * 2.0, h - gap_f * 2.0),
                );
                painter.rect_filled(left, 4.0, side_color);
                painter.text(left.center(), egui::Align2::CENTER_CENTER, "2", egui::FontId::monospace(11.0), egui::Color32::WHITE);

                let center = egui::Rect::from_min_size(
                    rect.min + egui::vec2(sidebar_w + gap_f, gap_f),
                    egui::vec2(center_w - gap_f * 2.0, top_h - gap_f * 2.0),
                );
                painter.rect_filled(center, 4.0, center_color);
                painter.text(center.center(), egui::Align2::CENTER_CENTER, "1 (foco)", egui::FontId::monospace(11.0), egui::Color32::WHITE);

                let right = egui::Rect::from_min_size(
                    rect.min + egui::vec2(sidebar_w + center_w + gap_f, gap_f),
                    egui::vec2(sidebar_w - gap_f * 2.0, h - gap_f * 2.0),
                );
                painter.rect_filled(right, 4.0, side_color);
                painter.text(right.center(), egui::Align2::CENTER_CENTER, "3", egui::FontId::monospace(11.0), egui::Color32::WHITE);

                let bot_left = egui::Rect::from_min_size(
                    rect.min + egui::vec2(sidebar_w + gap_f, top_h + gap_f),
                    egui::vec2(center_w / 2.0 - gap_f * 1.5, bot_h - gap_f * 2.0),
                );
                painter.rect_filled(bot_left, 4.0, bot_color);
                painter.text(bot_left.center(), egui::Align2::CENTER_CENTER, "4", egui::FontId::monospace(10.0), egui::Color32::WHITE);

                let bot_right = egui::Rect::from_min_size(
                    rect.min + egui::vec2(sidebar_w + center_w / 2.0 + gap_f * 0.5, top_h + gap_f),
                    egui::vec2(center_w / 2.0 - gap_f * 1.5, bot_h - gap_f * 2.0),
                );
                painter.rect_filled(bot_right, 4.0, bot_color);
                painter.text(bot_right.center(), egui::Align2::CENTER_CENTER, "5", egui::FontId::monospace(10.0), egui::Color32::WHITE);
            }
        }
    }
}

impl eframe::App for RavenGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(rx) = &self.rx_theme {
            let mut changed = false;
            while let Ok(_) = rx.try_recv() {
                changed = true;
            }
            if changed {
                if let Some(p) = KdePalette::read_from_system() {
                    self.kde_palette = p;
                }
                ctx.request_repaint();
            }
        }

        let mut visuals = if self.kde_palette.is_dark { egui::Visuals::dark() } else { egui::Visuals::light() };

        visuals.window_fill = self.kde_palette.window_bg;
        visuals.panel_fill = self.kde_palette.window_bg;
        visuals.widgets.noninteractive.bg_fill = self.kde_palette.window_bg;
        visuals.widgets.inactive.bg_fill = self.kde_palette.button_bg;
        visuals.widgets.hovered.bg_fill = self.kde_palette.selection_bg;
        visuals.widgets.active.bg_fill = self.kde_palette.selection_bg;
        
        visuals.widgets.noninteractive.fg_stroke.color = self.kde_palette.window_fg;
        visuals.widgets.inactive.fg_stroke.color = self.kde_palette.button_fg;
        visuals.widgets.hovered.fg_stroke.color = self.kde_palette.selection_fg;
        visuals.widgets.active.fg_stroke.color = self.kde_palette.selection_fg;

        visuals.window_rounding = 12.0.into();
        ctx.set_visuals(visuals);

        let accent = self.kde_palette.selection_bg;

        egui::TopBottomPanel::bottom("footer_panel").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let button = ui.add_sized(
                    [160.0, 32.0],
                    egui::Button::new(egui::RichText::new("Guardar Cambios").strong()),
                );
                if button.clicked() {
                    self.save_and_restart();
                }
                ui.add_space(12.0);
                ui.label(egui::RichText::new(&self.status_msg).size(13.0));
            });
            ui.add_space(8.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(12.0, 14.0);

            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(
                    egui::RichText::new("🐦 Raven — Centro de Bienvenida")
                        .size(24.0)
                        .strong()
                        .color(accent),
                );
                ui.label(
                    egui::RichText::new("Motor nativo de mosaico para KDE Plasma 6 — v2.8")
                        .weak()
                        .size(13.0),
                );
                ui.add_space(6.0);
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {

                // ── Sección 1: Control del Motor ──
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("⚙️ Control del Motor Raven").strong().size(15.0));
                            ui.add_space(10.0);
                            let active = self.is_service_active();
                            if active {
                                ui.colored_label(egui::Color32::from_rgb(76, 201, 240), "● Activo");
                            } else {
                                ui.colored_label(egui::Color32::from_rgb(247, 37, 133), "● Inactivo");
                            }
                        });
                        ui.add_space(4.0);
                        ui.label("Administra el demonio en segundo plano que gestiona tus ventanas.");
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            if ui.add_sized([130.0, 28.0], egui::Button::new("▶ Arrancar Motor")).clicked() {
                                self.start_service();
                            }
                            ui.add_space(10.0);
                            if ui.add_sized([130.0, 28.0], egui::Button::new("⏹ Apagar Motor")).clicked() {
                                self.stop_service();
                            }
                        });
                        ui.add_space(8.0);
                        ui.checkbox(
                            &mut self.config.tiling_enabled_on_startup,
                            "Arrancar por defecto (Ejecutar Raven al iniciar la sesión)",
                        );
                    });
                });

                ui.add_space(4.0);

                // ── Sección 2: Presets de Layout ──
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🎨 Preset de Composición Foveal").strong().size(15.0));
                        ui.add_space(4.0);
                        ui.label("Selecciona un perfil de distribución. Los valores de gaps y ratio se ajustarán automáticamente.");
                        ui.add_space(8.0);

                        let current_preset = self.config.layout_preset.clone();
                        let mut selected_preset = current_preset.clone();

                        for preset in PRESETS {
                            let is_sel = preset.name == selected_preset;
                            ui.horizontal(|ui| {
                                if ui.radio(is_sel, egui::RichText::new(preset.display).strong()).clicked() {
                                    selected_preset = preset.name.to_string();
                                }
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new(preset.desc).weak().size(12.0));
                            });
                        }

                        // Aplicar preset si cambió
                        if selected_preset != current_preset {
                            if let Some(p) = PRESETS.iter().find(|p| p.name == selected_preset) {
                                self.config.layout_preset = p.name.to_string();
                                self.config.layout_type   = p.layout_type.to_string();
                                self.config.default_gaps  = p.gaps;
                                self.config.master_ratio  = p.ratio;
                            }
                        }

                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Preview del Layout (5 ventanas):").size(12.0).weak());
                        ui.add_space(4.0);
                        Self::draw_layout_preview(
                            ui,
                            &self.config.layout_type,
                            self.config.master_ratio,
                            self.config.default_gaps,
                            &self.kde_palette,
                        );
                    });
                });

                ui.add_space(4.0);

                // ── Sección 3: Ajustes de Composición ──
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📐 Ajustes de Composición").strong().size(15.0));
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Márgenes entre ventanas (Gaps):");
                            ui.add(egui::Slider::new(&mut self.config.default_gaps, 0..=40).suffix(" px"));
                        });

                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Ratio de ventana central:");
                            ui.add(
                                egui::Slider::new(&mut self.config.master_ratio, 0.35..=0.85)
                                    .fixed_decimals(2),
                            );
                        });

                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Altura del panel del sistema (px):");
                            ui.add(egui::Slider::new(&mut self.config.panel_height, 20..=80).suffix(" px"));
                        });

                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Posición preferida de Picture-in-Picture (PiP):");
                            egui::ComboBox::from_id_source("pip_pos")
                                .selected_text(match self.config.pip_position.as_str() {
                                    "top-left"    => "Superior Izquierda",
                                    "top-right"   => "Superior Derecha",
                                    "bottom-left" => "Inferior Izquierda",
                                    "bottom-right"=> "Inferior Derecha",
                                    other         => other,
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut self.config.pip_position, "top-left".to_string(),    "Superior Izquierda");
                                    ui.selectable_value(&mut self.config.pip_position, "top-right".to_string(),   "Superior Derecha");
                                    ui.selectable_value(&mut self.config.pip_position, "bottom-left".to_string(), "Inferior Izquierda");
                                    ui.selectable_value(&mut self.config.pip_position, "bottom-right".to_string(),"Inferior Derecha");
                                });
                        });
                    });
                });

                // ── Sección 4: Reglas y Cuarentenas ──
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🛡️ Reglas y Excepciones").strong().size(15.0));
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("Clases en cuarentena (separadas por coma):").weak());
                        
                        let mut quarantine_str = self.config.quarantine_classes.join(", ");
                        if ui.text_edit_singleline(&mut quarantine_str).changed() {
                            self.config.quarantine_classes = quarantine_str
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();
                        }

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("Reglas específicas por clase de ventana:").weak());
                        
                        let mut rules_to_remove = Vec::new();
                        for (i, rule) in self.config.window_rules.iter_mut().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label("Clase:");
                                ui.add(egui::TextEdit::singleline(&mut rule.class).desired_width(100.0));
                                
                                egui::ComboBox::from_id_source(format!("action_{}", i))
                                    .selected_text(&rule.action)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut rule.action, "float".to_string(), "Flotante (Float)");
                                    });
                                
                                ui.checkbox(&mut rule.pip, "PiP");
                                
                                if ui.button("❌").clicked() {
                                    rules_to_remove.push(i);
                                }
                            });
                        }
                        
                        for idx in rules_to_remove.into_iter().rev() {
                            self.config.window_rules.remove(idx);
                        }

                        if ui.button("➕ Agregar Regla").clicked() {
                            self.config.window_rules.push(WindowRule {
                                class: "".to_string(),
                                action: "float".to_string(),
                                pip: false,
                            });
                        }
                    });
                });

                ui.add_space(4.0);

                // ── Sección 5: Guía Rápida ──
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📖 Guía Rápida de Comportamiento").strong().size(15.0));
                        ui.add_space(4.0);

                        ui.label(egui::RichText::new("🔄 Habilitar / Deshabilitar el Mosaico:").strong());
                        ui.label("• Alterna el mosaico global con 'Meta + Backspace' o desde el Plasmoid.");

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("📊 Ajuste de Proporción & Reinicio Automático:").strong());
                        ui.label("• Modifica el ratio con 'Meta + H / L'. Al abrir o cerrar ventanas, el ratio se restablece a 0.5.");

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("🗂️ Control de Capacidad (nmaster):").strong());
                        ui.colored_label(accent, "• Meta + I / D: Incrementa/Decrementa el nº de ventanas en el área central.");

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("🚚 Intercambio & Migración:").strong());
                        ui.label("• 'Meta + Shift + J / K': Intercambia posición de ventanas.");
                        ui.label("• 'Meta + Alt + Flechas': Migra la ventana activa a otro monitor/escritorio.");

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("⌨️ Atajos de Teclado:").strong());
                        for (key, desc) in [
                            ("Meta + H / L",         "Ajustar ratio de la ventana"),
                            ("Meta + I / D",         "Incrementar/Decrementar nmaster"),
                            ("Meta + Shift + J / K", "Intercambiar posición de ventanas"),
                            ("Meta + Alt + Flechas", "Migrar ventana a otro escritorio/monitor"),
                            ("Meta + G",             "Toggle global del mosaico"),
                        ] {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(key).strong().color(egui::Color32::LIGHT_GRAY));
                                ui.label(format!("→ {}", desc));
                            });
                        }
                    });
                });
            });
        });
    }
}

/// Punto de entrada principal del Centro de Bienvenida de Raven v2.8.
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 800.0])
            .with_title("Raven — Centro de Bienvenida v2.8"),
        ..Default::default()
    };

    eframe::run_native(
        "Raven Config",
        options,
        Box::new(|cc| Box::new(RavenGuiApp::new(cc))),
    )
}
