#include "systemstats.h"
#include <QGuiApplication>
#include <QPalette>
#include <QFile>
#include <QTextStream>
#include <QProcessEnvironment>
#include <QStandardPaths>
#include <QDir>
#include <QUrl>
#include <QIcon>
#include <sys/utsname.h>

SystemStats::SystemStats(QObject *parent)
    : QObject(parent)
{
    loadStaticInfo();
    readKdeGlobalsTheme();
    updateStats();

    m_themeWatcher = new QFileSystemWatcher(this);
    QString configPath = QDir::homePath() + QStringLiteral("/.config/kdeglobals");
    if (QFile::exists(configPath)) {
        m_themeWatcher->addPath(configPath);
    }
    connect(m_themeWatcher, &QFileSystemWatcher::fileChanged, this, [this](const QString &path) {
        readKdeGlobalsTheme();
        if (m_themeWatcher && !m_themeWatcher->files().contains(path) && QFile::exists(path)) {
            m_themeWatcher->addPath(path);
        }
    });

    m_timer = new QTimer(this);
    m_timer->setInterval(2000);
    connect(m_timer, &QTimer::timeout, this, &SystemStats::updateStats);
    if (m_active) {
        m_timer->start();
    }
}

void SystemStats::setActive(bool active)
{
    if (m_active == active) return;
    m_active = active;
    
    if (m_active) {
        updateStats(); // Force an immediate update
        m_timer->start();
    } else {
        m_timer->stop();
    }
    
    emit activeChanged();
}

void SystemStats::loadStaticInfo()
{
    // 1. OS Name & Distro Icon
    m_osName = QStringLiteral("Linux");
    m_distroIcon = QStringLiteral("start-here-kde");
    QString distroId;
    QString distroLogo;

    QFile osRelease(QStringLiteral("/etc/os-release"));
    if (osRelease.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QTextStream in(&osRelease);
        while (!in.atEnd()) {
            QString line = in.readLine().trimmed();
            if (line.startsWith(QLatin1String("PRETTY_NAME="))) {
                QString name = line.mid(12).trimmed();
                if (name.startsWith(QLatin1Char('"')) && name.endsWith(QLatin1Char('"')) && name.length() >= 2) {
                    name = name.mid(1, name.length() - 2);
                }
                if (!name.isEmpty()) {
                    m_osName = name;
                }
            } else if (line.startsWith(QLatin1String("NAME=")) && m_osName == QStringLiteral("Linux")) {
                QString name = line.mid(5).trimmed();
                if (name.startsWith(QLatin1Char('"')) && name.endsWith(QLatin1Char('"')) && name.length() >= 2) {
                    name = name.mid(1, name.length() - 2);
                }
                if (!name.isEmpty()) {
                    m_osName = name;
                }
            } else if (line.startsWith(QLatin1String("LOGO="))) {
                distroLogo = line.mid(5).trimmed();
                if (distroLogo.startsWith(QLatin1Char('"')) && distroLogo.endsWith(QLatin1Char('"')) && distroLogo.length() >= 2) {
                    distroLogo = distroLogo.mid(1, distroLogo.length() - 2);
                }
            } else if (line.startsWith(QLatin1String("ID="))) {
                distroId = line.mid(3).trimmed();
                if (distroId.startsWith(QLatin1Char('"')) && distroId.endsWith(QLatin1Char('"')) && distroId.length() >= 2) {
                    distroId = distroId.mid(1, distroId.length() - 2);
                }
            }
        }
        osRelease.close();
    }

    // Resolver icono prioritario: LOGO -> distro-<ID> / start-here-<ID> -> start-here-kde -> kde -> plasma
    if (!distroLogo.isEmpty() && QIcon::hasThemeIcon(distroLogo)) {
        m_distroIcon = distroLogo;
    } else if (!distroId.isEmpty()) {
        QStringList candidates = {
            QStringLiteral("distro-%1").arg(distroId),
            QStringLiteral("start-here-%1").arg(distroId),
            distroId,
            QStringLiteral("start-here-kde"),
            QStringLiteral("kde"),
            QStringLiteral("plasma")
        };
        for (const QString &cand : candidates) {
            if (QIcon::hasThemeIcon(cand)) {
                m_distroIcon = cand;
                break;
            }
        }
    } else if (QIcon::hasThemeIcon(QStringLiteral("start-here-kde"))) {
        m_distroIcon = QStringLiteral("start-here-kde");
    } else if (QIcon::hasThemeIcon(QStringLiteral("kde"))) {
        m_distroIcon = QStringLiteral("kde");
    } else {
        m_distroIcon = QStringLiteral("plasma");
    }

    // 2. Kernel Version
    struct utsname buf;
    if (uname(&buf) == 0) {
        m_kernelVersion = QString::fromLatin1(buf.release);
    } else {
        m_kernelVersion = QStringLiteral("Linux");
    }

    // 3. Compositor & Session Type
    QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    QString sessionType = env.value(QStringLiteral("XDG_SESSION_TYPE"));
    if (sessionType.isEmpty()) {
        sessionType = env.contains(QStringLiteral("WAYLAND_DISPLAY")) ? QStringLiteral("wayland") : QStringLiteral("x11");
    }
    sessionType = sessionType.left(1).toUpper() + sessionType.mid(1).toLower();

    QString desktop = env.value(QStringLiteral("XDG_CURRENT_DESKTOP"));
    if (desktop.contains(QLatin1String("KDE"), Qt::CaseInsensitive)) {
        m_compositor = QStringLiteral("KWin (%1)").arg(sessionType);
    } else {
        m_compositor = QStringLiteral("%1 (%2)").arg(desktop.isEmpty() ? QStringLiteral("Desktop") : desktop, sessionType);
    }

    // 4. CPU Model
    QFile cpuInfo(QStringLiteral("/proc/cpuinfo"));
    if (cpuInfo.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QTextStream in(&cpuInfo);
        while (!in.atEnd()) {
            QString line = in.readLine().trimmed();
            if (line.startsWith(QLatin1String("model name"))) {
                int colonPos = line.indexOf(QLatin1Char(':'));
                if (colonPos != -1) {
                    m_cpuModel = line.mid(colonPos + 1).trimmed();
                    break;
                }
            }
        }
        cpuInfo.close();
    }

    // 5. User Name & Avatar
    m_userName = qEnvironmentVariable("USER");
    if (m_userName.isEmpty()) {
        m_userName = QDir::home().dirName();
    }
    
    QString facePath = QDir::homePath() + QStringLiteral("/.face.icon");
    if (!QFile::exists(facePath)) {
        facePath = QDir::homePath() + QStringLiteral("/.face");
        if (!QFile::exists(facePath)) {
            facePath = QStringLiteral("/var/lib/AccountsService/icons/") + m_userName;
        }
    }
    if (QFile::exists(facePath)) {
        m_userFace = QUrl::fromLocalFile(facePath).toString();
    } else {
        m_userFace = QString();
    }
}

