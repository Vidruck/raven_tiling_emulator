#ifndef GPU_STATS_H
#define GPU_STATS_H

#include <QObject>
#include <QString>
#include <QTimer>
#include <QProcess>
#include <qqmlintegration.h>

/**
 * @class GPUStats
 * @brief Singleton C++ that probes the host GPU for usage, VRAM and temperature.
 * Detects available command‑line utilities (nvidia‑smi, radeontop, intel_gpu_top, etc.)
 * and parses their output. If no utility is found the component reports unavailable
 * and the UI graph remains hidden.
 */
class GPUStats : public QObject {
    Q_OBJECT
    QML_SINGLETON
    QML_ELEMENT

    /** @brief Flag indicating whether a supported GPU utility was found. */
    Q_PROPERTY(bool available READ isAvailable NOTIFY availabilityChanged)

    /** @brief GPU utilisation percentage (0‑100). */
    Q_PROPERTY(int usage READ usage NOTIFY statsChanged)

    /** @brief VRAM usage percentage (0‑100). */
    Q_PROPERTY(int vramUsage READ vramUsage NOTIFY statsChanged)

    /** @brief Average temperature in Celsius (‑1 if not reported). */
    Q_PROPERTY(int temperature READ temperature NOTIFY statsChanged)

    /** @brief Activation flag – stops polling when plasmoid is hidden. */
    Q_PROPERTY(bool active READ active WRITE setActive NOTIFY activeChanged)

public:
    explicit GPUStats(QObject *parent = nullptr);
    bool isAvailable() const { return m_available; }
    int usage() const { return m_usage; }
    int vramUsage() const { return m_vramUsage; }
    int temperature() const { return m_temperature; }
    bool active() const { return m_active; }
    void setActive(bool act);

    Q_INVOKABLE void refresh();

signals:
    void availabilityChanged();
    void statsChanged();
    void activeChanged();

private slots:
    void updateStats();
    void onProcessFinished(int exitCode, QProcess::ExitStatus exitStatus);

private:
    void detectUtility();
    bool readSysfsStats();
    void parseNvidiaSmi(const QString &output);
    void parseRadeonTop(const QString &output);
    void parseIntelGpuTop(const QString &output);

    bool m_available = false;          ///< true if sysfs or a utility was detected
    bool m_useSysfs = false;           ///< zero-overhead Linux sysfs direct reading
    QString m_sysfsGpuBusy;
    QString m_sysfsVramUsed;
    QString m_sysfsVramTotal;
    QString m_sysfsTemp;

    QString m_utilityPath;             ///< full path to the detected binary
    QTimer *m_timer = nullptr;         ///< polling timer (2 s interval)
    QProcess *m_process = nullptr;     ///< asynchronous non-blocking process
    bool m_active = false;             ///< mirrors SystemStats behaviour
    int m_usage = 0;
    int m_vramUsage = 0;
    int m_temperature = -1;
};

#endif // GPU_STATS_H
