//! # Capa de Infraestructura
//!
//! Implementa la comunicación con el mundo exterior, incluyendo la gestión
//! del bus de datos (D-Bus) y la persistencia de la configuración en el sistema de archivos.

/// Módulo de persistencia y carga de la configuración de Raven.
pub mod config;
/// Módulo de comunicación por D-Bus para integración con el compositor.
pub mod dbus;
