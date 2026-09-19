import QtQuick
import QtQuick.Layouts

// Screen 04 (feed) and screen 07 (empty vs unreadable), which are one screen
// because they are three states of the same read.
//
// The three states are mutually exclusive by construction — `state` is computed
// from one variable, so no combination of flags can render two at once. That is
// the point: `SPEC.md` requires an empty feed and an unreadable store to be told
// apart by "a named failure — never by the same neutral empty list", and two
// independent booleans is how that eventually renders one as the other.
ScreenFrame {
    id: screen

    // The Stoa being read. Supplied by whatever navigated here.
    property string stoaAddress: ""
    property string stoaTitle: ""

    // The Stoa's genesis record, hex. Core needs it to know who may moderate,
    // and nothing records joined Stoas yet — so it travels with the request.
    //
    // Passing it from the view is safe rather than a hole: the address is the
    // hash of this record, so core re-derives it and refuses a mismatch. A
    // record naming a different creator describes a different Stoa and is
    // rejected, not obeyed.
    property string stoaGenesis: ""

    // ---- read state -----------------------------------------------------
    //
    // One enum-ish string rather than several booleans:
    //   "unread"  nothing asked for yet
    //   "ok"      the store answered; `rows` is what it holds (possibly none)
    //   "failed"  the store could not be read; `failure` says why
    property string readState: "unread"
    property var rows: []
    property string failure: ""
    property bool hasMore: false
    property int page: 0
    property bool includeHidden: false

    // The posting gate. Probed on every render, never cached across one —
    // PLAN.md §9.1: "caching it across a keystore change is how a button
    // outlives the key that justified it."
    //
    // **Always `{canPost: <bool>, reason: <string>}`, both fields always
    // present.** Every assignment goes through `capabilityFrom()`, so a reader
    // of this property does not have to know which branch produced it.
    property var capability: ({ canPost: false, reason: "" })

    // One probe reply, normalised into the one shape above.
    //
    // **This used to be two shapes and the difference was invisible at the use
    // site.** The open arm assigned `probe.value` wholesale — whatever object
    // core sent, with whatever fields — while the closed arm constructed a
    // view-owned object. So `reason` was a guaranteed string on one path and
    // possibly absent on the other, and the closed gate's `Text` needed a
    // `!== undefined` guard that read as defending the closed branch while
    // actually defending the open one. Review measured it: removing that guard
    // emitted `Unable to assign [undefined] to QString` on fourteen tests,
    // every one an OPEN-gate case, because QML evaluates the closed body's
    // bindings even while that body is invisible.
    //
    // That is CLAUDE.md's named shape — a guard restated per consumer because
    // the data structure does not hold the invariant — and it is the same
    // "establish the value where it is produced" move `voteTarget()` makes for
    // a row, applied one level up to the probe reply. A second consumer of
    // `capability.reason` now inherits the invariant instead of rediscovering
    // it, and the guard becomes unnecessary rather than merely explained.
    //
    // **Fail closed, and `=== true` is what makes that structural.** A probe
    // that could not be reached, answered with something that is not a probe
    // reply, or answered with an object carrying neither an affirmative
    // capability nor a reason must all reach the SAME state as a probe
    // reporting "not possible" — because the gate's whole point is that a box
    // the user can type into can actually submit. `canPost !== true` covers
    // every one of those without a branch per shape: absent, `undefined`,
    // `"true"`, `1` and `null` are all not-`true`.
    //
    // A probe that supplied no reason leaves `reason` empty rather than
    // inventing text: the spec forbids substituting a reason of the view's own,
    // and an empty reason is a visible gap in core's answer rather than a
    // plausible sentence covering for one.
    //
    // **The rule itself lives on `Core`**, which is where the probe reply is
    // produced, so this screen and the thread screen cannot hold copies that
    // drift apart. See `Core.qml`'s `capabilityFrom` for the fail-closed
    // argument.
    function capabilityFrom(probe) {
        return Core.capabilityFrom(probe)
    }

    // The identity report, as `who_am_i` answered it THIS render.
    //
    // **A different question from the capability probe, and the two can honestly
    // disagree** — `lib.rs:258`: "a stored identity whose keystore permissions
    // are too open is a real identity that cannot currently be used. A view with
    // only the posting probe would have to render 'you are nobody' to a user who
    // has an identity and a fixable problem."
    //
    // That disagreement is why the footer chip binds THIS and not
    // `capability.canPost`. Routing on the posting probe alone collapses "no
    // identity" and "an identity that cannot be used" into one state and offers
    // identity CREATION to the second — which for an existing identity is the
    // one irreversible wrong answer available, since `keep_identity` refuses
    // where an identity already exists and replacing one discards every identity
    // derived from it. The user who most needs to be told what is wrong is
    // instead offered a new key. (design.md D3.)
    //
    // **Held for one render, written by `reload()` and by nothing else.** Same
    // lifetime as `capability` and subject to the same rule for the same reason:
    // it reads a store the view cannot see, so a previous run's answer is not
    // evidence about this one.
    property var identity: ({ hasIdentity: false, publicKey: "", reason: "" })

    // `hasIdentity` as the design bundle names it — `examples/FeedScreen.qml`
    // gates on `property bool hasIdentity` and SPEC.md:21 makes screens 03 and
    // 05 one screen with that one flag.
    //
    // **Derived, never stored.** The bundle's shape is a bool, and this is that
    // bool — but it is a `readonly` expression over the answer `reload()` most
    // recently received rather than a settable property anything can write. So
    // the bundle's interface is honoured and nothing can hold it across a
    // keystore change: there is no setter for a stale value to be written
    // through. (design.md D4.)
    readonly property bool hasIdentity: screen.identity.hasIdentity === true

    // One `who_am_i` reply, normalised into one shape.
    //
    // `hasIdentity === true` and nothing looser, for the reason `DIdentityChip`
    // states in its own header: `"true"`, `1`, `null` and `undefined` are all
    // not-`true`, and each is truthy-or-falsy in a way that does not match what
    // it means. A chip handed any of them under a looser test renders the
    // IDENTITY PRESENT arm, claiming an identity the machine does not have, with
    // every gate green.
    function identityFrom(probe) {
        return Core.identityFrom(probe)
    }

    // Whether the closed gate's guidance is revealed. A view-local disclosure,
    // reset on nothing — it says nothing about the world, so there is nothing
    // for it to go stale against.
    property bool showFix: false

    // ---- the ordering row -----------------------------------------------
    //
    // Built from a model, as the bundle requires, so that an ordering can
    // appear or disappear without the layout changing around it.
    //
    // There is exactly ONE entry, because core implements exactly one ordering.
    // Its label is "newest first", which `feed-view` makes the label this
    // interface uses. The word to read positionally: the order is descending
    // Lamport counter, carried in each op's signed bytes, ties broken by
    // ascending op id — so the post that orders first is the LATEST ONE THE
    // FORUM'S ORDER KNOWS OF. That is a real claim and it is the claim a reader
    // wants. What it is not is a claim about instants, which is what the denial
    // below exists to say; "most recent first" would make that claim and
    // `feed-view` forbids it by name.
    //
    // **This label gives up a property the old one asserted, and that was a
    // deliberate trade.** "Same order for everyone" asserted CONVERGENCE —
    // every peer holding the same ops computes the same sequence — which
    // "newest first" does not say. The property is unchanged and is not lost:
    // the denial's third sentence carries it, which is part of why that
    // sentence must stay. What the old label lacked was any answer to the
    // question a reader actually arrives with, which is what order this is.
    //
    // Convergence would NOT be true of the feed in general, and that
    // distinction is why the label lives in the model rather than in the
    // layout. Vouching is per-reader and never published, so a vote-weighted
    // ordering would give two readers different orders over the identical op
    // set, and both would be correct. When such an ordering arrives it joins
    // this model with its own honest label; this one does not have to change.
    property var orderings: [
        { key: "convergent", label: "newest first" }
    ]
    property string ordering: "convergent"

    // ---- the viewer's own votes -----------------------------------------
    //
    // A map from the op a vote targeted to the direction published: -1 or +1.
    //
    // **Keyed by the post, so marking the wrong post is unrepresentable.** There
    // is no code path that could write one post's vote into another's slot,
    // because the key IS the post — which is the "a vote on one post does not
    // mark another" rule held by construction rather than checked at a call
    // site.
    //
    // **Session-only, and it must stay that way.** No call returns the viewer's
    // earlier votes, so after a reload this is empty and every control renders
    // neutral — which is honest about a peer that cannot read votes back. A
    // control that appeared to remember across a reload would be the view
    // inventing state core never reported. Nothing persists this and nothing
    // should.
    property var ownVotes: ({})

    // Written ONLY on a successful publish, so a refused vote leaves the control
    // showing exactly what it showed before the attempt — nothing was recorded,
    // so there is nothing to reflect.
    //
    // A NEW object rather than a mutation: QML's property-change detection does
    // not fire when an object's contents change in place, so mutating would
    // update the map and repaint nothing. That is the failure where the feature
    // looks broken in the way hardest to attribute.
    function recordVote(op, direction) {
        var next = {}
        for (var k in screen.ownVotes)
            next[k] = screen.ownVotes[k]
        next[op] = direction
        screen.ownVotes = next
    }

    // **The op a row's vote targets, or "" if the row does not name one.**
    //
    // A guard is a job, and this is the one place that judgement is made — so
    // "is it applied everywhere a row keys the vote map?" stays a question with
    // an answer. Both consumers go through it: the control's `vote` binding and
    // `voteOn`.
    //
    // **Feed rows are peer-supplied and their element shape is not validated
    // anywhere.** `reload()` checks that `items` is an array and nothing about
    // what is inside it, and today's core always sends `currentVersion`
    // (`wire.rs` maps it unconditionally) — which is exactly the "guarantee made
    // one module away" that the `items` guard a few lines below already refuses
    // to rest on. The identical argument applies one level down.
    //
    // Two concrete failures if it does not, both reproduced by review:
    //
    //   - Two rows missing the field key the map on the JavaScript value
    //     `undefined`, which stringifies to the single key `"undefined"`. They
    //     share one slot, so a vote on the first marks the second — the precise
    //     thing "a vote on one post does not mark another" forbids.
    //   - `JSON.stringify` omits a key whose value is `undefined`, so the
    //     request reaching core carries no `target` at all: the view would ask
    //     core to vote on nothing and then mark two controls on its answer.
    //
    // `design.md` claimed this was held "by construction, because the key IS the
    // post". That holds only while every row carries a distinct one, which is a
    // property of peer data rather than of the code. It is now held by
    // construction for real: a row with no usable op yields "", which renders a
    // non-interactive control and reaches no call.
    function voteTarget(rowData) {
        if (rowData === null || rowData === undefined)
            return ""
        return typeof rowData.currentVersion === "string" && rowData.currentVersion !== ""
            ? rowData.currentVersion
            : ""
    }

    function voteOn(op, direction) {
        // The guard, restated at the call rather than assumed from the binding:
        // `voteOn` is reachable from anywhere in this file, and a second caller
        // that skipped `voteTarget` would reintroduce the defect silently.
        if (typeof op !== "string" || op === "")
            return

        var reply = Core.publishVote(screen.stoaAddress, op,
                                     direction > 0 ? "up" : "down")

        // The same success test the composer applies, and for the same reason: a
        // reply the view could not interpret must not be recorded as a vote that
        // happened. `wasNew` is not consulted — a vote published twice is the
        // same vote, and both answers mean the viewer's vote is on record.
        if (reply.ok && typeof reply.value.opId === "string" && reply.value.opId !== "")
            screen.recordVote(op, direction)
    }

    Component.onCompleted: screen.reload()

    // **A different Stoa is a different read.** This screen is mounted once and
    // re-pointed at whichever Stoa the navigator chose, so without this the
    // first Stoa opened would be the only one ever read: `reload()` ran at
    // construction, when the address was still empty, and nothing asked again.
    //
    // It is `stoaAddress` that triggers rather than `stoaGenesis`, and the pair
    // is not arbitrary — the address is what identifies the Stoa, and the record
    // travels with it. A genesis arriving separately for the same address is the
    // same Stoa, so re-reading on it would issue a second identical call.
    //
    // This is also what re-probes BOTH identity answers on arrival at a feed,
    // which is the rule `reload()` carries: neither is answered from a value
    // retained across the transition.
    onStoaAddressChanged: screen.reload()

    function reload() {
        if (screen.stoaAddress === "") {
            screen.readState = "failed"
            screen.failure = "No Stoa address was given to this view."
            return
        }

        // The posting gate is re-probed with every render of the feed, never
        // read from a build flag, a configuration value, or an answer obtained
        // before this render.
        //
        // **Fail closed, and `=== true` is what makes that structural.** A probe
        // that could not be reached, answered with something that is not a probe
        // reply, or answered with an object carrying neither an affirmative
        // capability nor a reason must all reach the SAME state as a probe
        // reporting "not possible" — because the gate's whole point is that a
        // box the user can type into can actually submit. `canPost !== true`
        // covers every one of those without a branch per shape: absent,
        // `undefined`, `"true"`, `1` and `null` are all not-`true`.
        screen.capability = screen.capabilityFrom(
            Core.getCapabilities(screen.stoaAddress))

        // BOTH probes, every render, neither cached. They answer different
        // questions and the routing needs both: three outcomes — no identity, an
        // identity that cannot be used, and a user who can act — and a routing
        // that renders only two of them is silently merging a pair.
        screen.identity = screen.identityFrom(Core.whoAmI(screen.stoaAddress))

        var reply = Core.listThreads({
            stoa: screen.stoaAddress,
            genesis: screen.stoaGenesis,
            page: screen.page,
            includeHidden: screen.includeHidden
        })

        if (!reply.ok) {
            // The failure path never writes `rows`, so a previous success
            // cannot be left on screen underneath a failure banner.
            screen.readState = "failed"
            screen.failure = reply.error
            return
        }

        // A success MUST carry `items`. Core guarantees the two shapes are
        // disjoint — `error_json` never emits `items` and the page shape never
        // emits `error`, with a wire test asserting the two replies differ —
        // but "empty and unreadable must never look alike" is the whole point
        // of this screen, so it does not rest on a guarantee made one module
        // away. A reply that is neither shape is a failure here rather than an
        // `undefined` assigned to `rows`, which would render as an empty feed.
        if (reply.value.items === undefined || !Array.isArray(reply.value.items)) {
            screen.readState = "failed"
            screen.failure = "The core module answered without a list of posts, "
                           + "so what it holds for this Stoa is unknown."
            return
        }

        screen.rows = reply.value.items
        screen.hasMore = reply.value.hasMore === true
        screen.failure = ""
        screen.readState = "ok"
    }

    // The user is finished with this Stoa and wants whatever brought them here.
    //
    // **Arriving somewhere is half a transition.** This screen had no signals at
    // all, so `Main.qml` had nothing to clear `chosen` with and the first row a
    // user opened was the last screen they saw until they restarted — taking the
    // list, the share affordance and the join field with it. 103 tests passed
    // while that held, because a suite that asserts up to a transition and
    // nothing after it cannot see a one-way trip.
    //
    // A signal rather than a direct write: this screen does not know what is
    // above it, and the caller decides what "back" means. That is the symmetric
    // counterpart of the `cancelled` the join screen already has, and it is why
    // no `StackView` is needed — the navigator still holds one nullable property
    // per screen, as design.md D5 argues.
    signal closed()

    // The user wants an identity for this Stoa.
    //
    // A signal rather than a direct write, for the reason `closed()` is one:
    // this screen does not know what is above it, and the caller decides where
    // identity acquisition lives. It carries no Stoa — the navigator already
    // knows which Stoa's feed is up, and a screen telling its parent something
    // the parent set is a second source for one value.
    signal createIdentityRequested()

    // A row's thread was opened.
    //
    // **Carries the ROOT POST's op id — `id`, not `currentVersion`** — because
    // the id does not move when the post is revised, so a route carrying the
    // version would point at a thread that stops answering to it after an edit.
    // The Stoa and its record are the navigator's already, so they are not
    // repeated here.
    //
    // A signal rather than a direct write, for the same reason `closed` is one:
    // this screen does not know what is above it, and the caller decides what
    // opening a thread means.
    signal threadOpened(string rootOp)

    // The op id a row's thread is opened by, or "" where the row names none.
    //
    // **A separate judgement from `voteTarget`, reading a DIFFERENT field**, and
    // that is the whole reason it exists rather than being folded into it: a
    // vote names the version it was cast on, where a thread is named by the root
    // post's stable id. Opening a thread by `currentVersion` would point the read
    // at an identifier that moves the moment the root is edited.
    //
    // **The field is `thread`, and it is measured rather than assumed.** Both
    // pieces that reached this line independently got it wrong, in opposite
    // directions, and each was plausible:
    //
    //   - `currentVersion` is present on the row but is the version id —
    //     `feed.rs:135-140`, "different from `thread` the moment the post has
    //     been edited", and it is the field a MODERATOR acts on.
    //   - `id` is what a THREAD item carries (`wire.rs:1837`) and is not a field
    //     of a feed row at all. `wire.rs:6015-6027` pins the feed row's whole
    //     key set — `attachments, author, body, currentVersion, isHidden,
    //     isRevised, thread` — as a SET, so an added field fails it too. Reading
    //     `id` here yields `undefined` on every row core sends, which renders as
    //     no thread link anywhere in the feed.
    //
    // `FeedRow::thread` is documented as "the thread's id, which is the root
    // post's op id" and "**never changes across edits**, which is what makes it
    // the thing a reply names as its parent and the thing a view uses as a
    // stable row key" (`feed.rs:130-134`). That is exactly the identifier
    // `view-navigation` requires to travel.
    //
    // The guard is made in one place for the reason `voteTarget`'s is: a row
    // naming no thread would otherwise open one identified by the JavaScript
    // value `undefined`, which `JSON.stringify` omits entirely — so the request
    // reaching core would carry no thread at all, and the screen would render
    // core's refusal of a question the user never asked.
    function threadTarget(rowData) {
        if (rowData === null || rowData === undefined)
            return ""
        return typeof rowData.thread === "string" && rowData.thread !== ""
            ? rowData.thread
            : ""
    }

    // The moderation screen was asked for.
    //
    // A signal rather than a direct write, for the reason `closed()` is one:
    // this screen does not know what is rendering it, and a screen that reached
    // out to change what surrounds it could not be tested in isolation. The
    // Stoa is the navigator's already, so it is not repeated here.
    signal moderationRequested()

    // ---- header ---------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: 16

        // Offered unconditionally, because the state this control exists to
        // leave is exactly the state that must not withdraw it. The feed's own
        // read may have failed — that is when a user most wants out.
        FlatButton {
            objectName: "feedBackButton"
            text: "All Stoas"
            kind: "secondary"
            onClicked: screen.closed()
        }

        Identicon {
            address: screen.stoaAddress
            size: 34
            visible: screen.stoaAddress !== ""
        }

        ColumnLayout {
            spacing: 2

            Text {
                text: screen.stoaTitle
                font: DTheme.heading
                color: DTheme.ink
                textFormat: Text.PlainText   // peer-supplied: never rich text
                visible: screen.stoaTitle !== ""
            }

            // The address is on screen beside the title, never one click away.
            // A Stoa's title is moderator-chosen and freely forgeable; the
            // address is the identity.
            AddressLabel { address: screen.stoaAddress }
        }

        Item { Layout.fillWidth: true }

        // The row is a Repeater over a model, which is what SPEC.md requires:
        // an ordering must be able to disappear without the layout changing.
        //
        // There is NO click handler, and the local reason is that there is
        // nothing for one to do: with a single ordering there is nothing to
        // select, and `reload()` does not read `ordering`, so a handler that
        // set it and re-read the feed would run and change nothing. It arrives
        // with the second ordering, which is the change that gives it work.
        Repeater {
            model: screen.orderings
            delegate: Text {
                required property var modelData
                text: modelData.label
                font: DTheme.bodySmall
                color: screen.ordering === modelData.key ? DTheme.ink : DTheme.inkMuted
                textFormat: Text.PlainText
            }
        }

        // A separate view over a filter the projection already applies. It
        // needs no key and no authority — seeing what was moderated is a
        // reader's affordance, not a moderator privilege.
        Text {
            text: "SHOW HIDDEN"
            font: DTheme.label
            color: screen.includeHidden ? DTheme.accent : DTheme.inkMuted
            // Explicit even though the text is a literal today. QML's default
            // is AutoText, which SNIFFS its input and switches to rich text
            // when the string looks like markup — so an element left on the
            // default is one dynamic binding away from rendering peer markup,
            // and nothing about that change would look like it touched
            // rendering. CI now greps for this on every Text.
            textFormat: Text.PlainText

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    screen.includeHidden = !screen.includeHidden
                    screen.page = 0
                    screen.reload()
                }
            }
        }

        // The route to the moderation screen.
        //
        // **This is an affordance to a screen that publishes nothing**, and it
        // is offered anyway because the owner reversed ruling 3's screen half so
        // the screen could be SEEN — a screen no route reaches is a screen
        // nobody can look at, which is the whole of what the reversal asked for.
        // `moderation-view` contracts the reachability for that reason.
        //
        // **It is NOT gated on whether this peer may moderate**, and that is a
        // decision rather than an omission. Nothing answers the question:
        // `getModerationCapability` is designed in PLAN.md §9.1 and does not
        // exist, and `stoa-membership` states that a listed Stoa means the user
        // chose it rather than that the user governs it — the retained creator
        // key is never re-checked against the peer's current signing key. So a
        // gate here would be a guess, and a guess in this position is the one
        // that tells a user they moderate a Stoa. Offering the route claims
        // nothing; hiding it on a guess would.
        Text {
            objectName: "moderateLink"
            text: "MODERATE"
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: screen.moderationRequested()
            }
        }
    }

    // The double rule under a section heading: two hairlines, 2px apart.
    //
    // `Layout.preferredHeight` rather than `height` throughout this file: an
    // Item inside a layout has its `height` overwritten by the layout, which
    // the linter reports as undefined behaviour and CI gates on.
    //
    // Note a comment whose first word is the linter's own name is parsed as a
    // lint DIRECTIVE, and each following word becomes an unknown category —
    // eight warnings from one sentence. Do not begin a comment with that word.
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 2
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
    }

    // ---- what this ordering is, and what it is not ----------------------
    //
    // This sentence used to live in the apparatus column's ON THIS ORDERING
    // note, which was annotation explaining the design rather than interface.
    // The column is gone — `SPEC.md`: "No annotation or commentary column: the
    // caveats belong in the copy itself" — and the obligation is not, so the
    // sentence lives in the screen's own body, where a user can read it.
    //
    // It is load-bearing in a way the ordering LABEL is not. A reader meeting a
    // forum feed assumes a chronological one unless told otherwise, and no
    // label can carry its own disclaimer — so removing this would leave the
    // interface silently relying on the reader not to make the ordinary
    // assumption. `feed-view` requires the denial to be rendered rather than
    // left implicit, for exactly that reason.
    //
    // **It denies the temporal READING, and must never negate the label.** This
    // is the trap the wording fell into once and the reason the opening clause
    // now reads as it does. "Newest first" and "latest by the clock" are two
    // readings of the SAME superlative: one positional, which the ordering
    // supports, one temporal, which it does not. A denial opening "Not newest
    // first" asserts that the feed is not the thing its own label says it is,
    // which leaves a reader no way to tell which of the two strings is live —
    // an honest label and an honest denial that together contradict each other.
    // So the clause names the forbidden READING ("not latest by the clock")
    // instead of the superlative, and `feed-view` carries that as its own
    // direction.
    //
    // **Sentences two and three are not to be touched.** Two discharges the
    // hardest requirement verbatim: the reason the feed is not ordered by the
    // displayed time is that the time is THE AUTHOR'S OWN CLAIM, explicitly not
    // that no time is available. Three carries the convergence property the old
    // "same order for everyone" label asserted and the new label does not.
    //
    // Why the old wording had to go: it read "Timestamps do not reach this
    // machine yet [...] When real times arrive this label changes and nothing
    // else does." Both halves are now false. An op's signed bytes carry a
    // Lamport counter AND an author-asserted wall-clock, and the feed is ordered
    // by that counter today. The old sentence promised the reader a future in
    // which the feed becomes chronological, and that future is one core has
    // decided against rather than merely not reached: a counter is CAUSAL, not
    // temporal — it says its author had seen something, never when — and the
    // wall-clock is the author's own claim, so ordering by it would let anyone
    // reach the top of a feed by lying. A denial phrased as a limitation
    // awaiting a missing field promises the chronological feed this design has
    // refused, so it must never be softened back into "not yet".
    //
    // **No copy.json citation, deliberately** — every other string on this
    // screen carries one and this one cannot. Both strings here are the core
    // contract's rather than the bundle's: `feed-view` overrules the bundle's
    // ordering copy, and `proposal.md` records that. Citing a key here would
    // imply the bundle sanctions this wording, and it does not.
    Text {
        text: "Newest first means latest in this forum's order, not latest by the clock. Posts carry a time their author claimed, which anyone could set, so the feed is not ordered by it. The order used instead is one every peer computes identically from the posts they hold."
        font: DTheme.note
        color: DTheme.inkSoft
        wrapMode: Text.WordWrap
        lineHeight: 1.4
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- state: the store could not be read -----------------------------
    //
    // Screen 07's failed half. An accent border, the failure named, and a
    // retry. Deliberately NOT the same layout as the empty state: the two mean
    // opposite things and a reader must never have to tell them apart by
    // reading carefully.
    Rectangle {
        visible: screen.readState === "failed"
        Layout.fillWidth: true
        implicitHeight: failedBody.implicitHeight + 2 * DTheme.cardPaddingY
        color: DTheme.field
        border.width: DTheme.border
        border.color: DTheme.accent

        ColumnLayout {
            id: failedBody
            anchors.fill: parent
            anchors.margins: DTheme.cardPaddingY
            spacing: DTheme.itemGap

            Text {
                // copy.json `states.failedTitle`
                text: "The store could not be read, so nothing can be shown."
                font: DTheme.heading
                color: DTheme.accent
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                // copy.json `states.failedBody` opens with this sentence, and
                // it is the load-bearing half: it says what this ISN'T.
                text: "This is not an empty Stoa. Posts you already hold are on disk and unreadable right now."
                font: DTheme.bodySmall
                color: DTheme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            // The failure as core named it. Core's errors are written to name a
            // fix, so this is shown verbatim rather than reworded into
            // something more soothing and less actionable.
            Text {
                text: screen.failure
                font: DTheme.address
                color: DTheme.ink
                wrapMode: Text.WrapAnywhere
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            FlatButton {
                // copy.json `states.retry`
                text: "Try reading again"
                kind: "primary"
                onClicked: screen.reload()
            }
        }
    }

    // ---- state: read fine, holds nothing --------------------------------
    //
    // Screen 07's empty half. A paper border and a plain statement about THIS
    // MACHINE — never a claim about the Stoa, which this peer cannot make.
    Rectangle {
        visible: screen.readState === "ok" && screen.rows.length === 0
        Layout.fillWidth: true
        implicitHeight: emptyBody.implicitHeight + 2 * DTheme.cardPaddingY
        color: DTheme.paper
        border.width: DTheme.hairline
        border.color: DTheme.rule2

        ColumnLayout {
            id: emptyBody
            anchors.fill: parent
            anchors.margins: DTheme.cardPaddingY
            spacing: DTheme.itemGap

            Text {
                // copy.json `states.emptyTitle`
                text: "You have not received anything for this Stoa yet."
                font: DTheme.heading
                color: DTheme.ink
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                // copy.json `states.emptyBody`
                text: "The store was read without error; it holds no posts for this address. Other peers may hold posts you have not been sent. This is a fact about your copy, not about the Stoa."
                font: DTheme.bodySmall
                color: DTheme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            // The store status line. It counts what THIS MACHINE holds, which
            // is the only count this software can honestly render — there is no
            // peer count available, so the copy string's "%1 PEERS REACHABLE"
            // half is deliberately not used.
            Text {
                // Bound, not a literal reading "0". It only renders when the
                // list is empty, so the literal was accurate — and a literal
                // among bound neighbours is the kind of thing that stays
                // accurate right up until the visibility condition changes.
                text: "STORE READ OK · " + screen.rows.length + " POSTS HELD"
                font: DTheme.label
                color: DTheme.inkMuted
                textFormat: Text.PlainText
            }
        }
    }

    // ---- state: posts ---------------------------------------------------

    Repeater {
        model: screen.readState === "ok" ? screen.rows : []

        delegate: RowLayout {
            id: row
            required property var modelData
            Layout.fillWidth: true
            spacing: DTheme.itemGap

            // The arrows, with NO number. `showScore` is left at its default
            // false, which is the decision rather than an omission: no call in
            // the contract returns a score, and the control's own default would
            // otherwise print "0" — a tally core never reported.
            //
            // `vote` reads the session map. A post with no recorded vote gets
            // `undefined`, and `|| 0` renders it in the same neutral state as a
            // post before any vote — which is what an unknown vote state must
            // look like, since the view has no way to say "you have not voted".
            //
            // The gate governs this as it governs every other posting
            // affordance: publishing a vote is publishing an op.
            VoteControl {
                id: voteControl

                // "" when the row names no op to vote on — see `voteTarget`.
                // The empty string is never a key this map holds, so the vote
                // reads 0 and two such rows cannot share a slot.
                readonly property string target: screen.voteTarget(row.modelData)

                visible: screen.capability.canPost === true
                vote: voteControl.target !== ""
                    ? (screen.ownVotes[voteControl.target] || 0)
                    : 0

                // A row with no target offers no press rather than a press that
                // does nothing: the arrows go non-interactive, so the affordance
                // matches what is actually available. `voteOn` guards again
                // anyway — this is the presentation half, not the safety half.
                interactive: screen.capability.canPost === true
                             && voteControl.target !== ""

                Layout.alignment: Qt.AlignTop
                onVoted: function (direction) {
                    // Direction 0 is the control's "undo" press. Core has no
                    // vote retraction, so publishing something for it would be
                    // publishing an op that does not mean what the press meant.
                    // Nothing is sent and nothing changes — which is honest, and
                    // is the reason this is a branch rather than a mapping.
                    if (direction !== 0)
                        screen.voteOn(voteControl.target, direction)
                }
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: DTheme.itemGap

                PostHeader {
                    identityKey: row.modelData.author
                    // There is no generated name on the wire, and that is the
                    // contract rather than a gap: `generated-names` requires a
                    // name never travel on any reply, because a derived value
                    // beside the material it derives from is two values that
                    // could disagree, and a name on the wire is one a relay could
                    // strip or forge.
                    //
                    // `author` now carries the author's PUBLIC KEY, which IS that
                    // derivation's input — so deriving here is possible where it
                    // was not while the row carried an author address. Wiring
                    // `displayName` to that derivation is the next piece; until it
                    // lands the key carries the row alone, which is honest rather
                    // than incomplete.
                    generatedName: ""
                    edited: row.modelData.isRevised === true
                    Layout.fillWidth: true
                }

                // A hidden row is visible only in the show-hidden view, and it
                // must not render indistinguishably from a visible one — a
                // reader who asked to see what was hidden is owed knowing which
                // those were.
                Text {
                    visible: row.modelData.isHidden === true
                    text: "HIDDEN BY A MODERATOR · SHOWN BECAUSE YOU ASKED TO SEE HIDDEN POSTS"
                    font: DTheme.label
                    color: DTheme.accent
                    textFormat: Text.PlainText
                }

                SanitisedText {
                    value: row.modelData.body
                    Layout.fillWidth: true
                }

                Repeater {
                    model: row.modelData.attachments
                    delegate: SanitisedText {
                        required property var modelData
                        value: modelData
                        bodyFont: DTheme.address
                        bodyColor: DTheme.inkMuted
                        Layout.fillWidth: true
                    }
                }

                // ---- the row's actions, as the design lays them out ------
                //
                // "read the thread" is the affordance that opens this row's
                // thread, and **the only route into one** — which is what makes
                // the thread screen reachable at all. `view-navigation`
                // contracts the transition and what travels across it; what
                // travels is this row's `id`, the Stoa the feed was rendered
                // for, and that Stoa's founding record where the view holds one.
                //
                // It is offered on every row and NOT gated on the posting probe:
                // reading needs no identity, and gating it would withhold the
                // thread from exactly the reader the feed is otherwise happy to
                // serve.
                RowLayout {
                    Layout.fillWidth: true
                    spacing: 16

                    Text {
                        objectName: "readThreadLink"

                        // "" when the row names no op — see `threadTarget`. A
                        // row with no target offers no press rather than a press
                        // that reaches core with no thread named.
                        //
                        // **`threadTarget` and not `voteTarget`**: a thread is
                        // opened by the post's stable `id`, where a vote names
                        // the `currentVersion` it was cast on. The two guards
                        // read different fields, so they are two functions.
                        readonly property string target: screen.threadTarget(row.modelData)

                        visible: target !== ""
                        text: "read the thread"
                        font: DTheme.bodySmall
                        color: DTheme.ink
                        textFormat: Text.PlainText

                        MouseArea {
                            anchors.fill: parent
                            cursorShape: Qt.PointingHandCursor
                            onClicked: screen.threadOpened(parent.target)
                        }
                    }

                    // The reading-is-free statement the design carries under a
                    // row when there is no identity. It names what an identity
                    // is FOR rather than asserting a restriction — reading needs
                    // nothing, and the sentence says so by omission.
                    Text {
                        visible: !screen.hasIdentity
                        text: "voting and replying need an identity"
                        font: DTheme.note
                        color: DTheme.inkFaint
                        textFormat: Text.PlainText
                    }

                    Item { Layout.fillWidth: true }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: DTheme.hairline
                    color: DTheme.rule
                }
            }
        }
    }

    // ---- pagination -----------------------------------------------------
    //
    // Pagination only: no infinite scroll and no totals, as `SPEC.md` requires.
    // "Next" is offered when this peer holds another page, which is a fact about
    // this copy and not a claim about how much exists.
    //
    // That fact used to live only in the comment you are reading, which no user
    // opens, so the sentence beside the control is interface rather than
    // annotation. `tst_feed_extent_claim.qml` carries the argument for why, and
    // the note that no spec contracts it.
    //
    // The whole control is one ColumnLayout so the sentence CANNOT render without
    // the claim it qualifies, and cannot fail to render with it: there is one
    // `visible:` binding for both, and it is the binding the row already had.
    // The rule's other half — a screen asserting no extent owes nothing — is
    // therefore satisfied by construction rather than by a second guard someone
    // has to remember: no paging offered, no sentence.
    ColumnLayout {
        visible: screen.readState === "ok" && (screen.hasMore || screen.page > 0)
        Layout.fillWidth: true
        spacing: DTheme.itemGap

        RowLayout {
            Layout.fillWidth: true
            spacing: DTheme.itemGap

            FlatButton {
                text: "Previous"
                kind: "secondary"
                visible: screen.page > 0
                onClicked: { screen.page = screen.page - 1; screen.reload() }
            }

            FlatButton {
                text: "Next"
                kind: "secondary"
                visible: screen.hasMore
                onClicked: { screen.page = screen.page + 1; screen.reload() }
            }

            Item { Layout.fillWidth: true }
        }

        // The locality statement obligation 10 requires. Deliberately about the
        // PAGES rather than about the posts: "Next" is the claim being qualified,
        // so the sentence has to deny what "Next" would otherwise be read to say.
        Text {
            text: "Pages are what this machine holds. \"Next\" means another page has reached your copy — not that the Stoa has more, which no peer can know."
            font: DTheme.note
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.4
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // ---- posting gate: open ---------------------------------------------
    //
    // The composer exists ONLY on this branch, and the two branches are driven
    // by one expression against its complement — so no probe answer can render
    // both, and none can render neither.
    //
    // The probe is re-run by `reload()` on every render and never cached across
    // one: a gate decided from a stale answer is a button that outlives the key
    // that justified it.
    ColumnLayout {
        visible: screen.capability.canPost === true
        Layout.fillWidth: true
        spacing: DTheme.itemGap

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }

        Text {
            text: "Post to this Stoa"
            font: DTheme.heading
            color: DTheme.ink
            textFormat: Text.PlainText
        }

        DComposer {
            id: composer
            kind: "post"
            stoaAddress: screen.stoaAddress
            Layout.fillWidth: true

            // **No optimistic row.** The feed is re-read and shows what core
            // reports; nothing composed by the view is inserted. A row the view
            // built would carry the sanitiser's counts and a revision flag it
            // would have to invent, and inventing them is how a view comes to
            // render something core never said.
            //
            // If the re-read does not show the post — which happens for a reply,
            // since a reply is not a thread head and has no row in a feed of
            // thread heads — the success message stays. It is still correct
            // about what occurred, which is why it never says "your post is now
            // below".
            onPublished: screen.reload()
        }
    }

    // ---- posting gate: shut ----------------------------------------------
    //
    // There is no disabled composer here and no text field behind the gate. A
    // box the user could type into and not send would lose what they wrote.
    ColumnLayout {
        visible: screen.capability.canPost !== true
        Layout.fillWidth: true
        spacing: 6

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }

        // **NOT copy.json's `compose.blockedTitle`.** That string is "You cannot
        // reply in this Stoa yet", and it is wrong twice over: the gate
        // withholds posting AND voting as well as replying, so a heading naming
        // only replying misdescribes what is blocked; and the bundle pairs it
        // with a reason ending "the reply box comes back when a reply would
        // actually send", which promises a delivery outcome nothing in this
        // system checks. The probe establishes whether a publish would be
        // accepted and stored LOCALLY. Publishing and delivering are two events
        // at two times, and the second is not wired.
        Text {
            text: "You cannot post, reply or vote in this Stoa yet."
            font: DTheme.heading
            color: DTheme.ink
            wrapMode: Text.WordWrap
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        // The reason comes from core and already names a fix — a documented
        // obligation on the keystore errors, with a test.
        //
        // **Verbatim, and nothing here branches on its text.** The reason's
        // wording is deliberately not part of the contract, so a view selecting
        // what it shows by matching on that prose would turn every improvement
        // to it into a silent breaking change. The bundle's own `noKeystore` and
        // `badPermissions` strings are not used at all: a view-supplied reason is
        // one nobody checked against what the probe can actually establish, and
        // both of those make the delivery promise described above.
        Text {
            // No `!== undefined` guard, and its absence is the point:
            // `capabilityFrom()` makes `reason` a string on every path, so
            // there is nothing here to defend against. The guard that used to
            // stand here read as protecting this closed branch and actually
            // protected the open one — a reader following the comment would
            // have concluded it was redundant and deleted it for the wrong
            // reason. The invariant now lives where the value is made.
            text: screen.capability.reason
            font: DTheme.bodySmall
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        // **Why there is no box, stated where the gate is rendered.**
        //
        // copy.json `compose.apparatus`, verbatim — it survived the audit that
        // dropped two other compose strings because it is a statement about this
        // interface's own design and true of it: there IS no disabled composer
        // here.
        //
        // It lives in the gate's body rather than in the apparatus column, and
        // the move is the requirement rather than a layout preference. The
        // column is annotation explaining the design to a reader of the design;
        // it reached the shipped interface by mistake and is being removed. An
        // obligation expressed as "this text appears in that column" disappears
        // with the column — silently, while still being required — and a test
        // asserting the column's string fails for the wrong reason when it goes.
        // So the statement is owed to the reader facing the gate, and it is here.
        Text {
            text: "There is no disabled composer here. A box you could type into and not send would lose what you wrote."
            font: DTheme.bodySmall
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        // The route to acting on it, so the reader gets a reason AND somewhere
        // to go rather than the reason alone.
        //
        // It reveals guidance rather than navigating: there is nowhere to
        // navigate to, and a button that changed nothing would read as a working
        // control — which is the reason the ordering row above deliberately has
        // no click handler. This one has something to do.
        FlatButton {
            // copy.json `compose.fix` — verbatim, and it carries no guarantee
            // claim, which is why this one survived the audit that dropped two.
            text: "Show me how to fix it"
            kind: "secondary"
            visible: !screen.showFix
            onClicked: screen.showFix = true
        }

        // **Guidance about the shape of the problem, never a second reason.**
        // Core's reason above is the one that names the fix; this says where to
        // act and what the gate is actually about, and it deliberately makes no
        // claim about what happens after — in particular not that anything will
        // then send.
        Text {
            visible: screen.showFix
            text: "This is settled on your machine, not by anyone else. The gate above reports what "
                + "the core module found when it looked for a usable key just now, and the line "
                + "before it is that report word for word. Resolve what it names and reopen this "
                + "Stoa; the gate is checked again every time this feed is read."
            font: DTheme.bodySmall
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // ---- the footer: who you are acting as -------------------------------
    //
    // The design's footer is "pagination left, identity chip centred, the three
    // lamps right" (SPEC.md:152). Pagination is above, where the rows it pages
    // are; the lamps are shared chrome and live in `Main.qml`, because they must
    // appear on EVERY main-area screen and this is one screen. What is here is
    // the half that is Stoa-scoped: the chip answers "who am I posting as in
    // THIS Stoa", and there is no such answer without a Stoa.
    //
    // **Bound to the identity report, NOT to `capability.canPost`.** The chip's
    // own header says to bind `canPost === true`, which was right while the chip
    // was the posting gate's indicator and is wrong for this placement: binding
    // it here shows the identity-choosing affordance to a user who HAS one and cannot
    // currently use it, and routing them to creation is irreversible. The header
    // is updated to state both bindings and which placement takes which.
    // (design.md D3.)
    RowLayout {
        Layout.fillWidth: true
        spacing: 16

        Item { Layout.fillWidth: true }

        DIdentityChip {
            objectName: "identityChip"
            hasIdentity: screen.hasIdentity
            // No generated name is derivable in the sandbox — the QML engine
            // holds no wordlists, and `generated-names` records the derivation
            // as reachable by no caller until an entry point exists. `""` is
            // what the existing consumer passes and is correct until it does.
            generatedName: ""
            identityAddress: screen.identity.publicKey

            onCreateRequested: screen.createIdentityRequested()
        }

        Item { Layout.fillWidth: true }
    }
}
