#include "weathercontroller.h"
#include <QNetworkRequest>
#include <QNetworkReply>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QUrl>
#include <QUrlQuery>

WeatherController::WeatherController(QObject *parent)
    : QObject(parent)
{
    m_nam = new QNetworkAccessManager(this);

    m_refreshTimer = new QTimer(this);
    m_refreshTimer->setInterval(30 * 60 * 1000); // 30 minutos
    connect(m_refreshTimer, &QTimer::timeout, this, &WeatherController::fetchGeoLocation);

    // Consulta inicial diferida para no bloquear inicio
    QTimer::singleShot(1000, this, &WeatherController::fetchGeoLocation);
    m_refreshTimer->start();
}

void WeatherController::refresh()
{
    fetchGeoLocation();
}

void WeatherController::fetchGeoLocation()
{
    m_loading = true;
    emit weatherChanged();

    // Query HTTPS geolocation endpoint
    QUrl url(QStringLiteral("https://ipwho.is/"));
    QNetworkRequest request(url);
    request.setHeader(QNetworkRequest::UserAgentHeader, QStringLiteral("RavenLauncher/1.0"));

    QNetworkReply *reply = m_nam->get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply]() {
        if (reply->error() == QNetworkReply::NoError) {
            QByteArray data = reply->readAll();
            QJsonDocument doc = QJsonDocument::fromJson(data);
            if (doc.isObject()) {
                QJsonObject obj = doc.object();
                if (obj.value(QStringLiteral("success")).toBool(true) && obj.contains(QStringLiteral("latitude"))) {
                    double lat = obj.value(QStringLiteral("latitude")).toDouble();
                    double lon = obj.value(QStringLiteral("longitude")).toDouble();
                    QString city = obj.value(QStringLiteral("city")).toString();
                    QString country = obj.value(QStringLiteral("country_code")).toString();
                    QString locName = city.isEmpty() ? country : (city + QStringLiteral(", ") + country);

                    fetchWeatherData(lat, lon, locName);
                    reply->deleteLater();
                    return;
                }
            }
        }
        
        // Fallback to http://ip-api.com if needed
        QUrl fallbackUrl(QStringLiteral("http://ip-api.com/json/?fields=status,city,countryCode,lat,lon"));
        QNetworkRequest fbRequest(fallbackUrl);
        fbRequest.setHeader(QNetworkRequest::UserAgentHeader, QStringLiteral("RavenLauncher/1.0"));
        QNetworkReply *fbReply = m_nam->get(fbRequest);
        connect(fbReply, &QNetworkReply::finished, this, [this, fbReply]() {
            if (fbReply->error() == QNetworkReply::NoError) {
                QByteArray fbData = fbReply->readAll();
                QJsonDocument fbDoc = QJsonDocument::fromJson(fbData);
                if (fbDoc.isObject()) {
                    QJsonObject fbObj = fbDoc.object();
                    if (fbObj.value(QStringLiteral("status")).toString() == QStringLiteral("success")) {
                        double lat = fbObj.value(QStringLiteral("lat")).toDouble();
                        double lon = fbObj.value(QStringLiteral("lon")).toDouble();
                        QString city = fbObj.value(QStringLiteral("city")).toString();
                        QString country = fbObj.value(QStringLiteral("countryCode")).toString();
                        QString locName = city.isEmpty() ? country : (city + QStringLiteral(", ") + country);

                        fetchWeatherData(lat, lon, locName);
                        fbReply->deleteLater();
                        return;
                    }
                }
            }
            fetchWeatherData(19.4326, -99.1332, QStringLiteral("América"));
            fbReply->deleteLater();
        });
        
        reply->deleteLater();
    });
}

