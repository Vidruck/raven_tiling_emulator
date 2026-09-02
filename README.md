# Raven Tiling Emulator 🐦

<p align="center">
  <img src="icon/org.kde.raven.tiling.svg" width="250" alt="Raven Logo">
</p>

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![C++](https://img.shields.io/badge/c++-%2300599C.svg?style=for-the-badge&logo=c%2B%2B&logoColor=white)
![JavaScript](https://img.shields.io/badge/javascript-%23323330.svg?style=for-the-badge&logo=javascript&logoColor=%23F7DF1E)
![KDE](https://img.shields.io/badge/KDE%20Plasma%206-21D359?style=for-the-badge&logo=kde&logoColor=white)
![Wayland](https://img.shields.io/badge/Wayland-9999ff?style=for-the-badge&logo=wayland&logoColor=white)
![Fedora](https://img.shields.io/badge/Fedora-51A2DA?style=for-the-badge&logo=fedora&logoColor=white)
![Arch Linux](https://img.shields.io/badge/Arch%20Linux-1793D1?style=for-the-badge&logo=arch-linux&logoColor=white)
![Debian](https://img.shields.io/badge/Debian-A81D33?style=for-the-badge&logo=debian&logoColor=white)
![GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)

**Raven Tiling Emulator** es un gestor de ventanas dinámico en mosaico (Tiling Window Manager) de alto rendimiento diseñado específicamente para **KDE Plasma 6 (Wayland)**. 

Con el lanzamiento de la **Versión 3.4**, Raven consolida su suite completa que fusiona el motor de composición en Rust con un **Lanzador de Aplicaciones y Centro de Mando Integrado (Raven Hub v3.4)** en el panel de Plasma: reloj digital monospaced y fecha localizada en tiempo real, control de sesión, monitorización de hardware en vivo, reproductor multimedia con ecualizador animado armónico, mediador predictivo de capacidad entre algoritmos ($C_{max}$), navegación espacial instantánea en sub-islas, buscador de aplicaciones y sidebar de selección rápida para los 6 algoritmos de distribución.

---

## ⚡ Novedades Principales de la Versión 3.3

### 🛸 1. Raven Plasmoid & Hub Fusionado (v3.3)
- **Plasmoide Híbrido con Arquitectura de Islas Modulares**: Unifica el menú de aplicaciones y el centro de control de mosaico en una ventana flotante fluida con estética de vidrio esmerilado (*frosted glass*).
- **Adaptabilidad Cromática Dinámica (KDE Plasma Theme Engine)**: Extrae en tiempo real la paleta de colores del usuario (`~/.config/kdeglobals`), ajustando automáticamente contraste, luminancia ITU-R BT.601, bordes sutiles y transparencias en temas claros y oscuros (Nord, Ayu, Breeze, Catppuccin, etc.).
- **Detección Automática de Icono del Sistema**: Detecta la distribución Linux activa (`/etc/os-release`) y adopta dinámicamente el logotipo oficial de la distro o el icono nativo de KDE Plasma en el panel.
- **Isla de Comando y Control en Sub-Islas**:
  - **Monitores**: Detección reactiva de pantallas y migración instantánea de ventanas con controles direccionales.
  - **Carrusel de Escritorios Virtuales**: Navegación continua con indicador dinámico (`Desk N`) y migración rápida de ventanas activas al escritorio anterior o siguiente.
  - **Márgenes y Quick Peek**: Ajuste fino de espaciado (`Gaps ±2`) y conmutación de ventana flotante temporal unificada.
  - **Intercambio y GUI**: Botones rápidos de *Swap* de ventanas y acceso al Centro de Control gráfico.
- **Sidebar Vertical de 6 Algoritmos y Control de Sesión**:
  - Selección directa de cualquiera de los 6 modos de mosaico con sincronización D-Bus Push instantánea en pantalla.
  - Acciones de energía y sesión aisladas en la parte inferior (Bloqueo, Cierre de Sesión, Reinicio y Apagado).
- **Reproductor Multimedia Inteligente con Ecualizador Gráfico**:
  - Visualizador de espectro dinámico animado de 31 bandas ISO, barra de progreso interactiva con salto de posición (`SetPosition`), carátula integrada con recorte redondeado y sincronización continua MPRIS2 (Spotify, VLC, YouTube, navegadores web).
- **Buscador de Apps de Alta Velocidad**:
  - Filtrado en tiempo real de aplicaciones del sistema con navegación completa por teclado (flechas y tecla **Enter** para lanzamiento inmediato).

---

## 📐 Algoritmos de Distribución Espacial (Layouts)

El motor geométrico se organiza en submódulos especializados dentro de `domain/layout/`, ofreciendo desacoplamiento total y 6 algoritmos seleccionables:

| Algoritmo | Identificador | Descripción y Caso de Uso |
| :--- | :--- | :--- |
| **Raven (BSP Foveal)** | `"raven"` | Composición dinámica foveal con ranuras de utilidad laterales e inferiores para monitores ultrapanorámicos. |
| **Clásico (Tall)** | `"tall"` | Columna maestra principal en el lateral con apilamiento secundario vertical. |
| **Monóculo** | `"monocle"` | Maximización total enfocada de una sola ventana para concentración intensiva. |
| **Flujo Avanzado** | `"strict_dwindle"` | División fractal en espiral binaria simétrica secuencial. |
| **Flujo Avanzado (Invertido)** | `"inverted_strict_dwindle"` | División fractal en espiral binaria simétrica secuencial con orden de acomodo invertido. |
| **Divisor** | `"divisor"` | Reparto equitativo proporcional en $N$ columnas verticales. |

---

## 🚀 Arquitectura Single-Trip D-Bus IPC & Push Reactivo (zbus 4)
- **Cero Polling y Tráfico Optimizado**: Se eliminaron las transmisiones masivas de estado redundantes. El script de KWin y el motor de Rust interactúan mediante un modelo síncrono de consulta-respuesta en un solo viaje IPC (`syncStateAndUpdateLayout` y `syncWindowDelta`).
- **Sincronización Inmediata por Señales D-Bus (`tilingCommandsPending`)**: Los cambios emitidos desde el plasmoide (ajuste de gaps, cambio de layout, alternancia de mosaico) se aplican de forma inmediata en KWin sin requerir el cierre del launcher.
- **Reducción del 90% en Bus IPC**: Minimiza el uso de CPU de KWin y elimina cuellos de botella en composiciones complejas.

---

## 🎨 Centro de Control Nativo (`raven_gui`)
- **Interfaz Modular en egui/eframe**: Aplicación gráfica ligera, dividida en pestañas especializadas para una configuración integral:
  - **Composición**: Configuración de algoritmos de mosaico, márgenes (gaps), proporciones (`ratio`, `nmaster`) y escalabilidad dinámica de ventanas PiP.
  - **Layouts**: Selector de algoritmos con previsualización geométrica y configuración granular de parámetros.
  - **Servicio**: Control directo del demonio nativo systemd (encendido, apagado, autoinicio) y visualizador de logs en tiempo real.
  - **Atajos**: Referencia visual e interactiva de todos los atajos de teclado globales.
- **Previsualizador Avanzado en Vivo**: Renderiza espacialmente la distribución del layout seleccionado en tiempo real, ilustrando la estructura fractal hasta la 7ma ventana.
- **Posicionamiento PiP de 8 Puntos**: Configura las ventanas Picture-in-Picture a través de un lienzo interactivo con anclaje expandido: esquinas y puntos cardinales.
- **Sincronización de Paleta KDE y Efecto Glass**: Lee dinámicamente `~/.config/kdeglobals` adaptando la estética a cualquier tema del sistema.

### 🦊 Erradicación del Desacomodo Nativo en Navegadores Gecko
- **Protocolo de Doble Confirmación**: Elimina definitivamente los parpadeos, traslapes y saltos geométricos causados por la inicialización asíncrona de marcos CSD/SSD en navegadores basados en Gecko (Firefox, Zen, Floorp, LibreWolf).
- **Cuarentena Dinámica Heurística**: Mantiene la ventana entrante en una fase de aislamiento temporal calibrada dinámicamente según la clase de la aplicación, esperando a que el motor gráfico de la ventana notifique sus dimensiones estables definitivas.
- **Marca de Acomodo (`sb`)**: Aplica una bandera interna de verificación síncrona en el adaptador de KWin, garantizando que el motor de Rust solo integre la ventana a la retícula espacial una vez validado su estado geométrico final mediante confirmación bilateral.

---

## 📉 Eficiencia Energética, Huella en Disco y Rendimiento

El proyecto prioriza la eficiencia extrema y el uso mínimo de recursos del sistema.

### 📊 Evolución del Consumo por Versión

| Versión | Arquitectura | RAM (Runtime) | Peso Binario Motor | Tráfico IPC |
| :--- | :--- | :--- | :--- | :--- |
| **v1.0** | Python Puro | 55.0 MB | ~15 MB | Alto (Polling continuo) |
| **v1.6** | Híbrida (Python + Rust FFI) | ~25.9 MB | ~18 MB | Medio |
| **v2.6** | Rust Nativo Asíncrono | ~4.3 MB | 1.4 MB | Continuo |
| **v3.0** | Rust Nativo (Single-Trip IPC & 5 Layouts) | ~4.9 MB | 1.9 MB | Ultra-bajo (-90%) |
| **v3.3** | **Rust Nativo + C++/QML Hub (6 Layouts & D-Bus Push)** | **~5.4 MB** | **1.9 MB** | **Tiempo Real Reactivo** |

### 💾 Desglose de Almacenamiento e Instalación Local

| Componente | Tipo de Recurso | Tamaño en Disco | Notas Técnicas |
| :--- | :--- | :--- | :--- |
| **`raven_engine`** | Daemon Nativo en Rust | **1.9 MB** | Motor de 6 layouts, topología PiP, D-Bus IPC (zbus 4). |
| **`raven_gui`** | Centro de Control (egui/eframe) | **4.3 MB** | Renderizado GPU nativo OpenGL, previsualizador fractal y lector de paletas KDE. |
| **Adaptadores & Plasmoides** | KWin Script & Plugin C++/QML | **< 200 KB** | Puente sensor-actuador y plugin compilado para Plasma 6. |
| **Total Instalación** | Entorno Local (`~/.local/share/raven/`) | **~6.9 MB** | **Huella ultra-compacta en almacenamiento.** |

---

## 🏗️ Estructura del Proyecto

El proyecto está organizado en un **Cargo Workspace** que integra el motor en Rust, el centro de control en egui, el plugin C++/QML y el script KWin:

```text
.
├── adapters/
│   ├── kwin_script/                          # Script de KWin para Plasma 6
│   │   ├── contents/code/
│   │   │   ├── core/                         # Reglas de ventanas, cuarentena CSD y foco
│   │   │   ├── services/                     # D-Bus IPC Bridge y registro de Atajos Globales
│   │   │   ├── utils/                        # Logger, geometría, temporizadores
│   │   │   ├── index.js                      # Inicializador de hooks del ciclo de vida de KWin
│   │   │   └── main.js                       # Bundle monolítico generado para KWin
│   │   └── metadata.json                     # Descriptor del paquete de script KWin
│   └── plasmoid/                             # Plasmoide y Launcher para el Panel
│       ├── package/                          # Interfaz QML del Plasmoide (Raven Hub)
│       │   ├── contents/ui/                  # Vistas: MainWindowView, AppGridView, MediaWidgetView, etc.
│       │   └── metadata.json                 # Metadatos del Applet para Plasma 6
│       ├── plugin/                           # Plugin nativo C++ / QML de alto rendimiento
│       │   ├── apprunner.{h,cpp}             # Indexador y ejecutor de apps del sistema
│       │   ├── colorextractor.{h,cpp}        # Extractor de colores de carátulas
│       │   ├── mediacontroller.{h,cpp}       # Controlador MPRIS2 y barra de progreso
│       │   ├── ravencontroller.{h,cpp}       # Puente D-Bus con el motor Rust
│       │   ├── systemcontroller.{h,cpp}      # Acciones de sesión y energía
│       │   ├── systemstats.{h,cpp}           # Métricas de hardware, distro icon y paleta KDE
│       │   ├── weathercontroller.{h,cpp}     # Consulta meteorológica en segundo plano
│       │   ├── RavenTheme.qml                # Singleton de tokens de diseño adaptativo
│       │   └── CMakeLists.txt                # Configuración de compilación C++ Qt6
│       └── CMakeLists.txt
├── core/
│   └── engine_rs/                            # Motor Principal en Rust (Daemon Systemd)
│       ├── src/
│       │   ├── application/                  # Controlador de orquestación y ciclo de vida
│       │   ├── domain/                       # Lógica de dominio, layout strategies y saturación
│       │   ├── infrastructure/               # Adaptador de servicio D-Bus (zbus 4)
│       │   ├── ports/                        # Interfaces y contratos del motor
│       │   ├── lib.rs
│       │   └── main.rs
│       ├── tests/                            # Tests de integración, multimonitor y estrés
│       └── Cargo.toml
├── crates/
│   └── raven_core/                           # Biblioteca núcleo compartida
│       ├── src/
│       │   ├── action.rs                     # Comandos y acciones geométricas
│       │   ├── config.rs                     # Esquema y persistencia de configuración
│       │   ├── geometry.rs                   # Primitivas geométricas (Rect, WindowNode)
│       │   └── lib.rs
│       └── Cargo.toml
├── icon/                                     # Iconografía vectorial oficial
│   └── org.kde.raven.tiling.svg
├── raven_gui/                                # Centro de Control Gráfico en egui / Rust
│   ├── src/
│   │   ├── components/                       # Componentes gráficos (layout_preview, etc.)
│   │   ├── tabs/                             # Pestañas: composition, layouts, service, shortcuts, about
│   │   ├── app.rs                            # Estructura principal y ciclo de dibujado egui
│   │   ├── kde_theme.rs                      # Lector de temas nativos KDE
│   │   ├── models.rs / services.rs           # Estado y comunicación D-Bus
│   │   └── main.rs
│   └── Cargo.toml
├── systemd/                                  # Definiciones de servicios de usuario
│   ├── org.kde.raven.Daemon.service          # Activación por D-Bus bajo demanda
│   └── raven.service                         # Servicio continuo systemd
├── build_kwin_bundle.sh                      # Ensamblador del bundle JavaScript de KWin
├── raven-setup.sh                            # Suite TUI de instalación y mantenimiento
├── Cargo.toml                                # Configuración raíz del Workspace de Rust
└── README.md
```

---

## 🛠️ Instalación y Uso

### Requisitos Previos
- **KDE Plasma 6** sobre **Wayland**.
- Compilador de Rust (Cargo) y herramientas base de compilación (`build-essential` / `pkg-config`, `cmake`, `extra-cmake-modules`).

### Pasos de Instalación y Gestión (TUI Suite)
```bash
git clone https://github.com/Vidruck/raven_tiling_emulator.git
cd raven_tiling_emulator

# Menú TUI Interactivo / Instalación
./raven-setup.sh               # O bien: ./raven-setup.sh --install
```

El script orquestador `./raven-setup.sh` ofrece una interfaz gráfica de consola (TUI) para:
- 🚀 **Instalación Completa**: Detección de dependencias, compilación de Rust, compilación del plugin C++, empaquetado JS, registro de KWin/Plasmoid y activación de Systemd.
- 🔄 **Recompilación Rápida**: Reconstruir binarios de Rust y reiniciar el servicio de usuario.
- 🎨 **Reconstruir Bundle KWin & Plasmoid**: Invocar `build_kwin_bundle.sh` y actualizar componentes en Plasma 6.
- 📊 **Ver Estado del Sistema**: Comprobar la presencia y ejecución del demonio `raven_engine`, KWin Script y Plasmoide.
- 🗑️ **Desinstalación limpia**: Detención de servicios y purgado selectivo de datos.

> **Nota para distribuciones Linux**: El instalador detecta automáticamente el gestor de paquetes del sistema (Fedora, Arch Linux, openSUSE, Debian/Ubuntu) e instala las herramientas necesarias en caso de que falten.

---

### ⌨️ Atajos de Teclado Globales (KWin / Plasma 6)

| Categoría | Atajo | Acción / Función |
| :--- | :--- | :--- |
| **Mosaico & Flotación** | **`Meta + Space`** | Habilitar / Deshabilitar el motor de mosaico (On / Off) |
| | **`Meta + Shift + F`** | Alternar Ventana Flotante Dinámica / Quick Peek (On / Off) |
| | **`Meta + Shift + L`** | Ciclar secuencialmente entre los 6 algoritmos de Layout |
| **Navegación & Foco** | **`Meta + J` / `Meta + K`** | Mover el foco a la ventana Siguiente / Anterior |
| | **`Meta + Flechas`** | Foco direccional nativo (Izquierda / Derecha / Arriba / Abajo) |
| | **`Meta + Shift + J` / `Meta + Shift + K`** | Intercambiar posición de la ventana activa (Swap Siguiente / Anterior) |
| **Geometría & Dimensiones** | **`Meta + Alt + Right` / `Meta + Alt + Left`** | Aumentar / Reducir ANCHO de ventana (2D Dynamic Resize) |
| | **`Meta + Alt + Down` / `Meta + Alt + Up`** | Aumentar / Reducir ALTO de ventana (2D Dynamic Resize) |
| | **`Meta + H` / `Meta + L`** | Expandir / Contraer la proporción del área Master |
| | **`Meta + ]` / `Meta + [`** | Incrementar / Decrementar cantidad de ventanas principales (`nmaster`) |
| | **`Meta + =` / `Meta + -`** | Incrementar / Decrementar espaciado entre ventanas (Gaps ±2px) |
| **Multimonitor & Escritorios** | **`Meta + Shift + M` / `Meta + Shift + N`** | Migrar ventana activa al Monitor Siguiente / Anterior |
| | **`Meta + Shift + Right` / `Meta + Shift + Left`** | Migrar ventana activa al Escritorio Virtual Siguiente / Anterior |

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
# O bien seleccionando la opción [6] ejecutando ./raven-setup.sh
```

---

---

## 📄 Licencia y Autoría

Este proyecto es software libre y de código abierto desarrollado y mantenido por **Alejandro González Hernández (Vidruck)** bajo los términos de la **GNU General Public License Version 3 (GPL-3.0)**.

Puedes consultar los términos legales completos en el archivo [LICENSE.txt](LICENSE.txt).

---

## ⚠️ Descargo de Responsabilidad (Disclaimer)

**Este software se proporciona "tal cual" (AS IS), sin garantía de ningún tipo.** Raven interactúa directamente con el compositor KWin y el bus D-Bus de Plasma. El usuario asume la responsabilidad de su uso.

---

*Desarrollado por **Alejandro González Hernández (Vidruck)** — Licencia **GPL-3.0**.*  
*¡Huélum!*