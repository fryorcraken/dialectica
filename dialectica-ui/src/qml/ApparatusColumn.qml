import QtQuick
import QtQuick.Layouts

Rectangle {
    id: root
    default property alias content: inner.data

    implicitWidth: Theme.apparatusWidth
    color: Theme.paperDeep

    Rectangle {          // the left border of the column
        width: Theme.hairline
        height: parent.height
        color: Theme.ink
    }

    ColumnLayout {
        id: inner
        anchors.fill: parent
        anchors.margins: 20
        anchors.topMargin: Theme.cardPaddingY
        spacing: 18

        Text { text: "APPARATUS"; font: Theme.label; color: Theme.inkMuted }
    }
}
