/*
 * animation_controller.cpp
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 */

#include "animation_controller.h"
#include "raveneffect.h"
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <chrono>
#include <cmath>

AnimationController::AnimationController()
{
}

void AnimationController::clear()
{
    m_activeGeometryAnimations.clear();
    m_activeBirthAnimations.clear();
    m_pendingAnimations.clear();
}

bool AnimationController::hasPending(const QString &windowId) const
{
    return m_pendingAnimations.contains(windowId);
}

AnimationController::PendingAnimation AnimationController::takePending(const QString &windowId)
{
    return m_pendingAnimations.take(windowId);
}

void AnimationController::removePending(const QString &windowId)
{
    m_pendingAnimations.remove(windowId);
}

void AnimationController::storePending(const QString &windowId, const PendingAnimation &anim)
{
    m_pendingAnimations.insert(windowId, anim);
}

bool AnimationController::hasActiveBirth(const QString &windowId) const
{
    return m_activeBirthAnimations.contains(windowId);
}

QEasingCurve AnimationController::parseEasing(const QString &easingName) const
{
    if (easingName == QLatin1String("EaseOutExpo")) {
        return QEasingCurve(QEasingCurve::OutExpo);
    } else if (easingName == QLatin1String("EaseOutCubic")) {
        return QEasingCurve(QEasingCurve::OutCubic);
    } else if (easingName == QLatin1String("EaseOutBack")) {
        return QEasingCurve(QEasingCurve::OutBack);
    } else if (easingName == QLatin1String("EaseOutElastic") || easingName == QLatin1String("Jelly")) {
        QEasingCurve elastic(QEasingCurve::OutElastic);
        elastic.setPeriod(0.25);
        elastic.setAmplitude(1.05);
        return elastic;
    } else if (easingName == QLatin1String("EaseInOutQuad")) {
        return QEasingCurve(QEasingCurve::InOutQuad);
    } else if (easingName == QLatin1String("Linear")) {
        return QEasingCurve(QEasingCurve::Linear);
    }
    return QEasingCurve(QEasingCurve::OutCubic);
}

void AnimationController::cancelGeometryAnimation(RavenEffect *effect, const QString &windowId)
{
    if (m_activeGeometryAnimations.contains(windowId)) {
        const auto anims = m_activeGeometryAnimations.take(windowId);
        if (effect) {
            for (quint64 id : anims) {
                effect->cancel(id);
            }
        }
    }
}

void AnimationController::cancelBirthAnimation(RavenEffect *effect, const QString &windowId)
{
    if (m_activeBirthAnimations.contains(windowId)) {
        const auto anims = m_activeBirthAnimations.take(windowId);
        if (effect) {
            for (quint64 id : anims) {
                effect->cancel(id);
            }
        }
    }
}

void AnimationController::cancelAnimation(RavenEffect *effect, const QString &windowId)
{
    cancelGeometryAnimation(effect, windowId);
    cancelBirthAnimation(effect, windowId);
}

void AnimationController::onAnimationEnded(KWin::EffectWindow *w, KWin::AnimationEffect::Attribute a)
{
    if (!w) return;

    const QString windowId = w->internalId().toString();
    const QString hexId = QStringLiteral("0x%1").arg(reinterpret_cast<quintptr>(w), 0, 16);
    const QString decId = QString::number(reinterpret_cast<quintptr>(w));

    if (a == KWin::AnimationEffect::Scale) {
        for (const QString &key : {windowId, hexId, decId}) {
            if (!key.isEmpty()) {
                m_activeGeometryAnimations.remove(key);
                m_activeBirthAnimations.remove(key);
            }
        }
    }

    w->addRepaintFull();
}

