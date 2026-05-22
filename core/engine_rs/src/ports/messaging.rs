/// Interfaz (trait) para el envío de comandos físicos al compositor.
///
/// Define las operaciones básicas que la infraestructura debe implementar para
/// materializar las decisiones del motor de mosaico (tiling engine) sobre las ventanas.
pub trait CommandDispatcher {
    /// Envía una orden de movimiento y redimensionamiento de una ventana específica.
    ///
    /// # Parámetros
    /// * `id` - Identificador de la ventana.
    /// * `x` - Coordenada horizontal de destino.
    /// * `y` - Coordenada vertical de destino.
    /// * `w` - Ancho en píxeles.
    /// * `h` - Alto en píxeles.
    fn send_move(&self, id: String, x: i32, y: i32, w: i32, h: i32);
}
