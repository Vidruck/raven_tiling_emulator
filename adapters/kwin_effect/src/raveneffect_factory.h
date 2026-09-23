#pragma once

#include "raveneffect.h"
#include <effect/effect.h>
#include <QObject>

/**
 * @brief Fábrica para instanciar el efecto de Raven en KWin.
 * 
 * Interfaz requerida por KWin para descubrir y cargar efectos de terceros.
 */
class RavenEffectFactory : public KWin::EffectPluginFactory
{
    Q_OBJECT
    Q_PLUGIN_METADATA(IID EffectPluginFactory_iid FILE "metadata.json")
    Q_INTERFACES(KPluginFactory)

public:
    /**
     * @brief Constructor por defecto.
     */
    RavenEffectFactory() = default;

    /**
     * @brief Destructor.
     */
    ~RavenEffectFactory() override = default;

    /**
     * @brief Verifica si el efecto está soportado por el compositor actual.
     * @return true si es compatible.
     */
    bool isSupported() const override
    {
        return RavenEffect::supported();
    }

    bool enabledByDefault() const override
    {
        return true;
    }

    KWin::Effect *createEffect() const override
    {
        return new RavenEffect();
    }
};
