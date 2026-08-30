#include "systemcontroller.h"
#include <QDBusMessage>
#include <QDBusConnection>
#include <QProcess>
#include <QDebug>

SystemController::SystemController(QObject *parent)
    : QObject(parent)
{
}

void SystemController::lock()
{
    // Try freedesktop screensaver interface (standard across Plasma and desktop environments)
    QDBusMessage msg = QDBusMessage::createMethodCall(
        QStringLiteral("org.freedesktop.ScreenSaver"),
        QStringLiteral("/ScreenSaver"),
        QStringLiteral("org.freedesktop.ScreenSaver"),
        QStringLiteral("Lock")
    );
    QDBusConnection::sessionBus().asyncCall(msg);
}

void SystemController::logout()
{
    QDBusMessage msg = QDBusMessage::createMethodCall(
        QStringLiteral("org.kde.LogoutPrompt"),
        QStringLiteral("/LogoutPrompt"),
        QStringLiteral("org.kde.LogoutPrompt"),
        QStringLiteral("promptLogout")
    );
    QDBusConnection::sessionBus().asyncCall(msg);
}

void SystemController::suspend()
{
    QDBusMessage msg = QDBusMessage::createMethodCall(
        QStringLiteral("org.kde.Solid.PowerManagement"),
        QStringLiteral("/org/kde/Solid/PowerManagement/Actions/SuspendSession"),
        QStringLiteral("org.kde.Solid.PowerManagement.Actions.SuspendSession"),
        QStringLiteral("suspendToRam")
    );
    QDBusConnection::sessionBus().asyncCall(msg);
}

void SystemController::reboot()
{
    QDBusMessage msg = QDBusMessage::createMethodCall(
        QStringLiteral("org.kde.LogoutPrompt"),
        QStringLiteral("/LogoutPrompt"),
        QStringLiteral("org.kde.LogoutPrompt"),
        QStringLiteral("promptReboot")
    );
    QDBusConnection::sessionBus().asyncCall(msg);
}

void SystemController::shutdown()
{
    QDBusMessage msg = QDBusMessage::createMethodCall(
        QStringLiteral("org.kde.LogoutPrompt"),
        QStringLiteral("/LogoutPrompt"),
        QStringLiteral("org.kde.LogoutPrompt"),
        QStringLiteral("promptShutDown")
    );
    QDBusConnection::sessionBus().asyncCall(msg);
}

