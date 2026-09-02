/**
 * @file mediacontroller.h
 * @brief Controlador multimedia compatible con la especificación MPRIS2 para KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

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

/**
 * @class MediaController
 * @brief Orquestador multimedia de alto rendimiento para el widget de reproducción y ecualizador de Raven Hub.
 *
 * Implementa el estándar `org.mpris.MediaPlayer2.Player` sobre D-Bus para interactuar
 * de forma reactiva con Spotify, YouTube, reproductores web, VLC, Elisa, Zuno y navegadores Chromium/Firefox.
 *
 * Soporta extrapolación temporal suave a 1 Hz, sincronización asíncrona no bloqueante
 * para evitar congelamientos en la interfaz y detección automática de nombres de servicio.
 */
class MediaController : public QObject
{
    Q_OBJECT
    QML_ELEMENT

    /** @brief Determina si el widget está activo (visible en el plasmoide) para habilitar o suspender timers. */
    Q_PROPERTY(bool active READ active WRITE setActive NOTIFY activeChanged)
    
    /** @brief Indica si existe un reproductor MPRIS2 conectado y disponible en la sesión D-Bus. */
    Q_PROPERTY(bool hasPlayer READ hasPlayer NOTIFY mediaChanged)
    
    /** @brief Nombre amigable de la aplicación que reproduce audio (ej. 'Spotify', 'YouTube / Firefox'). */
    Q_PROPERTY(QString playerName READ playerName NOTIFY mediaChanged)
    
    /** @brief Título de la pista o video actual. */
    Q_PROPERTY(QString trackTitle READ trackTitle NOTIFY mediaChanged)
    
    /** @brief Artista o creador del contenido en reproducción. */
    Q_PROPERTY(QString artist READ artist NOTIFY mediaChanged)
    
    /** @brief Álbum o lista de reproducción asociada. */
    Q_PROPERTY(QString album READ album NOTIFY mediaChanged)
    
    /** @brief URL o ruta local a la portada del álbum (artUrl). */
    Q_PROPERTY(QString artUrl READ artUrl NOTIFY mediaChanged)
    
    /** @brief Estado de reproducción reportado por MPRIS2 ('Playing', 'Paused', 'Stopped'). */
    Q_PROPERTY(QString playbackStatus READ playbackStatus NOTIFY mediaChanged)
    
    /** @brief Bandera booleana de conveniencia para indicar si hay reproducción activa. */
    Q_PROPERTY(bool isPlaying READ isPlaying NOTIFY mediaChanged)
    
    /** @brief Posición temporal actual de reproducción en segundos. */
    Q_PROPERTY(qint64 position READ position NOTIFY positionChanged)
    
    /** @brief Duración total de la pista en segundos. */
    Q_PROPERTY(qint64 length READ length NOTIFY mediaChanged)
    
    /** @brief Indica si el reproductor soporta avanzar a la siguiente pista. */
    Q_PROPERTY(bool canGoNext READ canGoNext NOTIFY mediaChanged)
    
    /** @brief Indica si el reproductor soporta retroceder a la pista anterior. */
    Q_PROPERTY(bool canGoPrevious READ canGoPrevious NOTIFY mediaChanged)

public:
    /**
     * @brief Constructor principal. Suscribe escuchadores de eventos NameOwnerChanged en D-Bus.
     * @param parent Puntero opcional al objeto padre Qt.
     */
    explicit MediaController(QObject *parent = nullptr);

    /** @return true si hay un reproductor MPRIS2 enlazado. */
    bool hasPlayer() const { return m_hasPlayer; }
    
    /** @return Nombre de la aplicación reproductora. */
    QString playerName() const { return m_playerName; }
    
    /** @return Título de la pista en curso. */
    QString trackTitle() const { return m_trackTitle; }
    
    /** @return Nombre del artista. */
    QString artist() const { return m_artist; }
    
    /** @return Nombre del álbum. */
    QString album() const { return m_album; }
    
    /** @return Ruta o URI de la carátula del álbum. */
    QString artUrl() const { return m_artUrl; }
    
    /** @return Cadena de estado MPRIS ('Playing', 'Paused', 'Stopped'). */
    QString playbackStatus() const { return m_playbackStatus; }
    
    /** @return true si la pista está sonando activamente. */
    bool isPlaying() const { return m_playbackStatus == QStringLiteral("Playing"); }
    
    /** @return Posición actual en segundos. */
    qint64 position() const { return m_position; }
    
    /** @return Longitud total en segundos. */
    qint64 length() const { return m_length; }
    
    /** @return true si está disponible el salto adelante. */
    bool canGoNext() const { return m_canGoNext; }
    
