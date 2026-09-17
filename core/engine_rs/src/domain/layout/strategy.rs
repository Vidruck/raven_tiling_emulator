//! # Estrategias de Layout y Fábrica Polimórfica
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Define el contrato (`trait`) [`LayoutStrategy`] que gobierna todos los algoritmos
//! matemáticos de partición espacial y provee la fábrica dinámica [`get_strategy`].

use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

use super::{
    DivisorStrategy, DwindleBSPStrategy, InvertedStrictDwindleStrategy, LuaLayoutStrategy,
    MonocleStrategy, StrictDwindleStrategy, TallStrategy,
};

/// Rasgo común para todos los algoritmos de distribución de ventanas.
///
/// Garantiza la compatibilidad con hilos (`Send + Sync`) para ejecución concurrente segura en Rust.
pub trait LayoutStrategy: Send + Sync {
    /// Calcula el mapa de geometrías resultantes para una lista dada de ventanas.
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        nmaster: usize,
        master_ratio: f32,
        default_gaps: i32,
        active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>);

    /// Predice cuántas ventanas puede alojar de forma ergonómica y estable este algoritmo en la pantalla dada.
    fn predict_capacity(&self, screen_rect: Rect, default_gaps: i32) -> usize {
        let usable_w = std::cmp::max(1, screen_rect.width - default_gaps);
        let usable_h = std::cmp::max(1, screen_rect.height - default_gaps);
        let cols = (usable_w / 300).max(1) as usize;
        let rows = (usable_h / 250).max(1) as usize;
        cols * rows
    }
}

/// Fábrica para la instanciación dinámica de algoritmos de distribución según su nombre.
///
/// # Parámetros
/// - `layout_type`: Nombre de la estrategia (`"tall"`, `"monocle"`, `"strict_dwindle"`, `"inverted_strict_dwindle"`, `"divisor"`, `"raven"`).
///
/// # Retorno
/// Un puntero inteligente `Box<dyn LayoutStrategy>` listo para ser invocado.
pub fn get_strategy(layout_type: &str) -> Box<dyn LayoutStrategy> {
    match layout_type {
        "tall" => Box::new(TallStrategy),
        "monocle" => Box::new(MonocleStrategy),
        "strict_dwindle" => Box::new(StrictDwindleStrategy),
        "inverted_strict_dwindle" => Box::new(InvertedStrictDwindleStrategy),
        "divisor" => Box::new(DivisorStrategy),
        "raven" => Box::new(DwindleBSPStrategy),
        _ => {
            // [EXTENSIÓN LUA]: Si el nombre del layout no coincide con ninguno compilado nativamente,
            // el motor asume que es un script de usuario y busca un archivo `.lua` homónimo en la configuración.
            let home_dir = std::env::var("HOME").unwrap_or_else(|_| "/home/vidruck".to_string());
            let lua_path = std::path::PathBuf::from(home_dir)
                .join(".config/raven/layouts")
                .join(format!("{}.lua", layout_type));

            tracing::info!("Intentando cargar layout de usuario desde: {:?}", lua_path);

            if let Ok(script) = std::fs::read_to_string(&lua_path) {
                tracing::info!("Script {} cargado exitosamente", layout_type);
                Box::new(LuaLayoutStrategy::new(layout_type.to_string(), script))
            } else {
                // Fallback de resiliencia: Si el archivo no existe o hay error de lectura,
                // prevenimos pánicos cayendo suavemente al algoritmo estrella (Dwindle BSP).
                tracing::warn!(
                    "No se pudo cargar {}.lua en {:?}. Cayendo al layout BSP por defecto.",
                    layout_type,
                    lua_path
                );
                Box::new(DwindleBSPStrategy)
            }
        }
    }
}
