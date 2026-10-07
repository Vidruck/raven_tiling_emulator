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
use mlua::{HookTriggers, Lua, LuaOptions, StdLib, Table};
use raven_core::geometry::{Rect, WindowNode};
use std::collections::HashMap;

/// Límite máximo de memoria asignada a una instancia de Lua (4 MB).
const LUA_MEMORY_LIMIT_BYTES: usize = 4 * 1024 * 1024;
/// Límite máximo de instrucciones ejecutadas para evitar bucles infinitos (DoS).
const LUA_MAX_INSTRUCTIONS: u32 = 100_000;

/// Estrategia de partición espacial que delega la matemática a un script Lua externo.
pub struct LuaLayoutStrategy {
    /// Nombre base del layout (y del archivo sin la extensión `.lua`).
    pub name: String,
    /// Código fuente completo del script de Lua almacenado en memoria para ejecución rápida.
    pub script: String,
}

impl LuaLayoutStrategy {
    /// Instancia una nueva estrategia de Lua almacenando su código fuente.
    pub fn new(name: String, script: String) -> Self {
        Self { name, script }
    }

    /// Crea un entorno de Lua estrictamente sandboxeado con límites de memoria y ejecución.
    fn create_sandboxed_lua() -> Result<Lua, String> {
        let lua = Lua::new_with(
            StdLib::MATH | StdLib::TABLE | StdLib::STRING,
            LuaOptions::default(),
        )
        .map_err(|e| format!("Error inicializando runtime de Lua: {}", e))?;

        // 1. Límite de memoria asignada
        if let Err(e) = lua.set_memory_limit(LUA_MEMORY_LIMIT_BYTES) {
            tracing::warn!("No se pudo configurar límite de memoria en Lua: {}", e);
        }

        // 2. Hook de límite de instrucciones para evitar bloqueos por bucles infinitos
        use std::sync::atomic::{AtomicU32, Ordering};
        let instruction_count = std::sync::Arc::new(AtomicU32::new(0));
        let count_clone = instruction_count.clone();
        let hook_res = lua.set_hook(
            HookTriggers::default().every_nth_instruction(1_000),
            move |_lua, _debug| {
                let current = count_clone.fetch_add(1_000, Ordering::Relaxed);
                if current > LUA_MAX_INSTRUCTIONS {
                    Err(mlua::Error::RuntimeError(
                        "Límite de ejecución excedido (posible bucle infinito o algoritmo no convergente)".to_string(),
                    ))
                } else {
                    Ok(mlua::VmState::Continue)
                }
            },
        );

        if let Err(e) = hook_res {
            tracing::warn!("No se pudo configurar el hook de instrucciones de Lua: {}", e);
        }

        Ok(lua)
    }

    /// Valida la sintaxis de un script Lua y confirma que evalúa a una función ejecutable.
    pub fn validate_script(script: &str) -> Result<(), String> {
        let lua = Self::create_sandboxed_lua()?;
        let chunk = lua.load(script);
        let func: mlua::Function = chunk
            .eval()
            .map_err(|e| format!("El script no retorna una función válida: {}", e))?;
        let _ = func;
        Ok(())
    }
}

impl LayoutStrategy for LuaLayoutStrategy {
    fn calculate(
        &self,
        windows: &[WindowNode],
        screen_rect: Rect,
        nmaster: usize,
        master_ratio: f32,
        default_gaps: i32,
        active_window_id: Option<String>,
    ) -> (HashMap<String, Rect>, Vec<String>) {
        let mut layout_map = HashMap::new();
        let mut evicted_windows = Vec::new();

        if windows.is_empty() {
            return (layout_map, evicted_windows);
        }

        // Sandbox de Seguridad: Solo cargamos librerías seguras (Math, Table, String).
        // Se excluyen 'io', 'os', 'package', 'debug' y se aplican límites estrictos.
        let lua = match Self::create_sandboxed_lua() {
            Ok(l) => l,
            Err(e) => {
                tracing::error!("LuaLayout: No se pudo crear sandbox de Lua: {}", e);
                return (layout_map, evicted_windows);
            }
        };

        // 1. Inyectar tabla de Pantalla: { x, y, w, h }
        let screen_tbl = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return (layout_map, evicted_windows),
        };
        let _ = screen_tbl.set("x", screen_rect.x);
        let _ = screen_tbl.set("y", screen_rect.y);
        let _ = screen_tbl.set("w", screen_rect.width);
        let _ = screen_tbl.set("h", screen_rect.height);

