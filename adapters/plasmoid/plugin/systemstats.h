/**
 * @file systemstats.h
 * @brief Monitor del sistema y sincronizador del tema visual para KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

#ifndef SYSTEMSTATS_H
#define SYSTEMSTATS_H

#include <QObject>
#include <QString>
#include <QTimer>
#include <QFileSystemWatcher>
#include <qqmlintegration.h>

/**
 * @class SystemStats
 * @brief Monitor singleton C++ de telemetría de hardware, información de la distribución y motor de temas de Plasma.
 *
 * Lee de forma asíncrona y eficiente sin librerías externas pesadas los archivos del sistema Linux:
 * - `/proc/stat` para el cálculo en tiempo real del uso de CPU.
 * - `/proc/meminfo` para RAM en uso y total.
 * - `/sys/class/power_supply/` para nivel de carga y estado de batería.
 * - `/etc/os-release` y `uname` para metadata del sistema operativo y kernel.
 * - `~/.config/kdeglobals` vía `QFileSystemWatcher` para adaptar instantáneamente los colores oscuros/claros de Raven.
 */
class SystemStats : public QObject
{
    Q_OBJECT
    QML_SINGLETON
    QML_ELEMENT

    /** @brief Porcentaje de uso total de CPU (0 a 100%). */
    Q_PROPERTY(int cpuUsage READ cpuUsage NOTIFY statsChanged)
    
    /** @brief Porcentaje de memoria RAM ocupada (0 a 100%). */
    Q_PROPERTY(int ramUsage READ ramUsage NOTIFY statsChanged)
    
    /** @brief Cadena formateada de memoria utilizada (ej. '4.2 GiB'). */
    Q_PROPERTY(QString ramUsedString READ ramUsedString NOTIFY statsChanged)
    
    /** @brief Cadena formateada de memoria total instalada (ej. '15.5 GiB'). */
    Q_PROPERTY(QString ramTotalString READ ramTotalString NOTIFY statsChanged)
    
    /** @brief Nombre de la distribución GNU/Linux detectada (ej. 'Arch Linux', 'Fedora', 'openSUSE'). */
    Q_PROPERTY(QString osName READ osName CONSTANT)
    
    /** @brief Nombre del icono Freedesktop representativo de la distribución. */
    Q_PROPERTY(QString distroIcon READ distroIcon CONSTANT)
    
    /** @brief Versión del kernel Linux en ejecución. */
    Q_PROPERTY(QString kernelVersion READ kernelVersion CONSTANT)
    
    /** @brief Tiempo transcurrido desde el encendido del equipo formateado (ej. '2h 15m'). */
    Q_PROPERTY(QString uptimeString READ uptimeString NOTIFY statsChanged)
    
    /** @brief Compositor gráfico activo ('KWin (Wayland)' o 'X11'). */
    Q_PROPERTY(QString compositor READ compositor CONSTANT)
    
    /** @brief Modelo del procesador central (CPU). */
    Q_PROPERTY(QString cpuModel READ cpuModel CONSTANT)
    
    /** @brief Nombre comercial limpio y formateado del procesador (ej. 'AMD Ryzen 5 7530U', 'Intel Core i7-12700H'). */
    Q_PROPERTY(QString cpuBrandName READ cpuBrandName CONSTANT)
    
    /** @brief Fabricante de la CPU ('intel', 'amd', 'qualcomm', 'apple', 'unknown'). */
    Q_PROPERTY(QString cpuVendor READ cpuVendor CONSTANT)
    
    /** @brief Color de acento característico del fabricante de la CPU (Intel Azul #0071C5, AMD Rojo #ED1C24, Qualcomm Blanco/Negro, etc.). */
    Q_PROPERTY(QString cpuVendorColor READ cpuVendorColor CONSTANT)
    
    /** @brief Nombre de usuario en sesión. */
    Q_PROPERTY(QString userName READ userName CONSTANT)
    
    /** @brief Ruta a la imagen de perfil del usuario (~/.face o icono predeterminado). */
    Q_PROPERTY(QString userFace READ userFace CONSTANT)
    
