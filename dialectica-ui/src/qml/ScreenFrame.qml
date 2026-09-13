import QtQuick
import QtQuick.Layouts

// The card: content on paper. Every screen uses it, so padding, width and the
// hairline border are decided once.
//
// What this gives a screen, and what it asks of one, is stated for the person
// writing a screen in `docs/UI-BRIEF.md` under *What `ScreenFrame` gives you*.
// Keep the two in step. `git log` and `design.md` §4 carry how this shape was
// arrived at.
Rectangle {
    id: root

    default property alias content: body.data

    implicitWidth: Theme.cardWidth
    color: Theme.paper
    border.width: Theme.hairline
    border.color: Theme.ink

    // The card is as tall as its content plus the padding above and below it.
    // `Main.qml` reads this to size the Flickable's contentHeight, so a card
    // that does not report its height is a feed that does not scroll.
    //
    // This reads `body.implicitHeight` (what the children need) and never
    // `body.height` (what the frame grants), so it cannot form a loop with the
    // height binding below.
    implicitHeight: body.implicitHeight + 2 * Theme.cardPaddingY

    // `body.height` is BOUND rather than left to the column, and that line is
    // load-bearing for a reason not visible from this file.
    //
    // Anchoring three edges and letting the column size itself gives the same
    // card height — but a ColumnLayout with no bottom constraint has no spare
    // space to distribute, so a child declaring `Layout.fillHeight: true`
    // collapses to its implicitHeight of 0. Measured at Qt 6.10.3 on a bare
    // Rectangle child in a 600px-high card: 544 high with this binding, 0
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
    // **The trade this makes, stated because it is a real one.** A ColumnLayout
    // taller than its content distributes the slack *among its children*, so in
    // a card given an explicit height with no child claiming it, two 40px rows
    // land at y=111 and y=393 rather than stacked at y=0 and y=60. It is
    // accepted rather than solved because no caller gives a card an explicit
    // height: the card is sized from `implicitHeight` above, where content and
    // card agree and nothing is scattered.
    //
    // **If you give a ScreenFrame an explicit height and its content scatters,
    // this is why** — add `Layout.fillHeight: true` to the child that should
    // absorb the slack. Do **not** reach for a trailing
    // `Item { Layout.fillHeight: true }`: it fixes the scatter equally, but its
    // `spacing` gap enters `body.implicitHeight` and inflates the card by
    // `Theme.blockGap`, so the Flickable scrolls past the end of the content —
    // re-breaking what `implicitHeight` above exists to fix. Measured, same two
    // 40px rows: both forms give y=0 and y=60, but `implicitHeight` is 156 with
    // a real child claiming the slack and 176 with a trailing spacer.
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
