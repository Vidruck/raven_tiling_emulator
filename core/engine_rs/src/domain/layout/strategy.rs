//! # Módulo de Estrategias y Fábrica de Layouts
//!
//! Define la interfaz abstracta `LayoutStrategy` que implementa cada algoritmo de ordenamiento
//! y proporciona el patrón Fábrica (`get_strategy`) para instanciar dinámicamente el layout deseado.

use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

use super::{
    DivisorStrategy, DwindleBSPStrategy, InvertedStrictDwindleStrategy, MonocleStrategy,
    StrictDwindleStrategy, TallStrategy,
};

/// Interfaz (Trait) común para todos los algoritmos de distribución de ventanas (Layout Strategies).
///
/// Garantiza la compatibilidad con hilos (`Send + Sync`) para ejecución concurrente segura en Rust.
pub trait LayoutStrategy: Send + Sync {
    /// Calcula el mapa de geometrías resultantes para una lista dada de ventanas.
    ///
    /// # Parámetros
    /// - `windows`: Lista de ventanas no minimizadas registradas en el workspace.
    /// - `screen_rect`: Dimensión y posición de la pantalla o área útil de trabajo.
    /// - `nmaster`: Cantidad de ventanas principales/maestras solicitadas.
    /// - `master_ratio`: Relación de tamaño entre el área principal y secundaria (ej. 0.50..0.80).
    /// - `default_gaps`: Espaciado interno en píxeles.
    /// - `active_window_id`: Identificador opcional de la ventana enfocado en la interfaz.
    ///
    /// # Retorno
    /// Una tupla conteniendo:
    /// 1. `HashMap<String, Rect>`: Mapa que vincula cada `window_id` con su geometría de pantalla calculada (`Rect`).
    /// 2. `Vec<String>`: Lista de IDs de ventanas evictadas/minimizadas si la capacidad fue rebasada.
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        nmaster: usize,
        master_ratio: f32,
        default_gaps: i32,
        active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>);
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
        "raven" | _ => Box::new(DwindleBSPStrategy),
    }
}
