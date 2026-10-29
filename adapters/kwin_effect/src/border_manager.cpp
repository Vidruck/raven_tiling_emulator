/*
 * border_manager.cpp
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 */

#include "border_manager.h"
#include "window_filter.h"
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
#include <QJsonObject>
#include <QFile>
#include <QStandardPaths>
#include <cmath>
#include <algorithm>

BorderManager::BorderManager()
{
    loadConfig();
}

BorderManager::~BorderManager()
{
    clearAll();
}

void BorderManager::clearAll()
{
    for (auto &pair : m_borders) {
        QObject::disconnect(pair.second.radiusConnection);
        QObject::disconnect(pair.second.decorationConnection);
#ifdef HAVE_KWIN_OUTLINED_BORDER
        pair.second.item.reset();
#endif
    }
    m_borders.clear();
}

void BorderManager::loadConfig()
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

qreal BorderManager::detectSystemCornerRadius() const
{
    if (m_customBorderRadius >= 0.0) {
        return m_customBorderRadius;
    }

    if (m_cachedSystemCornerRadius > 0.0) {
        return m_cachedSystemCornerRadius;
    }

    const QString configDir = QStandardPaths::writableLocation(QStandardPaths::ConfigLocation);

    qreal roundCornersSize = readKdeConfigValue(configDir + QStringLiteral("/kwinrc"), QStringLiteral("Round-Corners"), QStringLiteral("Size"), -1.0);
    if (roundCornersSize > 0.0) {
        m_cachedSystemCornerRadius = std::clamp(roundCornersSize, 2.0, 32.0);
        return m_cachedSystemCornerRadius;
    }
    qreal inactiveRadius = readKdeConfigValue(configDir + QStringLiteral("/kwinrc"), QStringLiteral("Round-Corners"), QStringLiteral("InactiveCornerRadius"), -1.0);
    if (inactiveRadius > 0.0) {
        m_cachedSystemCornerRadius = std::clamp(inactiveRadius, 2.0, 32.0);
        return m_cachedSystemCornerRadius;
    }

    qreal klassyRadius = readKdeConfigValue(configDir + QStringLiteral("/klassy/klassyrc"), QStringLiteral("Windeco"), QStringLiteral("WindowCornerRadius"), -1.0);
    if (klassyRadius > 0.0) {
        m_cachedSystemCornerRadius = std::clamp(klassyRadius, 2.0, 32.0);
        return m_cachedSystemCornerRadius;
    }

    qreal darklyRadius = readKdeConfigValue(configDir + QStringLiteral("/darklyrc"), QStringLiteral("Common"), QStringLiteral("CornerRadius"), -1.0);
    if (darklyRadius > 0.0) {
        m_cachedSystemCornerRadius = std::clamp(darklyRadius, 2.0, 32.0);
        return m_cachedSystemCornerRadius;
    }
    qreal darklyOtherRadius = readKdeConfigValue(configDir + QStringLiteral("/darklyrc"), QStringLiteral("Windeco"), QStringLiteral("OtherCornerRadius"), -1.0);
    if (darklyOtherRadius > 0.0) {
        m_cachedSystemCornerRadius = std::clamp(darklyOtherRadius, 2.0, 32.0);
        return m_cachedSystemCornerRadius;
    }

    m_cachedSystemCornerRadius = 8.0;
    return m_cachedSystemCornerRadius;
}

