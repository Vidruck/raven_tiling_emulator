//! # Pestaña de Información, Autoría y Licencia (`about_tab.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Presenta los créditos oficiales, enlaces al repositorio GitHub, información del desarrollador y términos de licencia GPL-3.0.

use eframe::egui;

/// Renderiza la vista de información general, autoría y licencia del proyecto.
pub fn show(ui: &mut egui::Ui, accent: egui::Color32) {
    ui.heading(egui::RichText::new("ℹ️ Acerca de Raven Tiling Emulator").strong().size(18.0).color(accent));
    ui.label(egui::RichText::new("Bienvenido al Centro de Configuracion del Motor nativo de mosaico dinámico para KDE Plasma 6.").weak());
    ui.add_space(12.0);

    //Tarjeta de Información
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.heading(egui::RichText::new("Sobre el Proyecto:").strong().size(14.0));
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Raven Tiling Emulator nacio para ofrecerte una experiencia de mosaico dinámico dentro del ecosistema KDE Plasma, para brindarte comodidad, fluidez y eficiencia sin abandonar el ecosistema.").weak());
        });
    });

    ui.add_space(12.0);

    // Tarjeta del Desarrollador
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.heading(egui::RichText::new("👤 Desarrollador & Creador").strong().size(14.0));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Desarrollado por:").strong());
                ui.label(egui::RichText::new("Alejandro González Hernández (Vidruck)").color(accent).strong());
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Repositorio Oficial:").strong());
                ui.hyperlink_to("github.com/Vidruck/raven_tiling_emulator", "https://github.com/Vidruck/raven_tiling_emulator");
            });
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Arquitectura: Motor en Rust nativo, IPC D-Bus de viaje único e integración fluida con KWin API de Plasma 6.").size(12.0).weak());
        });
    });

    ui.add_space(12.0);

    // Tarjeta de Licencia
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.heading(egui::RichText::new("📜 Licencia").strong().size(14.0));
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Este proyecto esta licenciado bajo GPL-3.0.").strong());
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Este software es libre y de código abierto. Puedes usarlo, estudiarlo, modificarlo y redistribuirlo de acuerdo con los términos de su licencia.").size(12.0).weak());
        });
    });

    ui.add_space(12.0);

    // Tarjeta de Agradecimiento
    ui.group(|ui| {
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            ui.heading(egui::RichText::new("💖 Agradecimiento y Apoyo").strong().size(14.0).color(accent));
            ui.add_space(6.0);
            ui.label(egui::RichText::new("¡Muchas gracias por utilizar y apoyar Raven Tiling Emulator!").strong().size(13.0));
            ui.add_space(6.0);
            ui.label(
                "Este proyecto fue diseñado con pasión para ofrecerte una experiencia de tiling "
                .to_string() + "fluida, limpia y personalizada en KDE Plasma. Tu uso, comentarios, "
                + "reportes de errores y contribuciones hacen posible que este software continúe evolucionando."
            );
            ui.add_space(8.0);
            ui.label(egui::RichText::new("✨ ¡Espero de corazón que lo disfruten!, ¡Huelum!").size(12.0).italics());
        });
    });
}
