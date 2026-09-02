//! # Raven Engine (`raven_engine`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 3.4  
//! **Licencia:** GPL-3.0  
//!
//! Motor central y demonio de alto rendimiento para el emulador de mosaico Raven.
//! Diseñado bajo los principios de la **Arquitectura Hexagonal (Puertos y Adaptadores)**
//! para garantizar el desacoplamiento total entre la lógica geométrica matemática y la infraestructura D-Bus / KWin.
//!
//! ## Estructura de Capas
//! - [`application`]: Orquestación, controladores de estado y modelo de concurrencia basado en actores (`RavenControllerActor`).
//! - [`domain`]: Algoritmos matemáticos de partición espacial, árboles binarios (BSP), mediador de capacidad y mitigación de saturación.
//! - [`infrastructure`]: Enlace con D-Bus (`zbus`) y almacenamiento persistente de configuración.
//! - [`ports`]: Contratos, rasgos (`traits`) y definiciones de interfaces desacopladas.

/// Capa de aplicación que contiene orquestadores, controladores y actores de concurrencia.
pub mod application;
/// Capa de dominio que contiene modelos espaciales y algoritmos matemáticos de disposición.
pub mod domain;
/// Capa de infraestructura que maneja comunicación D-Bus asíncrona y configuración.
pub mod infrastructure;
/// Capa de puertos que define interfaces y contratos desacoplados.
pub mod ports;
