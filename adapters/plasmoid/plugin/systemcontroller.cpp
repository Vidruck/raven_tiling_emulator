/**
 * @file systemcontroller.cpp
 * @brief Implementación del controlador de acciones de gestión de energía y sesión.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 * @license GPL-3.0
 */

#include "systemcontroller.h"
#include <QDBusMessage>
#include <QDBusConnection>
#include <QProcess>
#include <QDebug>

/**
 * @brief Constructor del controlador de energía y sesión.
 * @param parent Objeto padre.
 */
SystemController::SystemController(QObject *parent)
    : QObject(parent)
{
}

/**
 * @brief Bloquea la sesión actual de forma segura.
 *
 * Utiliza la interfaz D-Bus estándar de Freedesktop para interactuar con
 * el protector de pantalla subyacente y bloquear la pantalla.
 */
void SystemController::lock()
{
    // Intentar usar la interfaz de screensaver de freedesktop (estándar en Plasma y otros entornos de escritorio)
    QDBusMessage msg = QDBusMessage::createMethodCall(
        QStringLiteral("org.freedesktop.ScreenSaver"),
        QStringLiteral("/ScreenSaver"),
        QStringLiteral("org.freedesktop.ScreenSaver"),
        QStringLiteral("Lock")
    );
    QDBusConnection::sessionBus().asyncCall(msg);
}

/**
 * @brief Llama al cuadro de diálogo nativo para cerrar la sesión.
 *
 * Emite una llamada D-Bus a `org.kde.LogoutPrompt` para permitir
 * al usuario guardar su trabajo antes de salir.
 */
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

/**
 * @brief Suspende el equipo a memoria RAM (Sleep).
 *
 * Llama a la API de Solid PowerManagement a través de D-Bus para
 * entrar en estado de suspensión de forma controlada.
 */
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

/**
 * @brief Llama al cuadro de diálogo nativo para reiniciar el sistema.
 *
 * Emite una llamada D-Bus a `org.kde.LogoutPrompt` configurado
 * para solicitar confirmación de reinicio.
 */
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

/**
 * @brief Llama al cuadro de diálogo nativo para apagar el sistema.
 *
 * Emite una llamada D-Bus a `org.kde.LogoutPrompt` configurado
 * para solicitar confirmación de apagado total.
 */
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

