//! # Estado interno y eventos de Wayland
//!
//! **Autor:** Alejandro González Hernández (Vidruck)
//! **Licencia:** GPL-3.0

use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, info};
use wayland_client::protocol::wl_output::{self, WlOutput};
use wayland_client::protocol::wl_registry;
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

use raven_core::backend::CompositorEvent;
use raven_core::geometry::{OutputNode, Rect, Topology, WindowNode};

/// Contiene la información extraída de una salida física o virtual (`wl_output`) descubierta en el compositor.
///
/// Proporciona detalles como nombre (usualmente el identificador del hardware, e.g., "eDP-1"),
/// la geometría actual (resolución física en el sistema de coordenadas globales) y el factor de escala UI.
#[derive(Debug, Clone)]
pub struct OutputInfo {
    pub name: String,
    pub description: String,
    pub geometry: Rect,
    pub scale: i32,
}

impl Default for OutputInfo {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            geometry: Rect::new(0, 0, 1920, 1080),
            scale: 1,
        }
    }
}

/// Mantiene el estado en tiempo real de una ventana gestionada por `zwlr_foreign_toplevel_manager_v1`.
///
/// Almacena propiedades descriptivas como el título de la ventana y el identificador de aplicación (`app_id`),
/// junto con indicadores booleanos que reflejan su estado (maximizado, minimizado, foco, fullscreen).
#[derive(Debug, Clone, Default)]
pub struct ToplevelInfo {
    pub title: String,
    pub app_id: String,
    pub is_maximized: bool,
    pub is_minimized: bool,
    pub is_activated: bool,
    pub is_fullscreen: bool,
    pub output_name: Option<String>,
}

/// Estado global de conexión Wayland gestionado de forma síncrona por el bucle de eventos.
///
/// Actúa como un contenedor mutable (`Dispatch handler`) para todos los objetos Wayland. 
/// Registra y actualiza las salidas físicas (`outputs`) y las ventanas activas (`toplevels`).
/// Permite inyectar un transmisor (`event_tx`) para emitir variaciones de estado asíncronamente
/// hacia el controlador (e.g., `TopologyChanged`).
pub struct WaylandState {
    pub outputs: HashMap<WlOutput, OutputInfo>,
    pub toplevel_manager: Option<ZwlrForeignToplevelManagerV1>,
    pub toplevels: HashMap<ZwlrForeignToplevelHandleV1, ToplevelInfo>,
    pub event_tx: Option<mpsc::Sender<CompositorEvent>>,
}

impl WaylandState {
    pub fn new(event_tx: Option<mpsc::Sender<CompositorEvent>>) -> Self {
        Self {
            outputs: HashMap::new(),
            toplevel_manager: None,
            toplevels: HashMap::new(),
            event_tx,
        }
    }

    /// Construye y devuelve la topología espacial actual basada en los datos recopilados de Wayland.
    ///
    /// Transforma el diccionario de `wl_output` en una estructura genérica [`Topology`], 
    /// la cual es digerida directamente por el `TilingEngine` de `raven_core` para su distribución.
    pub fn build_topology(&self) -> Topology {
        let mut output_nodes = Vec::new();
        let mut outputs = Vec::new();

        for info in self.outputs.values() {
            let name = if info.name.is_empty() {
                "unknown".to_string()
            } else {
                info.name.clone()
            };
            outputs.push(name.clone());
            output_nodes.push(OutputNode {
                name,
                rect: info.geometry,
                scale: if info.scale > 0 { info.scale as f64 } else { 1.0 },
            });
        }

        Topology {
            outputs,
            output_nodes,
            desktops: vec!["default".to_string()],
            current_desktop: "default".to_string(),
        }
    }
}

// ── Registry Dispatch ──

/// Implementación de despachador para el registro global de Wayland.
/// Escucha anuncios de interfaces globales y vincula protocolos esenciales como `wl_output`
/// y la extensión de manejo de ventanas `zwlr_foreign_toplevel_manager_v1`.
impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_output" => {
                    let out = registry.bind::<WlOutput, _, _>(name, version.min(4), qh, ());
                    state.outputs.insert(out, OutputInfo::default());
                    debug!("[WAYLAND-BACKEND] Descubierto wl_output id: {}", name);
                }
                "zwlr_foreign_toplevel_manager_v1" => {
                    let mgr = registry.bind::<ZwlrForeignToplevelManagerV1, _, _>(
                        name,
                        version.min(3),
                        qh,
                        (),
                    );
                    state.toplevel_manager = Some(mgr);
                    info!(
                        "[WAYLAND-BACKEND] Vinculado zwlr_foreign_toplevel_manager_v1 id: {}",
                        name
                    );
                }
                _ => {}
            }
        }
    }
}

// ── WlOutput Dispatch ──

