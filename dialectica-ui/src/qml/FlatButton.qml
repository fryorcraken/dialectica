import QtQuick

// No radius, no gradient, no shadow. Three kinds only.
// destructive is reserved for publishing something irreversible.
Rectangle {
    id: root

    property string text: ""
    property string kind: "secondary"   // "primary" | "secondary" | "destructive"
    signal clicked()

    readonly property bool filled: kind !== "secondary"

    color: kind === "primary" ? DTheme.ink
         : kind === "destructive" ? DTheme.accent
         : "transparent"
    border.width: filled ? 0 : DTheme.hairline
    border.color: DTheme.ink
    implicitWidth: label.implicitWidth + (filled ? 36 : 32)
    implicitHeight: label.implicitHeight + (filled ? 16 : 14)

    Text {
        id: label
        anchors.centerIn: parent
        text: root.text
        font: DTheme.body
        color: root.filled ? DTheme.paper : DTheme.ink
        textFormat: Text.PlainText
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
