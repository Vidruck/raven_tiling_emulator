//! # Guardia de Oscilación y Anti-Flapping (`flap_guard`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Rastrea y penaliza ventanas rebeldes que entran en bucles infinitos de redimensionamiento
//! debido a conflictos entre su tamaño mínimo forzado por el cliente y el layout del motor.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::warn;

use crate::domain::geometry::{Rect, WindowNode};

/// Registro individual de oscilación por ventana.
#[derive(Debug, Clone)]
struct FlapTracker {
    last_toggle_time: u64,
    toggle_count: u64,
    is_penalized: bool,
    last_rect: Option<Rect>,
    last_output: Option<String>,
    last_minimized: bool,
}

/// Detector y mitigador determinista de oscilación geométrica rápida.
#[derive(Default)]
pub struct FlapGuard {
    registry: HashMap<String, FlapTracker>,
}

impl FlapGuard {
    /// Crea una nueva instancia de `FlapGuard`.
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
        }
    }

    /// Limpia el registro completo.
    pub fn clear(&mut self) {
        self.registry.clear();
    }

    /// Remueve una ventana del registro (por ejemplo, al cerrarse o migrar).
    pub fn remove(&mut self, window_id: &str) {
        self.registry.remove(window_id);
    }

    /// Evalúa si la ventana está en un bucle de oscilación y actualiza su penalización.
    pub fn is_window_flapping(&mut self, win: &WindowNode) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let tracker = self
            .registry
            .entry(win.window_id.clone())
            .or_insert_with(|| FlapTracker {
                last_toggle_time: now,
                toggle_count: 0,
                is_penalized: false,
                last_rect: Some(win.geometry),
                last_output: Some(win.output.clone()),
                last_minimized: win.is_minimized,
            });

        // Si la ventana cambió de monitor, el salto de coordenadas es legítimo
        if tracker.last_output.as_ref() != Some(&win.output) {
            tracker.last_output = Some(win.output.clone());
            tracker.last_rect = Some(win.geometry);
            tracker.toggle_count = 0;
            tracker.is_penalized = false;
            return false;
        }

        if win.is_minimized != tracker.last_minimized {
            tracker.last_minimized = win.is_minimized;
            tracker.toggle_count = 0;
            return false;
        }

        if tracker.is_penalized {
            if now - tracker.last_toggle_time > 1500 {
                tracker.is_penalized = false;
                tracker.toggle_count = 0;
                warn!(
                    "[FlapGuard] Ventana {} liberada de penalización.",
                    win.window_id
                );
            } else {
                return true;
            }
        }

        let is_jumping = match tracker.last_rect {
            Some(old_r) => {
                let dx = (old_r.x - win.geometry.x).abs();
                let dy = (old_r.y - win.geometry.y).abs();
                let dw = (old_r.width - win.geometry.width).abs();
                let dh = (old_r.height - win.geometry.height).abs();
                dx > 10 || dy > 10 || dw > 10 || dh > 10
            }
            None => false,
        };

        tracker.last_rect = Some(win.geometry);

        if is_jumping {
            if now - tracker.last_toggle_time < 200 {
                tracker.toggle_count += 1;
                if tracker.toggle_count >= 8 {
                    tracker.is_penalized = true;
                    warn!(
                        "[FlapGuard] Ventana {} penalizada por oscilación (flap detectado).",
                        win.window_id
                    );
                    return true;
                }
            } else {
                tracker.toggle_count = 1;
            }
            tracker.last_toggle_time = now;
        }

        false
    }
}
