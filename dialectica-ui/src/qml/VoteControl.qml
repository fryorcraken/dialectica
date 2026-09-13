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

    // Whether the number is rendered at all. **Default false, and the default
    // is the decision.**
    //
    // No call in the current contract returns a score: a publish reply carries
    // `{opId, wasNew}` and a feed row carries no tally. So `score` has nothing
    // to bind to, and a control left to its default would print `Math.max(0, 0)`
    // — the numeral "0" — beside every post. That is worse than printing
    // nothing: zero is a number, so it reads as a tally, and it reads as the
    // tally ZERO, a claim that this post is known to have received no votes.
    // Core has never said that, and it is false the moment any peer votes. An
    // absent number says "not known here".
    //
    // Defaulting to false rather than true is what makes the honest rendering
    // the one you get by not thinking about it: a call site that forgets this
    // property shows no number, instead of a fabricated one. Showing a count
    // becomes something a later change has to decide to do — which is the
    // point, since that change is the one that must first find a count to show.
    property bool showScore: false

    signal voted(int direction)

    spacing: 3
    width: 40

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "\u25B2"
        font.pixelSize: 15
        color: root.vote > 0 ? Theme.ink : Theme.rule2
        textFormat: Text.PlainText
        MouseArea {
            anchors.fill: parent
            enabled: root.interactive
            cursorShape: Qt.PointingHandCursor
            onClicked: root.voted(root.vote > 0 ? 0 : 1)
        }
    }

    Text {
        // `visible` AND `height: 0` when suppressed: an invisible Item in a
        // Column still occupies its row, which would leave a gap between the
        // arrows that reads as a number that failed to load.
        visible: root.showScore
        height: root.showScore ? implicitHeight : 0
        anchors.horizontalCenter: parent.horizontalCenter
        text: Math.max(0, root.score)        // the floor, applied in one place
        font: Theme.address
        color: root.score > 0 ? Theme.ink : Theme.inkMuted
        textFormat: Text.PlainText
    }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        text: "\u25BC"
        font.pixelSize: 15
        color: root.vote < 0 ? Theme.ink : Theme.rule2
        textFormat: Text.PlainText
        MouseArea {
            anchors.fill: parent
            enabled: root.interactive
            cursorShape: Qt.PointingHandCursor
            onClicked: root.voted(root.vote < 0 ? 0 : -1)
        }
    }
}
