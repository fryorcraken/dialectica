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
    // `stoaAddress` is a BINDING — `Main.qml` ships one reused `DJoinScreen` and
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

    // ---- the lookup, and the reference it describes -----------------------
    //
    // `getStoa` answers what this Stoa is called for a reference this peer has
    // not joined. It is asked as soon as a reference is on screen, without the
    // user acting, because asking joins nothing.
    //
    // **Stored with the reference it was made for, exactly as the join outcome
    // is**, and for the same reason: a title answered for one reference and
    // rendered over the next lends a name the user may trust to an address they
    // should not. `null` means nothing has been asked. Otherwise:
    //   { stoa, genesis, ok, isGenesisFallback, title, description, error }
    // with the fields `Core.stoaMetadataFrom` fills for `ok` or for a failure.
    property var lookup: null

    readonly property var currentLookup:
        (screen.lookup !== null
         && screen.lookup.stoa === screen.stoaAddress
         && screen.lookup.genesis === screen.stoaGenesis)
            ? screen.lookup : null

    // Whether the reply says this peer holds no moderator-set title, so its
    // `title` is the founding one.
    readonly property bool lookupFellBack:
        screen.currentLookup !== null && screen.currentLookup.ok
        && screen.currentLookup.isGenesisFallback

    // The failure a lookup ended in, as a reason to render, or "".
    readonly property string lookupFailure:
        screen.currentLookup !== null && !screen.currentLookup.ok
            ? screen.currentLookup.error : ""

    // Ask about the reference on screen. Never with half of one: an empty
    // address or record is how "no reference" reaches this screen, and a paste
    // that is not a reference never gets here at all, because
    // `DStoaReference.parse` refuses it on the list.
    //
    // **Synchronous, and called on every change of either half.** `Main.qml`
    // binds the two strings separately, so switching straight from one
    // reference to another asks once for the new address with the old record.
    // The core refuses that pair, and the answer is stored against a pair that
    // is never on screen, so nothing renders it. The shipped navigator passes
    // through no preview between two pastes, so it never does this.
    // design.md decision 7.
    function lookUp() {
        var stoa = screen.stoaAddress
        var genesis = screen.stoaGenesis
        if (stoa === "" || genesis === "")
            return
        var answer = Core.stoaMetadataFrom(Core.getStoa(stoa, genesis))
        answer.stoa = stoa
        answer.genesis = genesis
        screen.lookup = answer
    }

    // Whether construction has finished. The two change handlers wait for it,
    // so a screen built with both halves already set asks once, from
    // `Component.onCompleted`, rather than once per property it was given.
    property bool constructed: false
    Component.onCompleted: {
        screen.constructed = true
        screen.lookUp()
    }
    onStoaAddressChanged: if (screen.constructed) screen.lookUp()
    onStoaGenesisChanged: if (screen.constructed) screen.lookUp()

    // The founding title, where one is available for this reference.
    //
    // **Two calls can answer one, and no other can.** A successful join
    // reply's `foundingTitle`, and a `getStoa` reply whose `isGenesisFallback`
    // is `true`: that resolution fell back to the founding values, so its
    // `title` IS the founding title. A reply whose flag is `false` carries a
    // current title and no founding title beside it, so for that Stoa none is
    // available before a join. The title is inside the genesis record the user
    // pasted, and decoding it here would be a second implementation of the
    // core's encoding, the thing the core/UI split exists to prevent.
    //
    // The join's is preferred where it has one. The two can only disagree if
    // the core answers the same record two ways.
    //
    // **A blank title is not an available founding title**, whichever reply
    // carried it. `Core.stoaMetadataFrom` already refuses a lookup that carries
    // one; the join reply's is tested here. A caption over a blank value would
    // tell the reader this Stoa's founding title is blank, which no valid
    // Stoa's is.
    //
    // Derived, not assigned: it can only ever be a title the core returned for
    // THIS reference. A title carried over from another Stoa would caption an
    // untrusted address with a name the user already trusts — the exact
    // impersonation the lookalike requirement exists to expose, delivered by the
    // view itself, and invisible to that requirement because the two titles
    // being compared would be the same string from the same source.
    readonly property string foundingTitle: {
        var joined = screen.currentOutcome !== null
            ? screen.currentOutcome.foundingTitle : ""
        if (typeof joined === "string" && !Core.isBlankTitle(joined))
            return joined
        return screen.lookupFellBack ? screen.currentLookup.title : ""
    }

    // Whether a founding title is available for this reference.
    //
    // The panel and the no-title note are two renderings of this one fact, so
    // they cannot disagree about which is showing — an `implicitHeight`
    // computed over a hidden panel and a note bound to a different expression
    // is how a screen ends up with both or neither.
    readonly property bool titleKnown: screen.foundingTitle !== ""

    // A resolved CURRENT title: the `title` of a `getStoa` reply whose
    // `isGenesisFallback` is `false`, which only a moderator-signed metadata op
    // the core found binding can produce. "" otherwise.
    //
    // **Derived from the lookup, never assigned**, and never filled with the
    // founding value. A current-title panel holding the founding title would
    // assert that no moderator has renamed this Stoa. A fallback reply does not
    // establish that: it says only that this machine holds no rename.
    readonly property string currentTitle:
        screen.currentLookup !== null && screen.currentLookup.ok
        && !screen.currentLookup.isGenesisFallback
            ? screen.currentLookup.title : ""

    // The description a moderator set with that current title, or "".
    //
    // Read only from a non-fallback reply. A fallback reply's is always ""
    // (the genesis record has none), and reading it there would render a
    // value no moderator set.
    //
    // From the lookup itself and not through `currentTitle`, so the two cannot
    // be read from different sources.
    readonly property string currentDescription:
        screen.currentLookup !== null && screen.currentLookup.ok
        && !screen.currentLookup.isGenesisFallback
            ? screen.currentLookup.description : ""

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

    property DClipboardSink clipboard: null

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
    // Note this depends on the DERIVED `foundingTitle`, so a lookalike can
    // only be reported against a title the core returned for the reference on
    // screen. A carried-over title could otherwise suppress the comparison
    // entirely by making both sides equal.
    //
    // **Over FOUNDING titles only, and never over `currentTitle`.** That is the
    // owner's scope ruling on #143, not an omission. A Stoa founded under any
    // title and renamed to match one the user holds gets a non-fallback reply,
    // so no founding title is available and the comparison does not run. For
    // now the creator's key, the identicon and whether the user already joined
    // it are the signal; on-chain Stoas, after 0.1.0, are what will fix a
    // Stoa's identity. `titleUnknownNote` says the comparison has not been
    // made, so its silence is not read as a clean result.
    //
    // So it runs at preview time for a Stoa the core answers as a fallback, and
    // otherwise only after a join. That is late rather than absent.
    //
    // **A blank title matches nothing, and that needs no check here.**
    // `foundingTitle` is never blank, and two equal strings are blank or not
    // together, so a held Stoa whose title is blank can never equal it. A guard
    // for it would have no test that could fail. design.md decision 11.
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
                font: DTheme.label
                color: DTheme.accent
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
                    var text = DStoaReference.shareText(screen.stoaAddress, screen.stoaGenesis)
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
                font: DTheme.note
                color: DTheme.inkSoft
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
        //
        // **Conditional on one being available**: after a join, or from a
        // lookup that fell back. An unconditional caption reading FOUNDING
        // TITLE — FIXED FOREVER over an empty value tells a reader this Stoa's
        // founding title is blank. No valid Stoa's is, so the caption would be
        // asserting something false on the screen where the reader decides
        // whether to trust an address.
        Rectangle {
            objectName: "foundingTitlePanel"
            visible: screen.titleKnown
            Layout.fillWidth: true
            Layout.fillHeight: true
            implicitHeight: foundingBody.implicitHeight + 28
            color: "transparent"
            border.width: DTheme.hairline
            border.color: DTheme.ink

            ColumnLayout {
                id: foundingBody
                anchors.fill: parent
                anchors.margins: 14
                spacing: 5

                Text {
                    // copy.json `join.foundingTitle`
                    text: "FOUNDING TITLE — FIXED FOREVER"
                    font: DTheme.label
                    color: DTheme.inkMuted
                    textFormat: Text.PlainText
                }

                Text {
                    objectName: "foundingTitleText"
                    text: screen.foundingTitle
                    font: DTheme.body
                    color: DTheme.ink
                    // Freely chosen by whoever created the Stoa, matched against
                    // nothing, and carrying whatever characters they typed.
                    // Never markup.
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }

        // The current title's position, filled only from a lookup that did NOT
        // fall back: the core found a moderator-signed rename that binds.
        //
        // Never from a fallback reply, whose `title` is the founding one. A
        // panel captioned "current title" holding the founding value would
        // assert that nobody has renamed this Stoa, and a fallback establishes
        // only that this machine holds no rename.
        //
        // The description goes here too, where the reply carried one. A
        // moderator sets it in the same op as the current title, and the
        // bundle's preview has no description position of its own. Its caption
        // attributes it to that same op.
        Rectangle {
            objectName: "currentTitlePanel"
            visible: screen.currentTitle !== ""
            Layout.fillWidth: true
            Layout.fillHeight: true
            implicitHeight: currentBody.implicitHeight + 28
            color: "transparent"
            border.width: DTheme.hairline
            border.color: DTheme.ink

            ColumnLayout {
                id: currentBody
                anchors.fill: parent
                anchors.margins: 14
                spacing: 5

                Text {
                    // copy.json `join.currentTitle`
                    text: "CURRENT TITLE — CHOSEN BY A MODERATOR, CHANGEABLE"
                    font: DTheme.label
                    color: DTheme.accent
                    textFormat: Text.PlainText
                }

                Text {
                    objectName: "currentTitleText"
                    text: screen.currentTitle
                    font: DTheme.body
                    color: DTheme.ink
                    // Chosen by whoever holds the creator's key, and carrying
                    // whatever characters they typed. Never markup.
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }

                // No caption over an empty description: an empty one is a
                // value the moderator set, and a caption over nothing would
                // read as a description that failed to load.
                Text {
                    objectName: "currentDescriptionCaption"
                    visible: screen.currentDescription !== ""
                    text: "DESCRIPTION — SET WITH THE CURRENT TITLE"
                    font: DTheme.label
                    color: DTheme.inkMuted
                    textFormat: Text.PlainText
                }

                Text {
                    objectName: "currentDescriptionText"
                    visible: screen.currentDescription !== ""
                    text: screen.currentDescription
                    font: DTheme.bodySmall
                    color: DTheme.inkSoft
                    textFormat: Text.PlainText
                    wrapMode: Text.WordWrap
                    Layout.fillWidth: true
                }
            }
        }
    }

    // ---- what a fallback does and does not establish ----------------------
    //
    // A fallback reply's `title` fills the founding panel above. **This says
    // what that means**, because the founding title shown alone would read as
    // "this Stoa has not been renamed". A fallback establishes only that this
    // machine holds no binding rename: one it has not received looks exactly
    // the same. That is the issue's "this may be stale", surfaced rather than
    // only used to pick a panel.
    Text {
        objectName: "fallbackNote"
        visible: screen.lookupFellBack
        text: "This machine holds no title set by a moderator for this Stoa. A rename "
            + "it has not received would look exactly like this, so the founding "
            + "title may since have been changed."
        font: DTheme.note
        color: DTheme.inkSoft
        wrapMode: Text.WordWrap
        lineHeight: 1.5
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- a lookup that failed ---------------------------------------------
    //
    // The core's refusal, or the view's own reason for a reply it could not
    // place, rendered as it came. **It is not a refused join, and nothing here
    // says it is.** The join is a separate call the core answers for itself,
    // and the view cannot tell the causes apart without parsing the core's
    // text: a record that does not match its address would fail the join too,
    // and an op store that cannot be read would not. So the join affordance
    // stays, and the join's own reply decides. design.md decision 10.
    ColumnLayout {
        objectName: "lookupFailurePanel"
        visible: screen.lookupFailure !== ""
        Layout.fillWidth: true
        spacing: 6

        Text {
            text: "What this Stoa is called could not be looked up."
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText
        }

        Text {
            objectName: "lookupFailureText"
            text: screen.lookupFailure
            font: DTheme.address
            color: DTheme.ink
            wrapMode: Text.WrapAnywhere
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // ---- when no founding title is available here ------------------------
    //
    // **The screen states the absence of a check rather than leaving the check's
    // silence to be read as its result.** Where no founding title is available,
    // two things follow and both matter to the decision the user is about to
    // make:
    //
    //   - no founding title is shown. An empty panel captioned FOUNDING TITLE
    //     would read as a blank title, which no valid Stoa has.
    //   - the same-title comparison has not run, because it is over founding
    //     titles. A reader who sees no lookalike warning and infers there is no
    //     lookalike has been misled by an unrun check — the impersonation this
    //     whole screen is written against, arriving through the defence rather
    //     than around it.
    //
    // This happens before a join whenever the lookup did not fall back: it
    // failed, it has not answered, or it answered a CURRENT title. The copy
    // must therefore not say that nothing here knows what the Stoa is called,
    // because a current title may be on screen above. After a join succeeds,
    // the only way here is a join reply whose founding title was blank, and
    // "joining would supply one" would then be false. Hence two texts.
    ColumnLayout {
        objectName: "titleUnknownNote"
        visible: !screen.titleKnown
        Layout.fillWidth: true
        spacing: 6

        Text {
            text: "NO FOUNDING TITLE IS AVAILABLE HERE"
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText
        }

        Text {
            objectName: "titleUnknownText"
            text: screen.joinState === "joined"
                ? "The join succeeded, but its reply carried no usable founding "
                  + "title, so none is shown and the same-title comparison against "
                  + "the Stoas you already hold has not been made. The address "
                  + "above is the only thing that can tell you which Stoa this is."
                : "The founding title is inside the record you pasted, and this "
                  + "screen does not decode it; joining is what would supply it. "
                  + "So no founding title is shown, and the same-title comparison "
                  + "against the Stoas you already hold has not been made. If you "
                  + "are expecting this to be a Stoa you have seen before, the "
                  + "address above is the only thing that can tell you."
            font: DTheme.note
            color: DTheme.inkSoft
            wrapMode: Text.WordWrap
            lineHeight: 1.5
            textFormat: Text.PlainText
            Layout.fillWidth: true
        }
    }

    // ---- a Stoa already held presenting the same title --------------------

    Rectangle {
        objectName: "lookalikePanel"
        visible: screen.lookalikes.length > 0
        Layout.fillWidth: true
        implicitHeight: lookalikeBody.implicitHeight + 28
        color: DTheme.field
        border.width: DTheme.hairline
        border.color: DTheme.rule2

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
                font: DTheme.label
                color: DTheme.inkMuted
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
                        font: DTheme.body
                        color: DTheme.ink
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
                        font: DTheme.note
                        color: DTheme.accent
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
            font: DTheme.heading
            color: DTheme.accent
            textFormat: Text.PlainText
        }

        Text {
            objectName: "joinFailureText"
            text: screen.failure
            font: DTheme.address
            color: DTheme.ink
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
            font: DTheme.heading
            color: DTheme.ink
            textFormat: Text.PlainText
        }

        Text {
            // What joining actually does, and nothing broader. It does not
            // create an identity for this Stoa alone — one key signs in every
            // Stoa in this release — it enrols nobody in a list any peer can
            // see, and it says nothing about moderating anything.
            text: "This machine has started collecting this Stoa's records. Nobody was "
                + "notified and no peer can see that you did this."
            font: DTheme.bodySmall
            color: DTheme.inkSoft
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
            visible: DStoaReference.shareText(screen.stoaAddress, screen.stoaGenesis) !== ""
            onClicked: {
                var text = DStoaReference.shareText(screen.stoaAddress, screen.stoaGenesis)
                if (text !== "" && screen.clipboard)
                    screen.clipboard.copy(text)
            }
        }

        FlatButton {
            // The way back to the list, before a join and after one
            // (`view-navigation`). Named so the end-to-end suite can press
            // it: `seeded-join.yaml` leaves a successful join through here.
            objectName: "joinCancelButton"
            // copy.json `join.cancel`
            text: "Cancel"
            kind: "secondary"
            onClicked: screen.cancelled()
        }

        Item { Layout.fillWidth: true }
    }
}
