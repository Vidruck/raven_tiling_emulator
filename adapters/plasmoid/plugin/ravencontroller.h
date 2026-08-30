#pragma once

#include <QObject>
#include <QString>
#include <QDBusInterface>
#include <QDBusReply>
#include <QTimer>
#include <QtQml/qqmlregistration.h>

/**
 * @brief Controlador C++ para comunicarse con el demonio Raven Tiling vía D-Bus.
 * Expone propiedades reactivas y métodos invocables para QML.
 */
class RavenController : public QObject
{
    Q_OBJECT
    QML_ELEMENT
    QML_SINGLETON

    Q_PROPERTY(bool tilingEnabled READ isTilingEnabled WRITE setTilingEnabled NOTIFY tilingEnabledChanged)
    Q_PROPERTY(QString currentLayout READ currentLayout NOTIFY currentLayoutChanged)
    Q_PROPERTY(int defaultGaps READ defaultGaps NOTIFY defaultGapsChanged)
    Q_PROPERTY(double masterRatio READ masterRatio NOTIFY masterRatioChanged)
    Q_PROPERTY(int monitorCount READ monitorCount NOTIFY monitorCountChanged)
    Q_PROPERTY(int currentDesktop READ currentDesktop NOTIFY desktopStatusChanged)
    Q_PROPERTY(int prevDesktop READ prevDesktop NOTIFY desktopStatusChanged)
    Q_PROPERTY(int nextDesktop READ nextDesktop NOTIFY desktopStatusChanged)
    Q_PROPERTY(QString desktopStatus READ desktopStatus NOTIFY desktopStatusChanged)

public:
    explicit RavenController(QObject *parent = nullptr);
    ~RavenController() override = default;

    bool isTilingEnabled() const { return m_tilingEnabled; }
    void setTilingEnabled(bool enabled);

    QString currentLayout() const { return m_currentLayout; }
    int defaultGaps() const { return m_defaultGaps; }
    double masterRatio() const { return m_masterRatio; }
    int activeWindowCount() const { return m_activeWindowCount; }
    int monitorCount() const { return m_monitorCount; }
    int currentDesktop() const { return m_currentDesktop; }
    int prevDesktop() const { return m_prevDesktop; }
    int nextDesktop() const { return m_nextDesktop; }
    QString desktopStatus() const { return m_desktopStatus; }

public Q_SLOTS:
    void refreshState();
    void toggleTiling();
    void cycleLayout();
    void setLayout(const QString &layoutName);
    void toggleFloating();
    void incrementGaps(int delta);
    void incrementMaster();
    void decrementMaster();
    void increaseRatio();
    void decreaseRatio();
    void swapPrev();
    void swapNext();
    void focusPrev();
    void focusNext();
    void migrateActiveToScreen();
    void migrateActiveToPrevScreen();
    void migrateActiveToDesktop();
    void migrateActiveToPrevDesktop();
    void openControlCenter();

Q_SIGNALS:
    void tilingEnabledChanged();
    void currentLayoutChanged();
    void defaultGapsChanged();
    void masterRatioChanged();
    void activeWindowCountChanged();
    void monitorCountChanged();
    void desktopStatusChanged();

private:
    void sendDbusAction(const QString &action);
    void sendDbusActionWithArg(const QString &action, int arg);

    bool m_tilingEnabled = true;
    QString m_currentLayout = QStringLiteral("raven");
    int m_defaultGaps = 10;
    double m_masterRatio = 0.50;
    int m_activeWindowCount = 0;
    int m_monitorCount = 1;
    int m_currentDesktop = 1;
    int m_prevDesktop = 1;
    int m_nextDesktop = 1;
    QString m_desktopStatus = QStringLiteral("Escritorio 1");

    QDBusInterface *m_dbusInterface = nullptr;
    QTimer *m_pollTimer = nullptr;
};
