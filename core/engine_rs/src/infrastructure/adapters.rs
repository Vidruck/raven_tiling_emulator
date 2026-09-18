//! # Adaptadores de Infraestructura: Almacenamiento y Notificaciones
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Licencia:** GPL-3.0  
//!
//! Implementaciones concretas de los puertos secundarios definidos en `raven_core::ports`.

use std::collections::VecDeque;
use std::path::PathBuf;
use raven_core::ports::{HistoryStorage, NotificationPort};

/// Adaptador de almacenamiento en el sistema de archivos Linux (`~/.cache/raven/history.json`).
pub struct FileHistoryStorage {
    cache_path: PathBuf,
}

impl Default for FileHistoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl FileHistoryStorage {
    /// Crea una nueva instancia resolviendo el directorio HOME del usuario.
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("~"));
        let mut path = PathBuf::from(home);
        path.push(".cache");
        path.push("raven");
        path.push("history.json");
        Self { cache_path: path }
    }
}

impl HistoryStorage for FileHistoryStorage {
    fn load_history(&self) -> VecDeque<String> {
        if let Ok(content) = std::fs::read_to_string(&self.cache_path) {
            if let Ok(history) = serde_json::from_str(&content) {
                return history;
            }
        }
        VecDeque::new()
    }

    fn save_history(&self, history: VecDeque<String>) {
        let path = self.cache_path.clone();
        tokio::spawn(async move {
            if let Some(parent) = path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            if let Ok(json) = serde_json::to_string(&history) {
                let _ = tokio::fs::write(path, json).await;
            }
        });
    }
}

/// Adaptador de notificaciones OSD asíncronas mediante `notify-send` en entornos Linux.
#[derive(Default, Clone)]
pub struct NotifySendNotifier;

impl NotificationPort for NotifySendNotifier {
    fn notify_osd(&self, title: &str, body: &str) {
        let t = title.to_string();
        let b = body.to_string();
        tokio::spawn(async move {
            let _ = tokio::process::Command::new("notify-send")
                .arg("-a")
                .arg("Raven Tiling")
                .arg("-t")
                .arg("1200")
                .arg("-h")
                .arg("string:x-canonical-private-synchronous:raven-osd")
                .arg(&t)
                .arg(&b)
                .output()
                .await;
        });
    }
}
