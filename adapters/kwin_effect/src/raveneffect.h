/*
 * raveneffect.h
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 * 
 * Descripción: Fachada (Facade) orquestadora del efecto nativo de KWin para Raven.
 * Conecta y delega en subsistemas especializados con bajo acoplamiento:
 * - WindowFilter: Clasificación, heurísticas y caché O(1) de elegibilidad visual.
 * - BorderManager: Renderizado y ciclo de vida de contornos OutlinedBorderItem.
 * - AnimationController: Interpolación geométrica FLIP y efectos de nacimiento.
 */

#pragma once

#include <effect/animationeffect.h>
#include <effect/effectwindow.h>
#include <QHash>
#include <QPointer>
#include <QUuid>

#include "window_filter.h"
#include "border_manager.h"
#include "animation_controller.h"

class RavenEffectAdaptor;
class AnimationController;

class RavenEffect : public KWin::AnimationEffect
{
    Q_OBJECT
    friend class AnimationController;

public:
    RavenEffect();
    ~RavenEffect() override;

    static bool supported();
    bool isActive() const override;

    // Control y animación de ventanas (delegado a AnimationController)
    void animateWindowGeometry(const QString &windowId, const QRectF &startRect, const QRectF &targetRect,
                               int durationMs, const QString &easingName);
    void animateBatchGeometry(const QString &jsonPayload);
    void animateWindowBirth(const QString &windowId, int durationMs = 180);
    void animateWindowBirth(KWin::EffectWindow *w, int durationMs = 180);
    void cancelWindowAnimation(const QString &windowId);

    // Acciones de ventana nativas
    void minimizeWindow(const QString &windowId);
    void unminimizeWindow(const QString &windowId);
    void closeWindow(const QString &windowId);
    void activateWindow(const QString &windowId);

    // Configuración de contornos (delegado a BorderManager)
    void updateBorderConfig(bool enabled, double width, const QString &activeColor, const QString &inactiveColor, double radius = -1.0);
    void setBorderRadius(double radius);
    qreal detectSystemCornerRadius() const;
    KWin::BorderRadius resolveAdaptiveBorderRadius(KWin::EffectWindow *w) const;

protected:
    void prePaintWindow(KWin::RenderView *view, KWin::EffectWindow *w, KWin::WindowPrePaintData &data) override;
    void animationEnded(KWin::EffectWindow *w, Attribute a, uint meta) override;

private Q_SLOTS:
    void slotWindowAdded(KWin::EffectWindow *w);
    void slotWindowClosed(KWin::EffectWindow *w);
    void slotWindowDeleted(KWin::EffectWindow *w);
    void slotWindowActivated(KWin::EffectWindow *w);
    void slotWindowGeometryChanged(KWin::EffectWindow *w, const KWin::RectF &oldGeometry);
    void slotWindowStepUserMovedResized(KWin::EffectWindow *w, const KWin::RectF &geometry);
    void slotWindowMinimizedChanged(KWin::EffectWindow *w);

private:
    KWin::EffectWindow *findWindowById(const QString &windowId) const;
    void setupWindowConnections(KWin::EffectWindow *w);

    RavenEffectAdaptor *m_dbusAdaptor;

    // Subsistemas especializados
    WindowFilter m_filter;
    BorderManager m_borderManager;
    AnimationController m_animController;

    QHash<QUuid, KWin::EffectWindow*> m_windowByUuid;
    QHash<KWin::EffectWindow*, QList<QMetaObject::Connection>> m_windowConnections;
    QPointer<KWin::EffectWindow> m_lastActiveWindow;
};
