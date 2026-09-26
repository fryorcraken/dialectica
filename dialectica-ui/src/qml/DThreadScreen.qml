import QtQuick
import QtQuick.Layouts

// Screen 06 — a thread: the root, its replies nested by the parent chain, and
// one composer at the foot.
//
// The visual structure is the design bundle's screen 06
// (`reference/Dialectica App.dc.html`, the `06 Thread` block): a vote column
// 40px wide beside each post, replies indented in 34px steps with a hairline
// rule down the left of each indented block, and the composer separated from
// the thread by a 3px double rule.
//
// **What this screen must not do is re-parent an item whose parent it cannot
// find.** `thread.rs` derives thread membership from the parent chain precisely
// so an authentically-signed post cannot inject itself into a conversation by
// naming it — and a post whose chain cannot be completed is returned under no
// thread rather than placed by its claim. A view that rendered such an item as a
// direct reply to the root would hand the attacker, at the last step, the
// placement core refused. See `resolveDepth` and design.md D1.
ScreenFrame {
    id: screen

    // ---- what was navigated here with -----------------------------------
    //
    // All three supplied by whatever opened this screen. There is no default
    // holding a usable value: a second source for the thread this screen exists
    // to render is a build shipping a hardcoded thread.
    property string stoaAddress: ""
    property string stoaGenesis: ""
    property string stoaTitle: ""

    // The ROOT POST's op id — `id`, not `currentVersion`. The root's id does not
    // move when the post is revised, so a route that carried the version would
    // point at a thread that no longer answers to it after an edit.
    property string threadId: ""

    // ---- read state -----------------------------------------------------
    //
    // One enum-ish string rather than several booleans, exactly as FeedScreen
    // does it and for the same reason: an unreadable thread and a thread with no
    // replies must never be able to render as each other, and two independent
    // flags is how that eventually happens.
    //
    //   "unread"  nothing asked for yet
    //   "ok"      core answered; `items` is what it holds
    //   "failed"  the read was refused; `failure` says why, in core's words
    property string readState: "unread"
    property var items: []
    property string failure: ""
    property bool hasMore: false
    property bool includeHidden: false

    // The posting gate, probed on every read and never cached across one.
    // `composer-view` owns this rule; the shape is FeedScreen's `capabilityFrom`
    // contract — both fields always present, so a consumer never branches on
    // which arm produced it.
    property var capability: ({ canPost: false, reason: "" })

    function capabilityFrom(probe) {
        return Core.capabilityFrom(probe)
    }

    // The identity report, as `who_am_i` answered it THIS read.
    //
    // **A DIFFERENT question from the posting probe, and the two can honestly
    // disagree** (`lib.rs:258`): a stored identity whose keystore permissions
    // are too open is a real identity that cannot currently be used. The
    // `REPLYING AS` line names who a reply would be signed by, which only this
    // answers — `capability.canPost` says whether one could be sent, not by
    // whom.
    //
    // **Held for one render, written by `reload()` and by nothing else**, the
    // same lifetime `capability` has and for the same reason: it reads a store
    // the view cannot see, so a previous run's answer is not evidence about
    // this one.
    property var identity: ({ hasIdentity: false, publicKey: "", reason: "" })

    // One `who_am_i` reply, normalised. `hasIdentity === true` and nothing
    // looser: `"true"`, `1` and `null` are each truthy-or-falsy in a way that
    // does not match what they mean, and a line handed one of them under a
    // looser test names an identity the machine may not have.
    //
    // **The rule lives on `Core`**, shared with `FeedScreen.qml` rather than
    // copied into it — see `Core.qml`'s `identityFrom`.
    function identityFrom(probe) {
        return Core.identityFrom(probe)
    }

    signal closed()

    // ---- nesting --------------------------------------------------------
    //
    // How far a reply may be indented before the indent stops growing. The bound
    // clamps PIXELS and nothing else: it never changes a computed depth, never
    // changes which item is reported as a parent, never omits an item and never
    // touches the sequence. A chain deeper than this renders at this indent and
    // stays complete and in order.
    //
    // Two numbers doing two jobs rather than one doing both — a walk that
    // truncated would make a deep thread stop being readable at a line nobody
    // could derive, which is the failure `thread.rs` names when it chooses a
    // visited set over a depth limit.
    property int maxIndentDepth: 6

    // The design's indent step: 34px per level, matching screen 06's
    // `padding-left:34px` then `68px`.
    property int indentStep: 34

    // An item's op id, or "" where it names none.
    //
    // A guard is a job, and this is the one place the judgement is made, so
    // "is it applied everywhere?" stays a question with an answer. Peer-supplied
    // items are not validated element-by-element anywhere on this path, and an
    // item missing `id` would otherwise key the parent map on the JavaScript
    // value `undefined` — which stringifies to the single key "undefined", so
    // two such items would share one slot and each would resolve as the other's
    // parent.
    function itemId(item) {
        if (item === null || item === undefined)
            return ""
        return typeof item.id === "string" && item.id !== "" ? item.id : ""
    }

    // ---- the three things an item's `parent` field can be ------------------
    //
    // **`parent` is OMITTED on the root** rather than sent as null (`wire.rs`),
    // so "reports no parent" and "reports a parent we cannot use" are different
    // facts here, and the whole of `resolveDepth` depends on keeping them apart.
    //
    // They were kept apart by CONVENTION before — both answered `""`, and
    // `resolveDepth` read that `""` as "this is the root". Anything that was not
    // a non-empty string therefore rendered at the root's own depth with no
    // notice: `parent: null`, a number, an object. That is the placement
    // `thread.rs` refuses, handed back at the last step, and mutating the `-1`
    // guards below does not reveal it because this route never reaches them.
    //
    // The fix is a shape rather than a fourth guard. `parentOf` now answers with
    // a KIND, and there is no longer a value the two cases share, so a caller
    // cannot read one as the other by omission — it has to name which kind it
    // means. See design.md D13.
    readonly property string parentRoot: "root"           // no `parent` field
    readonly property string parentNamed: "named"         // a usable op id
    readonly property string parentUnusable: "unusable"   // present, not usable

    // `{ kind, id }`. `id` is meaningful only when `kind === parentNamed`; it is
    // "" for the other two so that a caller reaching for it anyway gets a value
    // that resolves nothing rather than one that resolves something wrong.
    function parentOf(item) {
        if (item === null || item === undefined)
            return { kind: screen.parentUnusable, id: "" }
        if (!("parent" in Object(item)) || item.parent === undefined)
            return { kind: screen.parentRoot, id: "" }
        if (typeof item.parent === "string" && item.parent !== "")
            return { kind: screen.parentNamed, id: item.parent }
        return { kind: screen.parentUnusable, id: "" }
    }

    // A map from op id to the item holding it, for the page in hand.
    //
    // Rebuilt from `items` rather than maintained alongside it, so it cannot
    // describe a page that is no longer on screen.
    readonly property var itemsById: {
        var byId = ({})
        for (var i = 0; i < screen.items.length; i++) {
            var id = screen.itemId(screen.items[i])
            if (id !== "")
                byId[id] = screen.items[i]
        }
        return byId
    }

    // **The depth of an item, or -1 where the view cannot establish one.**
    //
    // -1 is a real answer and not a failure: it means "this item answers
    // something that is not on this page", which is a state the design renders
    // rather than hides. It must never be folded into depth 0 — see the file
    // header and design.md D1.
    //
    // The walk carries a visited set, which is what makes it terminate over
    // peer-supplied parent references: an item naming itself, and a cycle of any
    // length, both reach an op the walk has already seen. `thread.rs` makes the
    // same choice for the same reason, and records that a module which stops
    // answering has to be restarted.
    //
    // A cycle yields -1, honestly: an item inside a parent cycle has no
    // establishable depth. Treating it as depth 0 would re-create the defect
    // above by a second route.
    function resolveDepth(item) {
        var parent = screen.parentOf(item)

        // NO `parent` FIELD AT ALL: this is the ROOT, which is not a
        // missing-parent case. It is the outermost depth. Only this kind takes
        // this branch — a present-but-unusable `parent` falls through to -1
        // below, which is the whole point of the kinds.
        if (parent.kind === screen.parentRoot)
            return 0

        // Present but not a usable op id. The item claims to answer something,
        // and the claim names nothing we can follow, so no depth can be
        // established. Same answer as a parent that is off the page.
        if (parent.kind !== screen.parentNamed)
            return -1

        var visited = ({})
        var id = screen.itemId(item)
        if (id !== "")
            visited[id] = true

        var depth = 0
        var cursor = parent.id

        while (true) {
            if (visited[cursor] === true)
                return -1               // a cycle: no depth can be established

            var next = screen.itemsById[cursor]
            if (next === undefined)
                return -1               // the parent is not on this page

            visited[cursor] = true
            depth += 1

            var nextParent = screen.parentOf(next)
            if (nextParent.kind === screen.parentRoot)
                return depth            // reached the root: the chain completes
            if (nextParent.kind !== screen.parentNamed)
                return -1               // the chain breaks on an unusable claim

            cursor = nextParent.id
        }
    }

    // The indent in pixels for a computed depth. An unresolvable item (-1)
    // renders at the outermost indent, where its own notice carries the meaning
    // rather than a position implying a parent it does not have.
    function indentFor(depth) {
        if (depth <= 0)
            return 0
        return Math.min(depth, screen.maxIndentDepth) * screen.indentStep
    }

    // ---- reading --------------------------------------------------------

    Component.onCompleted: screen.reload()

    // Re-read when the screen is pointed at a different thread, so reopening
    // from a feed row renders that row's thread rather than the previous one.
    //
    // **The Stoa and its record must already be here when the id arrives**,
    // and that is the caller's to guarantee: `Main.qml` withholds the id until
    // the address and the record it binds have landed. It used to hold only by
    // the order QML happened to update the bindings in — the order `FeedScreen`
    // lost, which was issue #152 (the `e2e-created-stoa-flow` change's design.md
    // D8).
    onThreadIdChanged: screen.reload()

    function reload() {
        if (screen.stoaAddress === "" || screen.threadId === "") {
            screen.readState = "failed"
            screen.failure = "No thread was given to this view."
            screen.items = []
            return
        }

        screen.capability = screen.capabilityFrom(
            Core.getCapabilities(screen.stoaAddress))

        // BOTH probes, every read, neither cached. They answer different
        // questions — whether a reply could be sent, and who it would be signed
        // by — and the `REPLYING AS` line needs the second.
        screen.identity = screen.identityFrom(Core.whoAmI(screen.stoaAddress))

        var reply = Core.readThread({
            stoa: screen.stoaAddress,
            genesis: screen.stoaGenesis,
            thread: screen.threadId,
            includeHidden: screen.includeHidden
        })

        if (!reply.ok) {
            // The failure path never writes `items`, so a previous success
            // cannot be left on screen underneath a failure banner.
            screen.readState = "failed"
            screen.failure = reply.error
            screen.items = []
            return
        }

        // A success MUST carry `items`. Core keeps the two shapes disjoint — an
        // error reply carries no `items` key at all — but "refused and empty must
        // never look alike" is the distinction this screen exists to make, so it
        // does not rest on a guarantee made one module away. A reply that is
        // neither shape reaches the refused state rather than assigning
        // `undefined` to `items`, which would render as a thread with no replies.
        if (reply.value.items === undefined || !Array.isArray(reply.value.items)) {
            screen.readState = "failed"
            screen.failure = "The core module answered without a list of posts, "
                           + "so what it holds for this thread is unknown."
            screen.items = []
            return
        }

        screen.items = reply.value.items
        screen.hasMore = reply.value.hasMore === true
        screen.failure = ""
        screen.readState = "ok"
    }

    // ---- header ---------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: DTheme.blockGap

        // Offered unconditionally, and that is the requirement rather than a
        // layout choice: the state a user most needs to leave is a refused read,
        // so this control must not be withdrawn by any state the screen can be
        // in.
        FlatButton {
            objectName: "threadBackButton"
            text: "Back to the feed"
            kind: "secondary"
            onClicked: screen.closed()
        }

        Identicon {
            address: screen.stoaAddress
            size: DTheme.markInFeed
            visible: screen.stoaAddress !== ""
        }

        ColumnLayout {
            spacing: 2

            Text {
                text: screen.stoaTitle
                font: DTheme.bodySmall
                color: DTheme.inkMuted
                textFormat: Text.PlainText     // peer-supplied: never rich text
                visible: screen.stoaTitle !== ""
            }

            AddressLabel { address: screen.stoaAddress }
        }

        Item { Layout.fillWidth: true }

        // The same reader's affordance the feed carries. Seeing what was
        // moderated needs no key and no authority.
        Text {
            text: "SHOW HIDDEN"
            font: DTheme.label
            color: screen.includeHidden ? DTheme.accent : DTheme.inkMuted
            textFormat: Text.PlainText

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: {
                    screen.includeHidden = !screen.includeHidden
                    screen.reload()
                }
            }
        }
    }

    // ---- state: the thread could not be read ----------------------------
    //
    // Core's message, verbatim, inside an accent border. Deliberately NOT the
    // same shape as a thread holding no replies: the two mean opposite things,
    // and a reader must never have to tell them apart by reading carefully.
    Rectangle {
        objectName: "threadFailed"
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
                text: "This thread could not be read."
                font: DTheme.heading
                color: DTheme.accent
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            // The refusal as core named it — shown as supplied, never reworded
            // and never matched on. Core contracts three distinguishable
            // refusals that reach a view only as differing prose inside one
            // error shape, so a view selecting behaviour by that text would turn
            // every improvement to the wording into a silent breaking change.
            Text {
                objectName: "threadFailureMessage"
                text: screen.failure
                font: DTheme.address
                color: DTheme.ink
                wrapMode: Text.WrapAnywhere
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            // No claim that this is permanent and none that the user caused it.
            // For the commonest refusal — an op this peer has not received — the
            // thread may still arrive.
            Text {
                text: "This is a fact about your copy. The thread may not have reached this machine yet."
                font: DTheme.note
                color: DTheme.inkSoft
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            FlatButton {
                text: "Try reading again"
                kind: "primary"
                onClicked: screen.reload()
            }
        }
    }

    // ---- state: the thread ----------------------------------------------

    Repeater {
        objectName: "threadItems"
        model: screen.readState === "ok" ? screen.items : []

        delegate: RowLayout {
            id: post
            required property var modelData

            // Computed once per row rather than at each use, so the notice and
            // the indent cannot disagree about whether the parent was found.
            readonly property int depth: screen.resolveDepth(post.modelData)
            readonly property bool parentUnresolved: post.depth < 0

            Layout.fillWidth: true
            Layout.leftMargin: screen.indentFor(post.depth)
            spacing: DTheme.blockGap

            // The design's left rule down each indented block. Absent at the
            // root, where there is nothing to indent from.
            Rectangle {
                Layout.fillHeight: true
                Layout.preferredWidth: DTheme.hairline
                color: DTheme.rule
                visible: post.depth !== 0
            }

            // The vote column: 40px, arrows only, NO number. `showScore` stays
            // false — no call in the contract returns a score for a thread item,
            // and the control's own default would otherwise print a tally core
            // never reported.
            VoteControl {
                showScore: false
                interactive: false
                Layout.alignment: Qt.AlignTop
                Layout.preferredWidth: 40
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: DTheme.itemGap

                PostHeader {
                    identityKey: typeof post.modelData.author === "string"
                        ? post.modelData.author : ""
                    // No generated name travels on any reply — it is a pure
                    // function of the key beside it, and a name on the wire is
                    // one a relay could strip or forge. Deriving it is the next
                    // piece; until then the key carries the row alone.
                    generatedName: ""
                    edited: post.modelData.isRevised === true
                    markSize: post.depth === 0 ? DTheme.markInFeed : 17
                    Layout.fillWidth: true
                }

                // **The item answers something not on this page.**
                //
                // Rendered rather than hidden, and rendered INSTEAD of a
                // position implying the root. This is the security property:
                // core refused to place this item by its claim, and so does the
                // view.
                Text {
                    objectName: "unresolvedParentNotice"
                    visible: post.parentUnresolved
                    text: "IN REPLY TO A POST NOT SHOWN"
                    font: DTheme.label
                    color: DTheme.accent
                    textFormat: Text.PlainText
                }

                // A hidden item, shown because the reader asked to see hidden
                // content. It must not render indistinguishably from a visible
                // one.
                //
                // **`moderation.state`, not `isHidden`.** A thread item carries
                // no `isHidden` field at all — the feed does, and copying a feed
                // delegate here is exactly how this renders blank. design.md D7
                // carries the divergence.
                Text {
                    visible: post.modelData.moderation !== undefined
                             && post.modelData.moderation.state === "hidden"
                    text: "HIDDEN BY A MODERATOR"
                    font: DTheme.label
                    color: DTheme.accent
                    textFormat: Text.PlainText
                }

                // A moderation that deliberately restored this post — a state
                // the feed's boolean cannot express at all.
                Text {
                    visible: post.modelData.moderation !== undefined
                             && post.modelData.moderation.state === "unhidden"
                    text: "RESTORED BY A MODERATOR"
                    font: DTheme.label
                    color: DTheme.inkMuted
                    textFormat: Text.PlainText
                }

                // **A withheld body is not an empty one.**
                //
                // `body` is OMITTED where the content is withheld, and present
                // as `{"text":""}` where an author cleared it. The distinction
                // survives the whole core path and is destroyed here if both
                // render blank — so the test is on the FIELD's presence, never
                // on its contents. A `body.text || ""` would collapse them.
                Text {
                    objectName: "withheldNotice"
                    visible: post.modelData.body === undefined
                    text: "CONTENT WITHHELD — THIS POST IS HIDDEN IN THIS STOA"
                    font: DTheme.label
                    color: DTheme.accent
                    textFormat: Text.PlainText
                }

                // Present-body case. Core sanitised it and reported what that
                // found; this renders the pair and applies no second
                // transformation of its own. A marked character stays in the
                // text — replacing it would turn a deceptive string into a
                // plausible one.
                SanitisedText {
                    visible: post.modelData.body !== undefined
                    value: post.modelData.body !== undefined
                        ? post.modelData.body
                        : ({ text: "", removed: 0, marked: 0 })
                    bodyFont: post.depth === 0 ? DTheme.postBody : DTheme.body
                    Layout.fillWidth: true
                }

                Repeater {
                    model: post.modelData.attachments !== undefined
                        ? post.modelData.attachments : []
                    delegate: SanitisedText {
                        required property var modelData
                        value: modelData
                        bodyFont: DTheme.address
                        bodyColor: DTheme.inkMuted
                        Layout.fillWidth: true
                    }
                }

                // The per-post affordance row, as the design shows it: the
                // `edited` state is carried by PostHeader above, and beside it
                // the earlier-versions control.
                RowLayout {
                    spacing: DTheme.blockGap
                    visible: post.modelData.isRevised === true

                    // **INERT, and it reaches no call.**
                    //
                    // No method on the module surface reads a prior version:
                    // superseded versions stay in the op log and nothing exposes
                    // them. So this is present and offers no action, under the
                    // inertness convention `composer-view` already fixed —
                    // present rather than absent, and never acting on a
                    // substituted value.
                    //
                    // Static text rather than a button with an empty handler: a
                    // control that depresses and does nothing reads as broken,
                    // where a statement reads as a statement. There is no
                    // handler here for a call to be reached from.
                    Text {
                        objectName: "earlierVersionsInert"
                        text: "read the earlier versions"
                        font: DTheme.bodySmall
                        color: DTheme.inkMuted
                        textFormat: Text.PlainText
                    }

                    Text {
                        text: "NOT YET AVAILABLE"
                        font: DTheme.label
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

    // ---- state: read fine, no replies -----------------------------------
    //
    // A thread that holds only its root is a SUCCESS, and says so. The failed
    // state above is a different shape entirely, which is the point: a reader
    // shown an empty thread when the thread was never received concludes a post
    // vanished.
    Text {
        objectName: "noRepliesNotice"
        visible: screen.readState === "ok" && screen.items.length === 1
        text: "No replies have reached this machine for this thread."
        font: DTheme.note
        color: DTheme.inkSoft
        wrapMode: Text.WordWrap
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // The locality claim, owed wherever this peer's holdings could be read as
    // the Stoa's. Rendered only where there is more than one page, so a screen
    // asserting no extent owes nothing.
    Text {
        visible: screen.readState === "ok" && screen.hasMore
        text: "More replies are held than are shown here. What this machine holds is not what the Stoa has, which no peer can know."
        font: DTheme.note
        color: DTheme.inkSoft
        wrapMode: Text.WordWrap
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- the composer ---------------------------------------------------
    //
    // The design's 3px double rule separating the thread from the reply box.
    // Two hairlines with a gap, which is how a double rule is drawn here.
    ColumnLayout {
        visible: screen.readState === "ok"
        Layout.fillWidth: true
        spacing: 2

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
    }

    // ---- posting gate: open ---------------------------------------------
    //
    // One composer at the thread's foot, as the design places it, rather than
    // one per post. It replies to the ROOT — and because the root is the only
    // post offering a reply affordance, a request naming the wrong parent is not
    // constructible here rather than being guarded against.
    ColumnLayout {
        objectName: "replyComposerOpen"
        visible: screen.readState === "ok"
                 && screen.capability.canPost === true
                 && screen.threadId !== ""
        Layout.fillWidth: true
        spacing: DTheme.itemGap

        // "REPLYING AS" plus the mark and the address — the design's
        // attribution line above the box, kept from `piece/ui-navigation`'s
        // screen. The address is on screen rather than one click away, which is
        // the standing rule wherever an identity is named: a reply is signed by
        // a key, and the user is owed knowing which one before they write.
        //
        // The key is `identity.publicKey` and not `capability`'s: the posting
        // probe answers whether a reply could be sent, never by whom.
        RowLayout {
            Layout.fillWidth: true
            spacing: DTheme.itemGap

            Identicon {
                address: screen.identity.publicKey
                size: DTheme.markInFeed
                visible: screen.identity.publicKey !== ""
            }

            Text {
                text: "REPLYING AS"
                font: DTheme.label
                color: DTheme.inkMuted
                textFormat: Text.PlainText
            }

            AddressLabel { address: screen.identity.publicKey }

            Item { Layout.fillWidth: true }
        }

        DComposer {
            id: replyComposer
            objectName: "replyComposer"
            kind: "reply"
            stoaAddress: screen.stoaAddress

            // The root's op id — `id` and not `currentVersion`, because a reply
            // names the post rather than the version, and the post's id does not
            // move when it is edited.
            parentOp: screen.threadId
            Layout.fillWidth: true

            // **No optimistic row.** The thread is re-read and shows what core
            // reports; nothing composed by the view is inserted. A row the view
            // built would carry a sanitiser report and a revision flag it would
            // have to invent.
            onPublished: screen.reload()
        }

        // What a reply IS. The bundle's own caveat ends "earlier versions stay
        // readable", which promises the facility the control above is inert for
        // — so the promise is dropped and what remains is true.
        Text {
            text: "A reply is a signed record. It can be edited later."
            font: DTheme.note
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // ---- posting gate: shut ----------------------------------------------
    //
    // No disabled composer and no text field behind the gate: a box the user
    // could type into and not send would lose what they wrote. `composer-view`
    // owns this rule; the two branches are one expression against its
    // complement, so no probe answer renders both and none renders neither.
    ColumnLayout {
        objectName: "replyGateShut"
        visible: screen.readState === "ok" && screen.capability.canPost !== true
        Layout.fillWidth: true
        spacing: 6

        Text {
            text: "You cannot reply in this Stoa yet."
            font: DTheme.heading
            color: DTheme.ink
            wrapMode: Text.WordWrap
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        // Core's reason, verbatim. Nothing here branches on its text.
        Text {
            text: screen.capability.reason
            font: DTheme.bodySmall
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        Text {
            text: "There is no disabled composer here. A box you could type into and not send would lose what you wrote."
            font: DTheme.bodySmall
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }
}
