//! # Motor de Composición Predictiva — Saturación de Pantalla
//!
//! Calcula el número máximo de ventanas que caben en una pantalla antes de que
//! ocurra colapso visual (Cmax), y modela los estados de saturación del compositor.


/// Estado de saturación de la composición en pantalla.
#[derive(Debug, Clone, PartialEq)]
pub enum SaturationState {
    /// N < Cmax: El algoritmo BSP/Foveal opera con ratios libres.
    Fluid,
    /// N = Cmax - 1: Pre-advertencia. Enviar señal al bridge.
    PreSaturation,
    /// N = Cmax: Ratios congelados en proporciones óptimas fijas.
    Saturated,
    /// N > Cmax: Evicción atómica de la ventana más antigua.
    Overloaded,
}

/// Resultado del cálculo de capacidad de pantalla.
#[derive(Debug, Clone)]
pub struct ScreenCapacity {
    /// Número máximo de ventanas estables que caben en pantalla (Cmax).
    pub cmax: usize,
    /// Estado actual de saturación basado en el número de ventanas activas.
    pub state: SaturationState,
}

/// Calcula la capacidad máxima de ventanas estables en una pantalla (Cmax).
///
/// Fórmula:
/// `Cmax = floor((W - 2*gaps) / min_w) * floor((H - panel_h - 2*gaps) / min_h)`
///
/// # Parámetros
/// * `screen_w` - Ancho de la pantalla en píxeles.
/// * `screen_h` - Alto de la pantalla en píxeles.
/// * `gaps` - Margen entre ventanas en píxeles.
/// * `min_w` - Ancho mínimo funcional de una ventana en píxeles.
/// * `min_h` - Alto mínimo funcional de una ventana en píxeles.
/// * `active_windows` - Número actual de ventanas activas (no flotantes).
///
/// # Retorno
/// `ScreenCapacity` con el Cmax calculado y el estado de saturación actual.
pub fn calculate_screen_capacity(
    screen_w: i32,
    screen_h: i32,
    gaps: i32,
    min_w: i32,
    min_h: i32,
    active_windows: usize,
) -> ScreenCapacity {
    // Asegurar valores mínimos para evitar divisiones por cero
    let eff_min_w = std::cmp::max(min_w, 150);
    let eff_min_h = std::cmp::max(min_h, 120);
    let eff_gap = std::cmp::max(gaps, 0);

    let usable_w = std::cmp::max(1, screen_w - 2 * eff_gap);
    let usable_h = std::cmp::max(1, screen_h - 2 * eff_gap);

    let cols = (usable_w / eff_min_w).max(1) as usize;
    let rows = (usable_h / eff_min_h).max(1) as usize;
    let cmax = (cols * rows).max(1);

    let state = if active_windows == 0 || active_windows < cmax.saturating_sub(1) {
        SaturationState::Fluid
    } else if active_windows == cmax.saturating_sub(1) {
        SaturationState::PreSaturation
    } else if active_windows == cmax {
        SaturationState::Saturated
    } else {
        SaturationState::Overloaded
    };

    ScreenCapacity { cmax, state }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capacidad_pantalla_1080p() {
        // 1920x1080, gaps=8, min_w=300, min_h=250
        // usable_w = 1920 - 16 = 1904, cols = 1904/300 = 6
        // usable_h = 1080 - 16 = 1064, rows = 1064/250 = 4
        // Cmax = 24
        let cap = calculate_screen_capacity(1920, 1080, 8, 300, 250, 3);
        assert_eq!(cap.cmax, 24);
        assert_eq!(cap.state, SaturationState::Fluid);
    }

    #[test]
    fn test_estado_presaturacion() {
        let cap = calculate_screen_capacity(1920, 1080, 8, 300, 250, 23);
        assert_eq!(cap.state, SaturationState::PreSaturation);
    }

    #[test]
    fn test_estado_sobrecargado() {
        let cap = calculate_screen_capacity(1920, 1080, 8, 300, 250, 30);
        assert_eq!(cap.state, SaturationState::Overloaded);
    }

    #[test]
    fn test_pantalla_laptop_14() {
        // 1366x768, gaps=4, min_w=250, min_h=180
        // usable_w = 1366 - 8 = 1358, cols = 1358/250 = 5
        // usable_h = 768 - 8 = 760, rows = 760/180 = 4
        // Cmax = 20
        let cap = calculate_screen_capacity(1366, 768, 4, 250, 180, 1);
        assert_eq!(cap.cmax, 20);
        assert_eq!(cap.state, SaturationState::Fluid);
    }
}
