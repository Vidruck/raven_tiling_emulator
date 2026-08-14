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
    property string desktopStatus: " - | Escritorio : 1 | - "

    property string queryCmd: "qdbus6 org.kde.raven.Daemon /Events org.kde.raven.Events.getTilingState"
    property string queryMonitorsCmd: "qdbus6 org.kde.raven.Daemon /Events org.kde.raven.Events.getMonitorCount"
    property string queryDesktopsCmd: "qdbus6 org.kde.raven.Daemon /Events org.kde.raven.Events.getDesktopStatus"

    /**
     * Ejecuta un comando mapeándolo a los atajos globales de KWin para activar la arquitectura Single-Trip.
     *
     * @param {string} method - Método lógico a invocar.
     * @param {var} args - Argumentos adicionales (como incrementos de gaps).
     */
    function execDbus(method, args) {
        let shortcutMap = {
            "toggleTiling": "RavenToggleTiling",
            "focusPrev": "RavenFocusPrev",
            "focusNext": "RavenFocusNext",
            "swapPrev": "RavenSwapPrev",
            "swapNext": "RavenSwapNext",
            "increaseRatio": "RavenIncreaseRatio",
            "decreaseRatio": "RavenDecreaseRatio",
            "migrateActiveToScreen": "RavenMigrateMonitor",
            "migrateActiveToPrevScreen": "RavenMigratePrevMonitor",
            "migrateActiveToDesktop": "RavenMigrateDesktop",
            "migrateActiveToPrevDesktop": "RavenMigratePrevDesktop",
            "incrementMaster": "RavenIncrementMaster",
            "decrementMaster": "RavenDecrementMaster"
        };
        
        let targetShortcut = shortcutMap[method] || "";
        
        if (method === "incrementGaps") {
            if (args === "2" || args === 2 || args === "int32:2") {
                targetShortcut = "RavenIncrementGaps";
            } else if (args === "-2" || args === -2 || args === "int32:-2") {
                targetShortcut = "RavenDecrementGaps";
            }
        }
        
        if (targetShortcut !== "") {
            let cmd = "qdbus6 org.kde.kglobalaccel /component/kwin invokeShortcut " + targetShortcut;
            executable.exec(cmd);
        }
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
                    root.desktopStatus = output;
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
        implicitWidth: Kirigami.Units.gridUnit * 16
        implicitHeight: Kirigami.Units.gridUnit * 17.5
        background: null

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: Kirigami.Units.mediumSpacing
            spacing: Kirigami.Units.mediumSpacing

            // ── Tarjeta Hero Top (Estado & Toggle Switch) ──
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: Kirigami.Units.gridUnit * 2.8
                radius: 12
                color: Kirigami.Theme.backgroundColor
                border.color: root.isEngineEnabled ? Kirigami.Theme.highlightColor : Qt.rgba(1, 1, 1, 0.1)
                border.width: 1.5

                RowLayout {
                    anchors.fill: parent
                    anchors.margins: Kirigami.Units.mediumSpacing
                    spacing: Kirigami.Units.smallSpacing

                    Kirigami.Icon {
                        source: "org.kde.raven.tiling"
                        implicitWidth: Kirigami.Units.iconSizes.smallMedium
                        implicitHeight: Kirigami.Units.iconSizes.smallMedium
                    }

                    ColumnLayout {
                        spacing: 1
                        PlasmaComponents.Label {
                            text: "Raven Tiling Emulator"
                            font.bold: true
                            font.pixelSize: Kirigami.Units.gridUnit * 0.75
                        }
                        PlasmaComponents.Label {
                            text: root.isEngineEnabled ? "● Modo Mosaico" : "○ Modo Flotantes"
                            opacity: 0.8
                            color: root.isEngineEnabled ? Kirigami.Theme.highlightColor : Kirigami.Theme.disabledTextColor
                            font.pixelSize: Kirigami.Units.gridUnit * 0.6
                        }
                    }

                    Item { Layout.fillWidth: true }

                    PlasmaComponents.Switch {
                        checked: root.isEngineEnabled
                        onClicked: root.toggleRaven()
                    }
                }
            }

            // ── Sección 1: Carrusel Horizontal de Algoritmos (Material Carousel) ──
            ColumnLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                PlasmaComponents.Label {
                    text: "Algoritmos"
                    font.bold: true
                    opacity: 0.8
                    font.pixelSize: Kirigami.Units.gridUnit * 0.65
                }

                Component {
                    id: layoutCardDelegate
                    Rectangle {
                        width: Kirigami.Units.gridUnit * 5.2
                        height: Kirigami.Units.gridUnit * 3.6
                        radius: 10
                        color: layoutListView.currentIndex === index ? Kirigami.Theme.highlightColor : Qt.rgba(Kirigami.Theme.backgroundColor.r, Kirigami.Theme.backgroundColor.g, Kirigami.Theme.backgroundColor.b, 0.6)
                        border.color: layoutListView.currentIndex === index ? Kirigami.Theme.highlightColor : Qt.rgba(1, 1, 1, 0.12)
                        border.width: 1.5

                        Behavior on color { ColorAnimation { duration: Kirigami.Units.shortDuration } }

                        MouseArea {
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: {
                                layoutListView.currentIndex = index;
                                var val = model.value;
                                executable.exec("qdbus6 org.kde.raven.Daemon /Events org.kde.raven.Events.setLayoutForCurrentWorkspace " + val);
                            }
                        }

                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 2

                            Kirigami.Icon {
                                source: model.iconName
                                Layout.alignment: Qt.AlignHCenter
                                implicitWidth: Kirigami.Units.iconSizes.small
                                implicitHeight: Kirigami.Units.iconSizes.small
                                active: layoutListView.currentIndex === index
                            }

                            PlasmaComponents.Label {
                                text: model.text
                                Layout.alignment: Qt.AlignHCenter
                                font.bold: layoutListView.currentIndex === index
                                font.pixelSize: Kirigami.Units.gridUnit * 0.55
                                color: layoutListView.currentIndex === index ? Kirigami.Theme.highlightedTextColor : Kirigami.Theme.textColor
                                elide: Text.ElideRight
                                maximumLineCount: 1
                                horizontalAlignment: Text.AlignHCenter
                            }
                        }
                    }
                }

                ListView {
                    id: layoutListView
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.gridUnit * 3.8
                    orientation: ListView.Horizontal
                    spacing: Kirigami.Units.smallSpacing
                    clip: true
                    snapMode: ListView.SnapToItem

                    model: ListModel {
                        ListElement { text: "Raven"; value: "raven"; iconName: "view-grid" }
                        ListElement { text: "Clásico"; value: "tall"; iconName: "view-split-left-right" }
                        ListElement { text: "Monóculo"; value: "monocle"; iconName: "view-fullscreen" }
                        ListElement { text: "Avanzado"; value: "strict_dwindle"; iconName: "view-list-tree" }
                        ListElement { text: "Invertido"; value: "inverted_strict_dwindle"; iconName: "view-split-top-bottom" }
                        ListElement { text: "Divisor"; value: "divisor"; iconName: "view-file-columns" }
                    }

                    delegate: layoutCardDelegate
                }
            }

            // ── Sección 2: Gestión de Ventanas y Foco (Icon-Only Pill Buttons) ──
            GridLayout {
                columns: 2
                Layout.fillWidth: true
                rowSpacing: Kirigami.Units.smallSpacing
                columnSpacing: Kirigami.Units.smallSpacing

                // Tarjeta Foco
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.gridUnit * 3.2
                    radius: 10
                    color: Qt.rgba(Kirigami.Theme.backgroundColor.r, Kirigami.Theme.backgroundColor.g, Kirigami.Theme.backgroundColor.b, 0.5)
                    border.color: Qt.rgba(1, 1, 1, 0.08)

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.smallSpacing
                        spacing: 2

                        PlasmaComponents.Label {
                            text: "Foco"
                            Layout.alignment: Qt.AlignHCenter
                            opacity: 0.75
                            font.pixelSize: Kirigami.Units.gridUnit * 0.58
                        }

                        RowLayout {
                            spacing: Kirigami.Units.smallSpacing
                            PlasmaComponents.Button {
                                icon.name: "go-previous"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("focusPrev", "")
                                PlasmaComponents.ToolTip.text: "Navegar a ventana anterior"
                            }
                            PlasmaComponents.Button {
                                icon.name: "go-next"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("focusNext", "")
                                PlasmaComponents.ToolTip.text: "Navegar a ventana siguiente"
                            }
                        }
                    }
                }

                // Tarjeta Intercambiar
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.gridUnit * 3.2
                    radius: 10
                    color: Qt.rgba(Kirigami.Theme.backgroundColor.r, Kirigami.Theme.backgroundColor.g, Kirigami.Theme.backgroundColor.b, 0.5)
                    border.color: Qt.rgba(1, 1, 1, 0.08)

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.smallSpacing
                        spacing: 2

                        PlasmaComponents.Label {
                            text: "Intercambiar"
                            Layout.alignment: Qt.AlignHCenter
                            opacity: 0.75
                            font.pixelSize: Kirigami.Units.gridUnit * 0.58
                        }

                        RowLayout {
                            spacing: Kirigami.Units.smallSpacing
                            PlasmaComponents.Button {
                                icon.name: "edit-undo"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("swapPrev", "")
                                PlasmaComponents.ToolTip.text: "Intercambiar posición hacia atrás"
                            }
                            PlasmaComponents.Button {
                                icon.name: "edit-redo"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("swapNext", "")
                                PlasmaComponents.ToolTip.text: "Intercambiar posición hacia adelante"
                            }
                        }
                    }
                }
            }

            // ── Sección 3: Ajustes de Espacio (Icon-Only Pill Buttons) ──
            GridLayout {
                columns: 2
                Layout.fillWidth: true
                rowSpacing: Kirigami.Units.smallSpacing
                columnSpacing: Kirigami.Units.smallSpacing

                // Tarjeta Ratio
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.gridUnit * 3.2
                    radius: 10
                    color: Qt.rgba(Kirigami.Theme.backgroundColor.r, Kirigami.Theme.backgroundColor.g, Kirigami.Theme.backgroundColor.b, 0.5)
                    border.color: Qt.rgba(1, 1, 1, 0.08)

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.smallSpacing
                        spacing: 2

                        PlasmaComponents.Label {
                            text: "Ratio Máster"
                            Layout.alignment: Qt.AlignHCenter
                            opacity: 0.75
                            font.pixelSize: Kirigami.Units.gridUnit * 0.58
                        }

                        RowLayout {
                            spacing: Kirigami.Units.smallSpacing
                            PlasmaComponents.Button {
                                icon.name: "format-justify-left"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("decreaseRatio", "")
                                PlasmaComponents.ToolTip.text: "Contraer tamaño maestro"
                            }
                            PlasmaComponents.Button {
                                icon.name: "format-justify-right"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("increaseRatio", "")
                                PlasmaComponents.ToolTip.text: "Expandir tamaño maestro"
                            }
                        }
                    }
                }

                // Tarjeta Márgenes
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.gridUnit * 3.2
                    radius: 10
                    color: Qt.rgba(Kirigami.Theme.backgroundColor.r, Kirigami.Theme.backgroundColor.g, Kirigami.Theme.backgroundColor.b, 0.5)
                    border.color: Qt.rgba(1, 1, 1, 0.08)

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.smallSpacing
                        spacing: 2

                        PlasmaComponents.Label {
                            text: "Márgenes"
                            Layout.alignment: Qt.AlignHCenter
                            opacity: 0.75
                            font.pixelSize: Kirigami.Units.gridUnit * 0.58
                        }

                        RowLayout {
                            spacing: Kirigami.Units.smallSpacing
                            PlasmaComponents.Button {
                                icon.name: "zoom-out"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("incrementGaps", "-2")
                                PlasmaComponents.ToolTip.text: "Reducir márgenes (Gaps)"
                            }
                            PlasmaComponents.Button {
                                icon.name: "zoom-in"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("incrementGaps", "2")
                                PlasmaComponents.ToolTip.text: "Ampliar márgenes (Gaps)"
                            }
                        }
                    }
                }
            }

            // ── Sección 4: Migración a Escritorios / Monitores ──
            GridLayout {
                columns: 2
                Layout.fillWidth: true
                rowSpacing: Kirigami.Units.smallSpacing
                columnSpacing: Kirigami.Units.smallSpacing

                // Monitor
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.gridUnit * 3.2
                    radius: 10
                    color: Qt.rgba(Kirigami.Theme.backgroundColor.r, Kirigami.Theme.backgroundColor.g, Kirigami.Theme.backgroundColor.b, 0.5)
                    border.color: Qt.rgba(1, 1, 1, 0.08)

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.smallSpacing
                        spacing: 2

                        PlasmaComponents.Label {
                            text: "Monitor (" + root.monitorCount + ")"
                            Layout.alignment: Qt.AlignHCenter
                            opacity: 0.75
                            font.pixelSize: Kirigami.Units.gridUnit * 0.58
                        }

                        RowLayout {
                            spacing: Kirigami.Units.smallSpacing
                            PlasmaComponents.Button {
                                icon.name: "go-previous"
                                Layout.fillWidth: true
                                enabled: root.monitorCount > 1
                                onClicked: root.execDbus("migrateActiveToPrevScreen", "")
                                PlasmaComponents.ToolTip.text: "Migrar ventana al monitor anterior"
                            }
                            PlasmaComponents.Button {
                                icon.name: "go-next"
                                Layout.fillWidth: true
                                enabled: root.monitorCount > 1
                                onClicked: root.execDbus("migrateActiveToScreen", "")
                                PlasmaComponents.ToolTip.text: "Migrar ventana al monitor siguiente"
                            }
                        }
                    }
                }

                // Escritorio
                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: Kirigami.Units.gridUnit * 3.2
                    radius: 10
                    color: Qt.rgba(Kirigami.Theme.backgroundColor.r, Kirigami.Theme.backgroundColor.g, Kirigami.Theme.backgroundColor.b, 0.5)
                    border.color: Qt.rgba(1, 1, 1, 0.08)

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: Kirigami.Units.smallSpacing
                        spacing: 2

                        PlasmaComponents.Label {
                            text: root.desktopStatus
                            Layout.alignment: Qt.AlignHCenter
                            opacity: 0.75
                            font.pixelSize: Kirigami.Units.gridUnit * 0.58
                            elide: Text.ElideRight
                        }

                        RowLayout {
                            spacing: Kirigami.Units.smallSpacing
                            PlasmaComponents.Button {
                                icon.name: "go-previous"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("migrateActiveToPrevDesktop", "")
                                PlasmaComponents.ToolTip.text: "Migrar ventana al escritorio anterior"
                            }
                            PlasmaComponents.Button {
                                icon.name: "go-next"
                                Layout.fillWidth: true
                                onClicked: root.execDbus("migrateActiveToDesktop", "")
                                PlasmaComponents.ToolTip.text: "Migrar ventana al escritorio siguiente"
                            }
                        }
                    }
                }
            }

            Item { Layout.fillHeight: true }

            PlasmaComponents.Label {
                text: "© 2026 Vidruck"
                Layout.alignment: Qt.AlignHCenter
                opacity: 0.35
                font.pixelSize: Kirigami.Units.gridUnit * 0.55
            }
        }
    }
}
