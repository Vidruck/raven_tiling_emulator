/* raveneffect.cpp
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 *
 * Descripción: Implementación del efecto nativo de KWin para Raven Tiling Emulator.
 * Optimizado con OutlinedBorderItem nativo (SDF anti-aliasing con curvatura real),
 * animaciones basadas en traslación FLIP y búsqueda O(1) de ventanas.
 */

#include "raveneffect.h"
#include "dbus/raven_effect_adaptor.h"
#include <effect/effecthandler.h>
#include <window.h>

#if __has_include(<kdecoration3/decoration.h>)
#include <kdecoration3/decoration.h>
namespace KDeco = KDecoration3;
#define HAVE_KDECORATION 1
#elif __has_include(<KDecoration3/Decoration>)
#include <KDecoration3/Decoration>
namespace KDeco = KDecoration3;
#define HAVE_KDECORATION 1
#elif __has_include(<kdecoration2/decoration.h>)
#include <kdecoration2/decoration.h>
namespace KDeco = KDecoration2;
#define HAVE_KDECORATION 1
#elif __has_include(<KDecoration2/Decoration>)
#include <KDecoration2/Decoration>
namespace KDeco = KDecoration2;
#define HAVE_KDECORATION 1
#endif
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QFile>
#include <QDir>
#include <QStandardPaths>
#include <QLoggingCategory>
#include <chrono>
#include <cmath>
#include <algorithm>

Q_LOGGING_CATEGORY(RAVEN_EFFECT, "raven.effect", QtInfoMsg)

/**
 * @brief Constructor de la clase RavenEffect.
 *
 * Configura D-Bus, lee la configuración inicial y enlaza los eventos de ciclo de vida
 * de ventanas del compositor KWin.
 */
RavenEffect::RavenEffect()
    : m_dbusAdaptor(new RavenEffectAdaptor(this))
{
    loadBorderConfig();

    QDBusConnection dbus = QDBusConnection::sessionBus();
    if (dbus.registerService(QStringLiteral("org.kde.kwin.RavenEffect"))) {
        if (dbus.registerObject(QStringLiteral("/Effects/Raven"), this)) {
            qCInfo(RAVEN_EFFECT) << "RavenEffect D-Bus registrado exitosamente en /Effects/Raven";
        } else {
            qCWarning(RAVEN_EFFECT) << "Fallo al registrar objeto D-Bus /Effects/Raven";
        }
    } else {
        qCWarning(RAVEN_EFFECT) << "Fallo al registrar servicio D-Bus org.kde.kwin.RavenEffect";
    }

    if (KWin::effects) {
        connect(KWin::effects, &KWin::EffectsHandler::windowAdded, this, &RavenEffect::slotWindowAdded);
        connect(KWin::effects, &KWin::EffectsHandler::windowClosed, this, &RavenEffect::slotWindowClosed);
        connect(KWin::effects, &KWin::EffectsHandler::windowActivated, this, &RavenEffect::slotWindowActivated);

        const auto windows = KWin::effects->stackingOrder();
        for (auto *win : windows) {
            if (win) {
                m_windowByUuid.insert(win->internalId(), win);
                setupWindowConnections(win);

                if (m_bordersEnabled) {
                    updateBorder(win);
                }
            }
        }
    }
}

/**
 * @brief Destructor de la clase RavenEffect.
 *
 * Libera todos los OutlinedBorderItem registrados en el scene graph de KWin,
 * desconecta las señales y desregistra el servicio D-Bus.
 */
RavenEffect::~RavenEffect()
{
    if (KWin::effects) {
        disconnect(KWin::effects, &KWin::EffectsHandler::windowAdded, this, &RavenEffect::slotWindowAdded);
        disconnect(KWin::effects, &KWin::EffectsHandler::windowClosed, this, &RavenEffect::slotWindowClosed);
        disconnect(KWin::effects, &KWin::EffectsHandler::windowActivated, this, &RavenEffect::slotWindowActivated);
    }

    // Limpieza de todos los bordes nativos
    for (auto &pair : m_borders) {
        disconnect(pair.second.radiusConnection);
        disconnect(pair.second.classConnection);
        disconnect(pair.second.captionConnection);
        disconnect(pair.second.decorationConnection);
#ifdef HAVE_KWIN_OUTLINED_BORDER
        pair.second.item.reset();
#endif
    }
    m_borders.clear();
    m_windowByUuid.clear();
    m_borderVerdictCache.clear();

    QDBusConnection dbus = QDBusConnection::sessionBus();
    dbus.unregisterObject(QStringLiteral("/Effects/Raven"));
    dbus.unregisterService(QStringLiteral("org.kde.kwin.RavenEffect"));
}

/**
 * @brief Verifica si el compositor soporta efectos de aceleración por hardware.
 */
bool RavenEffect::supported()
{
    return KWin::effects && KWin::effects->compositingType() != KWin::NoCompositing;
}

/**
 * @brief Comprueba si el efecto tiene alguna animación en progreso.
 *
 * Con OutlinedBorderItem los bordes son gestionados directamente por el scene graph
 * nativo de KWin, por lo que isActive() solo devuelve true cuando hay animaciones
 * de movimiento/escala activas, liberando ciclos en reposo.
 */
