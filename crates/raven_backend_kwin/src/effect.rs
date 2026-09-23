//! # Cliente D-Bus para el Efecto Nativo de KWin (`RavenEffectClient`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Provee comunicación directa con el plugin de efecto `kwin4_effect_raven` (`org.kde.kwin.RavenEffect`)
//! permitiendo animaciones de movimiento y apertura sin pasar por el sandbox de `QJSEngine`.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};
use zbus::Connection;

/// Descriptor de animación geométrica para una ventana.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowAnimation {
    /// ID de la ventana en KWin.
    pub id: String,
    /// Rectángulo origen `[x, y, w, h]`.
    pub from: [f64; 4],
    /// Rectángulo destino `[x, y, w, h]`.
    pub to: [f64; 4],
    /// Duración en milisegundos.
    pub duration_ms: u32,
    /// Curva de interpolación (ej. `"EaseOutCubic"`, `"EaseOutExpo"`).
    pub easing: String,
}

/// Payload por lote de animaciones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnimationBatchPayload {
    pub animations: Vec<WindowAnimation>,
}

/// Cliente D-Bus para enviar órdenes de renderizado al efecto C++ de KWin.
#[derive(Clone, Default)]
pub struct RavenEffectClient {
    connection: Arc<RwLock<Option<Arc<Connection>>>>,
}

impl RavenEffectClient {
    /// Crea una nueva instancia de `RavenEffectClient`.
    pub fn new() -> Self {
        Self {
            connection: Arc::new(RwLock::new(None)),
        }
    }

    /// Asigna la conexión D-Bus de sesión activa.
    pub async fn set_connection(&self, conn: Arc<Connection>) {
        let mut guard = self.connection.write().await;
        *guard = Some(conn);
    }

    /// Envía un lote de animaciones de geometría al efecto nativo.
    pub async fn animate_batch(&self, animations: Vec<WindowAnimation>) {
        if animations.is_empty() {
            return;
        }

        let payload = AnimationBatchPayload { animations };
        let json = match serde_json::to_string(&payload) {
            Ok(j) => j,
            Err(e) => {
                warn!("[EFFECT-CLIENT] Error al serializar AnimationBatch: {}", e);
                return;
            }
        };

        let conn_opt = self.connection.read().await.clone();
        if let Some(conn) = conn_opt {
            tokio::spawn(async move {
                let call_res = conn
                    .call_method(
                        Some("org.kde.kwin.RavenEffect"),
                        "/Effects/Raven",
                        Some("org.kde.kwin.RavenEffect"),
                        "AnimateBatch",
                        &(json,),
                    )
                    .await;

                if let Err(e) = call_res {
                    debug!("[EFFECT-CLIENT] Fallo al invocar AnimateBatch (posiblemente efecto inactivo/no cargado): {}", e);
                }
            });
        }
    }

    /// Envía orden de animación de nacimiento (Zoom/Fade) para una ventana nueva.
    pub async fn animate_birth(&self, window_id: &str, duration_ms: u32) {
        let wid = window_id.to_string();
        let conn_opt = self.connection.read().await.clone();
        if let Some(conn) = conn_opt {
            tokio::spawn(async move {
                let call_res = conn
                    .call_method(
                        Some("org.kde.kwin.RavenEffect"),
                        "/Effects/Raven",
                        Some("org.kde.kwin.RavenEffect"),
                        "AnimateBirth",
                        &(&wid, duration_ms as i32),
                    )
                    .await;

                if let Err(e) = call_res {
                    debug!("[EFFECT-CLIENT] Fallo al invocar AnimateBirth: {}", e);
                }
            });
        }
    }

    /// Cancela animaciones pendientes para una ventana específica.
    pub async fn cancel_animation(&self, window_id: &str) {
        let wid = window_id.to_string();
        let conn_opt = self.connection.read().await.clone();
        if let Some(conn) = conn_opt {
            tokio::spawn(async move {
                let _ = conn
                    .call_method(
                        Some("org.kde.kwin.RavenEffect"),
                        "/Effects/Raven",
                        Some("org.kde.kwin.RavenEffect"),
                        "CancelAnimation",
                        &(&wid,),
                    )
                    .await;
            });
        }
    }
}
