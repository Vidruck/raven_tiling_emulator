//! # Utilidades Geométricas y Distribución Ponderada
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Provee funciones auxiliares de bajo nivel para la aplicación simétrica de márgenes
//! (`apply_gaps`) y el algoritmo de distribución de dimensiones con pesos relativos
//! y restricciones mínimas innegociables (`distribute_weighted_sizes`).

use crate::domain::geometry::Rect;

/// Aplica un espaciado interno (gap) a un rectángulo de ventana.
///
/// Reduce el tamaño del rectángulo en `2 * gap` píxeles tanto horizontal como verticalmente
/// y desplaza las coordenadas `(x, y)` por `gap` píxeles hacia adentro.
///
/// # Parámetros
/// - `rect`: Rectángulo original del contenedor.
/// - `gap`: Cantidad de píxeles de margen interno a aplicar.
///
/// # Retorno
/// Un nuevo objeto `Rect` ajustado con el espaciado aplicado.
#[inline(always)]
pub(crate) fn apply_gaps(rect: &Rect, gap: i32) -> Rect {
    Rect {
        x: rect.x + gap,
        y: rect.y + gap,
        width: std::cmp::max(1, rect.width - (2 * gap)),
        height: std::cmp::max(1, rect.height - (2 * gap)),
    }
}

/// Distribuye un total de espacio lineal (ancho o alto) entre $N$ elementos respetando sus tamaños mínimos.
///
/// Realiza una asignación equitativa inicial y luego ajusta dinámicamente el espacio sobrante
/// o déficit entre los elementos flexibles para evitar colapsos o encimamientos.
///
/// # Parámetros
/// - `total`: Dimensión total en píxeles disponible para repartir.
/// - `minimums`: Arreglo de dimensiones mínimas requeridas por cada elemento.
///
/// # Retorno
/// Un vector de enteros `Vec<i32>` con los anchos o alturas exactas asignadas a cada elemento.
pub(crate) fn distribute_sizes(total: i32, minimums: &[i32]) -> Vec<i32> {
    let n = minimums.len();
    if n == 0 {
        return vec![];
    }

    let sum_mins: i32 = minimums.iter().sum();
    // Si la suma total de mínimos excede el espacio disponible, escalamos proporcionalmente
    let sanitized_mins: Vec<i32> = if sum_mins > total && sum_mins > 0 {
        let scale = total as f32 / sum_mins as f32;
        minimums
            .iter()
            .map(|&m| std::cmp::max(1, (m as f32 * scale).floor() as i32))
            .collect()
    } else {
        minimums.to_vec()
    };

    // Asignar reparto inicial equitativo y acumular el residuo en el último elemento
    let mut sizes = vec![total / n as i32; n];
    let rem = total % n as i32;
    if let Some(last) = sizes.last_mut() {
        *last += rem;
    }

    // Resolver iterativamente los déficits de tamaño mínimo
    let mut unresolved = true;
    let mut iterations = 0;
    while unresolved && iterations < 16 {
        iterations += 1;
        unresolved = false;
        let mut deficit = 0;
        let mut flexible_indices = Vec::new();

        for i in 0..n {
            if sizes[i] < sanitized_mins[i] {
                deficit += sanitized_mins[i] - sizes[i];
                sizes[i] = sanitized_mins[i];
                unresolved = true;
            } else if sizes[i] > sanitized_mins[i] {
                flexible_indices.push(i);
            }
        }

        if deficit > 0 && !flexible_indices.is_empty() {
            let deduction = deficit / flexible_indices.len() as i32;
            let mut remainder = deficit % flexible_indices.len() as i32;
            for &idx in &flexible_indices {
                let mut take = deduction;
                if remainder > 0 {
                    take += 1;
                    remainder -= 1;
                }
                let actual_take = std::cmp::min(take, sizes[idx] - sanitized_mins[idx]);
                sizes[idx] -= actual_take;
            }
        } else if deficit > 0 {
            break;
        }
    }
    sizes
}

/// Distribuye un total de espacio lineal (ancho o alto) considerando pesos (weights) opcionales y mínimos.
pub(crate) fn distribute_weighted_sizes(
    total: i32,
    minimums: &[i32],
    weights: &[Option<f32>],
) -> Vec<i32> {
    let n = minimums.len();
    if n == 0 {
        return vec![];
    }
    if weights.len() != n || weights.iter().all(|w| w.is_none()) {
        return distribute_sizes(total, minimums);
    }

    let default_weight = 1.0f32;
    let effective_weights: Vec<f32> = weights
        .iter()
        .map(|w| w.unwrap_or(default_weight).max(0.1))
        .collect();

    let total_weight: f32 = effective_weights.iter().sum();
    if total_weight <= 0.0 {
        return distribute_sizes(total, minimums);
    }

    let sum_mins: i32 = minimums.iter().sum();
    let sanitized_mins: Vec<i32> = if sum_mins > total && sum_mins > 0 {
        let scale = total as f32 / sum_mins as f32;
        minimums
            .iter()
            .map(|&m| std::cmp::max(1, (m as f32 * scale).floor() as i32))
            .collect()
    } else {
        minimums.to_vec()
    };

    let mut sizes: Vec<i32> = effective_weights
        .iter()
        .map(|&w| ((total as f32) * (w / total_weight)).round() as i32)
        .collect();

    let current_sum: i32 = sizes.iter().sum();
    let diff = total - current_sum;
    if let Some(last) = sizes.last_mut() {
        *last += diff;
    }

    let mut unresolved = true;
    let mut iterations = 0;
    while unresolved && iterations < 16 {
        iterations += 1;
        unresolved = false;
        let mut deficit = 0;
        let mut flexible_indices = Vec::new();

        for i in 0..n {
            if sizes[i] < sanitized_mins[i] {
                deficit += sanitized_mins[i] - sizes[i];
                sizes[i] = sanitized_mins[i];
                unresolved = true;
            } else if sizes[i] > sanitized_mins[i] {
                flexible_indices.push(i);
            }
        }

        if deficit > 0 && !flexible_indices.is_empty() {
            let deduction = deficit / flexible_indices.len() as i32;
            let mut remainder = deficit % flexible_indices.len() as i32;
            for &idx in &flexible_indices {
                let mut take = deduction;
                if remainder > 0 {
                    take += 1;
                    remainder -= 1;
                }
                let actual_take = std::cmp::min(take, sizes[idx] - sanitized_mins[idx]);
                sizes[idx] -= actual_take;
            }
        } else if deficit > 0 {
            break;
        }
    }
    sizes
}
