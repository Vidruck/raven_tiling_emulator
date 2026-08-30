#ifndef APPRUNNER_H
#define APPRUNNER_H

#include <QObject>
#include <QAbstractListModel>
#include <QSortFilterProxyModel>
#include <QVector>
#include <QString>
#include <QSet>
#include <qqmlintegration.h>

struct AppEntry {
    QString name;
    QString genericName;
    QString keywords;
    QString comment;
    QString icon;
    QString exec;
    QString categories;
    QString desktopPath;
};

class AppListModel : public QAbstractListModel
{
    Q_OBJECT

public:
    enum AppRoles {
        NameRole = Qt::UserRole + 1,
        GenericNameRole,
        KeywordsRole,
        CommentRole,
        IconRole,
        ExecRole,
        CategoriesRole,
        DesktopPathRole
    };

    explicit AppListModel(QObject *parent = nullptr);
    ~AppListModel() override;

    int rowCount(const QModelIndex &parent = QModelIndex()) const override;
    QVariant data(const QModelIndex &index, int role = Qt::DisplayRole) const override;
    QHash<int, QByteArray> roleNames() const override;

    void reloadApplications();

private:
    QVector<AppEntry> m_apps;
    void parseDesktopFile(const QString &filePath, QSet<QString> &processedNames);
};

class AppFilterModel : public QSortFilterProxyModel
{
    Q_OBJECT
    QML_ELEMENT
    Q_PROPERTY(QString searchFilter READ searchFilter WRITE setSearchFilter NOTIFY searchFilterChanged)
    Q_PROPERTY(QString categoryFilter READ categoryFilter WRITE setCategoryFilter NOTIFY categoryFilterChanged)
    Q_PROPERTY(int count READ count NOTIFY countChanged)

public:
    explicit AppFilterModel(QObject *parent = nullptr);

    QString searchFilter() const { return m_searchFilter; }
    void setSearchFilter(const QString &search);

    QString categoryFilter() const { return m_categoryFilter; }
    void setCategoryFilter(const QString &category);

    int count() const { return rowCount(); }

    Q_INVOKABLE void launchApp(const QString &execCmd, const QString &desktopPath = QString());
    Q_INVOKABLE void launchIndex(int index);
    Q_INVOKABLE void refresh();

signals:
    void searchFilterChanged();
    void categoryFilterChanged();
    void countChanged();

protected:
    bool filterAcceptsRow(int sourceRow, const QModelIndex &sourceParent) const override;

private:
    QString m_searchFilter;
    QString m_categoryFilter;
    AppListModel *m_sourceModel;
};

#endif // APPRUNNER_H
