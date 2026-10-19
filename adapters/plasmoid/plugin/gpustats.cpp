#include "gpustats.h"
#include <QFileInfo>
#include <QDir>
#include <QFile>
#include <QStandardPaths>
#include <QProcess>
#include <QRegularExpression>
#include <QDebug>

GPUStats::GPUStats(QObject *parent)
    : QObject(parent),
      m_timer(new QTimer(this)),
      m_process(new QProcess(this))
{
    connect(m_process, &QProcess::finished, this, &GPUStats::onProcessFinished);
    detectUtility();
    m_timer->setInterval(2000); // 2 s polling
    connect(m_timer, &QTimer::timeout, this, &GPUStats::updateStats);
    // timer starts when active flag is set from QML
}

void GPUStats::setActive(bool act)
{
    if (m_active == act) return;
    m_active = act;
    emit activeChanged();
    if (m_active && m_available) {
        m_timer->start();
        updateStats(); // immediate read
    } else {
        m_timer->stop();
        if (m_process && m_process->state() != QProcess::NotRunning) {
            m_process->terminate();
        }
    }
}

void GPUStats::refresh()
{
    if (m_available) updateStats();
}

void GPUStats::detectUtility()
{
    // 1. Prioridad: Inspección nativa directa de Linux sysfs (cero costo de CPU, sin forks)
    for (int cardIdx = 0; cardIdx < 8; ++cardIdx) {
        QString cardPath = QStringLiteral("/sys/class/drm/card%1/device").arg(cardIdx);
        if (QFile::exists(cardPath + QStringLiteral("/gpu_busy_percent"))) {
            m_sysfsGpuBusy = cardPath + QStringLiteral("/gpu_busy_percent");
            m_sysfsVramUsed = cardPath + QStringLiteral("/mem_info_vram_used");
            m_sysfsVramTotal = cardPath + QStringLiteral("/mem_info_vram_total");

            // Buscar sensor hwmon de temperatura
            QDir hwmonDir(cardPath + QStringLiteral("/hwmon"));
            QStringList hwmons = hwmonDir.entryList(QStringList() << QStringLiteral("hwmon*"), QDir::Dirs | QDir::NoDotAndDotDot);
            for (const QString &h : hwmons) {
                QString tempPath = hwmonDir.absoluteFilePath(h) + QStringLiteral("/temp1_input");
                if (QFile::exists(tempPath)) {
                    m_sysfsTemp = tempPath;
                    break;
                }
            }

            m_useSysfs = true;
            m_available = true;
            emit availabilityChanged();
            qDebug() << "GPUStats: detected native Linux sysfs DRM interface at" << cardPath;
            return;
        }
    }

    // 2. Fallback: herramientas CLI externas (nvidia-smi, radeontop, intel_gpu_top)
    const QStringList candidates = {"nvidia-smi", "radeontop", "intel_gpu_top"};
    for (const QString &bin : candidates) {
        const QString path = QStandardPaths::findExecutable(bin);
        if (!path.isEmpty()) {
            m_utilityPath = path;
            m_available = true;
            emit availabilityChanged();
            qDebug() << "GPUStats: detected utility" << bin << "at" << path;
            return;
        }
    }
    m_available = false;
    emit availabilityChanged();
}

bool GPUStats::readSysfsStats()
{
    if (!m_useSysfs) return false;

    // Lectura de gpu_busy_percent
    QFile busyFile(m_sysfsGpuBusy);
    if (busyFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QByteArray data = busyFile.readAll().trimmed();
        m_usage = qBound(0, data.toInt(), 100);
        busyFile.close();
    }

    // Lectura de VRAM
    if (!m_sysfsVramUsed.isEmpty() && !m_sysfsVramTotal.isEmpty()) {
        QFile usedFile(m_sysfsVramUsed);
        QFile totalFile(m_sysfsVramTotal);
        if (usedFile.open(QIODevice::ReadOnly | QIODevice::Text) &&
            totalFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
            quint64 used = usedFile.readAll().trimmed().toULongLong();
            quint64 total = totalFile.readAll().trimmed().toULongLong();
            if (total > 0) {
                m_vramUsage = qBound(0, int((used * 100) / total), 100);
            }
            usedFile.close();
            totalFile.close();
        }
    }

    // Lectura de Temperatura (miliCelsius -> Celsius)
    if (!m_sysfsTemp.isEmpty()) {
        QFile tempFile(m_sysfsTemp);
        if (tempFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
            int milliC = tempFile.readAll().trimmed().toInt();
            if (milliC > 0) {
                m_temperature = milliC / 1000;
            }
            tempFile.close();
        }
    }

    emit statsChanged();
    return true;
}

