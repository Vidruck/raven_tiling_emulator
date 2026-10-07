/**
 * @file mediacontroller.cpp
 * @brief Implementación del controlador multimedia MPRIS2 para KDE Plasma.
 * @author Alejandro González Hernández (Vidruck)
 * @license GPL-3.0
 */

#include "mediacontroller.h"
#include <QDBusConnectionInterface>
#include <QDBusMessage>
#include <QDBusReply>
#include <QDBusArgument>
#include <QDBusMetaType>
#include <QRegularExpression>
#include <QDBusObjectPath>
#include <QDebug>

#include <QProcess>
#include <QStandardPaths>
#include <QTemporaryFile>
#include <QDir>
#include <QFile>

/**
 * @brief Constructor del controlador multimedia MPRIS.
 * @param parent Objeto padre.
 */
MediaController::MediaController(QObject *parent)
    : QObject(parent)
{
    // Suscripción a cambios de propietario de nombres en D-Bus
    QDBusConnection::sessionBus().connect(
        QStringLiteral("org.freedesktop.DBus"),
        QStringLiteral("/org/freedesktop/DBus"),
        QStringLiteral("org.freedesktop.DBus"),
        QStringLiteral("NameOwnerChanged"),
        this,
        SLOT(onNameOwnerChanged(QString,QString,QString))
    );

    // Inicializar lista del espectro de 31 bandas en 0.0
    for (int i = 0; i < 31; ++i) {
        m_spectrumList.append(0.0);
    }

    m_positionTimer = new QTimer(this);
    m_positionTimer->setInterval(1000);
    connect(m_positionTimer, &QTimer::timeout, this, &MediaController::updatePosition);

    m_discoveryTimer = new QTimer(this);
    m_discoveryTimer->setInterval(3000);
    connect(m_discoveryTimer, &QTimer::timeout, this, [this]() {
        if (m_active && (!m_hasPlayer || !isPlaying())) {
            findActivePlayer();
        }
    });

    if (m_active) {
        m_discoveryTimer->start();
        findActivePlayer();
    }
}

/**
 * @brief Escanea la sesión de D-Bus para encontrar un reproductor multimedia activo.
 *
 * Busca servicios registrados que implementen la interfaz `org.mpris.MediaPlayer2`.
 * Da prioridad a aquellos reproductores cuyo estado sea 'Playing' (Reproduciendo).
 */
