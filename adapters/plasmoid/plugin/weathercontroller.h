/**
 * @file weathercontroller.h
 * @brief Controlador meteorológico con geolocalización IP y pronóstico en tiempo real para KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

#ifndef WEATHERCONTROLLER_H
#define WEATHERCONTROLLER_H

#include <QObject>
#include <QString>
#include <QTimer>
#include <QNetworkAccessManager>
#include <qqmlintegration.h>

/**
 * @class WeatherController
 * @brief Cliente meteorológico asíncrono para Raven Hub.
 *
 * Realiza consultas automáticas cada 30 minutos a:
 * 1. `ip-api.com` para geolocalización automática basada en IP (latitud, longitud y ciudad).
 * 2. `open-meteo.com` para obtención de temperatura actual, humedad, velocidad de viento y código WMO de condición.
 * Mapea los códigos internacionales WMO a cadenas traducidas en español y nombres de iconos compatibles con Kirigami.
 */
class WeatherController : public QObject
{
    Q_OBJECT
    QML_ELEMENT

    /** @brief true si los datos meteorológicos fueron recibidos y están listos para mostrar. */
    Q_PROPERTY(bool ready READ ready NOTIFY weatherChanged)
    
    /** @brief true mientras se realiza la petición de red HTTP. */
    Q_PROPERTY(bool loading READ loading NOTIFY weatherChanged)
    
    /** @brief Temperatura actual formateada (ej. '24°C'). */
    Q_PROPERTY(QString temperature READ temperature NOTIFY weatherChanged)
    
    /** @brief Descripción textual en español del clima (ej. 'Soleado', 'Lluvia moderada'). */
    Q_PROPERTY(QString condition READ condition NOTIFY weatherChanged)
    
    /** @brief Nombre del icono de Plasma/Kirigami correspondiente (ej. 'weather-clear', 'weather-storm'). */
    Q_PROPERTY(QString iconName READ iconName NOTIFY weatherChanged)
    
    /** @brief Nombre de la ciudad o localidad geográfica detectada. */
    Q_PROPERTY(QString location READ location NOTIFY weatherChanged)
    
    /** @brief Porcentaje de humedad relativa (ej. '65%'). */
    Q_PROPERTY(QString humidity READ humidity NOTIFY weatherChanged)
    
    /** @brief Velocidad del viento en kilómetros por hora (ej. '12 km/h'). */
    Q_PROPERTY(QString windSpeed READ windSpeed NOTIFY weatherChanged)

public:
    /**
     * @brief Constructor principal. Configura el temporizador de refresco periódico (30 min).
     * @param parent Puntero opcional al objeto padre Qt.
     */
    explicit WeatherController(QObject *parent = nullptr);

    /** @return true si los datos están disponibles. */
    bool ready() const { return m_ready; }
    
    /** @return true si hay una petición en curso. */
    bool loading() const { return m_loading; }
    
    /** @return Temperatura formateada. */
    QString temperature() const { return m_temperature; }
    
    /** @return Descripción del estado meteorológico. */
    QString condition() const { return m_condition; }
    
    /** @return Nombre del icono del sistema. */
    QString iconName() const { return m_iconName; }
    
    /** @return Localidad detectada. */
    QString location() const { return m_location; }
    
    /** @return Humedad relativa. */
    QString humidity() const { return m_humidity; }
    
    /** @return Velocidad del viento. */
    QString windSpeed() const { return m_windSpeed; }

    /** @brief Fuerza una consulta manual inmediata a los servicios meteorológicos. */
    Q_INVOKABLE void refresh();

signals:
    /** @brief Emitida cuando se reciben nuevos datos del clima o cambia el estado de carga. */
    void weatherChanged();

private slots:
    /** @brief Consulta ip-api.com para determinar las coordenadas geográficas de la conexión. */
    void fetchGeoLocation();
    
    /**
     * @brief Descarga el reporte meteorológico desde Open-Meteo para las coordenadas provistas.
     * @param latitude Latitud geográfica.
     * @param longitude Longitud geográfica.
     * @param city Nombre de la ciudad.
     */
    void fetchWeatherData(double latitude, double longitude, const QString &city);

private:
    /**
     * @brief Traduce el código numérico de clima WMO a una descripción en español.
     * @param code Código WMO (0 a 99).
     * @return Cadena descriptiva (ej. "Despejado").
     */
    QString weatherCodeToCondition(int code) const;
    
    /**
     * @brief Traduce el código numérico de clima WMO al nombre del icono Freedesktop adecuado.
     * @param code Código WMO.
     * @param isDay true si es de día; false para variantes nocturnas.
     * @return Identificador del icono (ej. "weather-clear-night").
     */
    QString weatherCodeToIcon(int code, bool isDay) const;

    bool m_ready = false;                                      ///< Datos listos.
    bool m_loading = false;                                    ///< En carga.
    QString m_temperature = QStringLiteral("--°C");            ///< Temperatura.
    QString m_condition = QStringLiteral("Cargando clima...");  ///< Condición.
    QString m_iconName = QStringLiteral("weather-clouds");     ///< Icono.
    QString m_location = QStringLiteral("Ubicación actual");   ///< Localidad.
    QString m_humidity = QStringLiteral("--%");                ///< Humedad.
    QString m_windSpeed = QStringLiteral("-- km/h");           ///< Viento.

    QNetworkAccessManager *m_nam = nullptr;                    ///< Gestor de red.
    QTimer *m_refreshTimer = nullptr;                          ///< Temporizador de 30 min.
};

#endif // WEATHERCONTROLLER_H
