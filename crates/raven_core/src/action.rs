//! # Catálogo de Acciones de Dominio (`RavenAction`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Este módulo define las acciones atómicas e intenciones geométricas emitidas por el motor
//! hacia los adaptadores de infraestructura (D-Bus / KWin Script).

use serde::{Deserialize, Serialize};

/// Acciones de dominio que representan intenciones del motor sobre las ventanas.
///
/// Estas acciones son completamente agnósticas a la infraestructura subyacente (D-Bus, X11, Wayland).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RavenAction {
    /// Mueve y redimensiona una ventana a una posición y dimensiones específicas.
    MoveWindow {
        /// Identificador de la ventana.
        window_id: String,
        /// Coordenada horizontal de destino.
        x: i32,
        /// Coordenada vertical de destino.
        y: i32,
        /// Ancho asignado en píxeles.
        width: i32,
        /// Alto asignado en píxeles.
        height: i32,
    },
    /// Solicita enfocar (focus) una ventana específica para activarla.
    FocusWindow {
        /// Identificador de la ventana a enfocar.
        window_id: String,
    },
    /// Migra una ventana a otra salida (output) física de pantalla.
    MigrateToOutput {
        /// Identificador de la ventana.
        window_id: String,
        /// Identificador de la salida de video destino.
        target_output: String,
    },
    /// Migra una ventana a otro escritorio virtual (virtual desktop).
    MigrateToDesktop {
        /// Identificador de la ventana.
        window_id: String,
        /// Identificador del escritorio virtual destino.
        target_desktop: String,
    },
    /// Minimiza una ventana en el compositor.
    MinimizeWindow {
        /// Identificador de la ventana a minimizar.
        window_id: String,
    },
    /// Desminimiza (unminimizes) una ventana para hacerla visible nuevamente.
    UnminimizeWindow {
        /// Identificador de la ventana a restaurar.
        window_id: String,
    },
    /// Señala a KWin que la ventana ha superado el periodo de cuarentena.
    ReleaseQuarantine {
        /// Identificador de la ventana.
        window_id: String,
    },
    /// Solicita retroalimentación (feedback) de sincronización de estado tras registrarse una ventana estricta.
    RequestFeedback {
        /// Identificador de la ventana.
        window_id: String,
    },
    /// Notifica al bridge que la pantalla está próxima o en estado de saturación.
    SaturationWarning {
        /// Número máximo calculado de ventanas estables en la pantalla activa.
        cmax: usize,
        /// Número actual de ventanas activas.
        active: usize,
    },
    /// Modifica el estado flotante dinámico de una ventana (Quick Peek) y su elevación.
    SetFloating {
        /// Identificador de la ventana.
        window_id: String,
        /// Indica si debe flotar (true) o volver al mosaico (false).
        floating: bool,
        /// Indica si debe mantenerse por encima (keepAbove).
        keep_above: bool,
    },
    /// Rectifica la geometría de una ventana post-cuarentena cuando la ventana ignoró las
    /// órdenes de posicionamiento iniciales (ej. navegadores con restauración de sesión CSD).
    ///
    /// A diferencia de `MoveWindow`, este comando se aplica forzosamente en el lado KWin
    /// sin la guardia anti-redundancia de geometrías idénticas, garantizando la corrección.
    RectifyWindow {
        /// Identificador de la ventana a rectificar.
        window_id: String,
        /// Coordenada horizontal corregida.
        x: i32,
        /// Coordenada vertical corregida.
        y: i32,
        /// Ancho corregido en píxeles.
        width: i32,
        /// Alto corregido en píxeles.
        height: i32,
    },
    /// Modifica el estado maximizado de una ventana.
    SetMaximize {
        /// Identificador de la ventana.
        window_id: String,
        /// Indica si debe maximizarse (true) o desmaximizarse (false).
        maximized: bool,
    },
    /// Cierra una ventana en el compositor.
    CloseWindow {
        /// Identificador de la ventana a cerrar.
        window_id: String,
    },
    /// Solicita forzar una sincronización global de estado desde el compositor.
    RequestSync,
}
