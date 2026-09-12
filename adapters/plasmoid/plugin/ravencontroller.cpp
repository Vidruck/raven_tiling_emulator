/**
 * @file ravencontroller.cpp
 * @brief Implementación de la interfaz de enlace C++/Qt para el control del motor Raven Tiling.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 * @license GPL-3.0
 */

#include "ravencontroller.h"
#include <QProcess>
#include <QDir>
#include <QFile>
#include <QDebug>
#include <QJsonDocument>
#include <QJsonObject>
#include <QDBusMessage>
#include <QDBusConnection>

/**
 * @brief Constructor de la clase RavenController.
 *
 * Establece la conexión D-Bus con el demonio de Raven Tiling e inicializa
 * un temporizador para refrescar el estado del sistema periódicamente.
 *
 * @param parent Objeto padre (opcional).
 */
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

/**
 * @brief Consulta y actualiza el estado del motor de Tiling.
 *
 * Llama de forma síncrona a los métodos del demonio para actualizar si el tiling está activo,
 * la cantidad de monitores y el estado de los escritorios virtuales.
 */
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

/**
 * @brief Invoca un atajo global de teclado registrado en KWin.
 *
 * @param shortcutName Nombre interno del atajo a invocar (ej. "RavenMigrateMonitor").
 */
void RavenController::invokeKWinShortcut(const QString &shortcutName)
{
    QDBusMessage msg = QDBusMessage::createMethodCall(
        QStringLiteral("org.kde.kglobalaccel"),
        QStringLiteral("/component/kwin"),
        QStringLiteral("org.kde.kglobalaccel.Component"),
        QStringLiteral("invokeShortcut")
    );
    msg << shortcutName;
    QDBusConnection::sessionBus().send(msg);
}

/**
 * @brief Envía una acción D-Bus sin parámetros al demonio de Raven.
 *
 * Si la interfaz directa no responde, recurre a la herramienta CLI `qdbus` como respaldo.
 *
 * @param action Nombre de la acción o método a invocar.
 */
void RavenController::sendDbusAction(const QString &action)
{
    if (m_dbusInterface && m_dbusInterface->isValid()) {
        QDBusMessage reply = m_dbusInterface->call(action);
        if (reply.type() == QDBusMessage::ReplyMessage && !reply.arguments().isEmpty()) {
            QString cmds = reply.arguments().first().toString();
            if (!cmds.isEmpty() && cmds != QStringLiteral("[]")) {
                QDBusConnection::sessionBus().send(
                    QDBusMessage::createMethodCall(
                        QStringLiteral("org.kde.raven.Daemon"),
                        QStringLiteral("/Events"),
                        QStringLiteral("org.kde.raven.Events"),
                        QStringLiteral("tilingCommandsPending")
                    ) << cmds
                );
            }
        }
    } else {
        // Fallback vía qdbus CLI si la interfaz directa aún no responde
        QProcess::startDetached(QStringLiteral("qdbus"), {
            QStringLiteral("org.kde.raven.Daemon"),
            QStringLiteral("/Events"),
            QStringLiteral("org.kde.raven.Events.%1").arg(action)
        });
    }
}

/**
 * @brief Envía una acción D-Bus con un argumento entero al demonio de Raven.
 *
 * @param action Nombre de la acción o método a invocar.
 * @param arg Argumento de tipo entero que acompaña la acción.
 */
void RavenController::sendDbusActionWithArg(const QString &action, int arg)
{
    if (m_dbusInterface && m_dbusInterface->isValid()) {
        QDBusMessage reply = m_dbusInterface->call(action, arg);
        if (reply.type() == QDBusMessage::ReplyMessage && !reply.arguments().isEmpty()) {
            QString cmds = reply.arguments().first().toString();
            if (!cmds.isEmpty() && cmds != QStringLiteral("[]")) {
                QDBusConnection::sessionBus().send(
                    QDBusMessage::createMethodCall(
                        QStringLiteral("org.kde.raven.Daemon"),
                        QStringLiteral("/Events"),
                        QStringLiteral("org.kde.raven.Events"),
                        QStringLiteral("tilingCommandsPending")
                    ) << cmds
                );
            }
        }
    } else {
        QProcess::startDetached(QStringLiteral("qdbus"), {
            QStringLiteral("org.kde.raven.Daemon"),
            QStringLiteral("/Events"),
            QStringLiteral("org.kde.raven.Events.%1").arg(action),
            QString::number(arg)
        });
    }
}

/**
 * @brief Alterna el estado de activación del Tiling (Mosaico) de ventanas.
 */
void RavenController::toggleTiling()
{
    m_tilingEnabled = !m_tilingEnabled;
    Q_EMIT tilingEnabledChanged();
    sendDbusAction(QStringLiteral("toggleTiling"));
}

/**
 * @brief Establece un estado explícito para el motor de Tiling.
 *
 * @param enabled True para habilitar, False para deshabilitar.
 */
void RavenController::setTilingEnabled(bool enabled)
{
    if (m_tilingEnabled != enabled) {
        toggleTiling();
    }
}

/**
 * @brief Cambia cíclicamente al siguiente esquema de disposición (Layout) disponible.
 */