/// Implementación de despachador para las salidas físicas o virtuales (`wl_output`).
/// Extrae la geometría posicional global, la resolución del modo actual y la escala UI,
/// emitiendo automáticamente un evento `CompositorEvent::TopologyChanged` al concluir un bloque de configuración (`Done`).
impl Dispatch<WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        output: &WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Some(info) = state.outputs.get_mut(output) {
            match event {
                wl_output::Event::Geometry {
                    x,
                    y,
                    physical_width: _,
                    physical_height: _,
                    subpixel: _,
                    make: _,
                    model: _,
                    transform: _,
                } => {
                    info.geometry.x = x;
                    info.geometry.y = y;
                }
                wl_output::Event::Mode {
                    flags,
                    width,
                    height,
                    refresh: _,
                } => {
                    let is_current = match flags {
                        WEnum::Value(mode_flags) => mode_flags.contains(wl_output::Mode::Current),
                        _ => false,
                    };
                    if is_current {
                        info.geometry.width = width;
                        info.geometry.height = height;
                    }
                }
                wl_output::Event::Name { name } => {
                    info.name = name;
                }
                wl_output::Event::Description { description } => {
                    info.description = description;
                }
                wl_output::Event::Scale { factor } => {
                    info.scale = factor;
                }
                wl_output::Event::Done => {
                    debug!(
                        "[WAYLAND-BACKEND] Configuración de monitor completa: {} -> {:?}",
                        info.name, info.geometry
                    );
                    if let Some(ref tx) = state.event_tx {
                        let topo = state.build_topology();
                        let _ = tx.blocking_send(CompositorEvent::TopologyChanged(topo));
                    }
                }
                _ => {}
            }
        }
    }
}

// ── Foreign Toplevel Manager Dispatch ──
impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } => {
                debug!("[WAYLAND-BACKEND] Nueva ventana toplevel descubierta");
                state.toplevels.insert(toplevel, ToplevelInfo::default());
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => {
                debug!("[WAYLAND-BACKEND] Foreign toplevel manager finalizado");
            }
            _ => {}
        }
    }
}

// ── Foreign Toplevel Handle Dispatch ──
impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let handle_id = format!("wlr_toplevel_{}", handle.id().protocol_id());

        match event {
            zwlr_foreign_toplevel_handle_v1::Event::Title { title } => {
                if let Some(info) = state.toplevels.get_mut(handle) {
                    info.title = title;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::AppId { app_id } => {
                if let Some(info) = state.toplevels.get_mut(handle) {
                    info.app_id = app_id;
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputEnter { output } => {
                if let Some(out_info) = state.outputs.get(&output) {
                    if let Some(info) = state.toplevels.get_mut(handle) {
                        info.output_name = Some(out_info.name.clone());
                    }
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::OutputLeave { .. } => {}
            zwlr_foreign_toplevel_handle_v1::Event::State { state: flags } => {
                if let Some(info) = state.toplevels.get_mut(handle) {
                    let u32_flags: &[u32] = bytemuck_cast_slice(&flags);
                    // 0 = maximized, 1 = minimized, 2 = activated, 3 = fullscreen
                    info.is_maximized = u32_flags.contains(&0);
                    info.is_minimized = u32_flags.contains(&1);
                    info.is_activated = u32_flags.contains(&2);
                    info.is_fullscreen = u32_flags.contains(&3);

                    if info.is_activated {
                        if let Some(ref tx) = state.event_tx {
                            let _ = tx.blocking_send(CompositorEvent::WindowFocused(Some(
                                handle_id,
                            )));
                        }
                    }
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Done => {
                if let Some(info) = state.toplevels.get(handle) {
                    let out_name = info
                        .output_name
                        .clone()
                        .unwrap_or_else(|| "default".to_string());
                    let ws_id = format!("{}||default", out_name);

                    let win_node = WindowNode::new(
                        handle_id,
                        ws_id,
                        out_name,
                        vec!["default".to_string()],
                        false,
                        info.is_minimized,
                        false,
                        Rect::new(0, 0, 800, 600),
                        100,
                        100,
                        false,
                        false,
                        info.is_fullscreen,
                    )
                    .with_class_and_caption(info.app_id.clone(), String::new(), false, info.title.clone());

                    if let Some(ref tx) = state.event_tx {
                        let _ = tx.blocking_send(CompositorEvent::WindowDiscovered(win_node));
                    }
                }
            }
            zwlr_foreign_toplevel_handle_v1::Event::Closed => {
                state.toplevels.remove(handle);
                if let Some(ref tx) = state.event_tx {
                    let _ = tx.blocking_send(CompositorEvent::WindowClosed(handle_id));
                }
            }
            _ => {}
        }
    }
}

/// Helper para convertir bytes en slice u32 de forma segura
fn bytemuck_cast_slice(bytes: &[u8]) -> &[u32] {
    if !bytes.len().is_multiple_of(4) {
        return &[];
    }
    unsafe {
        std::slice::from_raw_parts(bytes.as_ptr() as *const u32, bytes.len() / 4)
    }
}
