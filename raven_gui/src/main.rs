use eframe::egui;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Estructura de configuración (configuration structure) del motor Raven.
///
/// Contiene parámetros serializables (serializable parameters) para su persistencia en disco.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct RavenConfig {
    /// Espacio (gaps) por defecto entre ventanas en píxeles.
    pub default_gaps: i32,
    /// Indica si el motor de mosaico (tiling engine) debe activarse al iniciar sesión.
    pub tiling_enabled_on_startup: bool,
    /// Número óptimo de ventanas en el área de mosaico Dwindle BSP antes de desalojar.
    pub nmaster: usize,
    /// Proporción del área de corte asimétrico (master ratio).
    pub master_ratio: f32,
    /// Posición por defecto para ventanas Picture-in-Picture (PiP).
    pub pip_position: String,
}

impl Default for RavenConfig {
    /// Inicializa la configuración con los valores predeterminados (default values).
    fn default() -> Self {
        Self {
            default_gaps: 8,
            tiling_enabled_on_startup: true,
            nmaster: 1,
            master_ratio: 0.5,
            pip_position: "bottom-right".to_string(),
        }
    }
}

/// Aplicación gráfica del Centro de Bienvenida (Welcome Center) de Raven.
struct RavenGuiApp {
    /// Estado de configuración en memoria (in-memory config).
    config: RavenConfig,
    /// Ruta del archivo de configuración (config file path) JSON.
    config_path: PathBuf,
    /// Mensaje de estado (status message) para retroalimentación visual.
    status_msg: String,
}

impl RavenGuiApp {
    /// Crea una nueva instancia de `RavenGuiApp` cargando datos del disco.
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let home = env::var("HOME").unwrap_or_else(|_| "~".to_string());
        let config_path = PathBuf::from(format!("{}/.config/raven/raven.json", home));

        let config = if let Ok(content) = fs::read_to_string(&config_path) {
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            RavenConfig::default()
        };

        Self {
            config,
            config_path,
            status_msg: String::new(),
        }
    }

    /// Comprueba si el demonio (daemon) de Raven está activo mediante systemd.
    fn is_service_active(&self) -> bool {
        Command::new("systemctl")
            .arg("--user")
            .arg("is-active")
            .arg("raven.service")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
            .unwrap_or(false)
    }

    /// Lanza (start) el servicio de Raven.
    fn start_service(&mut self) {
        let status = Command::new("systemctl")
            .arg("--user")
            .arg("start")
            .arg("raven.service")
            .status();

        if status.is_ok() {
            self.status_msg = "✅ Motor Raven iniciado con éxito.".to_string();
        } else {
            self.status_msg = "❌ Error al iniciar el servicio.".to_string();
        }
    }

    /// Detiene (stop) el servicio de Raven.
    fn stop_service(&mut self) {
        let status = Command::new("systemctl")
            .arg("--user")
            .arg("stop")
            .arg("raven.service")
            .status();

        if status.is_ok() {
            self.status_msg = "🛑 Motor Raven apagado con éxito.".to_string();
        } else {
            self.status_msg = "❌ Error al detener el servicio.".to_string();
        }
    }

    /// Guarda la configuración en disco y reinicia (restart) el motor de mosaico si está activo.
    fn save_and_restart(&mut self) {
        if let Some(parent) = self.config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(json) = serde_json::to_string_pretty(&self.config) {
            if fs::write(&self.config_path, json).is_ok() {
                if self.is_service_active() {
                    let status = Command::new("systemctl")
                        .arg("--user")
                        .arg("restart")
                        .arg("raven.service")
                        .status();

                    if status.is_ok() {
                        self.status_msg = "✅ Configuración guardada y servicio reiniciado.".to_string();
                    } else {
                        self.status_msg = "❌ Error al reiniciar el servicio.".to_string();
                    }
                } else {
                    self.status_msg = "✅ Configuración guardada correctamente.".to_string();
                }
            } else {
                self.status_msg = "❌ Error al guardar la configuración.".to_string();
            }
        }
    }
}

