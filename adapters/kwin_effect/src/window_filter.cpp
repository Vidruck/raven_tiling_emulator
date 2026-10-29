/*
 * window_filter.cpp
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 */

#include "window_filter.h"
#include <effect/effecthandler.h>
#include <window.h>
#include <QStringList>

WindowFilter::WindowFilter()
{
}

void WindowFilter::invalidate(const KWin::EffectWindow *w)
{
    if (w) {
        m_borderVerdictCache.remove(w);
    }
}

void WindowFilter::clearCache()
{
    m_borderVerdictCache.clear();
}

bool WindowFilter::hasBirthed(const KWin::EffectWindow *w) const
{
    return m_birthedWindows.contains(w);
}

void WindowFilter::markBirthed(const KWin::EffectWindow *w)
{
    if (w) {
        if (m_birthedWindows.size() > 512) {
            m_birthedWindows.clear();
        }
        m_birthedWindows.insert(w);
    }
}

void WindowFilter::removeBirthed(const KWin::EffectWindow *w)
{
    if (w) {
        m_birthedWindows.remove(w);
    }
}

void WindowFilter::clearBirthed()
{
    m_birthedWindows.clear();
}

bool WindowFilter::shouldDrawBorder(KWin::EffectWindow *w)
{
    if (!w) {
        return false;
    }
    auto it = m_borderVerdictCache.constFind(w);
    if (it != m_borderVerdictCache.constEnd()) {
        return it.value();
    }
    if (m_borderVerdictCache.size() > 512) {
        m_borderVerdictCache.clear();
    }
    bool verdict = computeShouldDrawBorder(w);
    m_borderVerdictCache.insert(w, verdict);
    return verdict;
}

bool WindowFilter::shouldAnimateBirth(KWin::EffectWindow *w) const
{
    if (!w) {
        return false;
    }

    if (w->isDeleted()
        || w->isMinimized()
        || w->isHidden()
        || w->isFullScreen()) {
        return false;
    }

    if (KWin::EffectWindow *parent = w->transientFor()) {
        if (parent->isDeleted() || parent->isHidden() || !parent->isVisible()) {
            return false;
        }
    }

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
        || w->isPopupWindow()
        || w->isOutline()
        || w->isInputMethod()
        || w->isSpecialWindow()
        || w->isSplash()) {
        return false;
    }

    const QString cls = w->windowClass().toLower();
    const QString role = w->windowRole().toLower();

    static const QStringList kSystemBlacklist = {
        QStringLiteral("plasmashell"),
        QStringLiteral("kscreenlocker"),
        QStringLiteral("kscreenlocker_greet"),
        QStringLiteral("plasma-lockscreen"),
        QStringLiteral("plasmalogin"),
        QStringLiteral("sddm"),
        QStringLiteral("ksplash"),
        QStringLiteral("ksplashqml"),
        QStringLiteral("krunner"),
        QStringLiteral("org.kde.krunner"),
        QStringLiteral("kwin_wayland"),
        QStringLiteral("kwin_x11"),
        QStringLiteral("xwaylandvideobridge"),
        QStringLiteral("org.kde.kscreen.osd")
    };

    for (const QString &sc : kSystemBlacklist) {
        if (cls.contains(sc) || role.contains(sc)) {
            return false;
        }
    }

    if (w->isDialog() || w->isUtility() || w->isModal() || w->transientFor() != nullptr) {
        return false;
    }
    if (w->window() && (w->window()->isTransient() || w->window()->isModal())) {
        return false;
    }
    return w->isNormalWindow();
}

bool WindowFilter::computeShouldDrawBorder(KWin::EffectWindow *w) const
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

    if (KWin::EffectWindow *parent = w->transientFor()) {
        if (parent->isDeleted() || parent->isHidden() || !parent->isVisible()) {
            return false;
        }
    }
    if (w->findModal() != nullptr && w->findModal() != w) {
        return false;
    }
    if ((w->isDialog() || w->isUtility() || w->isModal() || w->transientFor() != nullptr)
        && KWin::effects && KWin::effects->activeWindow() != w) {
        return false;
    }

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
        || w->isPopupWindow()
        || w->isOutline()
        || w->isInputMethod()
        || w->isSpecialWindow()
        || w->isSplash()) {
        return false;
    }

    const QString cls = w->windowClass().toLower();
    const QString role = w->windowRole().toLower();
    const QString cap = w->caption().toLower();

    if (cls.trimmed().isEmpty() && cap.trimmed().isEmpty()) {
        if (!w->isNormalWindow() && !w->isDialog() && !w->isUtility()) {
            return false;
        }
    }

    if (cls.contains(QStringLiteral("plasma-welcome"))
        || cls.contains(QStringLiteral("plasma.welcome"))
        || cls.contains(QStringLiteral("welcome"))
        || cls.contains(QStringLiteral("plasmashell"))
        || cls.contains(QStringLiteral("kscreenlocker"))
        || cls.contains(QStringLiteral("screenlocker"))
        || cls.contains(QStringLiteral("lockscreen"))
        || cls.contains(QStringLiteral("plasmalogin"))
        || cls.contains(QStringLiteral("sddm"))
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
        QStringLiteral("kded"),
        QStringLiteral("org.kde.kdeconnect"),
        QStringLiteral("greeter"),
        QStringLiteral("org.kde.plasma.emojier"),
        QStringLiteral("org.kde.kscreen.osd"),
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

    return w->isNormalWindow() || w->isDialog() || w->isUtility();
}
