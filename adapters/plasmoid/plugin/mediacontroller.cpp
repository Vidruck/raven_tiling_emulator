#include "mediacontroller.h"
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusReply>
#include <QDBusArgument>
#include <QDBusMetaType>
#include <QRegularExpression>
#include <QDBusObjectPath>
#include <QDebug>

MediaController::MediaController(QObject *parent)
    : QObject(parent)
{
    // DBus name owner change subscription
    QDBusConnection::sessionBus().connect(
        QStringLiteral("org.freedesktop.DBus"),
        QStringLiteral("/org/freedesktop/DBus"),
        QStringLiteral("org.freedesktop.DBus"),
        QStringLiteral("NameOwnerChanged"),
        this,
        SLOT(onNameOwnerChanged(QString,QString,QString))
    );

    m_positionTimer = new QTimer(this);
    m_positionTimer->setInterval(1000);
    connect(m_positionTimer, &QTimer::timeout, this, &MediaController::updatePosition);

    findActivePlayer();
}

void MediaController::findActivePlayer()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *interface = bus.interface();
    if (!interface) return;

    QStringList services = interface->registeredServiceNames();
    QString foundPlaying;
    QString firstFound;

    for (const QString &service : services) {
        if (service.startsWith(QLatin1String("org.mpris.MediaPlayer2."))) {
            if (firstFound.isEmpty()) {
                firstFound = service;
            }

            QDBusMessage msg = QDBusMessage::createMethodCall(service,
                                                              QStringLiteral("/org/mpris/MediaPlayer2"),
                                                              QStringLiteral("org.freedesktop.DBus.Properties"),
                                                              QStringLiteral("Get"));
            msg << QStringLiteral("org.mpris.MediaPlayer2.Player") << QStringLiteral("PlaybackStatus");
            
            // Short timeout (100ms) to never block Plasma GUI
            QDBusReply<QDBusVariant> reply = bus.call(msg, QDBus::Block, 100);
            if (reply.isValid()) {
                QString status = reply.value().variant().toString();
                if (status == QLatin1String("Playing")) {
                    foundPlaying = service;
                    break;
                }
            }
        }
    }

    QString targetService = !foundPlaying.isEmpty() ? foundPlaying : firstFound;

    if (!targetService.isEmpty()) {
        connectToPlayer(targetService);
    } else {
        m_hasPlayer = false;
        m_currentService.clear();
        m_playerName.clear();
        m_trackTitle.clear();
        m_artist.clear();
        m_album.clear();
        m_artUrl.clear();
        m_playbackStatus = QStringLiteral("Stopped");
        m_position = 0;
        m_length = 0;
        m_positionTimer->stop();
        emit mediaChanged();
        emit positionChanged();
    }
}

void MediaController::connectToPlayer(const QString &service)
{
    QDBusConnection bus = QDBusConnection::sessionBus();

    if (!m_currentService.isEmpty() && m_currentService != service) {
        bus.disconnect(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                       QStringLiteral("org.freedesktop.DBus.Properties"),
                       QStringLiteral("PropertiesChanged"),
                       this, SLOT(onPropertiesChanged(QString,QVariantMap,QStringList)));
    }

    m_currentService = service;
    m_hasPlayer = true;

    QString cleanName = service.mid(QStringLiteral("org.mpris.MediaPlayer2.").length());
    if (cleanName.contains(QLatin1String("spotify"), Qt::CaseInsensitive)) {
        cleanName = QStringLiteral("Spotify");
    } else if (cleanName.contains(QLatin1String("vlc"), Qt::CaseInsensitive)) {
        cleanName = QStringLiteral("VLC Media Player");
    } else if (cleanName.contains(QLatin1String("mpv"), Qt::CaseInsensitive)) {
        cleanName = QStringLiteral("MPV");
    } else if (cleanName.contains(QLatin1String("elisa"), Qt::CaseInsensitive)) {
        cleanName = QStringLiteral("Elisa");
    } else if (cleanName.contains(QLatin1String("rhythmbox"), Qt::CaseInsensitive)) {
        cleanName = QStringLiteral("Rhythmbox");
    } else if (cleanName.contains(QLatin1String("audacious"), Qt::CaseInsensitive)) {
        cleanName = QStringLiteral("Audacious");
    } else if (cleanName.contains(QLatin1String("cider"), Qt::CaseInsensitive)) {
        cleanName = QStringLiteral("Apple Music / Cider");
    } else if (cleanName.contains(QLatin1String("plasma-browser-integration"), Qt::CaseInsensitive) ||
               cleanName.contains(QLatin1String("chromium"), Qt::CaseInsensitive) ||
               cleanName.contains(QLatin1String("chrome"), Qt::CaseInsensitive) ||
               cleanName.contains(QLatin1String("brave"), Qt::CaseInsensitive) ||
               cleanName.contains(QLatin1String("firefox"), Qt::CaseInsensitive)) {
        cleanName = QStringLiteral("Web Player / YouTube");
    } else {
        int dotIdx = cleanName.indexOf(QLatin1Char('.'));
        if (dotIdx != -1) cleanName = cleanName.left(dotIdx);
        if (!cleanName.isEmpty()) {
            cleanName = cleanName.left(1).toUpper() + cleanName.mid(1);
        }
    }
    m_playerName = cleanName;

    bus.connect(service, QStringLiteral("/org/mpris/MediaPlayer2"),
                QStringLiteral("org.freedesktop.DBus.Properties"),
                QStringLiteral("PropertiesChanged"),
                this, SLOT(onPropertiesChanged(QString,QVariantMap,QStringList)));

    refresh();
}

