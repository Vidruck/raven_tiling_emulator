//! # Estrategia de Layout Scriptable (Lua)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)
//! **Versión:** 4.0
//! **Licencia:** GPL-3.0
//!
//! Ejecuta algoritmos de layout definidos por el usuario en Lua mediante una máquina virtual
//! embebida ligera (`mlua`). Esto permite extender la capacidad matemática de Raven sin necesidad
//! de recompilar el motor en Rust, brindando flexibilidad absoluta al usuario.

use super::strategy::LayoutStrategy;
use mlua::{Lua, Table};
use raven_core::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Estrategia de partición espacial que delega la matemática a un script Lua externo.
///
/// Carga una máquina virtual embebida por cada ciclo de renderizado, inyecta la geometría
/// de la pantalla y la lista de ventanas, y captura la tabla resultante generada por el usuario.
pub struct LuaLayoutStrategy {
    /// Nombre base del layout (y del archivo sin la extensión `.lua`).
    pub name: String,
    /// Código fuente completo del script de Lua almacenado en memoria para ejecución rápida.
    pub script: String,
}

impl LuaLayoutStrategy {
    /// Instancia una nueva estrategia de Lua almacenando su código fuente.
    ///
    /// # Parámetros
    /// - `name`: Identificador único del script.
    /// - `script`: Contenido textual del archivo `.lua`.
    pub fn new(name: String, script: String) -> Self {
        Self { name, script }
    }
}

impl LayoutStrategy for LuaLayoutStrategy {
    /// Intercepta el cálculo nativo y lo delega a la máquina virtual de Lua.
    ///
    /// Transforma las estructuras `Rect` y `WindowNode` en tablas de Lua para que el usuario
    /// pueda iterarlas, y luego parsea los resultados de vuelta a `HashMap<String, Rect>`.
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        _nmaster: usize,
        _master_ratio: f32,
        _default_gaps: i32,
        _active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        let mut layout_map = HashMap::new();
        let mut evicted_windows = Vec::new();

        if windows.is_empty() {
            return (layout_map, evicted_windows);
        }

        // Crear nueva VM de Lua (muy ligero)
        let lua = Lua::new();

        // Preparar pantalla
        let screen_tbl = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return (layout_map, evicted_windows),
        };
        let _ = screen_tbl.set("x", screen_rect.x);
        let _ = screen_tbl.set("y", screen_rect.y);
        let _ = screen_tbl.set("w", screen_rect.width);
        let _ = screen_tbl.set("h", screen_rect.height);

        // Preparar array de ventanas
        let wins_tbl = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return (layout_map, evicted_windows),
        };
        for (i, w) in windows.iter().enumerate() {
            let _ = wins_tbl.set(i + 1, w.window_id.clone());
        }
        tracing::info!("LuaLayout: Enviando {} ventanas a Lua", windows.len());

        // Cargar el script
        let chunk = lua.load(&self.script);

        // Ejecutar el script que debe retornar una función
        let func: mlua::Function = match chunk.eval() {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Error al evaluar script Lua '{}': {}", self.name, e);
                return (layout_map, evicted_windows);
            }
        };

        // Llamar a la función Lua con (screen, windows)
        let result_tbl: Table = match func.call((screen_tbl, wins_tbl)) {
            Ok(t) => {
                tracing::info!("LuaLayout: Ejecución de script completada.");
                t
            }
            Err(e) => {
                tracing::error!("Error al ejecutar layout Lua '{}': {}", self.name, e);
                return (layout_map, evicted_windows);
            }
        };

        let mut results_count = 0;
        // Parsear resultado: un mapa de id_ventana -> {x, y, w, h}
        for pair in result_tbl.pairs::<String, Table>() {
            results_count += 1;
            if let Ok((win_id, rect_tbl)) = pair {
                if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                    rect_tbl.get::<_, f64>("x"),
                    rect_tbl.get::<_, f64>("y"),
                    rect_tbl.get::<_, f64>("w"),
                    rect_tbl.get::<_, f64>("h"),
                ) {
                    layout_map.insert(
                        win_id.clone(),
                        Rect::new(x as i32, y as i32, w as i32, h as i32),
                    );
                } else {
                    tracing::error!(
                        "LuaLayout: Faltan coordenadas (x,y,w,h) válidas para {}",
                        win_id
                    );
                }
            } else {
                tracing::error!(
                    "LuaLayout: La tabla de resultados tiene claves inválidas (no son strings)"
                );
            }
        }

        tracing::info!(
            "LuaLayout: Parseadas {} posiciones retornadas de Lua",
            results_count
        );

        // Ventanas no retornadas son eviccionadas
        for w in windows.iter() {
            if !layout_map.contains_key(&w.window_id) && !w.is_floating && !w.is_minimized {
                evicted_windows.push(w.window_id.clone());
            }
        }

        (layout_map, evicted_windows)
    }
}
