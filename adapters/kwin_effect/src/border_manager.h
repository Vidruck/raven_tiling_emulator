/*
 * border_manager.h
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 *
 * Descripción: Subsistema especializado en la administración y proyección de contornos
 * nativos por hardware utilizando OutlinedBorderItem de KWin 6.
 */

#pragma once

#include <effect/effectwindow.h>
#include <scene/windowitem.h>

#if __has_include(<scene/outlinedborderitem.h>)
#define HAVE_KWIN_OUTLINED_BORDER 1
#include <scene/outlinedborderitem.h>
#include <scene/borderoutline.h>
#include <scene/borderradius.h>
#elif __has_include(<kwin/scene/outlinedborderitem.h>)
#define HAVE_KWIN_OUTLINED_BORDER 1
#include <kwin/scene/outlinedborderitem.h>
#include <kwin/scene/borderoutline.h>
#include <kwin/scene/borderradius.h>
#endif

#include <QColor>
#include <QSizeF>
#include <QPointer>
#include <memory>
#include <unordered_map>

class WindowFilter;

class BorderManager
{
public:
    struct BorderData {
#ifdef HAVE_KWIN_OUTLINED_BORDER
        std::unique_ptr<KWin::OutlinedBorderItem> item;
        KWin::BorderRadius lastRadius;
#endif
        QSizeF lastSize;
        QColor lastColor;
        float lastWidth = 0.0f;
        QMetaObject::Connection radiusConnection;
        QMetaObject::Connection decorationConnection;
    };

    BorderManager();
    ~BorderManager();

    void loadConfig();
    void updateConfig(bool enabled, double width, const QString &activeColor, const QString &inactiveColor, double radius = -1.0);
    void setBorderRadius(double radius);

    bool isEnabled() const { return m_bordersEnabled; }
    float borderWidth() const { return m_borderWidth; }
    QColor activeColor() const { return m_activeBorderColor; }
    QColor inactiveColor() const { return m_inactiveBorderColor; }

    qreal detectSystemCornerRadius() const;
    KWin::BorderRadius resolveAdaptiveBorderRadius(KWin::EffectWindow *w) const;

    void updateBorder(KWin::EffectWindow *w, bool isFocused, WindowFilter &filter);
    void invalidateDecoration(KWin::EffectWindow *w);
    void removeBorder(KWin::EffectWindow *w);
    void clearAll();

    void prePaintWindow(KWin::EffectWindow *w, WindowFilter &filter);
    void refreshAllBorders(WindowFilter &filter);

    std::unordered_map<KWin::EffectWindow*, BorderData>& borders() { return m_borders; }

private:
    bool m_bordersEnabled = true;
    float m_borderWidth = 2.0f;
    QColor m_activeBorderColor = QColor(61, 174, 233, 230);
    QColor m_inactiveBorderColor = QColor(90, 101, 114, 160);
    qreal m_customBorderRadius = -1.0;
    mutable qreal m_cachedSystemCornerRadius = -1.0;
    mutable qreal m_cachedRoundCornersRadius = -2.0;

    std::unordered_map<KWin::EffectWindow*, BorderData> m_borders;
};

