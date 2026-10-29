//! # Pestaña Unificada de Composición, Pantallas y Algoritmos (`layouts_tab.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Unifica de forma elegante y coherente:
//! 1. El canvas vectorial interactivo de monitores Wayland y escritorios virtuales.
//! 2. Selección explícita con botón interactivo de asignación y confirmación ("Asignar en [Destino]").
//! 3. Resumen y lista viva ("¿Qué está asignado a qué?") para auditoría transparente de reglas antes de guardar.
//! 4. Previsualizador fractal dinámico en tiempo real y anclaje PiP interactivo de 8 puntos.
//! 5. Candado anti-colisiones para sincronización determinista con el archivo JSON y KWin.

use eframe::egui;
use raven_core::config::RavenConfig;
use crate::kde_theme::KdePalette;
use crate::models::PRESETS;
use crate::components::layout_preview::draw_layout_preview;
use crate::components::monitor_canvas::draw_monitor_canvas;
use crate::services::ServiceManager;
use std::fs;
use std::path::PathBuf;

/// Ámbito de aplicación del algoritmo seleccionado.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum LayoutScopeMode {
    /// Aplica al monitor seleccionado en el canvas gráfico.
    SelectedMonitor,
    /// Aplica al escritorio virtual seleccionado en el monitor activo.
    VirtualDesktop,
    /// Aplica como algoritmo por defecto global del sistema.
    GlobalDefault,
}

/// Estado persistente de la vista unificada de composición.
pub struct CompositionTabState {
    pub selected_output: String,
    pub selected_desktop: String,
    pub scope_mode: LayoutScopeMode,
    /// Algoritmo actualmente en borrador (en espera de ser asignado o previsualizado).
    pub staging_layout: String,
    /// Mensaje temporal de retroalimentación tras presionar el botón de asignación.
    pub feedback_msg: Option<(String, std::time::Instant)>,
}

impl Default for CompositionTabState {
    fn default() -> Self {
        Self {
            selected_output: String::new(),
            selected_desktop: String::new(),
            scope_mode: LayoutScopeMode::SelectedMonitor,
            staging_layout: "raven".to_string(),
            feedback_msg: None,
        }
    }
}

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

/// Retorna el nombre legible del algoritmo para mostrar al usuario.
fn get_readable_algo_name(algo_id: &str) -> &'static str {
    match algo_id {
        "raven" => "Raven (Base BSP)",
        "tall" => "Clásico (Tall)",
        "monocle" => "Monóculo",
        "strict_dwindle" => "Flujo Avanzado (Dwindle)",
        "inverted_strict_dwindle" => "Flujo Invertido",
        "divisor" => "Divisor (Columnas)",
        "floating" | "none" => "Flotante (Libre)",
        _ => "Script Lua",
    }
}

