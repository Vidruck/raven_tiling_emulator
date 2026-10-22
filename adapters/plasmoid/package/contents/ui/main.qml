/**
 * @file main.qml
 * @brief Componente raíz del Plasmoide de Raven para KDE Plasma 6 (PlasmoidItem).
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 * @license GPL-3.0
 */

import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import "./org/kde/plasma/ravenlauncher/plugin" as RavenPlugin

/**
 * @class main
 * @brief Elemento raíz del applet que gestiona la representación compacta en el panel y el diálogo flotante central.
 */
PlasmoidItem {
    id: root
    Plasmoid.icon: (Plasmoid.configuration.plasmoidIcon && Plasmoid.configuration.plasmoidIcon.length > 0) ? Plasmoid.configuration.plasmoidIcon : ((RavenPlugin.SystemStats && RavenPlugin.SystemStats.distroIcon) ? RavenPlugin.SystemStats.distroIcon : "start-here-kde")
    Plasmoid.backgroundHints: PlasmaCore.Types.NoBackground
    preferredRepresentation: compactRepresentation

    compactRepresentation: MouseArea {
        id: compactRoot
        Layout.minimumWidth: Kirigami.Units.iconSizes.small
        Layout.minimumHeight: Kirigami.Units.iconSizes.small
        Layout.preferredWidth: Kirigami.Units.iconSizes.medium
        Layout.preferredHeight: Kirigami.Units.iconSizes.medium
        onClicked: root.expanded = !root.expanded
        Kirigami.Icon {
            id: appletIcon
            anchors.fill: parent
            anchors.margins: Math.round(Kirigami.Units.smallSpacing / 2)
            source: (Plasmoid.configuration.plasmoidIcon && Plasmoid.configuration.plasmoidIcon.length > 0) ? Plasmoid.configuration.plasmoidIcon : ((RavenPlugin.SystemStats && RavenPlugin.SystemStats.distroIcon) ? RavenPlugin.SystemStats.distroIcon : "start-here-kde")
            fallback: "kde"
            active: compactRoot.containsMouse
        }
    }

    PlasmaCore.Dialog {
        id: centerDialog
        location: PlasmaCore.Types.Floating
        flags: Qt.WindowStaysOnTopHint | Qt.FramelessWindowHint
        // Prevenir que el panel Adaptable fuerce un fondo opaco/sólido de Plasma
        backgroundHints: PlasmaCore.Dialog.NoBackground
        hideOnWindowDeactivate: true
        visible: false

        property int targetWidth: 410
        property int maxAllowedHeight: {
            var screenH = (Plasmoid.screenGeometry && Plasmoid.screenGeometry.height > 0) ? Plasmoid.screenGeometry.height : 1080;
            return Math.max(300, screenH - 80);
        }

        width: targetWidth
        height: Math.min(dialogContent.implicitHeight, maxAllowedHeight)

        Component.onCompleted: updatePosition()
        onWidthChanged: updatePosition()
        onHeightChanged: updatePosition()

        function updatePosition() {
            var screenW = (Plasmoid.screenGeometry && Plasmoid.screenGeometry.width > 0) ? Plasmoid.screenGeometry.width : 1920;
            var screenH = (Plasmoid.screenGeometry && Plasmoid.screenGeometry.height > 0) ? Plasmoid.screenGeometry.height : 1080;
            x = Math.round((screenW - width) / 2);
            y = Math.round((screenH - height) / 2);
        }

        mainItem: Item {
            id: dialogContent
            
            Layout.minimumWidth: 410
            Layout.maximumWidth: 410
            Layout.preferredWidth: centerDialog.targetWidth
            Layout.preferredHeight: mainViewDialog.contentHeight
            Layout.minimumHeight: mainViewDialog.contentHeight
            Layout.maximumHeight: mainViewDialog.contentHeight
            
            implicitWidth: centerDialog.targetWidth
            implicitHeight: mainViewDialog.contentHeight

            onImplicitHeightChanged: {
                centerDialog.height = Qt.binding(function() {
                    return Math.min(dialogContent.implicitHeight, centerDialog.maxAllowedHeight);
                });
                centerDialog.updatePosition();
            }

            MainWindowView {
                id: mainViewDialog
                anchors.fill: parent
                appletExpanded: centerDialog.visible
                onAppClicked: (appUrl, execCmd) => {
                    centerDialog.visible = false
                    root.expanded = false
                }
            }
        }
    }

    fullRepresentation: Item {
        id: fullRepItem
        Layout.minimumWidth: 410
        Layout.maximumWidth: 410
        Layout.preferredWidth: 410
        Layout.preferredHeight: mainViewFull.contentHeight
        Layout.minimumHeight: mainViewFull.contentHeight
        Layout.maximumHeight: mainViewFull.contentHeight
        implicitWidth: 410
        implicitHeight: mainViewFull.contentHeight

        MainWindowView {
            id: mainViewFull
            anchors.fill: parent
            appletExpanded: root.expanded
            onAppClicked: (appUrl, execCmd) => {
                root.expanded = false
            }
        }
    }

    Connections {
        target: root
        function onExpandedChanged() {
            if (Plasmoid.location === PlasmaCore.Types.Floating) {
                if (root.expanded) {
                    centerDialog.height = Qt.binding(function() {
                        return Math.min(dialogContent.implicitHeight, centerDialog.maxAllowedHeight);
                    });
                    centerDialog.updatePosition();
                }
                centerDialog.visible = root.expanded;
            }
        }
    }
}
