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
// **The view holds no Stoa of its own to RENDER.** It used to take
// `stoaAddress`, `stoaTitle` and `stoaGenesis` as properties a developer filled
// in by hand, because nothing in the core recorded which Stoas this peer was in.
// That is no longer true, and leaving the properties in place would leave a
// SECOND source for the one value these screens exist to supply — a build
// shipping a hardcoded Stoa, and a Stoa on screen the membership does not
// record. Removing them is what makes that unrepresentable: there is no longer
// anywhere to put one. `chosen` is the only answer to "whose feed is up".
//
// `identityStoa` is the one address that survives, and it is a different
// question: core's identity calls each take a Stoa and refuse a request without
// one, so onboarding has nothing to ask without an address. It renders nothing.
// See its own comment for why that is honest under §5.2's MVP waypoint and what
// has to change when the waypoint ends.
//
// The app branches on launch between onboarding and that navigator, on the
// module's answer to who-am-I rather than on any flag this view stored.
//
// The feed carries, behind the posting gate, a composer for a top-level post
// plus a vote control on each row. There is still no THREAD view, and that is
// why there is no reply box: this feed lists thread heads, so a reply box under
// a row would be a thread-view affordance on a screen that is not one.
// `DComposer.qml` supports replying and is tested in that mode; the
// instantiation arrives with the thread screen.
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

    // ---- the launch branch ------------------------------------------------
    //
    // Which screen to show is **the module's answer**, never a flag this view
    // remembered. A remembered "this user has onboarded" outlives the thing it
    // remembers: a keystore that was deleted, moved or is unreadable leaves the
    // flag set and the user looking at a forum they cannot post in, with no path
    // back to the screen that would fix it. The module is the only party that
    // can see the store, so it is the only party that can answer.
    //
    //   "unknown"  not asked yet — reachable only before onCompleted has run
    //   "present"  there is an identity: the navigator below
    //   "absent"   there is none: onboarding
    //   "failed"   the call itself failed: neither branch, because which one is
    //              right is exactly what is not known
    property string identityState: "unknown"

    // The Stoa the identity question is asked ABOUT, and the only thing this
    // view holds a Stoa address for.
    //
    // **This is not a second source for which Stoa is on screen** — the property
    // the comment above says was removed. That one answered "whose feed is up",
    // and `chosen` answers it now. This one answers a different question the
    // core contract forces: `who_am_i`, `generate_identity_slate` and
    // `keep_identity` all take a Stoa and refuse a request without one
    // (`wire.rs`'s `parse_stoa`), so onboarding cannot ask anything at all
    // without an address. Nothing renders from it and no screen reads it as
    // "the current Stoa".
    //
    // That it is supplied from outside is the honest shape while identity is
    // per-peer: PLAN.md §5.2's MVP waypoint gives a user ONE identity across
    // every Stoa, so which Stoa the question names does not change the answer.
    // When §5.2's destination arrives and identity becomes per-Stoa again, this
    // is the seam that has to change — the question becomes one per Stoa, and
    // the gate moves inside the navigator rather than above it.
    property string identityStoa: ""

    // The module's reason when there is no identity, held UNPARSED.
    //
    // Both absent cases reach onboarding — no identity stored, and an identity
    // that exists but could not be loaded — because in each the user has no
    // usable identity and the screen that offers one is where they must arrive.
    // Core distinguishes the two by this string alone; keeping it verbatim is
    // what leaves them distinguishable to a later screen.
    //
    // Deliberately NOT classified here. Any `indexOf(...)` over core's wording
    // would be a second copy of core's error taxonomy, maintained in the wrong
    // module, and would silently reclassify the day core rewords a message.
    property string identityReason: ""

    // What the module said about recovery, three-valued: `undefined` means it
    // did not say, and the screen then makes no claim either way.
    property var recoveryNeedsTheRecord: undefined

    // The failure text when the report itself failed.
    property string identityFailure: ""

    Component.onCompleted: root.askWhoAmI()

    // Asked on launch, and asked AGAIN after a keep reports success. The second
    // ask costs a round trip and buys the property that no screen state is ever
    // derived from an action having been invoked: even a transition this view
    // just watched happen is decided by the module, which is the only party
    // that knows whether anything was written.
    function askWhoAmI() {
        if (root.identityStoa === "") {
            root.identityState = "failed"
            root.identityFailure = "No Stoa address was given to this view."
            return
        }

        var reply = Core.whoAmI(root.identityStoa)

        if (!reply.ok) {
            // Neither branch. Showing the forum would claim an identity that was
            // never reported; showing onboarding would offer to replace one
            // that may well exist.
            root.identityState = "failed"
            root.identityFailure = reply.error
            return
        }

        if (reply.value.hasIdentity === true) {
            root.identityReason = ""
            root.recoveryNeedsTheRecord = reply.value.recoveryNeedsTheRecord
            root.identityFailure = ""
            root.identityState = "present"
            return
        }

        // `hasIdentity` absent, false, or any value that is not `true` — all of
        // them are "the module did not report an identity", and none of them
        // may open the forum. Failing towards onboarding is safe in a way the
        // reverse is not: onboarding's own keep is refused by core where an
        // identity already exists, so the worst case is a refusal the user can
        // read, rather than a forum they cannot post in.
        // A non-string reason is held as absent rather than stringified.
        // `String({...})` yields `[object Object]`, and this value exists to
        // keep the two absent cases distinguishable to a later screen — a
        // placeholder that is the same text for every unreadable reason
        // distinguishes nothing, while an empty string is honestly "the module
        // gave no reason this view could read".
        root.identityReason = typeof reply.value.reason === "string"
            ? reply.value.reason
            : ""
        root.recoveryNeedsTheRecord = reply.value.recoveryNeedsTheRecord
        root.identityFailure = ""
        root.identityState = "absent"
    }

    // A four-screen navigator, and it still needs no StackView: its entire
    // state is the identity answer plus which of these two properties is
    // non-null, and a push/pop lifecycle alongside that is a second source of
    // truth that can disagree with it. One `visible:` binding each cannot.
    // (design.md D5.)
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
    // **The identity answer is tested FIRST**, so the three navigator screens
    // are reachable only once the module has reported an identity. That
    // ordering is the requirement rather than a preference: "both absent cases
    // reach onboarding" and "a failed report shows neither branch" are only
    // true if no `chosen`/`previewing` state can outrank them. Testing identity
    // last would let a Stoa opened before a keystore went missing keep its feed
    // on screen.
    readonly property string screenShown:
        root.identityState === "failed" ? "identityFailed"
      : root.identityState !== "present" ? "onboarding"
      : root.chosen !== null ? "feed"
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
        color: DTheme.desk
    }

    // The view's one clipboard, shared by every screen that copies. See
    // DClipboardSink for why it is a hidden TextEdit and not a platform API.
    DClipboardSink { id: clipboard }

    Flickable {
        anchors.fill: parent
        contentWidth: width
        contentHeight: pane.implicitHeight + 2 * DTheme.cardPaddingY
        clip: true

        ColumnLayout {
            id: pane
            width: parent.width
            spacing: 0

            Item { Layout.preferredHeight: DTheme.cardPaddingY }

            DOnboardingScreen {
                id: onboarding
                objectName: "onboarding"
                visible: root.screenShown === "onboarding"
                stoaAddress: root.identityStoa
                recoveryNeedsTheRecord: root.recoveryNeedsTheRecord
                // A kept identity is not this screen's word for it. The module
                // is asked again, and the answer is what moves the branch.
                onIdentityKept: root.askWhoAmI()
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(DTheme.cardWidth, root.width - 2 * DTheme.cardPaddingX)
            }

            // The report itself failed. No navigator screen and no slate,
            // because the view does not know which one is right — and the
            // module's own message is what is shown, rather than a reworded one.
            ScreenFrame {
                objectName: "identityFailure"
                visible: root.screenShown === "identityFailed"
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(DTheme.cardWidth, root.width - 2 * DTheme.cardPaddingX)

                Text {
                    text: "Whether you have an identity here could not be determined."
                    font: DTheme.heading
                    color: DTheme.accent
                    wrapMode: Text.WordWrap
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                }

                Text {
                    text: "Neither the forum nor a new identity is offered, because which of the two is right is exactly what is not known."
                    font: DTheme.bodySmall
                    color: DTheme.inkSoft
                    wrapMode: Text.WordWrap
                    lineHeight: 1.55
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                }

                Text {
                    text: root.identityFailure
                    font: DTheme.address
                    color: DTheme.ink
                    wrapMode: Text.WrapAnywhere
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                }

                FlatButton {
                    text: "Ask again"
                    kind: "primary"
                    onClicked: root.askWhoAmI()
                }
            }

            DStoaListScreen {
                id: list
                objectName: "stoaList"
                visible: root.screenShown === "list"
                clipboard: clipboard
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(DTheme.cardWidth, root.width - 2 * DTheme.cardPaddingX)

                onPreviewRequested: (stoa, genesis) => {
                    // Acting on a pasted reference reaches a PREVIEW and joins
                    // nothing. An interface that joined here would enrol a user
                    // in a Stoa they never chose.
                    root.preview(stoa, genesis)
                }

                onStoaChosen: (stoa, foundingTitle, genesis) => root.open(stoa, foundingTitle, genesis)
            }

            DJoinScreen {
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
                Layout.preferredWidth: Math.min(DTheme.cardWidth, root.width - 2 * DTheme.cardPaddingX)

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
                Layout.preferredWidth: Math.min(DTheme.cardWidth, root.width - 2 * DTheme.cardPaddingX)

                // The route back. `genesisByStoa` lives on the list rather than
                // here, so reopening a Stoa is as complete as the first open and
                // the return costs the user nothing.
                onClosed: root.closeFeed()
            }

            Item { Layout.preferredHeight: DTheme.cardPaddingY }
        }
    }
}
