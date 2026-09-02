/**
 * @file RavenTheme.qml
 * @brief Sistema de diseño centralizado (Design Tokens) y temas reactivos para Raven Hub.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

pragma Singleton
import QtQuick
import "." as RavenPlugin

/**
 * @class RavenTheme
 * @brief Singleton QML que expone tokens de diseño atómico y paleta cromática sincronizada con KDE Plasma.
 *
 * Adapta dinámicamente fondos, bordes, radios y colores de acento al esquema activo en el sistema,
 * soportando temas oscuros, claros y esquemas personalizados (ej. Breeze, Catppuccin, Nord, Ayu).
 */
QtObject {
    // ── TOKENS DE ESPACIADO (SPACING TOKENS) ──
    readonly property int spacingXs: 4   ///< Espaciado extra pequeño (4px).
    readonly property int spacingSm: 8   ///< Espaciado pequeño estándar (8px).
    readonly property int spacingMd: 12  ///< Espaciado medio (12px).
    readonly property int spacingLg: 16  ///< Espaciado grande para márgenes de islas (16px).
    readonly property int spacingXl: 24  ///< Espaciado extra grande para separadores de sección (24px).
    
    // ── TOKENS DE RADIOS DE CURVATURA (CORNER RADII) ──
    readonly property int radiusSm: 8    ///< Radio de esquinas pequeño para botones (8px).
    readonly property int radiusMd: 12   ///< Radio medio para tarjetas y popups (12px).
    readonly property int radiusLg: 16   ///< Radio grande para islas principales (16px).
    readonly property int radiusXl: 20   ///< Radio extra grande para ventanas flotantes (20px).
    readonly property int radiusRound: 999 ///< Radio circular para píldoras y badges (999px).
    
    // ── ESTADO DEL TEMA DINÁMICO (LIGHT / DARK AUTOMÁTICO) ──
    readonly property bool isDark: RavenPlugin.SystemStats.isDarkTheme ///< true si KDE Plasma opera en modo oscuro.

    // ── FONDOS Y SUPERFICIES ADAPTATIVAS SEGÚN KDE PLASMA ──
    readonly property color windowBackground: RavenPlugin.SystemStats.windowBgColor ///< Color de fondo de ventana raíz.
    readonly property color viewBackground: RavenPlugin.SystemStats.viewBgColor     ///< Color de fondo de vistas y cuadrículas.
    readonly property color cardBackground: RavenPlugin.SystemStats.cardBackground ///< Color de fondo para tarjetas e islas.
    readonly property color buttonBackground: RavenPlugin.SystemStats.buttonBgColor ///< Color de botones nativo de KDE.
    readonly property color buttonTextColor: RavenPlugin.SystemStats.buttonTextColor ///< Color de texto de botones nativo.
    readonly property color cardBorder: isDark ? Qt.rgba(1, 1, 1, 0.08) : Qt.rgba(0, 0, 0, 0.08) ///< Borde sutil de tarjeta.
    readonly property color hoverBackground: isDark ? Qt.rgba(1, 1, 1, 0.12) : Qt.rgba(0, 0, 0, 0.07) ///< Fondo al pasar el cursor (Hover).
    readonly property color surfaceElevated: isDark ? Qt.rgba(1, 1, 1, 0.06) : Qt.rgba(0, 0, 0, 0.04) ///< Superficie para sub-islas elevadas.

    // ── TIPOGRAFÍA Y CONTRASTE DEL SISTEMA ──
    readonly property string fontFamily: RavenPlugin.SystemStats.generalFontFamily ///< Familia de fuentes estándar del sistema.
    readonly property string fixedFontFamily: RavenPlugin.SystemStats.fixedFontFamily ///< Familia monospace del sistema.
    readonly property color textColor: RavenPlugin.SystemStats.textColor       ///< Color de texto principal de alto contraste.
    readonly property color subTextColor: RavenPlugin.SystemStats.subTextColor ///< Color de texto secundario y subtítulos.

    // ── COLORES SEMÁNTICOS Y DE ACENTO ──
    readonly property color highlightColor: RavenPlugin.SystemStats.highlightColor ///< Color de acento nativo de Plasma del usuario.
    readonly property color positiveColor: RavenPlugin.SystemStats.positiveTextColor ///< Color semántico de éxito (verde).
    readonly property color negativeColor: RavenPlugin.SystemStats.negativeTextColor ///< Color semántico de error/alerta (rojo).
}