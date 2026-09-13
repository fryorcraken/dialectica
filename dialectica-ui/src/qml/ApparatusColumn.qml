import QtQuick
import QtQuick.Layouts

Rectangle {
    id: root
    default property alias content: inner.data

    implicitWidth: DTheme.apparatusWidth
    color: DTheme.paperDeep

    Rectangle {          // the left border of the column
        width: DTheme.hairline
        height: parent.height
        color: DTheme.ink
    }

    ColumnLayout {
        id: inner
        anchors.fill: parent
        anchors.margins: 20
        anchors.topMargin: DTheme.cardPaddingY
        spacing: 18

        Text { text: "APPARATUS"; font: DTheme.label; color: DTheme.inkMuted; textFormat: Text.PlainText }
    }
}
