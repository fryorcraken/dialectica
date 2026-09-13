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
    //
    // **Set it through `preview()` rather than by assignment.** See below.
    property var previewing: null

    // A three-screen navigator, and it still needs no StackView: its entire
    // state is which of these two properties is non-null, and a push/pop
    // lifecycle alongside that is a second source of truth that can disagree
    // with it. One `visible:` binding each cannot. (design.md D5.)
    //
    // **`chosen` and `previewing` are never both set**, which is what makes the
    // ternary below a rendering of the state rather than a resolution of a
    // conflict. The two setters each clear the other, so the state
    // `(chosen ≠ null, previewing ≠ null)` — which has no rendering and which an
    // ordered ternary would silently resolve by accident of which test came
    // first — cannot be constructed.
    //
    // That mattered as a latent trap rather than a live one: nothing on the feed
    // emits a preview request today, so the swallowed-preview case was
    // unreachable. But the spec's own model is that an address inside a post is
    // an affordance a reader acts on, and a post lives on the feed — so the
    // piece that adds that affordance would have set `previewing` from the feed
    // and got silence. Now it gets the preview.
    readonly property string screenShown:
        root.chosen !== null ? "feed"
      : root.previewing !== null ? "join"
      : "list"

    // The two transitions, each clearing what the other owns.
    //
    // Functions rather than bare assignment because the clearing is the point:
    // a caller that assigns `previewing` directly re-creates the impossible
    // state, and a guard that must be remembered at every call site is the shape
    // CLAUDE.md says to replace with one the data enforces.
    function preview(stoa, genesis) {
        root.chosen = null
        root.previewing = { stoa: stoa, genesis: genesis }
    }

    function open(stoa, foundingTitle, genesis) {
        root.previewing = null
        root.chosen = { stoa: stoa, foundingTitle: foundingTitle, genesis: genesis }
    }

    // The way out of each screen. `closeFeed` is the counterpart of the
    // `cancelled` the join screen already had, and its absence was what stranded
    // a user on the first Stoa they opened.
    function closeFeed() {
        root.chosen = null
    }

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
                    root.preview(stoa, genesis)
                }

                onStoaChosen: (stoa, foundingTitle, genesis) => root.open(stoa, foundingTitle, genesis)
            }

            JoinScreen {
                id: join
                objectName: "joinScreen"
                visible: root.screenShown === "join"
                clipboard: clipboard
                stoaAddress: root.previewing !== null ? root.previewing.stoa : ""
                stoaGenesis: root.previewing !== null ? root.previewing.genesis : ""
                // The guard used to be re-implemented here. It now lives on the
                // property itself, so this call site — and any future one —
                // inherits it rather than having to remember it.
                heldStoas: list.visibleRows
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(Theme.cardWidth, root.width - 2 * Theme.cardPaddingX)

                onJoined: (stoa, foundingTitle, genesis) => {
                    // The record IS held for a Stoa joined in this session — it
                    // is the one the user pasted — so record it, which is what
                    // gives that row a share affordance and lets its feed open
                    // with a record. It is lost on restart, because the listing
                    // does not return retained records; closing that is a core
                    // change this piece does not make.
                    //
                    // **A JOINED Stoa is the only kind this ever holds a record
                    // for.** A Stoa the user CREATED is unshareable immediately,
                    // not merely after a restart: `create_stoa` returns
                    // `{stoa, foundingTitle, policy}` and no genesis record, so
                    // there is nothing to put in this map for it. The asymmetry
                    // is worth stating here because "lost on restart" alone
                    // reads as though creation and joining behaved alike, and
                    // they do not.
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

                // The route back. `genesisByStoa` lives on the list rather than
                // here, so reopening a Stoa is as complete as the first open and
                // the return costs the user nothing.
                onClosed: root.closeFeed()
            }

            Item { Layout.preferredHeight: Theme.cardPaddingY }
        }
    }
}
