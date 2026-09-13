import QtQuick

// One axis. The score is floored at zero: nothing ever renders negative, and a
// downvote must never read as removal. Bind score to whatever the ranking
// layer produces — do not compute ups minus downs here, so the meaning of a
// vote (e.g. bridging) can change without touching this control.
Column {
    id: root

    property int  score: 0
    property int  vote: 0          // -1, 0, +1 — the viewer's own
    property bool interactive: true
    signal voted(int direction)

    spacing: 3
    width: 40

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "\u25B2"
        font.pixelSize: 15
        color: root.vote > 0 ? DTheme.ink : DTheme.rule2
        textFormat: Text.PlainText
        MouseArea {
            anchors.fill: parent
            enabled: root.interactive
            cursorShape: Qt.PointingHandCursor
            onClicked: root.voted(root.vote > 0 ? 0 : 1)
        }
    }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: Math.max(0, root.score)        // the floor, applied in one place
        font: DTheme.address
        color: root.score > 0 ? DTheme.ink : DTheme.inkMuted
        textFormat: Text.PlainText
    }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "\u25BC"
        font.pixelSize: 15
        color: root.vote < 0 ? DTheme.ink : DTheme.rule2
        textFormat: Text.PlainText
        MouseArea {
            anchors.fill: parent
            enabled: root.interactive
            cursorShape: Qt.PointingHandCursor
            onClicked: root.voted(root.vote < 0 ? 0 : -1)
        }
    }
}
