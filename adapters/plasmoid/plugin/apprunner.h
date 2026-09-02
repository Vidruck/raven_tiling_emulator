/**
 * @file apprunner.h
 * @brief Indexador y lanzador de aplicaciones del sistema XDG para Raven Hub.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 */

#ifndef APPRUNNER_H
#define APPRUNNER_H

#include <QObject>
#include <QAbstractListModel>
#include <QSortFilterProxyModel>
#include <QVector>
#include <QString>
#include <QSet>
#include <qqmlintegration.h>

/**
 * @struct AppEntry
 * @brief Representación estructurada de una entrada de escritorio Freedesktop (.desktop).
 */
struct AppEntry {
    QString name;         ///< Nombre legible de la aplicación (Name o Name[es]).
    QString genericName;  ///< Categoría genérica (ej. 'Navegador Web', 'Terminal').
    QString keywords;     ///< Palabras clave de búsqueda indexadas (Keywords).
    QString comment;      ///< Descripción o tooltip de la aplicación (Comment).
    QString icon;         ///< Nombre o ruta del icono en el tema de Plasma (Icon).
    QString exec;         ///< Comando ejecutable con flags de argumentos depurados (Exec).
    QString categories;   ///< Categorías XDG (ej. 'Network;WebBrowser;').
    QString desktopPath;  ///< Ruta absoluta al archivo .desktop en el disco.
};

/**
 * @class AppListModel
 * @brief Modelo de lista C++ que indexa en memoria las aplicaciones instaladas en el sistema.
 *
 * Escanea de forma recursiva los directorios estándar de XDG:
 * - `/usr/share/applications`
 * - `/usr/local/share/applications`
 * - `~/.local/share/applications`
 * - Flatpak / Snap (`/var/lib/flatpak/exports/share/applications`, etc.)
 */
class AppListModel : public QAbstractListModel
{
    Q_OBJECT

public:
    /**
     * @enum AppRoles
     * @brief Roles de datos expuestos a las vistas QML (GridView / ListView).
     */
    enum AppRoles {
        NameRole = Qt::UserRole + 1, ///< Nombre de la aplicación.
        GenericNameRole,            ///< Nombre genérico o tipo de utilidad.
        KeywordsRole,               ///< Palabras clave para coincidencia fuzzy.
        CommentRole,                ///< Descripción explicativa.
        IconRole,                   ///< Identificador del icono gráfico.
        ExecRole,                   ///< Comando shell a ejecutar.
        CategoriesRole,             ///< Lista de categorías XDG.
        DesktopPathRole             ///< Ruta al archivo .desktop.
    };

    /**
     * @brief Constructor del modelo base. Indexa automáticamente las aplicaciones al instanciarse.
     * @param parent Puntero opcional al objeto padre Qt.
     */
    explicit AppListModel(QObject *parent = nullptr);
    
    /** @brief Destructor virtual por defecto. */
    ~AppListModel() override;

    /** @return Cantidad de aplicaciones válidas indexadas en el sistema. */
    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    
    /** @brief Provee los datos según el rol solicitado para el delegado QML. */
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    
    /** @brief Mapea los roles C++ con los nombres de propiedad disponibles en QML. */
    QHash<int, QByteArray> roleNames() const override;

    /** @brief Re-escanea los directorios de aplicaciones XDG y reconstruye el índice. */
    void reloadApplications();

private:
    QVector<AppEntry> m_apps; ///< Lista en memoria de todas las aplicaciones detectadas.
    
    /**
     * @brief Parsea un archivo .desktop individual respetando localización y banderas NoDisplay/Hidden.
     * @param filePath Ruta del archivo .desktop.
     * @param processedNames Conjunto para evitar duplicados de aplicaciones con mismo ID.
     */
    void parseDesktopFile(const QString &filePath, QSet<QString> &processedNames);
};

/**
 * @class AppFilterModel
 * @brief Modelo proxy reactivo para filtrado y búsqueda instantánea de aplicaciones en Raven Hub.
 *
 * Realiza búsquedas de texto dinámicas (coincidencias en nombre, genérico, palabras clave y ejecutable)
 * y filtrado por categoría XDG con alta eficiencia.
 */
class AppFilterModel : public QSortFilterProxyModel
{
    Q_OBJECT
    QML_ELEMENT
    
    /** @brief Texto ingresado en la barra de búsqueda para filtrar la cuadrícula. */
    Q_PROPERTY(QString searchFilter READ searchFilter WRITE setSearchFilter NOTIFY searchFilterChanged)
    
    /** @brief Filtro por categoría XDG (ej. 'Internet', 'Development', 'Multimedia' o 'All'). */
    Q_PROPERTY(QString categoryFilter READ categoryFilter WRITE setCategoryFilter NOTIFY categoryFilterChanged)
    
    /** @brief Cantidad de aplicaciones resultantes tras aplicar los filtros de búsqueda. */
    Q_PROPERTY(int count READ count NOTIFY countChanged)

public:
    /**
     * @brief Constructor del modelo proxy. Inicializa el modelo fuente AppListModel.
     * @param parent Puntero opcional al objeto padre Qt.
     */
    explicit AppFilterModel(QObject *parent = nullptr);

    /** @return Texto del filtro de búsqueda activo. */
    QString searchFilter() const { return m_searchFilter; }
    
    /** @brief Modifica el texto de búsqueda y re-evalúa el filtrado visual. */
    void setSearchFilter(const QString &search);

    /** @return Categoría seleccionada. */
    QString categoryFilter() const { return m_categoryFilter; }
    
    /** @brief Establece la categoría de filtrado. */
    void setCategoryFilter(const QString &category);

    /** @return Número de elementos visibles tras el filtrado. */
    int count() const { return rowCount(); }

    /**
     * @brief Ejecuta una aplicación de forma desacoplada mediante KRun/QProcess.
     * @param execCmd Comando shell o ejecutable.
     * @param desktopPath Ruta opcional al archivo .desktop para integración con Wayland/KWin.
     */
    Q_INVOKABLE void launchApp(const QString &execCmd, const QString &desktopPath = QString());
    
    /**
     * @brief Lanza la aplicación ubicada en el índice visual especificado del modelo.
     * @param index Fila seleccionada en el delegado QML.
     */
    Q_INVOKABLE void launchIndex(int index);
    
    /** @brief Recarga la base de aplicaciones desde el disco. */
    Q_INVOKABLE void refresh();

signals:
    /** @brief Emitida cuando se altera el texto de búsqueda. */
    void searchFilterChanged();
    
    /** @brief Emitida al seleccionar una nueva categoría. */
    void categoryFilterChanged();
    
    /** @brief Emitida cuando varía la cantidad de resultados encontrados. */
    void countChanged();

protected:
    /**
     * @brief Algoritmo de decisión para determinar si una fila cumple con los criterios de búsqueda y categoría.
     * @param sourceRow Fila en el modelo fuente AppListModel.
     * @param sourceParent Índice padre.
     * @return true si la aplicación debe mostrarse en la interfaz.
     */
    bool filterAcceptsRow(int sourceRow, const QModelIndex &sourceParent) const override;

private:
    QString m_searchFilter;         ///< Texto de búsqueda actual.
    QString m_categoryFilter;       ///< Categoría activa.
    AppListModel *m_sourceModel;    ///< Puntero al modelo de datos fuente.
};

#endif // APPRUNNER_H
