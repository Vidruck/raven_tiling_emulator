#pragma once

#include <QObject>
#include <QDBusAbstractAdaptor>
#include <QString>

class RavenEffect;

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

private:
    RavenEffect *m_effect;
};
