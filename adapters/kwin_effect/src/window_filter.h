/*
 * window_filter.h
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 *
 * Descripción: Subsistema especializado en la clasificación y filtrado visual
 * de ventanas para RavenEffect (KWin 6). Determina con precisión qué ventanas
 * son aptas para recibir contornos y animaciones de nacimiento con caché O(1).
 */

#pragma once

#include <effect/effecthandler.h>
#include <effect/effectwindow.h>
#include <QHash>
#include <QSet>
#include <QString>

class WindowFilter
{
public:
    WindowFilter();
    ~WindowFilter() = default;

    /**
     * @brief Comprueba si una ventana es apta para recibir contornos visuales usando caché O(1).
     */
    bool shouldDrawBorder(KWin::EffectWindow *w);

    /**
     * @brief Computa exhaustivamente las heurísticas de elegibilidad visual de una ventana.
     */
    bool computeShouldDrawBorder(KWin::EffectWindow *w) const;

    /**
     * @brief Determina si una ventana es apta para ejecutar la animación de nacimiento (birth).
     */
    bool shouldAnimateBirth(KWin::EffectWindow *w) const;

    /**
     * @brief Invalida el veredicto en caché para una ventana específica.
     */
    void invalidate(const KWin::EffectWindow *w);

    /**
     * @brief Limpia todo el caché de veredictos.
     */
    void clearCache();

    // Gestión del registro de ventanas nacientes
    bool hasBirthed(const KWin::EffectWindow *w) const;
    void markBirthed(const KWin::EffectWindow *w);
    void removeBirthed(const KWin::EffectWindow *w);
    void clearBirthed();

private:
    QHash<const KWin::EffectWindow*, bool> m_borderVerdictCache;
    QSet<const KWin::EffectWindow*> m_birthedWindows;
};