/// Renderiza la vista unificada de composición, topología de pantallas y algoritmos.
pub fn show(
    config: &mut RavenConfig,
    ui: &mut egui::Ui,
    accent: egui::Color32,
    palette: &KdePalette,
    service_mgr: &mut ServiceManager,
    state: &mut CompositionTabState,
) {
    let displays = service_mgr.get_displays();
    let virtual_desktops = service_mgr.get_virtual_desktops();

    if state.selected_output.is_empty() {
        if let Some(first) = displays.first() {
            state.selected_output = first.name.clone();
        }
    }

    if state.selected_desktop.is_empty() || !virtual_desktops.iter().any(|d| d.id == state.selected_desktop) {
        if let Some(first) = virtual_desktops.first() {
            state.selected_desktop = first.id.clone();
        }
    }

    // ── Encabezado Principal ──
    ui.heading(egui::RichText::new("🎨 Composición y Topología de Pantallas").strong().size(18.0).color(accent));
    ui.label(egui::RichText::new("Gestiona de forma unificada tus monitores Wayland, algoritmos por pantalla/escritorio y geometría PiP.").weak());
    ui.add_space(8.0);

    // ── Tarjeta Superior: Canvas Vectorial de Pantallas Wayland ──
    egui::Frame::new()
        .fill(palette.window_bg)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(10))
        .stroke(egui::Stroke::new(1.0, palette.button_bg))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("🖥️ Disposición de Pantallas (Wayland)").strong().size(13.5));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.selectable_label(state.scope_mode == LayoutScopeMode::GlobalDefault, "🌐 Global").on_hover_text("Configura el algoritmo por defecto para todo el sistema").clicked() {
                        state.scope_mode = LayoutScopeMode::GlobalDefault;
                    }
                    if ui.selectable_label(state.scope_mode == LayoutScopeMode::VirtualDesktop, "🗂️ Por Escritorio").on_hover_text("Configura el algoritmo para un escritorio específico en el monitor").clicked() {
                        state.scope_mode = LayoutScopeMode::VirtualDesktop;
                    }
                    if ui.selectable_label(state.scope_mode == LayoutScopeMode::SelectedMonitor, "🖥️ Por Monitor").on_hover_text("Configura el algoritmo exclusivo para el monitor seleccionado").clicked() {
                        state.scope_mode = LayoutScopeMode::SelectedMonitor;
                    }
                    ui.label(egui::RichText::new("Ámbito de Asignación:").weak().size(12.0));
                });
            });

            ui.add_space(6.0);
            draw_monitor_canvas(ui, &displays, &mut state.selected_output, config, palette, accent);

            // Barra inferior informativa del monitor y escritorio activo
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let mon_info = displays.iter().find(|d| d.name == state.selected_output);
                if let Some(m) = mon_info {
                    ui.label(egui::RichText::new(format!("Pantalla activa: {} ({}x{} @ {:.2}x)", m.name, m.rect.width, m.rect.height, m.scale)).strong().size(12.0).color(accent));
                } else {
                    ui.label(egui::RichText::new(format!("Pantalla activa: {}", state.selected_output)).strong().size(12.0).color(accent));
                }

                if state.scope_mode == LayoutScopeMode::VirtualDesktop {
                    ui.separator();
                    ui.label(egui::RichText::new("Escritorio:").weak());
                    let current_name = virtual_desktops
                        .iter()
                        .find(|d| d.id == state.selected_desktop)
                        .map(|d| d.name.as_str())
                        .unwrap_or("Escritorio");

                    egui::ComboBox::from_id_salt("combo_virtual_desktops")
                        .selected_text(current_name)
                        .show_ui(ui, |ui| {
                            for d in &virtual_desktops {
                                ui.selectable_value(&mut state.selected_desktop, d.id.clone(), &d.name);
                            }
                        });
                }
            });
        });

    ui.add_space(8.0);

    // ── Determinar qué clave de layout representa el ámbito seleccionado ──
    let current_target_key = match state.scope_mode {
        LayoutScopeMode::SelectedMonitor => Some(state.selected_output.clone()),
        LayoutScopeMode::VirtualDesktop => Some(format!("{}||{}", state.selected_output, state.selected_desktop)),
        LayoutScopeMode::GlobalDefault => None,
    };

    let target_display_name = match state.scope_mode {
        LayoutScopeMode::SelectedMonitor => format!("Pantalla '{}'", state.selected_output),
        LayoutScopeMode::VirtualDesktop => {
            let desk_name = virtual_desktops
                .iter()
                .find(|d| d.id == state.selected_desktop)
                .map(|d| d.name.as_str())
                .unwrap_or("Escritorio");
            format!("{} en {}", desk_name, state.selected_output)
        }
        LayoutScopeMode::GlobalDefault => "Todo el Sistema (Global)".to_string(),
    };

    // ── Columnas de Edición y Previsualización ──
    ui.columns(2, |cols| {
        // Columna Izquierda: Selección y Botón de Asignación Explícito
        cols[0].vertical(|ui| {
            ui.horizontal(|ui| {
                ui.heading(egui::RichText::new("1. Selecciona el Algoritmo").strong().size(14.0));
            });
            ui.add_space(4.0);

            // Función auxiliar para sincronizar el borrador con el ámbito seleccionado de inmediato
            let apply_staging_to_scope = |cfg: &mut RavenConfig, layout: &str, scope: &LayoutScopeMode, target_key: &Option<String>, sel_out: &str| {
                match scope {
                    LayoutScopeMode::GlobalDefault => {
                        cfg.layout_type = layout.to_string();
                        cfg.workspace_layouts.clear();
                    }
                    LayoutScopeMode::SelectedMonitor => {
                        let prefix = format!("{}||", sel_out);
                        cfg.workspace_layouts.retain(|k, _| !k.starts_with(&prefix));
                        if let Some(key) = target_key {
                            cfg.workspace_layouts.insert(key.clone(), layout.to_string());
                        }
                    }
                    LayoutScopeMode::VirtualDesktop => {
                        if let Some(key) = target_key {
                            cfg.workspace_layouts.insert(key.clone(), layout.to_string());
                        }
                    }
                }
            };

            // Presets Rápidos
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new("Presets:").weak().size(11.5));
                for p in PRESETS {
                    if ui.button(p.display).clicked() {
                        state.staging_layout = p.layout_type.to_string();
                        config.default_gaps = p.gaps;
                        config.master_ratio = p.ratio;
                        apply_staging_to_scope(
                            config,
                            &state.staging_layout,
                            &state.scope_mode,
                            &current_target_key,
                            &state.selected_output,
                        );
                    }
                }
            });

            ui.add_space(6.0);

            // Catálogo Nativo de Algoritmos (Radio buttons sincronizan el borrador y el ámbito activo)
            for p in PRESETS {
                if ui
                    .radio_value(
                        &mut state.staging_layout,
                        p.layout_type.to_string(),
                        egui::RichText::new(p.display).strong(),
                    )
                    .changed()
                {
                    apply_staging_to_scope(
                        config,
                        &state.staging_layout,
                        &state.scope_mode,
                        &current_target_key,
                        &state.selected_output,
                    );
                }
                ui.label(egui::RichText::new(p.desc).weak().size(11.0));
                ui.add_space(2.0);
            }

            // Opción Modo Flotante
            if ui
                .radio_value(
                    &mut state.staging_layout,
                    "floating".to_string(),
                    egui::RichText::new("🪟 Flotante (Libre)")
                        .strong()
                        .color(egui::Color32::from_rgb(235, 90, 90)),
                )
                .changed()
            {
                apply_staging_to_scope(
                    config,
                    &state.staging_layout,
                    &state.scope_mode,
                    &current_target_key,
                    &state.selected_output,
                );
            }
            ui.label(egui::RichText::new("Desactiva el mosaico en este espacio permitiendo ventanas libres de KWin.").weak().size(11.0));
            ui.add_space(4.0);

            // Scripts Lua
            let custom_layouts = get_custom_layouts();
            if !custom_layouts.is_empty() {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Scripts Lua:").strong().size(12.0));
                for layout_name in custom_layouts {
                    if ui
                        .radio_value(
                            &mut state.staging_layout,
                            layout_name.clone(),
                            egui::RichText::new(&layout_name).strong(),
                        )
                        .changed()
                    {
                        apply_staging_to_scope(
                            config,
                            &state.staging_layout,
                            &state.scope_mode,
                            &current_target_key,
                            &state.selected_output,
                        );
                    }
                }
            }

            ui.add_space(8.0);

            // ── BOTÓN DE ASIGNACIÓN EXPLÍCITA Y CONFIRMACIÓN ──
            let btn_label = format!("📥 Asignar en: {}", target_display_name);
            let btn_resp = ui.add(
                egui::Button::new(egui::RichText::new(btn_label).strong().size(13.0).color(egui::Color32::WHITE))
                    .fill(accent)
                    .corner_radius(8.0)
                    .min_size(egui::vec2(ui.available_width(), 36.0)),
            );

            if btn_resp.clicked() {
                apply_staging_to_scope(
                    config,
                    &state.staging_layout,
                    &state.scope_mode,
                    &current_target_key,
                    &state.selected_output,
                );
                state.feedback_msg = Some((
                    format!("✅ Asignado '{}' a {}", get_readable_algo_name(&state.staging_layout), target_display_name),
                    std::time::Instant::now(),
                ));
            }

            // Mensaje de retroalimentación temporal
            if let Some((ref msg, timestamp)) = state.feedback_msg {
                if timestamp.elapsed().as_secs() < 3 {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(msg).color(accent).size(11.5).strong());
                }
            }

            // Si es un ámbito local y tiene asignación personalizada, botón de restablecer a global
            if let Some(ref key) = current_target_key {
                if config.workspace_layouts.contains_key(key) {
                    ui.add_space(4.0);
                    if ui.button("↩ Quitar asignación (Heredar General)").clicked() {
                        config.workspace_layouts.remove(key);
                        state.feedback_msg = Some((
                            format!("🗑 Regla eliminada para {}. Ahora hereda la configuración general.", target_display_name),
                            std::time::Instant::now(),
                        ));
                    }
                }
            }
        });

        // Columna Derecha: Previsualización Fractal, Anclaje PiP y Parámetros
        cols[1].vertical(|ui| {
            ui.heading(egui::RichText::new("2. Previsualización y PiP").strong().size(14.0));
            ui.add_space(4.0);

            // Previsualización Gráfica Interactiva basada en el algoritmo en borrador
            draw_layout_preview(
                ui,
                &state.staging_layout,
                config.master_ratio,
                config.default_gaps,
                &mut config.pip_position,
                config.pip_size_ratio,
                palette,
            );

            ui.add_space(6.0);
            ui.label(egui::RichText::new(format!("• Anclaje PiP Activo: {}", config.pip_position)).strong().size(11.5));

            ui.add_space(8.0);
            ui.group(|ui| {
                ui.set_width(ui.available_width());
                ui.vertical(|ui| {
                    ui.heading(egui::RichText::new("Geometría y Márgenes").strong().size(12.5));
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label("Gaps:");
                        ui.add(egui::Slider::new(&mut config.default_gaps, 0..=30).suffix(" px"));
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label("Ratio maestro:");
                        ui.add(egui::Slider::new(&mut config.master_ratio, 0.2..=0.8).fixed_decimals(2));
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label("nmaster:");
                        ui.add(egui::Slider::new(&mut config.nmaster, 1..=4));
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label("Escala PiP:");
                        ui.add(egui::Slider::new(&mut config.pip_size_ratio, 0.10..=0.50).fixed_decimals(2));
                    });
                });
            });
        });
    });

    ui.add_space(10.0);
    ui.separator();
    ui.add_space(8.0);

    // ── Tarjeta Inferior: Lista de Asignaciones Activas ("¿Qué está asignado a qué?") ──
    egui::Frame::new()
        .fill(palette.window_bg)
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(10))
        .stroke(egui::Stroke::new(1.0, palette.button_bg))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("📋 Resumen de Asignaciones Activas").strong().size(13.5));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("Global por defecto: {}", get_readable_algo_name(&config.layout_type))).strong().color(accent).size(12.0));
                });
            });
            ui.add_space(4.0);

            if config.workspace_layouts.is_empty() {
                ui.label(egui::RichText::new("No hay asignaciones personalizadas. Todas las pantallas y escritorios usan el algoritmo global.").weak().italics().size(11.5));
            } else {
                let mut to_remove_key = None;

                egui::Grid::new("active_rules_grid")
                    .num_columns(3)
                    .spacing([20.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Espacio / Pantalla").strong().size(12.0));
                        ui.label(egui::RichText::new("Algoritmo Asignado").strong().size(12.0));
                        ui.label(egui::RichText::new("Acción").strong().size(12.0));
                        ui.end_row();

                        for (key, algo) in &config.workspace_layouts {
                            // Formatear nombre amigable para la clave (si contiene UUID, buscar nombre de escritorio)
                            let friendly_label = if key.contains("||") {
                                let mut parts = key.split("||");
                                let out = parts.next().unwrap_or("Pantalla");
                                let uuid = parts.next().unwrap_or("");
                                let desk_name = virtual_desktops
                                    .iter()
                                    .find(|d| d.id == uuid)
                                    .map(|d| d.name.as_str())
                                    .unwrap_or(uuid);
                                format!("🗂️ {} en {}", desk_name, out)
                            } else {
                                format!("🖥️ Pantalla '{}'", key)
                            };

                            ui.label(egui::RichText::new(friendly_label).size(11.5));
                            ui.label(egui::RichText::new(get_readable_algo_name(algo)).color(accent).strong().size(11.5));

                            if ui.button("🗑 Quitar").clicked() {
                                to_remove_key = Some(key.clone());
                            }
                            ui.end_row();
                        }
                    });

                if let Some(key) = to_remove_key {
                    config.workspace_layouts.remove(&key);
                }
            }
        });
}
