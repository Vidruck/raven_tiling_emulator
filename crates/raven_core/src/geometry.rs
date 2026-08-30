//! # Geometría y Estructuras de Datos
//!
//! Este submódulo define las estructuras de datos fundamentales utilizadas por el motor
//! para representar dimensiones de pantalla y propiedades de las ventanas.

use serde::{Deserialize, Serialize};

/// Representa un rectángulo en el espacio 2D de la pantalla.
///
/// Se utiliza para definir tanto el área total de la pantalla como el área
/// asignada a cada ventana después de calcular el layout.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Posición en el eje X (horizontal).
    pub x: i32,
    /// Posición en el eje Y (vertical).
    pub y: i32,
    /// Ancho del rectángulo en píxeles.
    pub width: i32,
    /// Alto del rectángulo en píxeles.
    pub height: i32,
}

impl Rect {
    /// Crea una nueva instancia de un rectángulo (`Rect`).
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Rect {
            x,
            y,
            width,
            height,
        }
    }
}

/// Representa una ventana y sus propiedades de estado dentro del motor.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowNode {
    /// Identificador único de la ventana (usualmente el WID de X11 o KWin).
    pub window_id: String,
    /// Identificador del escritorio o actividad donde se encuentra la ventana.
    pub workspace_id: String,
    /// Identificador del monitor físico.
    pub output: String,
    /// Identificadores de todos los escritorios virtuales asociados a la ventana.
    pub desktops: Vec<String>,
    /// Indica si la ventana está en modo flotante (floating).
    pub is_floating: bool,
    /// Indica si la ventana está minimizada (minimized).
    pub is_minimized: bool,
    /// Indica si la ventana está en modo Picture-in-Picture (PiP).
    pub is_pip: bool,
    /// Geometría actual de la ventana reportada por el compositor.
    pub geometry: Rect,
    /// Ancho mínimo innegociable reportado por KWin.
    pub min_w: i32,
    /// Alto mínimo innegociable reportado por KWin.
    pub min_h: i32,
    /// Bandera que indica si la ventana requiere retroalimentación (feedback) inmediata en su creación.
    pub strict_birth: bool,
    /// Indica si la ventana se encuentra actualmente en cuarentena de estabilización geométrica (Gecko/CSD).
    #[serde(default)]
    pub is_quarantined: bool,
    /// Indica si la ventana se encuentra en modo pantalla completa nativo.
    #[serde(default, rename = "fs")]
    pub is_fullscreen: bool,
    /// Clase WM / Resource class reportada por KWin (e.g. "firefox", "vlc").
    #[serde(default)]
    pub resource_class: String,
    /// Título / Caption de la ventana reportado por KWin.
    #[serde(default)]
    pub caption: String,
    /// Proporción dinámica de ancho personalizada (ratio 2D horizontal).
    #[serde(default)]
    pub custom_w_ratio: Option<f32>,
    /// Proporción dinámica de alto personalizada (ratio 2D vertical).
    #[serde(default)]
    pub custom_h_ratio: Option<f32>,
}

impl WindowNode {
    /// Crea una nueva instancia de un nodo de ventana (`WindowNode`) con sus propiedades iniciales.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        window_id: String,
        workspace_id: String,
        output: String,
        desktops: Vec<String>,
        is_floating: bool,
        is_minimized: bool,
        is_pip: bool,
        geometry: Rect,
        min_w: i32,
        min_h: i32,
        strict_birth: bool,
        is_quarantined: bool,
        is_fullscreen: bool,
    ) -> Self {
        WindowNode {
            window_id,
            workspace_id,
            output,
            desktops,
            is_floating,
            is_minimized,
            is_pip,
            geometry,
            min_w,
            min_h,
            strict_birth,
            is_quarantined,
            is_fullscreen,
            resource_class: String::new(),
            caption: String::new(),
            custom_w_ratio: None,
            custom_h_ratio: None,
        }
    }

    /// Añade información de clase y caption para arbitraje de reglas en Rust.
    pub fn with_class_and_caption(mut self, resource_class: String, caption: String) -> Self {
        self.resource_class = resource_class;
        self.caption = caption;
        self
    }
}
