#!/bin/bash
mkdir -p /tmp/test-kwin-qml/contents/ui
cat << 'QML' > /tmp/test-kwin-qml/contents/ui/main.qml
import QtQuick
import org.kde.kwin
Item {
    Component.onCompleted: {
        print("[TEST-KWIN-QML] workspace type: " + typeof workspace);
        print("[TEST-KWIN-QML] Workspace type: " + typeof Workspace);
        print("[TEST-KWIN-QML] KWin type: " + typeof KWin);
    }
}
QML
cat << 'JSON' > /tmp/test-kwin-qml/metadata.json
{
    "KPlugin": { "Id": "org.kde.test.qml", "Name": "Test QML" },
    "KPackageStructure": "KWin/Script",
    "X-Plasma-API": "declarativescript",
    "X-Plasma-MainScript": "ui/main.qml"
}
JSON
kpackagetool6 --type=KWin/Script -i /tmp/test-kwin-qml 2>/dev/null || kpackagetool6 --type=KWin/Script -u /tmp/test-kwin-qml
dbus-send --print-reply --dest=org.kde.KWin /Scripting org.kde.kwin.Scripting.start
sleep 2
journalctl --user -u plasma-kwin_wayland.service --since "10 seconds ago" | grep "\\[TEST-KWIN-QML\\]"
