#ifndef MEDIACONTROLLER_H
#define MEDIACONTROLLER_H

#include <QObject>
#include <QString>
#include <QVariantMap>
#include <QTimer>
#include <QDBusConnection>
#include <QDBusInterface>
#include <qqmlintegration.h>

#include <QDBusPendingCall>
#include <QDBusPendingCallWatcher>

class MediaController : public QObject
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(bool active READ active WRITE setActive NOTIFY activeChanged)
    Q_PROPERTY(bool hasPlayer READ hasPlayer NOTIFY mediaChanged)
    Q_PROPERTY(QString playerName READ playerName NOTIFY mediaChanged)
    Q_PROPERTY(QString trackTitle READ trackTitle NOTIFY mediaChanged)
    Q_PROPERTY(QString artist READ artist NOTIFY mediaChanged)
    Q_PROPERTY(QString album READ album NOTIFY mediaChanged)
    Q_PROPERTY(QString artUrl READ artUrl NOTIFY mediaChanged)
    Q_PROPERTY(QString playbackStatus READ playbackStatus NOTIFY mediaChanged)
    Q_PROPERTY(bool isPlaying READ isPlaying NOTIFY mediaChanged)
    Q_PROPERTY(qint64 position READ position NOTIFY positionChanged)
    Q_PROPERTY(qint64 length READ length NOTIFY mediaChanged)
    Q_PROPERTY(bool canGoNext READ canGoNext NOTIFY mediaChanged)
    Q_PROPERTY(bool canGoPrevious READ canGoPrevious NOTIFY mediaChanged)

public:
    explicit MediaController(QObject *parent = nullptr);

    bool hasPlayer() const { return m_hasPlayer; }
    QString playerName() const { return m_playerName; }
    QString trackTitle() const { return m_trackTitle; }
    QString artist() const { return m_artist; }
    QString album() const { return m_album; }
    QString artUrl() const { return m_artUrl; }
    QString playbackStatus() const { return m_playbackStatus; }
    bool isPlaying() const { return m_playbackStatus == QStringLiteral("Playing"); }
    qint64 position() const { return m_position; }
    qint64 length() const { return m_length; }
    bool canGoNext() const { return m_canGoNext; }
    bool canGoPrevious() const { return m_canGoPrevious; }

    Q_INVOKABLE void play();
    Q_INVOKABLE void pause();
    Q_INVOKABLE void playPause();
    Q_INVOKABLE void next();
    Q_INVOKABLE void previous();
    Q_INVOKABLE void stop();
    Q_INVOKABLE void setPosition(qint64 positionMs);
    Q_INVOKABLE void refresh();
    Q_INVOKABLE QString formatTime(qint64 seconds) const;

    bool active() const { return m_active; }
    void setActive(bool active);

signals:
    void activeChanged();
    void mediaChanged();
    void positionChanged();

private slots:
    void onNameOwnerChanged(const QString &name, const QString &oldOwner, const QString &newOwner);
    void onPropertiesChanged(const QString &interfaceName, const QVariantMap &changedProperties, const QStringList &invalidatedProperties);
    void updatePosition();
    void onPositionReply(QDBusPendingCallWatcher *watcher);

private:
    void findActivePlayer();
    void connectToPlayer(const QString &service);
    void updateMetadata(const QVariantMap &metadata);
    void queryPositionDirect();

    bool m_active = true;
    bool m_hasPlayer = false;
    QString m_currentService;
    QString m_playerName;
    QString m_trackTitle;
    QString m_artist;
    QString m_album;
    QString m_artUrl;
    QString m_playbackStatus = QStringLiteral("Stopped");
    QString m_trackId;
    QString m_trackUrl;
    qint64 m_position = 0;
    qint64 m_length = 0;
    bool m_canGoNext = true;
    bool m_canGoPrevious = true;

    QTimer *m_positionTimer = nullptr;
    QDBusPendingCallWatcher *m_posWatcher = nullptr;
};

#endif // MEDIACONTROLLER_H
