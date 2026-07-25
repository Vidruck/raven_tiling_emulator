# Raven Tiling Emulator 🐦


<p align="center">
  <img src="icon/org.kde.raven.tiling.svg" width="250" alt="Raven Logo">
</p>

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![JavaScript](https://img.shields.io/badge/javascript-%23323330.svg?style=for-the-badge&logo=javascript&logoColor=%23F7DF1E)
![KDE](https://img.shields.io/badge/KDE%20Plasma-21D359?style=for-the-badge&logo=kde&logoColor=white)
![Wayland](https://img.shields.io/badge/Wayland-9999ff?style=for-the-badge&logo=wayland&logoColor=white)
![GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)

Raven es un gestor de ventanas dinámico (Tiling Window Manager) diseñado específicamente para **KDE Plasma 6 (Wayland)**. Con la llegada de la **versión 3.0**, Raven evoluciona hacia una arquitectura de **Single-Trip Dbus Message** y un modelo de rectificación instantánea.

## 🚀 El Salto a la Versión 2.9: Composición Foveal Dinámica y Reconciliación Orgánica
Esta versión reimagina el motor de mosaico adoptando un esquema de diseño de foco centrado y paneles de utilidad distribuidos en base a una jerarquía foveal adaptativa, e implementa nuevos filtros de detección de ventanas *"Picture in Picture"*.

### 📉 Eficiencia Energética y de Almacenamiento
La optimización sigue siendo el pilar fundamental. El motor opera con recursos sumamente bajos y contenidos:

| Versión | Arquitectura | RAM (Runtime) | ROM (Binario) |
|---|---|---|---|
| **v1.0** | Python Puro | 55.0 MB | ~15 MB |
| **v1.6** | Híbrida (Python + Rust FFI) | ~25.9 MB | ~18 MB |
| **v2.6** | Asynchronous Native Rust (Fixed) | ~4.3 MB | 1.4 MB |
| **v2.7/2.8** | Asynchronous Native Rust (Foveal Layout) | ~4.5 MB | 1.4 MB |
| **v2.9** | **Asynchronous Native Rust (Flood Protection & GC-Safe)** | **~4.5 MB** | **1.4 MB** |
| **v3.0** | **Native Rust (Single-Trip IPC & Dynamic Min-Size)** | **~4.5 MB** | **1.4 MB** |

*La eficiencia extrema ha sido una directriz arquitectónica fundamental desde el inicio del proyecto. Tras validaciones exhaustivas en hardware real, se logró consolidar un motor de alto rendimiento que minimiza el impacto en recursos. Gracias al uso de LTO, el pruning de dependencias y la eliminación de símbolos, entregamos un binario ultra-compacto sin comprometer la estabilidad.*

## 🌟 Nuevas Funciones y Estabilidad (v3.0)
- **Motor Geomético Respetuoso (Dynami Min-Size Adaptation):** El motor Rust lee los requisitos de ventanas celosas de sus limites minimos de compocisión y **comprime dinámicamente el resto de la pila matemática** para asegurar que todos los elementos se muestren correctamente sin invadir el espacio de los demás.
- **Arquitectura Single-Trip D-Bus:** El puente JS y el motor Rust ahora usan un modelo de "consulta-respuesta" sincrónica (Single-Trip). Se eliminaron las transmisiones masivas de estado redundante. El tráfico en el bus IPC de sistema se redujo a menos de un 10% respecto a la v2.9, aniquilando los cuellos de botella del CPU de KWin.
- **Plasmóide Rediseñado con Carrusel Contextual:** El widget (plasmoide) oficial ahora actúa como un carrusel inteligente de orientación espacial. En lugar de textos genéricos, el plasmoide te muestra físicamente qué escritorio virtual tienes a tu izquierda y derecha (ej. 4 | Escritorio | 2), mejorando la navegación multisalida.
- **Nuevos Controles Maestros:** Protección rígida del ratio maestro para que nunca colapse por debajo del 30% (0.30) sin importar la cantidad de clics del usuario, protegiendo la legibilidad de la pantalla.
- Tránsito de Escritorios Secuencial: El envío de ventanas hacia otros escritorios (MigrateToDesktop) fue reescrito para saltar secuencialmente (1 -> 2 -> 3) en lugar de saltos abruptos.
- **Composición Foveal Dinámica (Dynamic Foveal Composition):** La pantalla se organiza de forma inteligente situando la ventana en foco activo en un *Centro Foveal* preponderante, flanqueado por ranuras de contexto lateral simétricas e inferiores de utilidad. Al superarse la cantidad base de ventanas, el motor aplica subdivisiones jerárquicas recursivas (empezando por los laterales) para preservar la legibilidad y área de trabajo central.
- **Redimensionamiento Asimétrico Focalizado:** El ajuste del ratio de división (`master_ratio`) se aplica únicamente al corte que involucra a la ventana en foco activo (focused window). El resto de las ventanas mantienen una proporción simétrica limpia de `0.5` (50-50).
- **Reinicio Automático de Proporción:** Para evitar deformaciones acumulativas, cualquier adición o remoción de ventanas en la composición restablece de manera atómica el ratio maestro a `0.5`, garantizando una transición visualmente limpia y simétrica de forma inmediata.
- **Bucle de Recálculo Dinámico sin Huecos:** Si una ventana no cabe en su celda calculada, el motor la desaloja y ejecuta una reconciliación iterativa redistribuyendo el 100% del área restante a las demás ventanas activas.
- **Resiliencia y Comunicación Asíncrona:** El puente KWin-Raven ahora es completamente no bloqueante. El motor Rust utiliza offloading asíncrono con `tokio::spawn` para liberar el bus de datos instantáneamente.
- **Envío de Ventanas mediante Toggle:** El toggle permite enviar la última ventana en foco al monitor o escritorio virtual alterno para comodidad del usuario.
- **Soporte a traspaso de ventanas vía arrastre:** Capacidad de arrastrar con el ratón la ventana a otro monitor o escritorio virtual disponible con reacomodo instantáneo de la composición.
- **Persistencia Topológica de Sesión (v2.8):** El motor ahora guarda en segundo plano tu historial topológico. Al reiniciar el motor o KWin, el layout se restaura con tu orden previo exacto, evitando un reacomodo de ventanas no deseado.
- **Reglas Dinámicas y Cuarentenas Integradas (v2.8):** Puedes definir desde la interfaz gráfica (GUI) qué clases de ventanas forzar como flotantes, PiP o cuáles poner en cuarentena, inyectándose en el compositor Wayland en tiempo real sin reiniciar.
- **Protección Anti-Saturación / Flood Protection (v2.9):** Implementación de *Debouncing* nativo en Rust con cerrojos atómicos (`AtomicBool`). Si el compositor Wayland bombardea el motor con miles de eventos simultáneos, el sistema agrupa inteligentemente las peticiones y ejecuta un único recálculo geométrico, impidiendo cuellos de botella infinitos.
- **Estabilización Zero-Allocation para Gecko (v2.9):** El *Silencio Geométrico* se ha optimizado para ser completamente amigable con el Garbage Collector (GC) de KWin. Se reutiliza un *Pool Estático de Temporizadores* (Zero-Allocation) para manejar las cuarentenas dinámicas de navegadores complejos (Firefox, LibreWolf, Floorp, Zen), logrando un acoplamiento perfecto libre de pausas o *micro-stutters*.
- **Integración Nativa de Atajos de Teclado (v2.9):** El script puente de KWin se adueña automáticamente de los atajos del teclado a través de la API `registerShortcut`. Esto elimina la necesidad de programas de terceros como `sxhkd`. Los atajos aparecen directamente en *Preferencias del Sistema -> Atajos -> KWin* para su fácil reconfiguración. El atajo predeterminado para habilitar/deshabilitar el mosaico es **`Super+Space`** (o `Meta+Space`).

### 🏗️ Arquitectura de Comunicación (High-Performance Bridge)
El sistema utiliza un puente de baja latencia altamente desacoplado entre el compositor KWin y el motor Raven, optimizado para los estándares de **Plasma 6 (Wayland)**:
- **Puente de Alto Rendimiento (Sensor-Actuator Model):** Basado en una investigación profunda de la API `QJSEngine`, Raven ahora utiliza un sistema de sincronización atómica donde el script de KWin actúa como un sensor debounced.
- **Optimización de D-Bus:** Se ha eliminado el envío masivo de estados redundantes. El tráfico en el bus de sistema se ha reducido en un **~70%**, liberando recursos críticos del compositor.
- **Uso de Identificadores Nativos:** Migración completa al uso de `internalId` y la topología global de `workspace.screens` de Plasma 6, eliminando desincronizaciones en configuraciones multi-monitor o escritorios virtuales.
- **Mecanismo Watchdog:** El script de KWin incorpora un temporizador de vigilancia (Watchdog) de 6 segundos para liberar bloqueos potenciales en la comunicación IPC.

## 🏗️ Nueva Estructura del Proyecto
- `core/engine_rs/`: El corazón del proyecto. Un daemon nativo asíncrono que escucha al compositor KWin.
- `raven_gui/`: Aplicación de preferencias nativa basada en egui para una configuración visual fluida.
- `adapters/`: 
    - `kwin_script/`: Bridge liviano en JavaScript para la API de Plasma 6.
    - `plasmoid/`: Widget de Plasma para el control rápido del estado del motor.
- `bin/`: Directorio de destino para los binarios optimizados una vez instalados.

## 🛠️ Instalación y Uso
El nuevo instalador gestiona la descarga de dependencias, crates de Rust y la compilación optimizada de los componentes nativos.

> [!NOTE]
> **Instalación automática de Rust/Cargo:** Si `cargo` no está presente en el sistema, `install.sh` intentará descargarlo e instalarlo de manera automatizada a través del instalador oficial `rustup.rs` (fuente segura por contrato social). Sin embargo, por seguridad y control del entorno, **se recomienda que el usuario gestione e instale el compilador de Rust por cuenta propia**. Esta opción automática está pensada solo para aquellos usuarios que buscan una automatización total; en tal caso, se requiere tener previamente instalado `curl` en el sistema para que el instalador actúe de forma autónoma.

1. Clona el repositorio.
2. Ejecuta `./install.sh`.
3. Activa "Raven Bridge" en la configuración de KWin (Scripts de KWin).

```bash
git clone https://github.com/Vidruck/raven_tiling_emulator
cd raven_tiling_emulator
./install.sh
```

## 🧹 Desinstalación
Si deseas eliminar Raven y todos sus binarios, ejecuta:
`./uninstall.sh`

## ⚠️ Descargo de Responsabilidad (Disclaimer)
**Este software se proporciona "tal cual" (AS IS), sin garantía de ningún tipo.** Raven interactúa directamente con el compositor de ventanas (KWin) y el bus de datos del sistema (DBus). El usuario asume toda la responsabilidad derivada de su uso. El autor no se hace responsable de inestabilidades en la sesión gráfica o conflictos con otros scripts del sistema.

---
**Si este proyecto te es útil, considera ayudarme a mejorarlo con feedback o contribuciones. ¡Huélum!**

*Desarrollado por Alejandro González Hernández (Vidruck). Licencia GPL-3.*