bool RavenEffect::isActive() const
{
    return !m_activeGeometryAnimations.isEmpty() || !m_activeBirthAnimations.isEmpty();
}

/**
 * @brief Maneja la finalización de una animación de KWin para purgar la memoria.
 */
void RavenEffect::animationEnded(KWin::EffectWindow *w, Attribute a, uint meta)
{
    Q_UNUSED(a);
    Q_UNUSED(meta);
    if (!w) {
        return;
    }

    const QString windowId = w->internalId().toString();
    const QString hexId = QStringLiteral("0x%1").arg(reinterpret_cast<quintptr>(w), 0, 16);
    const QString decId = QString::number(reinterpret_cast<quintptr>(w));

    if (!windowId.isEmpty()) {
        m_activeGeometryAnimations.remove(windowId);
        m_activeBirthAnimations.remove(windowId);
    }
    m_activeGeometryAnimations.remove(hexId);
    m_activeBirthAnimations.remove(hexId);
    m_activeGeometryAnimations.remove(decId);
    m_activeBirthAnimations.remove(decId);

    // Repintar la ventana para garantizar que cualquier transformación visual residual sea limpiada
    w->addRepaintFull();
}

/**
 * @brief Conecta todos los eventos de ciclo de vida, buffer, maximización y superficie para una ventana.
 */
void RavenEffect::setupWindowConnections(KWin::EffectWindow *w)
{
    if (!w) {
        return;
    }

    connect(w, &KWin::EffectWindow::windowFrameGeometryChanged, this, &RavenEffect::slotWindowGeometryChanged);
    connect(w, &KWin::EffectWindow::windowStepUserMovedResized, this, &RavenEffect::slotWindowStepUserMovedResized);
    connect(w, &KWin::EffectWindow::minimizedChanged, this, &RavenEffect::slotWindowMinimizedChanged);
    connect(w, &KWin::EffectWindow::windowMaximizedStateChanged, this, [this, w]() {
        invalidateBorderVerdict(w);
        if (m_bordersEnabled) updateBorder(w);
    });

    if (w->window()) {
        connect(w->window(), &KWin::Window::windowClassChanged, this, [this, w]() {
            invalidateBorderVerdict(w);
            if (m_bordersEnabled) updateBorder(w);
        });
        connect(w->window(), &KWin::Window::captionChanged, this, [this, w]() {
            invalidateBorderVerdict(w);
            if (m_bordersEnabled) updateBorder(w);
        });
        connect(w->window(), &KWin::Window::readyForPaintingChanged, this, [this, w]() {
            if (m_bordersEnabled) updateBorder(w);
        });
        connect(w->window(), &KWin::Window::surfaceChanged, this, [this, w]() {
            if (m_bordersEnabled) updateBorder(w);
        });
        connect(w->window(), &KWin::Window::bufferGeometryChanged, this, [this, w]() {
            if (m_bordersEnabled) updateBorder(w);
        });
    }
}

/**
 * @brief Hook ejecutado antes de pintar cada ventana en el ciclo del compositor.
 * Garantiza que cualquier ventana apta tenga su OutlinedBorderItem activo sin importar el toolkit.
 * Optimizado para cero asignaciones en la ruta caliente del render loop de KWin.
 */
void RavenEffect::prePaintWindow(KWin::RenderView *view, KWin::EffectWindow *w, KWin::WindowPrePaintData &data)
{
    if (m_bordersEnabled && w) {
        auto it = m_borders.find(w);
        if (it == m_borders.end() || !it->second.item) {
            if (shouldDrawBorder(w)) {
                updateBorder(w);
            }
        }
    }
    KWin::AnimationEffect::prePaintWindow(view, w, data);
}

/**
 * @brief Slot que se ejecuta cuando se añade una nueva ventana al compositor.
 */
void RavenEffect::slotWindowAdded(KWin::EffectWindow *w)
{
    if (!w) {
        return;
    }

    m_windowByUuid.insert(w->internalId(), w);
    setupWindowConnections(w);

    if (m_bordersEnabled) {
        updateBorder(w);
    }
}

/**
 * @brief Slot que se ejecuta cuando una ventana es cerrada.
 */
void RavenEffect::slotWindowClosed(KWin::EffectWindow *w)
{
    if (!w) {
        return;
    }

    disconnect(w, &KWin::EffectWindow::windowFrameGeometryChanged, this, &RavenEffect::slotWindowGeometryChanged);
    disconnect(w, &KWin::EffectWindow::windowStepUserMovedResized, this, &RavenEffect::slotWindowStepUserMovedResized);
    disconnect(w, &KWin::EffectWindow::minimizedChanged, this, &RavenEffect::slotWindowMinimizedChanged);
    invalidateBorderVerdict(w);

    QString windowId = w->internalId().toString();
    if (!windowId.isEmpty()) {
        m_pendingAnimations.remove(windowId);
        cancelWindowAnimation(windowId);
    }

    QString hexId = QStringLiteral("0x%1").arg(reinterpret_cast<quintptr>(w), 0, 16);
    m_pendingAnimations.remove(hexId);
    cancelWindowAnimation(hexId);

    QString decId = QString::number(reinterpret_cast<quintptr>(w));
    m_pendingAnimations.remove(decId);
    cancelWindowAnimation(decId);

    removeBorder(w);
    m_windowByUuid.remove(w->internalId());

    if (m_lastActiveWindow == w) {
        m_lastActiveWindow.clear();
    }
}

