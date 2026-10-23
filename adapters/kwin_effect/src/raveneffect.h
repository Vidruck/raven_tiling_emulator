/*
 * raveneffect.h
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 * 
 * Descripción: Declaración del efecto nativo de KWin para Raven Tiling Emulator.
 * Maneja las animaciones de movimiento, escalado y transparencia utilizando las
 * utilidades nativas de KWin (AnimationEffect) y contornos mediante OutlinedBorderItem.
 */

#pragma once

#include <effect/animationeffect.h>
#include <scene/windowitem.h>

#if __has_include(<scene/outlinedborderitem.h>)
#define HAVE_KWIN_OUTLINED_BORDER 1
#include <scene/outlinedborderitem.h>
#include <scene/borderoutline.h>
#include <scene/borderradius.h>
#elif __has_include(<kwin/scene/outlinedborderitem.h>)
#define HAVE_KWIN_OUTLINED_BORDER 1
#include <kwin/scene/outlinedborderitem.h>
#include <kwin/scene/borderoutline.h>
#include <kwin/scene/borderradius.h>
#endif

#include <QHash>
#include <QRectF>
#include <QEasingCurve>
#include <QDBusConnection>
#include <QPointer>
#include <QUuid>
#include <memory>
#include <unordered_map>

namespace KWin {
class RenderView;
}

class RavenEffectAdaptor;

/**
 * @brief Efecto nativo de animación para KWin que provee transiciones fluidas.
 * 
 * Esta clase hereda de KWin::AnimationEffect y expone métodos a través de D-Bus
 * para ser controlados externamente por el motor de diseño en Rust.
 */
class RavenEffect : public KWin::AnimationEffect
{
    Q_OBJECT

public:
    /**
     * @brief Constructor por defecto. Configura la conexión D-Bus.
     */
    RavenEffect();

    /**
     * @brief Destructor. Limpia los adaptadores y animaciones pendientes.
     */
    ~RavenEffect() override;

    /**
     * @brief Comprueba si el efecto está soportado por la configuración actual del compositor.
     * @return true si es compatible.
     */
    static bool supported();

    /**
     * @brief Anima el cambio de geometría (posición y tamaño) de una ventana.
     * @param windowId ID interno (QUuid string) de la ventana.
     * @param startRect Geometría inicial.
     * @param targetRect Geometría objetivo final.
     * @param durationMs Duración de la animación en milisegundos.
     * @param easingName Nombre de la curva de aceleración (ej. "EaseOutCubic").
     */
    void animateWindowGeometry(const QString &windowId, const QRectF &startRect, const QRectF &targetRect,
                               int durationMs, const QString &easingName);

    /**
     * @brief Procesa un lote de animaciones a partir de un objeto JSON.
     * @param jsonPayload Cadena JSON que contiene un array de animaciones.
     */
    void animateBatchGeometry(const QString &jsonPayload);

    /**
     * @brief Ejecuta la animación de nacimiento (apertura) para una nueva ventana.
     * @param windowId ID interno (QUuid string) de la ventana.
     * @param durationMs Duración en milisegundos para el efecto de escalado/fade.
     */
    void animateWindowBirth(const QString &windowId, int durationMs);

    /**
     * @brief Cancela de manera inmediata cualquier animación activa sobre la ventana especificada.
     * @param windowId ID de la ventana.
     */
    void cancelWindowAnimation(const QString &windowId);

    /**
     * @brief Actualiza la configuración de los contornos activo/inactivo en tiempo real.
     */
    void updateBorderConfig(bool enabled, double width, const QString &activeColor, const QString &inactiveColor, double radius = -1.0);

    /**
     * @brief Modifica el radio de curvatura de las esquinas (-1.0 = auto-adaptativo).
     */
    void setBorderRadius(double radius);

    /**
     * @brief Detecta adaptativamente la curvatura de esquinas configurada en el sistema (6px, 8px, 12px, etc.).
     */
    qreal detectSystemCornerRadius() const;

    /**
     * @brief Resuelve la curvatura final para una ventana (reporte nativo o adaptativo global).
     */
    KWin::BorderRadius resolveAdaptiveBorderRadius(KWin::EffectWindow *w) const;

    /**
     * @brief Comprueba si este efecto debe permanecer activo en el ciclo de dibujado.
     * @return true si existen animaciones en progreso.
     */
    bool isActive() const override;

protected:
    /**
     * @brief Invocado por KWin antes de pintar cada ventana. Garantiza contorno activo al instante.
     */
    void prePaintWindow(KWin::RenderView *view, KWin::EffectWindow *w, KWin::WindowPrePaintData &data) override;

    /**
     * @brief Invocado por KWin cuando una animación concluye de forma natural.
     * @param w Ventana animada.
     * @param a Atributo animado.
     * @param meta Metadatos asociados.
     */
    void animationEnded(KWin::EffectWindow *w, Attribute a, uint meta) override;

private Q_SLOTS:
    /**
     * @brief Slot conectado a KWin::effects->windowAdded para registrar la ventana y configurar bordes.
     */
    void slotWindowAdded(KWin::EffectWindow *w);

