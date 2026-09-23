#pragma once

#include "raveneffect.h"
#include <effect/effect.h>
#include <QObject>

// Esta clase factory es el punto de entrada del plugin para KWin.
// Declara Q_PLUGIN_METADATA en un HEADER para que AUTOMOC lo procese
// y genere la sección .note.qt.metadata necesaria para el descubrimiento del efecto.
class RavenEffectFactory : public KWin::EffectPluginFactory
{
    Q_OBJECT
    Q_PLUGIN_METADATA(IID EffectPluginFactory_iid FILE "metadata.json")
    Q_INTERFACES(KPluginFactory)

public:
    RavenEffectFactory() = default;
    ~RavenEffectFactory() override = default;

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
