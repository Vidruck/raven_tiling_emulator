/**
 * @file configAppearance.qml
 * @brief Módulo de configuración KCM (KConfigModule) para el reloj, fecha y apariencia del plasmoide.
 * @author Alejandro González Hernández (Vidruck)
 * @version 3.4
 * @license GPL-3.0
 */

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM
import org.kde.iconthemes as KIconThemes

/**
 * @class configAppearance
 * @brief Interfaz de configuración gráfica para las preferencias de tiempo, formato y posición del lanzador.
 */
KCM.SimpleKCM {
    id: root

    property alias cfg_timeFormat: timeFormatCombo.currentValue
    property alias cfg_showSeconds: showSecondsCheck.checked
    property alias cfg_launcherPosition: positionCombo.currentValue
    property alias cfg_plasmoidIcon: iconField.text
    property alias cfg_gridColumns: gridColumnsSpin.value

    Kirigami.FormLayout {
        anchors.fill: parent

        Item {
            Kirigami.FormData.isSection: true
            Kirigami.FormData.label: i18n("Reloj y Hora")
        }

        ComboBox {
            id: timeFormatCombo
            Kirigami.FormData.label: i18n("Formato de hora:")
            textRole: "text"
            valueRole: "value"
            model: [
                { text: i18n("24 horas (ej. 14:30)"), value: "24h" },
                { text: i18n("12 horas (ej. 02:30 PM)"), value: "12h" }
            ]
        }

        CheckBox {
            id: showSecondsCheck
            Kirigami.FormData.label: i18n("Segundos:")
            text: i18n("Mostrar segundos en el reloj")
        }

        Item {
            Kirigami.FormData.isSection: true
            Kirigami.FormData.label: i18n("Comportamiento y Posición")
        }

        ComboBox {
            id: positionCombo
            Kirigami.FormData.label: i18n("Posición del lanzador:")
            textRole: "text"
            valueRole: "value"
            model: [
                { text: i18n("Centro de la pantalla"), value: "center" },
                { text: i18n("Acoplado al panel"), value: "panel" }
            ]
        }

        RowLayout {
            Kirigami.FormData.label: i18n("Icono del Plasmoide:")
            TextField {
                id: iconField
                Layout.fillWidth: true
                placeholderText: i18n("Ej. start-here-kde o ruta de imagen")
            }
            Button {
                icon.name: iconField.text.length > 0 ? iconField.text : "document-open"
                text: i18n("Seleccionar...")
                onClicked: iconDialog.open()
            }
        }

        KIconThemes.IconDialog {
            id: iconDialog
            onIconNameChanged: iconField.text = iconName
        }

        SpinBox {
            id: gridColumnsSpin
            Kirigami.FormData.label: i18n("Columnas (Grilla Apps):")
            from: 2
            to: 8
            stepSize: 1
        }
    }
}
