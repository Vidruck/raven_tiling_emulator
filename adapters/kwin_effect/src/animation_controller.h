/*
 * animation_controller.h
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 *
 * Descripción: Subsistema especializado en la orquestación y ciclo de vida de animaciones
 * FLIP (Geometry Translation, Scaling) y de nacimiento (Birth Fade/Scale) para RavenEffect en KWin 6.
 */

#pragma once

#include <effect/animationeffect.h>
#include <effect/effectwindow.h>
#include <QEasingCurve>
#include <QHash>
#include <QList>
#include <QRectF>
#include <QString>
#include <functional>

class RavenEffect;

class AnimationController
{
public:
    struct PendingAnimation {
        QRectF startRect;
        QRectF targetRect;
        int durationMs = 130;
        QString easing = QStringLiteral("EaseOutCubic");
    };

    AnimationController();
    ~AnimationController() = default;

    QEasingCurve parseEasing(const QString &easingName) const;

    void animateGeometry(RavenEffect *effect, KWin::EffectWindow *w, const QString &windowId,
                         const QRectF &startRect, const QRectF &targetRect, int durationMs, const QString &easingName);

    void animateBatch(RavenEffect *effect, const QString &jsonPayload,
                      const std::function<KWin::EffectWindow*(const QString&)> &findWindow);

    void animateBirth(RavenEffect *effect, KWin::EffectWindow *w, int durationMs = 180);

    void cancelAnimation(RavenEffect *effect, const QString &windowId);
    void cancelGeometryAnimation(RavenEffect *effect, const QString &windowId);
    void cancelBirthAnimation(RavenEffect *effect, const QString &windowId);

    void onAnimationEnded(KWin::EffectWindow *w, KWin::AnimationEffect::Attribute a);

    bool hasPending(const QString &windowId) const;
    PendingAnimation takePending(const QString &windowId);
    void removePending(const QString &windowId);
    void storePending(const QString &windowId, const PendingAnimation &anim);

    bool hasActiveBirth(const QString &windowId) const;

    void clear();

private:
    QHash<QString, QList<quint64>> m_activeGeometryAnimations;
    QHash<QString, QList<quint64>> m_activeBirthAnimations;
    QHash<QString, PendingAnimation> m_pendingAnimations;
};

