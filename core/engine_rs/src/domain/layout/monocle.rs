//! # Algoritmo de Enfoque Total Exclusivo (`MonocleStrategy`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Implementa la distribución monóculo exclusiva (Monocle): la ventana activa ocupa
//! el 100% del contenedor disponible con márgenes aplicados. En lugar de sobreponer
//! todas las ventanas, minimiza atómicamente a las demás (desalojo/evicción).
//! Al enfocar o aperturar otra ventana, esta pasa a primer plano y la previa se minimiza.

use super::{apply_gaps, LayoutStrategy};
use crate::domain::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Estrategia de layout "Monóculo Exclusivo": asigna el tamaño completo de la pantalla a la ventana activa
/// y minimiza el resto para evitar sobreposiciones visuales.
pub struct MonocleStrategy;

impl LayoutStrategy for MonocleStrategy {
    /// Predice la capacidad ergonómica del modo Monóculo (siempre 1 ventana visible a la vez).
    fn predict_capacity(&self, _screen_rect: Rect, _default_gaps: i32) -> usize {
        1
    }

    /// Asigna a la ventana activa la geometría de área completa con gaps y desaloja (eviction / minimización)
    /// a todas las demás ventanas del workspace.
    ///
    /// # Parámetros
    /// - `windows`: Lista de ventanas registradas en el workspace.
    /// - `screen_rect`: Área utilizable de la pantalla.
    /// - `_nmaster`: No utilizado en Monóculo.
    /// - `_master_ratio`: No utilizado en Monóculo.
    /// - `default_gaps`: Espaciado alrededor de la pantalla.
    /// - `active_window_id`: Identificador de la ventana enfocada o activa.
    ///
    /// # Retorno
    /// Tupla con la posición calculada para la ventana activa y la lista de ventanas a minimizar.
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        _nmaster: usize,
        _master_ratio: f32,
        default_gaps: i32,
        active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        let non_floating_windows: Vec<&WindowNode> = windows
            .iter()
            .filter(|w| !w.is_floating)
            .collect();

        if non_floating_windows.is_empty() {
            return (HashMap::new(), Vec::new());
        }

        // Determinar cuál es la ventana que debe estar visible en pantalla:
        // 1. Si `active_window_id` coincide con una de las ventanas no flotantes, esa es la elegida.
        // 2. Si no, tomar la primera ventana que no esté minimizada.
        // 3. Si todas están minimizadas, tomar la primera ventana no flotante.
        let target_active = active_window_id
            .as_ref()
            .and_then(|act_id| non_floating_windows.iter().find(|w| &w.window_id == act_id))
            .or_else(|| non_floating_windows.iter().find(|w| !w.is_minimized))
            .or_else(|| non_floating_windows.first())
            .copied();

        let mut layout_map = HashMap::with_capacity(1);
        let mut evicted_windows = Vec::new();
        let target_rect = apply_gaps(&screen_rect, default_gaps);

        if let Some(active_win) = target_active {
            layout_map.insert(active_win.window_id.clone(), target_rect);

            for win in non_floating_windows {
                if win.window_id != active_win.window_id && !win.is_minimized {
                    evicted_windows.push(win.window_id.clone());
                }
            }
        }

        (layout_map, evicted_windows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_dummy_window(id: &str, is_floating: bool, is_minimized: bool) -> WindowNode {
        WindowNode::new(
            id.to_string(),
            "HDMI-A-1||1".to_string(),
            "HDMI-A-1".to_string(),
            vec!["1".to_string()],
            is_floating,
            is_minimized,
            false,
            Rect::new(0, 0, 800, 600),
            100,
            100,
            false,
            false,
            false,
        )
    }

    #[test]
    fn test_monocle_single_window() {
        let strategy = MonocleStrategy;
        let screen = Rect::new(0, 0, 1920, 1080);
        let win = create_dummy_window("win1", false, false);
        let (layout, evicted) = strategy.calculate(&[win], screen, 1, 0.5, 10, None);

        assert_eq!(layout.len(), 1);
        assert!(evicted.is_empty());
        let rect = layout.get("win1").unwrap();
        assert_eq!(rect.width, 1920 - 20);
        assert_eq!(rect.height, 1080 - 20);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 10);
    }

    #[test]
    fn test_monocle_minimizes_previous_window_when_new_one_active() {
        let strategy = MonocleStrategy;
        let screen = Rect::new(0, 0, 1920, 1080);
        let win1 = create_dummy_window("win1", false, false);
        let win2 = create_dummy_window("win2", false, false);

        // Si win2 es la ventana activa (por ejemplo, recién abierta o invocada)
        let (layout, evicted) = strategy.calculate(
            &[win1, win2],
            screen,
            1,
            0.5,
            0,
            Some("win2".to_string()),
        );

        assert_eq!(layout.len(), 1);
        assert!(layout.contains_key("win2"));
        assert_eq!(evicted, vec!["win1".to_string()]);
    }

    #[test]
    fn test_monocle_swaps_back_when_minimized_window_is_activated() {
        let strategy = MonocleStrategy;
        let screen = Rect::new(0, 0, 1920, 1080);
        let win1 = create_dummy_window("win1", false, true); // previamente minimizada
        let win2 = create_dummy_window("win2", false, false); // actualmente en pantalla

        // Usuario invoca / enfoca de nuevo win1
        let (layout, evicted) = strategy.calculate(
            &[win1, win2],
            screen,
            1,
            0.5,
            0,
            Some("win1".to_string()),
        );

        assert_eq!(layout.len(), 1);
        assert!(layout.contains_key("win1"));
        assert_eq!(evicted, vec!["win2".to_string()]);
    }
}

