#include "ravencontroller.h"
#include <QProcess>
#include <QDir>
#include <QFile>
#include <QDebug>
#include <QJsonDocument>
#include <QJsonObject>
#include <QDBusMessage>
#include <QDBusConnection>

RavenController::RavenController(QObject *parent)
    : QObject(parent)
{
    m_dbusInterface = new QDBusInterface(
        QStringLiteral("org.kde.raven.Daemon"),
        QStringLiteral("/Events"),
        QStringLiteral("org.kde.raven.Events"),
        QDBusConnection::sessionBus(),
        this
    );

    m_pollTimer = new QTimer(this);
    m_pollTimer->setInterval(1500);
    connect(m_pollTimer, &QTimer::timeout, this, &RavenController::refreshState);
    m_pollTimer->start();

    refreshState();
}

void RavenController::refreshState()
{
    if (!m_dbusInterface || !m_dbusInterface->isValid()) {
        return;
    }

    QDBusMessage msgTiling = m_dbusInterface->call(QStringLiteral("getTilingState"));
    if (msgTiling.type() == QDBusMessage::ReplyMessage && !msgTiling.arguments().isEmpty()) {
        bool state = msgTiling.arguments().first().toBool();
        if (m_tilingEnabled != state) {
            m_tilingEnabled = state;
            Q_EMIT tilingEnabledChanged();
        }
    }

    QDBusMessage msgMonitors = m_dbusInterface->call(QStringLiteral("getMonitorCount"));
    if (msgMonitors.type() == QDBusMessage::ReplyMessage && !msgMonitors.arguments().isEmpty()) {
        int count = msgMonitors.arguments().first().toInt();
        if (m_monitorCount != count) {
            m_monitorCount = count;
            Q_EMIT monitorCountChanged();
        }
    }

    QDBusMessage msgDesktops = m_dbusInterface->call(QStringLiteral("getDesktopStatus"));
    if (msgDesktops.type() == QDBusMessage::ReplyMessage && !msgDesktops.arguments().isEmpty()) {
        QString rawStatus = msgDesktops.arguments().first().toString().trimmed();
        // Formato: "prev | Escritorio cur | next"
        QStringList parts = rawStatus.split(QLatin1Char('|'));
        if (parts.size() >= 3) {
            int prev = parts[0].trimmed().toInt();
            int next = parts[2].trimmed().toInt();
            QString curStr = parts[1].trimmed();
            int cur = curStr.split(QLatin1Char(' ')).last().toInt();

            if (m_currentDesktop != cur || m_prevDesktop != prev || m_nextDesktop != next || m_desktopStatus != curStr) {
                m_currentDesktop = cur > 0 ? cur : 1;
                m_prevDesktop = prev > 0 ? prev : 1;
                m_nextDesktop = next > 0 ? next : 1;
                m_desktopStatus = curStr.isEmpty() ? QStringLiteral("Escritorio 1") : curStr;
                Q_EMIT desktopStatusChanged();
            }
        }
    }
}

void RavenController::sendDbusAction(const QString &action)
{
    if (m_dbusInterface && m_dbusInterface->isValid()) {
        m_dbusInterface->call(QDBus::NoBlock, action);
    } else {
        // Fallback vía qdbus CLI si la interfaz directa aún no responde
        QProcess::startDetached(QStringLiteral("qdbus"), {
            QStringLiteral("org.kde.raven.Daemon"),
            QStringLiteral("/Events"),
            QStringLiteral("org.kde.raven.Events.%1").arg(action)
        });
    }
}

void RavenController::sendDbusActionWithArg(const QString &action, int arg)
{
    if (m_dbusInterface && m_dbusInterface->isValid()) {
        m_dbusInterface->call(QDBus::NoBlock, action, arg);
    } else {
        QProcess::startDetached(QStringLiteral("qdbus"), {
            QStringLiteral("org.kde.raven.Daemon"),
            QStringLiteral("/Events"),
            QStringLiteral("org.kde.raven.Events.%1").arg(action),
            QString::number(arg)
        });
    }
}

