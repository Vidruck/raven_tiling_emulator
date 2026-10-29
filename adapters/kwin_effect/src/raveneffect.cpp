/*
 * raveneffect.cpp
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 *
 * Descripción: Implementación de la Fachada (Facade) RavenEffect en KWin 6.
 * Coordina los subsistemas WindowFilter, BorderManager y AnimationController
 * con una arquitectura de bajo acoplamiento y libre de clases monolíticas.
 */

#include "raveneffect.h"
#include "dbus/raven_effect_adaptor.h"
#include <effect/effecthandler.h>
#include <window.h>
#include <QDBusConnection>
#include <QLoggingCategory>

Q_LOGGING_CATEGORY(RAVEN_EFFECT, "raven.effect", QtInfoMsg)

RavenEffect::RavenEffect()
    : m_dbusAdaptor(new RavenEffectAdaptor(this))
    , m_lastActiveWindow(nullptr)
{
    QDBusConnection dbus = QDBusConnection::sessionBus();
    if (!dbus.registerService(QStringLiteral("org.kde.kwin.RavenEffect"))) {
        qCWarning(RAVEN_EFFECT) << "No se pudo registrar el servicio D-Bus org.kde.kwin.RavenEffect";
    }

    if (!dbus.registerObject(QStringLiteral("/Effects/Raven"), this)) {
        qCWarning(RAVEN_EFFECT) << "No se pudo registrar el objeto D-Bus /Effects/Raven";
    } else {
        qCInfo(RAVEN_EFFECT) << "RavenEffect D-Bus registrado exitosamente en /Effects/Raven";
    }

    if (KWin::effects) {
        connect(KWin::effects, &KWin::EffectsHandler::windowAdded, this, &RavenEffect::slotWindowAdded);
        connect(KWin::effects, &KWin::EffectsHandler::windowClosed, this, &RavenEffect::slotWindowClosed);
        connect(KWin::effects, &KWin::EffectsHandler::windowDeleted, this, &RavenEffect::slotWindowDeleted);
        connect(KWin::effects, &KWin::EffectsHandler::windowActivated, this, &RavenEffect::slotWindowActivated);

        const auto windows = KWin::effects->stackingOrder();
        for (auto *w : windows) {
            if (w) {
                m_windowByUuid.insert(w->internalId(), w);
                setupWindowConnections(w);
                if (m_borderManager.isEnabled()) {
                    bool isFocused = (w == KWin::effects->activeWindow());
                    m_borderManager.updateBorder(w, isFocused, m_filter);
                }
            }
        }
    }
}

RavenEffect::~RavenEffect()
{
    if (KWin::effects) {
        disconnect(KWin::effects, &KWin::EffectsHandler::windowAdded, this, &RavenEffect::slotWindowAdded);
        disconnect(KWin::effects, &KWin::EffectsHandler::windowClosed, this, &RavenEffect::slotWindowClosed);
        disconnect(KWin::effects, &KWin::EffectsHandler::windowDeleted, this, &RavenEffect::slotWindowDeleted);
        disconnect(KWin::effects, &KWin::EffectsHandler::windowActivated, this, &RavenEffect::slotWindowActivated);
    }

    for (auto it = m_windowConnections.begin(); it != m_windowConnections.end(); ++it) {
        for (const auto &conn : it.value()) {
            disconnect(conn);
        }
    }
    m_windowConnections.clear();

    m_borderManager.clearAll();
    m_animController.clear();
    m_filter.clearCache();
    m_filter.clearBirthed();
    m_windowByUuid.clear();

    QDBusConnection dbus = QDBusConnection::sessionBus();
    dbus.unregisterObject(QStringLiteral("/Effects/Raven"));
    dbus.unregisterService(QStringLiteral("org.kde.kwin.RavenEffect"));
}

bool RavenEffect::supported()
{
    return KWin::effects && KWin::effects->compositingType() != KWin::NoCompositing;
}

bool RavenEffect::isActive() const
{
    return KWin::AnimationEffect::isActive();
}

void RavenEffect::animationEnded(KWin::EffectWindow *w, Attribute a, uint meta)
{
    Q_UNUSED(meta);
    m_animController.onAnimationEnded(w, a);
}

void RavenEffect::prePaintWindow(KWin::RenderView *view, KWin::EffectWindow *w, KWin::WindowPrePaintData &data)
{
    m_borderManager.prePaintWindow(w, m_filter);
    KWin::AnimationEffect::prePaintWindow(view, w, data);
}

