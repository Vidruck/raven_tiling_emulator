#include "raveneffect.h"
#include "dbus/raven_effect_adaptor.h"
#include <effect/effecthandler.h>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QLoggingCategory>
#include <chrono>

Q_LOGGING_CATEGORY(RAVEN_EFFECT, "raven.effect", QtInfoMsg)


RavenEffect::RavenEffect()
    : m_dbusAdaptor(new RavenEffectAdaptor(this))
{
    QDBusConnection dbus = QDBusConnection::sessionBus();
    if (dbus.registerService(QStringLiteral("org.kde.kwin.RavenEffect"))) {
        if (dbus.registerObject(QStringLiteral("/Effects/Raven"), this)) {
            qCInfo(RAVEN_EFFECT) << "RavenEffect D-Bus service registrado exitosamente en /Effects/Raven";
        } else {
            qCWarning(RAVEN_EFFECT) << "Fallo al registrar objeto D-Bus /Effects/Raven";
        }
    } else {
        qCWarning(RAVEN_EFFECT) << "Fallo al registrar servicio D-Bus org.kde.kwin.RavenEffect (¿servicio duplicado?)";
    }
}

RavenEffect::~RavenEffect()
{
    QDBusConnection dbus = QDBusConnection::sessionBus();
    dbus.unregisterObject(QStringLiteral("/Effects/Raven"));
    dbus.unregisterService(QStringLiteral("org.kde.kwin.RavenEffect"));
}

bool RavenEffect::supported()
{
    return KWin::effects && KWin::effects->compositingType() != KWin::NoCompositing;
}

bool RavenEffect::isEffectActive() const
{
    return true;
}

KWin::EffectWindow *RavenEffect::findWindowById(const QString &windowId) const
{
    if (!KWin::effects) {
        return nullptr;
    }

    const auto windows = KWin::effects->stackingOrder();
    for (auto *win : windows) {
        if (!win) continue;
        
        // Match con internal ID o address pointer hex string (0x...)
        QString hexId = QStringLiteral("0x%1").arg(reinterpret_cast<quintptr>(win), 0, 16);
        if (hexId.compare(windowId, Qt::CaseInsensitive) == 0) {
            return win;
        }

        // Match por windowId decimal o address
        if (QString::number(reinterpret_cast<quintptr>(win)).compare(windowId) == 0) {
            return win;
        }

        // Match con internalId QString si estuviera disponible
        if (win->internalId().toString().compare(windowId, Qt::CaseInsensitive) == 0) {
            return win;
        }
    }
    return nullptr;
}

QEasingCurve RavenEffect::parseEasing(const QString &easingName) const
{
    if (easingName == QLatin1String("EaseOutExpo")) {
        return QEasingCurve(QEasingCurve::OutExpo);
    } else if (easingName == QLatin1String("EaseOutCubic")) {
        return QEasingCurve(QEasingCurve::OutCubic);
    } else if (easingName == QLatin1String("EaseOutBack")) {
        return QEasingCurve(QEasingCurve::OutBack);
    } else if (easingName == QLatin1String("EaseOutElastic")) {
        return QEasingCurve(QEasingCurve::OutElastic);
    } else if (easingName == QLatin1String("EaseInOutQuad")) {
        return QEasingCurve(QEasingCurve::InOutQuad);
    } else if (easingName == QLatin1String("Linear")) {
        return QEasingCurve(QEasingCurve::Linear);
    }
    // Efecto ágil y profesional por defecto (rebote sutil)
    return QEasingCurve(QEasingCurve::OutBack);
}

