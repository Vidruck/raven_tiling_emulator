# Raven Tiling Emulator 🐦

<p align="center">
  <img src="icon/org.kde.raven.tiling.svg" width="250" alt="Raven Logo">
</p>

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![JavaScript](https://img.shields.io/badge/javascript-%23323330.svg?style=for-the-badge&logo=javascript&logoColor=%23F7DF1E)
![KDE](https://img.shields.io/badge/KDE%20Plasma%206-21D359?style=for-the-badge&logo=kde&logoColor=white)
![Wayland](https://img.shields.io/badge/Wayland-9999ff?style=for-the-badge&logo=wayland&logoColor=white)
![Fedora](https://img.shields.io/badge/Fedora-51A2DA?style=for-the-badge&logo=fedora&logoColor=white)
![Arch Linux](https://img.shields.io/badge/Arch%20Linux-1793D1?style=for-the-badge&logo=arch-linux&logoColor=white)
![Debian](https://img.shields.io/badge/Debian-A81D33?style=for-the-badge&logo=debian&logoColor=white)
![GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)

**Raven Tiling Emulator** es un gestor de ventanas dinámico en mosaico (Tiling Window Manager) de alto rendimiento diseñado específicamente para **KDE Plasma 6 (Wayland)**. 

Con el lanzamiento de la **Versión 3.1**, implementa una nueva **interfaz de usuario en eGUI**, arquitectura modular en Rust nativo, comunicación de ultra-baja latencia **Single-Trip IPC**, multiples algoritmos de ordenamiento espacial y mejora su integración con navegadores web base **Gecko**.

---

## ⚡ Novedades Principales de la Versión 3.1

### 📐 1. Módulo de Layouts Rediseñado y Modular (Domain-Driven Architecture)
El motor geométrico se ha reestructurado por completo en submódulos especializados dentro de `domain/layout/`, ofreciendo desacoplamiento total y 5 algoritmos de distribución seleccionables:

| Algoritmo | Identificador | Descripción y Caso de Uso |
| :--- | :--- | :--- |
| **Raven (BSP Foveal)** | `"raven"` | Composición dinámica foveal con ranuras de utilidad laterales e inferiores para monitores ultrapanorámicos. |
| **Clásico (Tall)** | `"tall"` | Columna maestra principal en el lateral con apilamiento secundario vertical. |
| **Monóculo** | `"monocle"` | Maximización total enfocada de una sola ventana para concentración intensiva. |
| **Flujo Avanzado** | `"strict_dwindle"` | División fractal en espiral binaria simétrica secuencial. |
| **Flujo Avanzado (Invertido)** | `"inverted_strict_dwindle"` | División fractal en espiral binaria simétrica secuencial con orden de acomodo invertido. |
| **Divisor** | `"divisor"` | Reparto equitativo proporcional en $N$ columnas verticales. |

### 🚀 2. Arquitectura Single-Trip D-Bus IPC (zbus 4)
- **Cero Polling y Tráfico Optimizado**: Se eliminaron las transmisiones masivas de estado redundantes. El script de KWin y el motor de Rust interactúan mediante un modelo síncrono de consulta-respuesta en un solo viaje IPC (`syncStateAndUpdateLayout` y `syncWindowDelta`).
- **Reducción del 90% en Bus IPC**: Minimiza el uso de CPU de KWin y elimina cuellos de botella en composiciones complejas.

### 🎨 3. Centro de Control Nativo (`raven_gui`) con Temática KDE
- **Interfaz Modular en egui/eframe**: Aplicación gráfica ligera, dividida en pestañas especializadas para una configuración integral:
  - **Composición**: Configuración de algoritmos de mosaico, márgenes (gaps), proporciones (`ratio`, `nmaster`) y posiciones de PiP con selección interactiva.
  - **Reglas y Cuarentena**: Gestión de aplicaciones flotantes/PiP y lista de cuarentena para aislar aplicaciones problemáticas al iniciar.
  - **Gestión del Servicio**: Control directo del demonio nativo systemd (encendido, apagado, autoinicio) y mini-depurador de logs.
  - **Atajos**: Referencia integrada de atajos de teclado globales.
- **Previsualizador de Canvas en Vivo**: Renderiza espacialmente la distribución del layout seleccionado en tiempo real antes de aplicarlo.
- **Sincronización de Paleta KDE**: Lee dinámicamente la configuración de colores del sistema desde `~/.config/kdeglobals`, adaptando su apariencia a cualquier tema claro u oscuro de Plasma.

### 🦊 4. Erradicación del Desacomodo Nativo en Navegadores Gecko
- **Protocolo de Doble Confirmación**: Elimina definitivamente los parpadeos, traslapes y saltos geométricos causados por la inicialización asíncrona de marcos CSD/SSD en navegadores basados en Gecko (Firefox, Zen, Floorp, LibreWolf).
- **Cuarentena Dinámica Heurística**: Mantiene la ventana entrante en una fase de aislamiento temporal calibrada dinámicamente según la clase de la aplicación, esperando a que el motor gráfico de la ventana notifique sus dimensiones estables definitivas.
- **Marca de Acomodo (`sb`)**: Aplica una bandera interna de verificación síncrona en el adaptador de KWin, garantizando que el motor de Rust solo integre la ventana a la retícula espacial una vez validado su estado geométrico final mediante confirmación bilateral.

---

## 📉 Eficiencia Energética, Huella en Disco y Rendimiento (v3.0)

El proyecto prioriza la eficiencia extrema y el uso mínimo de recursos del sistema.

### 📊 Evolución del Consumo por Versión

| Versión | Arquitectura | RAM (Runtime) | Peso Binario Motor | Tráfico IPC |
| :--- | :--- | :--- | :--- | :--- |
| **v1.0** | Python Puro | 55.0 MB | ~15 MB | Alto (Polling continuo) |
| **v1.6** | Híbrida (Python + Rust FFI) | ~25.9 MB | ~18 MB | Medio |
| **v2.6** | Rust Nativo Asíncrono | ~4.3 MB | 1.4 MB | Continuo |
| **v2.9** | Rust Nativo (Flood Protection) | ~4.5 MB | 1.4 MB | Debounced |
| **v3.0** | **Rust Nativo (Single-Trip IPC & 5 Layouts)** | **~4.9 MB** | **1.9 MB** | **Ultra-bajo (-90%)** |

### 💾 Desglose de Almacenamiento e Instalación Local (v3.0)

| Componente | Tipo de Recurso | Tamaño en Disco | Notas Técnicas |
| :--- | :--- | :--- | :--- |
| **`raven_engine`** | Daemon Nativo en Rust | **1.9 MB** *(1,995 KB)* | Incluye los 5 algoritmos de layout, topología PiP e IPC Single-Trip (zbus 4). |
| **`raven_gui`** | Centro de Control (egui/eframe) | **4.3 MB** *(4,596 KB)* | Renderizado GPU nativo OpenGL, selector de presets y lector de paletas KDE. |
| **Adaptadores & Plasmoides** | KWin Script & Plasmoid QML | **< 60 KB** | Puente sensor-actuador ligero para Plasma 6. |
| **Total Instalación** | Entorno Local (`~/.local/share/raven/`) | **6.9 MB** | **Huella ultra-compacta en almacenamiento.** |

---

## 🏗️ Estructura del Proyecto

El proyecto está organizado en un **Cargo Workspace** que unifica los componentes en Rust, junto con los adaptadores y scripts para KDE Plasma:

- **Componentes Rust (Workspace)**:
  - **`crates/raven_core/`**: Biblioteca compartida con las entidades de dominio, geometría (`Rect`, `WindowNode`) y esquemas de configuración JSON.
  - **`core/engine_rs/`**: Motor principal asíncrono (Daemon systemd).
    - `domain/layout/`: Algoritmos de ordenamiento espacial (`dwindle_bsp`, `tall`, `monocle`, `strict_dwindle`, `divisor`, `topology`, `strategy`, `utils`).
    - `infrastructure/dbus.rs`: Servicio D-Bus zbus 4 que expone la interfaz IPC `org.kde.raven.Events`.
  - **`raven_gui/`**: Centro de control nativo basado en egui.
    - `src/tabs/`: Pestañas modulares de interfaz (Composición, Reglas, Servicio, Atajos, Acerca de).
    - `src/components/`: Componentes gráficos independientes (ej. `layout_preview.rs`).
- **`adapters/`**:
  - `kwin_script/`: Puente liviano para la API de KWin de Plasma 6, organizado en submódulos especializados:
    - `utils/`: Módulos del sistema (`logger`, `geometry`, `timer_pool`).
    - `core/`: Reglas de ventanas (`window_utils`), cuarentena CSD (`quarantine`) y foco direccional (`focus`).
    - `services/`: Puente de eventos D-Bus IPC (`dbus_bridge`) e inyección de atajos globales (`shortcuts`).
    - `index.js`: Punto de entrada que inicializa los atajos y el puente con todos los hooks del ciclo de vida de KWin (`windowAdded`, `windowRemoved`, etc.).
    - `main.js`: Bundle monolítico compilado que consume KWin Plasma 6.
  - `plasmoid/`: Widget de Plasma 6 para control rápido y estado desde el panel.
- **`build_kwin_bundle.sh`**: Script compilador independiente en Bash que ensambla automáticamente los submódulos JavaScript de `adapters/kwin_script/contents/code/` en el bundle distribuible `main.js` respetando el orden estricto de dependencias. Se invoca automáticamente durante `./install.sh` y facilita el desarrollo/depuración local.

---

## 🛠️ Instalación y Uso

### Requisitos Previos
- **KDE Plasma 6** sobre **Wayland**.
- Compilador de Rust (Cargo) y herramientas base de compilación (`build-essential` / `pkg-config`).

### Pasos de Instalación y Gestión (TUI Suite)
```bash
git clone https://github.com/Vidruck/raven_tiling_emulator.git
cd raven_tiling_emulator

# Menú TUI Interactivo Elegante / Instalación
./raven-setup.sh               # O bien: ./raven-setup.sh --install
./uninstall.sh                 # Para desinstalación
```

El script orquestador `./raven-setup.sh` ofrece una interfaz gráfica de consola (TUI) para:
- 🚀 **Instalación Completa**: Detección de dependencias, compilación de Rust, empaquetado JS, registro de KWin/Plasmoid y activación de Systemd.
- 🔄 **Recompilación Rápida**: Reconstruir binarios de Rust y reiniciar el servicio de usuario.
- 🎨 **Reconstruir Bundle KWin**: Invocar `build_kwin_bundle.sh` y actualizar el paquete en Plasma 6.
- 📊 **Ver Estado del Sistema**: Comprobar la presencia y ejecución del demonio `raven_engine`, KWin Script y Plasmoide.
- 🗑️ **Desinstalación limpia**: Detención de servicios y purgado selectivo de datos.

> **Nota para Fedora Linux (KDE Plasma Spin)**: El instalador detecta automáticamente Fedora e instala los paquetes necesarios (`kf6-kpackage`, `kde-cli-tools`, `nodejs`, `gcc`, `rsync`) en caso de que no estén presentes.

### Atajos de Teclado Predeterminados (KWin)

| Atajo | Función |
| :--- | :--- |
| **`Super + Space`** | Habilitar / Deshabilitar el motor de mosaico (On / Off) |
| **`Super + J` / `Super + K`** | Mover el foco a la ventana Siguiente / Anterior |
| **`Super + Flechas`** | Foco direccional nativo (Izquierda / Derecha / Arriba / Abajo) |
| **`Super + Shift + J` / `Super + Shift + K`** | Intercambiar posición de la ventana activa (Siguiente / Anterior) |
| **`Super + H` / `Super + L`** | Expandir / Contraer la proporción del área Master |
| **`Super + ]` / `Super + [`** | Incrementar / Decrementar cantidad de ventanas principales (`nmaster`) |
| **`Super + =` / `Super + -`** | Incrementar / Decrementar espaciado entre ventanas (Gaps) |
| **`Super + Shift + L`** | Ciclar entre los 5 algoritmos de Layout |
| **`Super + Shift + M` / `Super + Shift + N`** | Migrar ventana activa al Monitor Siguiente / Anterior |
| **`Super + Shift + Right` / `Super + Shift + Left`** | Migrar ventana activa al Escritorio Virtual Siguiente / Anterior |

---

## 🛡️ Solución para Aplicaciones que Rompen la Composición

Algunas aplicaciones presentan problemas conocidos en sesiones Wayland, como la apertura de múltiples ventanas temporales antes de terminar de inicializarse por completo. Esto provoca que la ventana final no se acomode correctamente en el mosaico o adquiera dimensiones inesperadas. Si notas este comportamiento, puedes mitigar el problema agregando la aplicación a la "Lista de Cuarentena".

**Pasos para añadir una aplicación a la lista de cuarentena:**
1. Abre el editor de menús de Plasma, busca la aplicación problemática y copia el texto que aparece en el campo **Programa**.
2. Abre **Raven Control Center > Reglas y Cuarentena > Lista de Cuarentena** y haz clic en **Añadir a la Lista**.
3. Pega el nombre que copiaste, guarda los cambios y reinicia la aplicación problemática.
> **Nota:** El motor es capaz de corregir la geometría y reasignar el tamaño adecuado ante cualquier actualización del entorno (por ejemplo, abrir otra ventana, interactuar con un plasmoide o minimizar y restaurar la aplicación). La recomendación de reiniciarla es únicamente para confirmar que, con la nueva regla de cuarentena, el problema desaparece desde el primer momento.

---

## 🧹 Desinstalación

Para remover completamente Raven y sus componentes del sistema sin afectar ningún otro programa:
```bash
./raven-setup.sh --uninstall
# O bien seleccionando la opción [5] ejecutando ./raven-setup.sh
```

---

## ⚠️ Descargo de Responsabilidad (Disclaimer)

**Este software se proporciona "tal cual" (AS IS), sin garantía de ningún tipo.** Raven interactúa directamente con el compositor KWin y el bus D-Bus de Plasma. El usuario asume la responsabilidad de su uso.

---

*Desarrollado por **Alejandro González Hernández (Vidruck)**. Licencia **GPL-3.0**.*  
*¡Huélum!*