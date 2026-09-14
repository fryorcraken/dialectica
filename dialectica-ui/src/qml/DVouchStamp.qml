import QtQuick

// The vouch stamp. Two states and no third:
//   not vouched — outline stamp, revealed only while the pointer is over the
//                 post (bind `revealed`), so a quiet feed stays quiet;
//   vouched     — inked stamp, ALWAYS visible, because it is the viewer's own
//                 record of a decision and must not need a hover to be found.
//
// A VOUCH IS NEVER PUBLISHED AND NEVER COUNTED, so there is no number here and
// no property that could hold one. That is a design constraint expressed as an
// absence: a count is the thing a reader would most naturally want, and the
// component cannot be asked for one.
//
// Vouching is not a vote direction. It does not enter ranking, and nothing on
// this component reports to anyone but the viewer.
Rectangle {
    id: root

    property bool vouched: false
    property bool revealed: false        // pointer is over the post row
    signal toggled()

    // copy.json `common.vouch` / `common.vouched`, verbatim.
    readonly property string label: vouched ? "VOUCHED" : "VOUCH"

    implicitWidth:  stampText.implicitWidth + 16
    implicitHeight: stampText.implicitHeight + 6

    color:        vouched ? DTheme.accent : "transparent"
    border.width: DTheme.hairline
    border.color: vouched ? DTheme.accent : DTheme.ink

    // A stamp sits slightly askew, the way an inked one would. Only when
    // vouched: an un-vouched outline is a prompt, not a record.
    rotation:     vouched ? -1.5 : 0

    // The whole visibility rule, in one binding. `vouched ||` is the
    // load-bearing half: a vouch the viewer has to hover to rediscover is a
    // record they cannot rely on having made.
    opacity:      vouched || revealed ? 1 : 0
    Behavior on opacity { NumberAnimation { duration: 120 } }

    // The double rule of the press aesthetic: a paper gap, then a second rule.
    // Both sit behind the stamp (z: -1) and neither takes input.
    Rectangle {
        anchors.fill: parent
        anchors.margins: -3
        z: -1
        color: "transparent"
        border.width: DTheme.hairline
        border.color: root.border.color
    }
    Rectangle {
        anchors.fill: parent
        anchors.margins: -1.5
        z: -1
        color: DTheme.paper
    }

    Text {
        id: stampText
        anchors.centerIn: parent
        text: root.label
        font: DTheme.label
        color: root.vouched ? DTheme.paper : DTheme.ink
        textFormat: Text.PlainText
    }

    MouseArea {
        id: stampHover
        anchors.fill: parent

        // A stamp at zero opacity is invisible, and an invisible control that
        // still takes clicks is a click nobody can predict. The enable follows
        // the same binding the opacity does, so the two cannot drift.
        enabled: root.opacity > 0

        cursorShape: Qt.PointingHandCursor
        hoverEnabled: true
        onClicked: root.toggled()

        // `DTip` rather than an attached tooltip binding. Both strings here are
        // hardcoded literals, so this site renders no markup TODAY — which is
        // exactly why it is converted rather than excused. A tooltip binding a
        // literal is one edit from binding a name or a reason, and that edit
        // has no reason to think it touched rendering. The rule is "no attached
        // tooltip binding in this tree", and a rule with a case-by-case
        // exemption is a rule a gate cannot state.
        DTip {
            // copy.json `common.vouchTooltip` / `common.vouchedTooltip`, verbatim.
            text: root.vouched ? "You vouched for this author — click to undo"
                               : "Vouch for this author"
            visible: stampHover.containsMouse
        }
    }
}
