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

    // The last listing the core answered with, **whether or not it is current**.
    //
    // `reload()`'s failure path deliberately does not write this: a failure must
    // not blank a good listing underneath a banner, so after a failed reload it
    // still holds the previous page. That makes it the WRONG property for any
    // caller asking "what is this peer in?", and it is private to this file for
    // that reason — read `visibleRows` instead.
    property var lastListing: []

    // What the screen is entitled to render and what any other caller may read.
    //
    // **The guard lives in the data rather than at each call site.** It used to
    // be written twice — once on this file's `Repeater` model, 213 lines from
    // the state that makes it necessary, and again in `Main.qml` as
    // `list.readState === "ok" ? list.rows : []`. Two copies of one guard in two
    // files is CLAUDE.md's signal to reshape: a third reader would have had to
    // know to write it a third time, and the one who forgets renders a listing
    // the screen has already said was not read.
    readonly property var visibleRows:
        screen.readState === "ok" ? screen.lastListing : []
    property string failure: ""
    property bool hasMore: false
    property int page: 0
    // NO SPEC: the spec names no page size for this listing. 25 was chosen to
    // fill a card without a scroll on the mockup's 1000px width; nothing else
    // rests on it and a reviewer changing it breaks no requirement.
    property int perPage: 25

    // Genesis records this view holds, keyed by address.
    //
    // **Both replies carry them**, and this map is filled from the reply rather
    // than from anything the view remembers. The core RETAINS the record for
    // every Stoa the peer is in — `stoa-membership` requires it so moderation can
    // be resolved later — and `create_stoa` and `list_stoas` now report it as
    // `genesis`, hex-encoded.
    //
    // Two things need one and neither can work around its absence, because
    // deriving a record from an address is what a one-way hash forbids: sharing a
    // Stoa, and opening its feed. Both were therefore unreachable while the
    // replies named only an address — "Open" handed `read_feed` an empty string,
    // which is zero bytes and fails the version-byte take as "genesis record
    // ended mid-field". That is the error the owner met, and it was this absence
    // rather than a codec defect.
    //
    // **Filling it from the LISTING is what survives a restart.** A map filled
    // only on create or join holds records for the current session, so a
    // relaunched app could open nothing it had not just made.
    property var genesisByStoa: ({})

    // Record what a reply says about one Stoa, or leave the map untouched.
    //
    // A non-string or an empty string is NOT recorded, and the omission is the
    // point: `genesisFor` returning "" is what hides the share affordance and is
    // an honest "this view holds no record". Writing "" in would make `canShare`
    // true for a Stoa whose share text cannot be built, and would send that same
    // "" to `read_feed` — reinstating the exact defect this closes.
    function rememberGenesis(stoa, genesis) {
        if (typeof stoa !== "string" || stoa === "")
            return
        if (typeof genesis !== "string" || genesis === "")
            return
        // **The explicit `genesisByStoaChanged()` is what makes the screen
        // re-evaluate, and it is load-bearing on its own.** A QML `var` property
        // does not notify on an in-place key write, so without this emit the map
        // would hold the record while `canShare`'s binding kept the share button
        // hidden and `Open` inert — data right, screen wrong, and silent, because
        // basecamp swallows QML errors.
        //
        // Do not replace it with a reassignment. `var next = screen.genesisByStoa`
        // binds `next` to the SAME object the property already holds — JS objects
        // are reference types and nothing here clones — so `screen.genesisByStoa =
        // next` assigns the property to the object it already pointed at and
        // notifies nothing on its own merits. That reassignment was here, was
        // credited by this comment for the fix, and was measured to be a no-op
        // (object identity unchanged across the call); it was removed rather than
        // left with a note, because a line whose only role is to be explained away
        // is the line a later reader mistakes for the mechanism.
        //
        // NOT covered by the suite: see `design.md`, "The record map notifies
        // through an explicit signal". Removing this emit leaves the specs green,
        // because they read `canShare` as a function rather than through a live
        // binding.
        screen.genesisByStoa[stoa] = genesis
        screen.genesisByStoaChanged()
    }

    // ---- this peer's master key -----------------------------------------
    //
    // A FIRST-RUN step, and it is a step rather than something folded into
    // creation on the owner's decision. `wire.rs` states the invariant creation
    // rests on — "Creation fails without a key rather than inventing one. There
    // is no path from here to `Keystore::generate()`" — and minting inside
    // `create_stoa` would overturn it. So the key is made here, explicitly, and
    // `create_stoa` keeps refusing without one.
    //
    // "" | "minting" | "ready" | "failed"
    property string keyState: ""
    property string keyFailure: ""
    // `{publicKey, encrypted, wasNew}` from the reply, or `null`.
    property var keyHeld: null

    // Mint this peer's master key, or report the one it already has.
    //
    // **Safe to press twice**, and the safety is core's rather than a guard
    // here: an existing key comes back with `wasNew:false` and is never
    // replaced. A disabled-button guard in this file would be a second copy of a
    // rule core already enforces, and the copy is the one that gets forgotten.
    function createIdentity() {
        screen.keyState = "minting"
        screen.keyFailure = ""

        var reply = Core.createIdentity()

        if (!reply.ok) {
            screen.keyState = "failed"
            screen.keyFailure = reply.error
            screen.keyHeld = null
            return
        }

        // A success MUST name the key. Without this the screen would report a
        // key that was made on the strength of `ok` alone, which is the shape
        // `Core.qml` warns about at its `ok: true` return — and here the user
        // would then press "Create it" and meet the deadlock's error again with
        // nothing explaining it.
        if (typeof reply.value.publicKey !== "string" || reply.value.publicKey === "") {
            screen.keyState = "failed"
            screen.keyFailure = "The core module answered without a key, so there "
                              + "is nothing to create a Stoa with."
            screen.keyHeld = null
            return
        }

        screen.keyHeld = reply.value
        screen.keyState = "ready"
    }

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

    property DClipboardSink clipboard: null

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

        // Every record the listing hands over, recorded before the rows render.
        //
        // Additive rather than a replacement: paging away from a Stoa must not
        // drop the record for it, and a peer with more Stoas than fit a page
        // would otherwise lose the ability to share or open one it had just
        // scrolled past. An item short of its `genesis` is skipped rather than
        // failing the read — the listing is still a truthful answer to "which
        // Stoas am I in", and the share affordance's absence is how that item
        // reads.
        for (var i = 0; i < reply.value.items.length; ++i) {
            var item = reply.value.items[i]
            if (item)
                screen.rememberGenesis(item.stoa, item.genesis)
        }

        screen.lastListing = reply.value.items
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

        // The record for the Stoa just created, from the creation reply itself.
        //
        // The reload below would also supply it, and this is still here rather
        // than left to it: the creation reply is the authority on the Stoa THIS
        // call settled on, and a `perPage`-th Stoa created by a peer already in a
        // full page would not appear on page 0 at all. Recording it here means
        // "create it, then share it" never depends on where the new row landed.
        screen.rememberGenesis(reply.value.stoa, reply.value.genesis)

        screen.page = 0
        screen.reload()
    }

    // ---- pasting --------------------------------------------------------

    function preview() {
        var parsed = DStoaReference.parse(screen.pasted)
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
            font: DTheme.display
            color: DTheme.ink
            textFormat: Text.PlainText
        }

        Text {
            // copy.json `stoaList.subtitle`
            text: "NO DIRECTORY EXISTS · JOIN BY ADDRESS"
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText
        }

        Item { Layout.fillWidth: true }
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 2
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }
    }

    // ---- state: membership could not be read ----------------------------
    //
    // Accent border, the core's reason verbatim, and a retry. Deliberately NOT
    // the layout the empty state uses: the two mean opposite things and a reader
    // must never have to tell them apart by reading carefully.
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
                text: "Which Stoas you are in could not be read."
                font: DTheme.heading
                color: DTheme.accent
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
                font: DTheme.bodySmall
                color: DTheme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

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

    // ---- state: read fine, in no Stoas ----------------------------------
    Rectangle {
        visible: screen.readState === "ok" && screen.visibleRows.length === 0
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
                text: "You are not in any Stoa yet."
                font: DTheme.heading
                color: DTheme.ink
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                text: "Your membership was read without error and records nothing. "
                    + "There is no directory to browse: a Stoa reaches you because "
                    + "somebody shared it, or because you create one."
                font: DTheme.bodySmall
                color: DTheme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }
    }

    // ---- state: the rows ------------------------------------------------

    Repeater {
        // No guard here any more: `visibleRows` is already empty unless the read
        // succeeded, so the invariant travels with the data instead of being
        // restated 213 lines from the state that makes it necessary.
        model: screen.visibleRows

        // The reference's screen 08 separates rows by a hairline under EVERY row,
        // the last one included — unlike the moderation lists, which drop it on
        // the last. A ColumnLayout per row rather than a bare RowLayout is what
        // gives the rule somewhere to live.
        delegate: ColumnLayout {
            id: rowBlock
            required property var modelData
            Layout.fillWidth: true
            spacing: DTheme.itemGap

            RowLayout {
                id: row
                readonly property var modelData: rowBlock.modelData

                // The address, as the reply spelled it. A row must never be able
                // to render a title without one: a founding title is chosen
                // freely by whoever created the Stoa, is not unique, is verified
                // against nothing, and can be picked to resemble another Stoa's.
                // The address is the only distinguishing half.
                readonly property string rowStoa:
                    typeof modelData.stoa === "string" ? modelData.stoa : ""
                readonly property string rowTitle:
                    typeof modelData.foundingTitle === "string" ? modelData.foundingTitle : ""

                Layout.fillWidth: true
                spacing: 16

                Identicon {
                    address: row.rowStoa
                    size: DTheme.markInList
                }

                ColumnLayout {
                    spacing: 2

                    // An EMPTY founding title renders as nothing, and the row is
                    // still a row. An empty title is legal — the genesis record
                    // has no minimum length — so a row that collapsed would be a
                    // Stoa the user cannot reach, and a substitute like
                    // "Untitled" would be a title no peer agrees on.
                    Text {
                        text: row.rowTitle
                        font: DTheme.rowTitle
                        color: DTheme.ink
                        // Peer-supplied, unnormalised, carrying whatever
                        // characters its creator typed. Never markup.
                        textFormat: Text.PlainText
                        visible: row.rowTitle !== ""
                    }

                    // The one abbreviation, owned by AddressLabel: head 8,
                    // middle 8, tail 6. No second elision is written anywhere —
                    // a head-and-tail form is the shape vanity-address
                    // generators are built to defeat, and a second
                    // implementation is how one screen quietly acquires the
                    // weaker one.
                    AddressLabel {
                        address: row.rowStoa
                        // A click copies the FULL address, not the abbreviation
                        // on screen. This is not the share string: an address
                        // alone cannot be joined, and the share affordance below
                        // is the only thing that produces something joinable.
                        copyText: row.rowStoa
                        onCopyRequested: {
                            if (screen.clipboard)
                                screen.clipboard.copy(row.rowStoa)
                        }
                    }
                }

                Item { Layout.fillWidth: true }

                // **Nothing occupies the position the mockup puts a count in**,
                // and that is deliberate. `31 posts received here` would be a
                // legitimate number — it counts what this machine holds — and it
                // is simply not computed: a listed item carries an address and a
                // title, no call answers how many posts this peer holds for a
                // Stoa, and the thread listing reports whether a further page
                // exists rather than a total. A page length from some other call
                // rendered here would look like a total, would not be one, and
                // would be wrong by an amount that grows with the Stoa.
                // `nothing received yet` is unavailable for the same reason: it
                // is a claim about a count nothing computed.

                // Share: offered only where this view HOLDS the genesis record,
                // and absent otherwise. An address is a one-way hash of the
                // record, so a share without one would produce a
                // plausible-looking string that fails to verify on somebody
                // else's machine, as a refusal they cannot explain. The absence
                // is the honest rendering and is not an error state.
                FlatButton {
                    objectName: "shareButton"
                    text: "Copy a shareable reference"
                    kind: "secondary"
                    visible: screen.canShare(row.rowStoa)
                    onClicked: {
                        var text = DStoaReference.shareText(row.rowStoa, screen.genesisFor(row.rowStoa))
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

            // The separator under every row, the last included. The reference
            // draws it that way so the list reads as a bounded block rather
            // than as rows trailing off into the page.
            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: DTheme.hairline
                color: DTheme.rule
            }
        }
    }

    // ---- pagination -----------------------------------------------------

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

    // ---- this peer's identity, before anything can be created ------------
    //
    // **The first-run step, and the reason a fresh profile was stuck.** Creating
    // a Stoa needs a creator key and mints none; the only thing that writes one
    // is per-Stoa onboarding, which refuses a request naming no Stoa. So a fresh
    // install could reach neither, and "Create it" answered the keystore's own
    // `no keystore found; create one before posting` with nothing anywhere able
    // to create one.
    //
    // **Always shown, in every read state.** It is not conditional on the
    // listing: a peer whose membership could not be READ may still have no key,
    // and hiding the one affordance that unblocks them behind a successful read
    // is how the deadlock would come back for exactly the users least able to
    // diagnose it.
    //
    // **Nothing here is a claim about whether a key exists.** The screen does not
    // probe — `who_am_i` and `get_capabilities` both take a Stoa and there is no
    // Stoa yet, which is the same reason "Create it" is always offered. So this
    // renders what the LAST press reported and asserts nothing before one.
    ColumnLayout {
        Layout.fillWidth: true
        spacing: DTheme.itemGap

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }

        // **The word "identity" is deliberately absent from every string in
        // this block**, and it is a requirement rather than a style choice.
        // `test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_
        // identity` bans it on this screen: one key signs in every Stoa in this
        // release, so raising "identity" here would offer an unlinkability
        // property the software does not have.
        //
        // The honest word is what this actually is — a key, belonging to this
        // machine, used everywhere. `DOnboardingScreen` is where a per-Stoa
        // identity is chosen, and that screen may say so because there the claim
        // is true.
        Text {
            text: "THIS MACHINE'S KEY"
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText
        }

        Text {
            text: "A Stoa records its creator's key, so this machine needs one "
                + "before it can create or post. Making it writes a key here and "
                + "tells nobody. The same key signs in every Stoa you hold."
            font: DTheme.bodySmall
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        FlatButton {
            objectName: "createIdentityButton"
            text: "Create this machine's key"
            kind: "primary"
            onClicked: screen.createIdentity()
        }

        // What the last press reported. `wasNew` distinguishes a key just made
        // from one that was already there — both are successes, and saying which
        // is what stops a second press reading as a failure.
        ColumnLayout {
            visible: screen.keyState === "ready" && screen.keyHeld !== null
            Layout.fillWidth: true
            spacing: 4

            Text {
                objectName: "identityOutcomeText"
                text: screen.keyHeld !== null && screen.keyHeld.wasNew === true
                    ? "A key was created for this machine."
                    : "This machine already had a key. Nothing was replaced."
                font: DTheme.body
                color: DTheme.ink
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            AddressLabel {
                objectName: "identityKeyLabel"
                address: screen.keyHeld !== null && typeof screen.keyHeld.publicKey === "string"
                       ? screen.keyHeld.publicKey : ""
                Layout.fillWidth: true
                onCopyRequested: {
                    if (screen.clipboard)
                        screen.clipboard.copy(address)
                }
            }

            // **Stated plainly when it is true, and this is the first run's
            // normal case.** With no passphrase set, core stores the key in the
            // clear and says so in `encrypted`. A user whose key is unprotected
            // should learn it from the interface rather than from a file. There
            // is no passphrase flow here to offer instead — saying the true
            // thing is a smaller claim than a control that does not exist.
            Text {
                objectName: "identityUnencryptedWarning"
                visible: screen.keyHeld !== null && screen.keyHeld.encrypted === false
                text: "This key is stored unencrypted on this machine. Anyone who "
                    + "can read the file can post as you."
                font: DTheme.bodySmall
                color: DTheme.accent
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }

        // The core's reason, unreworded — the keystore's own vocabulary, which
        // names a fix and reads like the one a failed creation gives.
        ColumnLayout {
            visible: screen.keyState === "failed"
            Layout.fillWidth: true
            spacing: 4

            Text {
                text: "No key was created."
                font: DTheme.body
                color: DTheme.accent
                textFormat: Text.PlainText
            }

            Text {
                objectName: "identityFailureText"
                text: screen.keyFailure
                font: DTheme.address
                color: DTheme.ink
                wrapMode: Text.WrapAnywhere
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }
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
        spacing: DTheme.itemGap

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }

        Text {
            text: "CREATE A STOA"
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: createField.implicitHeight + 10
                color: DTheme.field
                border.width: DTheme.hairline
                border.color: DTheme.ink

                TextInput {
                    id: createField
                    anchors.fill: parent
                    anchors.margins: 5
                    text: screen.createTitle
                    font: DTheme.body
                    color: DTheme.ink
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
                font: DTheme.label
                color: DTheme.inkMuted
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
                font: DTheme.body
                color: DTheme.accent
                textFormat: Text.PlainText
            }

            Text {
                objectName: "createFailureText"
                text: screen.createFailure
                font: DTheme.address
                color: DTheme.ink
                wrapMode: Text.WrapAnywhere
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }
    }

    // ---- paste a reference ----------------------------------------------

    ColumnLayout {
        Layout.fillWidth: true
        spacing: DTheme.itemGap

        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: DTheme.hairline; color: DTheme.ink }

        Text {
            // The mockup's `PASTE AN ADDRESS` is narrowed here, and narrowing it
            // is the point: an address alone can never join anything, so a field
            // captioned that way asks for input whose successful-looking form
            // cannot succeed.
            text: "PASTE A STOA REFERENCE — THE ADDRESS AND ITS FOUNDING RECORD"
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 14

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: pasteField.implicitHeight + 10
                color: DTheme.field
                border.width: DTheme.hairline
                border.color: DTheme.ink

                TextInput {
                    id: pasteField
                    anchors.fill: parent
                    anchors.margins: 5
                    text: screen.pasted
                    font: DTheme.address
                    color: DTheme.ink
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
            font: DTheme.bodySmall
            color: DTheme.accent
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // ---- THIS SCREEN CARRIES NO IDENTITY CHIP, AND THAT IS A REQUIREMENT ---
    //
    // A `DScreenFooter` — the pagination/identity/lamps row — was mounted here
    // and removed. It is recorded rather than silently dropped, because adding
    // one back is the obvious next idea and it breaks a merged requirement that
    // no gate would catch by inspection.
    //
    // **`stoa-navigation-view` R13 forbids this screen raising identity at all**
    // — `tst_stoa_screens.qml`'s
    // `test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_identity`
    // fails on the word, by design: one key signs in every Stoa in this release,
    // so anything on a per-Stoa screen that raises identity offers an
    // unlinkability property the software does not have. `DIdentityChip`'s
    // no-identity arm renders "Voting, posting and replying need an identity",
    // which trips it. The requirement is right and the chip is what was wrong.
    //
    // **This is NOT in tension with the key block above.** That block is about
    // a key belonging to THIS MACHINE — its strings say so and say nothing about
    // identity, which is exactly the distinction R13 draws. A chip claiming a
    // per-Stoa identity is the thing forbidden here; naming the one key the
    // machine signs everything with is not.
    //
    // **The lamps are not here either, and that is `Main.qml`'s decision rather
    // than this screen's.** `DStatusBar` is mounted once as shared chrome
    // outside every screen's `visible:` binding, so it accompanies this screen
    // too — see `Main.qml`'s "shared chrome" block for why a per-screen subset
    // would make its absence ambiguous.
    //
    // **The reference agrees on the chip**: screen 08 carries no identity chip.
    // It belongs to the feed (03/05), where posting is what the identity is FOR
    // and the claim is about this machine rather than about a Stoa.
}
