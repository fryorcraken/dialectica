import QtQuick
import QtQuick.Layouts

// The card: content on paper. Every screen is built in one.
//
// It was a two-column card until the first launch, with a 244px "apparatus"
// column on the right carrying margin notes. That column was the design
// bundle's ANNOTATION mechanism — notes explaining the design to someone
// reading the mockup — and its own copy gives it away: "When real times arrive
// this label changes and nothing else does" is addressed to a developer, not to
// a forum reader. So the column is gone and the card is one column.
//
// The rendering obligations the notes carried are not gone; they live in
// docs/UI-BRIEF.md, which is where a screen built in this card goes to find out
// what it is and is not allowed to claim.
Rectangle {
    id: root

    default property alias content: body.data

    implicitWidth: DTheme.cardWidth

    // A Rectangle is NOT a layout, so nothing derives its height from its
    // children — and this card's children are its entire content. Without this
    // the card reported height 0 while holding several hundred pixels: every
    // child was laid out at the same y, text drew on top of text, and
    // `Main.qml`'s `contentHeight: frame.implicitHeight + 2 * cardPaddingY`
    // scrolled over nothing but padding.
    //
    // Basecamp is what makes this fatal rather than cosmetic: it hosts a
    // `ui_qml` view inside a layout, so the view is sized by its PARENT asking
    // how tall it wants to be. A root that cannot answer gets a size it did not
    // choose. Radicle's tst_layout.qml records the same defect from the same
    // cause, found the same way.
    //
    // `implicitHeight` rather than `height`: the parent stays free to give the
    // card more room than it asked for, which is what a Flickable in a tall
    // window does. Binding `height` would fight the parent instead.
    implicitHeight: body.implicitHeight + 2 * DTheme.cardPaddingY

    color: DTheme.paper
    border.width: DTheme.hairline
    border.color: DTheme.ink

    ColumnLayout {
        id: body
        objectName: "screenBody"

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.leftMargin: DTheme.cardPaddingX
        anchors.rightMargin: DTheme.cardPaddingX
        anchors.topMargin: DTheme.cardPaddingY

        spacing: DTheme.blockGap
    }
}
