# Raven Tiling Emulator 🐦


<p align="center">
  <img src="icon/org.kde.raven.tiling.svg" width="250" alt="Raven Logo">
</p>

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![JavaScript](https://img.shields.io/badge/javascript-%23323330.svg?style=for-the-badge&logo=javascript&logoColor=%23F7DF1E)
![KDE](https://img.shields.io/badge/KDE%20Plasma-21D359?style=for-the-badge&logo=kde&logoColor=white)
![Wayland](https://img.shields.io/badge/Wayland-9999ff?style=for-the-badge&logo=wayland&logoColor=white)
![GPLv3](https://img.shields.io/badge/License-GPLv3-blue.svg?style=for-the-badge)

Raven es un gestor de ventanas dinámico (Tiling Window Manager) diseñado específicamente para **KDE Plasma 6 (Wayland)**. Con la llegada de la **versión 2.6**, Raven se reestructura y ajusta a los estándares de la **Arquitectura Hexagonal**, logrando el máximo desacoplamiento entre su lógica de negocio y la infraestructura del sistema operativo, a la vez que introduce optimizaciones agresivas de estabilidad y contención de recursos.

## 🚀 El Salto a la Versión 2.6: Arquitectura Hexagonal y Micro-Optimización
Esta versión se enfoca en la robustes estructural y la corrección de fallos críticos, garantizando una experiencia inquebrantable en entornos multi-monitor:

### 📉 Eficiencia Energética y de Almacenamiento
La optimización sigue siendo el pilar fundamental. El motor opera con recursos sumamente bajos y contenidos:

| Versión | Arquitectura | RAM (Runtime) | ROM (Binario) |
|---|---|---|---|
| **v1.0** | Python Puro | 55.0 MB | ~15 MB |
| **v1.6** | Híbrida (Python + Rust FFI) | ~25.9 MB | ~18 MB |
| **v2.6** | ***Asynchronous Native Rust** | **~4.3 MB** | **1.4 MB** |

*La eficiencia extrema ha sido una directriz arquitectónica fundamental desde el inicio del proyecto. Tras validaciones exhaustivas en hardware real, se logró consolidar un motor de alto rendimiento que minimiza el impacto en recursos. Gracias al uso de LTO, el pruning de dependencias y la eliminación de símbolos, entregamos un binario ultra-compacto sin comprometer la estabilidad.*

## 🌟 Nuevas Funciones y Estabilidad (v2.6+)
- **Resiliencia y Comunicación Asíncrona:** El puente KWin-Raven ahora es completamente no bloqueante. El motor Rust utiliza offloading asíncrono con `tokio::spawn` para liberar el bus de datos instantáneamente.
- **Estabilización de Ventanas Adaptativa:** Incorporación de un motor de detección por *Silencio Geométrico*. Las aplicaciones complejas (como Firefox, Floorp o Zen Browser) se mantienen en cuarentena dinámica hasta que cesan sus micro-redimensionamientos internos de Wayland, logrando un mosaico limpio y libre de parpadeos.
- **Gestión de Desbordamiento Inteligente:** Raven detecta matemáticamente la saturación de pantallas y escritorios. En lugar de fallos de layout, minimiza automáticamente las ventanas excedentes y lanza una notificación al usuario.
- **Robustez en el Adaptador JavaScript:** Refactorización del bridge con manejo defensivo de errores (`try/catch`) y filtrado atómico de señales (`__raven_ui_migrating`), garantizando que el entorno gráfico permanezca estable e inmune a efectos de rebote.
-**Envio de Ventanas mediante Toggle:** El toggle permite enviar la ultima ventana en foco a el monitor alterno para comodidad del usuario.
-**Soporte a traspaso de ventanas via arrastre:** Se incluye capacidad de arrastrar con el ratón la ventana a otro monitor o escritorio virtual disponible con reacomodo instantaneo de la composición. 

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
El nuevo instalador gestiona la descarga de crates de Rust y la compilación optimizada de los componentes nativos.

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

### Atajos Predeterminados
| Tecla | Acción |
|---|---|
| `Meta + I / D` | Incrementar/Disminuir ventanas maestras |
| `Meta + L / H` | Ajustar ratio del área maestra (Ancho) |
| `Meta + J / K` | Cambiar foco entre ventanas del stack |
| `Meta + G` | Alternar (Toggle) el motor de mosaico globalmente |

## ⚠️ Descargo de Responsabilidad (Disclaimer)
**Este software se proporciona "tal cual" (AS IS), sin garantía de ningún tipo.** Raven interactúa directamente con el compositor de ventanas (KWin) y el bus de datos del sistema (DBus). El usuario asume toda la responsabilidad derivada de su uso. El autor no se hace responsable de inestabilidades en la sesión gráfica o conflictos con otros scripts del sistema.

---
**Si este proyecto te es útil, considera ayudarme a mejorarlo con feedback o contribuciones. ¡Huélum!**

*Desarrollado por Alejandro González Hernández (Vidruck). Licencia GPL-3.*