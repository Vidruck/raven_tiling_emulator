#include "colorextractor.h"
#include <QImage>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QtConcurrent/QtConcurrentRun>
#include <QFutureWatcher>
#include <QFile>

QHash<QString, QPair<QColor, QColor>> ColorExtractor::s_colorCache;

ColorExtractor::ColorExtractor(QObject *parent) : QObject(parent)
{
    m_nam = new QNetworkAccessManager(this);
}

void ColorExtractor::setSource(const QUrl &url)
{
    if (m_source == url) return;
    m_source = url;
    m_ready = false;
    emit sourceChanged();
    emit colorsReady(); // reset visual state

    if (url.isEmpty()) return;

    QString cacheKey = url.toString();

    // Cache hit — inmediato, sin I/O
    if (s_colorCache.contains(cacheKey)) {
        auto cached = s_colorCache.value(cacheKey);
        m_dominant = cached.first;
        m_accent = cached.second;
        m_ready = true;
        emit colorsReady();
        return;
    }

    // Cargar imagen (local o remota)
    if (url.isLocalFile()) {
        QFile file(url.toLocalFile());
        if (file.open(QIODevice::ReadOnly)) {
            extractFromImage(file.readAll(), cacheKey);
        }
    } else {
        QNetworkReply *reply = m_nam->get(QNetworkRequest(url));
        connect(reply, &QNetworkReply::finished, this, [this, reply, cacheKey]() {
            if (reply->error() == QNetworkReply::NoError) {
                extractFromImage(reply->readAll(), cacheKey);
            }
            reply->deleteLater();
        });
    }
}

void ColorExtractor::extractFromImage(const QByteArray &data, const QString &cacheKey)
{
    // Ejecutar en hilo secundario — NUNCA bloquea el event loop de Plasma
    auto *watcher = new QFutureWatcher<QPair<QColor, QColor>>(this);
    connect(watcher, &QFutureWatcher<QPair<QColor, QColor>>::finished, this,
            [this, watcher, cacheKey]() {
        auto result = watcher->result();
        m_dominant = result.first;
        m_accent = result.second;
        m_ready = true;
        s_colorCache.insert(cacheKey, result);
        emit colorsReady();
        watcher->deleteLater();
    });

    watcher->setFuture(QtConcurrent::run([data]() -> QPair<QColor, QColor> {
        QImage img;
        if (!img.loadFromData(data)) {
            return {QColor(0x1a, 0x1a, 0x2e), QColor(0x6c, 0x5c, 0xe7)};
        }

        // Escalar a miniatura para performance
        QImage thumb = img.scaled(32, 32, Qt::IgnoreAspectRatio, Qt::FastTransformation);

        // Algoritmo simplificado
        int rSum = 0, gSum = 0, bSum = 0;
        int rSat = 0, gSat = 0, bSat = 0;
        int count = 0, satCount = 0;

        for (int y = 0; y < thumb.height(); ++y) {
            for (int x = 0; x < thumb.width(); ++x) {
                QColor c(thumb.pixel(x, y));
                rSum += c.red(); gSum += c.green(); bSum += c.blue();
                count++;
                if (c.saturation() > 60) {
                    rSat += c.red(); gSat += c.green(); bSat += c.blue();
                    satCount++;
                }
            }
        }
        
        if (count == 0) count = 1;

        QColor dominant(rSum / count, gSum / count, bSum / count);
        QColor accent = satCount > 0
            ? QColor(rSat / satCount, gSat / satCount, bSat / satCount)
            : dominant.lighter(140);

        return {dominant, accent};
    }));
}
