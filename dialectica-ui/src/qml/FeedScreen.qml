import QtQuick
import QtQuick.Layouts

// Screen 04 (feed) and screen 07 (empty vs unreadable), which are one screen
// because they are three states of the same read.
//
// The three states are mutually exclusive by construction — `state` is computed
// from one variable, so no combination of flags can render two at once. That is
// the point: UI-BRIEF obligation 5 says a storage failure must never render as
// an empty feed, and two independent booleans is how that eventually happens.
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
    property var capability: ({ canPost: false, reason: "" })

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
    // Its label is "same order for everyone", and that label is true OF THIS
    // ORDERING specifically: with no Lamport timestamp reaching this machine,
    // the order falls back to ascending op id, which every peer computes
    // identically from ops they all hold.
    //
    // It would NOT be true of the feed in general, and that distinction is why
    // the label lives in the model rather than in the layout. Vouching is
    // per-reader and never published, so a vote-weighted ordering would give two
    // readers different orders over the identical op set, and both would be
    // correct. When such an ordering arrives it joins this model with its own
    // honest label; this one does not have to change.
    property var orderings: [
        { key: "convergent", label: "same order for everyone" }
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
        var probe = Core.getCapabilities(screen.stoaAddress)
        screen.capability = (probe.ok && probe.value.canPost === true)
            ? probe.value
            : ({
                canPost: false,
                // A reason from whichever source has one. A probe that supplied
                // no reason leaves this empty rather than inventing text: the
                // spec forbids substituting a reason of the view's own, and an
                // empty reason is a visible gap in core's answer rather than a
                // plausible sentence covering for one.
                reason: probe.ok
                    ? (typeof probe.value.reason === "string" ? probe.value.reason : "")
                    : probe.error
            })

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

    // ---- header ---------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: 16

        Identicon {
            address: screen.stoaAddress
            size: 34
            visible: screen.stoaAddress !== ""
        }

        ColumnLayout {
            spacing: 2

            Text {
                text: screen.stoaTitle
                font: Theme.heading
                color: Theme.ink
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
        // There is deliberately NO click handler. With one ordering there is
        // nothing to select, and `reload()` does not read `ordering` — so a
        // handler that set it and re-read the feed was code that ran and
        // changed nothing, which is worse than absent: it reads as a working
        // control. It arrives with the second ordering, which is the change
        // that gives it something to do.
        Repeater {
            model: screen.orderings
            delegate: Text {
                required property var modelData
                text: modelData.label
                font: Theme.bodySmall
                color: screen.ordering === modelData.key ? Theme.ink : Theme.inkMuted
                textFormat: Text.PlainText
            }
        }

        // A separate view over a filter the projection already applies. It
        // needs no key and no authority — seeing what was moderated is a
        // reader's affordance, not a moderator privilege.
        Text {
            text: "SHOW HIDDEN"
            font: Theme.label
            color: screen.includeHidden ? Theme.accent : Theme.inkMuted
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
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }
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
        implicitHeight: failedBody.implicitHeight + 2 * Theme.cardPaddingY
        color: Theme.field
        border.width: Theme.border
        border.color: Theme.accent

        ColumnLayout {
            id: failedBody
            anchors.fill: parent
            anchors.margins: Theme.cardPaddingY
            spacing: Theme.itemGap

            Text {
                // copy.json `states.failedTitle`
                text: "The store could not be read, so nothing can be shown."
                font: Theme.heading
                color: Theme.accent
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                // copy.json `states.failedBody` opens with this sentence, and
                // it is the load-bearing half: it says what this ISN'T.
                text: "This is not an empty Stoa. Posts you already hold are on disk and unreadable right now."
                font: Theme.bodySmall
                color: Theme.inkSoft
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
                font: Theme.address
                color: Theme.ink
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
        implicitHeight: emptyBody.implicitHeight + 2 * Theme.cardPaddingY
        color: Theme.paper
        border.width: Theme.hairline
        border.color: Theme.rule2

        ColumnLayout {
            id: emptyBody
            anchors.fill: parent
            anchors.margins: Theme.cardPaddingY
            spacing: Theme.itemGap

            Text {
                // copy.json `states.emptyTitle`
                text: "You have not received anything for this Stoa yet."
                font: Theme.heading
                color: Theme.ink
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                // copy.json `states.emptyBody`
                text: "The store was read without error; it holds no posts for this address. Other peers may hold posts you have not been sent. This is a fact about your copy, not about the Stoa."
                font: Theme.bodySmall
                color: Theme.inkSoft
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
                font: Theme.label
                color: Theme.inkMuted
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
            spacing: Theme.itemGap

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
                spacing: Theme.itemGap

                PostHeader {
                    identityAddress: row.modelData.author
                    // There is no generated name on the wire: core sends an
                    // address, and the name is the address's shadow. Until the
                    // name derivation lands the address carries the row alone,
                    // which is the honest half of the pair.
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
                    font: Theme.label
                    color: Theme.accent
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
                        bodyFont: Theme.address
                        bodyColor: Theme.inkMuted
                        Layout.fillWidth: true
                    }
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: Theme.hairline
                    color: Theme.rule
                }
            }
        }
    }

    // ---- pagination -----------------------------------------------------
    //
    // Pagination only: no infinite scroll and no totals. "Next" is offered when
    // this peer holds another page, which is a fact about this copy and not a
    // claim about how much exists.
    RowLayout {
        visible: screen.readState === "ok" && (screen.hasMore || screen.page > 0)
        Layout.fillWidth: true
        spacing: Theme.itemGap

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
        spacing: Theme.itemGap

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }

        Text {
            text: "Post to this Stoa"
            font: Theme.heading
            color: Theme.ink
            textFormat: Text.PlainText
        }

        Composer {
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

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }

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
            font: Theme.heading
            color: Theme.ink
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
            text: screen.capability.reason !== undefined ? screen.capability.reason : ""
            font: Theme.bodySmall
            color: Theme.inkSoft
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
            font: Theme.bodySmall
            color: Theme.inkSoft
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
            font: Theme.bodySmall
            color: Theme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // **Nothing load-bearing lives in this column.**
    //
    // It is annotation explaining the design to a reader of the design, it
    // reached the shipped interface by mistake, and it is being removed. Two
    // obligations were attached to it and both have moved into the bodies they
    // qualify: the missing-box statement is now in the closed gate's own body,
    // and the delivery denial is in `PublishOutcome` beside the success it
    // qualifies. A requirement discharged from here disappears when the column
    // does — silently, while still being required — so anything a reader is
    // OWED belongs where they will meet it, not here.
    apparatus: [
        // What a published post is, and is not. It sits beside the composer
        // because the success message's claim is deliberately weaker than a
        // reader expects, and the weakness is the honest part.
        MarginNote {
            label: "ON PUBLISHING"
            body: "Publishing writes the post to this machine's log and signs it. Whether any other peer receives it happens later and is not reported back here, so nothing in this interface will tell you a post was delivered."
            visible: screen.capability.canPost === true
        },
        // Why the arrows carry no number. Without this the absence reads as a
        // count that failed to load.
        MarginNote {
            label: "ON THE ARROWS"
            body: "No number is shown beside them because nothing here counts votes. The arrows record yours on this machine for as long as this view is open; a zero would be a claim that nobody voted, which is not something this peer can know."
            visible: screen.capability.canPost === true
        },
        MarginNote {
            label: "ON THIS ORDERING"
            // copy.json `feed.orderingNote`
            body: "Not newest first. Timestamps do not reach this machine yet, so posts are ordered by a rule every peer computes identically. When real times arrive this label changes and nothing else does."
        },
        MarginNote {
            label: "ON WHAT YOU HOLD"
            caveat: false
            body: "Every number here counts what this machine has received. No peer can see the whole of a Stoa, so there is no total to show."
        },
        MarginNote {
            label: "ON THE MARK"
            caveat: false
            // The identicon is a second forgeable channel, and saying so is
            // part of not letting it stand in for the address.
            body: "The hatched shape is drawn from the address and is identical on every peer. It is a shortcut for recognition, never a proof of anything — which is why the address is printed beside it."
        }
    ]
}