/**
 * @brief Busca una ventana activa (EffectWindow) por su ID de manera O(1).
 */
KWin::EffectWindow *RavenEffect::findWindowById(const QString &windowId) const
{
    if (windowId.isEmpty() || !KWin::effects) {
        return nullptr;
    }

    // 1. Búsqueda rápida O(1) usando QUuid
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

    // 2. Comprobación por puntero hexadecimal si viene con prefijo "0x"
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

    // 3. Fallback en la lista de apilamiento
    for (auto *win : KWin::effects->stackingOrder()) {
        if (!win) continue;
        if (win->internalId().toString().compare(windowId, Qt::CaseInsensitive) == 0) {
            return win;
        }
    }
    return nullptr;
}

/**
 * @brief Convierte un nombre descriptivo en una curva QEasingCurve nativa.
 */
QEasingCurve RavenEffect::parseEasing(const QString &easingName) const
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

/**
 * @brief Interpola suavemente la posición y tamaño usando KWin::AnimationEffect::Translation.
 */
void RavenEffect::cancelGeometryAnimation(const QString &windowId)
{
    if (m_activeGeometryAnimations.contains(windowId)) {
        const auto anims = m_activeGeometryAnimations.take(windowId);
        for (quint64 id : anims) {
            cancel(id);
        }
    }
}

void RavenEffect::cancelBirthAnimation(const QString &windowId)
{
    if (m_activeBirthAnimations.contains(windowId)) {
        const auto anims = m_activeBirthAnimations.take(windowId);
        for (quint64 id : anims) {
            cancel(id);
        }
    }
}

/**
 * @brief Cancela de manera inmediata todas las animaciones activas asociadas a una ventana.
 */
void RavenEffect::cancelWindowAnimation(const QString &windowId)
{
    cancelGeometryAnimation(windowId);
    cancelBirthAnimation(windowId);
}

/**
 * @brief Interpola suavemente la posición y tamaño usando KWin::AnimationEffect::Translation.
 */
void RavenEffect::animateWindowGeometry(const QString &windowId, const QRectF &startRect, const QRectF &targetRect,
                                        int durationMs, const QString &easingName)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (!w) {
        return;
    }

    cancelGeometryAnimation(windowId);

    if (durationMs <= 0 || durationMs > 500) {
        durationMs = 130;
    }

    const QRectF currentGeom = w->frameGeometry();

    // Coordinación de procesos: si el script de KWin aún no ha aplicado la geometría objetivo (targetRect),
    // almacenamos la animación pendiente para que slotWindowGeometryChanged la tome en t=0 exacto
    if (currentGeom != targetRect) {
        m_pendingAnimations[windowId] = { startRect, targetRect, durationMs, easingName };
        return;
    }

    // La ventana ya está en targetRect en KWin: aplicar FLIP limpio inmediato
    m_pendingAnimations.remove(windowId);

    QEasingCurve curve = parseEasing(easingName);

    qreal fromX = startRect.x() - currentGeom.x();
    qreal fromY = startRect.y() - currentGeom.y();

    if (qAbs(fromX) > 0.5 || qAbs(fromY) > 0.5) {
        quint64 posAnim = animate(w,
                                  KWin::AnimationEffect::Translation,
                                  0,
                                  std::chrono::milliseconds(durationMs),
                                  KWin::FPx2(0.0, 0.0),
                                  curve,
                                  0,
                                  KWin::FPx2(fromX, fromY));

        if (posAnim != 0) {
            m_activeGeometryAnimations[windowId].append(posAnim);
        }
    }

    // 2. Animación de escala por geometría (solo si no hay animación de nacimiento activa con escala central)
    if (!m_activeBirthAnimations.contains(windowId) &&
        targetRect.width() > 0.0 && targetRect.height() > 0.0 &&
        startRect.width() > 0.0 && startRect.height() > 0.0 &&
        currentGeom.width() > 0.0 && currentGeom.height() > 0.0) {
        
        qreal fromScaleX = startRect.width() / currentGeom.width();
        qreal fromScaleY = startRect.height() / currentGeom.height();

        if (qAbs(fromScaleX - 1.0) > 0.005 || qAbs(fromScaleY - 1.0) > 0.005) {
            uint metaScale = 0;
            setMetaData(SourceAnchor, Anchor::Left | Anchor::Top, metaScale);

            quint64 scaleAnim = animate(w,
                                        KWin::AnimationEffect::Scale,
                                        metaScale,
                                        std::chrono::milliseconds(durationMs),
                                        KWin::FPx2(1.0, 1.0),
                                        curve,
                                        0,
                                        KWin::FPx2(fromScaleX, fromScaleY));

            if (scaleAnim != 0) {
                m_activeGeometryAnimations[windowId].append(scaleAnim);
            }
        }
    }

    w->addRepaintFull();
}

/**
 * @brief Procesa un lote de animaciones simultáneas a partir de un JSON de Raven Backend.
 */
