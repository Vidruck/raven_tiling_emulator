import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami
import "./org/kde/plasma/ravenlauncher/plugin" as RavenPlugin

PlasmoidItem {
    id: root
    Plasmoid.icon: (RavenPlugin.SystemStats && RavenPlugin.SystemStats.distroIcon) ? RavenPlugin.SystemStats.distroIcon : "start-here-kde"
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
            source: (RavenPlugin.SystemStats && RavenPlugin.SystemStats.distroIcon) ? RavenPlugin.SystemStats.distroIcon : "start-here-kde"
            fallback: "kde"
            active: compactRoot.containsMouse
        }
    }

    PlasmaCore.Dialog {
        id: centerDialog
        location: PlasmaCore.Types.Floating
        flags: Qt.WindowStaysOnTopHint | Qt.FramelessWindowHint
        backgroundHints: PlasmaCore.Types.NoBackground
        hideOnWindowDeactivate: true
        visible: false
        width: 495
        height: (Plasmoid.screenGeometry && Plasmoid.screenGeometry.height > 0) ? Math.min(Plasmoid.screenGeometry.height - 100, 900) : 900

        Component.onCompleted: {
            var screenW = (Plasmoid.screenGeometry && Plasmoid.screenGeometry.width > 0) ? Plasmoid.screenGeometry.width : 1920
            var screenH = (Plasmoid.screenGeometry && Plasmoid.screenGeometry.height > 0) ? Plasmoid.screenGeometry.height : 1080
            x = (screenW - 495) / 2
            y = (screenH - 880) / 2
        }

        mainItem: Item {
            id: dialogContent
            
            Layout.minimumWidth: 400
            Layout.maximumWidth: 1000
            Layout.preferredWidth: 495
            Layout.minimumHeight: 700
            Layout.maximumHeight: 1200
            Layout.preferredHeight: 880
            
            implicitWidth: 495
            implicitHeight: 880

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
        Layout.minimumWidth: 400
        Layout.minimumHeight: 700
        Layout.preferredWidth: 495
        Layout.preferredHeight: 880
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
