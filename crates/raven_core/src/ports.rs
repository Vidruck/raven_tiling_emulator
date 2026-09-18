//! # Puertos Secundarios del Dominio (`ports`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Define los contratos de interfaces (driven ports) para almacenamiento de estado,
//! emisión de notificaciones en el entorno de escritorio y servicios auxiliares,
//! aislando la lógica de aplicación de las llamadas al sistema operativo.

use std::collections::VecDeque;

/// Puerto secundario para persistencia y recuperación del historial de ventanas (LRU).
pub trait HistoryStorage: Send + Sync {
    /// Carga el historial ordenado de identificadores de ventana desde el medio persistente.
    fn load_history(&self) -> VecDeque<String>;
    /// Persiste de manera asíncrona o diferida el historial de ventanas.
    fn save_history(&self, history: VecDeque<String>);
}

/// Puerto secundario para notificaciones OSD y visuales en el entorno de escritorio.
pub trait NotificationPort: Send + Sync {
    /// Despacha una notificación OSD en tiempo real con reemplazo sincrónico si está disponible.
    fn notify_osd(&self, title: &str, body: &str);
}
