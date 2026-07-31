use crate::domain::geometry::Rect;

/// Aplica un espaciado (gap) interno a un rectángulo.
#[inline(always)]
pub(crate) fn apply_gaps(rect: &Rect, gap: i32) -> Rect {
    Rect {
        x: rect.x + gap,
        y: rect.y + gap,
        width: std::cmp::max(1, rect.width - (2 * gap)),
        height: std::cmp::max(1, rect.height - (2 * gap)),
    }
}

/// Distribuye un total de espacio lineal entre N elementos respetando sus tamaños mínimos.
pub(crate) fn distribute_sizes(total: i32, minimums: &[i32]) -> Vec<i32> {
    let n = minimums.len();
    if n == 0 { return vec![]; }

    // Medida defensiva global: evitar que la suma de mínimos consuma todo el espacio útil
    let max_min_per_item = std::cmp::max(10, total / n as i32);
    let sanitized_mins: Vec<i32> = minimums.iter().map(|&m| m.min(max_min_per_item)).collect();

    let mut sizes = vec![total / n as i32; n];
    sizes[n - 1] += total % n as i32;

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
                    if remainder > 0 { take += 1; remainder -= 1; }
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
