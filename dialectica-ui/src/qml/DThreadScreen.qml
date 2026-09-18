import QtQuick
import QtQuick.Layouts

// Screen 06 — a thread: the root post and its replies, nested.
//
// **This screen's CONTENTS are `piece/ui-thread-view`'s to contract.** This
// piece owns the transition into and out of it — what travels across, and that
// the feed is reachable again. What is built here is the design bundle's screen
// 06 so the app can be launched and seen; the requirements about what a thread
// renders, how a hidden root reads, and what a missing parent chain does belong
// to that capability and are not asserted here.
//
// NESTING IS COMPUTED HERE, and that is the contract rather than a choice.
// `read_thread`'s trait doc: "The items are flat and each names its parent.
// Nesting is the view's to compute — depth is a count of parents, and the view
// holds the parents. No item reports a depth or an indentation level." So a
// depth read off a reply would be reading a field core deliberately does not
// send.
//
// **Depth is counted by walking parents, never by trusting an order.** A flat
// list in arrival order says nothing about structure, and a view that indented
// by position would draw a shape the data does not have.
ScreenFrame {
    id: screen

    // What travelled across the transition. All three come from the feed row
    // that opened this thread; none is defaulted to something plausible.
    property string stoaAddress: ""
    property string stoaGenesis: ""
    property string threadRoot: ""

    // ---- read state -----------------------------------------------------
    //
    // The same three-state shape `FeedScreen` uses, and for the same reason: an
    // empty read and an unreadable one mean opposite things and must never
    // render alike.
    //   "unread"  nothing asked for yet
    //   "ok"      the store answered; `rows` is what it holds
    //   "failed"  the store could not be read; `failure` says why
    property string readState: "unread"
    property var rows: []
    property string failure: ""

    // The posting gate, re-probed on every render exactly as the feed does it.
    // Never cached across a render — PLAN.md §9.1: "caching it across a keystore
    // change is how a button outlives the key that justified it."
    property var capability: ({ canPost: false, reason: "" })

    // The identity report. A DIFFERENT question from the capability probe, and
    // the two can honestly disagree (`lib.rs:258`). Held for one render only,
    // written by `reload()` and by nothing else.
    property var identity: ({ hasIdentity: false, publicKey: "", reason: "" })

    signal closed()

    Component.onCompleted: screen.reload()

    // A different thread is a different read, for the reason `FeedScreen`
    // carries the same handler: this screen is mounted once and re-pointed at
    // whichever thread was opened, so without this the first thread opened would
    // be the only one ever read — `reload()` ran at construction, when the root
    // was still empty, and nothing asked again.
    //
    // Keyed on the root op rather than on the Stoa: the Stoa can stay the same
    // across two threads, and it is the root that says which thread this is.
    onThreadRootChanged: screen.reload()

    function reload() {
        if (screen.stoaAddress === "" || screen.threadRoot === "") {
            screen.readState = "failed"
            screen.failure = "No thread was given to this view."
            return
        }

        screen.capability = screen.capabilityFrom(
            Core.getCapabilities(screen.stoaAddress))
        screen.identity = screen.identityFrom(Core.whoAmI(screen.stoaAddress))

        var reply = Core.readThread({
            stoa: screen.stoaAddress,
            genesis: screen.stoaGenesis,
            thread: screen.threadRoot,
            page: 0,
            includeHidden: false
        })

        if (!reply.ok) {
            screen.readState = "failed"
            screen.failure = reply.error
            return
        }

        // A success MUST carry `items`. The same refusal to rest on a guarantee
        // made one module away that `FeedScreen.reload()` makes, for the same
        // reason: an absent array assigned into `rows` renders as a thread with
        // nothing in it, which is not what an unreadable reply means.
        if (reply.value.items === undefined || !Array.isArray(reply.value.items)) {
            screen.readState = "failed"
            screen.failure = "The core module answered without a list of posts, "
                           + "so what it holds for this thread is unknown."
            return
        }

        screen.rows = screen.nested(reply.value.items)
        screen.failure = ""
        screen.readState = "ok"
    }

    // Normalised exactly as `FeedScreen` normalises it, so a reader of
    // `capability` does not have to know which branch produced it. `=== true`
    // is what makes failing closed structural: absent, `undefined`, `"true"`,
    // `1` and `null` are all not-`true`.
    function capabilityFrom(probe) {
        var granted = probe.ok && probe.value.canPost === true
        var supplied = probe.ok
            ? (typeof probe.value.reason === "string" ? probe.value.reason : "")
            : probe.error
        return {
            canPost: granted,
            reason: granted ? "" : (typeof supplied === "string" ? supplied : "")
        }
    }

    // One `who_am_i` reply, normalised. `hasIdentity === true` and nothing
    // looser: this decides whether the footer claims an identity, and a chip
    // handed `"true"` or `1` under a truthy test claims one the machine may not
    // have.
    function identityFrom(probe) {
        var present = probe.ok && probe.value.hasIdentity === true
        return {
            hasIdentity: present,
            publicKey: present && typeof probe.value.publicKey === "string"
                ? probe.value.publicKey : "",
            reason: !present && probe.ok
                    && typeof probe.value.reason === "string"
                ? probe.value.reason
                : (probe.ok ? "" : probe.error)
        }
    }

    // Depth for every item, computed by walking the parent chain.
    //
    // **A chain this peer cannot complete stops at what it holds** rather than
    // guessing. `read_thread` already refuses to place a post by its claimed
    // thread — "A post whose parent chain this peer cannot complete is returned
    // under no thread rather than placed by its claim" — and the same caution
    // applies to depth: an unresolvable parent yields depth 0 rather than an
    // invented position under some other post.
    //
    // The walk is bounded by the item count, so a cycle in peer-supplied data
    // terminates instead of hanging the view. Peer data is attacker-controlled
    // and `parent` is a field a peer fills in.
    function nested(items) {
        var byId = ({})
        var i
        for (i = 0; i < items.length; i++) {
            if (items[i] !== null && typeof items[i] === "object"
                && typeof items[i].opId === "string")
                byId[items[i].opId] = items[i]
        }

        var out = []
        for (i = 0; i < items.length; i++) {
            var row = items[i]
            if (row === null || typeof row !== "object")
                continue

            var depth = 0
            var seen = ({})
            var cursor = row
            // Bounded by the number of items: every step consumes one distinct
            // id, so a cycle or a chain longer than the page cannot spin.
            while (depth < items.length
                   && cursor !== undefined
                   && typeof cursor.parent === "string"
                   && cursor.parent !== ""
                   && byId[cursor.parent] !== undefined
                   && seen[cursor.parent] === undefined) {
                seen[cursor.parent] = true
                cursor = byId[cursor.parent]
                depth = depth + 1
            }

            out.push({ row: row, depth: depth })
        }
        return out
    }

    // ---- header ---------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: 16

        // Offered unconditionally, for the reason the feed's own back control
        // is: the state this exists to leave is exactly the state that must not
        // withdraw it. A failed read is when a user most wants out.
        FlatButton {
            objectName: "threadBackButton"
            text: "Back to the feed"
            kind: "secondary"
            onClicked: screen.closed()
        }

        Item { Layout.fillWidth: true }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 2
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
    }

    // ---- state: the thread could not be read ----------------------------

    Rectangle {
        objectName: "threadFailedPanel"
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
                text: "This thread could not be read, so nothing can be shown."
                font: DTheme.heading
                color: DTheme.accent
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            // Core's own message. Its errors are written to name a fix, and the
            // three ways a root can be unreadable are three different messages
            // because they call for three different responses — rewording here
            // would collapse them.
            Text {
                text: screen.failure
                font: DTheme.address
                color: DTheme.ink
                wrapMode: Text.WrapAnywhere
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

    // ---- state: the posts -----------------------------------------------
    //
    // Indentation per depth, with the rule down the left that the design draws.
    // The indent is a multiple of `cardPaddingX`, which is what the reference
    // uses (34px, then 68px).
    Repeater {
        model: screen.readState === "ok" ? screen.rows : []

        delegate: RowLayout {
            id: row
            required property var modelData
            Layout.fillWidth: true
            Layout.leftMargin: row.modelData.depth * DTheme.cardPaddingX
            spacing: DTheme.blockGap

            // The rule down the left of a reply. Absent on the root, which is
            // not nested under anything.
            Rectangle {
                visible: row.modelData.depth > 0
                Layout.preferredWidth: DTheme.hairline
                Layout.fillHeight: true
                color: DTheme.rule
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: DTheme.itemGap

                PostHeader {
                    identityKey: row.modelData.row.author
                    // No generated name on the wire, by contract — a derived
                    // value beside the material it derives from is two values
                    // that could disagree, and a name on the wire is one a relay
                    // could strip or forge. The key carries the row alone.
                    generatedName: ""
                    edited: row.modelData.row.isRevised === true
                    Layout.fillWidth: true
                }

                // A hidden root is returned marked with its body withheld, and
                // must not render as an ordinary post. A reader owed the subject
                // of the thread is owed knowing it was hidden.
                Text {
                    visible: row.modelData.row.isHidden === true
                    text: "HIDDEN BY A MODERATOR"
                    font: DTheme.label
                    color: DTheme.accent
                    textFormat: Text.PlainText
                }

                SanitisedText {
                    value: row.modelData.row.body
                    bodyFont: row.modelData.depth === 0 ? DTheme.postBody : DTheme.body
                    Layout.fillWidth: true
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: DTheme.hairline
                    color: DTheme.rule
                }
            }
        }
    }

    // ---- the reply composer ----------------------------------------------
    //
    // Under the double rule, exactly as the design places it. Present only
    // where a reply would actually send: there is no disabled composer and no
    // text field behind the gate, because a box the user could type into and
    // not send would lose what they wrote.
    ColumnLayout {
        objectName: "threadComposer"
        visible: screen.capability.canPost === true && screen.readState === "ok"
        Layout.fillWidth: true
        spacing: DTheme.itemGap

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
            Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
        }

        // "REPLYING AS" plus the mark, the name and the address — the design's
        // attribution line above the box. The address is on screen rather than
        // one click away, which is the standing rule wherever an identity is
        // named.
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
            parentOp: screen.threadRoot
            Layout.fillWidth: true

            // Re-read rather than inserting a row the view built. A composed row
            // would carry the sanitiser's counts and a revision flag the view
            // would have to invent.
            onPublished: screen.reload()
        }
    }

    // ---- posting gate: shut ----------------------------------------------

    ColumnLayout {
        objectName: "threadGateShut"
        visible: screen.capability.canPost !== true && screen.readState === "ok"
        Layout.fillWidth: true
        spacing: 6

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }

        Text {
            text: "You cannot post, reply or vote in this Stoa yet."
            font: DTheme.heading
            color: DTheme.ink
            wrapMode: Text.WordWrap
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        // Core's reason, verbatim and unbranched-on. The wording is deliberately
        // not part of the contract, so a view selecting what it shows by
        // matching on that prose would turn every improvement to it into a
        // silent breaking change.
        Text {
            text: screen.capability.reason
            font: DTheme.bodySmall
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }
}
