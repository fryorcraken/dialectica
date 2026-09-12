import QtQuick
import QtQuick.Layouts

// Dialectica — the view.
//
// Basecamp sandboxes this engine with a deny-all network access manager and no
// filesystem access outside the plugin directory (PLAN.md §2.1), so the view
// cannot fetch or read anything itself. Everything below goes through the core
// module, via the `Core` singleton, which is the only place a core call is
// written.
//
// ## There is no screen here yet, and that is the point
//
// This file is the CHASSIS: the desk ground, a scrolling viewport, and one card
// centred in it at the supplied width. It hosts no screen, because none of the
// screens worth building can render anything yet — no identity exists, no Stoa
// exists, and core is read-only. Identity creation is the first screen (PLAN.md
// §9.1), and it arrives as its own change.
//
// A module that loads and shows an empty card is honest about that: it proves
// the plugin loads, the theme resolves and the card sizes itself, which is
// exactly what this change fixed and all it claims. A module that showed a feed
// over a Stoa that cannot exist would be asserting something false, and the
// first launch already demonstrated what that costs to debug.
//
// ## Why no StackLayout and no loader
//
// Sibling screens are coming and the eventual shape here is almost certainly a
// StackLayout or a Loader keyed on some route. It is not built yet DELIBERATELY:
// navigation written before there are two destinations encodes a guess about how
// they are selected, and the first real screen is what turns that guess into a
// fact. One screen needs no navigation; two will say what kind it needs.
Item {
    id: root

    Rectangle {
        anchors.fill: parent
        color: DTheme.desk
    }

    Flickable {
        anchors.fill: parent
        contentWidth: width

        // Derived from the COLUMN, not from the card plus a repetition of the
        // padding arithmetic. The old form was
        // `frame.implicitHeight + 2 * cardPaddingY`, which restated the two
        // spacer heights below and so had two ways to be wrong: it could
        // disagree with the spacers, and — the failure that actually shipped —
        // it silently became just the padding when `ScreenFrame` reported an
        // implicitHeight of 0, leaving a Flickable that scrolled over nothing
        // while the card's content was drawn on top of itself.
        //
        // Reading the column's own implicitHeight means the spacers are counted
        // because they are IN it, so the number cannot drift from the layout it
        // describes.
        contentHeight: page.implicitHeight
        clip: true

        ColumnLayout {
            id: page
            width: parent.width
            spacing: 0

            Item { Layout.preferredHeight: DTheme.cardPaddingY }

            // The card the first screen will be built in. Empty until that
            // screen exists; its own implicitHeight keeps it from collapsing
            // into the spacers above and below.
            ScreenFrame {
                id: frame
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(DTheme.cardWidth, root.width - 2 * DTheme.cardPaddingX)
            }

            Item { Layout.preferredHeight: DTheme.cardPaddingY }
        }
    }
}
