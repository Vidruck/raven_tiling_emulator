//! # Algoritmo de Enfoque Total (`MonocleStrategy`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Implementa la distribución monóculo (Monocle): cada ventana activa ocupa el 100%
//! del contenedor disponible con márgenes aplicados, superponiéndose en capas tipo pestañas.

use super::{apply_gaps, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Estrategia de layout "Monóculo": asigna el tamaño completo de la pantalla a cada ventana.
pub struct MonocleStrategy;

impl LayoutStrategy for MonocleStrategy {
    /// Asigna a todas las ventanas no flotantes del workspace exactamente la misma geometría de área completa.
    ///
    /// # Parámetros
    /// - `windows`: Lista de ventanas registradas.
    /// - `screen_rect`: Área utilizable de la pantalla.
    /// - `_nmaster`: No utilizado en Monóculo.
    /// - `_master_ratio`: No utilizado en Monóculo.
    /// - `default_gaps`: Espaciado alrededor de la pantalla.
    /// - `_active_window_id`: Identificador de la ventana enfocada.
    ///
    /// # Retorno
    /// Tupla con las posiciones calculadas (todas idénticas) y lista de ventanas evictadas (vacía).
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        _nmaster: usize,
        _master_ratio: f32,
        default_gaps: i32,
        _active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        let active_windows: Vec<&WindowNode> = windows
            .iter()
            .filter(|w| !w.is_floating && !w.is_minimized)
            .collect();

        if active_windows.is_empty() {
            return (HashMap::new(), Vec::new());
        }

        let mut layout_map = HashMap::with_capacity(active_windows.len());
        let evicted_windows = Vec::new();
        let target_rect = apply_gaps(&screen_rect, default_gaps);

        // 2. Definir el contenedor principal aplicando los gaps configurados
        for win in active_windows {
            layout_map.insert(win.window_id.clone(), target_rect);
        }

        (layout_map, evicted_windows)
    }
}
