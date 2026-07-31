//! # Algoritmos de Layout - Versión 3.0
//!
//! Submódulo orquestador para el cálculo de la disposición de las ventanas en mosaico (tiling).
//! Coordina las estrategias de espacio binario (BSP), algoritmos individuales, topología global
//! y ventanas emergentes (PiP).

pub mod divisor;
pub mod dwindle_bsp;
pub mod monocle;
pub mod strategy;
pub mod strict_dwindle;
pub mod tall;
pub mod topology;
pub mod utils;

pub use divisor::DivisorStrategy;
pub use dwindle_bsp::DwindleBSPStrategy;
pub use monocle::MonocleStrategy;
pub use strategy::{get_strategy, LayoutStrategy};
pub use strict_dwindle::StrictDwindleStrategy;
pub use tall::TallStrategy;
pub use topology::calculate_global_topology;
pub(crate) use utils::{apply_gaps, distribute_sizes};
