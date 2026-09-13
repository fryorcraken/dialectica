import QtQuick
import QtQuick.Layouts

// The card: content on paper, apparatus column on the right. Every screen uses
// it, so a caveat always has somewhere to live next to the thing it qualifies.
Rectangle {
    id: root

    default property alias content: body.data

    // The margin column.
    //
    // **Apparatus may REPEAT an obligation, never carry it alone.** Anything a
    // spec requires a screen to state must also live in `content`; a margin
    // note is the second copy, never the only one.
    //
    // The reason is what this column is: annotation explaining the design to a
    // reader, which makes it removable by a change that has no reason to read
    // any screen's spec. A requirement stated only here is deleted as a side
    // effect of dropping decoration, and the change that drops it cannot see
    // that it did.
    //
    // `OnboardingScreen` has the worked example — its uniqueness statement is
    // in the body AND in a MarginNote, because the spec requires the screen to
    // state it. Repetition is permitted, not required: a note that only
    // explains (its "ON THE MARK") is free to be margin-only.
    property alias apparatus: app.content

    implicitWidth: Theme.cardWidth
    color: Theme.paper
    border.width: Theme.hairline
    border.color: Theme.ink

    RowLayout {
        anchors.fill: parent
        spacing: 0

        ColumnLayout {
            id: body
            Layout.fillWidth: true
            Layout.alignment: Qt.AlignTop
            Layout.margins: Theme.cardPaddingY
            Layout.leftMargin: Theme.cardPaddingX
            Layout.rightMargin: Theme.cardPaddingX
            spacing: Theme.blockGap
        }

        ApparatusColumn {
            id: app
            Layout.fillHeight: true
            Layout.preferredWidth: Theme.apparatusWidth
        }
    }
}