void GPUStats::updateStats()
{
    if (!m_available) return;

    // Si disponemos de sysfs, leemos de inmediato sin subprocesos
    if (m_useSysfs) {
        readSysfsStats();
        return;
    }

    // Para utilidades CLI externas, ejecutar de forma estrictamente asíncrona sin bloquear la UI
    if (m_process->state() != QProcess::NotRunning) {
        // La consulta previa aún sigue en ejecución, descartar para evitar saturación
        return;
    }

    m_process->setProgram(m_utilityPath);
    if (m_utilityPath.endsWith(QLatin1String("nvidia-smi"))) {
        m_process->setArguments({QStringLiteral("-q"), QStringLiteral("-x")});
    } else if (m_utilityPath.endsWith(QLatin1String("radeontop"))) {
        m_process->setArguments({QStringLiteral("-d"), QStringLiteral("-"), QStringLiteral("-l"), QStringLiteral("1")});
    } else if (m_utilityPath.endsWith(QLatin1String("intel_gpu_top"))) {
        m_process->setArguments({QStringLiteral("-J"), QStringLiteral("-s"), QStringLiteral("100")});
    }
    m_process->start();
}

void GPUStats::onProcessFinished(int exitCode, QProcess::ExitStatus exitStatus)
{
    if (exitStatus != QProcess::NormalExit || exitCode != 0) {
        return;
    }

    const QString out = QString::fromUtf8(m_process->readAllStandardOutput());
    if (m_utilityPath.contains(QLatin1String("nvidia-smi"))) {
        parseNvidiaSmi(out);
    } else if (m_utilityPath.contains(QLatin1String("radeontop"))) {
        parseRadeonTop(out);
    } else if (m_utilityPath.contains(QLatin1String("intel_gpu_top"))) {
        parseIntelGpuTop(out);
    }
    emit statsChanged();
}

void GPUStats::parseNvidiaSmi(const QString &output)
{
    QRegularExpression reUtil(R"(<gpu_util>(\d+)\s*%))");
    QRegularExpressionMatch m = reUtil.match(output);
    if (m.hasMatch()) m_usage = m.captured(1).toInt();

    QRegularExpression reMem(R"(<fb_memory_usage><total>(\d+)\s*MiB</total><used>(\d+)\s*MiB</used>)");
    m = reMem.match(output);
    if (m.hasMatch()) {
        int total = m.captured(1).toInt();
        int used  = m.captured(2).toInt();
        m_vramUsage = total ? (used * 100 / total) : 0;
    }

    QRegularExpression reTemp(R"(<temperature>\s*([\d\.]+)\s*C)");
    m = reTemp.match(output);
    if (m.hasMatch()) m_temperature = qRound(m.captured(1).toDouble());
}

void GPUStats::parseRadeonTop(const QString &output)
{
    QRegularExpression re(R"(GPU%:\s*([\d\.]+)\s*Mem%:\s*([\d\.]+)\s*Temp:\s*([\d\.]+)C)");
    QRegularExpressionMatch m = re.match(output);
    if (m.hasMatch()) {
        m_usage = qRound(m.captured(1).toDouble());
        m_vramUsage = qRound(m.captured(2).toDouble());
        m_temperature = qRound(m.captured(3).toDouble());
    }
}

void GPUStats::parseIntelGpuTop(const QString &output)
{
    QRegularExpression re(R"("renderUtil"\s*:\s*([\d\.]+))");
    QRegularExpressionMatch m = re.match(output);
    if (m.hasMatch()) m_usage = qRound(m.captured(1).toDouble());
    re.setPattern(R"("memUtil"\s*:\s*([\d\.]+))");
    m = re.match(output);
    if (m.hasMatch()) m_vramUsage = qRound(m.captured(1).toDouble());
    re.setPattern(R"("temperature"\s*:\s*([\d\.]+))");
    m = re.match(output);
    if (m.hasMatch()) m_temperature = qRound(m.captured(1).toDouble());
}