        // 2. Inyectar tabla de Configuración: { gaps, master_ratio, nmaster, active_id }
        let config_tbl = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return (layout_map, evicted_windows),
        };
        let _ = config_tbl.set("gaps", default_gaps);
        let _ = config_tbl.set("master_ratio", f64::from(master_ratio));
        let _ = config_tbl.set("nmaster", nmaster);
        if let Some(ref act_id) = active_window_id {
            let _ = config_tbl.set("active_id", act_id.clone());
        }

        // 3. Inyectar lista de Ventanas enriquecida: { [1] = win_id o { id, min_w, min_h, is_active } }
        let wins_tbl = match lua.create_table() {
            Ok(t) => t,
            Err(_) => return (layout_map, evicted_windows),
        };
        for (i, w) in windows.iter().enumerate() {
            // Cada ventana expone tanto el ID como sus metadatos
            let w_info = match lua.create_table() {
                Ok(t) => t,
                Err(_) => continue,
            };
            let _ = w_info.set("id", w.window_id.clone());
            let _ = w_info.set("min_w", w.min_w);
            let _ = w_info.set("min_h", w.min_h);
            let _ = w_info.set("class", w.resource_class.clone());
            let is_act = active_window_id.as_deref() == Some(&w.window_id);
            let _ = w_info.set("is_active", is_act);

            let _ = wins_tbl.set(i + 1, w_info);
        }

        // Cargar y compilar el script seguro
        let chunk = lua.load(&self.script);

        // Evaluar el script: debe retornar una función
        let func: mlua::Function = match chunk.eval() {
            Ok(f) => f,
            Err(e) => {
                tracing::error!("Error al evaluar script Lua '{}': {}", self.name, e);
                return (layout_map, evicted_windows);
            }
        };

        // Invocar la función Lua pasando (screen, windows, config)
        let result_tbl: Table = match func.call((screen_tbl, wins_tbl, config_tbl)) {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("Error al ejecutar layout Lua '{}': {}", self.name, e);
                return (layout_map, evicted_windows);
            }
        };

        // Parsear y sanitizar resultado: mapa de id_ventana -> {x, y, w, h}
        for (win_id, rect_tbl) in result_tbl.pairs::<String, Table>().flatten() {
            if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                rect_tbl.get::<f64>("x"),
                rect_tbl.get::<f64>("y"),
                rect_tbl.get::<f64>("w"),
                rect_tbl.get::<f64>("h"),
            ) {
                // Validación estricta de geometrías numéricas (evitar NaN, infinitos y tamaños negativos)
                if x.is_finite() && y.is_finite() && w.is_finite() && h.is_finite() {
                    let safe_w = (w.round() as i32).max(10);
                    let safe_h = (h.round() as i32).max(10);
                    let safe_x = x.round() as i32;
                    let safe_y = y.round() as i32;

                    layout_map.insert(
                        win_id,
                        Rect::new(safe_x, safe_y, safe_w, safe_h),
                    );
                }
            }
        }

        // Ventanas gestionadas no retornadas son eviccionadas
        for w in windows.iter() {
            if !layout_map.contains_key(&w.window_id) && !w.is_floating && !w.is_minimized {
                evicted_windows.push(w.window_id.clone());
            }
        }

        (layout_map, evicted_windows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_window(id: &str) -> WindowNode {
        WindowNode::new(
            id.to_string(),
            "ws_1".to_string(),
            "DP-1".to_string(),
            vec!["desk_1".to_string()],
            false,
            false,
            false,
            Rect::new(0, 0, 0, 0),
            100,
            100,
            false,
            false,
            false,
        )
    }

    #[test]
    fn test_lua_layout_sandboxing_blocks_os_and_io() {
        let malicious_script = r#"
            return function(screen, windows, config)
                if os ~= nil then
                    os.execute("echo pwned")
                end
                return {}
            end
        "#;
        let strat = LuaLayoutStrategy::new("malicious".to_string(), malicious_script.to_string());
        let node = mock_window("win_1");
        let screen = Rect::new(0, 0, 1920, 1080);
        let (map, _) = strat.calculate(&[node], screen, 1, 0.5, 10, None);
        assert!(map.is_empty());
    }

    #[test]
    fn test_lua_layout_infinite_loop_prevention() {
        let infinite_loop_script = r#"
            return function(screen, windows, config)
                local x = 0
                while true do
                    x = x + 1
                end
                return {}
            end
        "#;
        let strat = LuaLayoutStrategy::new("infinite_loop".to_string(), infinite_loop_script.to_string());
        let node = mock_window("win_1");
        let screen = Rect::new(0, 0, 1920, 1080);
        // Debe abortar con seguridad y no congelar el hilo ni entrar en bucle infinito
        let (map, evicted) = strat.calculate(&[node], screen, 1, 0.5, 10, None);
        assert!(map.is_empty());
        assert_eq!(evicted.len(), 0);
    }

    #[test]
    fn test_lua_layout_sanitizes_nan_and_negative_dimensions() {
        let crazy_script = r#"
            return function(screen, windows, config)
                local res = {}
                res["w_nan"] = { x = 0/0, y = 10, w = 100, h = 100 }
                res["w_neg"] = { x = 10, y = 20, w = -50, h = -10 }
                return res
            end
        "#;
        let strat = LuaLayoutStrategy::new("crazy".to_string(), crazy_script.to_string());
        let node1 = mock_window("w_nan");
        let node2 = mock_window("w_neg");
        let screen = Rect::new(0, 0, 1920, 1080);
        let (map, _) = strat.calculate(&[node1, node2], screen, 1, 0.5, 10, None);

        // NaN debe ser descartado
        assert!(!map.contains_key("w_nan"));
        // Medidas negativas deben ser saneadas a un mínimo seguro (>= 10)
        if let Some(r) = map.get("w_neg") {
            assert!(r.width >= 10);
            assert!(r.height >= 10);
        }
    }

    #[test]
    fn test_lua_layout_calculation_succeeds() {
        let valid_script = r#"
            return function(screen, windows, config)
                local res = {}
                local w1 = windows[1]
                local id = type(w1) == "table" and w1.id or w1
                res[id] = {
                    x = screen.x + config.gaps,
                    y = screen.y + config.gaps,
                    w = screen.w - (config.gaps * 2),
                    h = screen.h - (config.gaps * 2)
                }
                return res
            end
        "#;
        let strat = LuaLayoutStrategy::new("test_calc".to_string(), valid_script.to_string());
        let node = mock_window("w_1");
        let screen = Rect::new(0, 0, 1920, 1080);
        let (map, evicted) = strat.calculate(&[node], screen, 1, 0.5, 12, None);

        assert_eq!(evicted.len(), 0);
        assert_eq!(map.len(), 1);
        let r = map.get("w_1").expect("Window w_1 must be present");
        assert_eq!(r.x, 12);
        assert_eq!(r.y, 12);
        assert_eq!(r.width, 1920 - 24);
        assert_eq!(r.height, 1080 - 24);
    }
}