void SystemStats::readKdeGlobalsTheme()
{
    // Raven Dark Base Defaults
    m_windowBgColor = QStringLiteral("#0f131a");
    m_viewBgColor = QStringLiteral("#151922");
    m_cardBackground = QStringLiteral("#181c26");
    m_cardBorder = QStringLiteral("rgba(255, 255, 255, 0.09)");
    m_hoverBackground = QStringLiteral("rgba(255, 255, 255, 0.12)");
    m_surfaceElevated = QStringLiteral("rgba(255, 255, 255, 0.06)");
    m_textColor = QStringLiteral("#ffffff");
    m_subTextColor = QStringLiteral("rgba(255, 255, 255, 0.65)");
    m_highlightColor = QStringLiteral("#00c8d2");
    m_isDarkTheme = true;

    int winR = 15, winG = 19, winB = 26;
    int viewR = 21, viewG = 25, viewB = 34;

    // Read explicit values from ~/.config/kdeglobals if present
    QString configPath = QDir::homePath() + QStringLiteral("/.config/kdeglobals");
    QFile file(configPath);
    if (file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QTextStream in(&file);
        QString currentGroup;
        while (!in.atEnd()) {
            QString line = in.readLine().trimmed();
            if (line.startsWith(QLatin1Char('[')) && line.endsWith(QLatin1Char(']'))) {
                currentGroup = line.mid(1, line.length() - 2);
                continue;
            }

            int eq = line.indexOf(QLatin1Char('='));
            if (eq <= 0) continue;
            QString key = line.left(eq).trimmed();
            QString val = line.mid(eq + 1).trimmed();

            if (currentGroup == QLatin1String("Colors:Window")) {
                if (key == QLatin1String("BackgroundNormal")) {
                    QStringList rgb = val.split(QLatin1Char(','));
                    if (rgb.size() >= 3) {
                        winR = rgb[0].toInt(); winG = rgb[1].toInt(); winB = rgb[2].toInt();
                        m_windowBgColor = QStringLiteral("#%1%2%3")
                                          .arg(winR, 2, 16, QLatin1Char('0'))
                                          .arg(winG, 2, 16, QLatin1Char('0'))
                                          .arg(winB, 2, 16, QLatin1Char('0'));
                    }
                } else if (key == QLatin1String("ForegroundNormal")) {
                    QStringList rgb = val.split(QLatin1Char(','));
                    if (rgb.size() >= 3) {
                        int tr = rgb[0].toInt(), tg = rgb[1].toInt(), tb = rgb[2].toInt();
                        m_textColor = QStringLiteral("#%1%2%3")
                                      .arg(tr, 2, 16, QLatin1Char('0'))
                                      .arg(tg, 2, 16, QLatin1Char('0'))
                                      .arg(tb, 2, 16, QLatin1Char('0'));
                    }
                } else if (key == QLatin1String("ForegroundPositive") || key == QLatin1String("PositiveText")) {
                    QStringList rgb = val.split(QLatin1Char(','));
                    if (rgb.size() >= 3) {
                        m_positiveTextColor = QStringLiteral("#%1%2%3")
                                              .arg(rgb[0].toInt(), 2, 16, QLatin1Char('0'))
                                              .arg(rgb[1].toInt(), 2, 16, QLatin1Char('0'))
                                              .arg(rgb[2].toInt(), 2, 16, QLatin1Char('0'));
                    }
                } else if (key == QLatin1String("ForegroundNegative") || key == QLatin1String("NegativeText")) {
                    QStringList rgb = val.split(QLatin1Char(','));
                    if (rgb.size() >= 3) {
                        m_negativeTextColor = QStringLiteral("#%1%2%3")
                                              .arg(rgb[0].toInt(), 2, 16, QLatin1Char('0'))
                                              .arg(rgb[1].toInt(), 2, 16, QLatin1Char('0'))
                                              .arg(rgb[2].toInt(), 2, 16, QLatin1Char('0'));
                    }
                }
            } else if (currentGroup == QLatin1String("Colors:View")) {
                if (key == QLatin1String("BackgroundNormal")) {
                    QStringList rgb = val.split(QLatin1Char(','));
                    if (rgb.size() >= 3) {
                        viewR = rgb[0].toInt(); viewG = rgb[1].toInt(); viewB = rgb[2].toInt();
                        m_viewBgColor = QStringLiteral("#%1%2%3")
                                        .arg(viewR, 2, 16, QLatin1Char('0'))
                                        .arg(viewG, 2, 16, QLatin1Char('0'))
                                        .arg(viewB, 2, 16, QLatin1Char('0'));
                    }
                }
            } else if (currentGroup == QLatin1String("Colors:Button")) {
                if (key == QLatin1String("BackgroundNormal")) {
                    QStringList rgb = val.split(QLatin1Char(','));
                    if (rgb.size() >= 3) {
                        m_buttonBgColor = QStringLiteral("#%1%2%3")
                                          .arg(rgb[0].toInt(), 2, 16, QLatin1Char('0'))
                                          .arg(rgb[1].toInt(), 2, 16, QLatin1Char('0'))
                                          .arg(rgb[2].toInt(), 2, 16, QLatin1Char('0'));
                    }
                } else if (key == QLatin1String("ForegroundNormal")) {
                    QStringList rgb = val.split(QLatin1Char(','));
                    if (rgb.size() >= 3) {
                        m_buttonTextColor = QStringLiteral("#%1%2%3")
                                            .arg(rgb[0].toInt(), 2, 16, QLatin1Char('0'))
                                            .arg(rgb[1].toInt(), 2, 16, QLatin1Char('0'))
                                            .arg(rgb[2].toInt(), 2, 16, QLatin1Char('0'));
                    }
                }
            } else if (currentGroup == QLatin1String("Colors:Selection")) {
                if (key == QLatin1String("BackgroundNormal")) {
                    QStringList rgb = val.split(QLatin1Char(','));
                    if (rgb.size() >= 3) {
                        int hR = rgb[0].toInt(), hG = rgb[1].toInt(), hB = rgb[2].toInt();
                        m_highlightColor = QStringLiteral("#%1%2%3")
                                           .arg(hR, 2, 16, QLatin1Char('0'))
                                           .arg(hG, 2, 16, QLatin1Char('0'))
                                           .arg(hB, 2, 16, QLatin1Char('0'));
                    }
                }
            } else if (currentGroup == QLatin1String("General")) {
                if (key == QLatin1String("font")) {
                    QStringList fParts = val.split(QLatin1Char(','));
                    if (!fParts.isEmpty() && !fParts.first().trimmed().isEmpty()) {
                        m_generalFontFamily = fParts.first().trimmed();
                    }
                } else if (key == QLatin1String("fixed")) {
                    QStringList fParts = val.split(QLatin1Char(','));
                    if (!fParts.isEmpty() && !fParts.first().trimmed().isEmpty()) {
                        m_fixedFontFamily = fParts.first().trimmed();
                    }
                }
            }
        }
        file.close();
    }

    // Determinar Dark vs Light mediante luminancia ITU-R BT.601
    double luminance = (0.299 * winR + 0.587 * winG + 0.114 * winB);
    m_isDarkTheme = (luminance < 140.0);

    if (m_isDarkTheme) {
        // En temas oscuros: asegurar tipografía clara de alto contraste absoluto
        m_textColor = QStringLiteral("#FFFFFF");
        m_subTextColor = QStringLiteral("#A0AEC0"); // Gris claro de alta legibilidad (Tailwind/Nord Slate)

        // Fondo de tarjeta con tono armónico adoptando directamente el fondo de las vistas de KDE
        m_cardBackground = m_viewBgColor;

        // Bordes finos y discretos (1px)
        m_cardBorder = QStringLiteral("rgba(255, 255, 255, 0.08)");
        m_hoverBackground = QStringLiteral("rgba(255, 255, 255, 0.12)");
        m_surfaceElevated = QStringLiteral("rgba(255, 255, 255, 0.06)");
    } else {
        // En temas claros:
        m_textColor = QStringLiteral("#111827");
        m_subTextColor = QStringLiteral("#4B5563");

        m_cardBackground = m_viewBgColor;

        m_cardBorder = QStringLiteral("rgba(0, 0, 0, 0.07)");
        m_hoverBackground = QStringLiteral("rgba(0, 0, 0, 0.06)");
        m_surfaceElevated = QStringLiteral("rgba(0, 0, 0, 0.03)");
    }

    emit themeChanged();
}