KWin::EffectWindow *RavenEffect::findWindowById(const QString &windowId) const
{
    if (windowId.isEmpty() || !KWin::effects) {
        return nullptr;
    }

    QUuid uuid = QUuid::fromString(windowId);
    if (!uuid.isNull()) {
        auto it = m_windowByUuid.constFind(uuid);
        if (it != m_windowByUuid.cend() && it.value()) {
            return it.value();
        }
        if (auto *win = KWin::effects->findWindow(uuid)) {
            return win;
        }
    }

    if (windowId.startsWith(QLatin1String("0x"), Qt::CaseInsensitive)) {
        bool ok = false;
        quintptr ptrVal = windowId.toULongLong(&ok, 16);
        if (ok && ptrVal != 0) {
            for (auto *win : KWin::effects->stackingOrder()) {
                if (win && reinterpret_cast<quintptr>(win) == ptrVal) {
                    return win;
                }
            }
        }
    }

    for (auto *win : KWin::effects->stackingOrder()) {
        if (!win) continue;
        if (win->internalId().toString().compare(windowId, Qt::CaseInsensitive) == 0) {
            return win;
        }
    }
    return nullptr;
}

void RavenEffect::cancelWindowAnimation(const QString &windowId)
{
    m_animController.cancelAnimation(this, windowId);
}

void RavenEffect::animateWindowGeometry(const QString &windowId, const QRectF &startRect, const QRectF &targetRect,
                                        int durationMs, const QString &easingName)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (w) {
        m_animController.animateGeometry(this, w, windowId, startRect, targetRect, durationMs, easingName);
    }
}

void RavenEffect::animateBatchGeometry(const QString &jsonPayload)
{
    m_animController.animateBatch(this, jsonPayload, [this](const QString &id) {
        return findWindowById(id);
    });
}

void RavenEffect::animateWindowBirth(KWin::EffectWindow *w, int durationMs)
{
    m_animController.animateBirth(this, w, durationMs);
}

void RavenEffect::animateWindowBirth(const QString &windowId, int durationMs)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (w) {
        animateWindowBirth(w, durationMs);
    }
}

void RavenEffect::minimizeWindow(const QString &windowId)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (!w || w->isDeleted()) return;

    m_animController.removePending(windowId);
    cancelWindowAnimation(windowId);
    m_borderManager.removeBorder(w);
    w->minimize();
}

void RavenEffect::unminimizeWindow(const QString &windowId)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (!w || w->isDeleted()) return;

    w->unminimize();
    if (KWin::effects) {
        KWin::effects->activateWindow(w);
    }
}

void RavenEffect::closeWindow(const QString &windowId)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (!w || w->isDeleted()) return;

    cancelWindowAnimation(windowId);
    m_borderManager.removeBorder(w);
    w->closeWindow();
}

void RavenEffect::activateWindow(const QString &windowId)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (w && !w->isDeleted() && KWin::effects) {
        KWin::effects->activateWindow(w);
    }
}

void RavenEffect::updateBorderConfig(bool enabled, double width, const QString &activeColor, const QString &inactiveColor, double radius)
{
    m_borderManager.updateConfig(enabled, width, activeColor, inactiveColor, radius);
    m_filter.clearCache();

    if (KWin::effects) {
        const auto windows = KWin::effects->stackingOrder();
        for (auto *win : windows) {
            if (win) {
                bool isFocused = (win == KWin::effects->activeWindow());
                m_borderManager.updateBorder(win, isFocused, m_filter);
            }
        }
    }
}

void RavenEffect::setBorderRadius(double radius)
{
    m_borderManager.setBorderRadius(radius);
    if (KWin::effects) {
        const auto windows = KWin::effects->stackingOrder();
        for (auto *win : windows) {
            if (win) {
                bool isFocused = (win == KWin::effects->activeWindow());
                m_borderManager.updateBorder(win, isFocused, m_filter);
            }
        }
    }
}

qreal RavenEffect::detectSystemCornerRadius() const
{
    return m_borderManager.detectSystemCornerRadius();
}

KWin::BorderRadius RavenEffect::resolveAdaptiveBorderRadius(KWin::EffectWindow *w) const
{
    return m_borderManager.resolveAdaptiveBorderRadius(w);
}

