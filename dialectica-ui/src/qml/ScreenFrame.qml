import QtQuick
import QtQuick.Layouts

// The card: content on paper. Every screen uses it, so padding, width and the
// hairline border are decided once.
//
// It used to be a two-column RowLayout whose right column was an `APPARATUS`
// panel of marginal notes. Those notes were annotation explaining the design to
// a reader of the mockup, not interface — so they are gone, and the two-column
// layout with them rather than left as an empty column holding 244px of dead
// width. The obligations the notes carried are stated in `docs/UI-BRIEF.md`,
// which is where the interface's obligations belong.
Rectangle {
    id: root

    default property alias content: body.data

    implicitWidth: Theme.cardWidth
    color: Theme.paper
    border.width: Theme.hairline
    border.color: Theme.ink

    // Anchored to three edges, NOT `anchors.fill`. The content packs to the top
    // and the card's height follows it — which is what the old RowLayout bought
    // with `Layout.alignment: Qt.AlignTop` on this same column. Filling the
    // height instead would hand every screen's spare vertical space to whichever
    // child a ColumnLayout decided to stretch.
    ColumnLayout {
        id: body
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.topMargin: Theme.cardPaddingY
        anchors.leftMargin: Theme.cardPaddingX
        anchors.rightMargin: Theme.cardPaddingX
        spacing: Theme.blockGap
    }

    // The card is as tall as its content plus the padding above and below it.
    // Previously the RowLayout filled the card and the card's height came from
    // whatever placed it; with one column the content is the only thing that
    // knows how tall this should be.
    implicitHeight: body.implicitHeight + 2 * Theme.cardPaddingY
}
