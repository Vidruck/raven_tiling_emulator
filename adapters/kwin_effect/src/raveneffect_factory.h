/*
 * raveneffect_factory.h
 *
 * Autor: Alejandro González Hernández (Vidruck)
 * Licencia: GPL-3.0
 * 
 * Descripción: Factoría oficial de KWin 6 para el registro y carga del plugin de efecto Raven.
 */

#pragma once

#include "raveneffect.h"
#include <effect/effect.h>

KWIN_EFFECT_FACTORY_SUPPORTED(RavenEffect, "metadata.json", return RavenEffect::supported();)

