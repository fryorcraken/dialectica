import QtQuick
import QtQuick.Layouts

// Screen 01 — entering a Stoa by choosing a key.
//
// The user is picking one of several candidate identities. Nothing is written
// until a candidate is KEPT, and what is kept cannot be changed afterwards:
// there is no rename, because the name is only the key written out.
//
// Three things this screen must get right, each of which has its own reason:
//
//   1. **A refusal is not a failure and not a success.** Core answers a refused
//      keep with `{"kept":false,"reason":…}`, which is a SUCCESS at the wire
//      level — `Core.call()` reports `ok:true` for it, correctly. A screen that
//      branched on `ok` alone would report an identity that was never stored.
//   2. **Nothing is pre-selected.** A pre-selected candidate plus one press is a
//      permanent choice the user never made.
//   3. **Addresses are shown in full here.** This is where the decision is
//      made, and the address is the only unforgeable way to tell two candidates
//      apart. Elsewhere they are abbreviated; the abbreviation lives in
//      AddressLabel and is never hand-rolled a second time.
ScreenFrame {
    id: screen

    // The Stoa this identity is being chosen for. Every core call here takes it.
    property string stoaAddress: ""

    // Emitted once a keep has REPORTED that it stored something. Whoever
    // navigated here re-asks the module who the user is; this signal does not
    // carry the identity, because a caller acting on what this screen says
    // would be acting on the screen rather than on the store.
    signal identityKept()

    // ---- the one state variable -----------------------------------------
    //
    // Exactly one of these, never a combination. `FeedScreen` computes one
    // `readState` for the same reason and it is a requirement here: with one
    // string, "at most one of kept, refused and failed is shown" holds by
    // construction rather than by every visible: binding being got right.
    //
    //   "intro"    nothing asked for yet, no candidates (the opening state)
    //   "slate"    candidates on offer
    //   "refused"  a keep came back kept:false — candidates are still on screen
    //   "kept"     a keep came back kept:true and carried an identity
    //   "failed"   the failure shape, or a reply this screen cannot read
    property string phase: "intro"

    property var candidates: []
    property string slateId: ""
    property string failure: ""
    property string refusal: ""

    // The "nothing selected" sentinel, named once rather than spelled `-1` at
    // six call sites.
    //
    // Naming it is not cosmetic. While it was a bare literal, a candidate whose
    // reply carried `index:-1` collided with it and rendered as chosen while
    // nothing was selected — the sentinel and a position were the same value
    // and nothing said they must not be. The collision is now closed at the
    // boundary (`isCandidate` refuses a negative index), and this constant is
    // what makes the two readable as different things at every site that
    // compares them.
    readonly property int nothingSelected: -1

    // Which candidate is selected, as that candidate's own `index`, or
    // `nothingSelected`. Every path that replaces the candidate list assigns
    // the sentinel: a position carried across a refresh would name a candidate
    // the user never looked at.
    property int selectedIndex: nothingSelected

    // The identity as the KEEP REPLY reported it — never the row that was sent.
    // Null until a reply said `kept:true` and carried an address.
    //
    // `encrypted` is copied across raw rather than coerced with `=== true`. An
    // absent field and a reported `false` mean different things: one is "the
    // module did not say", the other is "the module said your key is in the
    // clear". Collapsing them would make the screen claim the second when the
    // first is true.
    property var keptIdentity: null

    // Whether recovering the identity needs more than the master key, as the
    // module reported it. Same three-valued treatment as `encrypted`, and for
    // the same reason — `undefined` means no claim is made.
    property var recoveryNeedsTheRecord: undefined

    // NO SPEC: the spec does not say whether this screen opens its own
    // who-am-I call. It does not: the launch decision is Main's, taken before
    // this screen exists, and the opening state's one action that reaches the
    // module is the slate request. Nothing here runs on completion.

    // ---- asking for candidates ------------------------------------------
    //
    // Unlimited by contract. There is no counter, no cooldown and no control
    // that disables itself after some number of presses — a view that rationed
    // this would impose a limit the module does not have, on the one action
    // that makes the result a choice rather than a value the user was handed.
    function requestSlate() {
        if (screen.stoaAddress === "") {
            screen.enterFailed("No Stoa address was given to this view.")
            return
        }

        var reply = Core.generateIdentitySlate(screen.stoaAddress)

        if (!reply.ok) {
            // Core's message, unchanged. Its errors are written to name a fix,
            // and rewording one here would mean maintaining the same guidance
            // in two places.
            screen.enterFailed(reply.error)
            return
        }

        // A reply this screen cannot read is a FAILURE, not a slate with
        // nothing in it. The two mean opposite things and look identical when
        // collapsed — the same confusion FeedScreen guards against between an
        // empty store and an unreadable one.
        if (!Array.isArray(reply.value.candidates)) {
            screen.enterFailed("The core module answered without a list of candidate "
                             + "identities, so what it offered is unknown.")
            return
        }

        // A set identifier the screen cannot read is a reply it cannot read.
        // NOT `String(...)`: stringifying an object yields the literal text
        // `[object Object]`, which would then be sent back to core as the set
        // the selection was made against.
        if (typeof reply.value.slate !== "string") {
            screen.enterFailed("The core module answered without naming the set of "
                             + "candidates it offered, so a choice made here could "
                             + "not be tied to what was shown.")
            return
        }

        // **Every entry must be a candidate, not merely present.** The check
        // above establishes that `candidates` is a list; it says nothing about
        // what is in it. An array of `[null,"str",…]` passed it, reached the
        // slate phase and built rows that threw on every access — rendering a
        // blank row with no address and no mark, and inviting the user to
        // choose between it and a real one.
        //
        // This runs BEFORE anything is assigned, so a bad entry anywhere means
        // no slate rather than a slate with a hole in it. Core validates its
        // own output; this is the view validating what it is handed, which is
        // the standing rule for everything arriving from outside.
        for (var i = 0; i < reply.value.candidates.length; i++) {
            if (!screen.isCandidate(reply.value.candidates[i])) {
                screen.enterFailed("The core module offered something this screen "
                                 + "cannot read as an identity, so what it offered "
                                 + "is unknown.")
                return
            }
        }

        // The count is the reply's. A view laying out a fixed number of rows
        // would show four of five, or an empty fifth, the moment the module's
        // count changed — and the module's count is the module's to change.
        screen.candidates = reply.value.candidates
        screen.slateId = reply.value.slate
        screen.selectedIndex = screen.nothingSelected
        screen.refusal = ""
        screen.failure = ""
        screen.keptIdentity = null
        screen.phase = "slate"
    }

    // What this screen can render and address as a candidate.
    //
    // **`index` must be a non-negative integer**, and that is the load-bearing
    // clause rather than a tidiness check. `selectedIndex` uses a negative
    // sentinel for "nothing selected", so a candidate carrying a negative index
    // COLLIDES with it: the row drew the chosen background, the chosen border
    // and a visible SELECTED word while nothing was selected, and the keep
    // button beneath it was dead — `keepSelected()`'s guard returned silently
    // and clicking the row re-selected the sentinel, so the user could not
    // escape it.
    //
    // Refusing the whole slate rather than range-checking at the render site is
    // the fix the shape of the bug asks for: a range check would leave a
    // candidate on screen that the user can see and cannot choose, which is the
    // same dead end with a narrower blast radius. A candidate this screen
    // cannot address is not a candidate.
    function isCandidate(value) {
        if (value === null || typeof value !== "object")
            return false
        if (typeof value.index !== "number" || !isFinite(value.index))
            return false
        if (value.index < 0 || Math.floor(value.index) !== value.index)
            return false
        // The address is the only unforgeable way to tell two candidates apart,
        // so one without a usable address is not something to choose between.
        if (typeof value.address !== "string" || value.address === "")
            return false
        return true
    }

    // Every failure arrives here, so there is one place where `candidates` is
    // cleared and the selection dropped. A failed refresh must not leave the
    // previous set on screen as though it were current.
    function enterFailed(message) {
        screen.candidates = []
        screen.slateId = ""
        screen.selectedIndex = screen.nothingSelected
        screen.refusal = ""
        screen.keptIdentity = null
        screen.failure = message
        screen.phase = "failed"
    }

    // Selecting reaches the module not at all. Nothing is written until a
    // candidate is kept, so there is nothing here to tell core about.
    function select(index) {
        screen.selectedIndex = index
    }

    // ---- keeping ---------------------------------------------------------

    function keepSelected() {
        // ONE guard, not two. `FlatButton` emits `clicked()` from its own
        // MouseArea whatever it looks like, so dimming the control is
        // appearance and this line is the refusal.
        //
        // This was two guards — `selectedIndex === nothingSelected` and then
        // `candidate === null` — and the review found the first untestable:
        // every fixture's indexes were non-negative, so the second caught
        // everything and removing the first left the suite green. The answer is
        // not a third fixture to reach a guard, it is that there is only one
        // question here. `isCandidate()` refuses a negative index at the
        // boundary, so the sentinel is not a value any candidate can carry, so
        // **"is a candidate selected" and "does the selection name a candidate"
        // are the same question** — and `candidateAt()` is the one thing that
        // answers it. A second guard that could only ever be true when the
        // first was is not a guard, it is a comment that runs.
        var candidate = screen.candidateAt(screen.selectedIndex)
        if (candidate === null)
            return

        var reply = Core.keepIdentity(screen.stoaAddress, screen.slateId, candidate.index)

        if (!reply.ok) {
            screen.enterFailed(reply.error)
            return
        }

        // THE THREE OUTCOMES. `ok:true` is not "kept" — it is "the module
        // answered". What it answered is `kept`, and a refusal is a successful
        // reply carrying a negative answer.
        if (reply.value.kept !== true) {
            // A NON-STRING reason is treated as absent, not stringified.
            // `String({code:7})` is the literal text `[object Object]`, which
            // names no fix and is strictly worse than the fallback below — the
            // fallback exists for exactly the case where no usable reason
            // arrived, and a reason that cannot be read is that case.
            screen.refusal = typeof reply.value.reason === "string"
                             && reply.value.reason !== ""
                ? reply.value.reason
                : "The core module refused to keep this identity and gave no reason."
            // `candidates` and `slateId` are deliberately untouched: a refusal
            // leaves the set on screen so the keep can be tried again.
            screen.phase = "refused"
            return
        }

        // A `kept:true` with no address is a reply this screen cannot read.
        // Rendering it would show a kept identity with an empty address, which
        // claims a success the reply did not actually describe.
        if (typeof reply.value.address !== "string" || reply.value.address === "") {
            screen.enterFailed("The core module reported an identity was kept but did "
                             + "not say which, so what is stored is unknown.")
            return
        }

        // Taken from the REPLY, never from the row that was sent. The two agree
        // when core is right, and when they disagree the reply is what is true
        // about the store.
        screen.keptIdentity = {
            address: reply.value.address,
            publicKey: reply.value.publicKey,
            path: reply.value.path,
            encrypted: reply.value.encrypted
        }
        screen.refusal = ""
        screen.failure = ""
        screen.phase = "kept"
        screen.identityKept()
    }

    // The candidate carrying this `index`, or null.
    //
    // **This is `keepSelected()`'s only guard**, and it can be because
    // `isCandidate()` guarantees no candidate carries a negative index: the
    // sentinel therefore matches nothing here, and "nothing is selected" and
    // "the selection names no candidate" collapse into one answer. A scan
    // rather than `candidates[index]` because `index` is the candidate's own
    // identifier as core published it, not its position in the array — those
    // agree as core builds them and reading one as the other would be the view
    // re-deriving a value it was handed.
    function candidateAt(index) {
        for (var i = 0; i < screen.candidates.length; i++) {
            if (screen.candidates[i].index === index)
                return screen.candidates[i]
        }
        return null
    }

    // ---- heading ---------------------------------------------------------
    //
    // Shown in every phase. What is being chosen does not stop being true once
    // a set is on screen.
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 7

        Text {
            // copy.json `onboarding.title`
            text: "Choose the identity you will keep here."
            font: Theme.display
            color: Theme.ink
            wrapMode: Text.WordWrap
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        Text {
            // copy.json `onboarding.body`, FIRST HALF ONLY.
            //
            // The bundle's string continues "…and the key is yours in this Stoa
            // only — it cannot be linked to you anywhere else". That is FALSE
            // in this release: one key signs in every Stoa (PLAN.md §5.2, "the
            // public key is the join"), so an observer watching two Stoas can
            // tell it is the same participant. PLAN.md §5.2.1: "Cross-Stoa
            // unlinkability is suspended, not withdrawn … Do not describe the
            // MVP as having it, and do not describe the design as having
            // dropped it."
            //
            // Nothing replaces the withheld clause. A substitute privacy claim
            // would be the same failure in different words, and saying less is
            // available where saying something false is not.
            text: "You are picking a key. Its name is computed from it, so the name cannot be changed afterwards."
            font: Theme.body
            color: Theme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: Theme.lineHeightBody
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // The double rule under the heading.
    ColumnLayout {
        Layout.fillWidth: true
        spacing: 2
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }
        Rectangle { Layout.fillWidth: true; Layout.preferredHeight: Theme.hairline; color: Theme.ink }
    }

    // ---- phase: the opening state ---------------------------------------
    //
    // No candidates, and no slate call made. A set generated on arrival is a
    // set the user never asked to see, and it makes the first thing on screen a
    // choice rather than an explanation of what is being chosen.
    ColumnLayout {
        visible: screen.phase === "intro"
        Layout.fillWidth: true
        spacing: Theme.itemGap

        Text {
            text: "You have no identity here yet. Nothing has been generated and nothing has been stored."
            font: Theme.bodySmall
            color: Theme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        FlatButton {
            // NO SPEC: the spec requires that the opening state offer exactly
            // one action reaching the module and does not pin its label. This
            // one says what pressing it does; the bundle's "Show me five more"
            // would be wrong here, because there is no first five yet.
            text: "Show me some keys"
            kind: "primary"
            onClicked: screen.requestSlate()
        }
    }

    // ---- phase: candidates ----------------------------------------------
    //
    // One row per candidate the reply carried, in the reply's order. A row
    // shows the mark and the address IN FULL and presents nothing as a name.
    //
    // **There is no name to show.** Core deliberately carries none — which
    // words a key produces is a separate contract — and the view cannot compute
    // one, because the derivation is not built. So the row shows no derivation
    // path, no index, no position number and no truncated address in a name's
    // place: each would be read as the thing the user is choosing, and none of
    // them is. The name arrives as one Text above the address in the column
    // below, and nothing else about this row changes when it does.
    Repeater {
        model: (screen.phase === "slate" || screen.phase === "refused") ? screen.candidates : []

        delegate: Rectangle {
            id: row
            required property var modelData

            readonly property bool chosen: screen.selectedIndex === row.modelData.index

            Layout.fillWidth: true
            implicitHeight: rowBody.implicitHeight + 22
            color: row.chosen ? Theme.field : "transparent"
            border.width: row.chosen ? Theme.border : Theme.hairline
            border.color: row.chosen ? Theme.ink : Theme.rule2

            RowLayout {
                id: rowBody
                anchors.fill: parent
                anchors.margins: 11
                spacing: 14

                Identicon {
                    address: row.modelData.address
                    size: 38
                }

                // The column a generated name will join, above the address.
                // Empty of anything name-shaped today rather than holding a
                // placeholder: a bound "" would be filled by accident.
                ColumnLayout {
                    spacing: 2
                    Layout.fillWidth: true

                    AddressLabel {
                        // IN FULL. The user is choosing between keys and the
                        // addresses are the only unforgeable way to tell the
                        // candidates apart. `full` is AddressLabel's own mode —
                        // no elision is hand-rolled here or anywhere.
                        address: row.modelData.address
                        full: true
                        Layout.fillWidth: true
                    }
                }

                // Not a checkmark and not a badge. The word says what the state
                // is, and it is present only on the row that is actually
                // selected — never on a row the screen chose for the user.
                Text {
                    visible: row.chosen
                    text: "SELECTED"
                    font: Theme.label
                    color: Theme.ink
                    textFormat: Text.PlainText
                }
            }

            MouseArea {
                anchors.fill: parent
                cursorShape: Qt.PointingHandCursor
                onClicked: screen.select(row.modelData.index)
            }
        }
    }

    // ---- phase: the actions under a set ----------------------------------
    RowLayout {
        visible: screen.phase === "slate" || screen.phase === "refused"
        Layout.fillWidth: true
        spacing: 18

        FlatButton {
            // copy.json `onboarding.refresh`
            text: "Show me five more"
            kind: "secondary"
            onClicked: screen.requestSlate()
        }

        Text {
            // copy.json `onboarding.refreshNote`
            text: "Refresh as often as you like. Nothing is published until you keep one."
            font: Theme.note
            color: Theme.inkMuted
            wrapMode: Text.WordWrap
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }

        FlatButton {
            // copy.json `onboarding.keep`. Dimmed while nothing is selected —
            // and `keepSelected()` refuses independently, because this control
            // emits clicked() whatever it looks like.
            text: "Keep this identity"
            kind: screen.selectedIndex >= 0 ? "primary" : "secondary"
            opacity: screen.selectedIndex >= 0 ? 1.0 : 0.45
            onClicked: screen.keepSelected()
        }
    }

    // The permanence warning, on screen WITH the candidates and before the keep
    // is taken. This is the last moment at which it is still information rather
    // than an explanation of something that already happened.
    Text {
        visible: screen.phase === "slate" || screen.phase === "refused"
        // copy.json `onboarding.apparatus.permanence`, in the body as well as
        // the margin: it qualifies the action, and the action is here.
        text: "There is no settings screen where this can be changed later, because the name is only the key written out. Choosing again means being someone else here."
        font: Theme.bodySmall
        color: Theme.accent
        wrapMode: Text.WordWrap
        lineHeight: 1.55
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- phase: a keep that was refused ----------------------------------
    //
    // Its own block, neither the kept state nor the failed one. The set above
    // is still on screen and the keep can be tried again.
    Rectangle {
        visible: screen.phase === "refused"
        Layout.fillWidth: true
        implicitHeight: refusedBody.implicitHeight + 2 * Theme.cardPaddingY
        color: Theme.field
        border.width: Theme.border
        border.color: Theme.accent

        ColumnLayout {
            id: refusedBody
            anchors.fill: parent
            anchors.margins: Theme.cardPaddingY
            spacing: Theme.itemGap

            Text {
                text: "That identity was not kept."
                font: Theme.heading
                color: Theme.accent
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                text: "Nothing was stored. The keys above are still on offer."
                font: Theme.bodySmall
                color: Theme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            // The module's reason, as the module wrote it. Core's reasons are
            // written to name a fix.
            Text {
                text: screen.refusal
                font: Theme.address
                color: Theme.ink
                wrapMode: Text.WrapAnywhere
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }
    }

    // ---- phase: kept -----------------------------------------------------
    Rectangle {
        visible: screen.phase === "kept"
        Layout.fillWidth: true
        implicitHeight: keptBody.implicitHeight + 2 * Theme.cardPaddingY
        color: Theme.field
        border.width: Theme.hairline
        border.color: Theme.rule2

        ColumnLayout {
            id: keptBody
            anchors.fill: parent
            anchors.margins: Theme.cardPaddingY
            spacing: Theme.itemGap

            Text {
                text: "This is who you are here now."
                font: Theme.heading
                color: Theme.ink
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 14

                Identicon {
                    address: screen.keptIdentity !== null ? screen.keptIdentity.address : ""
                    size: Theme.markInList
                }

                AddressLabel {
                    // The REPLY's address. If it differed from the row that was
                    // sent, this is the one that is true about the store.
                    address: screen.keptIdentity !== null ? screen.keptIdentity.address : ""
                    full: true
                    Layout.fillWidth: true
                }
            }

            // ---- how the master key is protected -------------------------
            //
            // Three cases, not two. A reply that OMITTED the field produces no
            // claim at all, because an absent `false` and a reported `false`
            // mean different things and a screen that collapsed them would
            // state as fact something the module never said.
            //
            // Where the key is NOT encrypted this says so plainly, with no
            // lock, no shield and no use of the word "secure". Protection that
            // reads as strong while being absent is worse than visible
            // plaintext, because plaintext is something a person can act on.
            Text {
                readonly property var reported: screen.keptIdentity !== null
                    ? screen.keptIdentity.encrypted
                    : undefined

                visible: reported !== undefined
                text: reported === true
                    ? "The master key on this machine is stored encrypted."
                    : "The master key on this machine is stored unencrypted, in the clear. Anyone who can read the file can use it."
                font: Theme.bodySmall
                color: reported === true ? Theme.inkSoft : Theme.accent
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            // ---- the backup gap ------------------------------------------
            //
            // Also three-valued, and also taken from the module: the view has
            // no filesystem access, so a screen that did not ask would be
            // guessing about a secret on the user's disk. Where recovery needs
            // more than the master key, no text here may present any saved
            // value as a complete backup.
            Text {
                visible: screen.recoveryNeedsTheRecord === true
                text: "Which key you chose is recorded only on this machine. A copy of the master key by itself is not enough to get back in — it can derive this identity, but not tell you which one was yours."
                font: Theme.bodySmall
                color: Theme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: 1.55
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }
    }

    // ---- phase: failed ---------------------------------------------------
    //
    // The failure names itself, and core's own message is shown rather than a
    // reworded one. Deliberately not the same layout as any other state here.
    Rectangle {
        visible: screen.phase === "failed"
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
                text: "No identity could be offered, so nothing can be chosen yet."
                font: Theme.heading
                color: Theme.accent
                wrapMode: Text.WordWrap
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            Text {
                text: "Nothing was stored and nothing was lost. This is a failure to read or write on this machine, not a choice that went wrong."
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
                text: "Try again"
                kind: "primary"
                onClicked: screen.requestSlate()
            }
        }
    }

    apparatus: [
        MarginNote {
            label: "ON PERMANENCE"
            // copy.json `onboarding.apparatus.permanence`, verbatim.
            body: "There is no settings screen where this can be changed later, because the name is only the key written out. Choosing again means being someone else here."
        },
        MarginNote {
            label: "ON UNIQUENESS"
            // copy.json `onboarding.apparatus.uniqueness`, with the WORD COUNT
            // REMOVED rather than corrected.
            //
            // The bundle says "the same three words"; a previous revision of
            // this file said "four", because §5.2.1 had settled four at the
            // time. The count has now moved three times — three, then four
            // (merged), now three again on a different basis (adjective + noun
            // + "of" + place) — and each move made this copy wrong and left a
            // test pinning the wrong number, which then had to be argued with
            // before the copy could be corrected.
            //
            // So the sentence states its obligation and no number. That
            // obligation is what actually matters here and it does not depend
            // on the count: uniqueness is not merely unbuilt but UNAVAILABLE,
            // since there is no authority to hold a namespace, so the interface
            // has to stay correct when two identities present the same name —
            // and the correctness is that the address is always present.
            //
            // Required even though no row shows a name yet: what the user is
            // choosing is a key whose name follows from it, and a screen that
            // explained the choice without saying the name settles nothing
            // would have taught the opposite of what is true.
            body: "Names are not unique and are not identifiers. Someone else in this Stoa may hold the same name. Your address is what tells you apart, so it is printed beside your name everywhere."
        },
        MarginNote {
            label: "ON THE MARK"
            caveat: false
            // copy.json `onboarding.apparatus.mark`, verbatim. The mark is a
            // second recognition channel, never a verification: it is derived
            // from the same address an impersonator can grind against.
            body: "The hatched shape is drawn from the same key: two inks for the weave, a third for the outline, all three chosen by the key. A curved contour always means a person; an angular one always means a Stoa."
        }
    ]
}
