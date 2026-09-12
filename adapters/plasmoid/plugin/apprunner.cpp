/**
 * @file apprunner.cpp
 * @brief Implementación del indexador y lanzador de aplicaciones del sistema XDG.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 * @license GPL-3.0
 */

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

/**
 * @brief Constructor de la clase AppListModel.
 *
 * Inicializa el modelo principal que almacena en memoria la base de datos de 
 * aplicaciones (.desktop) parseada. Instancia un watcher global para detectar
 * instalaciones o desinstalaciones en tiempo real.
 *
 * @param parent Objeto padre.
 */
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

/**
 * @brief Destructor de la clase AppListModel.
 */
AppListModel::~AppListModel()
{
    s_activeModels.removeAll(this);
}

/**
 * @brief Devuelve la cantidad de aplicaciones en el índice actual.
 * @param parent Índice del elemento padre (usado en modelos de árbol).
 * @return Número de aplicaciones.
 */
int AppListModel::rowCount(const QModelIndex &parent) const
{
    if (parent.isValid()) return 0;
    return m_apps.size();
}

/**
 * @brief Retorna el dato correspondiente a la aplicación según su índice y el rol solicitado.
 * @param index Índice de la aplicación en el modelo.
 * @param role Tipo de dato a solicitar (ej. NameRole, ExecRole).
 * @return El dato envuelto en QVariant.
 */
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
    case ActionsRole: return entry.actions;
    default: return QVariant();
    }
}

/**
 * @brief Mapea los roles C++ con nombres de propiedades exportados a QML.
 * @return Mapeo entre enumeradores de rol y strings de QML.
 */
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
    roles[ActionsRole] = "actions";
    return roles;
}

/**
 * @brief Escanea y recarga la lista completa de aplicaciones en memoria.
 *
 * Lee los directorios estándar del sistema en busca de archivos `.desktop`,
 * parseando sus contenidos y almacenando el resultado en un caché estático.
 */