    /** @return true si está disponible el salto atrás. */
    bool canGoPrevious() const { return m_canGoPrevious; }

    /** @brief Inicia o reanuda la reproducción. */
    Q_INVOKABLE void play();
    
    /** @brief Pausa la reproducción actual. */
    Q_INVOKABLE void pause();
    
    /** @brief Alterna entre reproducir y pausar según el estado actual. */
    Q_INVOKABLE void playPause();
    
    /** @brief Salta a la siguiente pista. */
    Q_INVOKABLE void next();
    
    /** @brief Retrocede a la pista anterior o al inicio de la actual. */
    Q_INVOKABLE void previous();
    
    /** @brief Detiene la reproducción por completo. */
    Q_INVOKABLE void stop();
    
    /**
     * @brief Modifica la posición temporal de la pista (Seek).
     * @param positionMs Posición objetivo especificada en milisegundos.
     */
    Q_INVOKABLE void setPosition(qint64 positionMs);
    
    /** @brief Fuerza una re-inspección de los reproductores MPRIS disponibles en el bus de sesión. */
    Q_INVOKABLE void refresh();
    
    /**
     * @brief Formatea una cantidad de segundos a formato legible MM:SS o HH:MM:SS.
     * @param seconds Tiempo en segundos.
     * @return Cadena formateada (ej. "03:45").
     */
    Q_INVOKABLE QString formatTime(qint64 seconds) const;

    /** @return Estado de visibilidad/actividad del widget. */
    bool active() const { return m_active; }
    
    /** @brief Establece si el widget está activo para optimizar consumo de CPU. */
    void setActive(bool active);

signals:
    /** @brief Emitida cuando cambia la bandera de actividad del widget. */
    void activeChanged();
    
    /** @brief Emitida cuando cambian los metadatos de la pista, estado o reproductor. */
    void mediaChanged();
    
    /** @brief Emitida en cada avance de segundo de la barra de progreso. */
    void positionChanged();

private slots:
    /** @brief Maneja la aparición o cierre de servicios en el bus D-Bus. */
    void onNameOwnerChanged(const QString &name, const QString &oldOwner, const QString &newOwner);
    
    /** @brief Procesa cambios en las propiedades de MPRIS2 (metadatos, volumen, playbackStatus). */
    void onPropertiesChanged(const QString &interfaceName, const QVariantMap &changedProperties, const QStringList &invalidatedProperties);
    
    /** @brief Incrementa la extrapolación local y despacha la consulta asíncrona de posición. */
    void updatePosition();
    
    /** @brief Procesa la respuesta asíncrona de posición devuelta por D-Bus sin bloquear la GUI. */
    void onPositionReply(QDBusPendingCallWatcher *watcher);

private:
    /** @brief Escanea todos los servicios 'org.mpris.MediaPlayer2.*' y conecta al que esté sonando o al primero encontrado. */
    void findActivePlayer();
    
    /** @brief Establece las suscripciones D-Bus y lee metadatos iniciales del reproductor especificado. */
    void connectToPlayer(const QString &service);
    
    /** @brief Normaliza el mapa de metadatos xesam/mpris a las propiedades internas. */
    void updateMetadata(const QVariantMap &metadata);
    
    /** @brief Envía la petición asíncrona 'Position' a través de D-Bus con timeout de 500ms. */
    void queryPositionDirect();

    bool m_active = true;                      ///< Bandera de actividad del widget.
    bool m_hasPlayer = false;                  ///< Existencia de reproductor enlazado.
    QString m_currentService;                  ///< Nombre del servicio D-Bus actual.
    QString m_playerName;                      ///< Nombre del reproductor.
    QString m_trackTitle;                      ///< Título de la pista.
    QString m_artist;                          ///< Nombre del artista.
    QString m_album;                           ///< Nombre del álbum.
    QString m_artUrl;                          ///< URL de la carátula.
    QString m_playbackStatus = QStringLiteral("Stopped"); ///< Estado MPRIS2.
    QString m_trackId;                         ///< ID único del track según MPRIS2.
    QString m_trackUrl;                        ///< URL de la fuente de reproducción.
    qint64 m_position = 0;                     ///< Posición en segundos.
    qint64 m_length = 0;                       ///< Duración total en segundos.
    bool m_canGoNext = true;                   ///< Disponibilidad de pista siguiente.
    bool m_canGoPrevious = true;               ///< Disponibilidad de pista anterior.

    QTimer *m_positionTimer = nullptr;         ///< Temporizador de 1s para progreso y extrapolación.
    QDBusPendingCallWatcher *m_posWatcher = nullptr; ///< Observador de llamada asíncrona a D-Bus.
};

#endif // MEDIACONTROLLER_H
