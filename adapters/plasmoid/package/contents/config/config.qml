/**
 * @file config.qml
 * @brief Definición del modelo de configuración y categorías para el Plasmoide de Raven en KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 * @license GPL-3.0
 */

import QtQuick
import org.kde.plasma.configuration

/**
 * @class ConfigModel
 * @brief Modelo declarativo que enlaza las páginas KCM de configuración visual y preferencias del plasmoide.
 */
ConfigModel {
    ConfigCategory {
        name: "Reloj y Fecha"
        icon: "preferences-system-time"
        source: "configAppearance.qml"
    }
}