void MediaController::findActivePlayer()
{
    QDBusConnection bus = QDBusConnection::sessionBus();
    QDBusConnectionInterface *interface = bus.interface();
    if (!interface) return;

    QStringList services = interface->registeredServiceNames();
    QString bestPlayingWithMetadata;
    QString bestPlaying;
    QString bestWithMetadata;
    QString firstFound;

    for (const QString &service : services) {
        if (!service.startsWith(QLatin1String("org.mpris.MediaPlayer2."))) {
            continue;
        }

        // Ignorar subprocesos sandboxed / webkit que no son el reproductor principal
        if (service.contains(QLatin1String(".Sandboxed.")) || service.contains(QLatin1String(".instance-"))) {
            continue;
        }

        if (firstFound.isEmpty()) {
            firstFound = service;
        }

        // Consultar estado de reproducción
        QDBusMessage msgStatus = QDBusMessage::createMethodCall(service,
                                                               QStringLiteral("/org/mpris/MediaPlayer2"),
                                                               QStringLiteral("org.freedesktop.DBus.Properties"),
                                                               QStringLiteral("Get"));
        msgStatus << QStringLiteral("org.mpris.MediaPlayer2.Player") << QStringLiteral("PlaybackStatus");
        
        QString status;
        QDBusReply<QDBusVariant> replyStatus = bus.call(msgStatus, QDBus::Block, 100);
        if (replyStatus.isValid()) {
            status = replyStatus.value().variant().toString();
        }

        // Consultar metadatos para saber si tiene una canción real
        QDBusMessage msgMeta = QDBusMessage::createMethodCall(service,
                                                             QStringLiteral("/org/mpris/MediaPlayer2"),
                                                             QStringLiteral("org.freedesktop.DBus.Properties"),
                                                             QStringLiteral("Get"));
        msgMeta << QStringLiteral("org.mpris.MediaPlayer2.Player") << QStringLiteral("Metadata");
        QDBusReply<QDBusVariant> replyMeta = bus.call(msgMeta, QDBus::Block, 100);

        bool hasTrack = false;
        if (replyMeta.isValid()) {
            QVariant var = replyMeta.value().variant();
            QVariantMap metaMap;
            if (var.canConvert<QDBusArgument>()) {
                QDBusArgument arg = var.value<QDBusArgument>();
                metaMap = qdbus_cast<QVariantMap>(arg);
            } else if (var.canConvert<QVariantMap>()) {
                metaMap = var.toMap();
            }
            QString title = metaMap.value(QLatin1String("xesam:title")).toString();
            QString art = metaMap.value(QLatin1String("mpris:artUrl")).toString();
            if (!title.isEmpty() || !art.isEmpty()) {
                hasTrack = true;
            }
        }

        if (status == QLatin1String("Playing")) {
            if (hasTrack) {
                bestPlayingWithMetadata = service;
                break; // Máxima prioridad alcanzada
            } else if (bestPlaying.isEmpty()) {
                bestPlaying = service;
            }
        } else if (hasTrack && bestWithMetadata.isEmpty()) {
            bestWithMetadata = service;
        }
    }

    QString targetService;
    if (!bestPlayingWithMetadata.isEmpty()) {
        targetService = bestPlayingWithMetadata;
    } else if (!bestPlaying.isEmpty()) {
        targetService = bestPlaying;
    } else if (!bestWithMetadata.isEmpty()) {
        targetService = bestWithMetadata;
    } else {
        targetService = firstFound;
    }

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

/**
 * @brief Conecta el controlador a un reproductor MPRIS específico.
 *
 * Obtiene el nombre comercial del reproductor consultando la propiedad `Identity`
 * de la interfaz `org.mpris.MediaPlayer2` y suscribe a los cambios de propiedades.
 *
 * @param service Nombre del servicio D-Bus del reproductor.
 */
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

    // Intentar leer la propiedad 'Identity' de org.mpris.MediaPlayer2 en D-Bus
    QString identityName;
    {
        QDBusMessage msg = QDBusMessage::createMethodCall(service,
                                                          QStringLiteral("/org/mpris/MediaPlayer2"),
                                                          QStringLiteral("org.freedesktop.DBus.Properties"),
                                                          QStringLiteral("Get"));
        msg << QStringLiteral("org.mpris.MediaPlayer2") << QStringLiteral("Identity");
        QDBusReply<QDBusVariant> reply = bus.call(msg, QDBus::Block, 100);
        if (reply.isValid()) {
            identityName = reply.value().variant().toString().trimmed();
        }
    }

    if (!identityName.isEmpty()) {
        // Normalizar primera letra a mayúscula si viene en minúsculas (ej. "zuno" -> "Zuno")
        if (identityName.length() == 1) {
            m_playerName = identityName.toUpper();
        } else if (identityName.at(0).isLower()) {
            m_playerName = identityName.left(1).toUpper() + identityName.mid(1);
        } else {
            m_playerName = identityName;
        }
    } else {
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
            // Si tiene formato de dominio reverso (ej. org.gnome.Music, io.bassi.Amberol, etc.)
            QStringList parts = cleanName.split(QLatin1Char('.'));
            if (!parts.isEmpty()) {
                cleanName = parts.last();
            }
            if (!cleanName.isEmpty()) {
                cleanName = cleanName.left(1).toUpper() + cleanName.mid(1);
            }
        }
        m_playerName = cleanName;
    }

    bus.connect(service, QStringLiteral("/org/mpris/MediaPlayer2"),
                QStringLiteral("org.freedesktop.DBus.Properties"),
                QStringLiteral("PropertiesChanged"),
                this, SLOT(onPropertiesChanged(QString,QVariantMap,QStringList)));

    refresh();
}

/**
 * @brief Slot manejador que reacciona cuando un reproductor se abre o se cierra en el sistema.
 *
 * @param name Nombre del servicio que cambió.
 * @param oldOwner Antiguo propietario del servicio.
 * @param newOwner Nuevo propietario del servicio.
 */
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