void RavenEffect::animateBatchGeometry(const QString &jsonPayload)
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
            animateWindowGeometry(winId, from, to, durationMs, easing);
        }
    }
}

/**
 * @brief Animación de nacimiento controlada (fade-in + zoom elástico) llamada por D-Bus.
 */
void RavenEffect::animateWindowBirth(const QString &windowId, int durationMs)
{
    KWin::EffectWindow *w = findWindowById(windowId);
    if (!w) {
        return;
    }

    cancelBirthAnimation(windowId);

    if (durationMs <= 0) {
        durationMs = 140;
    }

    QEasingCurve curve(QEasingCurve::OutElastic);
    QEasingCurve opacityCurve(QEasingCurve::OutCubic);

    uint metaCenter = 0;
    setMetaData(SourceAnchor, Anchor::Horizontal | Anchor::Vertical, metaCenter);

    quint64 scaleAnim = animate(w,
                                KWin::AnimationEffect::Scale,
                                metaCenter,
                                std::chrono::milliseconds(durationMs),
                                KWin::FPx2(1.0, 1.0),
                                curve,
                                0,
                                KWin::FPx2(0.85, 0.85));

    uint metaOpacity = 0;
    quint64 opacityAnim = animate(w,
                                  KWin::AnimationEffect::Opacity,
                                  metaOpacity,
                                  std::chrono::milliseconds(static_cast<int>(durationMs * 0.8)),
                                  KWin::FPx2(1.0, 1.0),
                                  opacityCurve,
                                  0,
                                  KWin::FPx2(0.0, 0.0));

    if (scaleAnim != 0) m_activeBirthAnimations[windowId].append(scaleAnim);
    if (opacityAnim != 0) m_activeBirthAnimations[windowId].append(opacityAnim);
}

/**
 * @brief Carga la configuración de contornos desde ~/.config/raven/raven.json si existe.
 */
void RavenEffect::loadBorderConfig()
{
    QString configPath = QStandardPaths::writableLocation(QStandardPaths::ConfigLocation) + QStringLiteral("/raven/raven.json");
    QFile file(configPath);
    if (!file.exists() || !file.open(QIODevice::ReadOnly)) {
        return;
    }

    QByteArray data = file.readAll();
    QJsonDocument doc = QJsonDocument::fromJson(data);
    if (!doc.isObject()) {
        return;
    }

    QJsonObject obj = doc.object();
    if (obj.contains(QStringLiteral("borders_enabled"))) {
        m_bordersEnabled = obj.value(QStringLiteral("borders_enabled")).toBool(true);
    }
    if (obj.contains(QStringLiteral("border_width"))) {
        m_borderWidth = static_cast<float>(std::clamp(obj.value(QStringLiteral("border_width")).toDouble(2.0), 1.0, 6.0));
    }
    if (obj.contains(QStringLiteral("active_border_color"))) {
        QColor ac(obj.value(QStringLiteral("active_border_color")).toString());
        if (ac.isValid()) {
            m_activeBorderColor = ac;
        }
    }
    if (obj.contains(QStringLiteral("inactive_border_color"))) {
        QColor ic(obj.value(QStringLiteral("inactive_border_color")).toString());
        if (ic.isValid()) {
            m_inactiveBorderColor = ic;
        }
    }
    if (obj.contains(QStringLiteral("border_radius"))) {
        QJsonValue rVal = obj.value(QStringLiteral("border_radius"));
        if (rVal.isDouble()) {
            m_customBorderRadius = rVal.toDouble(-1.0);
        } else if (rVal.isString() && rVal.toString().toLower() == QLatin1String("auto")) {
            m_customBorderRadius = -1.0;
        }
    }
}

void RavenEffect::invalidateBorderVerdict(const KWin::EffectWindow *w)
{
    if (w) {
        m_borderVerdictCache.remove(w);
    }
}

/**
 * @brief Comprueba si una ventana es apta para recibir contornos visuales usando caché O(1).
 */
bool RavenEffect::shouldDrawBorder(KWin::EffectWindow *w) const
{
    if (!w) {
        return false;
    }
    auto it = m_borderVerdictCache.constFind(w);
    if (it != m_borderVerdictCache.constEnd()) {
        return it.value();
    }
    bool verdict = computeShouldDrawBorder(w);
    m_borderVerdictCache.insert(w, verdict);
    return verdict;
}

/**
 * @brief Computa exhaustivamente las heurísticas de elegibilidad visual de una ventana.
 */
