import QtQuick
import QtQuick.Layouts

// One entry in the apparatus column. Red rule when the note qualifies a claim
// the interface is making; grey when it is context.
RowLayout {
    id: root

    property string label: ""
    property string body: ""
    property bool   caveat: true
    property string linkText: ""
    signal linkActivated()

    spacing: 10

    Rectangle {
        // `Layout.preferredWidth`, not `width`: this Item is managed by the
        // enclosing RowLayout, which overwrites `width` — qmllint calls it
        // undefined behaviour and CI gates on the exit code. The 2px rule is
        // the design's, unchanged.
        Layout.preferredWidth: 2
        Layout.fillHeight: true
        color: root.caveat ? DTheme.accent : DTheme.rule2
    }

    ColumnLayout {
        spacing: 5
        Layout.fillWidth: true

        Text {
            text: root.label
            font: DTheme.label
            color: root.caveat ? DTheme.accent : DTheme.inkMuted
            textFormat: Text.PlainText
        }
        Text {
            text: root.body
            font: DTheme.note
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.4
            // `body` is a bound property, so a caller may one day put a
            // peer-supplied string here. AutoText would sniff it.
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
        Text {
            visible: root.linkText !== ""
            text: root.linkText
            font: DTheme.bodySmall
            color: DTheme.accent
            textFormat: Text.PlainText
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.linkActivated()
            }
        }
    }
}
