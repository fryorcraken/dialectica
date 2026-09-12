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
        objectName: "feedHeader"
        Layout.fillWidth: true
        spacing: 16

        Identicon {
            address: screen.stoaAddress
            size: 34
            visible: screen.stoaAddress !== ""
        }

        // The title/address pair takes the slack, and yields it back when the
        // card is narrow. Before this the column was incompressible — the title
        // and the `NoWrap` address label between them set a floor wide enough
        // that at a 760px viewport the header could not fit, and a RowLayout
        // whose minimums do not fit OVERFLOWS rather than shrinking, so the row
        // ran past the card's right edge with every child's `width` and
        // `visible` still perfectly correct. That is radicle's lesson exactly:
        // only a measurement against the container's bounds can see it.
        ColumnLayout {
            spacing: 2
            Layout.fillWidth: true
            // A floor, so the pair degrades rather than vanishing. The address
            // is already abbreviated to head-8/middle-8/tail-6, and eliding is
            // how it gives ground without the row overflowing.
            Layout.minimumWidth: 120

            Text {
                text: screen.stoaTitle
                font: DTheme.heading
                color: DTheme.ink
                textFormat: Text.PlainText   // peer-supplied: never rich text
                visible: screen.stoaTitle !== ""
                elide: Text.ElideRight
                Layout.fillWidth: true
            }

            // The address is on screen beside the title, never one click away.
            // A Stoa's title is moderator-chosen and freely forgeable; the
            // address is the identity.
            AddressLabel {
                address: screen.stoaAddress
                elide: Text.ElideRight
                Layout.fillWidth: true
            }
        }

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
                font: DTheme.bodySmall
                color: screen.ordering === modelData.key ? DTheme.ink : DTheme.inkMuted
                textFormat: Text.PlainText
                // Elides rather than pushing the row wider than the card.
                //
                // NOT `Layout.fillWidth`: that made this label greedy and it
                // then pushed the card itself past the viewport, trading one
                // overflow for a worse one — two geometry tests that had been
                // passing went red on the same change, which is the value of
                // measuring against bounds rather than properties. A minimum
                // plus an elide lets it give ground without ever asking for more
                // room than it has.
                elide: Text.ElideRight
                Layout.minimumWidth: 0
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

    // ---- state: the store could not be read -----------------------------
    //
    // Screen 07's failed half. An accent border, the failure named, and a
    // retry. Deliberately NOT the same layout as the empty state: the two mean
    // opposite things and a reader must never have to tell them apart by
    // reading carefully.
    Rectangle {
        objectName: "failedPanel"
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
                objectName: "retryButton"
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
        objectName: "emptyPanel"
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

        delegate: ColumnLayout {
            required property var modelData
            Layout.fillWidth: true
            spacing: DTheme.itemGap

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
                font: DTheme.label
                color: DTheme.accent
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
                    bodyFont: DTheme.address
                    bodyColor: DTheme.inkMuted
                    Layout.fillWidth: true
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: DTheme.hairline
                color: DTheme.rule
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

    // ---- posting gate: a reason, never a dead text field -----------------
    //
    // There is no disabled composer here and no text field behind the gate. A
    // box the user could type into and not send would lose what they wrote.
    ColumnLayout {
        visible: screen.capability.canPost !== true
        Layout.fillWidth: true
        spacing: 6

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }

        Text {
            // copy.json `compose.blockedTitle`
            text: "You cannot reply in this Stoa yet."
            font: DTheme.heading
            color: DTheme.ink
            textFormat: Text.PlainText
        }

        // The reason comes from core and already names a fix — that is a
        // documented obligation on the keystore errors, with a test. Rewording
        // it here would mean maintaining the same guidance twice.
        Text {
            text: screen.capability.reason !== undefined ? screen.capability.reason : ""
            font: DTheme.bodySmall
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // There was an `apparatus:` block here holding three MarginNotes — "ON THIS
    // ORDERING", "ON WHAT YOU HOLD", "ON THE MARK". They were the design
    // bundle's annotation of itself, written for someone reading the mockup
    // rather than for a forum reader, and they are gone with the column that
    // held them.
    //
    // What they explained is still enforced, in the places that actually bind:
    //
    // - the ordering's honest label is in the `orderings` model above, which is
    //   where UI-BRIEF's "do not label an ordering 'new' unless it is one" is
    //   met;
    // - "counts what this machine holds" is in the empty state's status line,
    //   which says POSTS HELD and deliberately renders no peer total;
    // - "the mark is never a proof" is met by printing the address beside every
    //   identicon rather than by a note saying so — AddressLabel in the header
    //   and in every PostHeader.
}
