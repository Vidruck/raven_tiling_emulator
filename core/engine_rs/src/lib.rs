//! # Raven Core - Motor de Mosaico (Tiling) Nativo
//!
//! Esta librería implementa la lógica central del gestor de mosaico Raven.
//! Sigue una arquitectura hexagonal para garantizar el desacoplamiento entre
//! la lógica de negocio y los detalles de infraestructura.

/// Capa de aplicación que contiene orquestadores y controladores.
pub mod application;
/// Capa de dominio que contiene modelos y algoritmos matemáticos de disposición.
pub mod domain;
/// Capa de infraestructura que maneja comunicación D-Bus y configuración.
pub mod infrastructure;
/// Capa de puertos que define interfaces y contratos.
pub mod ports;
