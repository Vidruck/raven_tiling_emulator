/**
 * @file colorextractor.h
 * @brief Extractor cromático asíncrono para ambientación dinámica de portadas de álbumes.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

#ifndef COLOREXTRACTOR_H
#define COLOREXTRACTOR_H

#include <QObject>
#include <QColor>
#include <QUrl>
#include <QHash>
#include <qqmlintegration.h>

class QNetworkAccessManager;

/**
 * @class ColorExtractor
 * @brief Analizador cromático C++ para extracción de paletas adaptativas en tiempo real.
 *
 * Descarga o carga imágenes locales de portadas de álbumes (`artUrl`) y ejecuta
 * un muestreo de píxeles para derivar:
 * - Color dominante de fondo para degradados glassmorphic.
 * - Color de acento vibrante (alta saturación) para botones y barras de progreso.
 * Cuenta con una caché estática en memoria para eliminar latencia y consumo de CPU en pistas repetidas.
 */
class ColorExtractor : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    
    /** @brief URL remota (http/https) o archivo local (file://) de la imagen a analizar. */
    Q_PROPERTY(QUrl source READ source WRITE setSource NOTIFY sourceChanged)
    
    /** @brief Color de fondo predominante calculado a partir de la imagen. */
    Q_PROPERTY(QColor dominantColor READ dominantColor NOTIFY colorsReady)
    
    /** @brief Color de realce vibrante extraído de los tonos más saturados de la carátula. */
    Q_PROPERTY(QColor accentColor READ accentColor NOTIFY colorsReady)
    
    /** @brief true cuando la extracción cromática finalizó y los colores están disponibles. */
    Q_PROPERTY(bool ready READ ready NOTIFY colorsReady)

public:
    /**
     * @brief Constructor principal.
     * @param parent Puntero opcional al objeto padre Qt.
     */
    explicit ColorExtractor(QObject *parent = nullptr);

    /** @return URL fuente actual. */
    QUrl source() const { return m_source; }
    
    /**
     * @brief Asigna una nueva URL fuente y dispara el proceso asíncrono de descarga/análisis.
     * @param url Dirección de la imagen.
     */
    void setSource(const QUrl &url);
    
    /** @return Color dominante extraído. */
    QColor dominantColor() const { return m_dominant; }
    
    /** @return Color de acento extraído. */
    QColor accentColor() const { return m_accent; }
    
    /** @return true si los colores han sido calculados. */
    bool ready() const { return m_ready; }

signals:
    /** @brief Emitida cuando se modifica la propiedad source. */
    void sourceChanged();
    
    /** @brief Emitida cuando el cálculo cromático termina exitosamente. */
    void colorsReady();

private:
    /**
     * @brief Procesa el buffer de bytes de la imagen, calcula histogramas y deriva los colores clave.
     * @param data Bytes de la imagen (PNG, JPG, WebP).
     * @param cacheKey Clave única para indexar en la caché en memoria.
     */
    void extractFromImage(const QByteArray &data, const QString &cacheKey);

    QUrl m_source;                                 ///< URL fuente.
    QColor m_dominant{0x1a, 0x1a, 0x2e};          ///< Color dominante por defecto.
    QColor m_accent{0x6c, 0x5c, 0xe7};            ///< Color de acento por defecto.
    bool m_ready = false;                          ///< Bandera de completitud.
    QNetworkAccessManager *m_nam = nullptr;        ///< Gestor de red para descargas HTTP.

    static QHash<QString, QPair<QColor, QColor>> s_colorCache; ///< Caché estática global en memoria.
};

#endif // COLOREXTRACTOR_H