/**
 * @brief Slot manejador para la señal PropertiesChanged de MPRIS.
 *
 * Escucha cambios en tiempo real del estado de reproducción, metadatos y controles
 * disponibles (CanGoNext, CanGoPrevious) para reflejarlos instantáneamente en la interfaz.
 *
 * @param interfaceName Nombre de la interfaz que emitió el cambio.
 * @param changedProperties Mapa de propiedades modificadas.
 * @param invalidatedProperties Lista de propiedades invalidadas.
 */
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
        updateCavaState();
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

/**
 * @brief Actualiza las variables internas con los metadatos recibidos del reproductor.
 *
 * Extrae título, artista, álbum, carátula (artUrl) y URL de pista. Si el reproductor
 * es un navegador con YouTube, intenta generar la carátula automáticamente.
 *
 * @param metadata Mapa de metadatos (xesam, mpris).
 */
void MediaController::updateMetadata(const QVariantMap &metadata)
{
    QString newTitle = metadata.value(QLatin1String("xesam:title")).toString();
    QString newTrackId = metadata.value(QLatin1String("mpris:trackid")).toString();
    QString newTrackUrl = metadata.value(QLatin1String("xesam:url")).toString();

    // Determinamos si es una pista completamente nueva antes de actualizar
    bool isDifferentTrack = false;
    if (!newTrackUrl.isEmpty() && !m_trackUrl.isEmpty()) {
        isDifferentTrack = (newTrackUrl != m_trackUrl);
    } else if (!newTrackId.isEmpty() && !m_trackId.isEmpty() && newTrackId != QStringLiteral("/org/mpris/MediaPlayer2/TrackList/NoTrack")) {
        isDifferentTrack = (newTrackId != m_trackId);
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

    // Respaldo: Si no hay artUrl pero es una URL de YouTube en xesam:url
    if (m_artUrl.isEmpty()) {
        QRegularExpression ytRegex(QStringLiteral("(?:v=|/v/|youtu\\.be/)([a-zA-Z0-9_-]{11})"));
        QRegularExpressionMatch match = ytRegex.match(newTrackUrl);
        if (match.hasMatch()) {
            QString videoId = match.captured(1);
            m_artUrl = QStringLiteral("https://img.youtube.com/vi/%1/hqdefault.jpg").arg(videoId);
        }
    }

    // Extraer mpris:length (convertir de microsegundos a segundos)
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

/**
 * @brief Activa o desactiva la actualización periódica de la posición multimedia.
 *
 * Utilizado para ahorrar recursos cuando el widget no está visible en la interfaz.
 *
 * @param active booleano que indica si se debe monitorizar activamente.
 */
void MediaController::updateCavaState()
{
    bool shouldRun = m_active && isPlaying();

    if (shouldRun) {
        if (!m_cavaProcess) {
            // Verificar si cava existe en el sistema
            QString cavaExe = QStandardPaths::findExecutable(QStringLiteral("cava"));
            if (cavaExe.isEmpty()) {
                // Si CAVA no está instalado, m_cavaProcess permanece nulo y la UI usará fallback
                return;
            }

            // Crear o refrescar archivo temporal de configuración para CAVA
            QString tempDir = QDir::tempPath();
            m_cavaConfigPath = QStringLiteral("%1/raven_cava_config").arg(tempDir);
            QFile cfgFile(m_cavaConfigPath);
            if (cfgFile.open(QIODevice::WriteOnly | QIODevice::Truncate | QIODevice::Text)) {
                QTextStream out(&cfgFile);
                out << "[general]\n"
                    << "bars = 31\n"
                    << "framerate = 60\n"
                    << "autosens = 1\n"
                    << "overshoot = 20\n"
                    << "sensitivity = 240\n"
                    << "lower_cutoff_freq = 40\n"
                    << "higher_cutoff_freq = 14000\n\n"
                    << "[input]\n"
                    << "method = pulse\n"
                    << "source = auto\n\n"
                    << "[output]\n"
                    << "channels = mono\n"
                    << "mono_option = average\n"
                    << "method = raw\n"
                    << "raw_target = /dev/stdout\n"
                    << "data_format = ascii\n"
                    << "ascii_max_range = 100\n"
                    << "bar_delimiter = 59\n\n"
                    << "[smoothing]\n"
                    << "integral = 60\n"
                    << "monstercat = 1\n"
                    << "waves = 0\n"
                    << "gravity = 140\n"
                    << "noise_reduction = 0.05\n";
                cfgFile.close();
            }

            m_cavaProcess = new QProcess(this);
            m_cavaProcess->setProgram(cavaExe);
            m_cavaProcess->setArguments({QStringLiteral("-p"), m_cavaConfigPath});

            connect(m_cavaProcess, &QProcess::readyReadStandardOutput, this, &MediaController::readCavaOutput);
            m_cavaProcess->start();
        }
    } else {
        if (m_cavaProcess) {
            m_cavaProcess->kill();
            m_cavaProcess->deleteLater();
            m_cavaProcess = nullptr;
        }
        // Resetear barras a cero
        bool changed = false;
        for (int i = 0; i < m_spectrumList.size(); ++i) {
            if (m_spectrumList[i].toDouble() != 0.0) {
                m_spectrumList[i] = 0.0;
                changed = true;
            }
        }
        if (changed) {
            emit spectrumChanged();
        }
    }
}

/**
 * @brief Lee la salida ascii de CAVA y actualiza m_spectrumList con valores normalizados (0.0 a 1.0).
 */
void MediaController::readCavaOutput()
{
    if (!m_cavaProcess) return;

    QByteArray latestLine;
    while (m_cavaProcess->canReadLine()) {
        QByteArray line = m_cavaProcess->readLine().trimmed();
        if (!line.isEmpty()) {
            latestLine = line;
        }
    }

    if (latestLine.isEmpty()) return;

    QList<QByteArray> parts = latestLine.split(';');
    if (parts.isEmpty()) return;

    bool hasUpdate = false;
    int count = qMin(parts.size(), m_spectrumList.size());
    for (int i = 0; i < count; ++i) {
        int val = parts[i].toInt();
        double normalized = qBound(0.0, static_cast<double>(val) / 100.0, 1.0);
        if (qAbs(m_spectrumList[i].toDouble() - normalized) > 0.002) {
            m_spectrumList[i] = normalized;
            hasUpdate = true;
        }
    }

    if (hasUpdate) {
        emit spectrumChanged();
    }
}

/**
 * @brief Activa o desactiva la actualización periódica de la posición multimedia y CAVA.
 *
 * Utilizado para ahorrar recursos cuando el widget no está visible en la interfaz.
 *
 * @param active booleano que indica si se debe monitorizar activamente.
 */
void MediaController::setActive(bool active)
{
    if (m_active == active) return;
    m_active = active;
    emit activeChanged();

    if (m_active) {
        if (!m_hasPlayer || !isPlaying()) {
            findActivePlayer();
        }
        if (m_discoveryTimer) {
            m_discoveryTimer->start();
        }
        if (m_hasPlayer && isPlaying()) {
            m_positionTimer->start();
            queryPositionDirect();
        }
    } else {
        if (m_discoveryTimer) {
            m_discoveryTimer->stop();
        }
        if (m_positionTimer) {
            m_positionTimer->stop();
        }
        if (m_posWatcher) {
            m_posWatcher->deleteLater();
            m_posWatcher = nullptr;
        }
    }

    updateCavaState();
}

/**
 * @brief Incrementa suavemente el segundero de posición en la interfaz.
 *
 * Realiza una extrapolación en vivo mientras se reproduce, evitando inundar a D-Bus
 * con llamadas innecesarias a cada segundo.
 */
void MediaController::updatePosition()
{
    if (!m_active || m_currentService.isEmpty()) return;

    // Extrapolación suave de 1 segundo mientras se reproduce, acotada al tamaño de la pista
    if (isPlaying()) {
        if (m_length <= 0 || m_position < m_length) {
            m_position += 1;
            emit positionChanged();
        }
    }

    queryPositionDirect();
}

/**
 * @brief Solicita asíncronamente la posición actual de reproducción por D-Bus.
 */
void MediaController::queryPositionDirect()
{
    if (m_currentService.isEmpty()) {
        return;
    }

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

/**
 * @brief Slot manejador para sincronizar la respuesta asíncrona de la posición con la propiedad interna.
 *
 * @param watcher Vigía de la llamada asíncrona.
 */
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
            // Solo sincronizamos si hay un desvío apreciable de más de 2 segundos o al reanudar/arrancar
            if (qAbs(m_position - realPos) > 2 || (m_position == 0 && realPos > 0)) {
                m_position = realPos;
                emit positionChanged();
            }
        }
    }
    watcher->deleteLater();
}