KWin::BorderRadius BorderManager::resolveAdaptiveBorderRadius(KWin::EffectWindow *w) const
{
#ifndef HAVE_KWIN_OUTLINED_BORDER
    Q_UNUSED(w);
    return KWin::BorderRadius();
#else
    if (m_customBorderRadius >= 0.0) {
        return KWin::BorderRadius(m_customBorderRadius);
    }

    if (!w) {
        return KWin::BorderRadius();
    }

    if (m_cachedRoundCornersRadius < -1.5) {
        const QString configDir = QStandardPaths::writableLocation(QStandardPaths::ConfigLocation);
        qreal rcSize = readKdeConfigValue(configDir + QStringLiteral("/kwinrc"), QStringLiteral("Round-Corners"), QStringLiteral("Size"), -1.0);
        if (rcSize <= 0.0) {
            rcSize = readKdeConfigValue(configDir + QStringLiteral("/kwinrc"), QStringLiteral("Round-Corners"), QStringLiteral("InactiveCornerRadius"), -1.0);
        }
        m_cachedRoundCornersRadius = (rcSize > 0.0) ? std::clamp(rcSize, 2.0, 32.0) : -1.0;
    }

    const bool hasServerDeco = w->hasDecoration() || (w->window() && w->window()->isDecorated() && !w->window()->noBorder());

    if (w->window()) {
        const KWin::BorderRadius nativeRadius = w->window()->borderRadius();
        if (!nativeRadius.isNull() && (nativeRadius.topLeft() > 0.0 || nativeRadius.topRight() > 0.0 ||
                                       nativeRadius.bottomLeft() > 0.0 || nativeRadius.bottomRight() > 0.0)) {
            if (hasServerDeco) {
                qreal sysR = detectSystemCornerRadius();
                qreal maxCorner = std::max({nativeRadius.topLeft(), nativeRadius.topRight(), nativeRadius.bottomRight(), nativeRadius.bottomLeft(), sysR});
                qreal topL = nativeRadius.topLeft() > 0.0 ? nativeRadius.topLeft() : maxCorner;
                qreal topR = nativeRadius.topRight() > 0.0 ? nativeRadius.topRight() : maxCorner;
                qreal botR = nativeRadius.bottomRight() > 0.0 ? nativeRadius.bottomRight() : maxCorner;
                qreal botL = nativeRadius.bottomLeft() > 0.0 ? nativeRadius.bottomLeft() : maxCorner;
                return KWin::BorderRadius(topL, topR, botR, botL);
            }
            return nativeRadius;
        }

#if defined(HAVE_KDECORATION)
        if (auto *decor = w->window()->decoration()) {
            const auto decorRadius = KWin::BorderRadius::from(decor->borderRadius());
            if (!decorRadius.isNull() && (decorRadius.topLeft() > 0.0 || decorRadius.topRight() > 0.0 ||
                                          decorRadius.bottomLeft() > 0.0 || decorRadius.bottomRight() > 0.0)) {
                qreal sysR = detectSystemCornerRadius();
                qreal maxCorner = std::max({decorRadius.topLeft(), decorRadius.topRight(), decorRadius.bottomRight(), decorRadius.bottomLeft(), sysR});
                qreal topL = decorRadius.topLeft() > 0.0 ? decorRadius.topLeft() : maxCorner;
                qreal topR = decorRadius.topRight() > 0.0 ? decorRadius.topRight() : maxCorner;
                qreal botR = decorRadius.bottomRight() > 0.0 ? decorRadius.bottomRight() : maxCorner;
                qreal botL = decorRadius.bottomLeft() > 0.0 ? decorRadius.bottomLeft() : maxCorner;
                return KWin::BorderRadius(topL, topR, botR, botL);
            }
        }
#endif
    }

    if (hasServerDeco) {
        return KWin::BorderRadius(detectSystemCornerRadius());
    }

    if (m_cachedRoundCornersRadius > 0.0) {
        return KWin::BorderRadius(m_cachedRoundCornersRadius);
    }

    const QString cls = w->windowClass().toLower();
    if (cls.contains(QStringLiteral("nautilus"))
        || cls.contains(QStringLiteral("gnome-"))
        || cls.contains(QStringLiteral("pavucontrol"))
        || cls.contains(QStringLiteral("adwaita"))
        || cls.contains(QStringLiteral("org.gnome."))) {
        return KWin::BorderRadius(12.0);
    }

    return KWin::BorderRadius(0.0);
#endif
}