    /**
     * @brief Slot conectado a KWin::effects->windowClosed para purgar animaciones y bordes.
     */
    void slotWindowClosed(KWin::EffectWindow *w);

    /**
     * @brief Slot conectado a KWin::effects->windowActivated para actualizar estilos de foco de contornos.
     */
    void slotWindowActivated(KWin::EffectWindow *w);

    /**
     * @brief Slot conectado a cambios de geometría de ventanas para reajustar dimensiones del contorno.
     */
    void slotWindowGeometryChanged(KWin::EffectWindow *w, const KWin::RectF &oldGeometry);

    /**
     * @brief Slot conectado a movimientos/redimensionados interactivos del usuario.
     */
    void slotWindowStepUserMovedResized(KWin::EffectWindow *w, const KWin::RectF &geometry);

    /**
     * @brief Slot conectado al cambio de estado minimizado de ventanas para mostrar u ocultar contornos.
     */
    void slotWindowMinimizedChanged(KWin::EffectWindow *w);

private:
    /**
     * @brief Determina si una ventana es apta para recibir contorno de foco o inactivo.
     */
    bool shouldDrawBorder(KWin::EffectWindow *w) const;

    /**
     * @brief Crea o actualiza el OutlinedBorderItem de la ventana especificada siguiendo su curvatura real.
     */
    void updateBorder(KWin::EffectWindow *w);

    /**
     * @brief Destruye y desvincula el OutlinedBorderItem de la ventana especificada.
     */
    void removeBorder(KWin::EffectWindow *w);

    /**
     * @brief Busca de manera segura un EffectWindow en el stack usando su ID interno (O(1) por UUID).
     * @param windowId String representando la ID de la ventana en D-Bus.
     * @return Puntero a KWin::EffectWindow, o nullptr si no se encuentra.
     */
    KWin::EffectWindow *findWindowById(const QString &windowId) const;

    /**
     * @brief Convierte un string descriptivo en un objeto QEasingCurve nativo.
     * @param easingName Nombre de la curva.
     * @return Objeto QEasingCurve correspondiente.
     */
    QEasingCurve parseEasing(const QString &easingName) const;

    /**
     * @brief Carga la configuración inicial desde ~/.config/raven/raven.json si está disponible.
     */
    void loadBorderConfig();

    void setupWindowConnections(KWin::EffectWindow *w);
    void cancelGeometryAnimation(const QString &windowId);
    void cancelBirthAnimation(const QString &windowId);

    RavenEffectAdaptor *m_dbusAdaptor;                      ///< Adaptador para exponer métodos en D-Bus
    QHash<QString, QList<quint64>> m_activeGeometryAnimations; ///< Mapeo de IDs a animaciones de geometría/traslación
    QHash<QString, QList<quint64>> m_activeBirthAnimations;    ///< Mapeo de IDs a animaciones de nacimiento (scale/fade)
    QHash<QUuid, KWin::EffectWindow*> m_windowByUuid;       ///< Mapeo rápido O(1) de UUID a EffectWindow

    struct PendingAnimation {
        QRectF startRect;
        QRectF targetRect;
        int durationMs = 130;
        QString easing = QStringLiteral("EaseOutCubic");
    };
    QHash<QString, PendingAnimation> m_pendingAnimations;   ///< Animaciones FLIP pendientes de sincronizar con geometryChanged

    // Configuración y estado de contornos de ventana
    bool m_bordersEnabled = true;                           ///< Habilita el dibujado de contornos
    float m_borderWidth = 2.0f;                             ///< Grosor del contorno (px)
    QColor m_activeBorderColor = QColor(61, 174, 233, 230); ///< Color del contorno en la ventana enfocada
    QColor m_inactiveBorderColor = QColor(90, 101, 114, 160);///< Color del contorno en ventanas inactivas
    qreal m_customBorderRadius = -1.0;                      ///< Radio manual (-1.0 = auto-adaptativo)

    // Estructura para gestión reactiva de bordes mediante el scene graph nativo de KWin
    struct BorderData {
#ifdef HAVE_KWIN_OUTLINED_BORDER
        std::unique_ptr<KWin::OutlinedBorderItem> item;
#endif
        QMetaObject::Connection radiusConnection;
        QMetaObject::Connection classConnection;
        QMetaObject::Connection captionConnection;
        QMetaObject::Connection decorationConnection;
        QMetaObject::Connection readyConnection;
        QMetaObject::Connection surfaceConnection;
        QMetaObject::Connection bufferConnection;
        QMetaObject::Connection maximizeConnection;
    };
    std::unordered_map<KWin::EffectWindow*, BorderData> m_borders;       ///< Contornos activos administrados por hardware
    bool computeShouldDrawBorder(KWin::EffectWindow *w) const;
    void invalidateBorderVerdict(const KWin::EffectWindow *w);
    mutable QHash<const KWin::EffectWindow*, bool> m_borderVerdictCache; ///< Caché O(1) de elegibilidad para no reevaluar en cada frame

    QPointer<KWin::EffectWindow> m_lastActiveWindow;        ///< Rastreo seguro de la ventana previamente activa
};