/**
 * @brief Fuerza una consulta completa del estado, propiedades y metadatos del reproductor.
 */
void MediaController::refresh()
{
    if (m_currentService.isEmpty()) {
        findActivePlayer();
        return;
    }

    QDBusConnection bus = QDBusConnection::sessionBus();

    // Consultar el estado de reproducción (PlaybackStatus)
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

    // Consultar los metadatos (Metadata)
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

/**
 * @brief Envía el comando de Iniciar Reproducción.
 */
void MediaController::play()
{
    if (m_currentService.isEmpty()) {
        findActivePlayer();
        if (m_currentService.isEmpty()) return;
    }
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QDBus::NoBlock, QStringLiteral("Play"));
}

/**
 * @brief Envía el comando de Pausar Reproducción.
 */
void MediaController::pause()
{
    if (m_currentService.isEmpty()) {
        findActivePlayer();
        if (m_currentService.isEmpty()) return;
    }
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QDBus::NoBlock, QStringLiteral("Pause"));
}

/**
 * @brief Envía el comando de Alternar (Reproducir/Pausar).
 */
void MediaController::playPause()
{
    if (m_currentService.isEmpty()) {
        findActivePlayer();
        if (m_currentService.isEmpty()) return;
    }
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QDBus::NoBlock, QStringLiteral("PlayPause"));
}