    /** @brief Porcentaje de carga restante de la batería (0 a 100%). */
    Q_PROPERTY(int batteryUsage READ batteryUsage NOTIFY statsChanged)
    
    /** @brief true si el equipo cuenta con una batería física instalada (ej. portátil). */
    Q_PROPERTY(bool hasBattery READ hasBattery NOTIFY statsChanged)
    
    /** @brief true si la batería se encuentra conectada al cargador y en proceso de recarga. */
    Q_PROPERTY(bool isCharging READ isCharging NOTIFY statsChanged)

    /** @brief Bandera de activación para suspender lecturas de disco si el plasmoide está oculto. */
    Q_PROPERTY(bool active READ active WRITE setActive NOTIFY activeChanged)
    
    /** @brief true si el esquema global de KDE Plasma es oscuro. */
    Q_PROPERTY(bool isDarkTheme READ isDarkTheme NOTIFY themeChanged)
    
    /** @brief Color de fondo para ventanas principales según kdeglobals. */
    Q_PROPERTY(QString windowBgColor READ windowBgColor NOTIFY themeChanged)
    
    /** @brief Color de fondo para vistas y listas. */
    Q_PROPERTY(QString viewBgColor READ viewBgColor NOTIFY themeChanged)
    
    /** @brief Color de fondo para tarjetas elevadas (Islas de Raven). */
    Q_PROPERTY(QString cardBackground READ cardBackground NOTIFY themeChanged)
    
    /** @brief Color de borde perimetral sutil para tarjetas. */
    Q_PROPERTY(QString cardBorder READ cardBorder NOTIFY themeChanged)
    
    /** @brief Color de fondo para elementos interactivos al pasar el cursor (hover). */
    Q_PROPERTY(QString hoverBackground READ hoverBackground NOTIFY themeChanged)
    
    /** @brief Color de superficie para sub-islas y contenedores anidados. */
    Q_PROPERTY(QString surfaceElevated READ surfaceElevated NOTIFY themeChanged)
    
    /** @brief Color de fondo para botones según kdeglobals. */
    Q_PROPERTY(QString buttonBgColor READ buttonBgColor NOTIFY themeChanged)
    
    /** @brief Color de texto para botones según kdeglobals. */
    Q_PROPERTY(QString buttonTextColor READ buttonTextColor NOTIFY themeChanged)
    
    /** @brief Familia tipográfica general del sistema KDE Plasma. */
    Q_PROPERTY(QString generalFontFamily READ generalFontFamily NOTIFY themeChanged)
    
    /** @brief Familia tipográfica de ancho fijo (monospace) del sistema KDE Plasma. */
    Q_PROPERTY(QString fixedFontFamily READ fixedFontFamily NOTIFY themeChanged)
    
    /** @brief Color semántico para estados positivos o de éxito (PositiveText). */
    Q_PROPERTY(QString positiveTextColor READ positiveTextColor NOTIFY themeChanged)
    
    /** @brief Color semántico para advertencias y errores (NegativeText). */
    Q_PROPERTY(QString negativeTextColor READ negativeTextColor NOTIFY themeChanged)
    
    /** @brief Color principal de texto. */
    Q_PROPERTY(QString textColor READ textColor NOTIFY themeChanged)
    
    /** @brief Color secundario atenuado para etiquetas y subtítulos. */
    Q_PROPERTY(QString subTextColor READ subTextColor NOTIFY themeChanged)
    
    /** @brief Color de realce o acento de la sesión Plasma (Highlight). */
    Q_PROPERTY(QString highlightColor READ highlightColor NOTIFY themeChanged)

public:
    /**
     * @brief Constructor principal. Inicializa watchers de temas y timers de sondeo de hardware.
     * @param parent Puntero opcional al objeto padre Qt.
     */
    explicit SystemStats(QObject *parent = nullptr);

    /** @return Uso de CPU en porcentaje. */
    int cpuUsage() const { return m_cpuUsage; }
    
    /** @return Uso de RAM en porcentaje. */
    int ramUsage() const { return m_ramUsage; }
    
