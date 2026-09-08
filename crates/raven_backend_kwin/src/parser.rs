//! # Serialización y Deserialización de Payloads KWin (`parser`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 4.0  
//! **Licencia:** GPL-3.0  
//!
//! Estructuras de datos para parsear las llamadas de sincronización emitidas por el script de KWin.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use raven_core::geometry::{Rect, Topology, WindowNode};

/// Representa la geometría de una pantalla en la estructura de serialización de KWin.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KWinScreen {
    /// Posición en X.
    pub x: i32,
    /// Posición en Y.
    pub y: i32,
    /// Ancho en píxeles.
    pub w: i32,
    /// Alto en píxeles.
    pub h: i32,
}

/// Representa una ventana individual recibida en el payload de sincronización de KWin.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct KWinWindow {
    /// Identificador único de la ventana en KWin.
    pub id: String,
    /// Identificador del espacio de trabajo compuesto ("Output||DesktopId").
    #[serde(default)]
    pub ws: String,
    /// Salida (output) física de la ventana.
    #[serde(default)]
    pub output: String,
    /// Lista de escritorios virtuales a los que pertenece la ventana.
    #[serde(default)]
    pub desktops: Vec<String>,
    /// Indica si la ventana es flotante.
    #[serde(default)]
    pub f: bool,
    /// Indica si la ventana está minimizada.
    #[serde(default)]
    pub m: bool,
    /// Indica si la ventana está en modo Picture-in-Picture o keepAbove.
    #[serde(default)]
    pub p: bool,
    /// Coordenada horizontal actual.
    pub x: i32,
    /// Coordenada vertical actual.
    pub y: i32,
    /// Ancho en píxeles.
    pub w: i32,
    /// Alto en píxeles.
    pub h: i32,
    /// Ancho mínimo admitido por la ventana.
    #[serde(default)]
    pub min_w: i32,
    /// Alto mínimo admitido por la ventana.
    #[serde(default)]
    pub min_h: i32,
    /// Indica si la ventana requiere retroalimentación (feedback) de sincronización inmediata tras su creación.
    #[serde(default)]
    pub sb: bool,
    /// Indica si la ventana se encuentra en cuarentena de estabilización (Gecko/CSD).
    #[serde(default)]
    pub iq: bool,
    /// Indica si la ventana está en modo pantalla completa nativa.
    #[serde(default)]
    pub fs: bool,
    /// Clase WM / Resource class reportada por KWin (ej. "firefox", "vlc").
    #[serde(default)]
    pub cls: String,
    /// Título / Caption reportado por KWin.
    #[serde(default)]
    pub cap: String,
}

/// Representa el estado global de salidas y escritorios virtuales en KWin (alias de la topología universal de raven_core).
pub type KWinTopology = Topology;

/// Contenedor raíz del payload de sincronización enviado por el script de KWin.
#[derive(Debug, Deserialize, Clone)]
pub struct KWinPayload {
    /// Listado de ventanas reportadas por el compositor.
    pub windows: Vec<KWinWindow>,
    /// Mapa de geometrías de pantalla indexadas por identificador de área de trabajo.
    pub screens: HashMap<String, KWinScreen>,
    /// Topología de pantallas y escritorios virtuales de KWin.
    #[serde(default)]
    pub topology: Topology,
}

/// Resultado del parseo del payload de KWin: (Workspaces, Ventanas, Topología).
pub type ParsedKWinPayload = (HashMap<String, Rect>, Vec<WindowNode>, Topology);

/// Parsea el payload JSON de KWin convirtiéndolo en estructuras nativas de `raven_core`.
pub fn parse_payload(
    payload_str: &str,
) -> Result<ParsedKWinPayload, serde_json::Error> {
    let payload: KWinPayload = serde_json::from_str(payload_str)?;
    let mut workspaces = HashMap::new();
    for (ws_id, screen) in &payload.screens {
        workspaces.insert(
            ws_id.clone(),
            Rect::new(screen.x, screen.y, screen.w, screen.h),
        );
    }
    let mut windows = Vec::new();
    for win in payload.windows {
        let ws_id = if !win.ws.is_empty() {
            win.ws
        } else {
            let out_name = if !win.output.is_empty() {
                win.output.as_str()
            } else {
                "default"
            };
            let desk_name = win.desktops.first().map(|d| d.as_str()).unwrap_or("default_desk");
            format!("{}||{}", out_name, desk_name)
        };

        windows.push(
            WindowNode::new(
                win.id,
                ws_id,
                win.output,
                win.desktops,
                win.f,
                win.m,
                win.p,
                Rect::new(win.x, win.y, win.w, win.h),
                win.min_w,
                win.min_h,
                win.sb,
                win.iq,
                win.fs,
            )
            .with_class_and_caption(win.cls, win.cap),
        );
    }

    let mut topology = payload.topology;
    // Si la topología no trae output_nodes poblados, derivarlos de screens y outputs conocidos
    if topology.output_nodes.is_empty() {
        let mut output_nodes = Vec::new();
        for out in &topology.outputs {
            // Buscar si hay alguna pantalla en payload.screens que corresponda al output
            let matching_rect = payload.screens.iter().find_map(|(ws_id, screen)| {
                if ws_id.starts_with(out) {
                    Some(Rect::new(screen.x, screen.y, screen.w, screen.h))
                } else {
                    None
                }
            });

            let rect = matching_rect.unwrap_or_else(|| Rect::new(0, 0, 1920, 1080));
            output_nodes.push(raven_core::geometry::OutputNode::new(out.clone(), rect, 1.0));
        }
        topology.output_nodes = output_nodes;
    }

    Ok((workspaces, windows, topology))
}