/**
 * @brief Envía el comando de Pista Siguiente.
 */
void MediaController::next()
{
    if (m_currentService.isEmpty()) {
        findActivePlayer();
        if (m_currentService.isEmpty()) return;
    }
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QDBus::NoBlock, QStringLiteral("Next"));
}

/**
 * @brief Envía el comando de Pista Anterior.
 */
void MediaController::previous()
{
    if (m_currentService.isEmpty()) {
        findActivePlayer();
        if (m_currentService.isEmpty()) return;
    }
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QDBus::NoBlock, QStringLiteral("Previous"));
}

/**
 * @brief Envía el comando de Detener.
 */
void MediaController::stop()
{
    if (m_currentService.isEmpty()) {
        findActivePlayer();
        if (m_currentService.isEmpty()) return;
    }
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    playerIface.call(QDBus::NoBlock, QStringLiteral("Stop"));
}

/**
 * @brief Ajusta la posición del cabezal de reproducción.
 *
 * @param positionSec Segundos exactos en los que se debe reanudar la pista.
 */
void MediaController::setPosition(qint64 positionSec)
{
    if (m_currentService.isEmpty() || m_trackId.isEmpty()) return;
    QDBusInterface playerIface(m_currentService, QStringLiteral("/org/mpris/MediaPlayer2"),
                              QStringLiteral("org.mpris.MediaPlayer2.Player"),
                              QDBusConnection::sessionBus());
    qulonglong posMicro = static_cast<qulonglong>(positionSec * 1000000);
    playerIface.call(QStringLiteral("SetPosition"), QVariant::fromValue(QDBusObjectPath(m_trackId)), posMicro);
}

/**
 * @brief Formatea un valor en segundos a una cadena MM:SS legible por humanos.
 *
 * @param seconds Cantidad de segundos totales.
 * @return Cadena formateada.
 */
QString MediaController::formatTime(qint64 seconds) const
{
    if (seconds < 0) seconds = 0;
    int m = static_cast<int>(seconds) / 60;
    int s = static_cast<int>(seconds) % 60;
    return QStringLiteral("%1:%2").arg(m).arg(s, 2, 10, QLatin1Char('0'));
}
