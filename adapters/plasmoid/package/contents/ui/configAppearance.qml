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

    property alias cfg_plasmoidIcon: iconField.text
    property alias cfg_gridRows: gridRowsSpin.value

    Kirigami.FormLayout {
        anchors.fill: parent

        Item {
            Kirigami.FormData.isSection: true
            Kirigami.FormData.label: i18n("Apariencia General")
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
            id: gridRowsSpin
            Kirigami.FormData.label: i18n("Filas Visibles (Grilla Apps):")
            from: 3
            to: 30
            stepSize: 1
        }
    }
}
