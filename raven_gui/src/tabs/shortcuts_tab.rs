// ============================================================================
// RAVEN GUI — PESTAÑA DE GUÍA DE ATAJOS GLOBALES (tabs/shortcuts_tab.rs)
// ============================================================================

use eframe::egui;

pub fn show(ui: &mut egui::Ui, accent: egui::Color32) {
    ui.heading(egui::RichText::new("⌨️ Guía Completa de Atajos de Teclado Globales").strong().size(18.0).color(accent));
    ui.label(egui::RichText::new("Atajos nativos integrados en KDE Plasma (kglobalshortcutsrc).").weak());
    ui.add_space(12.0);

    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            for (key, desc) in [
                ("Meta + Space",                "Alternar Mosaico Global (On / Off)"),
                ("Meta + J / K",                "Enfocar ventana Siguiente / Anterior"),
                ("Meta + Flechas",              "Foco direccional (Izquierda / Derecha / Arriba / Abajo)"),
                ("Meta + Shift + J / K",        "Intercambiar posición de ventana Siguiente / Anterior"),
                ("Meta + Alt + Derecha / Izquierda", "Aumentar / Reducir ANCHO de ventana (2D Resize)"),
                ("Meta + Alt + Abajo / Arriba",      "Aumentar / Reducir ALTO de ventana (2D Resize)"),
                ("Meta + H / L",                "Expandir / Contraer área Master"),
                ("Meta + ] / [",                "Incrementar / Decrementar nº de másters (nmaster)"),
                ("Meta + = / -",                "Incrementar / Decrementar Márgenes (Gaps)"),
                ("Meta + Shift + L",            "Ciclar estrategia de Layout"),
                ("Meta + Shift + M / N",        "Migrar ventana activa al monitor Siguiente / Anterior"),
                ("Meta + Shift + Right / Left", "Migrar ventana activa al escritorio Siguiente / Anterior"),
            ] {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(key).strong().size(13.0).color(accent));
                    ui.label(format!("→ {}", desc));
                });
                ui.add_space(4.0);
            }
        });
    });
}
