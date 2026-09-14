import QtQuick
import QtQuick.Layouts

// The card: content on paper. Every screen uses it, so padding, width and the
// hairline border are decided once.
//
// THE CONTRACT, for whoever writes a screen. It is short, and the failure mode
// of getting it wrong is silent, so it is stated here — in the file you already
// have open — rather than in a document a screen author has no reason to find.
//
//   - The card REPORTS ITS OWN HEIGHT from its content. `Main.qml` reads that to
//     size the scroll area, so this is what makes a long feed scroll at all.
//   - DO NOT GIVE A `ScreenFrame` AN EXPLICIT `height`. Let it size from its
//     content: `Main.qml` gives the card a width and an alignment and no height.
//   - So a card is exactly as tall as its content, and THERE IS NO SLACK TO
//     FILL. A child declaring `Layout.fillHeight: true` in a content-sized card
//     measures ZERO — not because the shell withholds the space, but because a
//     card sized from its content has none to give.
//   - DESIGN SCREENS THAT GROW DOWNWARD, not screens that fill a viewport. If a
//     region should look like it occupies the rest of the page, give it a height
//     you choose — a minimum, a ratio, a fixed block — rather than asking it to
//     fill. `Layout.fillWidth` behaves as you expect; its vertical twin does
//     not, and nothing warns you: no error, no binding loop, every gate green,
//     and a blank region on the screen.
//
// `git log` carries how this shape was arrived at; the numbers behind each
// clause are in the comments below, and `tst_screen_frame_geometry.qml` pins
// them.
Rectangle {
    id: root

    default property alias content: body.data

    implicitWidth: DTheme.cardWidth
    color: DTheme.paper
    border.width: DTheme.hairline
    border.color: DTheme.ink

    // The card is as tall as its content plus the padding above and below it.
    // `Main.qml` reads this to size the Flickable's contentHeight, so a card
    // that does not report its height is a feed that does not scroll.
    //
    // This reads `body.implicitHeight` (what the children need) and never
    // `body.height` (what the frame grants), so it cannot form a loop with the
    // height binding below.
    implicitHeight: body.implicitHeight + 2 * DTheme.cardPaddingY

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
    // READ THE SCOPE OF THAT NUMBER BEFORE RELYING ON IT. It is measured in a
    // card given an EXPLICIT height, and no caller gives one — `Main.qml` sets
    // width and alignment only. In a content-sized card there is no slack to
    // distribute, so a `fillHeight` child measures 0 even with this binding.
    // What the binding buys is that a card WITH a height behaves, not that a
    // body can fill a card sized from its own content. The contract at the top
    // of this file states it for screen authors in those terms, and states it
    // that carefully because an earlier write-up of it promised the opposite.
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
    // `DTheme.blockGap`, so the Flickable scrolls past the end of the content —
    // re-breaking what `implicitHeight` above exists to fix. Measured, same two
    // 40px rows: both forms give y=0 and y=60, but `implicitHeight` is 156 with
    // a real child claiming the slack and 176 with a trailing spacer.
    ColumnLayout {
        id: body
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.topMargin: DTheme.cardPaddingY
        anchors.leftMargin: DTheme.cardPaddingX
        anchors.rightMargin: DTheme.cardPaddingX
        spacing: DTheme.blockGap

        // Reads `root.height`, never `root.implicitHeight`, and `implicitHeight`
        // above reads `body.implicitHeight`, never `body.height` — the two
        // bindings touch disjoint properties, so they cannot form a loop. Qt
        // reports none.
        height: Math.max(implicitHeight, root.height - 2 * DTheme.cardPaddingY)
    }
}