bool RavenEffect::computeShouldDrawBorder(KWin::EffectWindow *w) const
{
    if (!w) {
        return false;
    }

    if (w->isDeleted()
        || w->isMinimized()
        || w->isHidden()
        || !w->isVisible()
        || w->isFullScreen()) {
        return false;
    }

    // Descartar ventanas que cubren por completo su pantalla (fullscreen de facto)
    if (auto *out = w->screen()) {
        const QRect og = QRect(out->geometry());
        const QRect wg = QRect(w->frameGeometry().toAlignedRect());
        if (!og.isEmpty() && wg.contains(og)) {
            return false;
        }
    }

    if (w->window() && w->window()->maximizeMode() != KWin::MaximizeRestore) {
        return false;
    }

    // Filtrar elementos de sistema, popups, overlays y paneles
    if (w->isLockScreen()
        || w->isDesktop()
        || w->isDock()
        || w->isNotification()
        || w->isCriticalNotification()
        || w->isTooltip()
        || w->isAppletPopup()
        || w->isOnScreenDisplay()
        || w->isComboBox()
        || w->isDNDIcon()
        || w->isDropdownMenu()
        || w->isPopupMenu()
        || w->isOutline()
        || w->isInputMethod()
        || w->isSpecialWindow()
        || w->isSplash()
        || !w->isManaged()) {
        return false;
    }

    const QString cls = w->windowClass().toLower();
    const QString role = w->windowRole().toLower();
    const QString cap = w->caption().toLower();

    // Si aún no tiene clase ni título (ventana Wayland recién creada), solo descartar si no es ventana normal ni diálogo
    if (cls.trimmed().isEmpty() && cap.trimmed().isEmpty()) {
        if (!w->isNormalWindow() && !w->isDialog()) {
            return false;
        }
    }

    if (cls.contains(QStringLiteral("spectacle"))
        || cls.contains(QStringLiteral("flameshot"))
        || cls.contains(QStringLiteral("ksnip"))
        || cls.contains(QStringLiteral("plasma-welcome"))
        || cls.contains(QStringLiteral("plasma.welcome"))
        || cls.contains(QStringLiteral("welcome"))
        || cls.contains(QStringLiteral("plasmashell"))
        || cls.contains(QStringLiteral("kscreenlocker"))
        || cls.contains(QStringLiteral("screenlocker"))
        || cls.contains(QStringLiteral("lockscreen"))
        || cls.contains(QStringLiteral("plasmalogin"))
        || cls.contains(QStringLiteral("sddm"))
        || cls.contains(QStringLiteral("polkit"))
        || cls.contains(QStringLiteral("desktopdialog"))
        || role.contains(QStringLiteral("spectacle"))
        || role.contains(QStringLiteral("welcome"))
        || role.contains(QStringLiteral("lockscreen"))
        || role.contains(QStringLiteral("login"))
        || cap.contains(QStringLiteral("centro de bienvenida"))
        || cap.contains(QStringLiteral("welcome center"))
        || cap.contains(QStringLiteral("bienvenida de plasma"))
        || cap.contains(QStringLiteral("bienvenida"))
        || cap.contains(QStringLiteral("inicio de sesión"))
        || cap.contains(QStringLiteral("pantalla de inicio de sesión"))
        || cap.contains(QStringLiteral("login screen"))
        || cap.contains(QStringLiteral("bloqueo de pantalla"))
        || cap.contains(QStringLiteral("screen lock"))) {
        return false;
    }

    static const QStringList kSystemClasses = {
        QStringLiteral("ksplash"),
        QStringLiteral("ksplashqml"),
        QStringLiteral("ksmserver"),
        QStringLiteral("kscreenlocker"),
        QStringLiteral("kscreenlocker_greet"),
        QStringLiteral("plasma-lockscreen"),
        QStringLiteral("plasmalogin"),
        QStringLiteral("sddm"),
        QStringLiteral("org.kde.plasma.welcome"),
        QStringLiteral("org.kde.plasmashell"),
        QStringLiteral("krunner"),
        QStringLiteral("org.kde.krunner"),
        QStringLiteral("kwin_wayland"),
        QStringLiteral("kwin_x11"),
        QStringLiteral("xwaylandvideobridge"),
        QStringLiteral("plasma-interactiveconsole"),
        QStringLiteral("org.kde.polkit-kde-authentication-agent"),
        QStringLiteral("polkit-kde-authentication-agent"),
        QStringLiteral("ksshaskpass"),
        QStringLiteral("kded"),
        QStringLiteral("org.kde.kdeconnect"),
        QStringLiteral("greeter"),
        QStringLiteral("org.kde.plasma.emojier"),
        QStringLiteral("org.kde.kscreen.osd"),
        QStringLiteral("org.kde.spectacle"),
        QStringLiteral("org.kde.plasma-welcome")
    };
    for (const QString &sc : kSystemClasses) {
        if (cls.contains(sc) || role.contains(sc)) {
            return false;
        }
    }

    if (cap.contains(QStringLiteral("picture-in-picture")) ||
        cap.contains(QStringLiteral("picture in picture")) ||
        cap.contains(QStringLiteral("imagen en imagen")) ||
        cap.contains(QStringLiteral("pantalla en pantalla")) ||
        cap.contains(QStringLiteral("reproductor en miniatura")) ||
        cap.contains(QStringLiteral("incrustation")) ||
        cap.contains(QStringLiteral("bild-in-bild")) ||
        cap.contains(QStringLiteral("imagem em imagem")) ||
        cap == QStringLiteral("pip")) {
        return false;
    }

    if (w->findModal() != nullptr) {
        return false;
    }

    if (w->isDialog() && w->transientFor() != nullptr && KWin::effects && KWin::effects->activeWindow() != w) {
        return false;
    }

    return w->isNormalWindow() || w->isDialog();
}

/**
 * @brief Lee de manera segura un valor numérico desde un archivo de configuración INI de KDE.
 */
