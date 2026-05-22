use std::fmt;

/// Errores de dominio del motor Raven.
#[derive(Debug)]
pub enum RavenError {
    /// Errores al cargar o interpretar la configuración (config).
    ConfigError(String),
    /// Errores de comunicación con componentes externos (IPC).
    IpcError(String),
    /// Errores en el cálculo matemático del layout.
    LayoutError(String),
    /// Errores de validación de datos provenientes de la infraestructura.
    ValidationError(String),
}

impl fmt::Display for RavenError {
    /// Formatea e imprime el error de dominio de forma descriptiva.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RavenError::ConfigError(msg) => write!(f, "ConfigError: {}", msg),
            RavenError::IpcError(msg) => write!(f, "IpcError: {}", msg),
            RavenError::LayoutError(msg) => write!(f, "LayoutError: {}", msg),
            RavenError::ValidationError(msg) => write!(f, "ValidationError: {}", msg),
        }
    }
}

impl std::error::Error for RavenError {}
