/*
 * raveneffect.h
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 * 
 * Descripción: Declaración del efecto nativo de KWin para Raven Tiling Emulator.
 * Maneja las animaciones de movimiento, escalado y transparencia utilizando las
 * utilidades nativas de KWin (AnimationEffect).
 */

#pragma once

#include <effect/animationeffect.h>
#include <QHash>
#include <QRectF>
#include <QEasingCurve>
#include <QDBusConnection>

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
     * @param easingName Nombre de la curva de aceleración (ej. "EaseOutBack").
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
     * @brief Comprueba si este efecto debe permanecer activo.
     * @return true si existen animaciones en progreso.
     */
    bool isActive() const override;

protected:
    /**
     * @brief Invocado por KWin cuando una animación concluye de forma natural.
     * @param w Ventana animada.
     * @param a Atributo animado.
     * @param meta Metadatos asociados.
     */
     void animationEnded(KWin::EffectWindow *w, Attribute a, uint meta) override;

private Q_SLOTS:
    /**
     * @brief Slot conectado a KWin::effects->windowAdded para iniciar animación de nacimiento y emitir D-Bus.
     */
    void slotWindowAdded(KWin::EffectWindow *w);

private:
    /**
     * @brief Busca de manera segura un EffectWindow en el stack usando su ID interno.
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

    RavenEffectAdaptor *m_dbusAdaptor;                      ///< Adaptador para exponer métodos en D-Bus
    QHash<QString, QList<quint64>> m_activeAnimations;      ///< Mapeo de IDs de ventanas a sus animaciones en curso
};