void MediaController::onNameOwnerChanged(const QString &name, const QString &oldOwner, const QString &newOwner)
{
    if (!name.startsWith(QLatin1String("org.mpris.MediaPlayer2."))) {
        return;
    }

    if (oldOwner.isEmpty() && !newOwner.isEmpty()) {
        connectToPlayer(name);
    } else if (!oldOwner.isEmpty() && newOwner.isEmpty()) {
        if (name == m_currentService) {
            findActivePlayer();
        }
    }
}

void MediaController::onPropertiesChanged(const QString &interfaceName, const QVariantMap &changedProperties, const QStringList &/*invalidatedProperties*/)
{
    if (interfaceName != QLatin1String("org.mpris.MediaPlayer2.Player")) {
        return;
    }

    if (changedProperties.contains(QLatin1String("PlaybackStatus"))) {
        m_playbackStatus = changedProperties.value(QLatin1String("PlaybackStatus")).toString();
        if (m_playbackStatus == QLatin1String("Playing")) {
            if (m_active) m_positionTimer->start();
            updatePosition();
        } else {
            m_positionTimer->stop();
        }
    }

    if (changedProperties.contains(QLatin1String("Metadata"))) {
        QVariant metaVar = changedProperties.value(QLatin1String("Metadata"));
        if (metaVar.canConvert<QDBusArgument>()) {
            QDBusArgument arg = metaVar.value<QDBusArgument>();
            QVariantMap metaMap = qdbus_cast<QVariantMap>(arg);
            updateMetadata(metaMap);
        } else if (metaVar.canConvert<QVariantMap>()) {
            updateMetadata(metaVar.toMap());
        } else {
            refresh();
            return;
        }
    }

    if (changedProperties.contains(QLatin1String("CanGoNext"))) {
        m_canGoNext = changedProperties.value(QLatin1String("CanGoNext")).toBool();
    }
    if (changedProperties.contains(QLatin1String("CanGoPrevious"))) {
        m_canGoPrevious = changedProperties.value(QLatin1String("CanGoPrevious")).toBool();
    }

    emit mediaChanged();
}

