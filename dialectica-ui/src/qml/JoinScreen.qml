import QtQuick
import QtQuick.Layouts

// Screen 03: what is about to be joined, shown before anything is joined.
//
// **This is the first screen in this project where the interface, rather than
// the core, is the security boundary.** A user acts here on a string that
// arrived from an untrusted channel, and every property that makes that safe —
// that the address is the identity, that the title is decoration, that a hash
// verifies a record and cannot reconstruct one — is a property the RENDERING has
// to carry. The core already refuses a record that does not match its address;
// what the core cannot do is stop a screen from showing a title and calling it
// verified.
ScreenFrame {
    id: screen

    // What is being previewed. Supplied by whatever navigated here — the paste
    // field, or an affordance inside a post.
    property string stoaAddress: ""
    property string stoaGenesis: ""

    // ---- the outcome, and the reference it describes ----------------------
    //
    // **An outcome is meaningless without the reference it belongs to, so the
    // two are stored together and never separately.** This is the fix for a
    // defect found in review, and the shape matters more than the fix:
    //
    // `stoaAddress` is a BINDING — `Main.qml` ships one reused `JoinScreen` and
    // rebinds it whenever a reference is previewed. The outcome used to be
    // independent mutable state (`joinState`, `foundingTitle`, `failure`), so
    // previewing a second reference moved the address and left the outcome
    // behind. Measured on the shipped code: paste a legitimate reference, join
    // it, then paste an attacker's — their address on screen, `joinState` still
    // `joined`, the joined panel claiming this machine had started collecting
    // their records, the join button gone, and `join_stoa` called once, for the
    // other Stoa. No verification had occurred at all, and the screen read as
    // done. That is strictly worse than the residual risk the spec analyses.
    //
    // A `reset()` called from every entry point would have fixed the symptom and
    // left the shape that produced it: a reset has to be REMEMBERED, at every
    // present and future caller, and the one place it is forgotten is a screen
    // making a claim about the wrong Stoa. Tying the outcome to its subject
    // makes the invariant hold by construction — a stale outcome is not merely
    // unlikely, it is unrepresentable, because the derived state below asks
    // "whose outcome is this?" and gets an answer.
    //
    // `null` means no call has been made for anything. Otherwise:
    //   { stoa, genesis, state, failure, foundingTitle }
    // where `state` is "joining" | "joined" | "failed".
    property var outcome: null

    // The outcome, but only when it belongs to the reference now on screen.
    //
    // Every reader goes through this rather than through `outcome` directly, so
    // a leftover from another reference is invisible to the whole screen at
    // once rather than at each site that remembered to check.
    readonly property var currentOutcome:
        (screen.outcome !== null
         && screen.outcome.stoa === screen.stoaAddress
         && screen.outcome.genesis === screen.stoaGenesis)
            ? screen.outcome : null

    // The founding title, where one is known. Today nothing resolves a title
    // from a genesis record inside the view, so this is "" until a join
    // succeeds and the reply names it — which is why the founding-title panel is
    // conditional rather than always filled.
    //
    // Derived, not assigned: it can only ever be the title the core returned for
    // THIS reference. A title carried over from another Stoa would caption an
    // untrusted address with a name the user already trusts — the exact
    // impersonation the lookalike requirement exists to expose, delivered by the
    // view itself, and invisible to that requirement because the two titles
    // being compared would be the same string from the same source.
    readonly property string foundingTitle:
        screen.currentOutcome !== null ? screen.currentOutcome.foundingTitle : ""

    // A resolved CURRENT title, from a moderator-signed metadata op. Nothing
    // supplies one and nothing on this build can: `stoa-metadata` says plainly
    // that resolution is not implemented and that metadata ops accumulate
    // unread. The property exists so the panel arrives WITH the value rather
    // than before it — filling this position with the founding value would
    // assert that no moderator has renamed this Stoa, which is a fact no peer
    // here has checked and which is false for every Stoa that has been renamed.
    property string currentTitle: ""

    // The Stoas this peer already holds, for the lookalike comparison below.
    // `[]` when the listing failed, which makes a missed lookalike the failure
    // mode rather than an invented one.
    property var heldStoas: []

    // ---- join state -----------------------------------------------------
    //
    // One string, not a set of booleans, and on this screen that matters more
    // than on the feed: the states include TWO failures that must not render
    // alike. A `malformed` that is a distinct value cannot reach the branch
    // `failed` renders through; an `isMalformed`/`hasError` pair can, and the
    // bug would be a missing `&& !`.
    //
    //   "previewing"  shown, nothing called FOR THIS REFERENCE
    //   "joining"     the call is out
    //   "joined"      the core answered successfully
    //   "failed"      the core refused; `failure` is its words
    //
    // **Derived from `currentOutcome`, never assigned.** A reference with no
    // outcome of its own is `previewing` whatever happened to any other
    // reference — which is what makes "a join is reported from the core's reply,
    // never assumed" true of the second preview as well as the first.
    readonly property string joinState:
        screen.currentOutcome !== null ? screen.currentOutcome.state : "previewing"

    // Likewise: a refusal describes the reference it was returned for and
    // nothing else. A user reading "the record you were sent is wrong" about a
    // reference the core has never seen is being told to distrust their sender
    // on no evidence.
    readonly property string failure:
        screen.currentOutcome !== null ? screen.currentOutcome.failure : ""

    property ClipboardSink clipboard: null

    signal joined(string stoa, string foundingTitle, string genesis)
    signal cancelled()

    // Stoas already held that present THIS title at a DIFFERENT address.
    //
    // This is the concrete case the whole title-is-not-an-identifier rule exists
    // for, and it is what an impersonating Stoa produces on purpose. A reader who
    // holds *Nym Research* and is handed a second *Nym Research* is exactly the
    // reader who needs to be shown that these are two addresses.
    //
    // **Comparing TITLES here is not the inference the idempotence rule
    // forbids**, and the two read alike enough to be worth separating. This is a
    // rendering decision made BEFORE the user acts. Comparing ADDRESSES to
    // decide whether a completed join was new would be a claim about what the
    // core did — which the reply deliberately does not answer — and `join()`
    // never reads this property or `heldStoas` for that reason.
    //
    // Note this now depends on the DERIVED `foundingTitle`, so a lookalike can
    // only be reported against a title the core returned for the reference on
    // screen. Previously a carried-over title could suppress the comparison
    // entirely by making both sides equal.
    readonly property var lookalikes: {
        var out = []
        if (screen.foundingTitle === "")
            return out
        for (var i = 0; i < screen.heldStoas.length; i++) {
            var held = screen.heldStoas[i]
            if (!held || typeof held.stoa !== "string")
                continue
            if (held.foundingTitle === screen.foundingTitle && held.stoa !== screen.stoaAddress)
                out.push(held)
        }
        return out
    }

    // The one action that joins anything. Nothing else on this screen calls it,
    // and nothing calls it on load — a preview that joined would enrol a user in
    // a Stoa they never chose, which is the harm the preview exists to prevent.
    function join() {
        // The reference is captured up front and every write below names it.
        // Nothing here reads `screen.stoaAddress` a second time: the property is
        // a binding on `Main.qml`'s `previewing`, so re-reading it after the
        // call would be reading whatever is on screen THEN rather than what was
        // submitted — the same class of confusion this whole shape prevents.
        var stoa = screen.stoaAddress
        var genesis = screen.stoaGenesis

        screen.outcome = {
            stoa: stoa, genesis: genesis,
            state: "joining", failure: "", foundingTitle: ""
        }

        var reply = Core.joinStoa(stoa, genesis)

        // Success is reported from the REPLY and never from having dispatched
        // the call. A screen that navigated onward on dispatch would show a Stoa
        // the user is not recorded as being in, and the discrepancy would survive
        // a restart: membership is what the core retained, not what was
        // displayed.
        if (!reply.ok) {
            screen.outcome = {
                stoa: stoa, genesis: genesis,
                state: "failed", failure: reply.error, foundingTitle: ""
            }
            return
        }

        // **Joining a Stoa already held is SUCCESS.** The core reports the same
        // reply either way, deliberately — a pasted address is exactly the input
        // a user supplies twice. The reply does not say whether the join was
        // new, and nothing here infers it: not from `heldStoas`, not from
        // `lookalikes`, not from a listing fetched earlier. A repeat is not an
        // error, a warning, or a collision.
        //
        // The title comes from THIS reply and is stored against THIS reference,
        // so it cannot outlive the Stoa it describes.
        var title = typeof reply.value.foundingTitle === "string"
            ? reply.value.foundingTitle : ""
        screen.outcome = {
            stoa: stoa, genesis: genesis,
            state: "joined", failure: "", foundingTitle: title
        }
        screen.joined(stoa, title, genesis)
    }

    // ---- the address, which is the subject -------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: 20

        Identicon {
            address: screen.stoaAddress
            size: 74
        }

        ColumnLayout {
            spacing: 6
            Layout.fillWidth: true

            Text {
                // copy.json `join.eyebrow`
                text: "YOU ARE ABOUT TO JOIN THIS ADDRESS"
                font: Theme.label
                color: Theme.accent
                textFormat: Text.PlainText
            }

            // IN FULL, not abbreviated. This is the screen where a decision is
            // being made about which Stoa this is, and the 8-8-6 form is a
            // recognition aid rather than a basis for a decision.
            AddressLabel {
                objectName: "previewAddress"
                address: screen.stoaAddress
                full: true
                Layout.fillWidth: true
                onCopyRequested: {
                    // What is copied is the SHAREABLE reference, not the bare
                    // address: an address alone cannot be joined by whoever
                    // receives it.
                    var text = StoaReference.shareText(screen.stoaAddress, screen.stoaGenesis)
                    if (text !== "" && screen.clipboard)
                        screen.clipboard.copy(text)
                }
            }

            // **The bundle's `join.note` is narrowed here, and the narrowing is
            // the whole point.** Its first half — "pasting it is itself the
            // verification" — is accurate about the mechanism and overreaching
            // about what it buys. The check is a hash comparison between TWO
            // INPUTS THE USER SUPPLIED and consults nothing else: no registry, no
            // peer, no network. So it proves exactly one thing, and a reader who
            // pasted a hostile address and saw a verified record has verified the
            // attacker's record against the attacker's address, perfectly
            // successfully. Telling that reader they have finished checking is
            // the failure this copy is written against.
            Text {
                objectName: "addressNote"
                text: "Shown in full, because it is the only thing here worth trusting. "
                    + "The address is a hash of the founding record, so it proves that "
                    + "the record shown below is the one this address names — and "
                    + "nothing more. It is not checked against any registry, peer or "
                    + "third party, and nothing here says this is the address you were "
                    + "meant to receive. Everything below the address is unverified."
                font: Theme.note
                color: Theme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: 1.5
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }
    }

    // ---- titles -----------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: 0

        // The founding title, labelled as FOUNDING. Rendering it unlabelled
        // would discard at the last step the distinction the core puts on the
        // wire by naming the field `foundingTitle` rather than `title`.
        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            implicitHeight: foundingBody.implicitHeight + 28
            color: "transparent"
            border.width: Theme.hairline
            border.color: Theme.ink

            ColumnLayout {
                id: foundingBody
                anchors.fill: parent
                anchors.margins: 14
                spacing: 5

                Text {
                    // copy.json `join.foundingTitle`
                    text: "FOUNDING TITLE — FIXED FOREVER"
                    font: Theme.label
                    color: Theme.inkMuted
                    textFormat: Text.PlainText
                }

                Text {
                    objectName: "foundingTitleText"
                    text: screen.foundingTitle
                    font: Theme.body
                    color: Theme.ink
                    // Freely chosen by whoever created the Stoa, matched against
                    // nothing, and carrying whatever characters they typed.
                    // Never markup.
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        // The current title's position, **reserved and empty**.
        //
        // The mockup fills this panel and it is exactly the right idea — the
        // distinction is why it is worth designing around. But nothing resolves
        // the moderator-signed metadata op that carries a current title, so this
        // renders only when a resolved one is actually supplied. A panel
        // captioned "current title" holding the founding value would assert that
        // nobody has renamed this Stoa, which is the one thing no peer on this
        // build has checked.
        Rectangle {
            visible: screen.currentTitle !== ""
            Layout.fillWidth: true
            Layout.fillHeight: true
            implicitHeight: currentBody.implicitHeight + 28
            color: "transparent"
            border.width: Theme.hairline
            border.color: Theme.ink

            ColumnLayout {
                id: currentBody
                anchors.fill: parent
                anchors.margins: 14
                spacing: 5

                Text {
                    // copy.json `join.currentTitle`
                    text: "CURRENT TITLE — CHOSEN BY A MODERATOR, CHANGEABLE"
                    font: Theme.label
                    color: Theme.accent
                    textFormat: Text.PlainText
                }

                Text {
                    objectName: "currentTitleText"
                    text: screen.currentTitle
                    font: Theme.body
                    color: Theme.ink
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }
    }

    // ---- a Stoa already held presenting the same title --------------------

    Rectangle {
        objectName: "lookalikePanel"
        visible: screen.lookalikes.length > 0
        Layout.fillWidth: true
        implicitHeight: lookalikeBody.implicitHeight + 28
        color: Theme.field
        border.width: Theme.hairline
        border.color: Theme.rule2

        ColumnLayout {
            id: lookalikeBody
            anchors.fill: parent
            anchors.margins: 14
            spacing: 8

            Text {
                // copy.json `join.collision` — the word is the bundle's, and it
                // is a description of what the reader is looking at rather than
                // a conflict to resolve. These are two Stoas; joining the second
                // does nothing to the first.
                text: "A STOA YOU ALREADY HOLD PRESENTS THE SAME TITLE"
                font: Theme.label
                color: Theme.inkMuted
                textFormat: Text.PlainText
            }

            Repeater {
                model: screen.lookalikes

                delegate: RowLayout {
                    id: lookalikeRow
                    required property var modelData
                    readonly property string heldStoa:
                        typeof modelData.stoa === "string" ? modelData.stoa : ""

                    Layout.fillWidth: true
                    spacing: 16

                    Identicon {
                        address: lookalikeRow.heldStoa
                        size: 30
                    }

                    Text {
                        text: typeof lookalikeRow.modelData.foundingTitle === "string"
                            ? lookalikeRow.modelData.foundingTitle : ""
                        font: Theme.body
                        color: Theme.ink
                        textFormat: Text.PlainText
                    }

                    // Both addresses on screen, which is the entire content of
                    // the panel: the titles are identical, so the addresses are
                    // the only thing that tells the reader these are two Stoas.
                    AddressLabel { address: lookalikeRow.heldStoa }

                    Item { Layout.fillWidth: true }

                    Text {
                        // copy.json `join.notThisOne`
                        text: "not this one"
                        font: Theme.note
                        color: Theme.accent
                        textFormat: Text.PlainText
                    }
                }
            }
        }
    }

    // ---- the join refused -------------------------------------------------
    //
    // The core's own refusal, rendered as it came. A refusal naming what was
    // wrong is the difference between a user who knows the record they were sent
    // is wrong and a user who thinks the app is broken.
    //
    // **There is deliberately no retry button here.** Where a record does not
    // verify against its address, somebody handed over a record that is not the
    // one that address names, and pressing again produces the same refusal —
    // a retry would invite the user to keep pressing until they mistook a
    // permanent answer for a transient fault.
    ColumnLayout {
        objectName: "joinFailurePanel"
        visible: screen.joinState === "failed"
        Layout.fillWidth: true
        spacing: 6

        Text {
            text: "This was not joined."
            font: Theme.heading
            color: Theme.accent
            textFormat: Text.PlainText
        }

        Text {
            objectName: "joinFailureText"
            text: screen.failure
            font: Theme.address
            color: Theme.ink
            wrapMode: Text.WrapAnywhere
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    ColumnLayout {
        objectName: "joinedPanel"
        visible: screen.joinState === "joined"
        Layout.fillWidth: true
        spacing: 6

        Text {
            text: "Joined."
            font: Theme.heading
            color: Theme.ink
            textFormat: Text.PlainText
        }

        Text {
            // What joining actually does, and nothing broader. It does not
            // create an identity for this Stoa alone — one key signs in every
            // Stoa in this release — it enrols nobody in a list any peer can
            // see, and it says nothing about moderating anything.
            text: "This machine has started collecting this Stoa's records. Nobody was "
                + "notified and no peer can see that you did this."
            font: Theme.bodySmall
            color: Theme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.55
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // ---- the actions ------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: 16

        FlatButton {
            objectName: "joinButton"
            // copy.json `join.join`. The ONLY thing on this screen that calls
            // the core's join.
            text: "Join this address"
            kind: "primary"
            visible: screen.joinState !== "joined"
            onClicked: screen.join()
        }

        FlatButton {
            objectName: "copyReferenceButton"
            // The bundle says `Copy the address`; what is copied is the
            // reference, because an address alone is not joinable by whoever
            // receives it. Same decision as the share on the list.
            text: "Copy a shareable reference"
            kind: "secondary"
            visible: StoaReference.shareText(screen.stoaAddress, screen.stoaGenesis) !== ""
            onClicked: {
                var text = StoaReference.shareText(screen.stoaAddress, screen.stoaGenesis)
                if (text !== "" && screen.clipboard)
                    screen.clipboard.copy(text)
            }
        }

        FlatButton {
            // copy.json `join.cancel`
            text: "Cancel"
            kind: "secondary"
            onClicked: screen.cancelled()
        }

        Item { Layout.fillWidth: true }
    }

    apparatus: [
        MarginNote {
            label: "ON WHAT THE ADDRESS PROVES"
            body: "That the record shown is the one this address names. That is a comparison between two things you supplied, and it consults nothing else — not a registry, not a peer, not the network. It does not establish that this is the address you were meant to receive."
        },
        MarginNote {
            label: "ON THE TITLE"
            body: "Decoration. Whoever created the Stoa chose it freely, it is not unique, it is matched against nothing, and it can be picked to resemble another Stoa's."
        },
        MarginNote {
            label: "ON WHERE THIS CAME FROM"
            body: "You opened this from a reference somebody handed you. Nothing was joined by opening it."
        },
        MarginNote {
            label: "ON WHAT JOINING DOES"
            caveat: false
            // The bundle's version of this note ends "and generates you an
            // identity for it alone". Per-Stoa identity is built in the core and
            // NOT switched on in this release — one key signs in every Stoa — so
            // that sentence would tell a user they have an unlinkability
            // property they do not have. It is the one false claim on these
            // screens that could actually harm somebody, and it is dropped.
            body: "It starts collecting this Stoa's records on this machine. There is no membership list, nobody is notified, and no peer can be stopped from publishing here."
        }
    ]
}
