/**
 * @file ravencontroller.h
 * @brief Interfaz de enlace C++/Qt para el control del motor Raven Tiling vía D-Bus.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

#pragma once

#include <QObject>
#include <QString>
#include <QDBusInterface>
#include <QDBusReply>
#include <QTimer>
#include <QtQml/qqmlregistration.h>

/**
 * @class RavenController
 * @brief Controlador singleton C++ que orquesta la comunicación entre la interfaz Plasmoide (QML) y el demonio Rust de Raven Tiling.
 *
 * Expone un conjunto reactivo de propiedades (Q_PROPERTY) y métodos invocables (Q_SLOTS)
 * para inspeccionar y mutar en tiempo real la topología de ventanas, los márgenes (gaps),
 * el ratio maestro, la distribución espacial activa y la migración entre pantallas y escritorios virtuales.
 */
class RavenController : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

    /** @brief Indica si el motor de mosaico interactivo está activo o si el escritorio opera en modo flotante clásico. */
    Q_PROPERTY(bool tilingEnabled READ isTilingEnabled WRITE setTilingEnabled NOTIFY tilingEnabledChanged)
    
    /** @brief Nombre identificador del algoritmo de disposición activo en el espacio de trabajo actual (ej. 'raven', 'tall', 'strict_dwindle'). */
    Q_PROPERTY(QString currentLayout READ currentLayout NOTIFY currentLayoutChanged)
    
    /** @brief Separación perimetral interna e inter-ventanas (gaps) configurada en píxeles. */
    Q_PROPERTY(int defaultGaps READ defaultGaps NOTIFY defaultGapsChanged)
    
    /** @brief Proporción del área maestra principal respecto a los paneles secundarios (0.30 a 0.85). */
    Q_PROPERTY(double masterRatio READ masterRatio NOTIFY masterRatioChanged)
    
    /** @brief Cantidad total de monitores físicos detectados en la sesión activa de Wayland/KWin. */
    Q_PROPERTY(int monitorCount READ monitorCount NOTIFY monitorCountChanged)
    
    /** @brief Índice del escritorio virtual activo (1-indexado). */
    Q_PROPERTY(int currentDesktop READ currentDesktop NOTIFY desktopStatusChanged)
    
    /** @brief Índice circular del escritorio virtual anterior. */
    Q_PROPERTY(int prevDesktop READ prevDesktop NOTIFY desktopStatusChanged)
    
    /** @brief Índice circular del escritorio virtual siguiente. */
    Q_PROPERTY(int nextDesktop READ nextDesktop NOTIFY desktopStatusChanged)
    
    /** @brief Etiqueta textual localizada del escritorio virtual en foco. */
    Q_PROPERTY(QString desktopStatus READ desktopStatus NOTIFY desktopStatusChanged)

public:
    /**
     * @brief Constructor principal. Inicializa las conexiones D-Bus y el temporizador de sondeo de estado.
     * @param parent Puntero opcional al objeto padre Qt.
     */
    explicit RavenController(QObject *parent = nullptr);
    
    /** @brief Destructor virtual por defecto. */
    ~RavenController() override = default;

    /** @return true si el mosaico está habilitado; false en modo flotante. */
    bool isTilingEnabled() const { return m_tilingEnabled; }
    
    /** @brief Activa o desactiva el mosaico emitiendo la orden al motor Rust. */
    void setTilingEnabled(bool enabled);

    /** @return Identificador del layout activo. */
    QString currentLayout() const { return m_currentLayout; }
    
    /** @return Tamaño actual de los márgenes en píxeles. */
    int defaultGaps() const { return m_defaultGaps; }
    
    /** @return Relación de aspecto del área maestra. */
    double masterRatio() const { return m_masterRatio; }
    
    /** @return Número de ventanas activas gestionadas en el espacio actual. */
    int activeWindowCount() const { return m_activeWindowCount; }
    
    /** @return Número de pantallas conectadas. */
    int monitorCount() const { return m_monitorCount; }
    
    /** @return Número de escritorio virtual actual. */
    int currentDesktop() const { return m_currentDesktop; }
    
    /** @return Número de escritorio virtual anterior. */
    int prevDesktop() const { return m_prevDesktop; }
    
    /** @return Número de escritorio virtual siguiente. */
    int nextDesktop() const { return m_nextDesktop; }
    
    /** @return Descripción del estado del escritorio. */
    QString desktopStatus() const { return m_desktopStatus; }

