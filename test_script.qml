import QtQuick
import org.kde.kwin
Item {
    Component.onCompleted: {
        print("TEST SCRIPT: workspace is " + typeof workspace);
        print("TEST SCRIPT: Workspace is " + typeof Workspace);
        print("TEST SCRIPT: KWin.workspace is " + typeof KWin.workspace);
    }
}
