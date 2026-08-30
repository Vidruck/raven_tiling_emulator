#include "apprunner.h"
#include <QDir>
#include <QFile>
#include <QTextStream>
#include <QProcess>
#include <QStandardPaths>
#include <QRegularExpression>
#include <QFileSystemWatcher>
#include <QTimer>
#include <QCoreApplication>
#include <QPointer>
#include <QFileInfo>
#include <QDebug>

static QVector<AppEntry> s_cachedApps;
static bool s_appsLoaded = false;
static QFileSystemWatcher *s_appWatcher = nullptr;
static QTimer *s_reloadTimer = nullptr;
static QList<QPointer<AppListModel>> s_activeModels;

AppListModel::AppListModel(QObject *parent)
    : QAbstractListModel(parent)
{
    s_activeModels.append(this);

    if (!s_appsLoaded) {
        reloadApplications();
    } else {
        m_apps = s_cachedApps;
    }

    if (!s_appWatcher) {
        s_appWatcher = new QFileSystemWatcher(qApp);
        s_reloadTimer = new QTimer(qApp);
        s_reloadTimer->setSingleShot(true);
        s_reloadTimer->setInterval(400); // Debounce
        
        auto triggerReload = []() {
            if (s_reloadTimer) s_reloadTimer->start();
        };

        QObject::connect(s_appWatcher, &QFileSystemWatcher::directoryChanged, s_reloadTimer, triggerReload);
        QObject::connect(s_appWatcher, &QFileSystemWatcher::fileChanged, s_reloadTimer, triggerReload);

        QObject::connect(s_reloadTimer, &QTimer::timeout, s_reloadTimer, []() {
            s_appsLoaded = false;
            for (const QPointer<AppListModel> &model : std::as_const(s_activeModels)) {
                if (model) {
                    model->reloadApplications();
                }
            }
        });
    }
}

AppListModel::~AppListModel()
{
    s_activeModels.removeAll(this);
}

int AppListModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid()) return 0;
    return m_apps.size();
}

QVariant AppListModel::data(const QModelIndex &index, int role) const
{
    if (!index.isValid() || index.row() < 0 || index.row() >= m_apps.size())
        return QVariant();

    const AppEntry &entry = m_apps.at(index.row());
    switch (role) {
    case NameRole: return entry.name;
    case GenericNameRole: return entry.genericName;
    case KeywordsRole: return entry.keywords;
    case CommentRole: return entry.comment;
    case IconRole: return entry.icon;
    case ExecRole: return entry.exec;
    case CategoriesRole: return entry.categories;
    case DesktopPathRole: return entry.desktopPath;
    default: return QVariant();
    }
}

QHash<int, QByteArray> AppListModel::roleNames() const
{
    QHash<int, QByteArray> roles;
    roles[NameRole] = "appName";
    roles[GenericNameRole] = "genericName";
    roles[KeywordsRole] = "keywords";
    roles[CommentRole] = "comment";
    roles[IconRole] = "iconName";
    roles[ExecRole] = "execCmd";
    roles[CategoriesRole] = "categories";
    roles[DesktopPathRole] = "desktopPath";
    return roles;
}

void AppListModel::reloadApplications()
{
    beginResetModel();
    m_apps.clear();
    m_apps.reserve(300);

    QSet<QString> processedDesktopIds;

    // Standard XDG application search paths in priority order (User -> Flatpak -> Snap -> System)
    QStringList searchPaths = {
        QDir::homePath() + QStringLiteral("/.local/share/applications"),
        QDir::homePath() + QStringLiteral("/.local/share/flatpak/exports/share/applications"),
        QStringLiteral("/var/lib/flatpak/exports/share/applications"),
        QStringLiteral("/var/lib/snapd/desktop/applications"),
        QStringLiteral("/snap/share/applications"),
        QStringLiteral("/usr/local/share/applications"),
        QStringLiteral("/usr/share/applications")
    };

    const QStringList stdPaths = QStandardPaths::standardLocations(QStandardPaths::ApplicationsLocation);
    for (const QString &p : stdPaths) {
        if (!searchPaths.contains(p)) {
            searchPaths.append(p);
        }
    }

    for (const QString &path : searchPaths) {
        QDir dir(path);
        if (!dir.exists()) continue;
        
        if (s_appWatcher && !s_appWatcher->directories().contains(path)) {
            s_appWatcher->addPath(path);
        }

        const QStringList entries = dir.entryList({QStringLiteral("*.desktop")}, QDir::Files);
        for (const QString &file : entries) {
            parseDesktopFile(dir.absoluteFilePath(file), processedDesktopIds);
        }
    }

    s_cachedApps = m_apps;
    s_appsLoaded = true;

    endResetModel();
}