void RavenEffect::setupWindowConnections(KWin::EffectWindow *w)
{
    if (!w) return;

    auto &conns = m_windowConnections[w];
    for (const auto &c : conns) {
        disconnect(c);
    }
    conns.clear();

    conns.append(connect(w, &KWin::EffectWindow::windowFrameGeometryChanged, this, &RavenEffect::slotWindowGeometryChanged));
    conns.append(connect(w, &KWin::EffectWindow::windowStepUserMovedResized, this, &RavenEffect::slotWindowStepUserMovedResized));
    conns.append(connect(w, &KWin::EffectWindow::minimizedChanged, this, &RavenEffect::slotWindowMinimizedChanged));
    conns.append(connect(w, &KWin::EffectWindow::windowHiddenChanged, this, [this, w]() {
        m_filter.invalidate(w);
        if (w->isHidden() || !w->isVisible() || w->isDeleted()) {
            const QString wid = w->internalId().toString();
            if (!wid.isEmpty()) {
                m_animController.removePending(wid);
                cancelWindowAnimation(wid);
            }
            m_borderManager.removeBorder(w);
            if (KWin::effects) {
                KWin::effects->addRepaint(w->expandedGeometry());
            }
        } else if (m_borderManager.isEnabled()) {
            bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
            m_borderManager.updateBorder(w, isFocused, m_filter);
        }
    }));
    conns.append(connect(w, &KWin::EffectWindow::windowMaximizedStateChanged, this, [this, w]() {
        m_filter.invalidate(w);
        if (m_borderManager.isEnabled()) {
            bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
            m_borderManager.updateBorder(w, isFocused, m_filter);
        }
    }));
    conns.append(connect(w, &KWin::EffectWindow::windowDecorationChanged, this, [this, w]() {
        if (m_borderManager.isEnabled()) {
            bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
            m_borderManager.updateBorder(w, isFocused, m_filter);
        }
    }));

    if (w->window()) {
        conns.append(connect(w->window(), &KWin::Window::closed, this, [this, w]() {
            slotWindowClosed(w);
        }));
        conns.append(connect(w->window(), &KWin::Window::surfaceChanged, this, [this, w]() {
            if (w->window() && !w->window()->surface()) {
                const QString wid = w->internalId().toString();
                if (!wid.isEmpty()) {
                    m_animController.removePending(wid);
                    cancelWindowAnimation(wid);
                }
                m_borderManager.removeBorder(w);
            } else if (m_borderManager.isEnabled()) {
                bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
                m_borderManager.updateBorder(w, isFocused, m_filter);
            }
        }));
        conns.append(connect(w->window(), &KWin::Window::transientChanged, this, [this, w]() {
            m_filter.invalidate(w);
            if (m_borderManager.isEnabled()) {
                bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
                m_borderManager.updateBorder(w, isFocused, m_filter);
            }
        }));
        conns.append(connect(w->window(), &KWin::Window::windowClassChanged, this, [this, w]() {
            m_filter.invalidate(w);
            if (m_borderManager.isEnabled()) {
                bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
                m_borderManager.updateBorder(w, isFocused, m_filter);
            }
        }));
        conns.append(connect(w->window(), &KWin::Window::captionChanged, this, [this, w]() {
            m_filter.invalidate(w);
            if (m_borderManager.isEnabled()) {
                bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
                m_borderManager.updateBorder(w, isFocused, m_filter);
            }
        }));
    }
}

void RavenEffect::slotWindowAdded(KWin::EffectWindow *w)
{
    if (!w) return;

    m_windowByUuid.insert(w->internalId(), w);
    setupWindowConnections(w);

    if (m_filter.shouldAnimateBirth(w) && !m_filter.hasBirthed(w)) {
        if (w->window() && !w->window()->readyForPainting()) {
            const QString wid = w->internalId().toString();
            QPointer<KWin::Window> winPtr = w->window();
            auto conn = std::make_shared<QMetaObject::Connection>();
            *conn = connect(w->window(), &KWin::Window::readyForPaintingChanged, this, [this, wid, winPtr, conn]() {
                if (winPtr && winPtr->readyForPainting() && !winPtr->isDeleted()) {
                    disconnect(*conn);
                    KWin::EffectWindow *ew = findWindowById(wid);
                    if (ew && !ew->isDeleted() && !m_filter.hasBirthed(ew)) {
                        m_filter.markBirthed(ew);
                        animateWindowBirth(ew, 180);
                    }
                }
            });
        } else if (!w->isDeleted()) {
            m_filter.markBirthed(w);
            animateWindowBirth(w, 180);
        }
    }

    if (m_borderManager.isEnabled() && !w->isDeleted()) {
        bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
        m_borderManager.updateBorder(w, isFocused, m_filter);
    }
}

