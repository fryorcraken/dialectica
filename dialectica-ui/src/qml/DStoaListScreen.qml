import QtQuick
import QtQuick.Layouts

// The home screen: the Stoas this peer is in, the field a user pastes a
// reference into, and — decided by one value, `machineKey` — either the block
// that makes this machine's key (no key held, reference screen 0A) or the
// create affordance with the key as one line at the foot (key held, 0B). A
// third key state, "could not be read", draws neither. See `machineKey` below.
//
// Two independent state machines live here and neither reads the other: the
// membership listing (`readState`, below) and the key state. Each key state
// renders the same whatever the listing did.
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

    // ---- this machine's key: ONE value, three states --------------------
    //
    // The whole screen below the listing is decided by this one property, the
    // `hasMachineKey` flag the issue names — widened from a boolean because the
    // question has three answers, not two:
    //
    //   { state: "none", refusal: <text> }                  no key held
    //   { state: "held", publicKey: <hex>, encrypted: <bool|null> }
    //   { state: "unreadable", reason: <text> }             could not be read
    //
    // **Why not a boolean**, which is what FeedScreen's `hasIdentity` is: that
    // flag is normalised with `=== true`, so its failures fold into "no
    // identity", and on the feed that is the safe direction — no composer is
    // drawn. Here the same fold is the dangerous direction: "no key" draws
    // "Create this machine's key", which the core then refuses for a peer whose
    // key exists and cannot be read, and the user is asked to make a key they
    // already hold. So the third answer needs its own state. See design.md.
    //
    // **Why one object and not three properties.** `held` without a key to show
    // is not constructible: only `heldKey()` builds that state, and it is only
    // called with a non-empty key. Separate `keyState` + `publicKey` properties
    // could disagree, and every block below would have to check both.
    //
    // **A refused mint lives inside the no-key state it was refused in.**
    // `refusal` is why the last press of "Create this machine's key" made no
    // key, or "". It is a field of the no-key value rather than a property
    // beside it, so the next answer — which replaces the whole value — takes
    // the refusal with it. That is what makes a refusal belong to the showing
    // it happened in: nothing has to remember to clear it.
    //
    // **Asked, never remembered.** Set only from a reply received in this run —
    // `askKeyState()` on each showing, `createMachineKey()` on a press. Nothing
    // is persisted. The initial value is the least-claiming state, and it is
    // replaced before the first frame: `askKeyState()` runs synchronously in
    // `Component.onCompleted`.
    property var machineKey: screen.unreadableKey("This machine's key has not been asked about yet.")

    function noKey(refusal) {
        return { state: "none", refusal: refusal }
    }

    function heldKey(publicKey, encrypted) {
        // `encrypted` is carried only as a boolean. Anything else — absent,
        // "false", 0 — is `null`, which renders NO claim about protection
        // either way: the spec forbids a claim the reply did not make.
        return { state: "held", publicKey: publicKey,
                 encrypted: typeof encrypted === "boolean" ? encrypted : null }
    }

    function unreadableKey(reason) {
        return { state: "unreadable", reason: reason }
    }

    // One `get_master_key` reply, as one of the three states.
    //
    // `=== true` and `=== false`, each strictly, and that is the point: the
    // two affirmative states are entered only on the exact boolean, so every
    // other reply — `"false"`, `0`, `null`, a missing field, a key claimed
    // with no key named — is "could not be read", which instantiates neither
    // creation affordance. The looser `!v.hasMasterKey` would put a reply
    // stating nothing into the no-key state and draw the create-key button.
    function keyFromQuery(reply) {
        if (!reply.ok)
            return screen.unreadableKey(reply.error)
        var v = reply.value
        if (v.hasMasterKey === false)
            return screen.noKey("")
        if (v.hasMasterKey === true) {
            if (typeof v.publicKey === "string" && v.publicKey !== "")
                return screen.heldKey(v.publicKey, v.encrypted)
            return screen.unreadableKey("The core module said this machine holds a key "
                                        + "without naming it, so which key it holds is unknown.")
        }
        return screen.unreadableKey("The core module answered without saying whether "
                                    + "this machine holds a key.")
    }

    // Ask the core whether this machine holds a key, and let the answer — and
    // only the answer — decide the state.
    //
    // Called on every showing, because the key can change while this screen
    // is hidden: keeping a per-Stoa identity inside a feed also writes the
    // master key, and a screen that kept its first answer would offer that
    // user a key they already hold.
    function askKeyState() {
        screen.machineKey = screen.keyFromQuery(Core.getMasterKey())
    }

    // Mint this machine's key, from the create-key action.
    //
    // **A successful reply naming a key is the key-held state whatever its
    // `wasNew` says.** `wasNew:false` means the core found a key rather than
    // made one, and the old screen said so ("already had a key, nothing was
    // replaced"). That sentence now has no path to the screen: the action is
    // drawn only in the no-key state, and should the core find a key anyway,
    // the truthful rendering is the key-held state it is in. The core's refusal
    // to replace a key is unchanged, and is what makes this safe.
    //
    // A failure keeps the screen in the no-key state, now carrying the reason.
    function createMachineKey() {
        var reply = Core.createIdentity()

        if (!reply.ok) {
            screen.machineKey = screen.noKey(reply.error)
            return
        }

        // A success MUST name the key. `ok` means the module answered — see the
        // warning at `Core.qml`'s `ok: true` return — and a key-held state
        // entered on `ok` alone would show a key nobody named.
        if (typeof reply.value.publicKey !== "string" || reply.value.publicKey === "") {
            screen.machineKey = screen.noKey("The core module answered without naming a "
                                             + "key, so no key can be shown as held.")
            return
        }

        screen.machineKey = screen.heldKey(reply.value.publicKey, reply.value.encrypted)
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

    Component.onCompleted: {
        screen.reload()
        if (screen.visible)
            screen.askKeyState()
    }

    // Each later showing asks again. `Main.qml` shows and hides this screen by
    // its `visible` binding and never destroys it, so this is the only hook a
    // return to the list passes through. A screen first created hidden is
    // asked here on its first showing, and not before.
    onVisibleChanged: {
        if (screen.visible)
            screen.askKeyState()
    }

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
            // copy.json `homeMachineKey.heading`, verbatim.
            text: "Stoas you joined"
            font: DTheme.display
            color: DTheme.ink
            textFormat: Text.PlainText
        }

        Text {
            // copy.json `homeMachineKey.headingNote`, verbatim.
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

                // ---- PLACEHOLDER, AND THE POSITION IS THE ONLY REAL PART ---
                //
                // **This number counts nothing. It is a fixed string.** Nothing
                // computes a per-Stoa count: a listed item carries an address
                // and a title, no call answers how many posts this peer holds
                // for a Stoa, and the thread listing reports whether a further
                // page exists rather than a total.
                //
                // It is here because the owner amended `stoa-navigation-view`'s
                // "Every number rendered is one this peer can actually answer"
                // to admit a marked placeholder, so the row can be seen as
                // designed. The requirement's permanent half is untouched and is
                // honoured here: nothing global is rendered, and this value is
                // derived from NO reply.
                //
                // **A FIXED string rather than a derived one, and that is the
                // decision rather than laziness.** A page length from some other
                // call would look like a total, would not be one, and — worse —
                // would MOVE with the data, so a reader comparing two rows would
                // be reading a real signal that means something other than what
                // the row says. A placeholder that never changes is honest about
                // being a placeholder in a way a derived wrong number is not.
                // The amended requirement forbids the derived form for exactly
                // this reason.
                //
                // **The unread half is a different kind of absence**, and the
                // two are worked off differently. The post count leaves when
                // core grows a call answering it; unread leaves only when
                // someone decides to build peer-local state, which ruling 2
                // excluded from the MVP and which no core change supplies.
                // PLAN.md §9.2 case 2 entries 3 and 7.
                //
                // Rendered in `note` on `inkMuted` — the design's own treatment
                // for this position, and the quietest type on the row, which is
                // right for the one value on it that is not a fact.
                Text {
                    objectName: "rowCountPlaceholder"
                    text: "counts not yet available"
                    font: DTheme.note
                    color: DTheme.inkMuted
                    textFormat: Text.PlainText
                }

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
            //
            // **Named, because the spec requires it and a test has to find it.**
            // `stoa-navigation-view`'s "Where one listed Stoa ends and the next
            // begins is rendered" contracts one boundary per row; a test keyed
            // on geometry instead would match whatever else happened to be a
            // 1px-high Rectangle, and would go on passing if this element were
            // deleted and some unrelated rule took its place in the walk.
            //
            // Not decorative: dropping the name takes tst_stoa_screens.qml from
            // 82 passed to 79 passed, 3 failed. Five other rectangles in this
            // file share this exact width and height, so a geometry walk would
            // count them too. See design.md D5e.
            Rectangle {
                objectName: "rowSeparator"
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

    // ---- the key-dependent blocks: INSTANTIATED, not shown ---------------
    //
    // **Each block below is a `Loader` whose `active` is one comparison against
    // `machineKey.state`, and that is the requirement, not a style.** The spec
    // says of the create affordance and of the create-key action that hiding,
    // disabling or greying out does not meet it: they must be absent from the
    // element tree. A `visible: false` element is still in the tree — still
    // reachable by a walker, by accessibility, by a later binding that flips
    // it — and the bundle's rule is "never show a compose field or affordance
    // the user cannot use". An inactive Loader has no item at all.
    //
    // Nothing here is conditional on the listing's `readState`: the key state
    // and the membership listing are answered by different calls, and each of
    // the three key states renders the same way whatever the listing did.
    //
    // **The word "identity" is absent from every string in these blocks**, and
    // that is a requirement rather than a style choice.
    // `test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_
    // identity` bans it on this screen: one key signs in every Stoa in this
    // release, so raising "identity" here would offer an unlinkability property
    // the software does not have. The honest word is what this is — a key,
    // belonging to this machine, used everywhere.

    // ---- no key held (0A): making the key is the only task ---------------
    //
    // Positioned above the paste section, as the reference's 0A draws it. The
    // strings are copy.json `homeMachineKey`, verbatim.
    Loader {
        objectName: "keyBlockLoader"
        active: screen.machineKey.state === "none"
        Layout.fillWidth: true

        sourceComponent: Rectangle {
            objectName: "keyBlock"
            implicitHeight: keyBlockBody.implicitHeight + 2 * 16
            color: DTheme.field
            border.width: DTheme.hairline
            border.color: DTheme.ink

            ColumnLayout {
                id: keyBlockBody
                anchors.fill: parent
                anchors.leftMargin: 18
                anchors.rightMargin: 18
                anchors.topMargin: 16
                anchors.bottomMargin: 16
                spacing: DTheme.itemGap

                Text {
                    text: "THIS MACHINE'S KEY"
                    font: DTheme.label
                    color: DTheme.inkMuted
                    textFormat: Text.PlainText
                }

                Text {
                    objectName: "keyExplanation"
                    text: "A Stoa records its creator's key, so this machine needs one "
                        + "before it can create or post. Making it writes a key here and "
                        + "tells nobody. The same key signs in every Stoa you hold."
                    font: DTheme.body
                    color: DTheme.inkSoft
                    wrapMode: Text.WordWrap
                    lineHeight: DTheme.lineHeightBody
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                }

                FlatButton {
                    objectName: "createKeyButton"
                    text: "Create this machine's key"
                    kind: "primary"
                    onClicked: screen.createMachineKey()
                }

                // The last press's refusal, the core's reason unreworded — the
                // keystore's own vocabulary, which names a fix.
                ColumnLayout {
                    visible: screen.machineKey.refusal !== ""
                    Layout.fillWidth: true
                    spacing: 4

                    Text {
                        text: "No key was created."
                        font: DTheme.body
                        color: DTheme.accent
                        textFormat: Text.PlainText
                    }

                    Text {
                        objectName: "mintFailureText"
                        text: screen.machineKey.refusal
                        font: DTheme.address
                        color: DTheme.ink
                        wrapMode: Text.WrapAnywhere
                        textFormat: Text.PlainText
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }

    // ---- the key state could not be read ----------------------------------
    //
    // Told apart from the no-key state by what it does NOT draw: no
    // explanation, no create-key action, no create affordance, and nothing
    // saying no key is held — because a key may be held and unreadable, and
    // offering to make one is exactly the invitation the core would refuse.
    // Where the no-key block sits, so the screen's shape does not jump.
    Loader {
        objectName: "keyUnreadableLoader"
        active: screen.machineKey.state === "unreadable"
        Layout.fillWidth: true

        sourceComponent: Rectangle {
            objectName: "keyUnreadable"
            implicitHeight: keyUnreadableBody.implicitHeight + 2 * 16
            color: DTheme.field
            border.width: DTheme.border
            border.color: DTheme.accent

            ColumnLayout {
                id: keyUnreadableBody
                anchors.fill: parent
                anchors.leftMargin: 18
                anchors.rightMargin: 18
                anchors.topMargin: 16
                anchors.bottomMargin: 16
                spacing: 4

                // NO SPEC: the spec requires the reason and forbids a no-key
                // claim, and gives no copy for this state. This sentence is
                // this change's; it states what failed and claims nothing about
                // whether a key exists.
                Text {
                    text: "Whether this machine holds a key could not be read."
                    font: DTheme.body
                    color: DTheme.accent
                    wrapMode: Text.WordWrap
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                }

                Text {
                    objectName: "keyUnreadableReason"
                    // The core's message unreworded, or the view's own naming of
                    // what was wrong with a reply that was neither shape.
                    text: typeof screen.machineKey.reason === "string"
                          ? screen.machineKey.reason : ""
                    font: DTheme.address
                    color: DTheme.ink
                    wrapMode: Text.WrapAnywhere
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                }
            }
        }
    }

    // ---- key held (0B): creating a Stoa, above pasting a reference --------
    //
    // A title and nothing else. There is no creator-key field and no identity
    // picker, because the creator key is what makes the creator the Stoa's sole
    // moderator and it is fixed inside the address preimage forever — a field
    // for one would be a field that mints a Stoa nobody can moderate.
    //
    // **Offered only in the key-held state**, which reverses what this block
    // used to say ("always offered ... a button hidden here would be hidden on
    // a guess"). It is no longer a guess: `get_master_key` answers. The
    // "a key held can still be unusable" case stays reachable — a key can be
    // reported held and refused at creation — which is why the core's reason
    // on a failed creation is still rendered below.
    Loader {
        objectName: "createBlockLoader"
        active: screen.machineKey.state === "held"
        Layout.fillWidth: true

        sourceComponent: ColumnLayout {
            objectName: "createBlock"
            spacing: DTheme.itemGap

            Text {
                text: "CREATE A STOA"
                font: DTheme.label
                color: DTheme.inkMuted
                textFormat: Text.PlainText
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 12

                Rectangle {
                    Layout.fillWidth: true
                    implicitHeight: createField.implicitHeight + 16
                    color: DTheme.field
                    border.width: DTheme.hairline
                    border.color: DTheme.ink

                    TextInput {
                        id: createField
                        objectName: "createTitleField"
                        anchors.fill: parent
                        anchors.leftMargin: 10
                        anchors.rightMargin: 10
                        anchors.topMargin: 8
                        anchors.bottomMargin: 8
                        text: screen.createTitle
                        font: DTheme.body
                        color: DTheme.ink
                        clip: true
                        onTextChanged: screen.createTitle = text
                    }

                    // The placeholder is a separate Text drawn over an EMPTY
                    // field, never the field's own text: a TextInput has no
                    // placeholder, and one written into `text` would be
                    // submitted as the title. It goes the moment anything is
                    // typed, and an empty field still submits "".
                    Text {
                        objectName: "createTitlePlaceholder"
                        visible: createField.text === ""
                        anchors.fill: createField
                        verticalAlignment: Text.AlignVCenter
                        text: "Title of the new Stoa"
                        font: DTheme.note
                        color: DTheme.inkFaint
                        textFormat: Text.PlainText
                    }
                }

                FlatButton {
                    objectName: "createStoaButton"
                    text: "Create it"
                    kind: "primary"
                    onClicked: screen.create()
                }
            }

            // The created Stoa's address. There is no registry to look a Stoa
            // up in later, so this is the only way to name what was just made —
            // a screen that discarded it would leave the user holding a Stoa
            // they cannot share. Nothing here says the user moderates it:
            // whether they still can is a question these screens cannot answer,
            // since the creator key recorded in the record is never re-checked
            // against the peer's current signing key.
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
                    // A decision is being made about what to share, so the
                    // whole address is on screen rather than the recognition
                    // abbreviation.
                    full: true
                    Layout.fillWidth: true
                    onCopyRequested: {
                        if (screen.clipboard)
                            screen.clipboard.copy(address)
                    }
                }
            }

            // Creation refused. The core's reason, unreworded — it is the
            // keystore's own vocabulary and it names a fix. Reachable in the
            // key-held state: a key reported held can still be unusable when
            // creation is attempted.
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
    }

    // ---- paste a reference: in every key state -----------------------------
    //
    // Unconditional, and declared outside every Loader so it holds by
    // construction: previewing a Stoa needs no key, and a peer whose key could
    // not be read can still read.

    ColumnLayout {
        objectName: "pasteSection"
        Layout.fillWidth: true
        spacing: 8

        Text {
            // copy.json `homeMachineKey.pasteLabel`, verbatim — which is also
            // the older narrowing of the mockup's `PASTE AN ADDRESS`, and for
            // the same reason: an address alone can never join anything, so a
            // field captioned that way asks for input whose
            // successful-looking form cannot succeed.
            text: "PASTE A STOA REFERENCE — THE ADDRESS AND ITS FOUNDING RECORD"
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            Rectangle {
                Layout.fillWidth: true
                implicitHeight: pasteField.implicitHeight + 16
                color: DTheme.field
                border.width: DTheme.hairline
                border.color: DTheme.ink

                TextInput {
                    id: pasteField
                    objectName: "pasteField"
                    anchors.fill: parent
                    anchors.leftMargin: 10
                    anchors.rightMargin: 10
                    anchors.topMargin: 8
                    anchors.bottomMargin: 8
                    text: screen.pasted
                    font: DTheme.address
                    color: DTheme.ink
                    clip: true
                    onTextChanged: screen.pasted = text
                }
            }

            FlatButton {
                // copy.json `homeMachineKey.pasteAction`. It previews and joins
                // NOTHING: an address inside a post is attacker-supplied content
                // and an interface that joined on paste would enrol a user in a
                // Stoa they never chose.
                objectName: "pasteButton"
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

    // ---- key held (0B): the key is a fact, one line at the foot -----------
    //
    // Below both the create affordance and the paste section, as the reference
    // draws it. The key goes through `AddressLabel` — the one 8-8-6
    // abbreviation — and nowhere else.
    //
    // **The unencrypted warning is conditional on the reply**, where the bundle
    // draws it every time. That is the owner's decision: protection is taken
    // from a reply and never assumed, so `encrypted === false` draws it,
    // `true` does not, and a reply without the field makes no claim either way.
    //
    // **No "already had a key" and no "was created"**: the line renders the
    // same text however the key-held state was reached — the query, a mint
    // that made the key, or a mint that found one.
    Loader {
        objectName: "keyLineLoader"
        active: screen.machineKey.state === "held"
        Layout.fillWidth: true

        sourceComponent: ColumnLayout {
            objectName: "keyLine"
            spacing: 4

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: DTheme.hairline
                Layout.bottomMargin: 8
                color: DTheme.rule
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Text {
                    text: "THIS MACHINE'S KEY"
                    font: DTheme.label
                    color: DTheme.inkMuted
                    textFormat: Text.PlainText
                    Layout.alignment: Qt.AlignBaseline
                }

                AddressLabel {
                    objectName: "keyLineAddress"
                    address: typeof screen.machineKey.publicKey === "string"
                             ? screen.machineKey.publicKey : ""
                    color: DTheme.ink
                    Layout.alignment: Qt.AlignBaseline
                    onCopyRequested: {
                        if (screen.clipboard)
                            screen.clipboard.copy(address)
                    }
                }

                Item { Layout.fillWidth: true }
            }

            Text {
                objectName: "unencryptedWarning"
                visible: screen.machineKey.encrypted === false
                text: "Stored unencrypted on this machine. Anyone who can read the "
                    + "file can post as you."
                font: DTheme.bodySmall
                color: DTheme.accent
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
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
