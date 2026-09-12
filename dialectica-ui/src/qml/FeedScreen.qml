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

    Component.onCompleted: screen.reload()

    function reload() {
        if (screen.stoaAddress === "") {
            screen.readState = "failed"
            screen.failure = "No Stoa address was given to this view."
            return
        }

        // The posting gate is re-probed with every render of the feed.
        var probe = Core.getCapabilities(screen.stoaAddress)
        screen.capability = probe.ok
            ? probe.value
            : ({ canPost: false, reason: probe.error })

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
            isPerson: false
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

        Repeater {
            model: screen.orderings
            delegate: Text {
                required property var modelData
                text: modelData.label
                font: Theme.bodySmall
                color: screen.ordering === modelData.key ? Theme.ink : Theme.inkMuted
                textFormat: Text.PlainText

                MouseArea {
                    anchors.fill: parent
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        screen.ordering = modelData.key
                        screen.page = 0
                        screen.reload()
                    }
                }
            }
        }

        // A separate view over a filter the projection already applies. It
        // needs no key and no authority — seeing what was moderated is a
        // reader's affordance, not a moderator privilege.
        Text {
            text: "SHOW HIDDEN"
            font: Theme.label
            color: screen.includeHidden ? Theme.accent : Theme.inkMuted

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
                text: "STORE READ OK · 0 POSTS HELD"
                font: Theme.label
                color: Theme.inkMuted
                textFormat: Text.PlainText
            }
        }
    }

    // ---- state: posts ---------------------------------------------------

    Repeater {
        model: screen.readState === "ok" ? screen.rows : []

        delegate: ColumnLayout {
            required property var modelData
            Layout.fillWidth: true
            spacing: Theme.itemGap

            PostHeader {
                identityAddress: modelData.author
                // There is no generated name on the wire: core sends an
                // address, and the name is the address's shadow. Until the
                // name derivation lands the address carries the row alone,
                // which is the honest half of the pair.
                generatedName: ""
                edited: modelData.isRevised === true
                Layout.fillWidth: true
            }

            // A hidden row is visible only in the show-hidden view, and it must
            // not render indistinguishably from a visible one — a reader who
            // asked to see what was hidden is owed knowing which those were.
            Text {
                visible: modelData.isHidden === true
                text: "HIDDEN BY A MODERATOR · SHOWN BECAUSE YOU ASKED TO SEE HIDDEN POSTS"
                font: Theme.label
                color: Theme.accent
                textFormat: Text.PlainText
            }

            SanitisedText {
                value: modelData.body
                Layout.fillWidth: true
            }

            Repeater {
                model: modelData.attachments
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

    // ---- posting gate: a reason, never a dead text field -----------------
    //
    // There is no disabled composer here and no text field behind the gate. A
    // box the user could type into and not send would lose what they wrote.
    ColumnLayout {
        visible: screen.capability.canPost !== true
        Layout.fillWidth: true
        spacing: 6

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }

        Text {
            // copy.json `compose.blockedTitle`
            text: "You cannot reply in this Stoa yet."
            font: Theme.heading
            color: Theme.ink
            textFormat: Text.PlainText
        }

        // The reason comes from core and already names a fix — that is a
        // documented obligation on the keystore errors, with a test. Rewording
        // it here would mean maintaining the same guidance twice.
        Text {
            text: screen.capability.reason !== undefined ? screen.capability.reason : ""
            font: Theme.bodySmall
            color: Theme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    apparatus: [
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