void MediaController::updateMetadata(const QVariantMap &metadata)
{
    QString newTitle = metadata.value(QLatin1String("xesam:title")).toString();
    QString newTrackId = metadata.value(QLatin1String("mpris:trackid")).toString();
    QString newTrackUrl = metadata.value(QLatin1String("xesam:url")).toString();

    // Solo reiniciar posición si realmente cambió la pista (trackId o URL diferente, o título significativamente distinto y teníamos anterior)
    bool isDifferentTrack = false;
    if (!newTrackUrl.isEmpty() && !m_trackUrl.isEmpty()) {
        isDifferentTrack = (newTrackUrl != m_trackUrl);
    } else if (!newTrackId.isEmpty() && !m_trackId.isEmpty() && newTrackId != QStringLiteral("/org/mpris/MediaPlayer2/TrackList/NoTrack")) {
        isDifferentTrack = (newTrackId != m_trackId);
    } else if (!newTitle.isEmpty() && !m_trackTitle.isEmpty()) {
        // En YouTube el título puede ganar prefijos como "(1) " o cambiar mínimamente sin ser otra canción
        isDifferentTrack = (newTitle != m_trackTitle && !newTitle.contains(m_trackTitle) && !m_trackTitle.contains(newTitle));
    }

    if (isDifferentTrack) {
        m_position = 0;
    }

    m_trackTitle = newTitle;
    m_trackUrl = newTrackUrl;

    QVariant artistVar = metadata.value(QLatin1String("xesam:artist"));
    if (artistVar.canConvert<QStringList>()) {
        m_artist = artistVar.toStringList().join(QLatin1String(", "));
    } else {
        m_artist = artistVar.toString();
    }

    m_album = metadata.value(QLatin1String("xesam:album")).toString();
    m_artUrl = metadata.value(QLatin1String("mpris:artUrl")).toString();

    // Fallback: If no artUrl but YouTube URL in xesam:url
    if (m_artUrl.isEmpty()) {
        QRegularExpression ytRegex(QStringLiteral("(?:v=|/v/|youtu\\.be/)([a-zA-Z0-9_-]{11})"));
        QRegularExpressionMatch match = ytRegex.match(newTrackUrl);
        if (match.hasMatch()) {
            QString videoId = match.captured(1);
            m_artUrl = QStringLiteral("https://img.youtube.com/vi/%1/hqdefault.jpg").arg(videoId);
        }
    }

    // Extract mpris:length (convert from microseconds to seconds)
    qint64 lenMicro = 0;
    if (metadata.contains(QLatin1String("mpris:length"))) {
        QVariant lenVar = metadata.value(QLatin1String("mpris:length"));
        lenMicro = lenVar.toLongLong();
        if (lenMicro <= 0) {
            lenMicro = lenVar.toULongLong();
        }
    }
    m_length = (lenMicro > 0) ? (lenMicro / 1000000) : 0;

    m_trackId = newTrackId.isEmpty() ? QStringLiteral("/org/mpris/MediaPlayer2/TrackList/NoTrack") : newTrackId;

    queryPositionDirect();
}

void MediaController::setActive(bool active)
{
    if (m_active == active) return;
    m_active = active;
    emit activeChanged();

    if (m_active && m_hasPlayer && isPlaying()) {
        m_positionTimer->start();
        queryPositionDirect();
    } else {
        m_positionTimer->stop();
    }
}

void MediaController::updatePosition()
{
    if (!m_active || m_currentService.isEmpty()) return;

    // Extrapolación suave de 1 segundo mientras se consulta la posición real
    if (isPlaying()) {
        if (m_length <= 0 || m_position < m_length) {
            m_position += 1;
            emit positionChanged();
        }
    }

    queryPositionDirect();
}

void MediaController::queryPositionDirect()
{
    if (m_currentService.isEmpty()) {
        return;
    }

    // Si había un watcher anterior pendiente pero no ha respondido en este ciclo, cancelarlo y reintentar
    if (m_posWatcher) {
        m_posWatcher->deleteLater();
        m_posWatcher = nullptr;
    }

    QDBusMessage msg = QDBusMessage::createMethodCall(m_currentService,
                                                      QStringLiteral("/org/mpris/MediaPlayer2"),
                                                      QStringLiteral("org.freedesktop.DBus.Properties"),
                                                      QStringLiteral("Get"));
    msg << QStringLiteral("org.mpris.MediaPlayer2.Player") << QStringLiteral("Position");

    QDBusPendingCall async = QDBusConnection::sessionBus().asyncCall(msg, 500);
    m_posWatcher = new QDBusPendingCallWatcher(async, this);
    connect(m_posWatcher, &QDBusPendingCallWatcher::finished, this, &MediaController::onPositionReply);
}

