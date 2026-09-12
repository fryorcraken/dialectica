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
        color: root.caveat ? Theme.accent : Theme.rule2
    }

    ColumnLayout {
        spacing: 5
        Layout.fillWidth: true

        Text {
            text: root.label
            font: Theme.label
            color: root.caveat ? Theme.accent : Theme.inkMuted
        }
        Text {
            text: root.body
            font: Theme.note
            color: Theme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.4
            Layout.fillWidth: true
        }
        Text {
            visible: root.linkText !== ""
            text: root.linkText
            font: Theme.bodySmall
            color: Theme.accent
            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: root.linkActivated()
            }
        }
    }
}
