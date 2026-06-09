/// # Presets de Layout de Composición Foveal
///
/// Define los 5 modos de composición semánticos que reemplazan la configuración
/// manual abstracta de `nmaster` por perfiles comprensibles para el usuario final.

/// Parámetros de configuración de un preset de layout.
#[derive(Debug, Clone)]
pub struct LayoutPreset {
    /// Nombre técnico interno del preset.
    pub name: &'static str,
    /// Nombre de presentación al usuario.
    pub display_name: &'static str,
    /// Descripción breve del comportamiento geométrico.
    pub description: &'static str,
    /// Espacio entre ventanas en píxeles.
    pub gaps: i32,
    /// Proporción de corte asimétrico del área central (0.0 a 1.0).
    pub master_ratio: f32,
    /// Ancho mínimo funcional de referencia para cálculo de Cmax.
    pub min_win_w: i32,
    /// Alto mínimo funcional de referencia para cálculo de Cmax.
    pub min_win_h: i32,
}

/// Catálogo de los 5 presets de composición disponibles.
pub const PRESETS: &[LayoutPreset] = &[
    LayoutPreset {
        name: "dense",
        display_name: "Cargada y Comprimida",
        description: "Máxima densidad: sin gaps, ventanas al límite. Ideal para multitarea intensa.",
        gaps: 0,
        master_ratio: 0.5,
        min_win_w: 200,
        min_win_h: 150,
    },
    LayoutPreset {
        name: "aesthetic",
        display_name: "Estética Raven",
        description: "Proporciones áureas con gaps amplios. Flujo de trabajo minimalista y enfocado.",
        gaps: 16,
        master_ratio: 0.618,
        min_win_w: 350,
        min_win_h: 250,
    },
    LayoutPreset {
        name: "functional",
        display_name: "Funcional",
        description: "Panel maestro amplio para la app principal. Secundarias apiladas a los lados.",
        gaps: 8,
        master_ratio: 0.7,
        min_win_w: 300,
        min_win_h: 200,
    },
    LayoutPreset {
        name: "balanced",
        display_name: "Punto Medio",
        description: "Distribución igualitaria entre todas las ventanas activas.",
        gaps: 8,
        master_ratio: 0.5,
        min_win_w: 300,
        min_win_h: 250,
    },
    LayoutPreset {
        name: "simple",
        display_name: "Pues Sirve",
        description: "Esquema lineal simplificado. Optimizado para laptops de 14\" y resoluciones bajas.",
        gaps: 4,
        master_ratio: 0.5,
        min_win_w: 250,
        min_win_h: 180,
    },
];

/// Busca un preset por su nombre técnico. Retorna `"balanced"` como fallback.
pub fn find_preset(name: &str) -> &'static LayoutPreset {
    PRESETS
        .iter()
        .find(|p| p.name == name)
        .unwrap_or(&PRESETS[3]) // fallback: balanced
}
