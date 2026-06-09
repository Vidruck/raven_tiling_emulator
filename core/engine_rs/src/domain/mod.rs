//! # Capa de Dominio
//!
//! Contiene los modelos de negocio, algoritmos de disposición (layout) y
//! las reglas fundamentales del motor de mosaico (tiling), aislados de cualquier
//! dependencia externa.

/// Módulo que define las acciones intencionales del motor sobre el compositor.
pub mod action;
/// Módulo de definición de errores del dominio de negocio de Raven.
pub mod error;
/// Módulo de geometría y estructuras de datos fundamentales de pantalla y ventana.
pub mod geometry;
/// Módulo de algoritmos de disposición de ventanas en mosaico (tiling layout).
pub mod layout;
/// Módulo de presets semánticos de composición de layout.
pub mod preset;
/// Módulo para la gestión y detección de saturación en el sistema.
pub mod saturation;
