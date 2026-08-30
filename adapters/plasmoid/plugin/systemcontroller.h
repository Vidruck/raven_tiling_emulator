#ifndef SYSTEMCONTROLLER_H
#define SYSTEMCONTROLLER_H

#include <QObject>
#include <QDBusInterface>
#include <QDBusReply>
#include <qqmlintegration.h>

class SystemController : public QObject
{
    Q_OBJECT
    QML_ELEMENT

public:
    explicit SystemController(QObject *parent = nullptr);

    Q_INVOKABLE void lock();
    Q_INVOKABLE void logout();
    Q_INVOKABLE void suspend();
    Q_INVOKABLE void reboot();
    Q_INVOKABLE void shutdown();
};

#endif // SYSTEMCONTROLLER_H