void RavenController::cycleLayout()
{
    sendDbusAction(QStringLiteral("cycleLayout"));
}

/**
 * @brief Establece un esquema de disposición específico para el espacio de trabajo actual.
 *
 * @param layoutName Nombre de la disposición (ej. "Master", "Grid", "Floating").
 */
void RavenController::setLayout(const QString &layoutName)
{
    if (m_currentLayout != layoutName) {
        m_currentLayout = layoutName;
        Q_EMIT currentLayoutChanged();
    }
    
    if (m_dbusInterface && m_dbusInterface->isValid()) {
        QDBusMessage reply = m_dbusInterface->call(QStringLiteral("setLayoutForCurrentWorkspace"), layoutName);
        if (reply.type() == QDBusMessage::ReplyMessage && !reply.arguments().isEmpty()) {
            QString cmds = reply.arguments().first().toString();
            if (!cmds.isEmpty() && cmds != QStringLiteral("[]")) {
                // Notificar directamente al servicio de KWin como reaseguro en tiempo real
                QDBusConnection::sessionBus().send(
                    QDBusMessage::createMethodCall(
                        QStringLiteral("org.kde.raven.Daemon"),
                        QStringLiteral("/Events"),
                        QStringLiteral("org.kde.raven.Events"),
                        QStringLiteral("tilingCommandsPending")
                    ) << cmds
                );
            }
        }
    } else {
        QProcess::startDetached(QStringLiteral("qdbus"), {
            QStringLiteral("org.kde.raven.Daemon"),
            QStringLiteral("/Events"),
            QStringLiteral("org.kde.raven.Events.setLayoutForCurrentWorkspace"),
            layoutName
        });
    }
}

/**
 * @brief Alterna el estado flotante de la ventana actualmente enfocada.
 */
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

/**
 * @brief Incrementa o decrementa los márgenes perimetrales de las ventanas (Gaps).
 *
 * @param delta Valor a sumar o restar a los gaps actuales.
 */
void RavenController::incrementGaps(int delta)
{
    m_defaultGaps = qMax(0, m_defaultGaps + delta);
    Q_EMIT defaultGapsChanged();
    sendDbusActionWithArg(QStringLiteral("incrementGaps"), delta);
}

/**
 * @brief Aumenta la cantidad máxima de ventanas en el área principal (Master).
 */
void RavenController::incrementMaster()
{
    sendDbusAction(QStringLiteral("incrementMaster"));
}

/**
 * @brief Disminuye la cantidad máxima de ventanas en el área principal (Master).
 */
void RavenController::decrementMaster()
{
    sendDbusAction(QStringLiteral("decrementMaster"));
}

/**
 * @brief Incrementa la proporción de tamaño de la ventana principal (Master Ratio).
 */
void RavenController::increaseRatio()
{
    m_masterRatio = qMin(0.85, m_masterRatio + 0.05);
    Q_EMIT masterRatioChanged();
    sendDbusAction(QStringLiteral("increaseRatio"));
}

/**
 * @brief Disminuye la proporción de tamaño de la ventana principal (Master Ratio).
 */
void RavenController::decreaseRatio()
{
    m_masterRatio = qMax(0.15, m_masterRatio - 0.05);
    Q_EMIT masterRatioChanged();
    sendDbusAction(QStringLiteral("decreaseRatio"));
}

/**
 * @brief Intercambia la posición de la ventana activa con la ventana anterior en el orden espacial.
 */
void RavenController::swapPrev()
{
    sendDbusAction(QStringLiteral("swapPrev"));
}

/**
 * @brief Intercambia la posición de la ventana activa con la ventana siguiente en el orden espacial.
 */
void RavenController::swapNext()
{
    sendDbusAction(QStringLiteral("swapNext"));
}

/**
 * @brief Transfiere el enfoque a la ventana anterior del mosaico actual.
 */
void RavenController::focusPrev()
{
    sendDbusAction(QStringLiteral("focusPrev"));
}

/**
 * @brief Transfiere el enfoque a la ventana siguiente del mosaico actual.
 */
void RavenController::focusNext()
{
    sendDbusAction(QStringLiteral("focusNext"));
}

/**
 * @brief Migra la ventana activa hacia el siguiente monitor disponible.
 */
void RavenController::migrateActiveToScreen()
{
    invokeKWinShortcut(QStringLiteral("RavenMigrateMonitor"));
}

/**
 * @brief Migra la ventana activa hacia el monitor anterior disponible.
 */
void RavenController::migrateActiveToPrevScreen()
{
    invokeKWinShortcut(QStringLiteral("RavenMigratePrevMonitor"));
}

/**
 * @brief Migra la ventana activa hacia el siguiente escritorio virtual.
 */
void RavenController::migrateActiveToDesktop()
{
    invokeKWinShortcut(QStringLiteral("RavenMigrateDesktop"));
}

/**
 * @brief Migra la ventana activa hacia el escritorio virtual anterior.
 */
void RavenController::migrateActiveToPrevDesktop()
{
    invokeKWinShortcut(QStringLiteral("RavenMigratePrevDesktop"));
}

/**
 * @brief Ejecuta y despliega la aplicación de Centro de Control (GUI de configuración) de Raven.
 */
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
