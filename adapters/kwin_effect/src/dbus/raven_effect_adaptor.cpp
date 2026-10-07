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

/**
 * @brief Constructor del adaptador D-Bus para el efecto Raven.
 * 
 * Configura el enrutamiento automático de señales desde el objeto nativo hacia D-Bus.
 * 
 * @param parent Puntero a la instancia principal del efecto de KWin.
 */
RavenEffectAdaptor::RavenEffectAdaptor(RavenEffect *parent)
    : QDBusAbstractAdaptor(parent)
    , m_effect(parent)
{
    setAutoRelaySignals(true);
}

/**
 * @brief Anima una ventana individual desde su posición inicial hasta la final.
 * 
 * Expone hacia D-Bus la capacidad de ejecutar interpolaciones de geometría y tamaño.
 * 
 * @param windowId Identificador de la ventana.
 * @param startX Posición X inicial.
 * @param startY Posición Y inicial.
 * @param startW Ancho inicial.
 * @param startH Alto inicial.
 * @param targetX Posición X objetivo.
 * @param targetY Posición Y objetivo.
 * @param targetW Ancho objetivo.
 * @param targetH Alto objetivo.
 * @param durationMs Duración de la animación en milisegundos.
 * @param easing Nombre de la curva de aceleración.
 */
void RavenEffectAdaptor::AnimateGeometry(const QString &windowId, int startX, int startY, int startW, int startH,
                                         int targetX, int targetY, int targetW, int targetH, int durationMs, const QString &easing)
{
    QRectF startRect(startX, startY, startW, startH);
    QRectF targetRect(targetX, targetY, targetW, targetH);
    m_effect->animateWindowGeometry(windowId, startRect, targetRect, durationMs, easing);
}

/**
 * @brief Anima un lote (batch) de ventanas en mosaico simultáneamente.
 * 
 * @param jsonPayload Carga útil en formato JSON con la información de las animaciones.
 */
void RavenEffectAdaptor::AnimateBatch(const QString &jsonPayload)
{
    m_effect->animateBatchGeometry(jsonPayload);
}

/**
 * @brief Ejecuta el efecto de zoom y opacidad de nacimiento para una nueva ventana.
 * 
 * @param windowId Identificador de la ventana.
 * @param durationMs Duración del efecto en milisegundos.
 */
void RavenEffectAdaptor::AnimateBirth(const QString &windowId, int durationMs)
{
    m_effect->animateWindowBirth(windowId, durationMs);
}

/**
 * @brief Cancela cualquier animación en curso para una ventana específica.
 * 
 * @param windowId Identificador de la ventana.
 */
void RavenEffectAdaptor::CancelAnimation(const QString &windowId)
{
    m_effect->cancelWindowAnimation(windowId);
}

/**
 * @brief Indica si el efecto está actualmente instanciado y activo.
 * 
 * @return true si el puntero al efecto es válido.
 */
bool RavenEffectAdaptor::IsActive() const
{
    return m_effect != nullptr;
}

