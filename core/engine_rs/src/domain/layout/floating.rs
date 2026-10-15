//! # Estrategia Flotante / Sin Mosaico (`floating.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Implementa la estrategia nula o flotante ([`FloatingStrategy`]).
//! Bajo esta estrategia, el motor de Raven omite el cálculo de mosaico para el espacio de
//! trabajo asignado, permitiendo que KWin gestione libremente la posición flotante de las
//! ventanas sin emitir comandos de reposicionamiento destructivos ni desalojar ventanas.

use crate::domain::geometry::{Rect, WindowNode};
use crate::domain::layout::strategy::LayoutStrategy;
use std::collections::HashMap;

/// Estrategia de layout flotante / libre.
///
/// Retorna un mapa de geometrías vacío y cero ventanas desalojadas con complejidad $O(1)$,
/// suspendiendo el cálculo de partición geométrica exclusivamente para el espacio de trabajo
/// o monitor configurado.
#[derive(Debug, Default, Clone, Copy)]
pub struct FloatingStrategy;

impl LayoutStrategy for FloatingStrategy {
    fn calculate(
        &self,
        _windows: &[WindowNode],
        _screen_rect: Rect,
        _nmaster: usize,
        _master_ratio: f32,
        _default_gaps: i32,
        _active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        (HashMap::new(), Vec::new())
    }

    fn predict_capacity(&self, _screen_rect: Rect, _default_gaps: i32) -> usize {
        usize::MAX
    }
}
