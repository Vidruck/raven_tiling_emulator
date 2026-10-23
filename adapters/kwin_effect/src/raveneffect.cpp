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
#include <kdecoration3/decoration.h>
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
                connect(win, &KWin::EffectWindow::windowFrameGeometryChanged, this, &RavenEffect::slotWindowGeometryChanged);
                connect(win, &KWin::EffectWindow::windowStepUserMovedResized, this, &RavenEffect::slotWindowStepUserMovedResized);
                connect(win, &KWin::EffectWindow::minimizedChanged, this, &RavenEffect::slotWindowMinimizedChanged);
                if (win->window()) {
                    connect(win->window(), &KWin::Window::windowClassChanged, win, [this, win]() {
                        if (m_bordersEnabled) updateBorder(win);
                    });
                    connect(win->window(), &KWin::Window::captionChanged, win, [this, win]() {
                        if (m_bordersEnabled) updateBorder(win);
                    });
                }

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

        const auto windows = KWin::effects->stackingOrder();
        for (auto *win : windows) {
            if (win) {
                disconnect(win, &KWin::EffectWindow::windowFrameGeometryChanged, this, &RavenEffect::slotWindowGeometryChanged);
                disconnect(win, &KWin::EffectWindow::windowStepUserMovedResized, this, &RavenEffect::slotWindowStepUserMovedResized);
                disconnect(win, &KWin::EffectWindow::minimizedChanged, this, &RavenEffect::slotWindowMinimizedChanged);
            }
        }
    }

    // Limpieza de todos los bordes nativos
    for (auto &pair : m_borders) {
        disconnect(pair.second.radiusConnection);
        disconnect(pair.second.classConnection);
        disconnect(pair.second.captionConnection);
        disconnect(pair.second.decorationConnection);
        pair.second.item.reset();
    }
    m_borders.clear();
    m_windowByUuid.clear();

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
 * @brief Slot que se ejecuta cuando se añade una nueva ventana al compositor.
 */