void AppListModel::parseDesktopFile(const QString &filePath, QSet<QString> &processedDesktopIds)
{
    QFileInfo fi(filePath);
    QString desktopId = fi.fileName();
    if (processedDesktopIds.contains(desktopId)) {
        return; // Higher priority search path already provided this desktop ID
    }

    QFile file(filePath);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) return;

    QTextStream in(&file);
    bool inDesktopEntry = false;
    bool noDisplay = false;
    bool hidden = false;
    QString name, genericName, keywords, comment, icon, exec, categories, type = QStringLiteral("Application");
    QString tryExec, onlyShowIn, notShowIn;

    QString sysLocale = QLocale::system().name();
    QString langCode = sysLocale.left(2);
    QString nameLoc1 = QStringLiteral("Name[%1]").arg(sysLocale);
    QString nameLoc2 = QStringLiteral("Name[%1]").arg(langCode);
    QString genLoc1 = QStringLiteral("GenericName[%1]").arg(sysLocale);
    QString genLoc2 = QStringLiteral("GenericName[%1]").arg(langCode);
    QString keyLoc1 = QStringLiteral("Keywords[%1]").arg(sysLocale);
    QString keyLoc2 = QStringLiteral("Keywords[%1]").arg(langCode);
    QString comLoc1 = QStringLiteral("Comment[%1]").arg(sysLocale);
    QString comLoc2 = QStringLiteral("Comment[%1]").arg(langCode);

    bool hasLocName = false;
    bool hasLocGen = false;
    bool hasLocKey = false;
    bool hasLocCom = false;

    while (!in.atEnd()) {
        QString line = in.readLine().trimmed();
        if (line.isEmpty() || line.startsWith(QLatin1Char('#'))) continue;

        if (line.startsWith(QLatin1Char('[')) && line.endsWith(QLatin1Char(']'))) {
            inDesktopEntry = (line == QStringLiteral("[Desktop Entry]"));
            continue;
        }

        if (!inDesktopEntry) continue;

        int eqPos = line.indexOf(QLatin1Char('='));
        if (eqPos <= 0) continue;

        const QString key = line.left(eqPos).trimmed();
        const QString value = line.mid(eqPos + 1).trimmed();

        // Name
        if (key == nameLoc1 || key == nameLoc2) {
            name = value;
            hasLocName = true;
        } else if (key == QStringLiteral("Name") && !hasLocName) {
            name = value;
        }
        // GenericName
        else if (key == genLoc1 || key == genLoc2) {
            genericName = value;
            hasLocGen = true;
        } else if (key == QStringLiteral("GenericName") && !hasLocGen) {
            genericName = value;
        }
        // Keywords
        else if (key == keyLoc1 || key == keyLoc2) {
            keywords = value;
            hasLocKey = true;
        } else if (key == QStringLiteral("Keywords") && !hasLocKey) {
            keywords = value;
        }
        // Comment
        else if (key == comLoc1 || key == comLoc2) {
            comment = value;
            hasLocCom = true;
        } else if (key == QStringLiteral("Comment") && !hasLocCom) {
            comment = value;
        }
        // General attributes
        else if (key == QStringLiteral("Icon")) {
            icon = value;
        } else if (key == QStringLiteral("Exec")) {
            exec = value;
        } else if (key == QStringLiteral("Categories")) {
            categories = value;
        } else if (key == QStringLiteral("Type")) {
            type = value;
        } else if (key == QStringLiteral("TryExec")) {
            tryExec = value;
        } else if (key == QStringLiteral("OnlyShowIn")) {
            onlyShowIn = value;
        } else if (key == QStringLiteral("NotShowIn")) {
            notShowIn = value;
        } else if (key == QStringLiteral("NoDisplay")) {
            noDisplay = (value.compare(QStringLiteral("true"), Qt::CaseInsensitive) == 0);
        } else if (key == QStringLiteral("Hidden")) {
            hidden = (value.compare(QStringLiteral("true"), Qt::CaseInsensitive) == 0);
        }
    }

    file.close();

    if (noDisplay || hidden || type != QStringLiteral("Application") || name.isEmpty() || exec.isEmpty()) {
        return;
    }

    // Check OnlyShowIn (only filter out if explicitly assigned and doesn't match KDE/Plasma/Qt)
    if (!onlyShowIn.isEmpty()) {
        QStringList allowed = onlyShowIn.split(QLatin1Char(';'), Qt::SkipEmptyParts);
        bool matchesKde = false;
        for (const QString &env : allowed) {
            if (env.compare(QLatin1String("KDE"), Qt::CaseInsensitive) == 0 ||
                env.compare(QLatin1String("Plasma"), Qt::CaseInsensitive) == 0 ||
                env.compare(QLatin1String("X-KDE"), Qt::CaseInsensitive) == 0 ||
                env.compare(QLatin1String("Qt"), Qt::CaseInsensitive) == 0) {
                matchesKde = true;
                break;
            }
        }
        if (!matchesKde) return;
    }

    // Check NotShowIn
    if (!notShowIn.isEmpty()) {
        QStringList blocked = notShowIn.split(QLatin1Char(';'), Qt::SkipEmptyParts);
        for (const QString &env : blocked) {
            if (env.compare(QLatin1String("KDE"), Qt::CaseInsensitive) == 0 ||
                env.compare(QLatin1String("Plasma"), Qt::CaseInsensitive) == 0 ||
                env.compare(QLatin1String("X-KDE"), Qt::CaseInsensitive) == 0) {
                return;
            }
        }
    }

    // Check TryExec if present
    if (!tryExec.isEmpty()) {
        if (tryExec.startsWith(QLatin1Char('/'))) {
            if (!QFile::exists(tryExec)) return;
        } else {
            if (QStandardPaths::findExecutable(tryExec).isEmpty()) return;
        }
    }

    processedDesktopIds.insert(desktopId);

    // Clean Exec field codes %u, %f, %U, %F, %i, %c, %k
    exec.remove(QRegularExpression(QStringLiteral("%[a-zA-Z]")));

    AppEntry entry;
    entry.name = name;
    entry.genericName = genericName;
    entry.keywords = keywords;
    entry.comment = comment;
    entry.icon = icon.isEmpty() ? QStringLiteral("application-x-executable") : icon;
    entry.exec = exec.trimmed();
    entry.categories = categories;
    entry.desktopPath = filePath;

    m_apps.append(entry);
}

