//! # Canvas Gráfico de Monitores y Topología Wayland (`monitor_canvas.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Renderiza un mapa espacial interactivo de las pantallas conectadas en Wayland/KWin,
//! permitiendo seleccionar pantallas y asignar algoritmos de mosaico o modo flotante.

use eframe::egui;
use raven_core::config::RavenConfig;
use raven_core::geometry::OutputNode;
use crate::kde_theme::KdePalette;

/// Dibuja la representación gráfica de las pantallas conectadas y su disposición relativa.
pub fn draw_monitor_canvas(
    ui: &mut egui::Ui,
    outputs: &[OutputNode],
    selected_output: &mut String,
    config: &mut RavenConfig,
    palette: &KdePalette,
    accent: egui::Color32,
) {
    if outputs.is_empty() {
        ui.label(egui::RichText::new("No se detectaron salidas de pantalla activas.").weak().italics());
        return;
    }

    let desired_size = egui::vec2(ui.available_width().max(300.0), 180.0);
    let (canvas_rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(canvas_rect);

    // Fondo del canvas con estilo KDE Breeze Glass
    painter.rect_filled(canvas_rect, 10.0, palette.window_bg);
    painter.rect_stroke(
        canvas_rect,
        10.0,
        egui::Stroke::new(1.0, palette.button_bg),
        egui::StrokeKind::Inside,
    );

    // Calcular la caja envolvente (bounding box) de todas las pantallas
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;

    for node in outputs {
        min_x = min_x.min(node.rect.x);
        min_y = min_y.min(node.rect.y);
        max_x = max_x.max(node.rect.x + node.rect.width);
        max_y = max_y.max(node.rect.y + node.rect.height);
    }

    let total_w = (max_x - min_x).max(1) as f32;
    let total_h = (max_y - min_y).max(1) as f32;

    let margin = 20.0;
    let avail_w = canvas_rect.width() - (margin * 2.0);
    let avail_h = canvas_rect.height() - (margin * 2.0);

    let scale_factor = (avail_w / total_w).min(avail_h / total_h);

    let offset_x = canvas_rect.min.x + margin + (avail_w - (total_w * scale_factor)) / 2.0;
    let offset_y = canvas_rect.min.y + margin + (avail_h - (total_h * scale_factor)) / 2.0;

    let clicked_pos = if response.clicked() {
        ui.input(|i| i.pointer.interact_pos())
    } else {
        None
    };

    for node in outputs {
        let screen_x = offset_x + ((node.rect.x - min_x) as f32 * scale_factor);
        let screen_y = offset_y + ((node.rect.y - min_y) as f32 * scale_factor);
        let screen_w = (node.rect.width as f32 * scale_factor).max(40.0);
        let screen_h = (node.rect.height as f32 * scale_factor).max(30.0);

        let monitor_rect = egui::Rect::from_min_size(
            egui::pos2(screen_x, screen_y),
            egui::vec2(screen_w, screen_h),
        );

        let is_selected = selected_output == &node.name;

        // Comprobar click
        if let Some(pos) = clicked_pos {
            if monitor_rect.contains(pos) {
                *selected_output = node.name.clone();
            }
        }

        // Consultar algoritmo asignado a este monitor (por output o global)
        let assigned_layout = config
            .workspace_layouts
            .get(&node.name)
            .cloned()
            .unwrap_or_else(|| config.layout_type.clone());

        let fill_color = if is_selected {
            egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 80)
        } else {
            palette.button_bg
        };

        let stroke_color = if is_selected {
            accent
        } else {
            palette.button_fg
        };

        let stroke_width = if is_selected { 2.0 } else { 1.0 };

        // Dibujar pantalla
        painter.rect_filled(monitor_rect, 6.0, fill_color);
        painter.rect_stroke(
            monitor_rect,
            6.0,
            egui::Stroke::new(stroke_width, stroke_color),
            egui::StrokeKind::Inside,
        );

        // Texto informativo en el monitor
        let center = monitor_rect.center();
        let name_color = if is_selected {
            palette.window_fg
        } else {
            palette.button_fg
        };

        painter.text(
            center - egui::vec2(0.0, 10.0),
            egui::Align2::CENTER_CENTER,
            &node.name,
            egui::FontId::proportional(12.0),
            name_color,
        );

        let res_label = format!("{}x{}", node.rect.width, node.rect.height);
        painter.text(
            center + egui::vec2(0.0, 4.0),
            egui::Align2::CENTER_CENTER,
            res_label,
            egui::FontId::monospace(9.5),
            palette.button_fg,
        );

        let algo_badge = match assigned_layout.as_str() {
            "tall" => "Tall",
            "monocle" => "Monocle",
            "strict_dwindle" => "Dwindle",
            "inverted_strict_dwindle" => "Inv-Dwindle",
            "divisor" => "Divisor",
            "floating" | "none" => "Flotante",
            _ => "Raven",
        };

        let badge_color = if assigned_layout == "floating" || assigned_layout == "none" {
            egui::Color32::from_rgb(230, 80, 80)
        } else {
            accent
        };

        painter.text(
            center + egui::vec2(0.0, 16.0),
            egui::Align2::CENTER_CENTER,
            format!("[{}]", algo_badge),
            egui::FontId::proportional(10.0),
            badge_color,
        );
    }
}
