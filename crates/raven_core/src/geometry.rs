//! # Geometría y Estructuras de Datos Espaciales
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Este submódulo define las estructuras de datos fundamentales utilizadas por el motor
//! para representar dimensiones de pantalla y propiedades de estado de las ventanas.

use serde::{Deserialize, Serialize};

/// Representa un rectángulo en el espacio bidimensional (2D) de la pantalla.
///
/// Se utiliza para definir tanto el área total de los monitores físicos como el marco
/// geométrico asignado a cada ventana tras el cálculo de partición del layout.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Posición en el eje X (horizontal en píxeles).
    pub x: i32,
    /// Posición en el eje Y (vertical en píxeles).
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
    /// Indica si la ventana se encuentra maximizada.
    #[serde(default, rename = "max")]
    pub is_maximized: bool,
    /// Bandera de sospecha activa: la ventana fue marcada por KWin por flood de señales
    /// o ausencia de clase WM. Rust endurece su modelo de rectificación para esta ventana.
    /// Con `is_suspicious = true`, la Fase 2 (RectifyWindow) re-envía el layout
    /// incondicionalmente sin comparar contra la geometría reportada por el bridge.
    #[serde(default, rename = "sus")]
    pub is_suspicious: bool,
    /// Clase WM / Resource class reportada por KWin (segunda parte de WM_CLASS, ej. "zen-browser", "firefox").
    #[serde(default)]
    pub resource_class: String,
    /// Nombre del ejecutable / Resource name (primera parte de WM_CLASS, ej. "zen", "Navigator").
    /// Más estable que la clase para identificar el ejecutable real del proceso.
    #[serde(default)]
    pub resource_name: String,
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
            is_maximized: false,
            is_suspicious: false,
            resource_class: String::new(),
            resource_name: String::new(),
            caption: String::new(),
            custom_w_ratio: None,
            custom_h_ratio: None,
        }
    }

    /// Asigna el estado maximizado a la ventana.
    pub fn with_maximized(mut self, is_maximized: bool) -> Self {
        self.is_maximized = is_maximized;
        self
    }

    /// Añade información de clase, nombre de ejecutable, sospecha y caption para arbitraje de reglas en Rust.
    ///
    /// El parámetro `is_suspicious` es `true` cuando KWin marcó la ventana durante el Timer-0
    /// por flood de señales de geometría o por ausencia de clase WM. Rust endurece su modelo
    /// de rectificación (Fase 2) para estas ventanas, re-enviando el layout incondicionalmente.
    pub fn with_class_and_caption(
        mut self,
        resource_class: String,
        resource_name: String,
        is_suspicious: bool,
        caption: String,
    ) -> Self {
        self.resource_class = resource_class;
        self.resource_name = resource_name;
        self.is_suspicious = is_suspicious;
        self.caption = caption;
        self
    }
}

/// Representa una salida o monitor físico en el compositor.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputNode {
    /// Nombre o conector del monitor (ej. "eDP-1", "DP-2", "HDMI-A-1").
    pub name: String,
    /// Rectángulo geométrico del monitor en coordenadas globales del compositor.
    pub rect: Rect,
    /// Factor de escala del monitor (ej. 1.0, 1.25, 2.0).
    #[serde(default = "default_scale")]
    pub scale: f64,
}

fn default_scale() -> f64 {
    1.0
}

impl OutputNode {
    /// Crea una nueva instancia de monitor físico (`OutputNode`).
    pub fn new(name: String, rect: Rect, scale: f64) -> Self {
        Self { name, rect, scale }
    }
}

/// Representa el modelo universal de topología física y lógica de pantallas y escritorios.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Topology {
    /// Listado de identificadores o nombres de pantallas físicas conectadas.
    #[serde(default)]
    pub outputs: Vec<String>,
    /// Nodos detallados de monitores con geometría y escala (si están disponibles).
    #[serde(default)]
    pub output_nodes: Vec<OutputNode>,
    /// Listado de identificadores de escritorios virtuales activos.
    #[serde(default)]
    pub desktops: Vec<String>,
    /// Identificador del escritorio virtual activo actualmente.
    #[serde(default)]
    pub current_desktop: String,
}

impl Topology {
    /// Crea una nueva topología básica a partir de listas de outputs y escritorios.
    pub fn new(
        outputs: Vec<String>,
        output_nodes: Vec<OutputNode>,
        desktops: Vec<String>,
        current_desktop: String,
    ) -> Self {
        Self {
            outputs,
            output_nodes,
            desktops,
            current_desktop,
        }
    }
}
