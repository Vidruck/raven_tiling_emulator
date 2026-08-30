#ifndef WEATHERCONTROLLER_H
#define WEATHERCONTROLLER_H

#include <QObject>
#include <QString>
#include <QTimer>
#include <QNetworkAccessManager>
#include <qqmlintegration.h>

class WeatherController : public QObject
{
    Q_OBJECT
    QML_ELEMENT

    Q_PROPERTY(bool ready READ ready NOTIFY weatherChanged)
    Q_PROPERTY(bool loading READ loading NOTIFY weatherChanged)
    Q_PROPERTY(QString temperature READ temperature NOTIFY weatherChanged)
    Q_PROPERTY(QString condition READ condition NOTIFY weatherChanged)
    Q_PROPERTY(QString iconName READ iconName NOTIFY weatherChanged)
    Q_PROPERTY(QString location READ location NOTIFY weatherChanged)
    Q_PROPERTY(QString humidity READ humidity NOTIFY weatherChanged)
    Q_PROPERTY(QString windSpeed READ windSpeed NOTIFY weatherChanged)

public:
    explicit WeatherController(QObject *parent = nullptr);

    bool ready() const { return m_ready; }
    bool loading() const { return m_loading; }
    QString temperature() const { return m_temperature; }
    QString condition() const { return m_condition; }
    QString iconName() const { return m_iconName; }
    QString location() const { return m_location; }
    QString humidity() const { return m_humidity; }
    QString windSpeed() const { return m_windSpeed; }

    Q_INVOKABLE void refresh();

signals:
    void weatherChanged();

private slots:
    void fetchGeoLocation();
    void fetchWeatherData(double latitude, double longitude, const QString &city);

private:
    QString weatherCodeToCondition(int code) const;
    QString weatherCodeToIcon(int code, bool isDay) const;

    bool m_ready = false;
    bool m_loading = false;
    QString m_temperature = QStringLiteral("--°C");
    QString m_condition = QStringLiteral("Cargando clima...");
    QString m_iconName = QStringLiteral("weather-clouds");
    QString m_location = QStringLiteral("Ubicación actual");
    QString m_humidity = QStringLiteral("--%");
    QString m_windSpeed = QStringLiteral("-- km/h");

    QNetworkAccessManager *m_nam = nullptr;
    QTimer *m_refreshTimer = nullptr;
};

#endif // WEATHERCONTROLLER_H