static qreal readKdeConfigValue(const QString &filePath, const QString &targetGroup, const QString &targetKey, qreal defaultVal = -1.0)
{
    QFile file(filePath);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return defaultVal;
    }

    bool inGroup = targetGroup.isEmpty();
    while (!file.atEnd()) {
        QString line = QString::fromUtf8(file.readLine()).trimmed();
        if (line.isEmpty() || line.startsWith(QLatin1Char('#')) || line.startsWith(QLatin1Char(';'))) {
            continue;
        }
        if (line.startsWith(QLatin1Char('[')) && line.endsWith(QLatin1Char(']'))) {
            QString group = line.mid(1, line.length() - 2).trimmed();
            inGroup = (group.compare(targetGroup, Qt::CaseInsensitive) == 0);
            continue;
        }
        if (inGroup) {
            int eqIdx = line.indexOf(QLatin1Char('='));
            if (eqIdx > 0) {
                QString k = line.left(eqIdx).trimmed();
                if (k.compare(targetKey, Qt::CaseInsensitive) == 0) {
                    bool ok = false;
                    qreal v = line.mid(eqIdx + 1).trimmed().toDouble(&ok);
                    if (ok) return v;
                }
            }
        }
    }
    return defaultVal;
}

/**
 * @brief Detecta adaptativamente la curvatura de esquinas configurada en el escritorio Plasma (6px, 8px, 12px, etc.).
 */
qreal RavenEffect::detectSystemCornerRadius() const
{
    if (m_customBorderRadius >= 0.0) {
        return m_customBorderRadius;
    }

    const QString configDir = QStandardPaths::writableLocation(QStandardPaths::ConfigLocation);

    // 1. Efecto Round-Corners en kwinrc (muy común en KDE Plasma 6 para bordes redondeados globales)
    qreal roundCornersSize = readKdeConfigValue(configDir + QStringLiteral("/kwinrc"), QStringLiteral("Round-Corners"), QStringLiteral("Size"), -1.0);
    if (roundCornersSize > 0.0) {
        return std::clamp(roundCornersSize, 2.0, 32.0);
    }
    qreal inactiveRadius = readKdeConfigValue(configDir + QStringLiteral("/kwinrc"), QStringLiteral("Round-Corners"), QStringLiteral("InactiveCornerRadius"), -1.0);
    if (inactiveRadius > 0.0) {
        return std::clamp(inactiveRadius, 2.0, 32.0);
    }

    // 2. Tema Klassy (Windeco -> WindowCornerRadius)
    qreal klassyRadius = readKdeConfigValue(configDir + QStringLiteral("/klassy/klassyrc"), QStringLiteral("Windeco"), QStringLiteral("WindowCornerRadius"), -1.0);
    if (klassyRadius > 0.0) {
        return std::clamp(klassyRadius, 2.0, 32.0);
    }

    // 3. Tema Darkly (Common -> CornerRadius, Windeco -> OtherCornerRadius)
    qreal darklyRadius = readKdeConfigValue(configDir + QStringLiteral("/darklyrc"), QStringLiteral("Common"), QStringLiteral("CornerRadius"), -1.0);
    if (darklyRadius > 0.0) {
        return std::clamp(darklyRadius, 2.0, 32.0);
    }
    qreal darklyOtherRadius = readKdeConfigValue(configDir + QStringLiteral("/darklyrc"), QStringLiteral("Windeco"), QStringLiteral("OtherCornerRadius"), -1.0);
    if (darklyOtherRadius > 0.0) {
        return std::clamp(darklyOtherRadius, 2.0, 32.0);
    }

    // 4. Valor por defecto estándar en KDE Plasma 6 (Breeze)
    return 8.0;
}

/**
 * @brief Resuelve adaptativamente el radio de curvatura exacto para el contorno de una ventana.
 */
KWin::BorderRadius RavenEffect::resolveAdaptiveBorderRadius(KWin::EffectWindow *w) const
{
#ifndef HAVE_KWIN_OUTLINED_BORDER
    Q_UNUSED(w);
    return KWin::BorderRadius();
#else
    if (m_customBorderRadius >= 0.0) {
        return KWin::BorderRadius(m_customBorderRadius);
    }

    // 1. Reporte nativo de KWin (si el cliente Wayland o el decorador informan radio real)
    if (w && w->window()) {
        const KWin::BorderRadius nativeRadius = w->window()->borderRadius();
        if (!nativeRadius.isNull() && (nativeRadius.topLeft() > 0.0 || nativeRadius.topRight() > 0.0 ||
                                       nativeRadius.bottomLeft() > 0.0 || nativeRadius.bottomRight() > 0.0)) {
            if (nativeRadius.bottomLeft() == 0.0 && nativeRadius.bottomRight() == 0.0 && nativeRadius.topLeft() > 0.0) {
                qreal sysR = detectSystemCornerRadius();
                if (sysR > 0.0) {
                    return KWin::BorderRadius(nativeRadius.topLeft(), nativeRadius.topRight(), sysR, sysR);
                }
            }
            return nativeRadius;
        }

#if defined(HAVE_KDECORATION)
        if (auto *decor = w->window()->decoration()) {
            const auto decorRadius = KWin::BorderRadius::from(decor->borderRadius());
            if (!decorRadius.isNull() && (decorRadius.topLeft() > 0.0 || decorRadius.topRight() > 0.0 ||
                                          decorRadius.bottomLeft() > 0.0 || decorRadius.bottomRight() > 0.0)) {
                if (decorRadius.bottomLeft() == 0.0 && decorRadius.bottomRight() == 0.0 && decorRadius.topLeft() > 0.0) {
                    qreal sysR = detectSystemCornerRadius();
                    if (sysR > 0.0) {
                        return KWin::BorderRadius(decorRadius.topLeft(), decorRadius.topRight(), sysR, sysR);
                    }
                }
                return decorRadius;
            }
        }
#endif
    }

    // 2. Resolver radio adaptativo del entorno de escritorio (Round-Corners 12px, Klassy 8px, Darkly 6/12px, Breeze 8px)
    qreal adaptiveR = detectSystemCornerRadius();

    // 3. Heurística contextual por clase de aplicación para clientes GTK4 / Libadwaita
    if (w && adaptiveR <= 8.0) {
        QString cls = w->windowClass().toLower();
        if (cls.contains(QStringLiteral("nautilus"))
            || cls.contains(QStringLiteral("gnome-"))
            || cls.contains(QStringLiteral("pavucontrol"))
            || cls.contains(QStringLiteral("adwaita"))) {
            adaptiveR = 12.0;
        }
    }

    return KWin::BorderRadius(adaptiveR);
#endif
}