    /** @return RAM usada en formato legible. */
    QString ramUsedString() const { return m_ramUsedString; }
    
    /** @return RAM total en formato legible. */
    QString ramTotalString() const { return m_ramTotalString; }
    
    /** @return Nombre del sistema operativo. */
    QString osName() const { return m_osName; }
    
    /** @return Icono de la distribución. */
    QString distroIcon() const { return m_distroIcon; }
    
    /** @return Versión de kernel. */
    QString kernelVersion() const { return m_kernelVersion; }
    
    /** @return Tiempo de encendido. */
    QString uptimeString() const { return m_uptimeString; }
    
    /** @return Nombre del compositor. */
    QString compositor() const { return m_compositor; }
    
    /** @return Modelo de la CPU. */
    QString cpuModel() const { return m_cpuModel; }
    
    /** @return Nombre comercial simplificado del procesador. */
    QString cpuBrandName() const { return m_cpuBrandName; }
    
    /** @return Identificador de marca del fabricante ('intel', 'amd', 'qualcomm', 'apple', 'unknown'). */
    QString cpuVendor() const { return m_cpuVendor; }
    
    /** @return Color temático del fabricante. */
    QString cpuVendorColor() const { return m_cpuVendorColor; }
    
    /** @return Nombre de usuario. */
    QString userName() const { return m_userName; }
    
    /** @return Ruta a la foto de perfil. */
    QString userFace() const { return m_userFace; }
    
    /** @return Carga de batería (%). */
    int batteryUsage() const { return m_batteryUsage; }
    
    /** @return true si tiene batería. */
    bool hasBattery() const { return m_hasBattery; }
    
    /** @return true si está cargando. */
    bool isCharging() const { return m_isCharging; }

    /** @return true si el tema de Plasma es oscuro. */
    bool isDarkTheme() const { return m_isDarkTheme; }
    
    /** @return Color de fondo de ventana. */
    QString windowBgColor() const { return m_windowBgColor; }
    
    /** @return Color de vista. */
    QString viewBgColor() const { return m_viewBgColor; }
    
    /** @return Color de tarjeta. */
    QString cardBackground() const { return m_cardBackground; }
    
    /** @return Color de borde de tarjeta. */
    QString cardBorder() const { return m_cardBorder; }
    
    /** @return Color de hover. */
    QString hoverBackground() const { return m_hoverBackground; }
    
    /** @return Color de superficie elevada. */
    QString surfaceElevated() const { return m_surfaceElevated; }
    
    /** @return Color de botones de KDE. */
    QString buttonBgColor() const { return m_buttonBgColor; }
    
    /** @return Color de texto de botones. */
    QString buttonTextColor() const { return m_buttonTextColor; }
    
    /** @return Tipografía general del sistema. */
    QString generalFontFamily() const { return m_generalFontFamily; }
    
    /** @return Tipografía monospace del sistema. */
    QString fixedFontFamily() const { return m_fixedFontFamily; }
    
    /** @return Color semántico de éxito. */
    QString positiveTextColor() const { return m_positiveTextColor; }
    
    /** @return Color semántico de alerta/error. */
    QString negativeTextColor() const { return m_negativeTextColor; }
    
    /** @return Color de texto primario. */
    QString textColor() const { return m_textColor; }
    
    /** @return Color de texto secundario. */
    QString subTextColor() const { return m_subTextColor; }
    
    /** @return Color de acento Plasma. */
    QString highlightColor() const { return m_highlightColor; }

    /** @brief Fuerza la lectura inmediata de métricas de hardware y esquema de colores. */
    Q_INVOKABLE void refresh();

    /** @return true si el monitor está activo. */
    bool active() const { return m_active; }
    
    /** @brief Modifica el estado de actividad del monitor. */
    void setActive(bool active);

signals:
    /** @brief Emitida al cambiar la bandera de actividad. */
    void activeChanged();
    
    /** @brief Emitida cuando se actualizan los valores de telemetría de hardware. */
    void statsChanged();
    
