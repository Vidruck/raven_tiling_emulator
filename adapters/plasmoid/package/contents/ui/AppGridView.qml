/**
 * @file AppGridView.qml
 * @brief Vista en cuadrícula interactiva de aplicaciones del sistema con filtrado en tiempo real.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 * @license GPL-3.0
 */

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import org.kde.kirigami as Kirigami
import "./org/kde/plasma/ravenlauncher/plugin" as RavenPlugin

/**
 * @class AppGridView
 * @brief Componente de cuadrícula de aplicaciones con buscador integrado y navegación por teclado.
 */
Item {
    id: root
    signal appLaunched(string appUrl, string execCmd)
    signal escapeRequested()

    property int selectedIndex: 0

    RavenPlugin.AppFilterModel {
        id: appRunner
        onCountChanged: {
            if (root.selectedIndex >= appRunner.count) {
                root.selectedIndex = Math.max(0, appRunner.count - 1);
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 8

        // Search bar
        TextField {
            id: searchField
            Layout.fillWidth: true
            Layout.preferredHeight: 38
            leftPadding: 34
            rightPadding: text.length > 0 ? 30 : 12
            placeholderText: i18n("Buscar aplicaciones...")
            color: RavenPlugin.RavenTheme.textColor
            placeholderTextColor: RavenPlugin.RavenTheme.subTextColor
            font.pixelSize: 12
            focus: true

            Kirigami.Icon {
                anchors.left: parent.left
                anchors.leftMargin: 10
                anchors.verticalCenter: parent.verticalCenter
                source: "search"
                implicitWidth: 16
                implicitHeight: 16
                opacity: 0.6
            }

            // Clear search button
            Kirigami.Icon {
                anchors.right: parent.right
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                source: "edit-clear"
                implicitWidth: 16
                implicitHeight: 16
                visible: searchField.text.length > 0
                opacity: clearMa.containsMouse ? 1.0 : 0.6
                MouseArea {
                    id: clearMa
                    anchors.fill: parent
                    hoverEnabled: true
                    onClicked: {
                        searchField.text = ""
                        searchField.forceActiveFocus()
                    }
                }
            }

            background: Rectangle {
                radius: RavenPlugin.RavenTheme.radiusSm
                color: RavenPlugin.RavenTheme.surfaceElevated
                border.width: 1
                border.color: searchField.activeFocus
                    ? RavenPlugin.RavenTheme.highlightColor
                    : RavenPlugin.RavenTheme.cardBorder
                Behavior on border.color { ColorAnimation { duration: 150 } }
            }

            onTextChanged: {
                appRunner.searchFilter = text;
                root.selectedIndex = 0;
            }

            onAccepted: {
                if (appRunner.count > 0) {
                    appRunner.launchIndex(root.selectedIndex);
                    root.appLaunched("", "");
                }
            }

            Keys.onDownPressed: (event) => {
                if (appRunner.count > 0) {
                    root.selectedIndex = Math.min(root.selectedIndex + 3, appRunner.count - 1);
                    ensureVisible(root.selectedIndex);
                    event.accepted = true;
                }
            }

            Keys.onUpPressed: (event) => {
                if (appRunner.count > 0) {
                    root.selectedIndex = Math.max(root.selectedIndex - 3, 0);
                    ensureVisible(root.selectedIndex);
                    event.accepted = true;
                }
            }

            Keys.onRightPressed: (event) => {
                if (appRunner.count > 0 && root.selectedIndex < appRunner.count - 1) {
                    root.selectedIndex++;
                    ensureVisible(root.selectedIndex);
                    event.accepted = true;
                }
            }

            Keys.onLeftPressed: (event) => {
                if (appRunner.count > 0 && root.selectedIndex > 0) {
                    root.selectedIndex--;
                    ensureVisible(root.selectedIndex);
                    event.accepted = true;
                }
            }

            Keys.onReturnPressed: (event) => {
                if (appRunner.count > 0) {
                    appRunner.launchIndex(root.selectedIndex);
                    root.appLaunched("", "");
                    event.accepted = true;
                }
            }

            Keys.onEnterPressed: (event) => {
                if (appRunner.count > 0) {
                    appRunner.launchIndex(root.selectedIndex);
                    root.appLaunched("", "");
                    event.accepted = true;
                }
            }

            Keys.onEscapePressed: (event) => {
                if (searchField.text.length > 0) {
                    searchField.text = "";
                    event.accepted = true;
                } else {
                    root.escapeRequested();
                }
            }
        }

        // App grid with ScrollView & ScrollBar
        ScrollView {
            id: scrollView
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical.policy: ScrollBar.AsNeeded

            Grid {
                id: appGrid
                width: scrollView.availableWidth
                columns: 3
                spacing: 0

                Repeater {
                    model: appRunner
                    delegate: Item {
                        id: appDelegate
                        width: Math.floor(appGrid.width / 3)
                        height: 82

                        readonly property bool isCurrentSelected: (index === root.selectedIndex)

                        MouseArea {
                            id: ma
                            anchors.fill: parent
                            anchors.margins: 3
                            hoverEnabled: true
                            onEntered: root.selectedIndex = index
                            onClicked: {
                                appRunner.launchApp(model.execCmd, model.desktopPath)
                                root.appLaunched(model.desktopPath, model.execCmd)
                            }

                            Rectangle {
                                anchors.fill: parent
                                radius: RavenPlugin.RavenTheme.radiusSm
                                color: (ma.containsMouse || isCurrentSelected)
                                    ? RavenPlugin.RavenTheme.highlightColor
                                    : "transparent"
                                opacity: isCurrentSelected ? 0.22 : (ma.containsMouse ? 0.14 : 0)
                                border.width: isCurrentSelected ? 1 : 0
                                border.color: RavenPlugin.RavenTheme.highlightColor
                                Behavior on opacity { NumberAnimation { duration: 120 } }
                            }

                            ColumnLayout {
                                anchors.centerIn: parent
                                spacing: 4

                                Kirigami.Icon {
                                    Layout.alignment: Qt.AlignHCenter
                                    source: model.iconName
                                    implicitWidth: 40
                                    implicitHeight: 40
                                }

                                Text {
                                    Layout.maximumWidth: appDelegate.width - 12
                                    text: model.appName
                                    horizontalAlignment: Text.AlignHCenter
                                    elide: Text.ElideRight
                                    color: RavenPlugin.RavenTheme.textColor
                                    font.pixelSize: 11
                                    font.bold: isCurrentSelected
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /**
     * @brief Actualiza la lista de aplicaciones instaladas recargando el modelo del plugin.
     */
    function refresh() {
        appRunner.refresh();
    }

    /**
     * @brief Limpia el campo de búsqueda de texto y restaura el foco activo en él.
     */
    function resetSearch() {
        searchField.text = "";
        searchField.forceActiveFocus();
    }

    /**
     * @brief Asegura que el elemento en el índice indicado sea visible dentro del área con scroll.
     * @param {number} idx Índice del elemento seleccionado.
     */
    function ensureVisible(idx) {
        var row = Math.floor(idx / 3);
        var itemY = row * 82;
        if (itemY < scrollView.contentItem.contentY) {
            scrollView.contentItem.contentY = itemY;
        } else if (itemY + 82 > scrollView.contentItem.contentY + scrollView.height) {
            scrollView.contentItem.contentY = itemY + 82 - scrollView.height;
        }
    }
}