// --- AppFilterModel ---

AppFilterModel::AppFilterModel(QObject *parent)
    : QSortFilterProxyModel(parent)
{
    m_sourceModel = new AppListModel(this);
    setSourceModel(m_sourceModel);
    setFilterCaseSensitivity(Qt::CaseInsensitive);
    sort(0, Qt::AscendingOrder);
}

void AppFilterModel::setSearchFilter(const QString &search)
{
    if (m_searchFilter == search) return;
    m_searchFilter = search;
    invalidate();
    emit searchFilterChanged();
    emit countChanged();
}

void AppFilterModel::setCategoryFilter(const QString &category)
{
    if (m_categoryFilter == category) return;
    m_categoryFilter = category;
    invalidate();
    emit categoryFilterChanged();
    emit countChanged();
}

void AppFilterModel::refresh()
{
    s_appsLoaded = false;
    m_sourceModel->reloadApplications();
    invalidate();
    emit countChanged();
}

void AppFilterModel::launchApp(const QString &execCmd, const QString &desktopPath)
{
    // Preferred: Launch via desktop file ID or gio
    if (!desktopPath.isEmpty() && QFile::exists(desktopPath)) {
        QFileInfo fi(desktopPath);
        QString desktopId = fi.fileName();
        if (QProcess::startDetached(QStringLiteral("gio"), QStringList() << QStringLiteral("launch") << desktopPath)) {
            return;
        }
        if (QProcess::startDetached(QStringLiteral("gtk-launch"), QStringList() << desktopId)) {
            return;
        }
    }

    // Safe execution without shell string evaluation
    if (!execCmd.isEmpty()) {
        QString cleanCmd = execCmd;
        cleanCmd = cleanCmd.remove(QRegularExpression(QStringLiteral("%[a-zA-Z]"))).trimmed();
        QStringList args = QProcess::splitCommand(cleanCmd);
        if (!args.isEmpty()) {
            QString program = args.takeFirst();
            if (QProcess::startDetached(program, args)) {
                return;
            }
        }
    }
}

void AppFilterModel::launchIndex(int idx)
{
    if (idx < 0 || idx >= rowCount()) return;
    QModelIndex modelIdx = index(idx, 0);
    QString execCmd = data(modelIdx, AppListModel::ExecRole).toString();
    QString desktopPath = data(modelIdx, AppListModel::DesktopPathRole).toString();
    launchApp(execCmd, desktopPath);
}

bool AppFilterModel::filterAcceptsRow(int sourceRow, const QModelIndex &sourceParent) const
{
    QModelIndex idx = sourceModel()->index(sourceRow, 0, sourceParent);
    if (!idx.isValid()) return false;

    // Search filter: matches Name, GenericName, Keywords, or Comment
    if (!m_searchFilter.isEmpty()) {
        QString appName = sourceModel()->data(idx, AppListModel::NameRole).toString();
        QString genericName = sourceModel()->data(idx, AppListModel::GenericNameRole).toString();
        QString keywords = sourceModel()->data(idx, AppListModel::KeywordsRole).toString();
        QString comment = sourceModel()->data(idx, AppListModel::CommentRole).toString();

        bool match = appName.contains(m_searchFilter, Qt::CaseInsensitive)
                  || genericName.contains(m_searchFilter, Qt::CaseInsensitive)
                  || keywords.contains(m_searchFilter, Qt::CaseInsensitive)
                  || comment.contains(m_searchFilter, Qt::CaseInsensitive);

        if (!match) {
            return false;
        }
    }

    // Category filter
    if (!m_categoryFilter.isEmpty() && m_categoryFilter != QStringLiteral("all")) {
        if (m_categoryFilter == QStringLiteral("favorites")) {
            return true;
        }

        QString appCategories = sourceModel()->data(idx, AppListModel::CategoriesRole).toString();
        if (!appCategories.contains(m_categoryFilter, Qt::CaseInsensitive)) {
            return false;
        }
    }

    return true;
}
