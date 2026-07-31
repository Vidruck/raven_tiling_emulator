use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

use super::{
    DivisorStrategy, DwindleBSPStrategy, MonocleStrategy, StrictDwindleStrategy, TallStrategy,
};

/// Interface común para todos los algoritmos de distribución de ventanas (Layout Strategies).
pub trait LayoutStrategy: Send + Sync {
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
pub fn get_strategy(layout_type: &str) -> Box<dyn LayoutStrategy> {
    match layout_type {
        "tall" => Box::new(TallStrategy),
        "monocle" => Box::new(MonocleStrategy),
        "strict_dwindle" => Box::new(StrictDwindleStrategy),
        "divisor" => Box::new(DivisorStrategy),
        "raven" | _ => Box::new(DwindleBSPStrategy),
    }
}
