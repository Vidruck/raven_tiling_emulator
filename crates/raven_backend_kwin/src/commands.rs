//! # Comandos Serializables para KWin (`commands`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 4.0  
//! **Licencia:** GPL-3.0  
//!
//! Define la estructura JSON de comandos que consume el actuador en KWin (`TilingCommand`).

use serde::Serialize;
use raven_core::action::RavenAction;

/// Comando de redimensionamiento, movimiento o foco serializado para el puente de KWin.
#[derive(Debug, Serialize, Clone)]
pub struct TilingCommand {
    /// Acción a ejecutar (p. ej., "move", "focus", "minimize", "set_floating", "migrate_to_output").
    pub action: String,
    /// Identificador único de la ventana objetivo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_id: Option<String>,
    /// Coordenada horizontal de destino.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<i32>,
    /// Coordenada vertical de destino.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<i32>,
    /// Ancho final en píxeles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    /// Alto final en píxeles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    /// Identificador del área de trabajo destino para migraciones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_ws: Option<String>,
    /// Dirección del comando.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    /// Estado flotante booleano para comandos set_floating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floating: Option<bool>,
    /// Estado keep_above booleano para comandos set_floating.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_above: Option<bool>,
}

impl From<RavenAction> for TilingCommand {
    /// Convierte una acción lógica de dominio `RavenAction` a un comando físico `TilingCommand`.
    fn from(action: RavenAction) -> Self {
        match action {
            RavenAction::MoveWindow {
                window_id,
                x,
                y,
                width,
                height,
            } => TilingCommand {
                action: "move".to_string(),
                window_id: Some(window_id),
                x: Some(x),
                y: Some(y),
                width: Some(width),
                height: Some(height),
                target_ws: None,
                direction: None,
                floating: None,
                keep_above: None,
            },
            RavenAction::FocusWindow { window_id } => TilingCommand {
                action: "focus".to_string(),
                window_id: Some(window_id),
                x: None,
                y: None,
                width: None,
                height: None,
                target_ws: None,
                direction: None,
                floating: None,
                keep_above: None,
            },
            RavenAction::MigrateToOutput {
                window_id,
                target_output,
            } => TilingCommand {
                action: "migrate_to_output".to_string(),
                window_id: Some(window_id),
                target_ws: Some(target_output),
                x: None,
                y: None,
                width: None,
                height: None,
                direction: None,
                floating: None,
                keep_above: None,
            },
            RavenAction::MigrateToDesktop {
                window_id,
                target_desktop,
            } => TilingCommand {
                action: "migrate_to_desktop".to_string(),
                window_id: Some(window_id),
                target_ws: Some(target_desktop),
                x: None,
                y: None,
                width: None,
                height: None,
                direction: None,
                floating: None,
                keep_above: None,
            },
            RavenAction::RequestFeedback { window_id } => TilingCommand {
                action: "request_feedback".to_string(),
                window_id: Some(window_id),
                x: None,
                y: None,
                width: None,
                height: None,
                target_ws: None,
                direction: None,
                floating: None,
                keep_above: None,
            },
            RavenAction::SaturationWarning { cmax, active } => TilingCommand {
                action: "saturation_warning".to_string(),
                window_id: None,
                x: Some(i32::try_from(cmax).unwrap_or(i32::MAX)),
                y: Some(i32::try_from(active).unwrap_or(i32::MAX)),
                width: None,
                height: None,
                target_ws: None,
                direction: None,
                floating: None,
                keep_above: None,
            },
            RavenAction::SetFloating {
                window_id,
                floating,
                keep_above,
            } => TilingCommand {
                action: "set_floating".to_string(),
                window_id: Some(window_id),
                floating: Some(floating),
                keep_above: Some(keep_above),
                x: None,
                y: None,
                width: None,
                height: None,
                target_ws: None,
                direction: None,
            },
            RavenAction::MinimizeWindow { window_id } => TilingCommand {
                action: "minimize".to_string(),
                window_id: Some(window_id),
                x: None,
                y: None,
                width: None,
                height: None,
                target_ws: None,
                direction: None,
                floating: None,
                keep_above: None,
            },
            RavenAction::UnminimizeWindow { window_id } => TilingCommand {
                action: "unminimize".to_string(),
                window_id: Some(window_id),
                x: None,
                y: None,
                width: None,
                height: None,
                target_ws: None,
                direction: None,
                floating: None,
                keep_above: None,
            },
        }
    }
}
