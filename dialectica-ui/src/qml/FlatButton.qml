import QtQuick

// No radius, no gradient, no shadow. Three kinds only.
// destructive is reserved for publishing something irreversible.
Rectangle {
    id: root

    property string text: ""
    property string kind: "secondary"   // "primary" | "secondary" | "destructive"
    signal clicked()

    readonly property bool filled: kind !== "secondary"

    color: kind === "primary" ? Theme.ink
         : kind === "destructive" ? Theme.accent
         : "transparent"
    border.width: filled ? 0 : Theme.hairline
    border.color: Theme.ink
    implicitWidth: label.implicitWidth + (filled ? 36 : 32)
    implicitHeight: label.implicitHeight + (filled ? 16 : 14)

    Text {
        id: label
        anchors.centerIn: parent
        text: root.text
        font: Theme.body
        color: root.filled ? Theme.paper : Theme.ink
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