void WeatherController::fetchWeatherData(double lat, double lon, const QString &city)
{
    // Usar Open-Meteo API pública y sin API-Key
    QUrl url(QStringLiteral("https://api.open-meteo.com/v1/forecast"));
    QUrlQuery query;
    query.addQueryItem(QStringLiteral("latitude"), QString::number(lat, 'f', 4));
    query.addQueryItem(QStringLiteral("longitude"), QString::number(lon, 'f', 4));
    query.addQueryItem(QStringLiteral("current"), QStringLiteral("temperature_2m,relative_humidity_2m,is_day,weather_code,wind_speed_10m"));
    query.addQueryItem(QStringLiteral("timezone"), QStringLiteral("auto"));
    url.setQuery(query);

    QNetworkRequest request(url);
    request.setHeader(QNetworkRequest::UserAgentHeader, QStringLiteral("RavenLauncher/1.0"));

    QNetworkReply *reply = m_nam->get(request);
    connect(reply, &QNetworkReply::finished, this, [this, reply, city]() {
        m_loading = false;
        if (reply->error() == QNetworkReply::NoError) {
            QByteArray data = reply->readAll();
            QJsonDocument doc = QJsonDocument::fromJson(data);
            if (doc.isObject()) {
                QJsonObject obj = doc.object();
                QJsonObject current = obj.value(QStringLiteral("current")).toObject();

                double temp = current.value(QStringLiteral("temperature_2m")).toDouble();
                int code = current.value(QStringLiteral("weather_code")).toInt();
                int isDay = current.value(QStringLiteral("is_day")).toInt(1);
                int humidity = current.value(QStringLiteral("relative_humidity_2m")).toInt();
                double wind = current.value(QStringLiteral("wind_speed_10m")).toDouble();

                m_temperature = QStringLiteral("%1°C").arg(qRound(temp));
                m_condition = weatherCodeToCondition(code);
                m_iconName = weatherCodeToIcon(code, isDay == 1);
                m_location = city;
                m_humidity = QStringLiteral("%1%").arg(humidity);
                m_windSpeed = QStringLiteral("%1 km/h").arg(qRound(wind));
                m_ready = true;

                emit weatherChanged();
                reply->deleteLater();
                return;
            }
        }
        reply->deleteLater();
        emit weatherChanged();
    });
}

QString WeatherController::weatherCodeToCondition(int code) const
{
    switch (code) {
    case 0: return QStringLiteral("Despejado");
    case 1: return QStringLiteral("Mayormente despejado");
    case 2: return QStringLiteral("Parcialmente nublado");
    case 3: return QStringLiteral("Nublado");
    case 45:
    case 48: return QStringLiteral("Niebla");
    case 51:
    case 53:
    case 55: return QStringLiteral("Llovizna");
    case 61:
    case 63:
    case 65: return QStringLiteral("Lluvia");
    case 71:
    case 73:
    case 75: return QStringLiteral("Nieve");
    case 80:
    case 81:
    case 82: return QStringLiteral("Chubascos");
    case 95:
    case 96:
    case 99: return QStringLiteral("Tormenta eléctrica");
    default: return QStringLiteral("Despejado");
    }
}

QString WeatherController::weatherCodeToIcon(int code, bool isDay) const
{
    switch (code) {
    case 0:
        return isDay ? QStringLiteral("weather-clear") : QStringLiteral("weather-clear-night");
    case 1:
        return isDay ? QStringLiteral("weather-few-clouds") : QStringLiteral("weather-few-clouds-night");
    case 2:
        return isDay ? QStringLiteral("weather-clouds") : QStringLiteral("weather-clouds-night");
    case 3:
        return QStringLiteral("weather-overcast");
    case 45:
    case 48:
        return QStringLiteral("weather-fog");
    case 51:
    case 53:
    case 55:
    case 61:
    case 63:
    case 65:
        return QStringLiteral("weather-showers");
    case 71:
    case 73:
    case 75:
        return QStringLiteral("weather-snow");
    case 80:
    case 81:
    case 82:
        return QStringLiteral("weather-showers-scattered");
    case 95:
    case 96:
    case 99:
        return QStringLiteral("weather-storm");
    default:
        return isDay ? QStringLiteral("weather-clear") : QStringLiteral("weather-clear-night");
    }
}
