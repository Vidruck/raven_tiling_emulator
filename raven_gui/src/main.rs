use eframe::egui;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Configuración del motor Raven sincronizada con el archivo JSON.
///
/// Esta estructura se utiliza para la persistencia de datos y el intercambio
/// de información entre la GUI (interfaz gráfica de usuario) y el daemon (demonio) de Rust.
#[derive(Debug, Deserialize, Serialize, Clone)]
struct RavenConfig {
    /// Espacio (gaps) por defecto entre ventanas en píxeles.
    pub default_gaps: i32,
    /// Indica si el motor de mosaico (tiling) debe activarse al iniciar sesión.
    pub tiling_enabled_on_startup: bool,
    /// Número óptimo de ventanas en el área de mosaico Dwindle BSP antes de desalojar.
    pub nmaster: usize,
    /// Proporción del área de corte asimétrico (0.1 a 0.9) para la espiral.
    pub master_ratio: f32,
    /// Posición por defecto para ventanas Picture-in-Picture (PiP).
    pub pip_position: String,
}

impl Default for RavenConfig {
    /// Inicializa la configuración con los valores por defecto del sistema.
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

/// Aplicación principal de Raven Control Center basada en eframe/egui.
struct RavenGuiApp {
    /// Estado de configuración actual en memoria.
    config: RavenConfig,
    /// Ruta absoluta al archivo de configuración JSON.
    config_path: PathBuf,
    /// Mensaje de estado para retroalimentación (feedback) visual al usuario.
    status_msg: String,
}

impl RavenGuiApp {
    /// Crea una nueva instancia de `RavenGuiApp` cargando el archivo de disco.
    ///
    /// # Parámetros
    /// * `_cc` - Contexto de creación de eframe.
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

    /// Guarda la configuración actual en disco y reinicia el servicio del demonio (daemon) mediante systemd.
    ///
    /// Intenta persistir los cambios en el archivo JSON y, en caso de éxito,
    /// invoca a systemctl para refrescar el estado del motor en tiempo real.
    fn save_and_restart(&mut self) {
        if let Some(parent) = self.config_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(json) = serde_json::to_string_pretty(&self.config) {
            if fs::write(&self.config_path, json).is_ok() {
                let status = Command::new("systemctl")
                    .arg("--user")
                    .arg("restart")
                    .arg("raven.service")
                    .status();

                if status.is_ok() {
                    self.status_msg = "✅ Cambios aplicados con éxito.".to_string();
                } else {
                    self.status_msg = "❌ Error al reiniciar el servicio.".to_string();
                }
            } else {
                self.status_msg = "❌ Error al guardar configuración.".to_string();
            }
        }
    }
}

impl eframe::App for RavenGuiApp {
    /// Genera la interfaz gráfica y procesa la actualización de cuadros (frames) en egui.
    ///
    /// # Parámetros
    /// * `ctx` - Contexto de renderizado de egui.
    /// * `_frame` - Instancia del manejador de la ventana de eframe.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(10.0, 12.0);

            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                ui.heading(
                    egui::RichText::new("🐦 Raven Control Center")
                        .size(24.0)
                        .strong(),
                );
                ui.label(egui::RichText::new("Configuración nativa del motor de mosaico").weak());
                ui.add_space(10.0);
            });

            ui.separator();
            ui.add_space(5.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🚀 Comportamiento").strong());
                        ui.checkbox(
                            &mut self.config.tiling_enabled_on_startup,
                            "Activar Mosaico (Tiling) al iniciar sesión",
                        );
                    });
                });

                ui.add_space(5.0);

                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("📐 Algoritmo Master-Stack").strong());
                        ui.add_space(2.0);

                        ui.horizontal(|ui| {
                            ui.label("Proporción del área maestra:");
                            ui.add(
                                egui::Slider::new(&mut self.config.master_ratio, 0.1..=0.9)
                                    .step_by(0.05)
                                    .text(""),
                            );
                        });
                    });
                });

                ui.add_space(5.0);

                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🎨 Apariencia").strong());
                        ui.add_space(2.0);

                        ui.horizontal(|ui| {
                            ui.label("Márgenes entre ventanas (Gaps):");
                            ui.add(
                                egui::Slider::new(&mut self.config.default_gaps, 0..=50)
                                    .suffix(" px"),
                            );
                        });
                    });
                });

                ui.add_space(5.0);

                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.label(egui::RichText::new("🖼️ Picture-in-Picture (PiP)").strong());
                        ui.add_space(2.0);

                        ui.horizontal(|ui| {
                            ui.label("Posición preferida:");
                            egui::ComboBox::from_id_source("pip_pos")
                                .selected_text(match self.config.pip_position.as_str() {
                                    "top-left" => "Esquina Superior Izquierda",
                                    "top-right" => "Esquina Superior Derecha",
                                    "bottom-left" => "Esquina Inferior Izquierda",
                                    "bottom-right" => "Esquina Inferior Derecha",
                                    _ => &self.config.pip_position,
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.config.pip_position,
                                        "top-left".to_string(),
                                        "Esquina Superior Izquierda",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.pip_position,
                                        "top-right".to_string(),
                                        "Esquina Superior Derecha",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.pip_position,
                                        "bottom-left".to_string(),
                                        "Esquina Inferior Izquierda",
                                    );
                                    ui.selectable_value(
                                        &mut self.config.pip_position,
                                        "bottom-right".to_string(),
                                        "Esquina Inferior Derecha",
                                    );
                                });
                        });
                    });
                });
            });

            ui.add_space(10.0);
            ui.separator();
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let button = ui.add_sized(
                    [160.0, 30.0],
                    egui::Button::new(egui::RichText::new("Guardar y Aplicar").strong()),
                );
                if button.clicked() {
                    self.save_and_restart();
                }

                ui.add_space(10.0);
                ui.label(&self.status_msg);
            });
        });
    }
}

/// Punto de entrada de la aplicación de control gráfico Raven Control Center.
fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([500.0, 550.0])
            .with_title("Raven Control Center"),
        ..Default::default()
    };

    eframe::run_native(
        "Raven Config",
        options,
        Box::new(|cc| Box::new(RavenGuiApp::new(cc))),
    )
}