void SystemStats::readCpuStats()
{
    QFile file(QStringLiteral("/proc/stat"));
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) return;

    QString content = QString::fromUtf8(file.readAll());
    file.close();

    QString line = content.split(QLatin1Char('\n')).first().trimmed();
    if (!line.startsWith(QLatin1String("cpu "))) return;

    QStringList parts = line.split(QLatin1Char(' '), Qt::SkipEmptyParts);
    if (parts.size() < 5) return;

    unsigned long long user = parts.at(1).toULongLong();
    unsigned long long nice = parts.at(2).toULongLong();
    unsigned long long system = parts.at(3).toULongLong();
    unsigned long long idle = parts.at(4).toULongLong();
    unsigned long long iowait = parts.size() > 5 ? parts.at(5).toULongLong() : 0;
    unsigned long long irq = parts.size() > 6 ? parts.at(6).toULongLong() : 0;
    unsigned long long softirq = parts.size() > 7 ? parts.at(7).toULongLong() : 0;
    unsigned long long steal = parts.size() > 8 ? parts.at(8).toULongLong() : 0;

    unsigned long long totalIdle = idle + iowait;
    unsigned long long total = user + nice + system + totalIdle + irq + softirq + steal;

    if (m_prevTotal > 0 && total > m_prevTotal) {
        unsigned long long deltaTotal = total - m_prevTotal;
        unsigned long long deltaIdle = totalIdle - m_prevIdle;
        if (deltaTotal > 0 && deltaTotal >= deltaIdle) {
            double usage = (1.0 - (double(deltaIdle) / double(deltaTotal))) * 100.0;
            m_cpuUsage = qBound(0, int(usage + 0.5), 100);
        }
    }

    m_prevIdle = totalIdle;
    m_prevTotal = total;
}