/**
 * @brief Crea o actualiza el OutlinedBorderItem de la ventana respetando su radio de curvatura.
 */
void RavenEffect::updateBorder(KWin::EffectWindow *w)
{
#ifndef HAVE_KWIN_OUTLINED_BORDER
    Q_UNUSED(w);
    return;
#else
    if (!w || !m_bordersEnabled || !shouldDrawBorder(w)) {
        removeBorder(w);
        return;
    }

    KWin::WindowItem *parentItem = w->windowItem();
    if (!parentItem) {
        removeBorder(w);
        return;
    }

    bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
    QColor color = isFocused ? m_activeBorderColor : m_inactiveBorderColor;

    if (color.alpha() == 0 || m_borderWidth <= 0.0f) {
        removeBorder(w);
        return;
    }

    // Obtener la curvatura real o adaptativa de la ventana
    KWin::BorderRadius radius = resolveAdaptiveBorderRadius(w);

    KWin::BorderOutline outline(m_borderWidth, color, radius);
    KWin::RectF innerRect(QPointF(0, 0), w->frameGeometry().size());

    auto it = m_borders.find(w);
    if (it != m_borders.end()) {
        if (it->second.item) {
            it->second.item->setInnerRect(innerRect);
            it->second.item->setOutline(outline);
        }
#if defined(HAVE_KDECORATION)
        if (w->window() && w->window()->decoration() && !it->second.decorationConnection) {
            auto *decor = w->window()->decoration();
            it->second.decorationConnection = connect(decor, &KDeco::Decoration::borderRadiusChanged, this, [this, w]() {
                updateBorder(w);
            });
        }
#endif
    } else {
        BorderData data;
        data.item = std::make_unique<KWin::OutlinedBorderItem>(innerRect, outline, parentItem);
        if (w->window()) {
            data.radiusConnection = connect(w->window(), &KWin::Window::borderRadiusChanged, this, [this, w]() {
                updateBorder(w);
            });
            data.classConnection = connect(w->window(), &KWin::Window::windowClassChanged, this, [this, w]() {
                updateBorder(w);
            });
            data.captionConnection = connect(w->window(), &KWin::Window::captionChanged, this, [this, w]() {
                updateBorder(w);
            });
            data.readyConnection = connect(w->window(), &KWin::Window::readyForPaintingChanged, this, [this, w]() {
                updateBorder(w);
            });
            data.surfaceConnection = connect(w->window(), &KWin::Window::surfaceChanged, this, [this, w]() {
                updateBorder(w);
            });
            data.bufferConnection = connect(w->window(), &KWin::Window::bufferGeometryChanged, this, [this, w]() {
                updateBorder(w);
            });
            data.maximizeConnection = connect(w, &KWin::EffectWindow::windowMaximizedStateChanged, this, [this, w]() {
                updateBorder(w);
            });
#if defined(HAVE_KDECORATION)
            if (auto *decor = w->window()->decoration()) {
                data.decorationConnection = connect(decor, &KDeco::Decoration::borderRadiusChanged, this, [this, w]() {
                    updateBorder(w);
                });
            }
#endif
        }
        m_borders.emplace(w, std::move(data));
    }
    w->addRepaintFull();
#endif
}

/**
 * @brief Elimina y desconecta el OutlinedBorderItem de la ventana.
 */
void RavenEffect::removeBorder(KWin::EffectWindow *w)
{
    auto it = m_borders.find(w);
    if (it != m_borders.end()) {
        disconnect(it->second.radiusConnection);
        disconnect(it->second.classConnection);
        disconnect(it->second.captionConnection);
        disconnect(it->second.decorationConnection);
        disconnect(it->second.readyConnection);
        disconnect(it->second.surfaceConnection);
        disconnect(it->second.bufferConnection);
        disconnect(it->second.maximizeConnection);
#ifdef HAVE_KWIN_OUTLINED_BORDER
        it->second.item.reset();
#endif
        m_borders.erase(it);
        w->addRepaintFull();
    }
}