void MediaController::onPositionReply(QDBusPendingCallWatcher *watcher)
{
    if (watcher == m_posWatcher) {
        m_posWatcher = nullptr;
    }

    QDBusPendingReply<QDBusVariant> reply = *watcher;
    if (reply.isValid()) {
        qint64 posMicro = reply.value().variant().toLongLong();
        if (posMicro <= 0) posMicro = reply.value().variant().toULongLong();

        if (posMicro >= 0) {
            qint64 realPos = posMicro / 1000000;
            // Sincronizar posición real recibida de D-Bus si hay desfase
            if (qAbs(m_position - realPos) >= 1 || (m_position == 0 && realPos > 0)) {
                m_position = realPos;
                emit positionChanged();
            }
        }
    }
    watcher->deleteLater();
}

void MediaController::refresh()
{
    if (m_currentService.isEmpty()) {
        findActivePlayer();
        return;
    }

    QDBusConnection bus = QDBusConnection::sessionBus();

    // Query PlaybackStatus
    {
        QDBusMessage msg = QDBusMessage::createMethodCall(m_currentService,
                                                          QStringLiteral("/org/mpris/MediaPlayer2"),
                                                          QStringLiteral("org.freedesktop.DBus.Properties"),
                                                          QStringLiteral("Get"));
        msg << QStringLiteral("org.mpris.MediaPlayer2.Player") << QStringLiteral("PlaybackStatus");
        QDBusReply<QDBusVariant> reply = bus.call(msg, QDBus::Block, 150);
        if (reply.isValid()) {
            m_playbackStatus = reply.value().variant().toString();
            if (m_playbackStatus == QLatin1String("Playing") && m_active) {
                m_positionTimer->start();
            } else {
                m_positionTimer->stop();
            }
        }
    }

    // Query Metadata
    {
        QDBusMessage msg = QDBusMessage::createMethodCall(m_currentService,
                                                          QStringLiteral("/org/mpris/MediaPlayer2"),
                                                          QStringLiteral("org.freedesktop.DBus.Properties"),
                                                          QStringLiteral("Get"));
        msg << QStringLiteral("org.mpris.MediaPlayer2.Player") << QStringLiteral("Metadata");
        QDBusReply<QDBusVariant> reply = bus.call(msg, QDBus::Block, 150);
        if (reply.isValid()) {
            QVariant var = reply.value().variant();
            if (var.canConvert<QDBusArgument>()) {
                QDBusArgument arg = var.value<QDBusArgument>();
                QVariantMap metaMap = qdbus_cast<QVariantMap>(arg);
                updateMetadata(metaMap);
            } else if (var.canConvert<QVariantMap>()) {
                updateMetadata(var.toMap());
            }
        }
    }

    updatePosition();
    emit mediaChanged();
}

void MediaController::play()
{
    if (m_currentService.isEmpty()) return;
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QStringLiteral("Play"));
}

void MediaController::pause()
{
    if (m_currentService.isEmpty()) return;
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QStringLiteral("Pause"));
}

void MediaController::playPause()
{
    if (m_currentService.isEmpty()) {
        findActivePlayer();
        if (m_currentService.isEmpty()) return;
    }
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QStringLiteral("PlayPause"));
}

void MediaController::next()
{
    if (m_currentService.isEmpty()) return;
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QStringLiteral("Next"));
}

void MediaController::previous()
{
    if (m_currentService.isEmpty()) return;
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QStringLiteral("Previous"));
}

void MediaController::stop()
{
    if (m_currentService.isEmpty()) return;
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QStringLiteral("Stop"));
}

void MediaController::setPosition(qint64 positionSec)
{
    if (m_currentService.isEmpty() || m_trackId.isEmpty()) return;
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    qulonglong posMicro = static_cast<qulonglong>(positionSec * 1000000);
    playerIface.call(QStringLiteral("SetPosition"), QVariant::fromValue(QDBusObjectPath(m_trackId)), posMicro);
}

QString MediaController::formatTime(qint64 seconds) const
{
    if (seconds < 0) seconds = 0;
    int m = static_cast<int>(seconds) / 60;
    int s = static_cast<int>(seconds) % 60;
    return QStringLiteral("%1:%2").arg(m).arg(s, 2, 10, QLatin1Char('0'));
}
