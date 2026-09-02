//! # Modelos y Estructuras de Datos de la GUI (`models.rs`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Define las enumeraciones de navegación (`NavTab`), definiciones de presets y catálogo de atajos.

/// Categorías principales del panel de navegación lateral (NavigationRail).
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum NavTab {
    /// Pestaña 1: Selección de algoritmos y presets de mosaico.
    Layouts,
    /// Pestaña 2: Composición, geometría (Gaps/Ratio), modo PiP, reglas y cuarentenas.
    CompositionPip,
    /// Pestaña 3: Visor de atajos globales de teclado integrados en KDE.
    ShortcutsGuide,
    /// Pestaña 4: Control del servicio systemd y arranque automático.
    EngineService,
    /// Pestaña 5: Créditos del desarrollador, repositorio y licencia.
    About,
}

/// Definición del catálogo de presets preconfigurados.
#[allow(dead_code)]
pub struct PresetDef {
    pub name: &'static str,
    pub display: &'static str,
    pub desc: &'static str,
    pub layout_type: &'static str,
    pub gaps: i32,
    pub ratio: f32,
}

/// Catálogo estático de presets alineados con la lógica del motor en Rust (`raven_engine`).
pub const PRESETS: &[PresetDef] = &[
    PresetDef {
        name: "raven",
        display: "Raven (Base)",
        desc: "Esquema dinámico y asimétrico para pantallas panorámicas.",
        layout_type: "raven",
        gaps: 6,
        ratio: 0.5,
    },
    PresetDef {
        name: "clasico",
        display: "Clásico",
        desc: "Esquema de panel maestro con pila secundaria.",
        layout_type: "tall",
        gaps: 8,
        ratio: 0.55,
    },
    PresetDef {
        name: "monoculo",
        display: "Monóculo",
        desc: "Modo maximizado de una sola ventana.",
        layout_type: "monocle",
        gaps: 0,
        ratio: 1.0,
    },
    PresetDef {
        name: "hyper",
        display: "Flujo Avanzado",
        desc: "Mosaico fractal estrictamente simétrico en espiral.",
        layout_type: "strict_dwindle",
        gaps: 8,
        ratio: 0.5,
    },
    PresetDef {
        name: "hyper_inverted",
        display: "Flujo Avanzado (Invertido)",
        desc: "Mosaico fractal en espiral con orden de acomodo invertido.",
        layout_type: "inverted_strict_dwindle",
        gaps: 8,
        ratio: 0.5,
    },
    PresetDef {
        name: "divisor",
        display: "Divisor",
        desc: "Disposición equitativa en columnas proporcionales.",
        layout_type: "divisor",
        gaps: 8,
        ratio: 0.5,
    },
];
