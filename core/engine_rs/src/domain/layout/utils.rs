//! # Utilidades Geométricas y Distribución Ponderada
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Provee funciones auxiliares de bajo nivel para la aplicación simétrica de márgenes
//! (`apply_gaps`) y el algoritmo de distribución de dimensiones con pesos relativos
//! y restricciones mínimas innegociables (`distribute_weighted_sizes`).

use crate::domain::geometry::{Rect, WindowNode};

/// Determina si una ventana pertenece a un navegador web o aplicación basada en motor web/Electron
/// que reporta dimensiones mínimas infladas (`minSize`) por barras de pestañas o marcadores,
/// pero que es perfectamente capaz de renderizar en divisiones 50/50 o menores.
///
/// # Parámetros
/// - `win`: Nodo de ventana a evaluar.
///
/// # Retorno
/// `true` si la ventana es un navegador o cliente web flexible; `false` en caso contrario.
#[inline]
pub(crate) fn is_flexible_browser_or_webapp(win: &WindowNode) -> bool {
    let cls = win.resource_class.to_ascii_lowercase();
    let name = win.resource_name.to_ascii_lowercase();
    const FLEXIBLE_PATTERNS: &[&str] = &[
        "firefox",
        "zen",
        "librewolf",
        "waterfox",
        "floorp",
        "chrome",
        "chromium",
        "brave",
        "vivaldi",
        "edge",
        "msedge",
        "opera",
        "thorium",
        "falkon",
        "qutebrowser",
        "epiphany",
        "midori",
        "ladybird",
        "electron",
        "code",
        "codium",
        "discord",
        "vesktop",
        "spotify",
        "slack",
        "teams",
        "obsidian",
        "ferdium",
        "telegram",
    ];
    FLEXIBLE_PATTERNS
        .iter()
        .any(|p| cls.contains(p) || name.contains(p))
}

/// Acota defensivamente el requerimiento de ancho mínimo (`min_w`) de una ventana para el cálculo de mosaico.
///
/// Evita que navegadores web o aplicaciones CSD que anuncian un `minSize` excesivo (ej. 900px-1350px)
/// rompan la simetría 50/50 de dos ventanas o secuestren la proporción `master_ratio` llevándola a 70/30.
///
/// # Parámetros
/// - `win`: Nodo de ventana cuyo `min_w` se desea sanitizar.
/// - `container_width`: Ancho total útil del contenedor o área de pantalla en píxeles.
/// - `num_columns`: Número de columnas activas en paralelo dentro del contenedor.
///
/// # Retorno
/// El ancho mínimo efectivo y seguro en píxeles que no distorsiona la composición.
#[inline]
pub(crate) fn clamp_min_width(win: &WindowNode, container_width: i32, num_columns: usize) -> i32 {
    if win.min_w <= 0 {
        return 0;
    }
    if num_columns <= 1 {
        return win.min_w.clamp(0, container_width.max(1));
    }
    let fair_share = (container_width / num_columns as i32).max(1);
    if is_flexible_browser_or_webapp(win) {
        return win.min_w.min(240).min(fair_share);
    }
    let max_allowed = ((container_width as f32 * 0.35) as i32)
        .min(fair_share)
        .clamp(80, 380);
    win.min_w.min(max_allowed)
}

/// Acota defensivamente el requerimiento de altura mínima (`min_h`) de una ventana para el cálculo de mosaico.
///
/// Garantiza que ninguna ventana apilada verticalmente desplace desproporcionadamente a sus vecinas
/// por reportar alturas mínimas infladas al compositor.
///
/// # Parámetros
/// - `win`: Nodo de ventana cuyo `min_h` se desea sanitizar.
/// - `container_height`: Altura total útil del contenedor o columna en píxeles.
/// - `num_rows`: Número de filas apiladas verticalmente en la columna.
///
/// # Retorno
/// La altura mínima efectiva y segura en píxeles.
#[inline]
pub(crate) fn clamp_min_height(win: &WindowNode, container_height: i32, num_rows: usize) -> i32 {
    if win.min_h <= 0 {
        return 0;
    }
    if num_rows <= 1 {
        return win.min_h.clamp(0, container_height.max(1));
    }
    let fair_share = (container_height / num_rows as i32).max(1);
    if is_flexible_browser_or_webapp(win) {
        return win.min_h.min(200).min(fair_share);
    }
    let max_allowed = ((container_height as f32 * 0.35) as i32)
        .min(fair_share)
        .clamp(80, 320);
    win.min_h.min(max_allowed)
}

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
