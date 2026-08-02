# Contribuyendo a Raven 🐦

¡Gracias por tu interés en mejorar Raven! Con el lanzamiento de la **v3.0**, el proyecto ha evolucionado hacia un gestor de mosaico dinámico multi-algoritmo (5 Layouts) de arquitectura **100% nativa en Rust**, **IPC Single-Trip libre de polling** y un **adaptador KWin modular multiarchivo**.

Estamos encantados de recibir colaboraciones que impulsen la estabilidad, el rendimiento y la experiencia de usuario en KDE Plasma 6 (Wayland).

---

## 🏗️ Pilar Arquitectónico y Paradigma v3.0

Para mantener la calidad y fluidez del proyecto, todas las contribuciones deben respetar estos pilares:

### 1. Motor Nativo y Domain-Driven Layouts (`core/engine_rs`)
- **Arquitectura de Dominio (`domain/layout/`)**: La lógica de mosaico soporta 5 algoritmos (`raven`, `tall`, `monocle`, `strict_dwindle`, `divisor`). Toda nueva adición o modificación de algoritmo debe implementar el rasgo `LayoutStrategy` e incluir pruebas unitarias.
- **Protección Foveal & Asimetría**: El layout predeterminado `raven` organiza el espacio en un *Centro Foveal* principal flanqueado por paneles laterales/inferiores de utilidad.
- **Límites de Seguridad Wayland**: Debe mantenerse la protección de acotamiento de seguridad a `300x250` px en la comprobación de dimensiones mínimas de ventanas para evitar evicciones o cuellos de botella en la composición.

### 2. Arquitectura Single-Trip D-Bus IPC (`infrastructure/dbus.rs` & `zbus 4`)
- **Cero Polling (Push-Based IPC)**: Está estrictamente prohibido introducir bucles de consulta (polling). La comunicación entre el script de KWin y el daemon de Rust se realiza de manera síncrona/asíncrona en un solo viaje IPC (`syncStateAndUpdateLayout` y `syncWindowDelta`).
- **Respuesta de Geometría Inmediata**: La respuesta D-Bus devuelve los comandos de reposicionamiento en el mismo viaje para minimizar latencia y consumo de CPU.

### 3. Adaptador KWin Modular Multiarchivo (`adapters/kwin_script/`)
Con la v3.0, el puente de KWin se organiza en submódulos especializados en `contents/code/`:
- `utils/`: Módulos del sistema (`logger`, `geometry`, `timer_pool`).
- `core/`: Reglas de ventanas (`window_utils`), cuarentena CSD (`quarantine`) y foco direccional (`focus`).
- `services/`: Puente de eventos D-Bus IPC (`dbus_bridge`) e inyección de atajos globales (`shortcuts`).

> **Reglas para el adaptador JS:**
> - Usar la API nativa de Plasma 6 en lugar de filtrados manuales.
> - Mantener la inmutabilidad de IDs rastreando ventanas exclusivamente mediante `w.internalId.toString()`.
> - Reutilizar el pool de temporizadores (`setKWinTimeout`) para evitar la presión sobre el Garbage Collector en QJSEngine.
> - El instalador (`install.sh`) se encarga automáticamente de procesar los `@include` y compilar el bundle final para `kpackagetool6`.

### 4. Interfaz Nativa de Preferencias (`raven_gui`)
- Construida en Rust con `egui/eframe`. Ofrece vista previa del canvas en vivo y lectura nativa del tema de colores de KDE desde `~/.config/kdeglobals`.

### 5. Optimización de Binario y Recursos (Binary Thinning)
- Mantenemos una huella en disco mínima (~1.9 MB para el motor). Evita clonaciones innecesarias de datos en el motor y evalúa críticamente cualquier dependencia externa adicional.

---

## 🚀 Cómo Colaborar

1. **Reporte de Bugs:** Abre un *Issue* indicando tu hardware, versión de KDE Plasma y logs del daemon (`journalctl --user -u raven -f`).
2. **Pull Requests:**
   - Crea una rama descriptiva (`feature/nuevo-layout` o `fix/caso-borde-kwin`).
   - Verifica la sanidad de tu código ejecutando:
     ```bash
     cargo check
     cargo clippy
     cargo fmt --check
     ```
   - Documenta cualquier modificación en la interfaz D-Bus o en la estructura de configuración.

---

## 🛠️ Requisitos de Desarrollo (*Stack*)

- **Rust Toolchain:** Edición 2021 o superior (`cargo`).
- **Node.js:** Necesario para el empaquetador del puente JS (`node` durante `./install.sh`).
- **Librerías de Desarrollo:** `libwayland`, `libx11`, `libxkbcommon`, `pkg-config`.
- **Herramientas de KDE:** `kpackagetool6` y `kbuildsycoca6`.

---

**Tu ayuda no solo mejora a Raven, me ayuda a mí a ser un mejor ingeniero. Hagamos de Raven el Tiling Engine más rápido, ligero y elegante para KDE. ¡Huélum!**
