#ifndef COLOREXTRACTOR_H
#define COLOREXTRACTOR_H

#include <QObject>
#include <QColor>
#include <QUrl>
#include <QHash>
#include <qqmlintegration.h>

class QNetworkAccessManager;

class ColorExtractor : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(QUrl source READ source WRITE setSource NOTIFY sourceChanged)
    Q_PROPERTY(QColor dominantColor READ dominantColor NOTIFY colorsReady)
    Q_PROPERTY(QColor accentColor READ accentColor NOTIFY colorsReady)
    Q_PROPERTY(bool ready READ ready NOTIFY colorsReady)

public:
    explicit ColorExtractor(QObject *parent = nullptr);

    QUrl source() const { return m_source; }
    void setSource(const QUrl &url);
    QColor dominantColor() const { return m_dominant; }
    QColor accentColor() const { return m_accent; }
    bool ready() const { return m_ready; }

signals:
    void sourceChanged();
    void colorsReady();

private:
    void extractFromImage(const QByteArray &data, const QString &cacheKey);

    QUrl m_source;
    QColor m_dominant{0x1a, 0x1a, 0x2e};
    QColor m_accent{0x6c, 0x5c, 0xe7};
    bool m_ready = false;
    QNetworkAccessManager *m_nam = nullptr;

    // Cache para evitar re-cálculos
    static QHash<QString, QPair<QColor, QColor>> s_colorCache;
};

#endif