impl eframe::App for RavenGuiApp {
    /// Renderiza y procesa los eventos del Centro de Bienvenida en tiempo de ejecución.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Paleta de colores dinámica (dynamic color palette) basada en el modo claro/oscuro del usuario (light/dark mode)
        let is_dark = ctx.style().visuals.dark_mode;
        let mut visuals = if is_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        // Personalización de colores de acento y fondos de paneles
        if is_dark {
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 180, 216); // Cian brillante
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(0, 119, 182); // Azul hover
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(30, 41, 59); // Slate-800 (pizarra oscura)
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(15, 23, 42); // Slate-900 (fondo profundo)
        } else {
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(0, 119, 182); // Azul
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(3, 4, 94); // Azul oscuro hover
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(226, 232, 240); // Slate-200 (gris claro)
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(241, 245, 249); // Slate-100 (fondo claro)
        }
        visuals.window_rounding = 12.0.into();
        ctx.set_visuals(visuals);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(12.0, 14.0);

            // Cabecera / Banner del Centro de Bienvenida
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading(
                    egui::RichText::new("🐦 Raven - Centro de Bienvenida")
                        .size(24.0)
                        .strong()
                        .color(if is_dark { egui::Color32::from_rgb(0, 180, 216) } else { egui::Color32::from_rgb(0, 119, 182) }),
                );
                ui.label(
                    egui::RichText::new("¡Bienvenido al motor nativo de mosaico para KDE Plasma 6!")
                        .weak()
                        .size(13.0),
                );
                ui.add_space(6.0);
            });

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                // Sección 1: Control del Motor
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
                            let btn_start = ui.add_sized([130.0, 28.0], egui::Button::new("▶ Arrancar Motor"));
                            if btn_start.clicked() {
                                self.start_service();
                            }

                            ui.add_space(10.0);

                            let btn_stop = ui.add_sized([130.0, 28.0], egui::Button::new("⏹ Apagar Motor"));
                            if btn_stop.clicked() {
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

                // Sección 2: Ajustes Rápidos (Quick Settings)
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📐 Ajustes de Composición").strong().size(15.0));
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Márgenes entre ventanas (Gaps):");
                            ui.add(
                                egui::Slider::new(&mut self.config.default_gaps, 0..=40)
                                    .suffix(" px"),
                            );
                        });

                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("Posición preferida de Picture-in-Picture (PiP):");
                            egui::ComboBox::from_id_source("pip_pos")
                                .selected_text(match self.config.pip_position.as_str() {
                                    "top-left" => "Superior Izquierda",
                                    "top-right" => "Superior Derecha",
                                    "bottom-left" => "Inferior Izquierda",
                                    "bottom-right" => "Inferior Derecha",
                                    _ => &self.config.pip_position,
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.config.pip_position,
                                        "top-left".to_string(),
                                        "Superior Izquierda",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.pip_position,
                                        "top-right".to_string(),
                                        "Superior Derecha",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.pip_position,
                                        "bottom-left".to_string(),
                                        "Inferior Izquierda",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.pip_position,
                                        "bottom-right".to_string(),
                                        "Inferior Derecha",
                                    );
                                });
                        });
                    });
                });

                ui.add_space(4.0);

                // Sección 3: Manual de Usuario Expandido
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📖 Guía Rápida de Comportamiento").strong().size(15.0));
                        ui.add_space(4.0);

                        ui.label(egui::RichText::new("🔄 Habilitar / Deshabilitar el Mosaico (Tiling Toggle):").strong());
                        ui.label("• Puedes alternar el mosaico global haciendo clic sobre el icono en tu panel del sistema  o presionando 'Meta + Backspace'. Al apagarlo, las ventanas recuperarán su comportamiento flotante ordinario.");

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("📊 Ajuste de Proporción (Ratio) & Reinicio Automático:").strong());
                        ui.label("• Modifica el tamaño de la ventana activa con 'Meta + H / L'. Las demás se ajustarán.");
                        ui.colored_label(
                            if is_dark { egui::Color32::from_rgb(76, 201, 240) } else { egui::Color32::from_rgb(0, 119, 182) },
                            "• ¡Orden Garantizado!: Al abrir o cerrar cualquier ventana, el ratio se restablece a 0.5 (50-50) para evitar geometrías deformes.",
                        );

                        ui.add_space(6.0);
                        ui.label(egui::RichText::new("🚚 Intercambio (Swap) & Migración de Ventanas (Migration):").strong());
                        ui.label("• Usa los botones del Plasmoide o presiona 'Meta + Shift + J / K' para alternar la posición física de dos ventanas en el mosaico.");
                        ui.label("• Envía la ventana activa a otro monitor físico o escritorio virtual con 'Meta + Alt + [Flechas]'. El mosaico restante se reorganizará de manera limpia.");

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new("⌨️ Atajos de Teclado Rápidos (Shortcuts):").strong());
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Meta + H / L").strong().color(egui::Color32::LIGHT_GRAY));
                            ui.label("→ Ajustar proporción (Ratio) de la ventana");
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Meta + Shift + J / K").strong().color(egui::Color32::LIGHT_GRAY));
                            ui.label("→ Intercambiar (Swap) posición de ventanas");
                        });
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Meta + Alt + Flechas").strong().color(egui::Color32::LIGHT_GRAY));
                            ui.label("→ Migrar ventana en foco a otro escritorio / monitor");
                        });
                    });
                });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            // Botón de guardado inferior y mensaje de estado
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
        });
    }
}

/// Punto de entrada principal (main entrypoint) para el Centro de Bienvenida de Raven.
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([650.0, 720.0])
            .with_title("Raven - Centro de Bienvenida"),
        ..Default::default()
    };

    eframe::run_native(
        "Raven Config",
        options,
        Box::new(|cc| Box::new(RavenGuiApp::new(cc))),
    )
}