void RavenEffect::slotWindowClosed(KWin::EffectWindow *w)
{
    if (!w) return;

    auto connIt = m_windowConnections.find(w);
    if (connIt != m_windowConnections.end()) {
        for (const auto &c : connIt.value()) {
            disconnect(c);
        }
        m_windowConnections.erase(connIt);
    }
    m_filter.invalidate(w);
    m_filter.removeBirthed(w);

    const auto allWindows = KWin::effects ? KWin::effects->stackingOrder() : QList<KWin::EffectWindow *>();
    for (auto *child : allWindows) {
        if (!child || child == w) continue;
        bool isTransientChild = (child->transientFor() == w) || child->mainWindows().contains(w);
        if (!isTransientChild && w->window() && child->window()) {
            isTransientChild = (child->window()->transientFor() == w->window())
                || w->window()->hasTransient(child->window(), true);
        }
        if (isTransientChild) {
            m_filter.invalidate(child);
            m_filter.removeBirthed(child);
            const QString childId = child->internalId().toString();
            if (!childId.isEmpty()) {
                m_animController.removePending(childId);
                cancelWindowAnimation(childId);
            }
            m_borderManager.removeBorder(child);
            child->addRepaintFull();
            if (KWin::effects) {
                KWin::effects->addRepaint(child->expandedGeometry());
            }
        }
    }

    QString windowId = w->internalId().toString();
    if (!windowId.isEmpty()) {
        m_animController.removePending(windowId);
        cancelWindowAnimation(windowId);
    }

    QString hexId = QStringLiteral("0x%1").arg(reinterpret_cast<quintptr>(w), 0, 16);
    m_animController.removePending(hexId);
    cancelWindowAnimation(hexId);

    QString decId = QString::number(reinterpret_cast<quintptr>(w));
    m_animController.removePending(decId);
    cancelWindowAnimation(decId);

    m_borderManager.removeBorder(w);
    m_windowByUuid.remove(w->internalId());

    if (m_lastActiveWindow == w) {
        m_lastActiveWindow.clear();
    }

    w->addRepaintFull();
    if (KWin::effects) {
        KWin::effects->addRepaint(w->expandedGeometry());
    }
}

void RavenEffect::slotWindowDeleted(KWin::EffectWindow *w)
{
    slotWindowClosed(w);
}

void RavenEffect::slotWindowActivated(KWin::EffectWindow *w)
{
    if (!m_borderManager.isEnabled()) return;

    if (m_lastActiveWindow && m_lastActiveWindow != w) {
        m_filter.invalidate(m_lastActiveWindow);
        m_borderManager.updateBorder(m_lastActiveWindow, false, m_filter);
    }
    if (w) {
        m_filter.invalidate(w);
        m_borderManager.updateBorder(w, true, m_filter);
        if (KWin::EffectWindow *parent = w->transientFor()) {
            m_filter.invalidate(parent);
            m_borderManager.updateBorder(parent, false, m_filter);
        }
    }
    m_lastActiveWindow = w;
}

void RavenEffect::slotWindowGeometryChanged(KWin::EffectWindow *w, const KWin::RectF &oldGeometry)
{
    if (!w) return;

    if (w->isDeleted() || w->isHidden() || w->isMinimized()) {
        m_borderManager.removeBorder(w);
        return;
    }

    if (m_borderManager.isEnabled()) {
        bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
        m_borderManager.updateBorder(w, isFocused, m_filter);
    }

    if (w->isUserMove() || w->isUserResize()) return;

    if (!w->isNormalWindow() || w->isDialog() || w->isUtility() || w->isModal() || w->transientFor() != nullptr) {
        return;
    }
    if (w->window() && (w->window()->isTransient() || w->window()->isModal())) {
        return;
    }

    const QRectF currentGeom = w->frameGeometry();
    if (oldGeometry.isEmpty() || currentGeom.isEmpty() || oldGeometry == currentGeom) {
        return;
    }

    const QString wid = w->internalId().toString();
    int durationMs = 130;
    QString easingName = QStringLiteral("EaseOutCubic");
    QRectF startRect = oldGeometry;

    if (m_animController.hasPending(wid)) {
        auto pending = m_animController.takePending(wid);
        if (!pending.startRect.isEmpty()) {
            startRect = pending.startRect;
        }
        durationMs = pending.durationMs;
        easingName = pending.easing;
    }

    m_animController.animateGeometry(this, w, wid, startRect, currentGeom, durationMs, easingName);
}

void RavenEffect::slotWindowStepUserMovedResized(KWin::EffectWindow *w, const KWin::RectF &geometry)
{
    Q_UNUSED(geometry);
    if (m_borderManager.isEnabled()) {
        bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
        m_borderManager.updateBorder(w, isFocused, m_filter);
    }
}

void RavenEffect::slotWindowMinimizedChanged(KWin::EffectWindow *w)
{
    if (!w) return;

    m_filter.invalidate(w);

    if (w->isMinimized()) {
        const QString wid = w->internalId().toString();
        if (!wid.isEmpty()) {
            m_animController.removePending(wid);
            cancelWindowAnimation(wid);
        }
        m_borderManager.removeBorder(w);
        if (m_lastActiveWindow == w) {
            m_lastActiveWindow.clear();
        }
        if (KWin::effects) {
            KWin::effects->addRepaint(w->expandedGeometry());
        }
    } else {
        bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
        m_borderManager.updateBorder(w, isFocused, m_filter);
    }
}

#include "moc_raveneffect.cpp"
