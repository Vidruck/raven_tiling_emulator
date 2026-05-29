# Contribuyendo a Raven 🐦

¡Gracias por el interés en mejorar Raven! Con el lanzamiento de la **v2.7**, el proyecto ha evolucionado hacia una gestión de **Composición Foveal Dinámica (Dynamic Foveal Composition)** construida sobre un ecosistema **100% nativo en Rust**. Estamos encantados de recibir colaboraciones que impulsen la estabilidad, el rendimiento y la experiencia de usuario en KDE Plasma 6.

## 🏗️ Filosofía de Diseño v2.7
Para mantener la robustez, fluidez y ligereza logradas, todas las contribuciones deben respetar estos pilares arquitectónicos:

1. **Ecosistema Nativo en Rust:**
   - **Core Asíncrono (core/engine_rs):** Toda la lógica de cálculo y la comunicación IPC reside en Rust. Utilizamos `zbus` para una integración asíncrona y de ultra-baja latencia con el bus de datos del sistema.
   - **Interfaz Nativa (raven_gui):** La UI se construye con `egui/eframe`, garantizando un consumo mínimo de recursos y una integración fluida con el compositor.

2. **Paradigma de Composición Foveal Dinámica (Dynamic Foveal Composition Paradigm):**
   - El motor organiza el espacio situando el foco activo en un *Centro Foveal* principal, flanqueado por paneles laterales de contexto simétricos y consolas inferiores de utilidad.
   - Toda modificación al layout debe respetar las subdivisiones jerárquicas recursivas (fraccionando los paneles laterales antes del Centro Foveal) y el reinicio automático del ratio a `0.5` en cuanto la composición sufre adiciones o supresiones.
   - El redimensionamiento asimétrico mediante `master_ratio` debe estar ligado estrictamente al foco activo, manteniendo las divisiones no enfocadas en una proporción simétrica de 0.5.
   - Es mandatorio proteger al motor contra evicciones (desalojos) accidentales causadas por límites de tamaño mínimos sobredimensionados en Wayland. Se debe aplicar el acotamiento de seguridad a `300x250` px en las comprobaciones.
   - La redistribución del espacio debe ocurrir a través de un bucle de recálculo recursivo que prevenga huecos negros y ocupe el 100% de la superficie disponible para las ventanas no desalojadas.

3. **Snapshot-Based Synchronization:**
   - Mantenemos el modelo de **Consistencia Eventual.** El Bridge (JS) envía el estado estructural que el daemon de Rust procesa de forma atómica para generar los comandos de posicionamiento.

4. **Debounced Sensing:**
   - El Bridge no debe reaccionar instantáneamente a eventos de geometría intermedios (como durante un redimensionado manual). Debe esperar a que la interacción finalice para sincronizar el estado, protegiendo la CPU y la estabilidad de `kwin_wayland`.

5. **Seguridad y Rendimiento Extremo:**
   - **Cero Costo:** Buscamos abstracciones de costo cero. Evita clonaciones innecesarias de datos en el motor.
   - **Rust Idiomático:** Favorecemos el uso de tipos seguros y el manejo de errores robusto (Result/Option). El uso de `unsafe` está estrictamente prohibido a menos que se justifique por interoperabilidad crítica con APIs de bajo nivel del sistema.

6. **Optimización de Peso (Binary Thinning):**
   - El minimalismo en el binario final es un requisito de diseño. Se exige a los colaboradores buscar la reducción máxima del peso en ROM, evaluando críticamente la inclusión de dependencias y sus features. El objetivo es mantener el footprint lo más bajo posible para el usuario final.

## 🚀 Cómo colaborar
1. **Reporte de Bugs:** Si encuentras un comportamiento extraño en Wayland, abre un *Issue* describiendo tu hardware, versión de Plasma y adjunta los logs del daemon si es posible (`journalctl --user -u raven`).
2. **Pull Requests:**
   - Crea una rama descriptiva (`feature/nueva-mejora` o `fix/error-especifico`).
   - Asegúrate de que tu código pase las pruebas de sanidad: `cargo check` y `cargo clippy`.
   - Documenta cualquier cambio en la interfaz DBus o en la estructura de configuración.

## 🛠️ Requisitos de Desarrollo *(Stack)*
Para compilación y pruebas necesitas:
- **Rust Toolchain:** Edición 2021 o superior.
- **Librerías de Desarrollo:** `libwayland`, `libx11`, `libxkbcommon` (requeridas por la interfaz gráfica).
- **Herramientas de KDE:** `kpackagetool6` y `kbuildsycoca6` para probar los adaptadores.

## 📝 Estándares de Código
- **Rust:** Formateo obligatorio con `cargo fmt`. Se recomienda encarecidamente seguir las sugerencias de `clippy` para un código más limpio y eficiente. Documenta las funciones públicas utilizando comentarios de documentación (`///`).
- **JavaScript (Bridge):** El código debe ser compatible con `QJSEngine` de Plasma 6.
    - **Native API First:** Prohibido el filtrado manual si existe una propiedad nativa (ej. `w.notification` vs filtrar por clase).
    - **Inmutabilidad de IDs:** El rastreo de ventanas es exclusivo mediante `w.internalId.toString()`.
    - **Atomicidad:** Los comandos desde Rust deben ser atómicos para evitar conflictos con el compositor.

---

**Tu ayuda no solo mejora a Raven, me ayuda a mí a ser un mejor ingeniero. Hagamos de Raven el Tiling Engine más rápido y elegante para KDE. ¡Huélum!**