void AppListModel::reloadApplications()
{
    beginResetModel();
    m_apps.clear();
    m_apps.reserve(300);

    QSet<QString> processedDesktopIds;

    // Rutas de búsqueda de aplicaciones XDG estándar en orden de prioridad (Usuario -> Flatpak -> Snap -> Sistema)
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

/**
 * @brief Analiza sintácticamente un archivo .desktop y lo añade al índice.
 *
 * Valida los parámetros requeridos (Name, Exec, Type=Application), resuelve
 * rutas, localizaciones y acciones secundarias. Descarta aplicaciones marcadas
 * como `NoDisplay` o `Hidden`.
 *
 * @param filePath Ruta absoluta al archivo `.desktop`.
 * @param processedDesktopIds Conjunto de IDs ya procesados para evitar duplicidad.
 */
void AppListModel::parseDesktopFile(const QString &filePath, QSet<QString> &processedDesktopIds)
{
    QFileInfo fi(filePath);
    QString desktopId = fi.fileName();
    if (processedDesktopIds.contains(desktopId)) {
        return; // Una ruta de búsqueda de mayor prioridad ya proveyó este ID de escritorio
    }

    QFile file(filePath);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) return;

    QTextStream in(&file);
    bool inDesktopEntry = false;
    QString currentActionId;
    bool inActionEntry = false;
    bool noDisplay = false;
    bool hidden = false;
    QString name, genericName, keywords, comment, icon, exec, categories, type = QStringLiteral("Application");
    QString tryExec, onlyShowIn, notShowIn;
    QString actionsListStr;

    struct RawAction {
        QString id;
        QString name;
        QString exec;
        QString icon;
        bool hasLocName = false;
    };
    QMap<QString, RawAction> actionEntries;

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
            if (line == QStringLiteral("[Desktop Entry]")) {
                inDesktopEntry = true;
                inActionEntry = false;
                currentActionId.clear();
            } else if (line.startsWith(QStringLiteral("[Desktop Action ")) && line.endsWith(QLatin1Char(']'))) {
                inDesktopEntry = false;
                inActionEntry = true;
                currentActionId = line.mid(16, line.length() - 17).trimmed();
                if (!actionEntries.contains(currentActionId)) {
                    RawAction act;
                    act.id = currentActionId;
                    actionEntries.insert(currentActionId, act);
                }
            } else {
                inDesktopEntry = false;
                inActionEntry = false;
                currentActionId.clear();
            }
            continue;
        }

        int eqPos = line.indexOf(QLatin1Char('='));
        if (eqPos <= 0) continue;

        const QString key = line.left(eqPos).trimmed();
        const QString value = line.mid(eqPos + 1).trimmed();

        if (inDesktopEntry) {
            // Nombre
            if (key == nameLoc1 || key == nameLoc2) {
                name = value;
                hasLocName = true;
            } else if (key == QStringLiteral("Name") && !hasLocName) {
                name = value;
            }
            // Nombre genérico
            else if (key == genLoc1 || key == genLoc2) {
                genericName = value;
                hasLocGen = true;
            } else if (key == QStringLiteral("GenericName") && !hasLocGen) {
                genericName = value;
            }
            // Palabras clave
            else if (key == keyLoc1 || key == keyLoc2) {
                keywords = value;
                hasLocKey = true;
            } else if (key == QStringLiteral("Keywords") && !hasLocKey) {
                keywords = value;
            }
            // Comentario
            else if (key == comLoc1 || key == comLoc2) {
                comment = value;
                hasLocCom = true;
            } else if (key == QStringLiteral("Comment") && !hasLocCom) {
                comment = value;
            }
            // Atributos generales
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
            } else if (key == QStringLiteral("Actions")) {
                actionsListStr = value;
            } else if (key == QStringLiteral("NoDisplay")) {
                noDisplay = (value.compare(QStringLiteral("true"), Qt::CaseInsensitive) == 0);
            } else if (key == QStringLiteral("Hidden")) {
                hidden = (value.compare(QStringLiteral("true"), Qt::CaseInsensitive) == 0);
            }
        } else if (inActionEntry && !currentActionId.isEmpty()) {
            RawAction &act = actionEntries[currentActionId];
            if (key == nameLoc1 || key == nameLoc2) {
                act.name = value;
                act.hasLocName = true;
            } else if (key == QStringLiteral("Name") && !act.hasLocName) {
                act.name = value;
            } else if (key == QStringLiteral("Exec")) {
                act.exec = value;
            } else if (key == QStringLiteral("Icon")) {
                act.icon = value;
            }
        }
    }

    file.close();

    if (noDisplay || hidden || type != QStringLiteral("Application") || name.isEmpty() || exec.isEmpty()) {
        return;
    }

    // Comprobar OnlyShowIn (filtrar solo si está explícitamente asignado y no coincide con KDE/Plasma/Qt)
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

    // Comprobar NotShowIn
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

    // Comprobar TryExec si está presente
    if (!tryExec.isEmpty()) {
        if (tryExec.startsWith(QLatin1Char('/'))) {
            if (!QFile::exists(tryExec)) return;
        } else {
            if (QStandardPaths::findExecutable(tryExec).isEmpty()) return;
        }
    }

    processedDesktopIds.insert(desktopId);

    // Limpiar códigos de campo Exec como %u, %f, %U, %F, %i, %c, %k
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

    // Construir Acciones de Escritorio en el orden definido o parseado
    QVariantList parsedActions;
    QStringList actionOrder = actionsListStr.split(QLatin1Char(';'), Qt::SkipEmptyParts);
    if (!actionOrder.isEmpty()) {
        for (const QString &actId : actionOrder) {
            const QString cleanId = actId.trimmed();
            if (actionEntries.contains(cleanId)) {
                const RawAction &ra = actionEntries.value(cleanId);
                if (!ra.exec.isEmpty()) {
                    QString cleanActionExec = ra.exec;
                    cleanActionExec.remove(QRegularExpression(QStringLiteral("%[a-zA-Z]")));
                    QVariantMap actMap;
                    actMap.insert(QStringLiteral("id"), cleanId);
                    actMap.insert(QStringLiteral("name"), ra.name.isEmpty() ? cleanId : ra.name);
                    actMap.insert(QStringLiteral("exec"), cleanActionExec.trimmed());
                    actMap.insert(QStringLiteral("icon"), ra.icon.isEmpty() ? entry.icon : ra.icon);
                    parsedActions.append(actMap);
                }
            }
        }
    } else {
        // Si Actions= no se especificó pero existen bloques [Desktop Action ...]
        for (auto it = actionEntries.constBegin(); it != actionEntries.constEnd(); ++it) {
            const RawAction &ra = it.value();
            if (!ra.exec.isEmpty()) {
                QString cleanActionExec = ra.exec;
                cleanActionExec.remove(QRegularExpression(QStringLiteral("%[a-zA-Z]")));
                QVariantMap actMap;
                actMap.insert(QStringLiteral("id"), ra.id);
                actMap.insert(QStringLiteral("name"), ra.name.isEmpty() ? ra.id : ra.name);
                actMap.insert(QStringLiteral("exec"), cleanActionExec.trimmed());
                actMap.insert(QStringLiteral("icon"), ra.icon.isEmpty() ? entry.icon : ra.icon);
                parsedActions.append(actMap);
            }
        }
    }
    entry.actions = parsedActions;

    m_apps.append(entry);
}

