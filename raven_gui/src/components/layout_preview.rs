// ============================================================================
// RAVEN GUI — COMPONENTE CANVAS PREVIEW 2D (components/layout_preview.rs)
// ============================================================================
// Proporciona la representación gráfica vectorial interactiva de la distribución
// de las ventanas en pantalla de acuerdo al algoritmo de mosaico seleccionado.

use eframe::egui;
use crate::kde_theme::KdePalette;

/// Dibuja en un canvas de `egui` la previsualización gráfica 2D del layout.
pub fn draw_layout_preview(ui: &mut egui::Ui, layout_type: &str, ratio: f32, gaps: i32, pip_position: &mut String, pip_size_ratio: f32, palette: &KdePalette) {
    let desired_size = egui::vec2(ui.available_width().min(420.0), 170.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

    let painter = ui.painter_at(rect);
    let bg = palette.window_bg;
    painter.rect_filled(rect, 12.0, bg);

    let gap_f = gaps as f32 * 0.4;
    let w = rect.width();
    let h = rect.height();

    let center_color = palette.selection_bg;
    let side_color   = palette.button_bg;
    let pip_color    = egui::Color32::from_rgb(247, 37, 133); // Magenta vibrante para PiP

    match layout_type {
        "raven" => {
            // Algoritmo Raven Base: Master Central (50%) con Pilas Laterales Completas Izquierda (V2, V3) y Derecha (V4, V5)
            let master_w = w * ratio;
            let side_w = (w - master_w) / 2.0;
            let half_h = h / 2.0;

            // Pila Izquierda: Ventana 2 (Arriba) y Ventana 3 (Abajo)
            let left_top = egui::Rect::from_min_size(
                rect.min + egui::vec2(gap_f, gap_f),
                egui::vec2(side_w - gap_f * 2.0, half_h - gap_f * 2.0),
            );
            painter.rect_filled(left_top, 6.0, side_color);
            painter.text(left_top.center(), egui::Align2::CENTER_CENTER, "2", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            let left_bot = egui::Rect::from_min_size(
                rect.min + egui::vec2(gap_f, half_h + gap_f),
                egui::vec2(side_w - gap_f * 2.0, half_h - gap_f * 2.0),
            );
            painter.rect_filled(left_bot, 6.0, side_color);
            painter.text(left_bot.center(), egui::Align2::CENTER_CENTER, "3", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            // Panel Central Maestro (Ventana 1 - Master)
            let master_rect = egui::Rect::from_min_size(
                rect.min + egui::vec2(side_w + gap_f, gap_f),
                egui::vec2(master_w - gap_f * 2.0, h - gap_f * 2.0),
            );
            painter.rect_filled(master_rect, 6.0, center_color);
            painter.text(master_rect.center(), egui::Align2::CENTER_CENTER, "1 (Master)", egui::FontId::monospace(11.0), egui::Color32::WHITE);

            // Pila Derecha: Ventana 4 (Arriba) y Ventana 5 (Abajo)
            let right_top = egui::Rect::from_min_size(
                rect.min + egui::vec2(side_w + master_w + gap_f, gap_f),
                egui::vec2(side_w - gap_f * 2.0, half_h - gap_f * 2.0),
            );
            painter.rect_filled(right_top, 6.0, side_color);
            painter.text(right_top.center(), egui::Align2::CENTER_CENTER, "4", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            let right_bot = egui::Rect::from_min_size(
                rect.min + egui::vec2(side_w + master_w + gap_f, half_h + gap_f),
                egui::vec2(side_w - gap_f * 2.0, half_h - gap_f * 2.0),
            );
            painter.rect_filled(right_bot, 6.0, side_color);
            painter.text(right_bot.center(), egui::Align2::CENTER_CENTER, "5", egui::FontId::monospace(10.0), egui::Color32::WHITE);
        }
        "tall" => {
            let master_w = w * ratio;
            let stack_w = w - master_w;
            let stack_h = h / 3.0;

            let master = egui::Rect::from_min_size(
                rect.min + egui::vec2(gap_f, gap_f),
                egui::vec2(master_w - gap_f * 2.0, h - gap_f * 2.0),
            );
            painter.rect_filled(master, 6.0, center_color);
            painter.text(master.center(), egui::Align2::CENTER_CENTER, "Master", egui::FontId::monospace(11.0), egui::Color32::WHITE);

            for i in 0..3 {
                let r = egui::Rect::from_min_size(
                    rect.min + egui::vec2(master_w + gap_f, i as f32 * stack_h + gap_f),
                    egui::vec2(stack_w - gap_f * 2.0, stack_h - gap_f * 2.0),
                );
                painter.rect_filled(r, 6.0, side_color);
                painter.text(r.center(), egui::Align2::CENTER_CENTER, &(i + 2).to_string(), egui::FontId::monospace(10.0), egui::Color32::WHITE);
            }
        }
        "monocle" => {
            let r = egui::Rect::from_min_size(
                rect.min + egui::vec2(gap_f, gap_f),
                egui::vec2(w - gap_f * 2.0, h - gap_f * 2.0),
            );
            painter.rect_filled(r, 6.0, center_color);
            painter.text(r.center(), egui::Align2::CENTER_CENTER, "Monóculo (Maximizado)", egui::FontId::monospace(12.0), egui::Color32::WHITE);
        }
        "strict_dwindle" => {
            let curr_x = rect.min.x;
            let curr_y = rect.min.y;

            let w1_w = w * ratio;
            let r1 = egui::Rect::from_min_size(
                egui::pos2(curr_x + gap_f, curr_y + gap_f),
                egui::vec2(w1_w - gap_f * 2.0, h - gap_f * 2.0),
            );
            painter.rect_filled(r1, 6.0, center_color);
            painter.text(r1.center(), egui::Align2::CENTER_CENTER, "1", egui::FontId::monospace(11.0), egui::Color32::WHITE);

            let rem_w = w - w1_w;
            let w2_h = h * ratio;
            let r2 = egui::Rect::from_min_size(
                egui::pos2(curr_x + w1_w + gap_f, curr_y + gap_f),
                egui::vec2(rem_w - gap_f * 2.0, w2_h - gap_f * 2.0),
            );
            painter.rect_filled(r2, 6.0, side_color);
            painter.text(r2.center(), egui::Align2::CENTER_CENTER, "2", egui::FontId::monospace(11.0), egui::Color32::WHITE);

            let rem_h = h - w2_h;
            let w3_w = rem_w * ratio;
            let r3 = egui::Rect::from_min_size(
                egui::pos2(curr_x + w1_w + gap_f, curr_y + w2_h + gap_f),
                egui::vec2(w3_w - gap_f * 2.0, rem_h - gap_f * 2.0),
            );
            painter.rect_filled(r3, 6.0, side_color);
            painter.text(r3.center(), egui::Align2::CENTER_CENTER, "3", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            let rem_w2 = rem_w - w3_w;
            let r4 = egui::Rect::from_min_size(
                egui::pos2(curr_x + w1_w + w3_w + gap_f, curr_y + w2_h + gap_f),
                egui::vec2(rem_w2 - gap_f * 2.0, rem_h - gap_f * 2.0),
            );
            painter.rect_filled(r4, 6.0, side_color);
            painter.text(r4.center(), egui::Align2::CENTER_CENTER, "4", egui::FontId::monospace(9.0), egui::Color32::WHITE);
        }
        "inverted_strict_dwindle" => {
            let curr_x = rect.min.x;
            let curr_y = rect.min.y;

            let w1_w = w * ratio;
            let rem_w = w - w1_w;
            let r1 = egui::Rect::from_min_size(
                egui::pos2(curr_x + rem_w + gap_f, curr_y + gap_f),
                egui::vec2(w1_w - gap_f * 2.0, h - gap_f * 2.0),
            );
            painter.rect_filled(r1, 6.0, center_color);
            painter.text(r1.center(), egui::Align2::CENTER_CENTER, "1 (Master Invertido)", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            let w2_h = h * ratio;
            let r2 = egui::Rect::from_min_size(
                egui::pos2(curr_x + gap_f, curr_y + gap_f),
                egui::vec2(rem_w - gap_f * 2.0, w2_h - gap_f * 2.0),
            );
            painter.rect_filled(r2, 6.0, side_color);
            painter.text(r2.center(), egui::Align2::CENTER_CENTER, "2", egui::FontId::monospace(11.0), egui::Color32::WHITE);

            let rem_h = h - w2_h;
            let w3_w = rem_w * ratio;
            let r3 = egui::Rect::from_min_size(
                egui::pos2(curr_x + gap_f, curr_y + w2_h + gap_f),
                egui::vec2(w3_w - gap_f * 2.0, rem_h - gap_f * 2.0),
            );
            painter.rect_filled(r3, 6.0, side_color);
            painter.text(r3.center(), egui::Align2::CENTER_CENTER, "3", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            let rem_w2 = rem_w - w3_w;
            let r4 = egui::Rect::from_min_size(
                egui::pos2(curr_x + w3_w + gap_f, curr_y + w2_h + gap_f),
                egui::vec2(rem_w2 - gap_f * 2.0, rem_h - gap_f * 2.0),
            );
            painter.rect_filled(r4, 6.0, side_color);
            painter.text(r4.center(), egui::Align2::CENTER_CENTER, "4", egui::FontId::monospace(9.0), egui::Color32::WHITE);
        }
        "divisor" => {
            let n = 4;
            let col_w = w / n as f32;
            for i in 0..n {
                let r = egui::Rect::from_min_size(
                    rect.min + egui::vec2(i as f32 * col_w + gap_f, gap_f),
                    egui::vec2(col_w - gap_f * 2.0, h - gap_f * 2.0),
                );
                painter.rect_filled(r, 6.0, if i == 0 { center_color } else { side_color });
                painter.text(r.center(), egui::Align2::CENTER_CENTER, &(i + 1).to_string(), egui::FontId::monospace(10.0), egui::Color32::WHITE);
            }
        }
        _ => {
            // Previsualización completa de 7 ventanas (Raven / Dwindle BSP)
            let bottom_h = h * 0.30;
            let main_h = h - bottom_h;
            let center_w = w * ratio;
            let sidebar_w = (w - center_w) / 2.0;

            // 1. Center
            let r1 = egui::Rect::from_min_size(
                rect.min + egui::vec2(sidebar_w + gap_f, gap_f),
                egui::vec2(center_w - gap_f * 2.0, main_h - gap_f * 2.0),
            );
            painter.rect_filled(r1, 6.0, center_color);
            painter.text(r1.center(), egui::Align2::CENTER_CENTER, "1", egui::FontId::monospace(14.0), egui::Color32::WHITE);

            // 2. Left 1 (Top)
            let r2 = egui::Rect::from_min_size(
                rect.min + egui::vec2(gap_f, gap_f),
                egui::vec2(sidebar_w - gap_f * 2.0, main_h / 2.0 - gap_f * 2.0),
            );
            painter.rect_filled(r2, 6.0, side_color);
            painter.text(r2.center(), egui::Align2::CENTER_CENTER, "2", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            // 3. Right 1 (Top)
            let r3 = egui::Rect::from_min_size(
                rect.min + egui::vec2(sidebar_w + center_w + gap_f, gap_f),
                egui::vec2(sidebar_w - gap_f * 2.0, main_h / 2.0 - gap_f * 2.0),
            );
            painter.rect_filled(r3, 6.0, side_color);
            painter.text(r3.center(), egui::Align2::CENTER_CENTER, "3", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            // 4. Bottom Left
            let r4 = egui::Rect::from_min_size(
                rect.min + egui::vec2(gap_f, main_h + gap_f),
                egui::vec2(w / 2.0 - gap_f * 2.0, bottom_h - gap_f * 2.0),
            );
            painter.rect_filled(r4, 6.0, side_color);
            painter.text(r4.center(), egui::Align2::CENTER_CENTER, "4", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            // 5. Bottom Right
            let r5 = egui::Rect::from_min_size(
                rect.min + egui::vec2(w / 2.0 + gap_f, main_h + gap_f),
                egui::vec2(w / 2.0 - gap_f * 2.0, bottom_h - gap_f * 2.0),
            );
            painter.rect_filled(r5, 6.0, side_color);
            painter.text(r5.center(), egui::Align2::CENTER_CENTER, "5", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            // 6. Left 2 (Bottom)
            let r6 = egui::Rect::from_min_size(
                rect.min + egui::vec2(gap_f, main_h / 2.0 + gap_f),
                egui::vec2(sidebar_w - gap_f * 2.0, main_h / 2.0 - gap_f * 2.0),
            );
            painter.rect_filled(r6, 6.0, side_color);
            painter.text(r6.center(), egui::Align2::CENTER_CENTER, "6", egui::FontId::monospace(10.0), egui::Color32::WHITE);

            // 7. Right 2 (Bottom)
            let r7 = egui::Rect::from_min_size(
                rect.min + egui::vec2(sidebar_w + center_w + gap_f, main_h / 2.0 + gap_f),
                egui::vec2(sidebar_w - gap_f * 2.0, main_h / 2.0 - gap_f * 2.0),
            );
            painter.rect_filled(r7, 6.0, side_color);
            painter.text(r7.center(), egui::Align2::CENTER_CENTER, "7", egui::FontId::monospace(10.0), egui::Color32::WHITE);
        }
    }

    // ── Definición de Zonas Calientes / Puntos de Anclaje PiP (Estilo Bordes KDE) ──
    let spot_radius = 9.0;
    let margin = 14.0;

    let spots = [
        ("top-left", egui::pos2(rect.min.x + margin, rect.min.y + margin)),
        ("top-right", egui::pos2(rect.max.x - margin, rect.min.y + margin)),
        ("bottom-left", egui::pos2(rect.min.x + margin, rect.max.y - margin)),
        ("bottom-right", egui::pos2(rect.max.x - margin, rect.max.y - margin)),
        ("top", egui::pos2(rect.center().x, rect.min.y + margin)),
        ("bottom", egui::pos2(rect.center().x, rect.max.y - margin)),
        ("left", egui::pos2(rect.min.x + margin, rect.center().y)),
        ("right", egui::pos2(rect.max.x - margin, rect.center().y)),
    ];

    if response.clicked() {
        if let Some(mouse_pos) = response.interact_pointer_pos() {
            for (key, pos) in &spots {
                if mouse_pos.distance(*pos) <= spot_radius * 2.0 {
                    *pip_position = key.to_string();
                    break;
                }
            }
        }
    }

    // Dibujar los 8 Puntos Calientes (Hot Spots)
    for (key, pos) in &spots {
        let is_active = pip_position == key;
        let color = if is_active {
            pip_color
        } else {
            egui::Color32::from_white_alpha(140)
        };
        painter.circle_filled(*pos, if is_active { spot_radius } else { spot_radius * 0.7 }, color);
        painter.circle_stroke(*pos, spot_radius, egui::Stroke::new(1.5, egui::Color32::WHITE));
    }

    // Overlay de la Ventana PiP Flotante en la Esquina Seleccionada
    // Se escala proporcionalmente multiplicando el ratio por un multiplicador visual (1.2)
    // para que sea más notable en la maqueta pequeña.
    let pip_size = egui::vec2(w * pip_size_ratio * 1.2, h * pip_size_ratio * 1.2 * 1.5);
    let pip_pos = match pip_position.as_str() {
        "top-left" => rect.min + egui::vec2(gap_f + 16.0, gap_f + 16.0),
        "top-right" => egui::pos2(rect.max.x - pip_size.x - gap_f - 16.0, rect.min.y + gap_f + 16.0),
        "bottom-left" => egui::pos2(rect.min.x + gap_f + 16.0, rect.max.y - pip_size.y - gap_f - 16.0),
        "bottom-right" => rect.max - pip_size - egui::vec2(gap_f + 16.0, gap_f + 16.0),
        "top" => egui::pos2(rect.center().x - pip_size.x / 2.0, rect.min.y + gap_f + 16.0),
        "bottom" => egui::pos2(rect.center().x - pip_size.x / 2.0, rect.max.y - pip_size.y - gap_f - 16.0),
        "left" => egui::pos2(rect.min.x + gap_f + 16.0, rect.center().y - pip_size.y / 2.0),
        "right" => egui::pos2(rect.max.x - pip_size.x - gap_f - 16.0, rect.center().y - pip_size.y / 2.0),
        _ => rect.max - pip_size - egui::vec2(gap_f + 16.0, gap_f + 16.0), // fallback bottom-right
    };

    let pip_rect = egui::Rect::from_min_size(pip_pos, pip_size);
    painter.rect_filled(pip_rect, 6.0, pip_color);
    painter.rect_stroke(pip_rect, 6.0, egui::Stroke::new(1.0, egui::Color32::WHITE));
    painter.text(pip_rect.center(), egui::Align2::CENTER_CENTER, "📌 PiP", egui::FontId::monospace(9.5), egui::Color32::WHITE);
}
