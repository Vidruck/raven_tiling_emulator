/**
 * @file systemcontroller.h
 * @brief Controlador de acciones de gestión de energía y sesión de usuario para KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

#ifndef SYSTEMCONTROLLER_H
#define SYSTEMCONTROLLER_H

#include <QObject>
#include <QDBusInterface>
#include <QDBusReply>
#include <qqmlintegration.h>

/**
 * @class SystemController
 * @brief Controlador C++ que expone acciones de administración de energía y sesión a la interfaz QML.
 *
 * Utiliza llamadas D-Bus directas hacia `org.freedesktop.login1` (systemd-logind),
 * `org.freedesktop.ScreenSaver` y `org.kde.ksmserver` para ejecutar de forma segura:
 * - Bloqueo de sesión interactiva.
 * - Cierre de sesión de usuario.
 * - Suspensión a RAM (Sleep).
 * - Reinicio del equipo.
 * - Apagado completo del sistema.
 */
class SystemController : public QObject
{
    Q_OBJECT
    QML_ELEMENT

public:
    /**
     * @brief Constructor principal.
     * @param parent Puntero opcional al objeto padre Qt.
     */
    explicit SystemController(QObject *parent = nullptr);

    /** @brief Bloquea la sesión actual activando la pantalla de bloqueo de KDE Plasma. */
    Q_INVOKABLE void lock();
    
    /** @brief Solicita el cierre de sesión interactivo a través de KSMServer. */
    Q_INVOKABLE void logout();
    
    /** @brief Suspende el equipo a memoria RAM (Sleep/Suspend vía systemd-logind). */
    Q_INVOKABLE void suspend();
    
    /** @brief Reinicia el sistema operativo de forma ordenada. */
    Q_INVOKABLE void reboot();
    
    /** @brief Apaga y desconecta la alimentación del equipo. */
    Q_INVOKABLE void shutdown();
};

#endif // SYSTEMCONTROLLER_H
