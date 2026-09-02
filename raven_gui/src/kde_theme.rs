//! # Sincronización del Esquema de Colores de KDE Plasma (`KdePalette`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Lee y parsea directamente el archivo `~/.config/kdeglobals` para sincronizar
//! de forma idéntica los colores de fondo, texto, botones y acento de Plasma en `egui`.

use eframe::egui::Color32;
use std::fs;
use std::path::PathBuf;

/// Paleta cromática nativa extraída de la sesión de KDE Plasma.
#[derive(Clone, Debug, PartialEq)]
pub struct KdePalette {
    pub window_bg: Color32,
    pub window_fg: Color32,
    pub view_bg: Color32,
    pub view_fg: Color32,
    pub button_bg: Color32,
    pub button_fg: Color32,
    pub selection_bg: Color32,
    pub selection_fg: Color32,
    pub is_dark: bool,
}

impl Default for KdePalette {
    fn default() -> Self {
        Self {
            window_bg: Color32::from_rgb(15, 23, 42),
            window_fg: Color32::from_rgb(226, 232, 240),
            view_bg: Color32::from_rgb(30, 41, 59),
            view_fg: Color32::from_rgb(226, 232, 240),
            button_bg: Color32::from_rgb(30, 41, 59),
            button_fg: Color32::from_rgb(226, 232, 240),
            selection_bg: Color32::from_rgb(0, 180, 216),
            selection_fg: Color32::WHITE,
            is_dark: true,
        }
    }
}

impl KdePalette {
    pub fn read_from_system() -> Option<Self> {
        let home = std::env::var("HOME").ok()?;
        let kdeglobals_path = PathBuf::from(home).join(".config/kdeglobals");
        
        let content = fs::read_to_string(kdeglobals_path).ok()?;
        
        let mut palette = KdePalette::default();
        let mut current_section = String::new();
        
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len()-1].to_string();
                continue;
            }
            
            if line.contains('=') {
                let parts: Vec<&str> = line.splitn(2, '=').collect();
                if parts.len() == 2 {
                    let key = parts[0].trim();
                    let val = parts[1].trim();
                    
                    if let Some(color) = parse_color(val) {
                        match current_section.as_str() {
                            "Colors:Window" => {
                                match key {
                                    "BackgroundNormal" => palette.window_bg = color,
                                    "ForegroundNormal" => palette.window_fg = color,
                                    _ => {}
                                }
                            }
                            "Colors:View" => {
                                match key {
                                    "BackgroundNormal" => palette.view_bg = color,
                                    "ForegroundNormal" => palette.view_fg = color,
                                    _ => {}
                                }
                            }
                            "Colors:Button" => {
                                match key {
                                    "BackgroundNormal" => palette.button_bg = color,
                                    "ForegroundNormal" => palette.button_fg = color,
                                    _ => {}
                                }
                            }
                            "Colors:Selection" => {
                                match key {
                                    "BackgroundNormal" => palette.selection_bg = color,
                                    "ForegroundNormal" => palette.selection_fg = color,
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        // Determinar si es dark mode basado en luminancia
        let lum = 0.299 * palette.window_bg.r() as f32 
                + 0.587 * palette.window_bg.g() as f32 
                + 0.114 * palette.window_bg.b() as f32;
        palette.is_dark = lum < 128.0;
        
        Some(palette)
    }
}

fn parse_color(rgb: &str) -> Option<Color32> {
    let parts: Vec<&str> = rgb.split(',').collect();
    if parts.len() == 3 {
        let r = parts[0].trim().parse::<u8>().ok()?;
        let g = parts[1].trim().parse::<u8>().ok()?;
        let b = parts[2].trim().parse::<u8>().ok()?;
        Some(Color32::from_rgb(r, g, b))
    } else {
        None
    }
}
