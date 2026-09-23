#pragma once

#include <effect/animationeffect.h>
#include <QHash>
#include <QRectF>
#include <QEasingCurve>
#include <QDBusConnection>

class RavenEffectAdaptor;

class RavenEffect : public KWin::AnimationEffect
{
    Q_OBJECT

public:
    RavenEffect();
    ~RavenEffect() override;

    static bool supported();

    void animateWindowGeometry(const QString &windowId, const QRectF &startRect, const QRectF &targetRect,
                               int durationMs, const QString &easingName);
    void animateBatchGeometry(const QString &jsonPayload);
    void animateWindowBirth(const QString &windowId, int durationMs);
    void cancelWindowAnimation(const QString &windowId);
    bool isEffectActive() const;

private:
    KWin::EffectWindow *findWindowById(const QString &windowId) const;
    QEasingCurve parseEasing(const QString &easingName) const;

    RavenEffectAdaptor *m_dbusAdaptor;
    QHash<QString, QList<quint64>> m_activeAnimations;
};
