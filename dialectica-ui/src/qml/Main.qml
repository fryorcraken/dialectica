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
// **The view holds no Stoa of its own.** It used to take `stoaAddress`,
// `stoaTitle` and `stoaGenesis` as properties a developer filled in by hand,
// because nothing in the core recorded which Stoas this peer was in. That is no
// longer true, and leaving the properties in place would leave a SECOND source
// for the one value these screens exist to supply — a build shipping a hardcoded
// Stoa, and a Stoa on screen the membership does not record. Removing them is
// what makes that unrepresentable: there is no longer anywhere to put one.
Item {
    id: root

    // The Stoa whose feed is up, or `null` when the list is.
    //
    // `{stoa, foundingTitle, genesis}`. `genesis` is "" where the view holds no
    // record — the listing does not return one — and FeedScreen passes that
    // through to the core unchanged. The core then refuses it, and the feed
    // renders that refusal, which is honest and is distinguishable from a Stoa
    // holding nothing. **Nothing here invents a record**: a fabricated one would
    // fail verification in the core and surface as a refusal the user cannot
    // act on.
    property var chosen: null

    // A reference being previewed, or `null`. Set by the paste field and by an
    // in-post affordance; NEVER set by anything that also joins.
    property var previewing: null

    // A two-state navigator needs no StackView. Its entire state is which of
    // these three is non-null, and a push/pop lifecycle alongside that is a
    // second source of truth that can disagree with it. One `visible:` binding
    // each cannot.
    readonly property string screenShown:
        root.chosen !== null ? "feed"
      : root.previewing !== null ? "join"
      : "list"

    Rectangle {
        anchors.fill: parent
        color: Theme.desk
    }

    // The view's one clipboard, shared by every screen that copies. See
    // ClipboardSink for why it is a hidden TextEdit and not a platform API.
    ClipboardSink { id: clipboard }

    Flickable {
        anchors.fill: parent
        contentWidth: width
        contentHeight: pane.implicitHeight + 2 * Theme.cardPaddingY
        clip: true

        ColumnLayout {
            id: pane
            width: parent.width
            spacing: 0

            Item { Layout.preferredHeight: Theme.cardPaddingY }

            StoaListScreen {
                id: list
                objectName: "stoaList"
                visible: root.screenShown === "list"
                clipboard: clipboard
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(Theme.cardWidth, root.width - 2 * Theme.cardPaddingX)

                onPreviewRequested: (stoa, genesis) => {
                    // Acting on a pasted reference reaches a PREVIEW and joins
                    // nothing. An interface that joined here would enrol a user
                    // in a Stoa they never chose.
                    root.previewing = { stoa: stoa, genesis: genesis }
                }

                onStoaChosen: (stoa, foundingTitle, genesis) => {
                    root.chosen = { stoa: stoa, foundingTitle: foundingTitle, genesis: genesis }
                }
            }

            JoinScreen {
                id: join
                objectName: "joinScreen"
                visible: root.screenShown === "join"
                clipboard: clipboard
                stoaAddress: root.previewing !== null ? root.previewing.stoa : ""
                stoaGenesis: root.previewing !== null ? root.previewing.genesis : ""
                heldStoas: list.readState === "ok" ? list.rows : []
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(Theme.cardWidth, root.width - 2 * Theme.cardPaddingX)

                onJoined: (stoa, foundingTitle, genesis) => {
                    // The record IS held for a Stoa joined in this session — it
                    // is the one the user pasted — so record it, which is what
                    // gives that row a share affordance and lets its feed open
                    // with a record. It is lost on restart, because the listing
                    // does not return retained records; closing that is a core
                    // change this piece does not make.
                    var held = list.genesisByStoa
                    held[stoa] = genesis
                    list.genesisByStoa = held
                    list.reload()
                }

                onCancelled: root.previewing = null
            }

            FeedScreen {
                id: feed
                objectName: "feed"
                visible: root.screenShown === "feed"
                stoaAddress: root.chosen !== null ? root.chosen.stoa : ""
                stoaTitle: root.chosen !== null ? root.chosen.foundingTitle : ""
                stoaGenesis: root.chosen !== null ? root.chosen.genesis : ""
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(Theme.cardWidth, root.width - 2 * Theme.cardPaddingX)
            }

            Item { Layout.preferredHeight: Theme.cardPaddingY }
        }
    }
}
