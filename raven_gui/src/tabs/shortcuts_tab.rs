//! # Pestaña de Guía de Atajos de Teclado (`shortcuts_tab.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Presenta el catálogo visual interactivo de atajos de teclado globales de KWin
//! (foco direccional, swaps, migración de pantallas/escritorios, Quick Peek y márgenes).

use eframe::egui;

/// Renderiza la vista de referencia interactiva de atajos de teclado globales.
pub fn show(ui: &mut egui::Ui, accent: egui::Color32) {
    ui.heading(egui::RichText::new("⌨️ Guía Completa de Atajos de Teclado Globales").strong().size(18.0).color(accent));
    ui.label(egui::RichText::new("Atajos nativos integrados en KDE Plasma (kglobalshortcutsrc) para controlar a Raven en tiempo real.").weak());
    ui.add_space(10.0);

    // Destacado: Quick Peek / Ventana Flotante Dinámica
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("✨ Novedad v3.2: Quick Peek (Pila Flotante Dinámica)").strong().size(13.5).color(accent));
            });
            ui.add_space(2.0);
            ui.label(egui::RichText::new("Pulsa Meta + Shift + F sobre cualquier ventana para elevarla como flotante centrada sin alterar el mosaico. Al terminar, vuelve a pulsar el atajo para regresarla al mosaico como nueva.").weak().size(11.5));
        });
    });

    ui.add_space(10.0);

    let shortcuts_data = [
        ("Mosaico y Flotación", vec![
            ("Meta + Space", "Alternar Mosaico Global (Activar / Desactivar)"),
            ("Meta + Shift + F", "Alternar Ventana Flotante Dinámica (Quick Peek)"),
            ("Meta + Shift + L", "Ciclar algoritmo de Layout"),
        ]),
        ("Navegación y Foco", vec![
            ("Meta + J / K", "Enfocar ventana Siguiente / Anterior"),
            ("Meta + Flechas", "Foco direccional (Izquierda / Derecha / Arriba / Abajo)"),
            ("Meta + Shift + J / K", "Intercambiar posición de ventana (Swap Siguiente / Anterior)"),
        ]),
        ("Dimensionamiento y Geometría", vec![
            ("Meta + Alt + Derecha / Izq", "Aumentar / Reducir ANCHO de ventana (2D Resize)"),
            ("Meta + Alt + Abajo / Arriba", "Aumentar / Reducir ALTO de ventana (2D Resize)"),
            ("Meta + H / L", "Expandir / Contraer proporción del área Master"),
            ("Meta + ] / [", "Incrementar / Decrementar nº de ventanas Master"),
            ("Meta + = / -", "Incrementar / Decrementar Márgenes (Gaps)"),
        ]),
        ("Migración Multimonitor y Escritorios", vec![
            ("Meta + Shift + M / N", "Migrar ventana activa al monitor Siguiente / Anterior"),
            ("Meta + Shift + Right / Left", "Migrar ventana activa al escritorio Siguiente / Anterior"),
        ]),
    ];

    for (category, items) in shortcuts_data {
        ui.heading(egui::RichText::new(category).strong().size(13.5));
        ui.add_space(4.0);

        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.vertical(|ui| {
                for (idx, (key, desc)) in items.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let is_highlight = *key == "Meta + Shift + F";
                        let key_color = if is_highlight { accent } else { accent };
                        ui.label(egui::RichText::new(*key).strong().size(12.5).color(key_color));
                        ui.label(egui::RichText::new(format!("→ {}", desc)).size(12.0));
                    });
                    if idx < items.len() - 1 {
                        ui.add_space(4.0);
                    }
                }
            });
        });

        ui.add_space(8.0);
    }
}