// --- AppFilterModel ---

/**
 * @brief Constructor del modelo de filtrado de aplicaciones.
 *
 * Instancia el modelo envolvente `QSortFilterProxyModel` que se conecta al
 * `AppListModel` para filtrar los resultados en la cuadrícula o buscador.
 *
 * @param parent Objeto padre.
 */
AppFilterModel::AppFilterModel(QObject *parent)
    : QSortFilterProxyModel(parent)
{
    m_sourceModel = new AppListModel(this);
    setSourceModel(m_sourceModel);
    setFilterCaseSensitivity(Qt::CaseInsensitive);
    sort(0, Qt::AscendingOrder);
}

/**
 * @brief Establece el término de búsqueda para filtrar la lista en tiempo real.
 * @param search Texto ingresado por el usuario.
 */
void AppFilterModel::setSearchFilter(const QString &search)
{
    if (m_searchFilter == search) return;
    m_searchFilter = search;
    invalidate();
    emit searchFilterChanged();
    emit countChanged();
}

/**
 * @brief Establece el filtro categórico de la lista de aplicaciones.
 * @param category Nombre interno de la categoría (ej. "favorites", "network").
 */
void AppFilterModel::setCategoryFilter(const QString &category)
{
    if (m_categoryFilter == category) return;
    m_categoryFilter = category;
    invalidate();
    emit categoryFilterChanged();
    emit countChanged();
}

/**
 * @brief Fuerza una recarga completa del modelo subyacente.
 */
void AppFilterModel::refresh()
{
    s_appsLoaded = false;
    m_sourceModel->reloadApplications();
    invalidate();
    emit countChanged();
}

/**
 * @brief Intenta lanzar el ejecutable de una aplicación de forma segura.
 *
 * @param execCmd Comando base que define el archivo `.desktop`.
 * @param desktopPath Ruta absoluta hacia el `.desktop` para permitir que `gio` lo resuelva.
 */
void AppFilterModel::launchApp(const QString &execCmd, const QString &desktopPath)
{
    // Preferido: Lanzar mediante ID de archivo de escritorio o gio
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

    // Ejecución segura sin evaluación de cadenas de shell
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

/**
 * @brief Lanza una aplicación basándose en su índice filtrado activo en la vista.
 * @param idx Fila correspondiente a la aplicación clickeada en la interfaz QML.
 */
void AppFilterModel::launchIndex(int idx)
{
    if (idx < 0 || idx >= rowCount()) return;
    QModelIndex modelIdx = index(idx, 0);
    QString execCmd = data(modelIdx, AppListModel::ExecRole).toString();
    QString desktopPath = data(modelIdx, AppListModel::DesktopPathRole).toString();
    launchApp(execCmd, desktopPath);
}

/**
 * @brief Recupera las acciones secundarias de una aplicación por su índice en la vista.
 * @param idx Fila de la aplicación clickeada con clic derecho en QML.
 * @return Lista de variantes estructuradas con `id`, `name`, `exec` e `icon`.
 */
QVariantList AppFilterModel::getAppActions(int idx) const
{
    if (idx < 0 || idx >= rowCount()) return QVariantList();
    QModelIndex modelIdx = index(idx, 0);
    return data(modelIdx, AppListModel::ActionsRole).toList();
}

/**
 * @brief Algoritmo interno de filtrado para decidir si una fila del modelo subyacente se muestra o se oculta.
 *
 * Cruza las restricciones de búsqueda por texto y la restricción por categoría.
 *
 * @param sourceRow Índice de la fila original en `AppListModel`.
 * @param sourceParent Índice del padre de la fila original.
 * @return True si la aplicación cumple los filtros, False si debe ocultarse.
 */
bool AppFilterModel::filterAcceptsRow(int sourceRow, const QModelIndex &sourceParent) const
{
    QModelIndex idx = sourceModel()->index(sourceRow, 0, sourceParent);
    if (!idx.isValid()) return false;

    // Filtro de búsqueda: comprueba coincidencias en Nombre, Nombre Genérico, Palabras Clave o Comentario
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

    // Filtro por categoría
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
