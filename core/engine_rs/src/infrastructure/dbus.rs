//! # Capa de Infraestructura D-Bus (Reexportada desde `raven_backend_kwin`)
//!
//! **Autor:** Alejandro González Hernández (Vidruck)  
//! **Versión:** 4.0  
//! **Licencia:** GPL-3.0  
//!
//! Este módulo reexporta los servicios, comandos y parsers nativos desde el crate
//! desacoplado [`raven_backend_kwin`].

pub use raven_backend_kwin::{
    parse_payload,
    KWinDbusService as RavenDBusService,
    KWinPayload,
    KWinScreen,
    KWinTopology,
    KWinWindow,
    TilingCommand,
    service::KWinBridgeMessage,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_payload_empty() {
        let json = "{}";
        let res = parse_payload(json);
        assert!(res.is_err());
    }

    #[test]
    fn test_parse_payload_no_windows() {
        let json = r#"{"screens": {}, "windows": []}"#;
        let (workspaces, windows, topology) = parse_payload(json).unwrap();
        assert!(workspaces.is_empty());
        assert!(windows.is_empty());
        assert!(topology.outputs.is_empty());
        assert!(topology.desktops.is_empty());
    }

    #[test]
    fn test_parse_payload_valid() {
        let json = r#"{
            "screens": {
                "ws1": {"x": 0, "y": 0, "w": 1920, "h": 1080}
            },
            "windows": [
                {
                    "id": "win1",
                    "ws": "ws1",
                    "output": "DP-1",
                    "desktops": ["desktop-1"],
                    "f": false,
                    "m": false,
                    "p": false,
                    "x": 10,
                    "y": 10,
                    "w": 500,
                    "h": 400,
                    "min_w": 0,
                    "min_h": 0,
                    "sb": false
                }
            ],
            "topology": {
                "outputs": ["DP-1"],
                "desktops": ["desktop-1"]
            }
        }"#;

        let (workspaces, windows, topology) = parse_payload(json).unwrap();
        assert_eq!(workspaces.len(), 1);
        assert_eq!(workspaces.get("ws1").unwrap().width, 1920);

        assert_eq!(windows.len(), 1);
        let win = &windows[0];
        assert_eq!(win.window_id, "win1");
        assert_eq!(win.geometry.width, 500);

        assert_eq!(topology.outputs.len(), 1);
        assert_eq!(topology.outputs[0], "DP-1");
    }
}