void RavenController::toggleTiling()
{
    m_tilingEnabled = !m_tilingEnabled;
    Q_EMIT tilingEnabledChanged();
    sendDbusAction(QStringLiteral("toggleTiling"));
}

void RavenController::setTilingEnabled(bool enabled)
{
    if (m_tilingEnabled != enabled) {
        toggleTiling();
    }
}

void RavenController::cycleLayout()
{
    sendDbusAction(QStringLiteral("cycleLayout"));
}

void RavenController::setLayout(const QString &layoutName)
{
    if (m_currentLayout != layoutName) {
        m_currentLayout = layoutName;
        Q_EMIT currentLayoutChanged();
    }
    
    if (m_dbusInterface && m_dbusInterface->isValid()) {
        m_dbusInterface->call(QDBus::NoBlock, QStringLiteral("setLayoutForCurrentWorkspace"), layoutName);
    } else {
        QProcess::startDetached(QStringLiteral("qdbus"), {
            QStringLiteral("org.kde.raven.Daemon"),
            QStringLiteral("/Events"),
            QStringLiteral("org.kde.raven.Events.setLayoutForCurrentWorkspace"),
            layoutName
        });
    }
}

void RavenController::toggleFloating()
{
    if (m_dbusInterface && m_dbusInterface->isValid()) {
        m_dbusInterface->call(QDBus::NoBlock, QStringLiteral("toggleFloating"), QString());
    } else {
        QProcess::startDetached(QStringLiteral("qdbus"), {
            QStringLiteral("org.kde.raven.Daemon"),
            QStringLiteral("/Events"),
            QStringLiteral("org.kde.raven.Events.toggleFloating"),
            QStringLiteral("")
        });
    }
}

void RavenController::incrementGaps(int delta)
{
    m_defaultGaps = qMax(0, m_defaultGaps + delta);
    Q_EMIT defaultGapsChanged();
    sendDbusActionWithArg(QStringLiteral("incrementGaps"), delta);
}

void RavenController::incrementMaster()
{
    sendDbusAction(QStringLiteral("incrementMaster"));
}

void RavenController::decrementMaster()
{
    sendDbusAction(QStringLiteral("decrementMaster"));
}

void RavenController::increaseRatio()
{
    m_masterRatio = qMin(0.85, m_masterRatio + 0.05);
    Q_EMIT masterRatioChanged();
    sendDbusAction(QStringLiteral("increaseRatio"));
}

void RavenController::decreaseRatio()
{
    m_masterRatio = qMax(0.15, m_masterRatio - 0.05);
    Q_EMIT masterRatioChanged();
    sendDbusAction(QStringLiteral("decreaseRatio"));
}

void RavenController::swapPrev()
{
    sendDbusAction(QStringLiteral("swapPrev"));
}

void RavenController::swapNext()
{
    sendDbusAction(QStringLiteral("swapNext"));
}

void RavenController::focusPrev()
{
    sendDbusAction(QStringLiteral("focusPrev"));
}

void RavenController::focusNext()
{
    sendDbusAction(QStringLiteral("focusNext"));
}

void RavenController::migrateActiveToScreen()
{
    sendDbusAction(QStringLiteral("migrateActiveToScreen"));
}

void RavenController::migrateActiveToPrevScreen()
{
    sendDbusAction(QStringLiteral("migrateActiveToPrevScreen"));
}

void RavenController::migrateActiveToDesktop()
{
    sendDbusAction(QStringLiteral("migrateActiveToDesktop"));
}

void RavenController::migrateActiveToPrevDesktop()
{
    sendDbusAction(QStringLiteral("migrateActiveToPrevDesktop"));
}

void RavenController::openControlCenter()
{
    QString homePath = QDir::homePath();
    QString localBinary = homePath + QStringLiteral("/.local/share/raven/bin/raven_gui");

    if (QFile::exists(localBinary)) {
        QProcess::startDetached(localBinary, QStringList());
    } else {
        QProcess::startDetached(QStringLiteral("raven_gui"), QStringList());
    }
}
