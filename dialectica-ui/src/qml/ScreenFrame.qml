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

    // The card is as tall as its content plus the padding above and below it.
    // Previously the RowLayout filled the card and the card's height came from
    // whatever placed it, which gave the Rectangle NO implicitHeight at all —
    // while `Main.qml` reads exactly that value for the Flickable's
    // contentHeight. The feed therefore did not scroll at any row count. With
    // one column the content is the only thing that knows how tall this should
    // be, so the reshape has to answer it.
    //
    // This reads `body.implicitHeight` (what the children need) and never
    // `body.height` (what the frame grants), so it cannot form a loop with the
    // height binding below.
    implicitHeight: body.implicitHeight + 2 * Theme.cardPaddingY

    // `body.height` is BOUND rather than left to the column, and that line is
    // load-bearing for a reason not visible from this file.
    //
    // The obvious form is three anchors and no height binding, letting the
    // column size itself — `implicitHeight` above already carries the card's
    // height, so nothing here needs it. That form works for every screen written
    // so far and **silently breaks the next one**: a ColumnLayout with no bottom
    // constraint has height equal to its own implicit height, so it has no spare
    // space to distribute, and a child declaring `Layout.fillHeight: true`
    // collapses to its implicitHeight of 0. Measured at Qt 6.10.3 on a bare
    // Rectangle child in a 600px-high card: 544 high with the binding, 0
    // without. It renders nothing, logs nothing, raises no binding loop, and
    // passes qmllint — the failure mode is a blank region on a screen where
    // every gate is green.
    //
    // That matters because a screen wanting a body that fills the card is the
    // ordinary case, and the text such a screen owes its reader — a seed-phrase
    // permanence warning, a closed-gate reason, a publish outcome that must not
    // claim delivery — is exactly what would vanish. An obligation discharged by
    // a zero-height element is an obligation not discharged.
    //
    // So the three anchors place the column and set its width, and `height`
    // gives it the larger of what its children need and what the card actually
    // has. A `fillHeight` child then has real slack to claim — 544 again,
    // matching the two-column shell this replaced.
    //
    // **The trade this makes, stated because it is a real one.** A ColumnLayout
    // taller than its content distributes the slack *among its children*, so in
    // a card given an explicit height with no child claiming it, two 40px rows
    // land at y=111 and y=393 rather than stacked at y=0 and y=60. That is the
    // top-packing the three-edge form gave away for free, and this binding does
    // not get it back.
    //
    // It is accepted rather than solved because **no caller here gives a card an
    // explicit height**: `Main.qml` sets `Layout.preferredWidth` and alignment
    // only, so the card is always sized from `implicitHeight` above, where
    // content and card agree and nothing is scattered (measured: y=0 and y=60).
    // The scattering needs a caller that does not exist; the `fillHeight`
    // collapse would have been hit by the next screen written. Both numbers
    // above came from running the two cases, not from reasoning about them.
    //
    // A trailing `fillHeight` spacer to absorb the slack was tried and is worse:
    // it splits the space with a genuine `fillHeight` child (544 → 262) and adds
    // a `spacing` gap to `body.implicitHeight`, inflating the card by 20px so
    // the Flickable scrolls past the end of the content.
    //
    // **If you give a ScreenFrame an explicit height and its content scatters,
    // this is why** — add `Layout.fillHeight` to the child that should absorb
    // the slack, or a trailing `Item { Layout.fillHeight: true }` in that
    // screen, where it costs nothing to the screens that do not want one.
    ColumnLayout {
        id: body
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.topMargin: Theme.cardPaddingY
        anchors.leftMargin: Theme.cardPaddingX
        anchors.rightMargin: Theme.cardPaddingX
        spacing: Theme.blockGap

        // Reads `root.height`, never `root.implicitHeight`, and `implicitHeight`
        // above reads `body.implicitHeight`, never `body.height` — the two
        // bindings touch disjoint properties, so they cannot form a loop. Qt
        // reports none.
        height: Math.max(implicitHeight, root.height - 2 * Theme.cardPaddingY)
    }
}