/**
 * @brief Slot llamado al cambiar el foco de la ventana activa en KWin.
 */
void RavenEffect::slotWindowActivated(KWin::EffectWindow *w)
{
    if (!m_bordersEnabled) {
        return;
    }

    if (m_lastActiveWindow && m_lastActiveWindow != w) {
        updateBorder(m_lastActiveWindow);
    }
    if (w) {
        updateBorder(w);
    }
    m_lastActiveWindow = w;
}

/**
 * @brief Slot llamado al mover, redimensionar o actualizar la geometría de una ventana.
 */
void RavenEffect::slotWindowGeometryChanged(KWin::EffectWindow *w, const KWin::RectF &oldGeometry)
{
    if (!w) {
        return;
    }

    if (m_bordersEnabled) {
        updateBorder(w);
    }

    if (w->isUserMove() || w->isUserResize()) {
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

    auto it = m_pendingAnimations.find(wid);
    if (it != m_pendingAnimations.end()) {
        if (!it->startRect.isEmpty()) {
            startRect = it->startRect;
        }
        durationMs = it->durationMs;
        easingName = it->easing;
        m_pendingAnimations.erase(it);
    }

    cancelGeometryAnimation(wid);

    QEasingCurve curve = parseEasing(easingName);

    qreal fromX = startRect.x() - currentGeom.x();
    qreal fromY = startRect.y() - currentGeom.y();

    if (qAbs(fromX) > 0.5 || qAbs(fromY) > 0.5) {
        quint64 posAnim = animate(w,
                                  KWin::AnimationEffect::Translation,
                                  0,
                                  std::chrono::milliseconds(durationMs),
                                  KWin::FPx2(0.0, 0.0),
                                  curve,
                                  0,
                                  KWin::FPx2(fromX, fromY));

        if (posAnim != 0) {
            m_activeGeometryAnimations[wid].append(posAnim);
        }
    }

    if (!m_activeBirthAnimations.contains(wid) &&
        startRect.width() > 0.0 && startRect.height() > 0.0 &&
        currentGeom.width() > 0.0 && currentGeom.height() > 0.0) {

        qreal fromScaleX = startRect.width() / currentGeom.width();
        qreal fromScaleY = startRect.height() / currentGeom.height();

        if (qAbs(fromScaleX - 1.0) > 0.005 || qAbs(fromScaleY - 1.0) > 0.005) {
            uint metaScale = 0;
            setMetaData(SourceAnchor, Anchor::Left | Anchor::Top, metaScale);

            quint64 scaleAnim = animate(w,
                                        KWin::AnimationEffect::Scale,
                                        metaScale,
                                        std::chrono::milliseconds(durationMs),
                                        KWin::FPx2(1.0, 1.0),
                                        curve,
                                        0,
                                        KWin::FPx2(fromScaleX, fromScaleY));

            if (scaleAnim != 0) {
                m_activeGeometryAnimations[wid].append(scaleAnim);
            }
        }
    }

    w->addRepaintFull();
}

/**
 * @brief Slot llamado durante el arrastre/redimensionamiento interactivo de ventanas.
 */
void RavenEffect::slotWindowStepUserMovedResized(KWin::EffectWindow *w, const KWin::RectF &geometry)
{
    Q_UNUSED(geometry);
    if (m_bordersEnabled) {
        updateBorder(w);
    }
}

/**
 * @brief Slot llamado al minimizar o restaurar una ventana.
 */
void RavenEffect::slotWindowMinimizedChanged(KWin::EffectWindow *w)
{
    if (!w) {
        return;
    }

    invalidateBorderVerdict(w);

    if (w->isMinimized()) {
        removeBorder(w);
        if (m_lastActiveWindow == w) {
            m_lastActiveWindow.clear();
        }
    } else {
        updateBorder(w);
    }
}

void RavenEffect::updateBorderConfig(bool enabled, double width, const QString &activeColor, const QString &inactiveColor, double radius)
{
    m_bordersEnabled = enabled;
    m_borderVerdictCache.clear();
    m_borderWidth = static_cast<float>(std::clamp(width, 1.0, 6.0));
    if (radius >= -1.0) {
        m_customBorderRadius = radius;
    }

    QColor ac(activeColor);
    if (ac.isValid()) {
        m_activeBorderColor = ac;
    }

    QColor ic(inactiveColor);
    if (ic.isValid()) {
        m_inactiveBorderColor = ic;
    }

    if (KWin::effects) {
        const auto windows = KWin::effects->stackingOrder();
        for (auto *win : windows) {
            if (win) {
                updateBorder(win);
            }
        }
    }
}

/**
 * @brief Modifica el radio de curvatura de esquinas (-1.0 = auto-adaptativo).
 */
void RavenEffect::setBorderRadius(double radius)
{
    m_customBorderRadius = radius;
    if (KWin::effects) {
        const auto windows = KWin::effects->stackingOrder();
        for (auto *win : windows) {
            if (win) {
                updateBorder(win);
            }
        }
    }
}

#include "moc_raveneffect.cpp"
