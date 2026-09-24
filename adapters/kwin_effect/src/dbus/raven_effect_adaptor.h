/*
 * raven_effect_adaptor.h
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 * 
 * Descripción: Declaración del adaptador D-Bus para el efecto de KWin.
 * Expone métodos en el bus de sesión para que el motor en Rust se comunique
 * con el efecto en C++.
 */

#pragma once

#include <QObject>
#include <QDBusAbstractAdaptor>
#include <QString>

class RavenEffect;

/**
 * @brief Adaptador D-Bus para la clase RavenEffect.
 * 
 * Esta clase expone una interfaz D-Bus en "org.kde.kwin.RavenEffect" para
 * permitir al motor backend (Rust) invocar directamente métodos de animación
 * en el plugin C++ de KWin.
 */
class RavenEffectAdaptor : public QDBusAbstractAdaptor
{
    Q_OBJECT
    Q_CLASSINFO("D-Bus Interface", "org.kde.kwin.RavenEffect")

public:
    explicit RavenEffectAdaptor(RavenEffect *parent);
    ~RavenEffectAdaptor() override = default;

public Q_SLOTS:
    /**
     * @brief Anima una ventana individual desde su posición inicial hasta la final.
     */
    void AnimateGeometry(const QString &windowId, int startX, int startY, int startW, int startH,
                         int targetX, int targetY, int targetW, int targetH, int durationMs, const QString &easing);

    /**
     * @brief Anima un lote (batch) de ventanas en mosaico simultáneamente.
     */
    void AnimateBatch(const QString &jsonPayload);

    /**
     * @brief Ejecuta el efecto de zoom/fade de nacimiento para una nueva ventana.
     */
    void AnimateBirth(const QString &windowId, int durationMs);

    /**
     * @brief Cancela animaciones activas para una ventana específica.
     */
    void CancelAnimation(const QString &windowId);

    /**
     * @brief Consulta el estado de activación y capacidades del efecto.
     */
    bool IsActive() const;

Q_SIGNALS:
    /**
     * @brief Señal emitida cuando el efecto detecta y comienza la animación de nacimiento de una ventana.
     * @param windowId Identificador interno o QUuid de la ventana.
     */
    void WindowBirthStarted(const QString &windowId);

private:
    RavenEffect *m_effect;
};
