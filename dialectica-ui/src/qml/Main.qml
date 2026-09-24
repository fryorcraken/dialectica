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
//
// The feed carries, behind the posting gate, a composer for a top-level post
// plus a vote control on each row. A reply box is not among them, and that is
// still right: this feed lists thread HEADS, so a reply box under a row would be
// a thread-view affordance on a screen that is not one. The reply composer lives
// on `DThreadScreen`, which is where a reader has a post in front of them to
// answer — and that screen is reached from a row's "read the thread".
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

    // There is no onboarding state. There was — `{stoa, foundingTitle,
    // genesis}`, the Stoa a per-Stoa identity was being chosen for — and
    // `machine-identity-scope` removed it with the route into `DOnboardingScreen`:
    // in this release the identity is this machine's key, the same in every
    // Stoa, and it is created on the Stoa list. `acquireIdentity()` below is the
    // route now. The release that restores per-Stoa identity (#108) puts the
    // state back; `view-navigation`'s removed requirement names what it restores.

    // The thread being read, or `null`. `{stoa, foundingTitle, genesis,
    // rootOp}` — the feed it was opened from, plus the root post's identifier.
    //
    // **The whole feed context travels, not just the root op**, because the way
    // back is "the feed for the Stoa the thread was opened from" and
    // reconstructing that from an op id is not something the view can do.
    //
    // **`chosen` is CLEARED while a thread is open**, and the feed context lives
    // here instead. The alternative — leaving `chosen` set so the feed is
    // "still there underneath" — would make two properties non-null at once and
    // turn the ternary below from a rendering of the state into a resolution of
    // a conflict, which is the exact shape the ordering of these properties
    // exists to make unconstructible. One state, one property.
    property var reading: null

    // The Stoa whose moderation screen is open, or `null`. `{stoa,
    // foundingTitle, genesis}` — the same shape `chosen` carries, and carrying
    // the whole thing rather than a bare address is what lets the return land
    // back on the feed the user left, exactly as a thread's return does.
    //
    // **The screen it opens publishes nothing**, which changes nothing about how
    // it is routed to: an inert screen is reached and left like any other, and
    // the navigator is not the layer that knows what a screen can do.
    property var moderating: null

    // A five-screen navigator, and it still needs no StackView: its entire
    // state is which of these properties is non-null, and a push/pop
    // lifecycle alongside that is a second source of truth that can disagree
    // with it. One `visible:` binding each cannot. (design.md D5.)
    //
    // **No two of them are ever both set**, which is what makes the ternary
    // below a rendering of the state rather than a resolution of a conflict.
    // Every setter clears the others, so a state with two set — which has no
    // rendering, and which an ordered ternary would silently resolve by accident
    // of which test came first — cannot be constructed.
    //
    // That mattered as a latent trap rather than a live one: nothing on the feed
    // emits a preview request today, so the swallowed-preview case was
    // unreachable. But the spec's own model is that an address inside a post is
    // an affordance a reader acts on, and a post lives on the feed — so the
    // piece that adds that affordance would have set `previewing` from the feed
    // and got silence. Now it gets the preview.
    //
    // **Identity is not in this expression, and must not be put in it.** The
    // route to acquiring one happens on a SIGNAL — an event — never on a retained
    // "does this user have an identity" answer. A navigation layer holding that
    // boolean is wrong for every screen at once the moment a keystore changes
    // under it, which is `FeedScreen`'s own no-caching rule one level up.
    // (design.md D4.)
    readonly property string screenShown:
        root.moderating !== null ? "moderation"
      : root.reading !== null ? "thread"
      : root.chosen !== null ? "feed"
      : root.previewing !== null ? "join"
      : "list"

    // ---- the one transition primitive -----------------------------------
    //
    // **Every transition below goes through `enterOnly`, and none of them clears
    // a sibling by hand.** That is a reshaping of what was here, and the reason
    // is the defect the old shape produced rather than a preference for brevity.
    //
    // Each setter used to name the other states and set each to `null`. With
    // four states that is four functions each listing three names — a guard that
    // must be got right at every call site, which is the shape CLAUDE.md says to
    // replace with one the data enforces. **It was already wrong**: adding this
    // change's `moderation` state required editing five functions, and two of
    // them (`openThread`, and the identity route) had been written with only three
    // of the four clears in the first place, so the invariant held by accident
    // of which screens could reach which rather than by anything in the code.
    // A seventh state would have meant getting six more edits right.
    //
    // Here the list of states is written ONCE, and entering one is "write this
    // one, null everything else in the list". A new state is one entry in
    // `stateNames` plus one line in `screenShown`; no existing function changes,
    // and there is no call site at which the clearing can be got wrong, because
    // no call site does any clearing. Removing one is the same edit backwards,
    // which is how `onboarding` left (`machine-identity-scope`).
    //
    // Behaviour is unchanged for every transition that was correct before. The
    // two that were not are now correct, which is the point.
    //
    // Lower-case initial because QML refuses a property name beginning with an
    // upper-case letter — `STATES` compiles to "Property names cannot begin with
    // an upper case letter", which takes `Main.qml` out entirely rather than
    // failing locally.
    readonly property var stateNames: ["previewing", "chosen", "reading",
                                       "moderating"]

    // Enter `name` carrying `payload`, and leave every other state empty.
    //
    // `payload` of `null` is how a state is LEFT: `enterOnly("", null)` empties
    // all of them, which is the list screen. There is deliberately no "clear one
    // state" primitive — a function that emptied one state without touching the
    // others would be the hand-clearing shape coming back through a different
    // door.
    function enterOnly(name, payload) {
        for (var i = 0; i < root.stateNames.length; i++) {
            var key = root.stateNames[i]
            root[key] = (key === name) ? payload : null
        }
    }

    // The transitions. Each names the state it enters and nothing else.
    function preview(stoa, genesis) {
        root.enterOnly("previewing", { stoa: stoa, genesis: genesis })
    }

    function open(stoa, foundingTitle, genesis) {
        root.enterOnly("chosen", { stoa: stoa, foundingTitle: foundingTitle,
                                   genesis: genesis })
    }

    // Into a thread, from the feed row that heads it.
    //
    // **What travels is the Stoa, that Stoa's founding record where the view
    // holds one, and the ROOT POST's identifier.** The record travels for the
    // same reason it travels to the feed: moderation cannot be resolved for a
    // Stoa whose record this peer does not hold.
    //
    // **Nothing is invented for a Stoa the view holds no record for.** The
    // genesis passed here is whatever `chosen` carries — "" where the listing
    // returned none — and it goes to the core unchanged. A fabricated or
    // placeholder record would fail verification in the core and surface as a
    // refusal the user cannot act on.
    // **A row naming no op opens nothing**, rather than opening a thread screen
    // that immediately asks core about a thread identified by the empty string.
    // `FeedScreen.threadTarget()` already withholds the affordance from such a
    // row, so this is the same judgement held a second time at the transition —
    // a guard is a job, and "is it applied everywhere?" stays a question with an
    // answer only if the navigator does not depend on every future caller having
    // remembered it. `thread-view`'s *No thread is rendered before one has been
    // chosen* requires that no read be made for a thread the user never asked
    // for, and this is where that is enforced for the route.
    function openThread(rootOp) {
        if (root.chosen === null || rootOp === "")
            return
        var from = root.chosen
        root.enterOnly("reading", { stoa: from.stoa, foundingTitle: from.foundingTitle,
                                    genesis: from.genesis, rootOp: rootOp })
    }

    // Out of the thread, back to the feed it was opened from — without the view
    // being restarted, which is what makes this a return rather than a reset.
    function closeThread() {
        var was = root.reading
        if (was === null)
            root.enterOnly("", null)
        else
            root.enterOnly("chosen", { stoa: was.stoa, foundingTitle: was.foundingTitle,
                                       genesis: was.genesis })
    }

    // To where an identity is acquired: the Stoa list, where this machine's key
    // is created.
    //
    // **In this release the identity is the machine key, the same in every
    // Stoa** (`identity`, `machine-identity-scope`), so there is no per-Stoa
    // choice to route to and nothing about the Stoa the user came from travels.
    // This used to be `createIdentityFor(stoa, …)`, entering the per-Stoa
    // onboarding screen with that Stoa — issue #149: the per-Stoa slate that
    // #108 schedules for 0.0.3, reachable in 0.0.1.
    //
    // **It makes no call of its own.** Rendering the list is the whole route.
    // The list, once shown, asks the read-only master-key query, as
    // `stoa-navigation-view` requires of every showing. That call is the list's
    // and not the route's (`view-navigation`, *Acquiring an identity is reached
    // from the navigator*). Creating the key is an action the user takes there,
    // and nothing the route leads to creates a key, requests a slate or keeps a
    // candidate on their behalf.
    //
    // `enterOnly("", null)` is the list by the navigator's own definition, so
    // this is the same transition `closeFeed()` makes — named for what the user
    // asked for rather than for what it happens to share with leaving a feed.
    // `tst_navigation.qml`'s no-call-of-its-own test uses `closeFeed()` as its
    // control on the strength of that shared body, so a change here that
    // diverges from it has to revisit that control.
    function acquireIdentity() {
        root.enterOnly("", null)
    }

    // Into the moderation screen, from the feed of the Stoa it is about.
    //
    // **The whole feed context travels**, exactly as it does into a thread,
    // because the way back is "the feed this was opened from" and the navigator
    // is the only layer holding it.
    //
    // **The screen this opens publishes nothing.** That is the screen's business
    // and not the navigator's: routing to an inert screen is routing, and a
    // navigator that branched on what a screen can do would be a second place
    // where the trait's surface is written down.
    function moderateIn(stoa, foundingTitle, genesis) {
        root.enterOnly("moderating", { stoa: stoa, foundingTitle: foundingTitle,
                                       genesis: genesis })
    }

    // Out of moderation, back to the feed it was entered from.
    //
    // **Unconditional on what the user did there**: a route out offered only on
    // some outcomes is a route absent in exactly the cases where the user is
    // stuck. On this screen that
    // is sharper than elsewhere, because every other control does nothing — so
    // this is the only control that answers a press at all.
    function closeModeration() {
        var was = root.moderating
        if (was === null)
            root.enterOnly("", null)
        else
            root.enterOnly("chosen", { stoa: was.stoa, foundingTitle: was.foundingTitle,
                                       genesis: was.genesis })
    }

    // The way out of each screen. `closeFeed` is the counterpart of the
    // `cancelled` the join screen already had, and its absence was what stranded
    // a user on the first Stoa they opened.
    function closeFeed() {
        root.enterOnly("", null)
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

                onCancelled: root.enterOnly("", null)
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

                // The route to acquiring an identity. The feed's footer chip
                // raises this when the identity report says there is nobody
                // here, and it lands on the Stoa list, where this machine's key
                // is created. Nothing about this Stoa travels: the key is not for
                // it, it is for every Stoa.
                onCreateIdentityRequested: root.acquireIdentity()

                // The route into a thread. The root op travels with the signal;
                // the Stoa and its record come from `chosen`, which the
                // navigator already holds.
                onThreadOpened: (rootOp) => root.openThread(rootOp)

                // The route into moderation. Same shape as the two above: the
                // screen asks, the navigator decides where that lands and what
                // travels.
                onModerationRequested: root.moderateIn(
                    root.chosen !== null ? root.chosen.stoa : "",
                    root.chosen !== null ? root.chosen.foundingTitle : "",
                    root.chosen !== null ? root.chosen.genesis : "")
            }

            // ---- no onboarding screen ---------------------------------------
            //
            // `DOnboardingScreen` — the per-Stoa candidate slate — is not
            // mounted, and `qmldir` records why. It stood here with a Back
            // button declared outside it (so the screen, contracted to navigate
            // nowhere itself, had no phase-bound way out to get wrong). The
            // release that restores per-Stoa identity mounts it again, and that
            // arrangement is the one to restore.

            // ---- the thread screen -----------------------------------------
            DThreadScreen {
                id: thread
                objectName: "thread"
                visible: root.screenShown === "thread"
                // Every value comes from `reading`, which `openThread()` filled
                // from the feed the row was rendered in. There is no property
                // on the screen holding a usable default — a second source for
                // the thread it renders is a build shipping a hardcoded one.
                //
                // **`threadId`, and it is the ROOT POST's `id`.** The screen
                // names the property that way rather than `threadRoot` because
                // what travels is an op id and not a thread handle: `id` does
                // not move when the post is revised, where `currentVersion`
                // does, so a route carrying the version would point at a thread
                // that stops answering to it after an edit.
                stoaAddress: root.reading !== null ? root.reading.stoa : ""
                stoaTitle: root.reading !== null ? root.reading.foundingTitle : ""
                stoaGenesis: root.reading !== null ? root.reading.genesis : ""
                threadId: root.reading !== null ? root.reading.rootOp : ""
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(DTheme.cardWidth, root.width - 2 * DTheme.cardPaddingX)

                onClosed: root.closeThread()
            }

            // ---- the moderation screen -------------------------------------
            //
            // **Mounted so the screen can be SEEN**, which is the whole of what
            // the owner's reversal of PLAN.md ruling 3 asked for. A registered
            // screen nothing instantiates passes every test written about it and
            // cannot be opened, and that is the defect `check_qml_reachable.py`
            // exists for — the worst possible one to ship on a screen whose only
            // purpose in this build is to be looked at.
            //
            // **The subject is a fixture, and it is supplied HERE rather than
            // inside the screen.** Nothing routes a real post into moderation,
            // because nothing on a feed row offers a moderate affordance — the
            // route in is the feed's header link, which names no post. So the
            // confirmation needs a subject and there is none to give it. It is
            // passed from the navigator so the screen holds no fixture of its own
            // beyond its two lists, which keeps the screen's own mock surface in
            // one place. PLAN.md §9.2's case 2 entry 5 carries it.
            DModerationScreen {
                id: moderation
                objectName: "moderation"
                visible: root.screenShown === "moderation"
                stoaAddress: root.moderating !== null ? root.moderating.stoa : ""
                subjectName: "slow cobalt lamplighter"
                subjectAddress: "c04e77b1a92f3c8d4e17b6520fa9c3d1de51a92f7b408c6e35a1f2d98c3714ab"
                subjectExcerpt: "“Keystore mode 0644 — what actually breaks…”"
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(DTheme.cardWidth, root.width - 2 * DTheme.cardPaddingX)

                onClosed: root.closeModeration()
            }

            // ---- shared chrome: the three lamps ---------------------------
            //
            // **On EVERY main-area screen, not a subset**, which is the
            // requirement rather than a placement preference. A status indicator
            // absent from a screen is an indicator whose absence a user reads as
            // "nothing to report" — and chrome reporting the machine's condition
            // is most needed exactly where something has gone wrong. A
            // per-screen subset is what makes its absence ambiguous: a user
            // cannot tell a screen that omits the lamps from a machine with
            // nothing to report.
            //
            // So it is declared HERE, outside every `visible:` binding above,
            // and there is no state in which it is withheld. That it accompanies
            // every screen holds by construction: there is no per-screen
            // condition in scope at this point in the file.
            //
            // **Rendered before its values arrive, and least-claiming while they
            // have not.** Every screen passes through the unbound state between
            // appearing and core answering, so withholding the bar would hide it
            // exactly during the window in which the machine's condition is
            // least known. `DStatusBar` defaults every lamp to `degraded` rather
            // than `ok` and maps every unrecognised state to `degraded` too, so
            // an unbound lamp claims nothing — which is what makes leaving a
            // lamp unbound honest rather than negligent.
            //
            // **DELIVERY IS DELIBERATELY UNBOUND.** It has no honest source: the
            // outcome arrives asynchronously through delivery's channel events,
            // after the publish call has returned, so no synchronous call
            // produces a signal to bind. It is a documented §9.2 case-2
            // placeholder — see PLAN.md case 2 entry 6 and design.md D7. Do not
            // invent a heuristic to fill it.
            //
            // The tooltip strings are likewise unset. `DStatusBar` invents
            // nothing when they are, so no explanation is shown and none is
            // wrong — a strictly smaller claim than a sentence nobody checked.
            DStatusBar {
                objectName: "statusBar"
                Layout.alignment: Qt.AlignHCenter
                Layout.topMargin: DTheme.blockGap

                // MOCK, and marked as one. Nothing in the contract reports
                // whether the store is readable as a machine-wide condition —
                // each read reports its own outcome — so this lamp reads the one
                // honest signal the view holds: whether the screen that last
                // read the store succeeded. PLAN.md case 2 entry 7.
                storageState: root.screenShown === "list"
                    ? (list.readState === "failed" ? "failed"
                       : list.readState === "ok" ? "ok" : "degraded")
                    : root.screenShown === "feed"
                    ? (feed.readState === "failed" ? "failed"
                       : feed.readState === "ok" ? "ok" : "degraded")
                    : "degraded"

                // Whether this machine is in a Stoa at all. The one lamp with a
                // genuine source: the membership listing answers it.
                zoneState: list.readState === "ok"
                    ? (list.visibleRows.length > 0 ? "ok" : "degraded")
                    : "degraded"
            }

            Item { Layout.preferredHeight: DTheme.cardPaddingY }
        }
    }
}
