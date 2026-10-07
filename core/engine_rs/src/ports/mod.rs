//! # Capa de Puertos (Interfaces)
//!
//! Define los contratos y abstracciones primarios (casos de uso) y secundarios (adaptadores)
//! necesarios para que el motor interactúe de forma desacoplada con compositores,
//! sistemas de notificación y persistencia.

pub use raven_core::ports::{HistoryStorage, NotificationPort};
pub use raven_core::backend::{CompositorBackend, CompositorEvent, BackendError};
