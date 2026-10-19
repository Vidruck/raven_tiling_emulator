#include "gpustats.h"
#include <QFileInfo>
#include <QStandardPaths>
#include <QProcess>
#include <QRegularExpression>
#include <QDebug>

GPUStats::GPUStats(QObject *parent)
    : QObject(parent),
      m_timer(new QTimer(this))
{
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
    }
}

void GPUStats::refresh()
{
    if (m_available) updateStats();
}

void GPUStats::detectUtility()
{
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

void GPUStats::updateStats()
{
    if (!m_available) return;

    QProcess proc;
    proc.setProgram(m_utilityPath);
    if (m_utilityPath.endsWith("nvidia-smi")) {
        proc.setArguments({"-q", "-x"});
    } else if (m_utilityPath.endsWith("radeontop")) {
        proc.setArguments({"-d", "-l"}); // single sample
    } else if (m_utilityPath.endsWith("intel_gpu_top")) {
        proc.setArguments({"-J"}); // JSON output if supported
    }
    proc.start();
    if (!proc.waitForFinished(1000)) {
        qWarning() << "GPUStats: timeout executing" << m_utilityPath;
        return;
    }
    const QString out = QString::fromUtf8(proc.readAllStandardOutput());

    if (m_utilityPath.contains("nvidia-smi")) {
        parseNvidiaSmi(out);
    } else if (m_utilityPath.contains("radeontop")) {
        parseRadeonTop(out);
    } else if (m_utilityPath.contains("intel_gpu_top")) {
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