void SystemStats::readRamStats()
{
    QFile file(QStringLiteral("/proc/meminfo"));
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) return;
    
    QString content = QString::fromUtf8(file.readAll());
    file.close();

    unsigned long long totalKb = 0;
    unsigned long long availableKb = 0;
    unsigned long long freeKb = 0;
    unsigned long long buffersKb = 0;
    unsigned long long cachedKb = 0;

    QStringList lines = content.split(QLatin1Char('\n'));
    for (const QString &rawLine : lines) {
        QString line = rawLine.trimmed();
        if (line.startsWith(QLatin1String("MemTotal:"))) {
            totalKb = line.split(QLatin1Char(' '), Qt::SkipEmptyParts).value(1).toULongLong();
        } else if (line.startsWith(QLatin1String("MemAvailable:"))) {
            availableKb = line.split(QLatin1Char(' '), Qt::SkipEmptyParts).value(1).toULongLong();
        } else if (line.startsWith(QLatin1String("MemFree:"))) {
            freeKb = line.split(QLatin1Char(' '), Qt::SkipEmptyParts).value(1).toULongLong();
        } else if (line.startsWith(QLatin1String("Buffers:"))) {
            buffersKb = line.split(QLatin1Char(' '), Qt::SkipEmptyParts).value(1).toULongLong();
        } else if (line.startsWith(QLatin1String("Cached:"))) {
            cachedKb = line.split(QLatin1Char(' '), Qt::SkipEmptyParts).value(1).toULongLong();
        }
    }

    if (totalKb == 0) return;

    if (availableKb == 0) {
        availableKb = freeKb + buffersKb + cachedKb;
    }

    unsigned long long usedKb = (totalKb > availableKb) ? (totalKb - availableKb) : 0;

    double usedGb = double(usedKb) / (1024.0 * 1024.0);
    double totalGb = double(totalKb) / (1024.0 * 1024.0);

    m_ramUsage = qBound(0, int((double(usedKb) / double(totalKb)) * 100.0 + 0.5), 100);
    m_ramUsedString = QString::number(usedGb, 'f', 1) + QLatin1String(" GB");
    m_ramTotalString = QString::number(totalGb, 'f', 1) + QLatin1String(" GB");
}