void BorderManager::updateBorder(KWin::EffectWindow *w, bool isFocused, WindowFilter &filter)
{
#ifndef HAVE_KWIN_OUTLINED_BORDER
    Q_UNUSED(w);
    Q_UNUSED(isFocused);
    Q_UNUSED(filter);
    return;
#else
    if (!w || !m_bordersEnabled || !filter.shouldDrawBorder(w)) {
        removeBorder(w);
        return;
    }

    KWin::WindowItem *parentItem = w->windowItem();
    if (!parentItem) {
        removeBorder(w);
        return;
    }

    QColor color = isFocused ? m_activeBorderColor : m_inactiveBorderColor;

    if (color.alpha() == 0 || m_borderWidth <= 0.0f) {
        removeBorder(w);
        return;
    }

    KWin::BorderRadius radius = resolveAdaptiveBorderRadius(w);
    KWin::BorderOutline outline(m_borderWidth, color, radius);
    const QSizeF currentSize = w->frameGeometry().size();
    KWin::RectF innerRect(QPointF(0, 0), currentSize);

    auto it = m_borders.find(w);
    if (it != m_borders.end()) {
        const bool changed = (it->second.lastSize != currentSize)
            || (it->second.lastColor != color)
            || (it->second.lastRadius != radius)
            || (std::abs(it->second.lastWidth - m_borderWidth) > 0.01f);

        if (it->second.item && changed) {
            it->second.item->setInnerRect(innerRect);
            it->second.item->setOutline(outline);
            it->second.lastSize = currentSize;
            it->second.lastColor = color;
            it->second.lastRadius = radius;
            it->second.lastWidth = m_borderWidth;
            w->addRepaintFull();
        }
#if defined(HAVE_KDECORATION)
        if (w->window() && w->window()->decoration() && !it->second.decorationConnection) {
            auto *decor = w->window()->decoration();
            it->second.decorationConnection = QObject::connect(decor, &KDeco::Decoration::borderRadiusChanged, [this, w, &filter]() {
                if (auto borderIt = m_borders.find(w); borderIt != m_borders.end()) {
                    borderIt->second.lastWidth = -1.0f;
                }
                bool focused = (KWin::effects && w == KWin::effects->activeWindow());
                updateBorder(w, focused, filter);
            });
        }
#endif
    } else {
        BorderData data;
        data.item = std::make_unique<KWin::OutlinedBorderItem>(innerRect, outline, parentItem);
        data.lastSize = currentSize;
        data.lastColor = color;
        data.lastRadius = radius;
        data.lastWidth = m_borderWidth;
        if (w->window()) {
            data.radiusConnection = QObject::connect(w->window(), &KWin::Window::borderRadiusChanged, [this, w, &filter]() {
                if (auto borderIt = m_borders.find(w); borderIt != m_borders.end()) {
                    borderIt->second.lastWidth = -1.0f;
                }
                bool focused = (KWin::effects && w == KWin::effects->activeWindow());
                updateBorder(w, focused, filter);
            });
#if defined(HAVE_KDECORATION)
            if (auto *decor = w->window()->decoration()) {
                data.decorationConnection = QObject::connect(decor, &KDeco::Decoration::borderRadiusChanged, [this, w, &filter]() {
                    if (auto borderIt = m_borders.find(w); borderIt != m_borders.end()) {
                        borderIt->second.lastWidth = -1.0f;
                    }
                    bool focused = (KWin::effects && w == KWin::effects->activeWindow());
                    updateBorder(w, focused, filter);
                });
            }
#endif
        }
        m_borders.emplace(w, std::move(data));
        w->addRepaintFull();
    }
#endif
}

void BorderManager::removeBorder(KWin::EffectWindow *w)
{
    auto it = m_borders.find(w);
    if (it != m_borders.end()) {
        QObject::disconnect(it->second.radiusConnection);
        QObject::disconnect(it->second.decorationConnection);
#ifdef HAVE_KWIN_OUTLINED_BORDER
        if (it->second.item) {
            it->second.item->setVisible(false);
            it->second.item->setParentItem(nullptr);
            it->second.item.reset();
        }
#endif
        m_borders.erase(it);
        if (w) {
            w->addRepaintFull();
        }
    }
}

void BorderManager::prePaintWindow(KWin::EffectWindow *w, WindowFilter &filter)
{
    if (!w) return;

    if (w->isDeleted() || w->isHidden() || w->isMinimized()) {
        auto it = m_borders.find(w);
        if (it != m_borders.end() && it->second.item) {
            it->second.item->setVisible(false);
        }
    } else if (m_bordersEnabled) {
        auto it = m_borders.find(w);
        if (it == m_borders.end() || !it->second.item) {
            if (filter.shouldDrawBorder(w)) {
                bool isFocused = (KWin::effects && w == KWin::effects->activeWindow());
                updateBorder(w, isFocused, filter);
            }
        }
    }
}

void BorderManager::updateConfig(bool enabled, double width, const QString &activeColor, const QString &inactiveColor, double radius)
{
    m_bordersEnabled = enabled;
    m_cachedSystemCornerRadius = -1.0;
    m_cachedRoundCornersRadius = -2.0;
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

    for (auto &pair : m_borders) {
        pair.second.lastWidth = -1.0f;
    }
}

void BorderManager::setBorderRadius(double radius)
{
    m_customBorderRadius = radius;
    m_cachedSystemCornerRadius = -1.0;
    m_cachedRoundCornersRadius = -2.0;
    for (auto &pair : m_borders) {
        pair.second.lastWidth = -1.0f;
    }
}