    /** @brief Emitida cuando el usuario cambia el esquema de colores en Preferencias del Sistema. */
    void themeChanged();

private slots:
    /** @brief Lee las métricas dinámicas de Linux y emite statsChanged(). */
    void updateStats();
    
    /** @brief Relee kdeglobals y actualiza la paleta reactiva. */
    void updateTheme();

private:
    bool m_active = true;                      ///< Estado de actividad.
    QTimer *m_timer = nullptr;                 ///< Temporizador de sondeo (intervalo de 2s).
    QFileSystemWatcher *m_themeWatcher = nullptr; ///< Vigilante de archivo para ~/.config/kdeglobals.
    
    void loadStaticInfo();                     ///< Carga información estática del sistema (OS, Kernel, CPU).
    void readCpuStats();                       ///< Parsea /proc/stat para calcular el delta jiffies de CPU.
    void readRamStats();                       ///< Parsea /proc/meminfo para calcular MemTotal y MemAvailable.
    void readBatteryStats();                   ///< Parsea /sys/class/power_supply/ para nivel y estado.
    void readUptime();                         ///< Parsea /proc/uptime y genera la cadena amigable.
    void readKdeGlobalsTheme();                ///< Parsea los bloques [Colors:Window] y [Colors:View] de KDE.

    int m_cpuUsage = 0;                        ///< Uso de CPU en porcentaje.
    unsigned long long m_prevIdle = 0;         ///< Jiffies de inactividad previos.
    unsigned long long m_prevTotal = 0;        ///< Jiffies totales previos.
    int m_ramUsage = 0;                        ///< Porcentaje de RAM ocupada.
    QString m_ramUsedString;                   ///< Texto de RAM usada.
    QString m_ramTotalString;                  ///< Texto de RAM total.
    QString m_osName;                          ///< Nombre de la distribución.
    QString m_distroIcon;                      ///< Nombre del icono.
    QString m_kernelVersion;                   ///< Versión del kernel.
    QString m_uptimeString;                    ///< Uptime formateado.
    QString m_compositor;                      ///< Compositor.
    QString m_cpuModel;                        ///< Modelo de CPU sin procesar.
    QString m_cpuBrandName;                    ///< Modelo de CPU comercial simplificado (ej. 'AMD Ryzen 5 7530U').
    QString m_cpuVendor;                       ///< Fabricante del procesador ('intel', 'amd', etc.).
    QString m_cpuVendorColor;                  ///< Color corporativo distintivo de la marca de CPU.
    QString m_userName;                        ///< Usuario.
    QString m_userFace;                        ///< Foto de perfil.
    int m_batteryUsage = 0;                    ///< Nivel de batería.
    bool m_hasBattery = false;                 ///< Presencia de batería.
    bool m_isCharging = false;                 ///< Estado de carga.

    bool m_isDarkTheme = true;                 ///< Esquema oscuro activo.
    QString m_windowBgColor = QStringLiteral("#15181d");
    QString m_viewBgColor = QStringLiteral("#0d1117");
    QString m_cardBackground = QStringLiteral("#161922");
    QString m_cardBorder = QStringLiteral("rgba(255, 255, 255, 0.09)");
    QString m_hoverBackground = QStringLiteral("rgba(255, 255, 255, 0.12)");
    QString m_surfaceElevated = QStringLiteral("rgba(255, 255, 255, 0.07)");
    QString m_buttonBgColor = QStringLiteral("#1e232d");
    QString m_buttonTextColor = QStringLiteral("#ffffff");
    QString m_generalFontFamily = QStringLiteral("Noto Sans");
    QString m_fixedFontFamily = QStringLiteral("Monospace");
    QString m_positiveTextColor = QStringLiteral("#2ECC71");
    QString m_negativeTextColor = QStringLiteral("#E74C3C");
    QString m_textColor = QStringLiteral("#ffffff");
    QString m_subTextColor = QStringLiteral("rgba(255, 255, 255, 0.65)");
    QString m_highlightColor = QStringLiteral("#00c8d2");
};

#endif // SYSTEMSTATS_H