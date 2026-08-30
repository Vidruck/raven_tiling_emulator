#ifndef SYSTEMSTATS_H
#define SYSTEMSTATS_H

#include <QObject>
#include <QString>
#include <QTimer>
#include <QFileSystemWatcher>
#include <qqmlintegration.h>

class SystemStats : public QObject
{
    Q_OBJECT
    QML_SINGLETON
    QML_ELEMENT

    Q_PROPERTY(int cpuUsage READ cpuUsage NOTIFY statsChanged)
    Q_PROPERTY(int ramUsage READ ramUsage NOTIFY statsChanged)
    Q_PROPERTY(QString ramUsedString READ ramUsedString NOTIFY statsChanged)
    Q_PROPERTY(QString ramTotalString READ ramTotalString NOTIFY statsChanged)
    Q_PROPERTY(QString osName READ osName CONSTANT)
    Q_PROPERTY(QString distroIcon READ distroIcon CONSTANT)
    Q_PROPERTY(QString kernelVersion READ kernelVersion CONSTANT)
    Q_PROPERTY(QString uptimeString READ uptimeString NOTIFY statsChanged)
    Q_PROPERTY(QString compositor READ compositor CONSTANT)
    Q_PROPERTY(QString cpuModel READ cpuModel CONSTANT)
    Q_PROPERTY(QString userName READ userName CONSTANT)
    Q_PROPERTY(QString userFace READ userFace CONSTANT)
    Q_PROPERTY(int batteryUsage READ batteryUsage NOTIFY statsChanged)
    Q_PROPERTY(bool hasBattery READ hasBattery NOTIFY statsChanged)
    Q_PROPERTY(bool isCharging READ isCharging NOTIFY statsChanged)

    Q_PROPERTY(bool active READ active WRITE setActive NOTIFY activeChanged)
    Q_PROPERTY(bool isDarkTheme READ isDarkTheme NOTIFY themeChanged)
    Q_PROPERTY(QString windowBgColor READ windowBgColor NOTIFY themeChanged)
    Q_PROPERTY(QString viewBgColor READ viewBgColor NOTIFY themeChanged)
    Q_PROPERTY(QString cardBackground READ cardBackground NOTIFY themeChanged)
    Q_PROPERTY(QString cardBorder READ cardBorder NOTIFY themeChanged)
    Q_PROPERTY(QString hoverBackground READ hoverBackground NOTIFY themeChanged)
    Q_PROPERTY(QString surfaceElevated READ surfaceElevated NOTIFY themeChanged)
    Q_PROPERTY(QString textColor READ textColor NOTIFY themeChanged)
    Q_PROPERTY(QString subTextColor READ subTextColor NOTIFY themeChanged)
    Q_PROPERTY(QString highlightColor READ highlightColor NOTIFY themeChanged)

public:
    explicit SystemStats(QObject *parent = nullptr);

    int cpuUsage() const { return m_cpuUsage; }
    int ramUsage() const { return m_ramUsage; }
    QString ramUsedString() const { return m_ramUsedString; }
    QString ramTotalString() const { return m_ramTotalString; }
    QString osName() const { return m_osName; }
    QString distroIcon() const { return m_distroIcon; }
    QString kernelVersion() const { return m_kernelVersion; }
    QString uptimeString() const { return m_uptimeString; }
    QString compositor() const { return m_compositor; }
    QString cpuModel() const { return m_cpuModel; }
    QString userName() const { return m_userName; }
    QString userFace() const { return m_userFace; }
    int batteryUsage() const { return m_batteryUsage; }
    bool hasBattery() const { return m_hasBattery; }
    bool isCharging() const { return m_isCharging; }

    bool isDarkTheme() const { return m_isDarkTheme; }
    QString windowBgColor() const { return m_windowBgColor; }
    QString viewBgColor() const { return m_viewBgColor; }
    QString cardBackground() const { return m_cardBackground; }
    QString cardBorder() const { return m_cardBorder; }
    QString hoverBackground() const { return m_hoverBackground; }
    QString surfaceElevated() const { return m_surfaceElevated; }
    QString textColor() const { return m_textColor; }
    QString subTextColor() const { return m_subTextColor; }
    QString highlightColor() const { return m_highlightColor; }

    Q_INVOKABLE void refresh();

    bool active() const { return m_active; }
    void setActive(bool active);

signals:
    void activeChanged();
    void statsChanged();
    void themeChanged();

private slots:
    void updateStats();
    void updateTheme();

private:
    bool m_active = true;
    QTimer *m_timer = nullptr;
    QFileSystemWatcher *m_themeWatcher = nullptr;
    void loadStaticInfo();
    void readCpuStats();
    void readRamStats();
    void readBatteryStats();
    void readUptime();
    void readKdeGlobalsTheme();

    int m_cpuUsage = 0;
    unsigned long long m_prevIdle = 0;
    unsigned long long m_prevTotal = 0;
    int m_ramUsage = 0;
    QString m_ramUsedString;
    QString m_ramTotalString;
    QString m_osName;
    QString m_distroIcon;
    QString m_kernelVersion;
    QString m_uptimeString;
    QString m_compositor;
    QString m_cpuModel;
    QString m_userName;
    QString m_userFace;

    int m_batteryUsage = 0;
    bool m_hasBattery = false;
    bool m_isCharging = false;

    bool m_isDarkTheme = true;
    QString m_windowBgColor = QStringLiteral("#15181d");
    QString m_viewBgColor = QStringLiteral("#0d1117");
    QString m_cardBackground = QStringLiteral("#161922");
    QString m_cardBorder = QStringLiteral("rgba(255, 255, 255, 0.09)");
    QString m_hoverBackground = QStringLiteral("rgba(255, 255, 255, 0.12)");
    QString m_surfaceElevated = QStringLiteral("rgba(255, 255, 255, 0.07)");
    QString m_textColor = QStringLiteral("#ffffff");
    QString m_subTextColor = QStringLiteral("rgba(255, 255, 255, 0.65)");
    QString m_highlightColor = QStringLiteral("#00c8d2");
};

#endif // SYSTEMSTATS_H