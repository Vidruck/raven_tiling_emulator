pragma Singleton
import QtQuick
import "." as RavenPlugin

QtObject {
    // Spacing tokens
    readonly property int spacingXs: 4
    readonly property int spacingSm: 8
    readonly property int spacingMd: 12
    readonly property int spacingLg: 16
    readonly property int spacingXl: 24
    
    // Corner radii
    readonly property int radiusSm: 8
    readonly property int radiusMd: 12
    readonly property int radiusLg: 16
    readonly property int radiusXl: 20
    readonly property int radiusRound: 999
    
    // Theme state dinámico (Light / Dark automático según KDE Plasma)
    readonly property bool isDark: RavenPlugin.SystemStats.isDarkTheme

    // Fondos reactivos adaptados al tema de Plasma
    readonly property color windowBackground: RavenPlugin.SystemStats.windowBgColor
    readonly property color viewBackground: RavenPlugin.SystemStats.viewBgColor
    readonly property color cardBackground: RavenPlugin.SystemStats.cardBackground
    readonly property color cardBorder: isDark ? Qt.rgba(1, 1, 1, 0.08) : Qt.rgba(0, 0, 0, 0.08)
    readonly property color hoverBackground: isDark ? Qt.rgba(1, 1, 1, 0.12) : Qt.rgba(0, 0, 0, 0.07)
    readonly property color surfaceElevated: isDark ? Qt.rgba(1, 1, 1, 0.06) : Qt.rgba(0, 0, 0, 0.04)

    // Tipografía de alto contraste del tema
    readonly property color textColor: RavenPlugin.SystemStats.textColor
    readonly property color subTextColor: RavenPlugin.SystemStats.subTextColor

    // Color de acento nativo de Plasma del usuario (Ayu, Nord, Catppuccin, Breeze, Custom)
    readonly property color highlightColor: RavenPlugin.SystemStats.highlightColor
}