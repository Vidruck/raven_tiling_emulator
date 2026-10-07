//! # Servicio de Flotación Dinámica y Quick Peek (`floating_service`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)
//! **Licencia:** GPL-3.0
//!
//! Encapsula el cálculo de geometrías flotantes dinámicas seguras (Quick Peek)
//! y la resolución espacial en base a las áreas de trabajo o nodos de salida (topology).

use crate::domain::geometry::{Rect, Topology};

/// Calcula las dimensiones y coordenadas seguras centradas para una ventana en modo Quick Peek (flotante dinámico).
///
/// Toma el 60% del área útil del espacio de trabajo con restricciones mínimas (640x480) y márgenes perimetrales.
pub fn calculate_dynamic_floating_rect(
    workspace_rect: Option<Rect>,
    workspace_id: &str,
    topology: &Topology,
) -> Rect {
    let ws_rect = workspace_rect
        .or_else(|| {
            let out_name = workspace_id.split("||").next().unwrap_or("");
            topology
                .output_nodes
                .iter()
                .find(|n| n.name == out_name)
                .map(|n| n.rect)
        })
        .filter(|r| r.width > 0 && r.height > 0)
        .unwrap_or_else(|| Rect::new(0, 0, 1920, 1080));

    let float_w = ((ws_rect.width as f32 * 0.60).round() as i32)
        .max(640)
        .min(ws_rect.width - 40);
    let float_h = ((ws_rect.height as f32 * 0.60).round() as i32)
        .max(480)
        .min(ws_rect.height - 40);
    let float_x = ws_rect.x + (ws_rect.width - float_w) / 2;
    let float_y = ws_rect.y + (ws_rect.height - float_h) / 2;

    Rect::new(float_x, float_y, float_w, float_h)
}