void RavenEffect::animateWindowGeometry(const QString &windowId, const QRectF &startRect, const QRectF &targetRect,
                                        int durationMs, const QString &easingName)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (!w) {
        return;
    }

    cancelWindowAnimation(windowId);

    if (durationMs <= 0) {
        durationMs = 60; // 250ms para un movimiento súper rápido pero fluido
    }

    QEasingCurve curve = parseEasing(easingName);

    uint meta = 0;
    setMetaData(SourceAnchor, Anchor::Left | Anchor::Top, meta);

    // Animación de posición y geometría relativa usando AnimationEffect de KWin
    quint64 animId = animate(w,
                             KWin::AnimationEffect::Position,
                             meta,
                             std::chrono::milliseconds(durationMs),
                             KWin::FPx2(0.0, 0.0),
                             curve,
                             0,
                             KWin::FPx2(startRect.x() - targetRect.x(), startRect.y() - targetRect.y()));

    if (animId != 0) {
        m_activeAnimations[windowId].append(animId);
    }
}

void RavenEffect::animateBatchGeometry(const QString &jsonPayload)
{
    QJsonDocument doc = QJsonDocument::fromJson(jsonPayload.toUtf8());
    if (!doc.isObject()) {
        return;
    }

    QJsonObject root = doc.object();
    QJsonArray anims = root.value(QStringLiteral("animations")).toArray();

    for (const auto &val : anims) {
        if (!val.isObject()) continue;
        QJsonObject obj = val.toObject();

        QString winId = obj.value(QStringLiteral("id")).toString();
        int durationMs = obj.value(QStringLiteral("duration_ms")).toInt(250);
        QString easing = obj.value(QStringLiteral("easing")).toString(QStringLiteral("EaseOutBack"));

        QJsonArray fromArr = obj.value(QStringLiteral("from")).toArray();
        QJsonArray toArr = obj.value(QStringLiteral("to")).toArray();

        if (fromArr.size() >= 4 && toArr.size() >= 4) {
            QRectF from(fromArr[0].toDouble(), fromArr[1].toDouble(), fromArr[2].toDouble(), fromArr[3].toDouble());
            QRectF to(toArr[0].toDouble(), toArr[1].toDouble(), toArr[2].toDouble(), toArr[3].toDouble());
            animateWindowGeometry(winId, from, to, durationMs, easing);
        }
    }
}

void RavenEffect::animateWindowBirth(const QString &windowId, int durationMs)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (!w) {
        return;
    }

    cancelWindowAnimation(windowId);

    if (durationMs <= 0) {
        durationMs = 250; // Efecto rápido y profesional
    }

    QEasingCurve curve(QEasingCurve::OutElastic); // Curva gelatina (rápida)
    QEasingCurve opacityCurve(QEasingCurve::OutCubic); // Fade suave

    uint metaCenter = 0;
    setMetaData(SourceAnchor, Anchor::Horizontal | Anchor::Vertical, metaCenter);

    // Animación de escala (Zoom-in desde 0.85 a 1.0 para que sea ágil y tipo gelatina)
    quint64 scaleAnim = animate(w,
                                KWin::AnimationEffect::Scale,
                                metaCenter,
                                std::chrono::milliseconds(durationMs),
                                KWin::FPx2(1.0, 1.0),
                                curve,
                                0,
                                KWin::FPx2(0.85, 0.85));

    // Animación de opacidad (Fade-in desde 0.0 a 1.0)
    uint metaOpacity = 0;
    quint64 opacityAnim = animate(w,
                                  KWin::AnimationEffect::Opacity,
                                  metaOpacity,
                                  std::chrono::milliseconds(static_cast<int>(durationMs * 0.7)), // Fade un poco más rápido que el rebote
                                  KWin::FPx2(1.0, 1.0),
                                  opacityCurve,
                                  0,
                                  KWin::FPx2(0.0, 0.0));

    if (scaleAnim != 0) m_activeAnimations[windowId].append(scaleAnim);
    if (opacityAnim != 0) m_activeAnimations[windowId].append(opacityAnim);
}

void RavenEffect::cancelWindowAnimation(const QString &windowId)
{
    if (m_activeAnimations.contains(windowId)) {
        const auto anims = m_activeAnimations.take(windowId);
        for (quint64 id : anims) {
            cancel(id);
        }
    }
}

#include "moc_raveneffect.cpp"
