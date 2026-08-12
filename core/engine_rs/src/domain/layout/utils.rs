//! # Módulo de Utilidades Geométricas de Layout
//!
//! Contiene funciones auxiliares para la manipulación de espaciados (gaps) y la
//! distribución proporcional de dimensiones sujetas a restricciones mínimas.

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

    // 1. Medida defensiva global: evitar que la suma de mínimos consuma todo el espacio útil
    let max_min_per_item = std::cmp::max(10, total / n as i32);
    let sanitized_mins: Vec<i32> = minimums.iter().map(|&m| m.min(max_min_per_item)).collect();

    // 2. Asignar reparto inicial equitativo y acumular el residuo en el último elemento
    let mut sizes = vec![total / n as i32; n];
    sizes[n - 1] += total % n as i32;

    // 3. Resolver iterativamente los déficits de tamaño mínimo
    let mut unresolved = true;
    while unresolved {
        unresolved = false;
        let mut deficit = 0;
        let mut flexible_count = 0;

        // Identificar elementos que no alcanzan su mínimo y los que pueden ceder espacio
        for i in 0..n {
            if sizes[i] < sanitized_mins[i] {
                deficit += sanitized_mins[i] - sizes[i];
                sizes[i] = sanitized_mins[i];
                unresolved = true;
            } else if sizes[i] > sanitized_mins[i] {
                flexible_count += 1;
            }
        }

        // Deducir proporcionalmente el déficit de los elementos con margen disponible
        if deficit > 0 && flexible_count > 0 {
            let deduction = deficit / flexible_count;
            let mut remainder = deficit % flexible_count;
            for i in 0..n {
                if sizes[i] > sanitized_mins[i] {
                    let mut take = deduction;
                    if remainder > 0 {
                        take += 1;
                        remainder -= 1;
                    }
                    let actual_take = std::cmp::min(take, sizes[i] - sanitized_mins[i]);
                    sizes[i] -= actual_take;
                }
            }
        } else if deficit > 0 {
            // Si el déficit no se puede reducir más, finalizar para evitar bucles infinitos
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

    let max_min_per_item = std::cmp::max(10, total / n as i32);
    let sanitized_mins: Vec<i32> = minimums.iter().map(|&m| m.min(max_min_per_item)).collect();

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
    while unresolved {
        unresolved = false;
        let mut deficit = 0;
        let mut flexible_count = 0;

        for i in 0..n {
            if sizes[i] < sanitized_mins[i] {
                deficit += sanitized_mins[i] - sizes[i];
                sizes[i] = sanitized_mins[i];
                unresolved = true;
            } else if sizes[i] > sanitized_mins[i] {
                flexible_count += 1;
            }
        }

        if deficit > 0 && flexible_count > 0 {
            let deduction = deficit / flexible_count;
            let mut remainder = deficit % flexible_count;
            for i in 0..n {
                if sizes[i] > sanitized_mins[i] {
                    let mut take = deduction;
                    if remainder > 0 {
                        take += 1;
                        remainder -= 1;
                    }
                    let actual_take = std::cmp::min(take, sizes[i] - sanitized_mins[i]);
                    sizes[i] -= actual_take;
                }
            }
        } else if deficit > 0 {
            break;
        }
    }
    sizes
}
