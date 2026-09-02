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
    implicitWidth: 510
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
                ctx.clearRect(0, 0, width, height);
                
                var x = width / 2;
                var y = height / 2;
                var radius = width / 2 - 3.5;
                var startAngle = Math.PI * 0.75;
                var endAngle = Math.PI * 2.25;
                
                // Track de fondo
                ctx.beginPath();
                ctx.arc(x, y, radius, startAngle, endAngle);
                ctx.lineWidth = 3.5;
                ctx.lineCap = "round";
                ctx.strokeStyle = RavenPlugin.RavenTheme.isDark ? Qt.rgba(1, 1, 1, 0.12) : Qt.rgba(0, 0, 0, 0.1);
                ctx.stroke();
                
                // Track de progreso
                var boundedVal = Math.min(100, Math.max(0, gaugeRoot.value));
                var valAngle = startAngle + (endAngle - startAngle) * (boundedVal / 100.0);
                ctx.beginPath();
                ctx.arc(x, y, radius, startAngle, valAngle);
                ctx.lineWidth = 3.5;
                ctx.lineCap = "round";
                ctx.strokeStyle = gaugeRoot.colorOverride !== "" ? gaugeRoot.colorOverride : RavenPlugin.RavenTheme.highlightColor;
                ctx.stroke();
            }
            Connections {
                target: gaugeRoot
                function onValueChanged() { canvas.requestPaint(); }
                function onColorOverrideChanged() { canvas.requestPaint(); }
            }
        }
        
        Column {
            anchors.centerIn: parent
            spacing: -2
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

                // ── ISLA 1: RAVEN COMMAND & CONTROL (SUB-ISLAS MODULARES) ──
                Island {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 205

                    ColumnLayout {
                        anchors.fill: parent
                        anchors.margins: 10
                        spacing: 10

                        // ── CABECERA HERO: RELOJ, FECHA, MARCA Y CONTROLES MAESTROS ──
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 12

                            Kirigami.Icon {
                                source: "window-duplicate"
                                implicitWidth: 32; implicitHeight: 32
                                color: RavenPlugin.RavenController.tilingEnabled ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.subTextColor
                                Layout.alignment: Qt.AlignVCenter
                            }

                            ColumnLayout {
                                spacing: 2
                                Layout.fillWidth: true

                                RowLayout {
                                    spacing: 8
                                    Text {
                                        text: Qt.formatTime(root.currentDate, "hh:mm")
                                        color: RavenPlugin.RavenTheme.highlightColor
                                        font.pixelSize: 18
                                        font.bold: true
                                        font.family: RavenPlugin.RavenTheme.fixedFontFamily || "Monospace"
                                    }
                                    Rectangle {
                                        width: 4; height: 4; radius: 2
                                        color: RavenPlugin.RavenTheme.subTextColor
                                        opacity: 0.5
                                        Layout.alignment: Qt.AlignVCenter
                                    }
                                    Text {
                                        text: i18n("RAVEN TILING")
                                        color: RavenPlugin.RavenTheme.textColor
                                        font.pixelSize: 11
                                        font.bold: true
                                        font.family: RavenPlugin.RavenTheme.fontFamily || "Noto Sans"
                                        font.letterSpacing: 0.8
                                    }
                                }

                                Text {
                                    text: Qt.formatDate(root.currentDate, Qt.DefaultLocaleLongDate) + "  •  " + (RavenPlugin.RavenController.tilingEnabled ? i18n("Modo Mosaico") : i18n("Modo Flotante"))
                                    color: RavenPlugin.RavenController.tilingEnabled ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.subTextColor
                                    font.pixelSize: 10
                                    font.capitalization: Font.Capitalize
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                            }

                            Item { Layout.fillWidth: true }

                            // Botón Swap Ant
                            Rectangle {
                                width: 28; height: 24; radius: 6
                                color: swapPrevMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                Kirigami.Icon {
                                    anchors.centerIn: parent
                                    source: "go-previous"; implicitWidth: 12; implicitHeight: 12
                                    color: swapPrevMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                }
                                MouseArea {
                                    id: swapPrevMa; anchors.fill: parent; hoverEnabled: true
                                    onClicked: RavenPlugin.RavenController.swapPrev()
                                }
                                ToolTip.visible: swapPrevMa.containsMouse
                                ToolTip.text: i18n("Intercambiar posición hacia atrás (Meta+Shift+K)")
                            }

                            // Botón Swap Sig
                            Rectangle {
                                width: 28; height: 24; radius: 6
                                color: swapNextMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                Kirigami.Icon {
                                    anchors.centerIn: parent
                                    source: "go-next"; implicitWidth: 12; implicitHeight: 12
                                    color: swapNextMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                }
                                MouseArea {
                                    id: swapNextMa; anchors.fill: parent; hoverEnabled: true
                                    onClicked: RavenPlugin.RavenController.swapNext()
                                }
                                ToolTip.visible: swapNextMa.containsMouse
                                ToolTip.text: i18n("Intercambiar posición adelante (Meta+Shift+J)")
                            }

                            // Botón Centro de Control GUI
                            Rectangle {
                                width: 24; height: 24; radius: 6
                                color: ccMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                Kirigami.Icon {
                                    anchors.centerIn: parent
                                    source: "configure"; implicitWidth: 13; implicitHeight: 13
                                    color: ccMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                }
                                MouseArea {
                                    id: ccMa; anchors.fill: parent; hoverEnabled: true
                                    onClicked: {
                                        RavenPlugin.RavenController.openControlCenter();
                                        root.appClicked("", "");
                                    }
                                }
                                ToolTip.visible: ccMa.containsMouse
                                ToolTip.text: i18n("Abrir Centro de Control Raven (GUI)")
                            }

                            // Switch Maestro On/Off
                            Rectangle {
                                width: 44; height: 22; radius: 11
                                color: RavenPlugin.RavenController.tilingEnabled ? RavenPlugin.RavenTheme.highlightColor : Qt.rgba(1, 1, 1, 0.15)
                                Behavior on color { ColorAnimation { duration: 150 } }

                                Rectangle {
                                    width: 16; height: 16; radius: 8
                                    anchors.verticalCenter: parent.verticalCenter
                                    x: RavenPlugin.RavenController.tilingEnabled ? parent.width - width - 3 : 3
                                    color: "#FFFFFF"
                                    Behavior on x { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }
                                }

                                MouseArea {
                                    anchors.fill: parent
                                    cursorShape: Qt.PointingHandCursor
                                    onClicked: RavenPlugin.RavenController.toggleTiling()
                                }
                            }
                        }

                        // Fila de 2 Sub-Islas: [ SUB-ISLA PANTALLA ] y [ SUB-ISLA ESCRITORIOS (CARRUSEL) ]
                        RowLayout {
                            Layout.fillWidth: true
                            spacing: 8

                            // ── SUB-ISLA 1: PANTALLA (Iconos compactos) ──
                            Rectangle {
                                Layout.preferredWidth: 130
                                Layout.preferredHeight: 52
                                radius: 8
                                color: RavenPlugin.RavenTheme.surfaceElevated || Qt.rgba(1, 1, 1, 0.05)
                                border.width: 1
                                border.color: RavenPlugin.RavenTheme.cardBorder

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 4
                                    spacing: 2

                                    Text {
                                        text: i18n("Pantalla (%1)", RavenPlugin.RavenController.monitorCount)
                                        color: RavenPlugin.RavenTheme.subTextColor
                                        font.pixelSize: 8
                                        font.bold: true
                                        Layout.alignment: Qt.AlignHCenter
                                    }

                                    RowLayout {
                                        Layout.alignment: Qt.AlignHCenter
                                        spacing: 6

                                        Rectangle {
                                            width: 44; height: 24; radius: 5
                                            color: monPrevMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            Kirigami.Icon {
                                                anchors.centerIn: parent
                                                source: "go-previous"
                                                implicitWidth: 12; implicitHeight: 12
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
                                            width: 44; height: 24; radius: 5
                                            color: monNextMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            Kirigami.Icon {
                                                anchors.centerIn: parent
                                                source: "go-next"
                                                implicitWidth: 12; implicitHeight: 12
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

                            // ── SUB-ISLA 2: ESCRITORIOS (CARRUSEL DINÁMICO) ──
                            Rectangle {
                                Layout.fillWidth: true
                                Layout.preferredHeight: 52
                                radius: 8
                                color: RavenPlugin.RavenTheme.surfaceElevated || Qt.rgba(1, 1, 1, 0.05)
                                border.width: 1
                                border.color: RavenPlugin.RavenTheme.cardBorder

                                ColumnLayout {
                                    anchors.fill: parent
                                    anchors.margins: 4
                                    spacing: 2

                                    Text {
                                        text: i18n("Escritorios Virtuales")
                                        color: RavenPlugin.RavenTheme.subTextColor
                                        font.pixelSize: 8
                                        font.bold: true
                                        Layout.alignment: Qt.AlignHCenter
                                    }

                                    RowLayout {
                                        Layout.alignment: Qt.AlignHCenter
                                        spacing: 6

                                        // Botón Hacia Escritorio Anterior con número
                                        Rectangle {
                                            width: 38; height: 24; radius: 5
                                            color: dskPrevMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            RowLayout {
                                                anchors.centerIn: parent; spacing: 2
                                                Kirigami.Icon {
                                                    source: "go-previous"; implicitWidth: 10; implicitHeight: 10
                                                    color: dskPrevMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                }
                                                Text {
                                                    text: RavenPlugin.RavenController.prevDesktop
                                                    color: dskPrevMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                    font.pixelSize: 9; font.bold: true
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
                                            width: 58; height: 24; radius: 5
                                            color: Qt.rgba(RavenPlugin.RavenTheme.highlightColor.r, RavenPlugin.RavenTheme.highlightColor.g, RavenPlugin.RavenTheme.highlightColor.b, 0.20)
                                            border.width: 1
                                            border.color: RavenPlugin.RavenTheme.highlightColor

                                            Text {
                                                anchors.centerIn: parent
                                                text: i18n("Desk %1", RavenPlugin.RavenController.currentDesktop)
                                                color: RavenPlugin.RavenTheme.highlightColor
                                                font.pixelSize: 9
                                                font.bold: true
                                            }
                                        }

                                        // Botón Hacia Escritorio Siguiente con número
                                        Rectangle {
                                            width: 38; height: 24; radius: 5
                                            color: dskNextMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                            RowLayout {
                                                anchors.centerIn: parent; spacing: 2
                                                Text {
                                                    text: RavenPlugin.RavenController.nextDesktop
                                                    color: dskNextMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                                    font.pixelSize: 9; font.bold: true
                                                }
                                                Kirigami.Icon {
                                                    source: "go-next"; implicitWidth: 10; implicitHeight: 10
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

                        // ── SUB-ISLA 3: MÁRGENES (GAPS) + FLOTANTE (QUICK PEEK) ──
                        Rectangle {
                            Layout.fillWidth: true
                            Layout.preferredHeight: 46
                            radius: 8
                            color: RavenPlugin.RavenTheme.surfaceElevated || Qt.rgba(1, 1, 1, 0.05)
                            border.width: 1
                            border.color: RavenPlugin.RavenTheme.cardBorder

                            RowLayout {
                                anchors.fill: parent
                                anchors.margins: 6
                                spacing: 8

                                // Botón Reducir Márgenes (-2)
                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    radius: 6
                                    color: gapsDecMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                    RowLayout {
                                        anchors.centerIn: parent; spacing: 4
                                        Kirigami.Icon {
                                            source: "zoom-out"; implicitWidth: 13; implicitHeight: 13
                                            color: gapsDecMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                        }
                                        Text {
                                            text: i18n("Márgenes -2")
                                            color: gapsDecMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                            font.pixelSize: 9; font.bold: true
                                        }
                                    }
                                    MouseArea {
                                        id: gapsDecMa; anchors.fill: parent; hoverEnabled: true
                                        onClicked: RavenPlugin.RavenController.incrementGaps(-2)
                                    }
                                    ToolTip.visible: gapsDecMa.containsMouse
                                    ToolTip.text: i18n("Reducir separación entre ventanas (Meta+-)")
                                }

                                // Botón Aumentar Márgenes (+2)
                                Rectangle {
                                    Layout.fillWidth: true
                                    Layout.fillHeight: true
                                    radius: 6
                                    color: gapsIncMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                    RowLayout {
                                        anchors.centerIn: parent; spacing: 4
                                        Kirigami.Icon {
                                            source: "zoom-in"; implicitWidth: 13; implicitHeight: 13
                                            color: gapsIncMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                        }
                                        Text {
                                            text: i18n("Márgenes +2")
                                            color: gapsIncMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                            font.pixelSize: 9; font.bold: true
                                        }
                                    }
                                    MouseArea {
                                        id: gapsIncMa; anchors.fill: parent; hoverEnabled: true
                                        onClicked: RavenPlugin.RavenController.incrementGaps(2)
                                    }
                                    ToolTip.visible: gapsIncMa.containsMouse
                                    ToolTip.text: i18n("Aumentar separación entre ventanas (Meta+=)")
                                }

                                // Botón Unificado Flotar (Quick Peek)
                                Rectangle {
                                    Layout.preferredWidth: 80
                                    Layout.fillHeight: true
                                    radius: 6
                                    color: floatMa.containsMouse ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.hoverBackground
                                    RowLayout {
                                        anchors.centerIn: parent; spacing: 4
                                        Kirigami.Icon {
                                            source: "view-restore"; implicitWidth: 13; implicitHeight: 13
                                            color: floatMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                        }
                                        Text {
                                            text: i18n("Flotar")
                                            color: floatMa.containsMouse ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
                                            font.pixelSize: 9; font.bold: true
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
                        onAppLaunched: (u, c) => root.appClicked(u, c)
                        onEscapeRequested: root.appClicked("", "")
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

                            // Badge de Versión
                            Rectangle {
                                radius: 4
                                color: Qt.rgba(RavenPlugin.RavenTheme.highlightColor.r, RavenPlugin.RavenTheme.highlightColor.g, RavenPlugin.RavenTheme.highlightColor.b, 0.15)
                                border.width: 1
                                border.color: Qt.rgba(RavenPlugin.RavenTheme.highlightColor.r, RavenPlugin.RavenTheme.highlightColor.g, RavenPlugin.RavenTheme.highlightColor.b, 0.35)
                                implicitWidth: vText.implicitWidth + 8
                                implicitHeight: 18

                                Text {
                                    id: vText
                                    anchors.centerIn: parent
                                    text: "Raven Hub • v3.4"
                                    color: RavenPlugin.RavenTheme.highlightColor
                                    font.pixelSize: 8
                                    font.bold: true
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

                // Icono decorativo superior
                Kirigami.Icon {
                    source: "view-grid"
                    implicitWidth: 16; implicitHeight: 16
                    Layout.alignment: Qt.AlignHCenter
                    color: RavenPlugin.RavenTheme.highlightColor
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
                            icon: "view-split-left-right", 
                            tooltip: i18n("Raven BSP (Espiral Áurea)") 
                        },

                        { 
                            name: i18n("Tall / Master"),          
                            id: "tall",                   
                            icon: "view-split-top-bottom", 
                            tooltip: i18n("Tall (Ventana Maestra + Pila)") 
                        },

                        { 
                            name: i18n("Monocle"),                
                            id: "monocle",                
                            icon: "view-fullscreen",       
                            tooltip: i18n("Monocle (Ventana Completa)") 
                        },

                        { 
                            name: i18n("Strict Dwindle"),         
                            id: "strict_dwindle",         
                            icon: "view-grid",             
                            tooltip: i18n("Strict Dwindle (Fibonacci Puro)") 
                        },

                        {   
                            name: i18n("Inverted Strict Dwindle"), 
                            id: "inverted_strict_dwindle", 
                            icon: "view-grid-symbolic",    
                            tooltip: i18n("Inverted Strict Dwindle (Espiral Inversa)") 
                        },

                        { 
                            name: i18n("Divisor"),                
                            id: "divisor",                
                            icon: "view-paged",            
                            tooltip: i18n("Divisor (Subdivisiones Equitativas)") 
                        }
                    ]
                    delegate: Rectangle {
                        width: 36; height: 36; radius: 8
                        Layout.alignment: Qt.AlignHCenter
                        property bool isActive: RavenPlugin.RavenController.currentLayout === modelData.id
                        color: {
                            if (layoutMa.containsMouse) return RavenPlugin.RavenTheme.highlightColor;
                            if (isActive) return Qt.rgba(RavenPlugin.RavenTheme.highlightColor.r, RavenPlugin.RavenTheme.highlightColor.g, RavenPlugin.RavenTheme.highlightColor.b, 0.25);
                            return RavenPlugin.RavenTheme.hoverBackground;
                        }
                        border.width: isActive ? 1 : 0
                        border.color: RavenPlugin.RavenTheme.highlightColor
                        Behavior on color { ColorAnimation { duration: 120 } }

                        Kirigami.Icon {
                            anchors.centerIn: parent
                            source: modelData.icon
                            implicitWidth: 18; implicitHeight: 18
                            color: (layoutMa.containsMouse || isActive) ? "#FFFFFF" : RavenPlugin.RavenTheme.textColor
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