void AnimationController::animateGeometry(RavenEffect *effect, KWin::EffectWindow *w, const QString &windowId,
                                         const QRectF &startRect, const QRectF &targetRect, int durationMs, const QString &easingName)
{
    if (!effect || !w || w->isDeleted() || w->isHidden() || w->isMinimized()) {
        return;
    }

    cancelGeometryAnimation(effect, windowId);

    if (durationMs <= 0 || durationMs > 500) {
        durationMs = 130;
    }

    const QRectF currentGeom = w->frameGeometry();

    if (currentGeom != targetRect) {
        m_pendingAnimations[windowId] = { startRect, targetRect, durationMs, easingName };
        return;
    }

    m_pendingAnimations.remove(windowId);
    QEasingCurve curve = parseEasing(easingName);

    qreal fromX = startRect.x() - currentGeom.x();
    qreal fromY = startRect.y() - currentGeom.y();

    if (std::abs(fromX) > 0.5 || std::abs(fromY) > 0.5) {
        quint64 posAnim = effect->animate(w,
                                          KWin::AnimationEffect::Translation,
                                          0,
                                          std::chrono::milliseconds(durationMs),
                                          KWin::FPx2(0.0, 0.0),
                                          curve,
                                          0,
                                          KWin::FPx2(fromX, fromY),
                                          false,
                                          false);

        if (posAnim != 0) {
            m_activeGeometryAnimations[windowId].append(posAnim);
        }
    }

    if (!m_activeBirthAnimations.contains(windowId) &&
        targetRect.width() > 0.0 && targetRect.height() > 0.0 &&
        startRect.width() > 0.0 && startRect.height() > 0.0 &&
        currentGeom.width() > 0.0 && currentGeom.height() > 0.0) {

        qreal fromScaleX = startRect.width() / currentGeom.width();
        qreal fromScaleY = startRect.height() / currentGeom.height();

        if (std::abs(fromScaleX - 1.0) > 0.005 || std::abs(fromScaleY - 1.0) > 0.005) {
            uint metaScale = 0;
            effect->setMetaData(KWin::AnimationEffect::SourceAnchor,
                                KWin::AnimationEffect::Anchor::Left | KWin::AnimationEffect::Anchor::Top,
                                metaScale);

            quint64 scaleAnim = effect->animate(w,
                                                KWin::AnimationEffect::Scale,
                                                metaScale,
                                                std::chrono::milliseconds(durationMs),
                                                KWin::FPx2(1.0, 1.0),
                                                curve,
                                                0,
                                                KWin::FPx2(fromScaleX, fromScaleY),
                                                false,
                                                false);

            if (scaleAnim != 0) {
                m_activeGeometryAnimations[windowId].append(scaleAnim);
            }
        }
    }

    w->addRepaintFull();
}

void AnimationController::animateBatch(RavenEffect *effect, const QString &jsonPayload,
                                      const std::function<KWin::EffectWindow*(const QString&)> &findWindow)
{
    if (jsonPayload.isEmpty()
        || jsonPayload == QLatin1String("[]")
        || jsonPayload == QLatin1String("{}")) {
        return;
    }

    QJsonDocument doc = QJsonDocument::fromJson(jsonPayload.toUtf8());
    if (!doc.isObject()) {
        return;
    }

    QJsonObject root = doc.object();
    QJsonArray anims = root.value(QStringLiteral("animations")).toArray();
    if (anims.isEmpty()) {
        return;
    }

    for (const auto &val : anims) {
        if (!val.isObject()) continue;
        QJsonObject obj = val.toObject();

        QString winId = obj.value(QStringLiteral("id")).toString();
        int durationMs = obj.value(QStringLiteral("duration_ms")).toInt(130);
        QString easing = obj.value(QStringLiteral("easing")).toString(QStringLiteral("EaseOutCubic"));

        QJsonArray fromArr = obj.value(QStringLiteral("from")).toArray();
        QJsonArray toArr = obj.value(QStringLiteral("to")).toArray();

        if (fromArr.size() >= 4 && toArr.size() >= 4) {
            QRectF from(fromArr[0].toDouble(), fromArr[1].toDouble(), fromArr[2].toDouble(), fromArr[3].toDouble());
            QRectF to(toArr[0].toDouble(), toArr[1].toDouble(), toArr[2].toDouble(), toArr[3].toDouble());
            if (auto *w = findWindow(winId)) {
                animateGeometry(effect, w, winId, from, to, durationMs, easing);
            }
        }
    }
}

void AnimationController::animateBirth(RavenEffect *effect, KWin::EffectWindow *w, int durationMs)
{
    if (!effect || !w || w->isDeleted() || w->isHidden() || w->isMinimized()) {
        return;
    }

    const QString windowId = w->internalId().toString();
    cancelBirthAnimation(effect, windowId);

    if (durationMs <= 0) {
        durationMs = 180;
    }

    QEasingCurve curve(QEasingCurve::OutBack);
    QEasingCurve opacityCurve(QEasingCurve::OutCubic);

    uint metaCenter = 0;
    effect->setMetaData(KWin::AnimationEffect::SourceAnchor,
                        KWin::AnimationEffect::Anchor::Horizontal | KWin::AnimationEffect::Anchor::Vertical,
                        metaCenter);

    quint64 scaleAnim = effect->animate(w,
                                        KWin::AnimationEffect::Scale,
                                        metaCenter,
                                        std::chrono::milliseconds(durationMs),
                                        KWin::FPx2(1.0, 1.0),
                                        curve,
                                        0,
                                        KWin::FPx2(0.88, 0.88),
                                        false,
                                        false);

    uint metaOpacity = 0;
    quint64 opacityAnim = effect->animate(w,
                                          KWin::AnimationEffect::Opacity,
                                          metaOpacity,
                                          std::chrono::milliseconds(static_cast<int>(durationMs * 0.85)),
                                          KWin::FPx2(1.0, 1.0),
                                          opacityCurve,
                                          0,
                                          KWin::FPx2(0.0, 0.0),
                                          false,
                                          false);

    if (scaleAnim != 0) m_activeBirthAnimations[windowId].append(scaleAnim);
    if (opacityAnim != 0) m_activeBirthAnimations[windowId].append(opacityAnim);
}

