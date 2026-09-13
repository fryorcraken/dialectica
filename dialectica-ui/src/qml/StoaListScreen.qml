import QtQuick
import QtQuick.Layouts

// Screen 02: the Stoas this peer is in, the create affordance, and the field a
// user pastes a reference into.
//
// Three read states, kept apart the way FeedScreen keeps its three apart — one
// string computed from one variable, so no combination of flags can put two on
// screen at once. The distinction matters more here than on the feed: a peer in
// no Stoas is a new install and should create or join one, while a peer whose
// membership cannot be READ may hold many Stoas and cannot see them. Telling
// that user they belong to nothing invites them to re-join Stoas they are
// already in.
ScreenFrame {
    id: screen

    // ---- read state -----------------------------------------------------
    //   "unread"  nothing asked for yet
    //   "ok"      membership answered; `rows` is what it holds (possibly none)
    //   "failed"  membership could not be read; `failure` says why
    property string readState: "unread"
    property var rows: []
    property string failure: ""
    property bool hasMore: false
    property int page: 0
    // NO SPEC: the spec names no page size for this listing. 25 was chosen to
    // fill a card without a scroll on the mockup's 1000px width; nothing else
    // rests on it and a reviewer changing it breaks no requirement.
    property int perPage: 25

    // Genesis records this view holds, keyed by address.
    //
    // **`list_stoas` does not return them.** The core RETAINS the record for
    // every Stoa the peer is in — `stoa-membership` requires it so moderation
    // can be resolved later — but the listing reply carries `stoa` and
    // `foundingTitle` only. So the view holds a record for exactly the Stoas it
    // created or joined in THIS session, and for no others.
    //
    // Two things need one and neither can work around its absence, because
    // deriving a record from an address is what a one-way hash forbids:
    // sharing a Stoa, and opening its feed. Both are therefore conditional on
    // this map, and the absence of a share affordance is the honest rendering
    // rather than an error state. Closing this properly is a CORE change —
    // widen the listing item to carry the retained record.
    property var genesisByStoa: ({})

    // ---- creation -------------------------------------------------------
    property string createTitle: ""
    // "" | "creating" | "created" | "failed"
    property string createState: ""
    property string createFailure: ""
    property var created: null   // {stoa, foundingTitle} from the reply

    // ---- the paste field ------------------------------------------------
    property string pasted: ""
    property string pasteFailure: ""

    // A reference the user has asked to look at, before anything is joined.
    // `null` until they act; the join screen renders from it.
    signal previewRequested(string stoa, string genesis)
    signal stoaChosen(string stoa, string foundingTitle, string genesis)

    property ClipboardSink clipboard: null

    Component.onCompleted: screen.reload()

    function reload() {
        var reply = Core.listStoas(screen.page, screen.perPage)

        if (!reply.ok) {
            // The failure path never writes `rows`, so a previous success
            // cannot be left on screen underneath a failure banner.
            screen.readState = "failed"
            screen.failure = reply.error
            return
        }

        // A success MUST carry `items`. The core keeps the two reply shapes
        // disjoint, but "in none" and "could not be read" are the very pair this
        // screen exists to distinguish, so it does not rest on a guarantee made
        // one module away: an absent array assigned into `rows` renders as "you
        // are in no Stoas", which is the one confusion to prevent.
        if (reply.value.items === undefined || !Array.isArray(reply.value.items)) {
            screen.readState = "failed"
            screen.failure = "The core module answered without a list of Stoas, "
                           + "so which Stoas you are in is unknown. This is not "
                           + "an empty membership."
            return
        }

        screen.rows = reply.value.items
        screen.hasMore = reply.value.hasMore === true
        screen.failure = ""
        screen.readState = "ok"
    }

    // The record this view holds for a Stoa, or "" when it holds none.
    function genesisFor(stoa) {
        var g = screen.genesisByStoa[stoa]
        return typeof g === "string" ? g : ""
    }

    function canShare(stoa) {
        return screen.genesisFor(stoa) !== ""
    }

    // ---- creating -------------------------------------------------------

    function create() {
        // The title is passed THROUGH, empty included. The core accepts an empty
        // title — the genesis record has no minimum length — so a view refusing
        // one would make a Stoa other peers decode and verify without complaint
        // unreachable through this interface. Nothing is appended either: adding
        // a suffix to obtain a different Stoa mints one with a title the user did
        // not choose, permanently, at an address that cannot be withdrawn.
        screen.createState = "creating"
        screen.createFailure = ""

        var reply = Core.createStoa(screen.createTitle)

        if (!reply.ok) {
            // The keystore's own reason, unreworded. It names a fix, and it is
            // the same vocabulary a posting failure uses — deliberately, so the
            // two read alike.
            screen.createState = "failed"
            screen.createFailure = reply.error
            screen.created = null
            return
        }

        if (typeof reply.value.stoa !== "string" || reply.value.stoa === "") {
            screen.createState = "failed"
            screen.createFailure = "The core module reported a creation without an address, "
                                 + "so there is nothing to share or read."
            screen.created = null
            return
        }

        // Creating the same title twice is ONE Stoa reported twice. A genesis
        // record carries no nonce and no timestamp, so the same creator and the
        // same title is the same record and the same address — the core returns
        // that address again, and this is success, not a collision to resolve.
        screen.created = reply.value
        screen.createState = "created"

        // A Stoa just created is one whose record this view could hold — but the
        // creation reply carries `stoa`, `foundingTitle` and `policy`, and no
        // genesis record either. So there is still nothing to record here, and
        // the share affordance stays absent for it. Said plainly rather than
        // left as an apparent oversight.

        screen.page = 0
        screen.reload()
    }

    // ---- pasting --------------------------------------------------------

    function preview() {
        var parsed = StoaReference.parse(screen.pasted)
        if (!parsed.ok) {
            // Refused BEFORE any call. This is the "not a Stoa reference"
            // outcome, and it is a different thing from a well-formed pair the
            // core refuses — that one means somebody handed over a record that
            // is not the one the address names, and the answer is not to retry.
            screen.pasteFailure = parsed.reason
            return
        }
        screen.pasteFailure = ""
        screen.previewRequested(parsed.stoa, parsed.genesis)
    }

    // ---- header ---------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: 16

        Text {
            // copy.json `stoaList.title`
            text: "Stoas you hold"
            font: Theme.display
            color: Theme.ink
            textFormat: Text.PlainText
        }

        Text {
            // copy.json `stoaList.subtitle`
            text: "NO DIRECTORY EXISTS · JOIN BY ADDRESS"
            font: Theme.label
            color: Theme.inkMuted
            textFormat: Text.PlainText
        }

        Item { Layout.fillWidth: true }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 2
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }
    }

    // ---- state: membership could not be read ----------------------------
    //
    // Accent border, the core's reason verbatim, and a retry. Deliberately NOT
    // the layout the empty state uses: the two mean opposite things and a reader
    // must never have to tell them apart by reading carefully.
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
                text: "Which Stoas you are in could not be read."
                font: Theme.heading
                color: Theme.accent
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                // The load-bearing sentence: it says what this ISN'T. A user
                // told they belong to nothing will re-join Stoas they are
                // already in.
                text: "This does not mean you are in no Stoas. Stoas you have created or "
                    + "joined are recorded on disk and cannot be listed right now."
                font: Theme.bodySmall
                color: Theme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                text: screen.failure
                font: Theme.address
                color: Theme.ink
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

    // ---- state: read fine, in no Stoas ----------------------------------
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
                text: "You are not in any Stoa yet."
                font: Theme.heading
                color: Theme.ink
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                text: "Your membership was read without error and records nothing. "
                    + "There is no directory to browse: a Stoa reaches you because "
                    + "somebody shared it, or because you create one."
                font: Theme.bodySmall
                color: Theme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }
    }

    // ---- state: the rows ------------------------------------------------

    Repeater {
        model: screen.readState === "ok" ? screen.rows : []

        delegate: RowLayout {
            id: row
            required property var modelData

            // The address, as the reply spelled it. A row must never be able to
            // render a title without one: a founding title is chosen freely by
            // whoever created the Stoa, is not unique, is verified against
            // nothing, and can be picked to resemble another Stoa's. The address
            // is the only distinguishing half.
            readonly property string rowStoa:
                typeof modelData.stoa === "string" ? modelData.stoa : ""
            readonly property string rowTitle:
                typeof modelData.foundingTitle === "string" ? modelData.foundingTitle : ""

            Layout.fillWidth: true
            spacing: 16

            Identicon {
                address: row.rowStoa
                size: Theme.markInList
            }

            ColumnLayout {
                spacing: 2

                // An EMPTY founding title renders as nothing, and the row is
                // still a row. An empty title is legal — the genesis record has
                // no minimum length — so a row that collapsed would be a Stoa
                // the user cannot reach, and a substitute like "Untitled" would
                // be a title no peer agrees on.
                Text {
                    text: row.rowTitle
                    font: Theme.body
                    color: Theme.ink
                    // Peer-supplied, unnormalised, carrying whatever characters
                    // its creator typed. Never markup.
                    textFormat: Text.PlainText
                    visible: row.rowTitle !== ""
                }

                // The one abbreviation, owned by AddressLabel: head 8, middle 8,
                // tail 6. No second elision is written anywhere — a head-and-tail
                // form is the shape vanity-address generators are built to
                // defeat, and a second implementation is how one screen quietly
                // acquires the weaker one.
                AddressLabel {
                    address: row.rowStoa
                    // A click copies the FULL address, not the abbreviation on
                    // screen. This is not the share string: an address alone
                    // cannot be joined, and the share affordance below is the
                    // only thing that produces something joinable.
                    copyText: row.rowStoa
                    onCopyRequested: {
                        if (screen.clipboard)
                            screen.clipboard.copy(row.rowStoa)
                    }
                }
            }

            Item { Layout.fillWidth: true }

            // **Nothing occupies the position the mockup puts a count in**, and
            // that is deliberate. `31 posts received here` would be a legitimate
            // number — it counts what this machine holds — and it is simply not
            // computed: a listed item carries an address and a title, no call
            // answers how many posts this peer holds for a Stoa, and the thread
            // listing reports whether a further page exists rather than a total.
            // A page length from some other call rendered here would look like a
            // total, would not be one, and would be wrong by an amount that
            // grows with the Stoa. `nothing received yet` is unavailable for the
            // same reason: it is a claim about a count nothing computed.

            // Share: offered only where this view HOLDS the genesis record, and
            // absent otherwise. An address is a one-way hash of the record, so a
            // share without one would produce a plausible-looking string that
            // fails to verify on somebody else's machine, as a refusal they
            // cannot explain. The absence is the honest rendering and is not an
            // error state.
            FlatButton {
                objectName: "shareButton"
                text: "Copy a shareable reference"
                kind: "secondary"
                visible: screen.canShare(row.rowStoa)
                onClicked: {
                    var text = StoaReference.shareText(row.rowStoa, screen.genesisFor(row.rowStoa))
                    if (text !== "" && screen.clipboard)
                        screen.clipboard.copy(text)
                }
            }

            FlatButton {
                text: "Open"
                kind: "secondary"
                onClicked: screen.stoaChosen(row.rowStoa, row.rowTitle,
                                             screen.genesisFor(row.rowStoa))
            }
        }
    }

    // ---- pagination -----------------------------------------------------

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

    // ---- create ---------------------------------------------------------
    //
    // A title and nothing else. There is no creator-key field and no identity
    // picker, because the creator key is what makes the creator the Stoa's sole
    // moderator and it is fixed inside the address preimage forever — a field
    // for one would be a field that mints a Stoa nobody can moderate.
    //
    // It is ALWAYS offered, whatever the keystore holds. The posting probe takes
    // a Stoa address and there is no Stoa yet at creation, so there is nothing to
    // ask it about; a button hidden or disabled here would be hidden on a guess
    // rather than on an answer the core gave.
    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.itemGap

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }

        Text {
            text: "CREATE A STOA"
            font: Theme.label
            color: Theme.inkMuted
            textFormat: Text.PlainText
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: createField.implicitHeight + 10
                color: Theme.field
                border.width: Theme.hairline
                border.color: Theme.ink

                TextInput {
                    id: createField
                    anchors.fill: parent
                    anchors.margins: 5
                    text: screen.createTitle
                    font: Theme.body
                    color: Theme.ink
                    clip: true
                    onTextChanged: screen.createTitle = text
                }
            }

            FlatButton {
                id: createButton
                objectName: "createStoaButton"
                text: "Create it"
                kind: "primary"
                onClicked: screen.create()
            }
        }

        // The created Stoa's address. There is no registry to look a Stoa up in
        // later, so this is the only way to name what was just made — a screen
        // that discarded it would leave the user holding a Stoa they cannot
        // share. Nothing here says the user moderates it: whether they still can
        // is a question these screens cannot answer, since the creator key
        // recorded in the record is never re-checked against the peer's current
        // signing key.
        ColumnLayout {
            visible: screen.createState === "created" && screen.created !== null
            Layout.fillWidth: true
            spacing: 4

            Text {
                text: "CREATED — THIS IS ITS ADDRESS"
                font: Theme.label
                color: Theme.inkMuted
                textFormat: Text.PlainText
            }

            AddressLabel {
                objectName: "createdAddress"
                address: screen.created !== null && typeof screen.created.stoa === "string"
                       ? screen.created.stoa : ""
                // A decision is being made about what to share, so the whole
                // address is on screen rather than the recognition abbreviation.
                full: true
                Layout.fillWidth: true
                onCopyRequested: {
                    if (screen.clipboard)
                        screen.clipboard.copy(address)
                }
            }
        }

        // Creation refused. The core's reason, unreworded — it is the keystore's
        // own vocabulary and it names a fix.
        ColumnLayout {
            visible: screen.createState === "failed"
            Layout.fillWidth: true
            spacing: 4

            Text {
                text: "The Stoa was not created."
                font: Theme.body
                color: Theme.accent
                textFormat: Text.PlainText
            }

            Text {
                objectName: "createFailureText"
                text: screen.createFailure
                font: Theme.address
                color: Theme.ink
                wrapMode: Text.WrapAnywhere
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }
    }

    // ---- paste a reference ----------------------------------------------

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Theme.itemGap

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }

        Text {
            // The mockup's `PASTE AN ADDRESS` is narrowed here, and narrowing it
            // is the point: an address alone can never join anything, so a field
            // captioned that way asks for input whose successful-looking form
            // cannot succeed.
            text: "PASTE A STOA REFERENCE — THE ADDRESS AND ITS FOUNDING RECORD"
            font: Theme.label
            color: Theme.inkMuted
            textFormat: Text.PlainText
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: pasteField.implicitHeight + 10
                color: Theme.field
                border.width: Theme.hairline
                border.color: Theme.ink

                TextInput {
                    id: pasteField
                    anchors.fill: parent
                    anchors.margins: 5
                    text: screen.pasted
                    font: Theme.address
                    color: Theme.ink
                    clip: true
                    onTextChanged: screen.pasted = text
                }
            }

            FlatButton {
                // copy.json `stoaList.pasteAction`. It previews and joins
                // NOTHING: an address inside a post is attacker-supplied content
                // and an interface that joined on paste would enrol a user in a
                // Stoa they never chose.
                text: "Look at it first"
                kind: "secondary"
                onClicked: screen.preview()
            }
        }

        Text {
            objectName: "pasteFailureText"
            visible: screen.pasteFailure !== ""
            text: screen.pasteFailure
            font: Theme.bodySmall
            color: Theme.accent
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    apparatus: [
        MarginNote {
            label: "ON TITLES"
            body: "A moderator may rename a Stoa to anything, including someone else's name. Two rows can carry the same title and be entirely different Stoas. The contour and the address differ; the title does not."
        },
        MarginNote {
            label: "ON COUNTS"
            caveat: false
            body: "No row says how many posts you hold for that Stoa. Such a number would be honest — it counts what this machine has — but nothing computes it, so there is none to show rather than one being withheld."
        },
        MarginNote {
            label: "ON SHARING"
            body: "A shareable reference carries the address and the founding record together, because an address is a hash of the record and cannot rebuild it. Rows this copy has no record for offer no share; that is the reference being absent, not broken."
        },
        MarginNote {
            label: "ON WHAT THIS LIST IS"
            caveat: false
            body: "Stoas you chose, recorded on this machine. Nobody was notified, no peer can see this list, and being in a Stoa does not mean you moderate it."
        }
    ]
}
