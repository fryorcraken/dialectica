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
// This is Stage A of PLAN.md §9.1 and only its first screen: a feed over one
// Stoa, with the empty and unreadable states that must never look alike. There
// is no onboarding, no Stoa list, no thread view and no composer — each is a
// later change, and each needs core methods that do not exist yet.
Item {
    id: root

    // Which Stoa this view reads.
    //
    // **Nothing in core answers this yet**, and that is honest rather than
    // unfinished: §9.1 Stage D is where `joinStoa` and a list of held Stoas
    // arrive, and until then there is no source for an address but the one a
    // developer supplies. So it is empty by default and the feed says plainly
    // that it was given nothing, rather than inventing a Stoa to show.
    property string stoaAddress: ""
    property string stoaTitle: ""

    // The Stoa's genesis record, hex. See FeedScreen — it travels with the
    // request because nothing records joined Stoas yet, and it is safe to pass
    // because core verifies it hashes to the address.
    property string stoaGenesis: ""

    Rectangle {
        anchors.fill: parent
        color: Theme.desk
    }

    Flickable {
        anchors.fill: parent
        contentWidth: width
        contentHeight: frame.implicitHeight + 2 * Theme.cardPaddingY
        clip: true

        ColumnLayout {
            width: parent.width
            spacing: 0

            Item { Layout.preferredHeight: Theme.cardPaddingY }

            FeedScreen {
                id: frame
                stoaAddress: root.stoaAddress
                stoaTitle: root.stoaTitle
                stoaGenesis: root.stoaGenesis
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(Theme.cardWidth, root.width - 2 * Theme.cardPaddingX)
            }

            Item { Layout.preferredHeight: Theme.cardPaddingY }
        }
    }
}
