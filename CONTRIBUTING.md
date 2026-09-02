# Contribuyendo a Raven 🐦

¡Gracias por tu interés en mejorar Raven! Con el lanzamiento de la **v3.4**, el proyecto se consolida como una suite completa de gestión de mosaico dinámico multi-algoritmo (6 Layouts) de arquitectura **100% nativa en Rust**, **IPC Single-Trip libre de polling**, un **adaptador KWin modular** y el **Raven Hub v3.4** en C++/QML para KDE Plasma 6 (Wayland).

Este proyecto es software libre creado y mantenido por **Alejandro González Hernández (Vidruck)** bajo la licencia **GNU General Public License Version 3 (GPL-3.0)**. Todas las contribuciones enviadas al repositorio se licenciarán bajo estos mismos términos.

---

## 🏗️ Pilares Arquitectónicos v3.4

Para mantener la calidad, seguridad de tipos y fluidez del proyecto, todas las contribuciones deben respetar estos principios:

### 1. Motor Nativo y Domain-Driven Layouts (`core/engine_rs` y `crates/raven_core`)
- **Arquitectura Hexagonal**: La capa de dominio (`domain/layout/`) es agnóstica de infraestructura. Soporta 6 algoritmos (`raven`, `tall`, `monocle`, `strict_dwindle`, `inverted_strict_dwindle`, `divisor`). Toda nueva estrategia debe implementar el rasgo `LayoutStrategy` e incorporar su método `predict_capacity`.
- **Mediador de Capacidad y Mitigación de Saturación**: El motor evalúa dinámicamente la capacidad del monitor ($C_{max}$) antes de transicionar entre layouts, aplicando desalojo atómico FIFO hacia minimización para evitar sobrecargas o colapso visual.
- **Protección Geométrica de Remanente (`min_rem`)**: En algoritmos recursivos de espiral, se deben respetar pisos dimensionales de seguridad ($\ge 100$–$120$ px) para evitar rectángulos con dimensiones cero o negativas.

### 2. Arquitectura Single-Trip D-Bus IPC (`infrastructure/dbus.rs` & `zbus 4`)
- **Cero Polling (Push-Based IPC)**: Prohibido introducir bucles activos de consulta. La comunicación entre el script de KWin, el Plasmoide y el daemon de Rust se realiza mediante llamadas síncronas/asíncronas en un solo viaje IPC (`syncStateAndUpdateLayout`, `syncWindowDelta` y la señal `tilingCommandsPending`).
- **Respuesta Reactiva Inmediata**: Cualquier cambio de márgenes, ratio o algoritmos debe sincronizarse en vivo con KWin en el mismo ciclo de eventos sin esperar a la pérdida de foco.

### 3. Adaptador KWin Modular Multiarchivo (`adapters/kwin_script/`)
El puente de KWin se estructura de forma modular en `contents/code/`:
- `utils/`: `logger.js`, `geometry.js`, `timer_pool.js` (reutilización de `QTimer` para mitigar la presión sobre el Garbage Collector en QJSEngine).
- `core/`: `window_utils.js` (heurísticas y PiP), `quarantine.js` (estabilización CSD para navegadores Gecko) y `focus.js` (resalte visual con Outline).
- `services/`: `dbus_bridge.js` (puente D-Bus) y `shortcuts.js` (atajos globales de KWin).

### 4. Plasmoide y Hub Integrado (`adapters/plasmoid/`)
- Desarrollado en **C++20 y Qt 6 / QML**:
  - Clases documentadas bajo estándar Doxygen en español.
  - Sincronización MPRIS2 no bloqueante y extrapolación a 1 Hz en `MediaController`.
  - Monitoreo en vivo de hardware y esquema de colores de Plasma en `SystemStats`.
  - Integración de tokens de diseño atómico centralizado en `RavenTheme.qml`.

### 5. Centro de Control Gráfico (`raven_gui`)
- Aplicación de escritorio nativa en Rust usando `egui/eframe`. Ofrece vista previa vectorial 2D en tiempo real, gestión de reglas y lectura de temas desde `~/.config/kdeglobals`.

---

## 🚀 Guía de Colaboración

1. **Reporte de Errores e Incidencias:** Abre un *Issue* en GitHub especificando:
   - Hardware y tarjeta gráfica (Intel / AMD / NVIDIA).
   - Versión exacta de KDE Plasma y servidor gráfico (Wayland).
   - Logs del daemon (`journalctl --user -u raven -f`) y trazas del compositor.
2. **Pull Requests:**
   - Crea una rama descriptiva a partir de `main` (`feature/nombre-mejora` o `fix/descripcion-bug`).
   - Verifica que el código supere todas las pruebas de sanidad y estilo:
     ```bash
     cargo check --workspace
     cargo clippy --workspace --all-targets -- -D warnings
     cargo test --workspace
     ```
   - Mantén los comentarios y documentación técnica en **español**, preservando la integridad del formato Doxygen/rustdoc.
   - Todo commit debe redactarse en **español** con descripciones claras del valor aportado.

---

## 🛠️ Stack y Requisitos de Desarrollo

- **Rust Toolchain:** Edición 2021 estable (`rustc`, `cargo`, `clippy`).
- **C++ / Qt:** Compilador C++20 (`g++` o `clang`), Qt 6 (Core, Qml, Quick, DBus, Network), CMake $\ge 3.16$ y `extra-cmake-modules`.
- **Herramientas de KDE:** `kpackagetool6`, `kbuildsycoca6` y `kwin_wayland`.
- **Librerías del Sistema:** `pkg-config`, `libxkbcommon`, `systemd`.

---

## 📄 Licencia

Al contribuir a este repositorio, aceptas que todo tu trabajo se distribuya bajo la **GNU General Public License Version 3 (GPL-3.0)**, reconociendo a **Alejandro González Hernández (Vidruck)** como autor principal y titular de los derechos del proyecto Raven Tiling Emulator. Consulta el archivo [LICENSE.txt](LICENSE.txt) para más detalles.

---

**¡Gracias por apoyar el desarrollo de Raven! Hagamos de este el gestor de mosaico más rápido, fluido y visualmente deslumbrante para KDE Plasma. ¡Huélum!**
