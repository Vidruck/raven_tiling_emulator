/**
 * @file MediaWidgetView.qml
 * @brief Widget de control multimedia MPRIS con carátulas de álbum y controles interactivos.
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
 * @class MediaWidgetView
 * @brief Componente visual para la reproducción de audio, visualización de metadatos y control de volumen/pista.
 */
Rectangle {
    id: rootMedia
    radius: RavenPlugin.RavenTheme.radiusLg
    clip: true
    color: RavenPlugin.RavenTheme.cardBackground
    border.width: 1
    border.color: RavenPlugin.RavenTheme.cardBorder

    property alias active: media.active

    RavenPlugin.MediaController {
        id: media
        active: true
    }

    // Blurred Background Album Art Effect (Con bordes suaves y recorte redondeado)
    Item {
        anchors.fill: parent
        clip: true
        layer.enabled: true
        layer.effect: Kirigami.ShadowedRectangle {
            radius: RavenPlugin.RavenTheme.radiusLg
            color: "transparent"
        }

        Image {
            anchors.fill: parent
            source: media.artUrl
            fillMode: Image.PreserveAspectCrop
            opacity: media.hasPlayer ? (RavenPlugin.RavenTheme.isDark ? 0.22 : 0.14) : 0
            visible: media.artUrl !== ""
            asynchronous: true
            Behavior on opacity { NumberAnimation { duration: 300 } }
        }
    }
    
    // Gradient overlay para contraste, profundidad y cuerpo sólido
    Rectangle {
        anchors.fill: parent
        radius: RavenPlugin.RavenTheme.radiusLg
        gradient: Gradient {
            GradientStop {
                position: 0.0
                color: RavenPlugin.RavenTheme.isDark
                    ? Qt.rgba(RavenPlugin.RavenTheme.cardBackground.r, RavenPlugin.RavenTheme.cardBackground.g, RavenPlugin.RavenTheme.cardBackground.b, 0.50)
                    : Qt.rgba(RavenPlugin.RavenTheme.cardBackground.r, RavenPlugin.RavenTheme.cardBackground.g, RavenPlugin.RavenTheme.cardBackground.b, 0.50)
            }
            GradientStop {
                position: 1.0
                color: RavenPlugin.RavenTheme.isDark
                    ? Qt.rgba(0, 0, 0, 0.40)
                    : Qt.rgba(0, 0, 0, 0.05)
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 10
        spacing: 6

        // ── FILA 1: PORTADA REDONDEADA + INFO DE PISTA + CONTROLES JUNTOS ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            // Carátula / Portada del Álbum con esquinas redondeadas y clip estricto
            Rectangle {
                width: 46
                height: 46
                radius: 10
                color: RavenPlugin.RavenTheme.isDark ? "#12141C" : "#E2E8F0"
                border.width: 1
                border.color: media.isPlaying ? RavenPlugin.RavenTheme.highlightColor : RavenPlugin.RavenTheme.cardBorder
                clip: true

                Image {
                    id: albumCoverImg
                    anchors.fill: parent
                    source: media.artUrl
                    fillMode: Image.PreserveAspectCrop
                    visible: media.artUrl !== "" && status === Image.Ready
                    asynchronous: true
                }

                Kirigami.Icon {
                    anchors.centerIn: parent
                    source: media.isPlaying ? "media-playback-start" : "audio-x-generic"
                    implicitWidth: 22; implicitHeight: 22
                    visible: !albumCoverImg.visible
                    opacity: 0.6
                }
            }

            // Info de Pista y Artista
            ColumnLayout {
                Layout.fillWidth: true
                spacing: 1

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 6

                    Text {
                        text: media.hasPlayer && media.trackTitle.length > 0
                            ? media.trackTitle
                            : i18n("Sin reproducción activa")
                        color: RavenPlugin.RavenTheme.textColor
                        font.pixelSize: 12
                        font.bold: true
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    Rectangle {
                        visible: media.hasPlayer && media.playerName.length > 0
                        radius: 4
                        height: 16
                        width: playerBadgeText.implicitWidth + 8
                        color: Qt.rgba(RavenPlugin.RavenTheme.highlightColor.r, RavenPlugin.RavenTheme.highlightColor.g, RavenPlugin.RavenTheme.highlightColor.b, 0.2)

                        Text {
                            id: playerBadgeText
                            anchors.centerIn: parent
                            text: media.playerName
                            color: RavenPlugin.RavenTheme.highlightColor
                            font.pixelSize: 9
                            font.bold: true
                        }
                    }
                }

                Text {
                    text: media.hasPlayer && media.artist.length > 0
                        ? (media.album.length > 0 ? media.artist + " — " + media.album : media.artist)
                        : i18n("Raven Media Player")
                    color: RavenPlugin.RavenTheme.subTextColor
                    font.pixelSize: 10
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }

            // Controles de Reproducción compactos integrados junto a la info de la pista
            RowLayout {
                spacing: 4
                Layout.alignment: Qt.AlignVCenter

                // Anterior
                Rectangle {
                    width: 26; height: 26; radius: 13
                    color: prevMa.containsMouse ? RavenPlugin.RavenTheme.hoverBackground : "transparent"
                    Kirigami.Icon {
                        anchors.centerIn: parent
                        source: "media-skip-backward"
                        implicitWidth: 14; implicitHeight: 14
                        color: RavenPlugin.RavenTheme.textColor
                    }
                    MouseArea {
                        id: prevMa
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: media.previous()
                    }
                    ToolTip.visible: prevMa.containsMouse
                    ToolTip.text: i18n("Pista anterior")
                }

                // Play / Pausa
                Rectangle {
                    width: 30; height: 30; radius: 15
                    color: RavenPlugin.RavenTheme.highlightColor
                    Kirigami.Icon {
                        anchors.centerIn: parent
                        source: media.isPlaying ? "media-playback-pause" : "media-playback-start"
                        implicitWidth: 15; implicitHeight: 15
                        color: "#FFFFFF"
                    }
                    MouseArea {
                        id: playMa
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: media.playPause()
                    }
                    ToolTip.visible: playMa.containsMouse
                    ToolTip.text: media.isPlaying ? i18n("Pausar") : i18n("Reproducir")
                }

                // Siguiente
                Rectangle {
                    width: 26; height: 26; radius: 13
                    color: nextMa.containsMouse ? RavenPlugin.RavenTheme.hoverBackground : "transparent"
                    Kirigami.Icon {
                        anchors.centerIn: parent
                        source: "media-skip-forward"
                        implicitWidth: 14; implicitHeight: 14
                        color: RavenPlugin.RavenTheme.textColor
                    }
                    MouseArea {
                        id: nextMa
                        anchors.fill: parent
                        hoverEnabled: true
                        onClicked: media.next()
                    }
                    ToolTip.visible: nextMa.containsMouse
                    ToolTip.text: i18n("Siguiente pista")
                }
            }
        }

        // ── FILA 2: BARRA DE PROGRESO Y TIEMPOS ──
        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Text {
                text: media.formatTime(media.position)
                color: RavenPlugin.RavenTheme.subTextColor
                font.pixelSize: 9
            }

            Item {
                Layout.fillWidth: true
                height: 4

                Rectangle {
                    anchors.fill: parent
                    radius: 2
                    color: RavenPlugin.RavenTheme.isDark ? Qt.rgba(1, 1, 1, 0.12) : Qt.rgba(0, 0, 0, 0.1)
                }

                Rectangle {
                    anchors.left: parent.left
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    width: media.length > 0 ? Math.min(parent.width, Math.max(0, parent.width * (media.position / media.length))) : 0
                    radius: 2
                    color: RavenPlugin.RavenTheme.highlightColor
                    Behavior on width { NumberAnimation { duration: 250 } }
                }

                MouseArea {
                    anchors.fill: parent
                    anchors.margins: -4
                    enabled: media.hasPlayer && media.length > 0
                    onClicked: (mouse) => {
                        var newPos = Math.round((mouse.x / width) * media.length);
                        media.setPosition(newPos);
                    }
                }
            }

            Text {
                text: media.formatTime(media.length)
                color: RavenPlugin.RavenTheme.subTextColor
                font.pixelSize: 9
            }
        }

        // ── FILA 3: ECUALIZADOR DINÁMICO RÍTMICO ARMÓNICO ──
        Item {
            id: eqContainer
            Layout.fillWidth: true
            Layout.preferredHeight: 34
            clip: true

            property real wavePhase: 0.0

            Timer {
                interval: 32 // ~30 fps fluidos
                running: media.isPlaying
                repeat: true
                onTriggered: {
                    eqContainer.wavePhase = (eqContainer.wavePhase + 0.16) % 628.3;
                }
            }

            RowLayout {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: parent.height
                spacing: 2

                Repeater {
                    model: 31 // 31 bandas de frecuencia (Estándar ISO 1/3 de octava)

                    delegate: Item {
                        id: barSlot
                        Layout.fillWidth: true
                        Layout.fillHeight: true

                        // Curva espectral de energía acústica: graves profundos con 'kick', medios dinámicos y caída de agudos
                        property real bandEnergy: {
                            if (!media.isPlaying) return 0.05;
                            var p = eqContainer.wavePhase;
                            var bandIdx = index;
                            
                            // Golpe de percusión / bajo (Bass Kick) en bandas 0..7
                            var bassBeat = (Math.sin(p * 2.2) + Math.cos(p * 4.4 + bandIdx * 0.2)) * 0.45;
                            // Envolvente de medios (Voces / Melodía) en bandas 8..20
                            var midWave = Math.sin(p * 3.1 + bandIdx * 0.45) * 0.35;
                            // Brillo y armónicos de agudos en bandas 21..30
                            var trebleShimmer = Math.cos(p * 5.0 - bandIdx * 0.6) * 0.25;

                            var weight = (bandIdx < 8) ? 0.85 : (bandIdx < 20 ? 0.70 : 0.45);
                            var composite = Math.abs(bassBeat + midWave + trebleShimmer) * weight;
                            return Math.max(0.12, Math.min(1.0, composite + 0.15));
                        }

                        Rectangle {
                            id: eqBar
                            anchors.left: parent.left
                            anchors.right: parent.right
                            anchors.bottom: parent.bottom // Anclado a la base inferior
                            radius: 1.5

                            // Altura dinámica coordinada por fase armónica
                            height: media.isPlaying
                                ? Math.max(3, Math.min(barSlot.height, barSlot.height * barSlot.bandEnergy))
                                : 2
                            Behavior on height { NumberAnimation { duration: 60; easing.type: Easing.OutQuad } }

                            gradient: Gradient {
                                GradientStop {
                                    position: 0.0
                                    color: RavenPlugin.RavenTheme.highlightColor
                                }
                                GradientStop {
                                    position: 1.0
                                    color: Qt.rgba(RavenPlugin.RavenTheme.highlightColor.r, RavenPlugin.RavenTheme.highlightColor.g, RavenPlugin.RavenTheme.highlightColor.b, 0.25)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
