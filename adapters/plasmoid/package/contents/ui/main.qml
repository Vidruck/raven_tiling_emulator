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
        backgroundHints: PlasmaCore.Dialog.StandardBackground
        hideOnWindowDeactivate: true
        visible: false
        property int calculatedHeight: {
            var rows = (Plasmoid.configuration && Plasmoid.configuration.gridRows >= 3) ? Plasmoid.configuration.gridRows : 10
            var desired = (rows * 82) + 120
            var maxH = (Plasmoid.screenGeometry && Plasmoid.screenGeometry.height > 0) ? (Plasmoid.screenGeometry.height - 100) : 1200
            return Math.min(desired, maxH)
        }

        width: 440
        height: calculatedHeight

        Component.onCompleted: {
            var screenW = (Plasmoid.screenGeometry && Plasmoid.screenGeometry.width > 0) ? Plasmoid.screenGeometry.width : 1920
            var screenH = (Plasmoid.screenGeometry && Plasmoid.screenGeometry.height > 0) ? Plasmoid.screenGeometry.height : 1080
            x = (screenW - 440) / 2
            y = (screenH - height) / 2
        }

        mainItem: Item {
            id: dialogContent
            
            Layout.minimumWidth: 380
            Layout.maximumWidth: 1000
            Layout.preferredWidth: 440
            Layout.minimumHeight: 300
            Layout.maximumHeight: 2000
            Layout.preferredHeight: centerDialog.calculatedHeight
            
            implicitWidth: 440
            implicitHeight: centerDialog.calculatedHeight

            MainWindowView {
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
        Layout.minimumWidth: 380
        Layout.minimumHeight: 300
        Layout.preferredWidth: 440
        Layout.preferredHeight: ((Plasmoid.configuration && Plasmoid.configuration.gridRows >= 3) ? Plasmoid.configuration.gridRows : 10) * 82 + 120
        MainWindowView {
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
                centerDialog.visible = root.expanded;
            }
        }
    }
}