public Q_SLOTS:
    /** @brief Sincroniza y actualiza todas las propiedades locales consultando al demonio Rust por D-Bus. */
    void refreshState();
    
    /** @brief Alterna el estado de mosaico (activar/desactivar tiling). */
    void toggleTiling();
    
    /** @brief Cicla secuencialmente al siguiente algoritmo de disposición disponible. */
    void cycleLayout();
    
    /**
     * @brief Establece un algoritmo de disposición específico de forma inmediata.
     * @param layoutName Nombre del algoritmo (ej. 'raven', 'tall', 'monocle', 'strict_dwindle', 'inverted_strict_dwindle', 'divisor').
     */
    void setLayout(const QString &layoutName);
    
    /** @brief Conmuta el estado de flotación libre de la ventana en foco. */
    void toggleFloating();
    
    /**
     * @brief Incrementa o reduce los márgenes entre ventanas en tiempo real.
     * @param delta Variación en píxeles (positivo para aumentar, negativo para reducir).
     */
    void incrementGaps(int delta);
    
    /** @brief Incrementa la capacidad del área maestra en layouts de columna (Tall). */
    void incrementMaster();
    
    /** @brief Reduce la capacidad del área maestra en layouts de columna (Tall). */
    void decrementMaster();
    
    /** @brief Aumenta la proporción del área maestra en 5%. */
    void increaseRatio();
    
    /** @brief Reduce la proporción del área maestra en 5%. */
    void decreaseRatio();
    
    /** @brief Intercambia la ventana activa con la posición de la ventana anterior en el árbol. */
    void swapPrev();
    
    /** @brief Intercambia la ventana activa con la posición de la ventana siguiente en el árbol. */
    void swapNext();
    
    /** @brief Traslada el foco visual a la ventana anterior. */
    void focusPrev();
    
    /** @brief Traslada el foco visual a la ventana siguiente. */
    void focusNext();
    
    /** @brief Migra la ventana activa al siguiente monitor físico disponible. */
    void migrateActiveToScreen();
    
    /** @brief Migra la ventana activa al monitor físico anterior. */
    void migrateActiveToPrevScreen();
    
    /** @brief Desplaza la ventana activa al siguiente escritorio virtual. */
    void migrateActiveToDesktop();
    
    /** @brief Desplaza la ventana activa al escritorio virtual anterior. */
    void migrateActiveToPrevDesktop();
    
    /** @brief Despliega el Centro de Control Gráfico nativo de Raven (Raven GUI). */
    void openControlCenter();

Q_SIGNALS:
    /** @brief Emitida cuando cambia el estado de activación del mosaico. */
    void tilingEnabledChanged();
    
    /** @brief Emitida al modificarse el algoritmo de disposición activo. */
    void currentLayoutChanged();
    
    /** @brief Emitida cuando se alteran los márgenes perimetrales. */
    void defaultGapsChanged();
    
    /** @brief Emitida cuando varía la proporción del área maestra. */
    void masterRatioChanged();
    
    /** @brief Emitida al cambiar la cantidad de ventanas administradas. */
    void activeWindowCountChanged();
    
    /** @brief Emitida cuando se conecta o desconecta una pantalla. */
    void monitorCountChanged();
    
    /** @brief Emitida al transicionar de escritorio virtual. */
    void desktopStatusChanged();

private:
    /**
     * @brief Despacha un comando de acción D-Bus sin parámetros y retransmite directivas en tiempo real.
     * @param action Nombre del método en la interfaz org.kde.raven.Events.
     */
    void sendDbusAction(const QString &action);
    
    /**
     * @brief Despacha un comando de acción D-Bus con un argumento entero.
     * @param action Nombre del método en la interfaz org.kde.raven.Events.
     * @param arg Valor entero del parámetro.
     */
    void sendDbusActionWithArg(const QString &action, int arg);

    bool m_tilingEnabled = true;                 ///< Estado interno de mosaico.
    QString m_currentLayout = QStringLiteral("raven"); ///< Algoritmo activo.
    int m_defaultGaps = 10;                     ///< Gaps actuales.
    double m_masterRatio = 0.50;                ///< Ratio maestro actual.
    int m_activeWindowCount = 0;                ///< Número de ventanas en mosaico.
    int m_monitorCount = 1;                     ///< Monitores detectados.
    int m_currentDesktop = 1;                   ///< Escritorio activo.
    int m_prevDesktop = 1;                      ///< Escritorio previo.
    int m_nextDesktop = 1;                      ///< Escritorio próximo.
    QString m_desktopStatus = QStringLiteral("Escritorio 1"); ///< Texto descriptivo del escritorio.

    QDBusInterface *m_dbusInterface = nullptr;  ///< Interfaz D-Bus persistente hacia org.kde.raven.Daemon.
    QTimer *m_pollTimer = nullptr;              ///< Temporizador de sondeo periódico de estado.
};
