import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import org.kde.plasma.plasmoid
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasma5support as Plasma5Support
import org.kde.plasma.components as PlasmaComponents

PlasmoidItem {
    id: root

    property bool isEngineEnabled: true
    property int monitorCount: 1
    property int desktopCount: 1

    property string queryCmd: "qdbus6 org.kde.raven.Daemon /Events org.kde.raven.Events.getTilingState"
    property string queryMonitorsCmd: "qdbus6 org.kde.raven.Daemon /Events org.kde.raven.Events.getMonitorCount"
    property string queryDesktopsCmd: "qdbus6 org.kde.raven.Daemon /Events org.kde.raven.Events.getDesktopCount"

    /**
     * Ejecuta un comando en el bus de datos (D-Bus) dirigido al demonio (daemon) de Raven.
     *
     * @param {string} method - Método de D-Bus a invocar.
     * @param {var} args - Argumentos adicionales para el método.
     */
    function execDbus(method, args) {
        let cmd = "qdbus6 org.kde.raven.Daemon /Events org.kde.raven.Events." + method;
        if (args) {
            let cleanArgs = args.toString().replace("int32:", "");
            cmd += " " + cleanArgs;
        }
        executable.exec(cmd);
    }

    /**
     * Alterna de forma interactiva el estado del motor de mosaico (tiling engine).
     */
    function toggleRaven() {
        execDbus("toggleTiling", "");
        root.isEngineEnabled = !root.isEngineEnabled;
    }

    /**
     * Consulta el estado del motor y de la topología del compositor.
     */
    function queryState() {
        executable.exec(queryCmd);
        executable.exec(queryMonitorsCmd);
        executable.exec(queryDesktopsCmd);
    }

    onExpandedChanged: {
        if (expanded) {
            queryState();
        }
    }

    Plasma5Support.DataSource {
        id: executable
        engine: "executable"
        connectedSources: []

        /**
         * Manejador de eventos (event handler) activado al recibir salida estándar (stdout) de un comando.
         */
        onNewData: (sourceName, data) => {
            if (data["stdout"] !== undefined) {
                let output = data["stdout"].trim();
                let cleanOutput = output.replace(/[a-zA-Z]/g, "").trim();

                if (sourceName === root.queryCmd) {
                    root.isEngineEnabled = output.toLowerCase().includes("true");
                } else if (sourceName === root.queryMonitorsCmd) {
                    let val = parseInt(cleanOutput, 10);
                    root.monitorCount = isNaN(val) ? 1 : val;
                } else if (sourceName === root.queryDesktopsCmd) {
                    let val = parseInt(cleanOutput, 10);
                    root.desktopCount = isNaN(val) ? 1 : val;
                }
            }
            disconnectSource(sourceName);
        }

        /**
         * Ejecuta una instrucción del sistema de archivos en segundo plano.
         *
         * @param {string} cmd - Comando shell a ejecutar.
         */
        function exec(cmd) {
            connectSource(cmd);
        }
    }

    compactRepresentation: MouseArea {
        id: compactRoot
        activeFocusOnTab: true
        onClicked: root.expanded = !root.expanded

        Kirigami.Icon {
            anchors.fill: parent
            anchors.margins: Kirigami.Units.smallSpacing
            source: "view-grid"
            active: root.isEngineEnabled
            opacity: root.isEngineEnabled ? 1.0 : 0.4
            Behavior on opacity {
                OpacityAnimator {
                    duration: Kirigami.Units.longDuration
                }
            }
        }

        PlasmaComponents.ToolTip {
            text: "Raven Tiling: " + (root.isEngineEnabled ? "Activo" : "Inactivo")
        }
    }

    fullRepresentation: Kirigami.Page {
        implicitWidth: Kirigami.Units.gridUnit * 18
        implicitHeight: Kirigami.Units.gridUnit * 20
        background: null

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Kirigami.Units.largeSpacing
            spacing: Kirigami.Units.largeSpacing

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.mediumSpacing

                Kirigami.Icon {
                    source: "org.kde.raven.tiling"
                    implicitWidth: Kirigami.Units.iconSizes.medium
                    implicitHeight: Kirigami.Units.iconSizes.medium
                }

                ColumnLayout {
                    spacing: 0
                    PlasmaComponents.Label {
                        text: "Raven Engine"
                        font.bold: true
                        font.pixelSize: Kirigami.Units.gridUnit * 0.9
                    }
                    PlasmaComponents.Label {
                        text: "v2.8 Master-Stack"
                        opacity: 0.6
                        font.pixelSize: Kirigami.Units.gridUnit * 0.7
                    }
                }

                Item {
                    Layout.fillWidth: true
                }

                PlasmaComponents.Switch {
                    checked: root.isEngineEnabled
                    onClicked: root.toggleRaven()
                }
            }

            Kirigami.Separator {
                Layout.fillWidth: true
            }

            Kirigami.Heading {
                text: "Gestión de Ventanas"
                level: 4
                opacity: 0.8
            }

            GridLayout {
                columns: 2
                Layout.fillWidth: true
                rowSpacing: Kirigami.Units.largeSpacing
                columnSpacing: Kirigami.Units.largeSpacing

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing
                    PlasmaComponents.Label {
                        text: "Foco"
                        Layout.alignment: Qt.AlignHCenter
                        opacity: 0.8
                        font.pixelSize: Kirigami.Units.gridUnit * 0.7
                    }
                    RowLayout {
                        spacing: Kirigami.Units.smallSpacing
                        PlasmaComponents.Button {
                            icon.name: "go-previous"
                            Layout.fillWidth: true
                            onClicked: root.execDbus("focusPrev", "")
                        }
                        PlasmaComponents.Button {
                            icon.name: "go-next"
                            Layout.fillWidth: true
                            onClicked: root.execDbus("focusNext", "")
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing
                    PlasmaComponents.Label {
                        text: "Intercambiar"
                        Layout.alignment: Qt.AlignHCenter
                        opacity: 0.8
                        font.pixelSize: Kirigami.Units.gridUnit * 0.7
                    }
                    RowLayout {
                        spacing: Kirigami.Units.smallSpacing
                        PlasmaComponents.Button {
                            icon.name: "go-previous"
                            Layout.fillWidth: true
                            onClicked: root.execDbus("swapPrev", "")
                        }
                        PlasmaComponents.Button {
                            icon.name: "go-next"
                            Layout.fillWidth: true
                            onClicked: root.execDbus("swapNext", "")
                        }
                    }
                }
            }


            Kirigami.Heading {
                text: "Ajustes de Espacio"
                level: 4
                opacity: 0.8
            }

            GridLayout {
                columns: 2
                Layout.fillWidth: true
                rowSpacing: Kirigami.Units.largeSpacing
                columnSpacing: Kirigami.Units.largeSpacing

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing
                    PlasmaComponents.Label {
                        text: "Ratio División"
                        Layout.alignment: Qt.AlignHCenter
                        opacity: 0.8
                        font.pixelSize: Kirigami.Units.gridUnit * 0.7
                    }
                    RowLayout {
                        spacing: Kirigami.Units.smallSpacing
                        PlasmaComponents.Button {
                            icon.name: "go-previous"
                            Layout.fillWidth: true
                            onClicked: root.execDbus("decreaseRatio", "")
                        }
                        PlasmaComponents.Button {
                            icon.name: "go-next"
                            Layout.fillWidth: true
                            onClicked: root.execDbus("increaseRatio", "")
                        }
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing
                    PlasmaComponents.Label {
                        text: "Márgenes"
                        Layout.alignment: Qt.AlignHCenter
                        opacity: 0.8
                        font.pixelSize: Kirigami.Units.gridUnit * 0.7
                    }
                    RowLayout {
                        spacing: Kirigami.Units.smallSpacing
                        PlasmaComponents.Button {
                            icon.name: "zoom-out"
                            Layout.fillWidth: true
                            onClicked: root.execDbus("incrementGaps", "-2")
                        }
                        PlasmaComponents.Button {
                            icon.name: "zoom-in"
                            Layout.fillWidth: true
                            onClicked: root.execDbus("incrementGaps", "2")
                        }
                    }
                }
            }

            Kirigami.Heading {
                text: "Enviar Foco Activo"
                level: 4
                opacity: 0.8
            }

            GridLayout {
                columns: 2
                Layout.fillWidth: true
                rowSpacing: Kirigami.Units.largeSpacing
                columnSpacing: Kirigami.Units.largeSpacing

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing
                    PlasmaComponents.Label {
                        text: "Monitor | " + root.monitorCount + " |"
                        Layout.alignment: Qt.AlignHCenter
                        opacity: 0.8
                        font.pixelSize: Kirigami.Units.gridUnit * 0.7
                    }
                    RowLayout {
                        spacing: Kirigami.Units.smallSpacing
                        PlasmaComponents.Button {
                            icon.name: "go-previous"
                            Layout.fillWidth: true
                            enabled: root.monitorCount > 1
                            onClicked: root.execDbus("migrateActiveToPrevScreen", "")
                        }
                        PlasmaComponents.Button {
                            icon.name: "go-next"
                            Layout.fillWidth: true
                            enabled: root.monitorCount > 1
                            onClicked: root.execDbus("migrateActiveToScreen", "")
                        }
                    }
                }
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing
                    PlasmaComponents.Label {
                        text: "Escritorio | " + root.desktopCount + " |"
                        Layout.alignment: Qt.AlignHCenter
                        opacity: 0.8
                        font.pixelSize: Kirigami.Units.gridUnit * 0.7
                    }
                    RowLayout {
                        spacing: Kirigami.Units.smallSpacing
                        PlasmaComponents.Button {
                            icon.name: "go-up"
                            Layout.fillWidth: true
                            enabled: root.desktopCount > 1
                            onClicked: root.execDbus("migrateActiveToPrevDesktop", "")
                        }
                        PlasmaComponents.Button {
                            icon.name: "go-down"
                            Layout.fillWidth: true
                            enabled: root.desktopCount > 1
                            onClicked: root.execDbus("migrateActiveToDesktop", "")
                        }
                    }
                }
            }

            Item {
                Layout.fillHeight: true
            }

            PlasmaComponents.Label {
                text: "© 2026 Vidruck"
                Layout.alignment: Qt.AlignHCenter
                opacity: 0.4
                font.pixelSize: Kirigami.Units.gridUnit * 0.6
            }
        }
    }
}
