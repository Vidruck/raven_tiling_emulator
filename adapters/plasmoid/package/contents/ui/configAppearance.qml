import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM

KCM.SimpleKCM {
    id: root

    property alias cfg_timeFormat: timeFormatCombo.currentValue
    property alias cfg_showSeconds: showSecondsCheck.checked
    property alias cfg_launcherPosition: positionCombo.currentValue

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
    }
}
