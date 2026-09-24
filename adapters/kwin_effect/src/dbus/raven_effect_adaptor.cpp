/*
 * raven_effect_adaptor.cpp
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 * 
 * Descripción: Implementación del adaptador D-Bus para el efecto nativo de KWin.
 */

#include "raven_effect_adaptor.h"
#include "raveneffect.h"
#include <QRectF>

RavenEffectAdaptor::RavenEffectAdaptor(RavenEffect *parent)
    : QDBusAbstractAdaptor(parent)
    , m_effect(parent)
{
    setAutoRelaySignals(true);
}

void RavenEffectAdaptor::AnimateGeometry(const QString &windowId, int startX, int startY, int startW, int startH,
                                         int targetX, int targetY, int targetW, int targetH, int durationMs, const QString &easing)
{
    QRectF startRect(startX, startY, startW, startH);
    QRectF targetRect(targetX, targetY, targetW, targetH);
    m_effect->animateWindowGeometry(windowId, startRect, targetRect, durationMs, easing);
}

void RavenEffectAdaptor::AnimateBatch(const QString &jsonPayload)
{
    m_effect->animateBatchGeometry(jsonPayload);
}

void RavenEffectAdaptor::AnimateBirth(const QString &windowId, int durationMs)
{
    m_effect->animateWindowBirth(windowId, durationMs);
}

void RavenEffectAdaptor::CancelAnimation(const QString &windowId)
{
    m_effect->cancelWindowAnimation(windowId);
}

bool RavenEffectAdaptor::IsActive() const
{
    return m_effect != nullptr;
}

