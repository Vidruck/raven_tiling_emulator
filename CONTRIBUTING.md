# Contribuyendo a Raven 🐦

¡Gracias por tu interés en mejorar Raven! Con el lanzamiento de la **v4.5.5**, el proyecto se consolida como una suite integral de gestión de mosaico dinámico multi-algoritmo (7 Layouts) con arquitectura **100% nativa en Rust**, **efectos de compositor en C++ acelerados por OpenGL**, **visualizador espectral CAVA**, un **adaptador KWin ultraligero ("dumb bridge")**, y el **Raven Hub v4.5.5** en C++/QML para KDE Plasma 6 (Wayland).

Este proyecto es software libre creado y mantenido por **Alejandro González Hernández (Vidruck)** bajo la licencia **GNU General Public License Version 3 (GPL-3.0)**. Todas las contribuciones enviadas al repositorio se licenciarán bajo estos mismos términos.

---

## 🏗️ Pilares Arquitectónicos v4.5.5

Para mantener la calidad, seguridad de tipos, resiliencia y fluidez del proyecto, todas las contribuciones deben respetar estos principios:

### 1. Motor Nativo y Domain-Driven Layouts (`core/engine_rs` y `crates/raven_core`)
- **Arquitectura Hexagonal**: La capa de dominio (`domain/layout/`) es completamente agnóstica de infraestructura. Soporta 7 algoritmos (`raven`, `tall`, `monocle`, `strict_dwindle`, `inverted_strict_dwindle`, `divisor`, `floating`). Toda nueva estrategia debe implementar el rasgo `LayoutStrategy` e incorporar su método `predict_capacity`.
- **Topología Híbrida Wayland + KWin**: La resolución espacial y la geometría de pantallas se obtienen directamente vía protocolo Wayland (`raven_backend_wayland`), mientras que KWin actúa como actuador de posicionamiento y sensor de foco.
- **Cuarentena Dinámica y Rectificación en Rust**: La estabilización de ventanas complejas (CSD en navegadores web, suites Electron y clientes Wayland fríos) es gestionada en Rust (`KWinQuarantineManager`) mediante temporizadores asíncronos y modelo de sospecha activa con rectificación de geometría post-layout.

### 2. Efecto Nativo de KWin en C++ (`adapters/kwin_effect/`)
- **Plugin Compositor de Alto Rendimiento**: Implementado en C++20 sobre las APIs de KWin 6 (`kwin4_effect_raven`). Intercepta el pipeline gráfico (`paintWindow`) usando shaders uniformes de OpenGL para renderizar contornos activos e inactivos sin sobrecarga de CPU.
- **Blindaje Anti-Flicker y Filtrado de Sistema**: Todo elemento que tome foco temporal (overlays de Spectacle, notificaciones, menús popup, lockscreen, bienvenida) debe ser filtrado mediante `shouldDrawBorder` para evitar alterar el contorno de la ventana activa subyacente.
- **Instalación y Despliegue Atómico**: Los binarios `.so` deben instalarse siempre mediante `install -m 755` para generar nuevos inodos y no corromper la memoria del proceso KWin en ejecución.

### 3. Adaptador KWin Ultraligero ("Dumb Bridge" JS) (`adapters/kwin_script/`)
- **Desacoplamiento de Atajos y Cuarentena**: El script de KWin no procesa combinaciones de teclas ni alberga lógica heurística pesada. Su rol es sensor de foco y actuador geométrico.
- **Construcción Monolítica**: El código se organiza de forma modular en `adapters/kwin_script/contents/code/` y se ensambla mediante `./build_kwin_bundle.sh` en `main.js`. Toda modificación debe realizarse en los submódulos correspondientes (`utils/`, `core/`, `services/`), **nunca editando `main.js` directamente**.

### 4. Plasmoide, Hub Integrado y CAVA (`adapters/plasmoid/`)
- Desarrollado en **C++20 y Qt 6 / QML**:
  - **Visualizador CAVA**: Subproceso reactivo optimizado en `MediaController` con drenado de tramas obsoletas para evitar congelamientos en el panel de Plasma.
  - Sincronización MPRIS2 no bloqueante y control de ciclo de vida de hardware en `SystemStats`.
  - Tokens de diseño atómico centralizado en `RavenTheme.qml`.

### 5. Centro de Control Gráfico (`raven_gui`)
- Aplicación de escritorio nativa en Rust usando `egui/eframe`. Ofrece vista previa vectorial 2D en tiempo real de monitores (`MonitorCanvas`), gestión granular de layouts por pantalla/escritorio y lectura de temas desde `~/.config/kdeglobals`.

---

## 🚀 Guía de Colaboración

1. **Reporte de Errores e Incidencias:** Abre un *Issue* en GitHub especificando:
   - Hardware, tarjeta gráfica (Intel / AMD / NVIDIA) y controladores.
   - Versión exacta de KDE Plasma (`plasmashell --version`), KWin y sesión Wayland.
   - Logs del daemon (`journalctl --user -u raven -f`) y trazas del compositor.
2. **Pull Requests:**
   - Trabaja siempre sobre la rama `Unstable` o ramas feature derivadas (`feature/nombre-mejora` o `fix/descripcion-bug`).
   - Verifica que el código supere todas las pruebas de sanidad y estilo:
     ```bash
     cargo check --workspace
     cargo clippy --workspace --all-targets -- -D warnings
     cargo test --workspace
     bash -n raven-setup.sh
     ```
   - Si modificas el adaptador KWin JS, regenera el bundle con `./build_kwin_bundle.sh`.
   - Mantén los comentarios y documentación técnica en **español**, preservando la integridad del formato Doxygen/rustdoc.
   - Todo commit debe redactarse en **español** con descripciones claras del valor aportado.

---

## 🛠️ Stack y Requisitos de Desarrollo

Para compilar, depurar y contribuir a la versión **v4.5.5**, asegúrate de tener instalado:

- **Rust Toolchain:** Edición 2021 estable $\ge 1.80$ (`rustc`, `cargo`, `clippy`).
- **C++ / Qt:** Compilador C++20 (`g++` o `clang`), Qt 6 (Core, Gui, Qml, Quick, DBus, Network), CMake $\ge 3.20$ y `extra-cmake-modules`.
- **Cabeceras de KWin:** `kwin-devel` o `kwin-dev` (para compilar `adapters/kwin_effect`).
- **Herramientas de Audio / CAVA:** `cava` (para el espectrograma en tiempo real de Raven Hub) y soporte PipeWire/PulseAudio.
- **Herramientas de KDE:** `kpackagetool6`, `kbuildsycoca6`, `kwriteconfig6`, `qdbus6` y sesión activa de `kwin_wayland`.
- **Librerías del Sistema:** `pkg-config`, `libxkbcommon-dev`, `wayland-protocols`, `libwayland-client`, `systemd`.

---

## 📄 Licencia

Al contribuir a este repositorio, aceptas que todo tu trabajo se distribuya bajo la **GNU General Public License Version 3 (GPL-3.0)**, reconociendo a **Alejandro González Hernández (Vidruck)** como autor principal y titular de los derechos del proyecto Raven Tiling Emulator. Consulta el archivo [LICENSE.txt](LICENSE.txt) para más detalles.

---

**¡Gracias por apoyar el desarrollo de Raven! Hagamos de este el gestor de mosaico más rápido, fluido y visualmente deslumbrante para KDE Plasma. ¡Huélum!**
