/**
 * @file MainWindowView.qml
 * @brief Vista y panel principal unificado de Raven Launcher y Command Hub en KDE Plasma 6.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 * @license GPL-3.0
 */

import QtQuick
import QtQuick.Layouts
import QtQuick.Controls
import org.kde.kirigami as Kirigami
import org.kde.plasma.plasmoid
import "./org/kde/plasma/ravenlauncher/plugin" as RavenPlugin

/**
 * @class MainWindowView
 * @brief Interfaz interactiva completa que integra el panel de control de mosaico, monitores, escritorios, cuadrícula de apps, widget multimedia y telemetría de hardware.
 */
Item {
    id: root
    implicitWidth: 440
    implicitHeight: 880

    property bool appletExpanded: false
    property var currentDate: new Date()
    signal appClicked(string appUrl, string execCmd)

    Timer {
        interval: 1000
        running: root.appletExpanded
        repeat: true
        triggeredOnStart: true
        onTriggered: {
            root.currentDate = new Date();
        }
    }

    // Sincronización al desplegar el launcher
    Component.onCompleted: {
        RavenPlugin.SystemStats.active = root.appletExpanded;
        RavenPlugin.SystemStats.refresh();
        RavenPlugin.RavenController.refreshState();
    }
    onAppletExpandedChanged: {
        RavenPlugin.SystemStats.active = root.appletExpanded;
        if (root.appletExpanded) {
            root.currentDate = new Date();
            RavenPlugin.SystemStats.refresh();
            RavenPlugin.RavenController.refreshState();
            if (appGridView) {
                appGridView.refresh();
                appGridView.resetSearch();
            }
        }
    }

    RavenPlugin.SystemController  { id: sysControl }

    // ── Circular Gauge Component ────────────────────────────────────────────
    component CircularGauge : Item {
        id: gaugeRoot
        width: 46
        height: 46
        property real value: 0
        property string title: ""
        property string colorOverride: ""
        
        Canvas {
            id: canvas
            anchors.fill: parent
            onPaint: {
                var ctx = getContext("2d");
                ctx.reset();
                var cx = width / 2;
                var cy = height / 2;
                var radius = (Math.min(width, height) - 7) / 2;
                var startAngle = -Math.PI * 0.75;
                var totalAngle = Math.PI * 1.5;
                var endAngle = startAngle + (totalAngle * Math.min(Math.max(gaugeRoot.value, 0), 100) / 100.0);

                // Background Track
                ctx.beginPath();
                ctx.arc(cx, cy, radius, startAngle, startAngle + totalAngle, false);
                ctx.strokeStyle = RavenPlugin.RavenTheme.isDark ? "rgba(255, 255, 255, 0.10)" : "rgba(0, 0, 0, 0.08)";
                ctx.lineWidth = 3.5;
                ctx.lineCap = "round";
                ctx.stroke();

                // Value Arc
                ctx.beginPath();
                ctx.arc(cx, cy, radius, startAngle, endAngle, false);
                if (gaugeRoot.colorOverride !== "") {
                    ctx.strokeStyle = gaugeRoot.colorOverride;
                } else if (gaugeRoot.value > 85) {
                    ctx.strokeStyle = RavenPlugin.RavenTheme.negativeColor;
                } else if (gaugeRoot.value > 65) {
                    ctx.strokeStyle = "#F39C12";
                } else {
                    ctx.strokeStyle = RavenPlugin.RavenTheme.highlightColor;
                }
                ctx.lineWidth = 3.5;
                ctx.lineCap = "round";
                ctx.stroke();
            }
            Connections {
                target: gaugeRoot
                function onValueChanged() { canvas.requestPaint(); }
                function onColorOverrideChanged() { canvas.requestPaint(); }
            }
            Connections {
                target: RavenPlugin.RavenTheme
                function onHighlightColorChanged() { canvas.requestPaint(); }
                function onIsDarkChanged() { canvas.requestPaint(); }
            }
        }
        
        Column {
            anchors.centerIn: parent
            spacing: 0
            Text {
                text: Math.round(gaugeRoot.value) + "%"
                color: RavenPlugin.RavenTheme.textColor
                font.pixelSize: 10
                font.bold: true
                anchors.horizontalCenter: parent.horizontalCenter
            }
            Text {
                text: gaugeRoot.title
                color: RavenPlugin.RavenTheme.subTextColor
                font.pixelSize: 8
                anchors.horizontalCenter: parent.horizontalCenter
            }
        }
    }

    // ── Island / Card Component ─────────────────────────────────────────────
    component Island : Rectangle {
        color: RavenPlugin.RavenTheme.cardBackground
        border.color: RavenPlugin.RavenTheme.cardBorder
        border.width: 1
        radius: RavenPlugin.RavenTheme.radiusLg
        clip: true
    }

    // ── Layout Principal: Contenido Central + Sidebar Fino de Algoritmos ──────
    RowLayout {
        anchors.fill: parent
        spacing: 8

        // ==========================================
        // COLUMNA PRINCIPAL DE ISLAS VERTICALES
        // ==========================================
        ScrollView {
            id: mainScroll
            Layout.fillWidth: true
            Layout.fillHeight: parent.height
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff
            ScrollBar.vertical.policy: (mainColumn.implicitHeight > root.height) ? ScrollBar.AsNeeded : ScrollBar.AlwaysOff

            ColumnLayout {
                id: mainColumn
                width: mainScroll.availableWidth
                spacing: 8

                // ── ISLA 1: RAVEN COMMAND & CONTROL (REDiseñada) ──
                Island {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 195

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 8

                        // ── CABECERA: TÍTULO/ESTADO (IZQUIERDA) + SWITCH + RELOJ/FECHA (DERECHA) ──
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            // Lado Izquierdo: Marca y Estado Tiling
                            ColumnLayout {
                                spacing: 2
                                Layout.alignment: Qt.AlignVCenter

                                Text {
                                    text: i18n("RAVEN TILING")
                                    color: RavenPlugin.RavenTheme.textColor
                                    font.pixelSize: 12
                                    font.bold: true
                                    font.family: RavenPlugin.RavenTheme.fontFamily || "Noto Sans"
                                    font.letterSpacing: 0.8
                                }

                                Text {
                                    text: RavenPlugin.RavenController.tilingEnabled ? i18n("• Modo Mosaico") : i18n("• Modo Flotante")
                                    color: RavenPlugin.RavenController.tilingEnabled ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.subTextColor
                                    font.pixelSize: 10
                                    font.bold: true
                                }
                            }

                            Item { Layout.fillWidth: true }

                            // Switch Maestro On/Off
                            Rectangle {
                                width: 40; height: 20; radius: 10
                                Layout.alignment: Qt.AlignVCenter
                                color: RavenPlugin.RavenController.tilingEnabled ? RavenPlugin.RavenTheme.highlightColor : Qt.rgba(1, 1, 1, 0.15)
                                Behavior on color { ColorAnimation { duration: 150 } }

                                Rectangle {
                                    width: 14; height: 14; radius: 7
                                    anchors.verticalCenter: parent.verticalCenter
                                    x: RavenPlugin.RavenController.tilingEnabled ? parent.width - width - 3 : 3
                                    color: "#FFFFFF"
                                    Behavior on x { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }
                                }

                                MouseArea {
                                    id: switchMa; anchors.fill: parent; hoverEnabled: true; onClicked: RavenPlugin.RavenController.toggleTiling()
                                }
                                ToolTip.visible: switchMa.containsMouse
                                ToolTip.text: RavenPlugin.RavenController.tilingEnabled ? i18n("Desactivar Tiling") : i18n("Activar Tiling")
                            }

                            // Separador Vertical fino
                            Rectangle {
                                width: 1; height: 24
                                color: RavenPlugin.RavenTheme.cardBorder
                                Layout.alignment: Qt.AlignVCenter
                            }

                            // Lado Derecho: Reloj y Fecha
                            ColumnLayout {
                                spacing: 1
                                Layout.alignment: Qt.AlignRight | Qt.AlignVCenter

                                Text {
                                    text: Qt.formatTime(root.currentDate, "hh:mm")
                                    color: RavenPlugin.RavenTheme.highlightColor
                                    font.pixelSize: 16
                                    font.bold: true
                                    font.family: RavenPlugin.RavenTheme.fixedFontFamily || "Monospace"
                                    Layout.alignment: Qt.AlignRight
                                }

                                Text {
                                    text: Qt.formatDate(root.currentDate, Qt.DefaultLocaleLongDate)
                                    color: RavenPlugin.RavenTheme.subTextColor
                                    font.pixelSize: 9
                                    font.capitalization: Font.Capitalize
                                    elide: Text.ElideRight
                                    Layout.alignment: Qt.AlignRight
                                }
                            }
                        }

                        // ── FILA 1 DE CONTROLES: PANTALLA Y ESCRITORIOS VIRTUALES ──
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            // [ SUB-ISLA PANTALLA ]
                            Rectangle {
                                Layout.preferredWidth: 110
                                Layout.preferredHeight: 48
                                radius: 8
                                color: RavenPlugin.RavenTheme.surfaceElevated || Qt.rgba(1, 1, 1, 0.05)
                                border.width: 1
                                border.color: RavenPlugin.RavenTheme.cardBorder

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 3
                                    spacing: 1

                                    Text {
                                        text: i18n("Pantalla (%1)", RavenPlugin.RavenController.monitorCount)
                                        color: RavenPlugin.RavenTheme.subTextColor
                                        font.pixelSize: 8
                                        font.bold: true
                                        Layout.alignment: Qt.AlignHCenter
                                    }

                                    RowLayout {
                                        Layout.alignment: Qt.AlignHCenter
                                        spacing: 4

                                        Rectangle {
                                            width: 36; height: 22; radius: 5
                                            color: monPrevMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            Kirigami.Icon {
                                                anchors.centerIn: parent
                                                source: "go-previous"
                                                implicitWidth: 11; implicitHeight: 11
                                                color: monPrevMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                            }
                                            MouseArea {
                                                id: monPrevMa; anchors.fill: parent; hoverEnabled: true
                                                onClicked: RavenPlugin.RavenController.migrateActiveToPrevScreen()
                                            }
                                            ToolTip.visible: monPrevMa.containsMouse
                                            ToolTip.text: i18n("Mover ventana al monitor anterior (Meta+Shift+N)")
                                        }

                                        Rectangle {
                                            width: 36; height: 22; radius: 5
                                            color: monNextMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            Kirigami.Icon {
                                                anchors.centerIn: parent
                                                source: "go-next"
                                                implicitWidth: 11; implicitHeight: 11
                                                color: monNextMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                            }
                                            MouseArea {
                                                id: monNextMa; anchors.fill: parent; hoverEnabled: true
                                                onClicked: RavenPlugin.RavenController.migrateActiveToScreen()
                                            }
                                            ToolTip.visible: monNextMa.containsMouse
                                            ToolTip.text: i18n("Mover ventana al monitor siguiente (Meta+Shift+M)")
                                        }
                                    }
                                }
                            }

                            // [ SUB-ISLA ESCRITORIOS VIRTUALES (CARRUSEL) ]
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 48
                                radius: 8
                                color: RavenPlugin.RavenTheme.surfaceElevated || Qt.rgba(1, 1, 1, 0.05)
                                border.width: 1
                                border.color: RavenPlugin.RavenTheme.cardBorder

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 3
                                    spacing: 1

                                    Text {
                                        text: i18n("Escritorios Virtuales")
                                        color: RavenPlugin.RavenTheme.subTextColor
                                        font.pixelSize: 8
                                        font.bold: true
                                        Layout.alignment: Qt.AlignHCenter
                                    }

                                    RowLayout {
                                        Layout.alignment: Qt.AlignHCenter
                                        spacing: 4

                                        // Escritorio Anterior
                                        Rectangle {
                                            width: 34; height: 22; radius: 5
                                            color: dskPrevMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            RowLayout {
                                                anchors.centerIn: parent; spacing: 2
                                                Kirigami.Icon {
                                                    source: "go-previous"; implicitWidth: 9; implicitHeight: 9
                                                    color: dskPrevMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                }
                                                Text {
                                                    text: RavenPlugin.RavenController.prevDesktop
                                                    color: dskPrevMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                    font.pixelSize: 8; font.bold: true
                                                }
                                            }
                                            MouseArea {
                                                id: dskPrevMa; anchors.fill: parent; hoverEnabled: true
                                                onClicked: RavenPlugin.RavenController.migrateActiveToPrevDesktop()
                                            }
                                            ToolTip.visible: dskPrevMa.containsMouse
                                            ToolTip.text: i18n("Mover al Escritorio %1 (Meta+Shift+Left)", RavenPlugin.RavenController.prevDesktop)
                                        }

                                        // Badge Escritorio Actual
                                        Rectangle {
                                            width: 48; height: 22; radius: 5
                                            color: Qt.rgba(RavenPlugin.RavenTheme.highlightColor.r, RavenPlugin.RavenTheme.highlightColor.g, RavenPlugin.RavenTheme.highlightColor.b, 0.20)
                                            border.width: 1
                                            border.color: RavenPlugin.RavenTheme.highlightColor

                                            Text {
                                                anchors.centerIn: parent
                                                text: i18n("Desk %1", RavenPlugin.RavenController.currentDesktop)
                                                color: RavenPlugin.RavenTheme.highlightColor
                                                font.pixelSize: 8
                                                font.bold: true
                                            }
                                        }

                                        // Escritorio Siguiente
                                        Rectangle {
                                            width: 34; height: 22; radius: 5
                                            color: dskNextMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            RowLayout {
                                                anchors.centerIn: parent; spacing: 2
                                                Text {
                                                    text: RavenPlugin.RavenController.nextDesktop
                                                    color: dskNextMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                    font.pixelSize: 8; font.bold: true
                                                }
                                                Kirigami.Icon {
                                                    source: "go-next"; implicitWidth: 9; implicitHeight: 9
                                                    color: dskNextMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                }
                                            }
                                            MouseArea {
                                                id: dskNextMa; anchors.fill: parent; hoverEnabled: true
                                                onClicked: RavenPlugin.RavenController.migrateActiveToDesktop()
                                            }
                                            ToolTip.visible: dskNextMa.containsMouse
                                            ToolTip.text: i18n("Mover al Escritorio %1 (Meta+Shift+Right)", RavenPlugin.RavenController.nextDesktop)
                                        }
                                    }
                                }
                            }
                        }

                        // ── FILA 2 DE CONTROLES: MÁRGENES (GAPS) + INTERCAMBIAR (SWAP) + FLOTAR ──
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            // [ CONTENEDOR MÁRGENES (GAPS) ]
                            Rectangle {
                                Layout.preferredWidth: 110
                                Layout.preferredHeight: 48
                                radius: 8
                                color: RavenPlugin.RavenTheme.surfaceElevated || Qt.rgba(1, 1, 1, 0.05)
                                border.width: 1
                                border.color: RavenPlugin.RavenTheme.cardBorder

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 3
                                    spacing: 1

                                    Text {
                                        text: i18n("Márgenes")
                                        color: RavenPlugin.RavenTheme.subTextColor
                                        font.pixelSize: 8
                                        font.bold: true
                                        Layout.alignment: Qt.AlignHCenter
                                    }

                                    RowLayout {
                                        Layout.alignment: Qt.AlignHCenter
                                        spacing: 4

                                        // Botón -2
                                        Rectangle {
                                            width: 36; height: 22; radius: 5
                                            color: gapsDecMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            RowLayout {
                                                anchors.centerIn: parent; spacing: 2
                                                Kirigami.Icon {
                                                    source: "zoom-out"; implicitWidth: 10; implicitHeight: 10
                                                    color: gapsDecMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                }
                                                Text {
                                                    text: "-2"
                                                    color: gapsDecMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                    font.pixelSize: 8; font.bold: true
                                                }
                                            }
                                            MouseArea {
                                                id: gapsDecMa; anchors.fill: parent; hoverEnabled: true
                                                onClicked: RavenPlugin.RavenController.incrementGaps(-2)
                                            }
                                            ToolTip.visible: gapsDecMa.containsMouse
                                            ToolTip.text: i18n("Reducir separación entre ventanas (Meta+-)")
                                        }

                                        // Botón +2
                                        Rectangle {
                                            width: 36; height: 22; radius: 5
                                            color: gapsIncMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            RowLayout {
                                                anchors.centerIn: parent; spacing: 2
                                                Kirigami.Icon {
                                                    source: "zoom-in"; implicitWidth: 10; implicitHeight: 10
                                                    color: gapsIncMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                }
                                                Text {
                                                    text: "+2"
                                                    color: gapsIncMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                    font.pixelSize: 8; font.bold: true
                                                }
                                            }
                                            MouseArea {
                                                id: gapsIncMa; anchors.fill: parent; hoverEnabled: true
                                                onClicked: RavenPlugin.RavenController.incrementGaps(2)
                                            }
                                            ToolTip.visible: gapsIncMa.containsMouse
                                            ToolTip.text: i18n("Aumentar separación entre ventanas (Meta+=)")
                                        }
                                    }
                                }
                            }

                            // [ CONTENEDOR INTERCAMBIAR POSICIÓN (SWAP) ]
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 48
                                radius: 8
                                color: RavenPlugin.RavenTheme.surfaceElevated || Qt.rgba(1, 1, 1, 0.05)
                                border.width: 1
                                border.color: RavenPlugin.RavenTheme.cardBorder

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 3
                                    spacing: 1

                                    Text {
                                        text: i18n("Intercambiar")
                                        color: RavenPlugin.RavenTheme.subTextColor
                                        font.pixelSize: 8
                                        font.bold: true
                                        Layout.alignment: Qt.AlignHCenter
                                    }

                                    RowLayout {
                                        Layout.alignment: Qt.AlignHCenter
                                        spacing: 6

                                        Rectangle {
                                            width: 44; height: 22; radius: 5
                                            color: swapPrevMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            Kirigami.Icon {
                                                anchors.centerIn: parent
                                                source: "go-previous"; implicitWidth: 11; implicitHeight: 11
                                                color: swapPrevMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                            }
                                            MouseArea {
                                                id: swapPrevMa; anchors.fill: parent; hoverEnabled: true
                                                onClicked: RavenPlugin.RavenController.swapPrev()
                                            }
                                            ToolTip.visible: swapPrevMa.containsMouse
                                            ToolTip.text: i18n("Intercambiar posición hacia atrás (Meta+Shift+K)")
                                        }

                                        Rectangle {
                                            width: 44; height: 22; radius: 5
                                            color: swapNextMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            Kirigami.Icon {
                                                anchors.centerIn: parent
                                                source: "go-next"; implicitWidth: 11; implicitHeight: 11
                                                color: swapNextMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                            }
                                            MouseArea {
                                                id: swapNextMa; anchors.fill: parent; hoverEnabled: true
                                                onClicked: RavenPlugin.RavenController.swapNext()
                                            }
                                            ToolTip.visible: swapNextMa.containsMouse
                                            ToolTip.text: i18n("Intercambiar posición adelante (Meta+Shift+J)")
                                        }
                                    }
                                }
                            }

                            // [ BOTÓN FLOTAR (QUICK PEEK) ]
                            Rectangle {
                                Layout.preferredWidth: 64
                                Layout.preferredHeight: 48
                                radius: 8
                                color: floatMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                border.width: 1
                                border.color: RavenPlugin.RavenTheme.cardBorder

                                ColumnLayout {
                                    anchors.centerIn: parent
                                    spacing: 2

                                    Kirigami.Icon {
                                        source: "view-restore"; implicitWidth: 14; implicitHeight: 14
                                        color: floatMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                        Layout.alignment: Qt.AlignHCenter
                                    }

                                    Text {
                                        text: i18n("Flotar")
                                        color: floatMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                        font.pixelSize: 8; font.bold: true
                                        Layout.alignment: Qt.AlignHCenter
                                    }
                                }

                                MouseArea {
                                    id: floatMa; anchors.fill: parent; hoverEnabled: true
                                    onClicked: RavenPlugin.RavenController.toggleFloating()
                                }
                                ToolTip.visible: floatMa.containsMouse
                                ToolTip.text: i18n("Alternar ventana activa a flotante temporal (Meta+Shift+F)")
                            }
                        }
                    }
                }

                // ── ISLA 2: APP GRID & BÚSQUEDA ───────────────────────────────
                Island {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Math.max(220, Math.min(300, root.height - 520))
                    Layout.minimumHeight: 200

                    AppGridView {
                        id: appGridView
                        anchors.fill: parent
                        anchors.margins: 10
                        onAppLaunched: (appUrl, execCmd) => {
                            root.appClicked(appUrl, execCmd);
                        }
                        onEscapeRequested: {
                            root.appClicked("", "");
                        }
                    }
                }

                // ── ISLA 3: REPRODUCTOR MULTIMEDIA EXPANDIDO  ──
                MediaWidgetView {
                    id: mediaWidget
                    Layout.fillWidth: true
                    Layout.preferredHeight: 155
                    active: root.appletExpanded
                }

                // ── ISLA 4: RECURSOS Y TELEMETRÍA DEL SISTEMA (FOOTER DASHBOARD) ──
                Island {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 118

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 8

                        // Cabecera: Distro & Uptime + Versión de Raven
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 10

                            Kirigami.Icon {
                                source: (RavenPlugin.SystemStats && RavenPlugin.SystemStats.distroIcon) ? RavenPlugin.SystemStats.distroIcon : "start-here-kde"
                                fallback: "kde"
                                implicitWidth: 24
                                implicitHeight: 24
                                Layout.alignment: Qt.AlignVCenter
                            }

                            ColumnLayout {
                                spacing: 1
                                Layout.fillWidth: true

                                Text {
                                    text: RavenPlugin.SystemStats.osName || "Linux"
                                    color: RavenPlugin.RavenTheme.textColor
                                    font.pixelSize: 11
                                    font.bold: true
                                    font.family: RavenPlugin.RavenTheme.fontFamily || "Noto Sans"
                                }

                                Text {
                                    text: i18n("Encendido: ") + (RavenPlugin.SystemStats.uptimeString || "0:00")
                                    color: RavenPlugin.RavenTheme.subTextColor
                                    font.pixelSize: 9
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }

                            Item { Layout.fillWidth: true }

                            // Módulo de CPU con Color del Fabricante (Intel Azul, AMD Rojo, Qualcomm Blanco, etc.)
                            Rectangle {
                                id: cpuPill
                                radius: 5
                                color: Qt.rgba(cpuColor.r, cpuColor.g, cpuColor.b, 0.14)
                                border.width: 1
                                border.color: Qt.rgba(cpuColor.r, cpuColor.g, cpuColor.b, 0.38)
                                implicitWidth: cpuLayout.implicitWidth + 10
                                implicitHeight: 20
                                Layout.alignment: Qt.AlignVCenter

                                readonly property color cpuColor: (RavenPlugin.SystemStats && RavenPlugin.SystemStats.cpuVendorColor) ? RavenPlugin.SystemStats.cpuVendorColor : RavenPlugin.RavenTheme.highlightColor

                                RowLayout {
                                    id: cpuLayout
                                    anchors.centerIn: parent
                                    spacing: 4

                                    Kirigami.Icon {
                                        source: "cpu"
                                        implicitWidth: 12
                                        implicitHeight: 12
                                        color: cpuPill.cpuColor
                                    }

                                    Text {
                                        id: cpuText
                                        text: (RavenPlugin.SystemStats && RavenPlugin.SystemStats.cpuBrandName) ? RavenPlugin.SystemStats.cpuBrandName : (RavenPlugin.SystemStats.cpuModel || "CPU")
                                        color: cpuPill.cpuColor
                                        font.pixelSize: 9
                                        font.bold: true
                                        font.family: RavenPlugin.RavenTheme.fixedFontFamily || "Monospace"
                                    }
                                }

                                ToolTip.visible: cpuMa.containsMouse
                                ToolTip.text: RavenPlugin.SystemStats.cpuModel || i18n("Procesador del Sistema")
                                ToolTip.delay: 200

                                MouseArea {
                                    id: cpuMa
                                    anchors.fill: parent
                                    hoverEnabled: true
                                }
                            }
                        }

                        // Fila de Métricas y Gauges del Sistema
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 16
                            Layout.alignment: Qt.AlignHCenter

                            CircularGauge {
                                value: RavenPlugin.SystemStats.cpuUsage
                                title: i18n("CPU")
                            }
                            CircularGauge {
                                value: RavenPlugin.SystemStats.ramUsage
                                title: i18n("RAM")
                            }
                            CircularGauge {
                                visible: RavenPlugin.SystemStats.hasBattery
                                value: RavenPlugin.SystemStats.batteryUsage
                                title: i18n("BAT")
                                colorOverride: {
                                    if (RavenPlugin.SystemStats.isCharging) return "#2ECC71";
                                    if (RavenPlugin.SystemStats.batteryUsage <= 20) return "#E74C3C";
                                    return "#F39C12";
                                }
                            }
                        }
                    }
                }
            }
        }

        // ==========================================
        // SIDEBAR VERTICAL (ALGORITMOS + SESIÓN SEPARADA)
        // ==========================================
        Island {
            Layout.preferredWidth: 48
            Layout.fillHeight: true

            ColumnLayout {
                id: sidebarCol
                anchors.fill: parent
                anchors.topMargin: 10
                anchors.bottomMargin: 10
                spacing: 6

                // Botón Centro de Control GUI (Reemplaza al icono decorativo)
                Rectangle {
                    width: 36; height: 36; radius: 8
                    Layout.alignment: Qt.AlignHCenter
                    color: ccMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                    Behavior on color { ColorAnimation { duration: 120 } }

                    Kirigami.Icon {
                        anchors.centerIn: parent
                        source: "configure"
                        implicitWidth: 18; implicitHeight: 18
                        color: ccMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.highlightColor
                    }

                    ToolTip.visible: ccMa.containsMouse
                    ToolTip.text: i18n("Abrir Centro de Control Raven (GUI)")
                    ToolTip.delay: 150

                    MouseArea {
                        id: ccMa
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: {
                            RavenPlugin.RavenController.openControlCenter();
                            root.appClicked("", "");
                        }
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Qt.rgba(1, 1, 1, 0.08)
                }

                // ── SECCIÓN 1: LOS 6 ALGORITMOS DE RAVEN ──────────────
                Repeater {
                    model: [
                        { 
                            name: i18n("Raven BSP"),
                            id: "raven",                  
                            iconKey: "dwindle_bsp", 
                            tooltip: i18n("Raven BSP (Espiral Áurea)") 
                        },

                        { 
                            name: i18n("Tall / Master"),          
                            id: "tall",                   
                            iconKey: "tall", 
                            tooltip: i18n("Tall (Ventana Maestra + Pila)") 
                        },

                        { 
                            name: i18n("Monocle"),                
                            id: "monocle",                
                            iconKey: "monocle", 
                            tooltip: i18n("Monocle (Ventana Completa)") 
                        },

                        { 
                            name: i18n("Strict Dwindle"),         
                            id: "strict_dwindle",         
                            iconKey: "strict_dwindle", 
                            tooltip: i18n("Strict Dwindle (Fibonacci Puro)") 
                        },

                        {   
                            name: i18n("Inverted Strict Dwindle"), 
                            id: "inverted_strict_dwindle", 
                            iconKey: "inverted_strict_dwindle", 
                            tooltip: i18n("Inverted Strict Dwindle (Espiral Inversa)") 
                        },

                        { 
                            name: i18n("Divisor"),                
                            id: "divisor",                
                            iconKey: "divisor", 
                            tooltip: i18n("Divisor (Subdivisiones Equitativas)") 
                        }
                    ]
                    delegate: Item {
                        width: 36; height: 36
                        Layout.alignment: Qt.AlignHCenter
                        property bool isActive: RavenPlugin.RavenController.currentLayout === modelData.id
                        property string iconSuffix: RavenPlugin.RavenTheme.isDark ? "dark" : "light"

                        // Indicador sutil de selección activa o hover (halo/borde exterior)
                        Rectangle {
                            anchors.fill: parent
                            radius: 10
                            color: {
                                if (layoutMa.containsMouse) return Qt.rgba(RavenPlugin.RavenTheme.highlightColor.r, RavenPlugin.RavenTheme.highlightColor.g, RavenPlugin.RavenTheme.highlightColor.b, 0.35);
                                if (isActive) return Qt.rgba(RavenPlugin.RavenTheme.highlightColor.r, RavenPlugin.RavenTheme.highlightColor.g, RavenPlugin.RavenTheme.highlightColor.b, 0.20);
                                return "transparent";
                            }
                            border.width: isActive ? 2 : (layoutMa.containsMouse ? 1 : 0)
                            border.color: RavenPlugin.RavenTheme.highlightColor
                            Behavior on color { ColorAnimation { duration: 120 } }
                            Behavior on border.width { NumberAnimation { duration: 120 } }
                        }

                        // Icono SVG nativo ocupando el espacio completo
                        Image {
                            anchors.fill: parent
                            anchors.margins: isActive ? 1 : (layoutMa.containsMouse ? 0 : 2)
                            source: Qt.resolvedUrl("../assets/icon_layouts/" + modelData.iconKey + "_" + iconSuffix + ".svg")
                            sourceSize.width: 72
                            sourceSize.height: 72
                            smooth: true
                            mipmap: true
                            fillMode: Image.PreserveAspectFit
                            opacity: (layoutMa.containsMouse || isActive) ? 1.0 : 0.88
                            scale: layoutMa.pressed ? 0.94 : (layoutMa.containsMouse ? 1.04 : 1.0)
                            Behavior on scale { NumberAnimation { duration: 120; easing.type: Easing.OutQuad } }
                            Behavior on opacity { NumberAnimation { duration: 120 } }
                            Behavior on anchors.margins { NumberAnimation { duration: 120 } }
                        }

                        ToolTip.visible: layoutMa.containsMouse
                        ToolTip.text: modelData.tooltip
                        ToolTip.delay: 150

                        MouseArea {
                            id: layoutMa
                            anchors.fill: parent
                            hoverEnabled: true
                            cursorShape: Qt.PointingHandCursor
                            onClicked: {
                                RavenPlugin.RavenController.setLayout(modelData.id);
                            }
                        }
                    }
                }

                // Botón: Ciclar Algoritmo
                Rectangle {
                    width: 36; height: 36; radius: 8
                    Layout.alignment: Qt.AlignHCenter
                    color: cycleMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                    Kirigami.Icon {
                        anchors.centerIn: parent
                        source: "media-playlist-repeat"
                        implicitWidth: 18; implicitHeight: 18
                    }
                    ToolTip.visible: cycleMa.containsMouse
                    ToolTip.text: i18n("Ciclar siguiente algoritmo (Meta+Shift+L)")
                    ToolTip.delay: 150
                    MouseArea {
                        id: cycleMa
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape: Qt.PointingHandCursor
                        onClicked: RavenPlugin.RavenController.cycleLayout()
                    }
                }

                // Espaciador elástico entre Algoritmos y Sesión
                Item { Layout.fillHeight: true }

                // ── SEPARADOR VISUAL CLARO ────────────────────────────
                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 1
                    color: Qt.rgba(1, 1, 1, 0.12)
                }

                // ── SECCIÓN 2: CONTROLES DE SESIÓN (INTEGRADOS AL SIDEBAR) ──
                Repeater {
                    model: [
                        { 
                            icon: "system-lock-screen",
                            tooltip: i18n("Bloquear pantalla"),
                            fn: function() { sysControl.lock(); } 
                        },

                        { 
                            icon: "system-log-out",
                            tooltip: i18n("Cerrar sesión"),
                            fn: function() { sysControl.logout(); } 
                        },

                        { 
                            icon: "system-reboot",
                            tooltip: i18n("Reiniciar"),
                            fn: function() { sysControl.reboot(); } 
                        },
                        
                        { 
                            icon: "system-shutdown",
                            tooltip: i18n("Apagar"),
                            fn: function() { sysControl.shutdown(); } 
                        }
                    ]
                    delegate: Rectangle {
                        width: 34; height: 34; radius: 17
                        Layout.alignment: Qt.AlignHCenter
                        color: btnMa.containsMouse ? (modelData.icon === "system-shutdown" ? "#E74C3C" : RavenPlugin.RavenTheme.highlightColor) : RavenPlugin.RavenTheme.hoverBackground
                        Behavior on color { ColorAnimation { duration: 120 } }

                        Kirigami.Icon {
                            anchors.centerIn: parent
                            source: modelData.icon
                            implicitWidth: 16; implicitHeight: 16
                            color: btnMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                        }

                        ToolTip.visible: btnMa.containsMouse
                        ToolTip.text: modelData.tooltip
                        ToolTip.delay: 200

                        MouseArea {
                            id: btnMa
                            anchors.fill: parent
                            hoverEnabled: true
                            onClicked: {
                                modelData.fn();
                                root.appClicked("", "");
                            }
                        }
                    }
                }
            }
        }
    }
}