void SystemStats::readBatteryStats()
{
    QDir powerSupply(QStringLiteral("/sys/class/power_supply"));
    QStringList batteries = powerSupply.entryList(QStringList() << QStringLiteral("BAT*"), QDir::Dirs | QDir::NoDotAndDotDot);
    
    if (batteries.isEmpty()) {
        m_hasBattery = false;
        m_batteryUsage = 0;
        m_isCharging = false;
        return;
    }
    
    m_hasBattery = true;
    int totalCap = 0;
    int batCount = 0;
    bool isAnyCharging = false;

    for (const QString &batName : batteries) {
        QString batPath = powerSupply.absoluteFilePath(batName);
        
        // Read capacity
        QFile capacityFile(batPath + QStringLiteral("/capacity"));
        if (capacityFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
            bool ok = false;
            int cap = QString::fromUtf8(capacityFile.readAll()).trimmed().toInt(&ok);
            if (ok && cap >= 0) {
                totalCap += cap;
                batCount++;
            }
            capacityFile.close();
        }
        
        // Read status
        QFile statusFile(batPath + QStringLiteral("/status"));
        if (statusFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
            QString status = QString::fromUtf8(statusFile.readAll()).trimmed();
            if (status.compare(QLatin1String("Charging"), Qt::CaseInsensitive) == 0) {
                isAnyCharging = true;
            }
            statusFile.close();
        }
    }

    m_batteryUsage = (batCount > 0) ? qBound(0, totalCap / batCount, 100) : 0;
    m_isCharging = isAnyCharging;
}

void SystemStats::readUptime()
{
    QFile file(QStringLiteral("/proc/uptime"));
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) return;

    QString line = QString::fromUtf8(file.readAll());
    file.close();
    
    double uptimeSec = line.split(QLatin1Char(' ')).first().toDouble();

    long long totalSeconds = static_cast<long long>(uptimeSec);
    long long days = totalSeconds / 86400;
    long long hours = (totalSeconds % 86400) / 3600;
    long long minutes = (totalSeconds % 3600) / 60;

    if (days > 0) {
        m_uptimeString = QStringLiteral("%1d %2h").arg(days).arg(hours);
    } else if (hours > 0) {
        m_uptimeString = QStringLiteral("%1h %2m").arg(hours).arg(minutes);
    } else {
        m_uptimeString = QStringLiteral("%1 min").arg(minutes);
    }
}

void SystemStats::updateStats()
{
    readCpuStats();
    readRamStats();
    readBatteryStats();
    readUptime();
    emit statsChanged();
}

void SystemStats::updateTheme()
{
    readKdeGlobalsTheme();
}

void SystemStats::refresh()
{
    readKdeGlobalsTheme();
    updateStats();
}