void RavenEffect::slotWindowAdded(KWin::EffectWindow *w)
{
    if (!w) {
        return;
    }

    m_windowByUuid.insert(w->internalId(), w);

    connect(w, &KWin::EffectWindow::windowFrameGeometryChanged, this, &RavenEffect::slotWindowGeometryChanged);
    connect(w, &KWin::EffectWindow::windowStepUserMovedResized, this, &RavenEffect::slotWindowStepUserMovedResized);
    connect(w, &KWin::EffectWindow::minimizedChanged, this, &RavenEffect::slotWindowMinimizedChanged);

    if (w->window()) {
        connect(w->window(), &KWin::Window::windowClassChanged, w, [this, w]() {
            if (m_bordersEnabled) updateBorder(w);
        });
        connect(w->window(), &KWin::Window::captionChanged, w, [this, w]() {
            if (m_bordersEnabled) updateBorder(w);
        });
    }

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

    removeBorder(w);
    m_windowByUuid.remove(w->internalId());

    if (m_lastActiveWindow == w) {
        m_lastActiveWindow.clear();
    }

    QString windowId = w->internalId().toString();
    if (!windowId.isEmpty()) {
        cancelWindowAnimation(windowId);
    }

    QString hexId = QStringLiteral("0x%1").arg(reinterpret_cast<quintptr>(w), 0, 16);
    cancelWindowAnimation(hexId);

    QString decId = QString::number(reinterpret_cast<quintptr>(w));
    cancelWindowAnimation(decId);
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

    // Cancelar únicamente animaciones de traslación/geometría previas,
    // preservando la animación de nacimiento (scale/opacity) si está en curso
    cancelGeometryAnimation(windowId);

    if (durationMs <= 0 || durationMs > 500) {
        durationMs = 140;
    }

    QEasingCurve curve = parseEasing(easingName);

    // FLIP Robusto:
    // w->frameGeometry() puede ya ser targetRect (si KWin script ya actualizó la geometría)
    // o todavía puede ser startRect (si KWin script aún no ha procesado el cambio en el compositor).
    // Para ser completamente inmune a carreras de sincronización entre D-Bus y el compositor:
    // Calculamos el offset relativo a la geometría actual G = w->frameGeometry():
    // Al inicio (t=0), la posición visual es G + from = startRect.
    // Al final (t=1), la posición visual es G + to = targetRect.
    const QRectF currentGeom = w->frameGeometry();
    qreal fromX = startRect.x() - currentGeom.x();
    qreal fromY = startRect.y() - currentGeom.y();
    qreal toX = targetRect.x() - currentGeom.x();
    qreal toY = targetRect.y() - currentGeom.y();

    if (qAbs(fromX - toX) > 0.5 || qAbs(fromY - toY) > 0.5) {
        quint64 posAnim = animate(w,
                                  KWin::AnimationEffect::Translation,
                                  0,
                                  std::chrono::milliseconds(durationMs),
                                  KWin::FPx2(toX, toY),
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
        qreal toScaleX = targetRect.width() / currentGeom.width();
        qreal toScaleY = targetRect.height() / currentGeom.height();

        if (qAbs(fromScaleX - toScaleX) > 0.005 || qAbs(fromScaleY - toScaleY) > 0.005) {
            uint metaScale = 0;
            setMetaData(SourceAnchor, Anchor::Left | Anchor::Top, metaScale);

            quint64 scaleAnim = animate(w,
                                        KWin::AnimationEffect::Scale,
                                        metaScale,
                                        std::chrono::milliseconds(durationMs),
                                        KWin::FPx2(toScaleX, toScaleY),
                                        curve,
                                        0,
                                        KWin::FPx2(fromScaleX, fromScaleY));

            if (scaleAnim != 0) {
                m_activeGeometryAnimations[windowId].append(scaleAnim);
            }
        }
    }
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
}

/**
 * @brief Comprueba si una ventana es apta para recibir contornos visuales.
 */
bool RavenEffect::shouldDrawBorder(KWin::EffectWindow *w) const
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

    // Si aún no tiene clase ni título (ventana Wayland recién creada), esperar a que se pueblen
    if (cls.trimmed().isEmpty() && cap.trimmed().isEmpty()) {
        return false;
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
 * @brief Crea o actualiza el OutlinedBorderItem de la ventana respetando su radio de curvatura.
 */
void RavenEffect::updateBorder(KWin::EffectWindow *w)
{
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

    // Obtener la curvatura real de la ventana reportada nativamente por KWin
    KWin::BorderRadius radius;
    KDecoration3::Decoration *decor = nullptr;
    if (w->window()) {
        radius = w->window()->borderRadius();
        decor = w->window()->decoration();
        if (radius.isNull() && decor) {
            const auto decorRadius = KWin::BorderRadius::from(decor->borderRadius());
            if (!decorRadius.isNull()) {
                radius = decorRadius;
            }
        }
    }

    KWin::BorderOutline outline(m_borderWidth, color, radius);
    KWin::RectF innerRect(QPointF(0, 0), w->frameGeometry().size());

    auto it = m_borders.find(w);
    if (it != m_borders.end()) {
        if (it->second.item) {
            it->second.item->setInnerRect(innerRect);
            it->second.item->setOutline(outline);
        }
        // Reconectar señal de decoración si cambió o se instanció posteriormente
        if (decor && !it->second.decorationConnection) {
            it->second.decorationConnection = connect(decor, &KDecoration3::Decoration::borderRadiusChanged, this, [this, w]() {
                updateBorder(w);
            });
        }
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
            if (decor) {
                data.decorationConnection = connect(decor, &KDecoration3::Decoration::borderRadiusChanged, this, [this, w]() {
                    updateBorder(w);
                });
            }
        }
        m_borders.emplace(w, std::move(data));
    }
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
        it->second.item.reset();
        m_borders.erase(it);
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
    Q_UNUSED(oldGeometry);
    if (m_bordersEnabled) {
        updateBorder(w);
    }
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

    if (w->isMinimized()) {
        removeBorder(w);
        if (m_lastActiveWindow == w) {
            m_lastActiveWindow.clear();
        }
    } else {
        updateBorder(w);
    }
}

/**
 * @brief Actualiza la configuración de bordes y refresca todas las ventanas en tiempo real.
 */
void RavenEffect::updateBorderConfig(bool enabled, double width, const QString &activeColor, const QString &inactiveColor)
{
    m_bordersEnabled = enabled;
    m_borderWidth = static_cast<float>(std::clamp(width, 1.0, 6.0));

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

#include "moc_raveneffect.cpp"
