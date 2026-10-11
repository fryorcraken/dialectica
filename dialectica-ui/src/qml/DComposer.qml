import QtQuick
import QtQuick.Layouts

// The open-gate composer: a draft, what is wrong with it, and what happened to
// it. One component for a post and for a reply — `kind` decides which core
// method is called and nothing else about the behaviour, because every rule the
// spec states about a refusal is the same rule for both.
//
// **This component renders nothing when the gate is shut.** It does not render a
// disabled field, a read-only one, or one that accepts text and refuses to
// submit — a box the user can type into and not send loses what they wrote. The
// caller gates it: `FeedScreen` gives it `visible:` from the probe. The closed
// state is a different thing on screen, not this thing greyed out.
ColumnLayout {
    id: root

    // "post" or "reply", deciding exactly one thing: which core method
    // `submit()` calls. Every rule the spec states about a refusal, a draft or
    // a byte count is the same rule for both, which is why one component serves
    // both modes rather than two components sharing a base.
    property string kind: "post"

    property string stoaAddress: ""

    // The op a reply answers. **Read only when `kind` is "reply"**, and an
    // earlier comment here claimed more than that — it said "a post must not
    // have one", stated as an invariant that nothing established. A
    // `DComposer { kind: "post"; parentOp: "deadbeef" }` was accepted and
    // silently dropped the parent: no warning, no refusal, no test. So the
    // comment described an enforcement that did not exist, and a call site
    // written against it would have been wrong.
    //
    // `replyParent` below is what makes it true rather than merely stated: a
    // parent belonging to a post is not "ignored", it is not reachable.
    property string parentOp: ""

    // The parent that actually reaches core, which is "" for a post whatever
    // `parentOp` holds. One expression rather than a rule to remember at each
    // call site, so a stray parent on a post is unrepresentable downstream
    // instead of dropped somewhere the caller cannot see.
    readonly property string replyParent: root.kind === "reply" ? root.parentOp : ""

    // The draft for the target this composer would publish to, and **the field
    // is the single place that one lives on screen**.
    //
    // An alias rather than a separate property with a two-way binding: `text:
    // root.draft` plus `onTextChanged: root.draft = text` looks symmetric and is
    // not — the first keystroke breaks the declarative binding, after which a
    // programmatic write to `draft` updates nothing on screen. That failure is
    // silent and looks exactly like the clear-on-success not working.
    //
    // Nothing in this component ever alters what the user typed: no trim, no
    // normalise, no strip anywhere on the path from here to `Core.publishPost`.
    // An op is signed over its bytes, so a composer that altered a draft would
    // publish, permanently and under the user's signature, something they did
    // not write. The field is written in two places only, and neither edits
    // text: `showDraftOfTarget()` puts back, whole, what was entered for the
    // target now pointed at, and `clearDraftOf()` empties it after a store.
    property alias draft: field.text

    // ---- a draft belongs to its target -----------------------------------
    //
    // **One composer serves every target of its kind.** The feed mounts one and
    // re-points it at whichever Stoa is open; the thread screen does the same
    // for whichever thread. So the field outlives the target it was typed for,
    // and until this section existed its text simply stayed: a post written for
    // one Stoa was in the field, and one press from being published, in the
    // next Stoa opened (`composer-view`, "A draft belongs to the target it was
    // entered for"; the change's `design.md` has the alternatives).
    //
    // The target is what `submit()` sends, and nothing coarser: which method,
    // which Stoa and which parent. `JSON.stringify` of the three is the key
    // because it cannot map two targets onto one string, whatever characters a
    // peer-supplied op id holds; joining them with a separator could.
    readonly property string targetKey:
        JSON.stringify([root.kind, root.stoaAddress, root.replyParent])

    // Every unsubmitted draft this composer has been given, by target key.
    //
    // **In this component's memory and nowhere else**, so it ends with the view
    // and reaches core only as the body of a publish. No bound on how many it
    // holds. It is mutated in place, which QML does not notice, and that is
    // deliberate: nothing binds to it. The field is what a binding reads.
    property var heldDrafts: ({})

    // "" for a target nothing is held for. The lookup answers `undefined`
    // there, which the field, a string, cannot be assigned.
    function heldDraft(key) {
        var held = root.heldDrafts[key]
        return typeof held === "string" ? held : ""
    }

    // An emptied draft is dropped and not kept as "": the two would read back
    // the same, and dropping keeps the map to what is actually unsubmitted.
    function holdDraft(key, text) {
        if (text === "")
            delete root.heldDrafts[key]
        else
            root.heldDrafts[key] = text
    }

    // The composer was pointed at a different target: the field takes what is
    // held for that one, which is nothing unless text was entered for it.
    //
    // **This handler is the whole of what keeps one target's text out of
    // another's publish.** `submit()` sends the field's text to the target
    // current when it runs, so the field must never hold another target's text
    // once the target has changed, and this runs on the change itself, before
    // anything else can act. Without it, the tests in `tst_draft_targets.qml`
    // that move to another target and then read or submit what its field holds
    // are red. Not every test that crosses targets is: one that enters text
    // again after the move, or comes back to the only target it wrote in, finds
    // the right text in a field nobody re-filled.
    //
    // A transition passes through targets nobody is shown: `Main.qml` empties a
    // screen's address before it sets the next one. Each is handled like any
    // other, which costs a lookup and writes nothing.
    //
    // Assigning `text` also resets the field's undo history, so undo in one
    // target's field cannot bring back another's text.
    onTargetKeyChanged: root.showDraftOfTarget()

    function showDraftOfTarget() {
        field.text = root.heldDraft(root.targetKey)
    }

    // Clears the draft of the target a publish NAMED, which is the one held
    // under `key`. Not "whatever the field holds": those are the same target
    // only while nothing re-points the composer between a submit and its
    // answer, and that holds today because the call into core is synchronous,
    // which is not this component's to rely on.
    function clearDraftOf(key) {
        root.holdDraft(key, "")
        if (key === root.targetKey)
            field.text = ""
    }

    // ---- the byte cap ---------------------------------------------------
    //
    // **150 KiB, and this is a copy of a number core does not expose.**
    // `dialectica-core`'s `authoring::MAX_BODY_LEN` (authoring.rs:206) is
    // `op::MAX_FIELD_LEN`, and no wire method returns it — so the view cannot
    // ask, and any value here is a second copy that can drift.
    //
    // It is one property rather than a literal at each use, so the day core does
    // expose the cap this becomes one binding. And the spec's requirement is on
    // the BEHAVIOUR (the user is told before submitting; the draft is never
    // truncated), which is what the tests assert — so a drift breaks a test
    // about the number rather than the rules built on it.
    property int bodyByteLimit: 150 * 1024

    signal published()

    // ---- the outcome, as ONE value --------------------------------------
    //
    // "" | "stored" | "existing" | "refused". See DPublishOutcome.qml for why
    // this is a string rather than a set of booleans.
    property string outcome: ""
    property string outcomeDetail: ""

    // Back to "nothing submitted", which renders no outcome. The outcome
    // belongs to the visit in which its publish was made (`composer-view`), and
    // this component cannot see a visit, so its screen calls this when one
    // begins. The draft is not touched: it belongs to its target and not to a
    // visit, and the section above is what keeps it.
    function clearOutcome() {
        root.outcome = ""
        root.outcomeDetail = ""
    }

    // ---- what the draft costs and what is hiding in it ------------------

    readonly property int draftBytes: root.utf8Length(root.draft)
    readonly property bool overLimit: root.draftBytes > root.bodyByteLimit
    readonly property int invisibleCount: root.countInvisible(root.draft)

    // Empty drafts are not submitted. Core would refuse one on its own terms;
    // offering a button that cannot do anything is the affordance equivalent of
    // a dead text field.
    readonly property bool submittable: root.draft.length > 0 && !root.overLimit

    spacing: DTheme.itemGap

    // ---- the UTF-8 byte count -------------------------------------------
    //
    // **A QML string is UTF-16, so `.length` is code units, not bytes.** A draft
    // of CJK text is three bytes per character and one unit per character, so a
    // composer counting `.length` permits a submission three times the size core
    // will accept — and the user discovers it on refusal, which is the thing the
    // spec exists to prevent.
    //
    // Rejected: `encodeURIComponent(s).replace(/%../g, "x").length`. It is the
    // one-liner everyone reaches for and it THROWS `URIError` on an unpaired
    // surrogate — which a paste or an IME can leave in a TextArea mid-edit. A
    // length function that throws is one that leaves the submit affordance in
    // whatever state it held when the exception unwound, which is a button whose
    // enabled-ness depends on what the user last pasted.
    //
    // So: walk code points. An unpaired surrogate is counted as 3, the cost of
    // the replacement character it will encode as.
    function utf8Length(s) {
        var bytes = 0
        for (var i = 0; i < s.length; i++) {
            var c = s.charCodeAt(i)
            if (c < 0x80) {
                bytes += 1
            } else if (c < 0x800) {
                bytes += 2
            } else if (c >= 0xD800 && c <= 0xDBFF) {
                // A high surrogate. Paired with a low one it is a single code
                // point costing 4 bytes; alone it is malformed and encodes as
                // the 3-byte replacement character.
                var next = i + 1 < s.length ? s.charCodeAt(i + 1) : 0
                if (next >= 0xDC00 && next <= 0xDFFF) {
                    bytes += 4
                    i += 1
                } else {
                    bytes += 3
                }
            } else {
                // Includes a lone low surrogate, which is malformed and costs
                // the same 3 bytes as the replacement character.
                bytes += 3
            }
        }
        return bytes
    }

    // ---- the invisible-character warning --------------------------------
    //
    // The same set `dialectica-core`'s `sanitise::is_invisible` removes on the
    // way OUT to a reader (sanitise.rs:141). The author is warned about them on
    // the way IN, and the text is published unchanged either way.
    //
    // **The asymmetry is the point.** Peer text is sanitised on display because
    // the reader cannot consent to what they are shown. The author's own text is
    // not sanitised, because they can — and they are the only person who can
    // still change it, which is why the warning goes to them and why it does not
    // block submission.
    //
    // NO SPEC: the spec asks the warning to name how many characters the
    // sanitiser would remove OR MARK, and this counts only the removals. The
    // marked half is a homoglyph JUDGEMENT — script ranges, a dominance rule and
    // a tie-break — and a second implementation of it in QML would give a
    // different number from core's on the same text, with nothing to notice the
    // disagreement. A set-membership test can drift too, but it drifts visibly:
    // a character is in the list or it is not. So this under-reports a mixed
    // Latin/Cyrillic draft, and that is a chosen gap rather than an oversight.
    function isInvisible(c) {
        return (c >= 0x202A && c <= 0x202E)   // bidi embeddings and overrides
            || (c >= 0x2066 && c <= 0x2069)   // bidi isolates
            || (c >= 0x200B && c <= 0x200D)   // zero-width space, non-joiner, joiner
            || c === 0x200E || c === 0x200F   // LTR and RTL marks
            || (c >= 0x2060 && c <= 0x2064)   // word joiner, invisible operators
            || c === 0x061C                   // Arabic letter mark
            || c === 0xFEFF                   // BOM, in its in-band spelling
            || (c >= 0xFFF9 && c <= 0xFFFB)   // interlinear annotation delimiters
    }

    function countInvisible(s) {
        var n = 0
        for (var i = 0; i < s.length; i++) {
            if (root.isInvisible(s.charCodeAt(i)))
                n += 1
        }
        return n
    }

    // ---- an outstanding publish ------------------------------------------
    //
    // **True from the moment a submission is made until an outcome is reported.**
    // The submit control reads it, and while it is true there is no control to
    // activate.
    //
    // # Why this exists, and what it is actually defending against
    //
    // An op's signed bytes now carry a Lamport counter that advances between two
    // publishes, so **two submissions of one draft are two distinct ops**. The
    // deduplication that used to absorb a double-tapped submit no longer does,
    // and core cannot restore it: at that layer a double tap and a person
    // deliberately posting the same line twice are identical acts producing
    // correctly-signed, correctly-ordered ops. The view is the only layer at
    // which the two are distinguishable, because only the view knows that one
    // gesture occurred.
    //
    // The cost of getting it wrong is permanent. This system has no delete, and
    // a revision replaces a post's content rather than withdrawing it — so a
    // duplicate is visible to every peer forever, and the author's only remedy
    // is to edit one into an apology.
    //
    // # BE PRECISE ABOUT WHAT THIS BUYS ON TODAY'S TRANSPORT
    //
    // `Core.call` reaches the host through `bridge.callModule`, which is
    // **synchronous**, and QML's JavaScript is single-threaded. So `submit()`
    // runs from its first line to its last within one event-handler turn, and no
    // second activation can be delivered in between: today the duplicate this
    // guards against is **already unreachable**, by the transport's shape rather
    // than by anything in this file.
    //
    // That is a reason to write the guard, not a reason to skip it. The property
    // being defended is "one gesture, one op", and it currently holds for a
    // reason this component neither chose nor controls — the day `callModule`
    // gains an asynchronous form, or a publish acquires a confirmation step, the
    // window opens and nothing would report that it had. A guard that is correct
    // before and after that change costs two bindings.
    //
    // **So the test that this disables the control is a real test, and a test
    // that "a double tap publishes once" would be measuring the single-threaded
    // event loop.** Say which is which rather than claiming coverage the shape
    // of the runtime is providing.
    property bool publishing: false

    // ---- submitting -----------------------------------------------------
    //
    // One function, and every exit from it sets `outcome` to exactly one of
    // three values. There is no path that leaves a progress state on screen:
    // the call is synchronous, so there is no window in which a spinner could
    // resolve into nothing.
    function submit() {
        if (!root.submittable)
            return
        // A submission while one is outstanding submits NOTHING. Unreachable
        // through the button, which is not rendered while `publishing` — this is
        // the guard for a caller reaching `submit()` directly, and for the day
        // the call stops being synchronous.
        if (root.publishing)
            return

        root.publishing = true

        // The target this publish names, taken before the call so the answer
        // is applied to it whatever the composer points at by then.
        var publishedKey = root.targetKey

        var reply = root.kind === "reply"
            ? Core.publishReply(root.stoaAddress, root.replyParent, root.draft)
            : Core.publishPost(root.stoaAddress, root.draft)

        // **Cleared before the outcome is applied, and on EVERY path.** A
        // refusal stored nothing, so the user must be able to retry; leaving
        // this true on the refusal path would be a composer that locks itself
        // out permanently the first time core says no. `applyReply` has five
        // exits and this sits above all of them rather than being spelled at
        // each — which is the shape that survives a sixth being added.
        root.publishing = false

        root.applyReply(reply, publishedKey)
    }

    // The reply, turned into an outcome. Separate from `submit()` because it is
    // a separate job — deciding what a reply MEANS is not deciding what to send
    // — and because it is the half worth testing against shapes a real core
    // would not produce.
    //
    // `publishedKey` is the key of the target the publish named, which is the
    // draft a store clears. A caller that names none means the target the
    // composer points at: without that default, a stored reply applied with no
    // key would report "stored" and leave its text in the field, one press from
    // a second signed op.
    function applyReply(reply, publishedKey = root.targetKey) {
        // Every failure to reach core, every non-JSON reply and core's own error
        // shape arrive here as `ok: false`, because they all go through the one
        // `call()` path. That is the whole reason to have one.
        if (!reply.ok) {
            root.refuse(reply.error)
            return
        }

        // **A success is not "the absence of an error".** A reply the view could
        // not interpret must not be reported as a publish that happened: the
        // user would go looking for a post that was never written. So the
        // success branch is guarded on the fields a success actually carries.
        if (typeof reply.value.opId !== "string" || reply.value.opId === "") {
            root.refuse("The core module answered without an op id, so whether anything was published is unknown.")
            return
        }

        // `wasNew` absent and `wasNew: false` are different facts. Absent means
        // a reply that is not core's success shape for this call, and guessing
        // an outcome from no evidence would tell the user either that a post was
        // saved or that it already existed, with nothing behind either claim.
        if (reply.value.wasNew === true) {
            root.outcome = "stored"
            root.outcomeDetail = ""
            // **Cleared here and kept in the other two cases**, which is the
            // spec's "The draft is cleared when the op was newly stored, and
            // kept otherwise" — the asymmetry is the decision rather than an
            // inconsistency, and `design.md` carries the argument.
            //
            // This carried a `NO SPEC` marker until the spec contracted the
            // rule in full. Leaving it would have said a decision was unmade
            // when it is made, contracted and argued, which is how the next
            // reader concludes they are free to change it.
            root.clearDraftOf(publishedKey)
            root.published()
            return
        }
        if (reply.value.wasNew === false) {
            root.outcome = "existing"
            root.outcomeDetail = ""
            // **The draft is NOT cleared here.** Nothing new was written, so
            // there is nothing for the user to look at in a feed — and if they
            // meant to publish something different, clearing the box would have
            // taken away the text they need to edit.
            root.published()
            return
        }

        root.refuse("The core module answered without saying whether the "
                  + root.kind + " was newly stored, so what happened is unknown.")
    }

    // **The draft survives every refusal, unchanged.** The refusal most likely
    // to occur in ordinary use is a reply whose parent has not yet reached this
    // peer, which is expected to succeed on a retry — so discarding what the
    // user wrote is the worst available response to it. Nothing here reads
    // `message`; it is displayed, never matched on.
    function refuse(message) {
        root.outcome = "refused"
        root.outcomeDetail = message
    }

    // ---- the field ------------------------------------------------------

    Rectangle {
        Layout.fillWidth: true
        implicitHeight: 130
        color: DTheme.field
        border.width: DTheme.hairline
        border.color: root.overLimit ? DTheme.accent : DTheme.rule2

        TextEdit {
            id: field
            // Named for the end-to-end suite, and derived from `kind` rather
            // than written as one literal: the feed and the thread each mount a
            // composer, both stay in the element tree whichever screen is
            // shown, and a spec typing into "the composer" must not depend on
            // which one the host happens to report as visible. Each screen
            // mounts one composer of its own kind, so "postDraftField" and
            // "replyDraftField" each name exactly one field.
            objectName: root.kind + "DraftField"
            anchors.fill: parent
            anchors.margins: DTheme.itemGap
            font: DTheme.body
            color: DTheme.ink
            wrapMode: TextEdit.Wrap
            selectByMouse: true

            // PlainText, always, and for a second reason beyond the one that
            // applies to peer text: a rich-text editor normalises what is typed
            // into it, and normalising is the one thing this component must not
            // do to a draft.
            textFormat: TextEdit.PlainText

            // Every edit is held for the target the composer points at, as it
            // is made, so what comes back is the text as last edited and there
            // is no moment of leaving at which it has to be saved.
            onTextChanged: root.holdDraft(root.targetKey, field.text)
        }
    }

    // ---- what is wrong with the draft, before it is sent -----------------

    // The length, always visible once anything is typed, so the limit is
    // something the user watches approach rather than discovers on refusal.
    //
    // In BYTES. "%1 of %2 bytes" and not characters, because the unit is the
    // thing a composer gets wrong: the two numbers agree for ASCII and diverge
    // by a factor of four for emoji.
    Text {
        visible: root.draft.length > 0
        text: root.draftBytes + " OF " + root.bodyByteLimit + " BYTES"
        font: DTheme.label
        color: root.overLimit ? DTheme.accent : DTheme.inkMuted
        textFormat: Text.PlainText
    }

    // Over the cap: said plainly, and the submit affordance goes away.
    //
    // **The draft is not truncated to fit.** Silently discarding the end of what
    // someone wrote is a worse outcome than refusing to send it — they would
    // have published a post ending mid-sentence and had no way to know.
    Text {
        visible: root.overLimit
        text: "This " + root.kind + " is longer than the core module will accept, so it "
            + "cannot be sent yet. Nothing has been removed from what you wrote."
        font: DTheme.bodySmall
        color: DTheme.accent
        wrapMode: Text.WordWrap
        lineHeight: 1.55
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // The invisibles warning. Information, never a gate — the submit affordance
    // below does not read it.
    Text {
        visible: root.invisibleCount > 0
        text: "This draft contains " + root.invisibleCount + " invisible character(s). "
            + "They will be published exactly as you typed them, and readers' software "
            + "will remove them when it renders this " + root.kind + "."
        font: DTheme.bodySmall
        color: DTheme.accent
        wrapMode: Text.WordWrap
        lineHeight: 1.55
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- submit ---------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: DTheme.itemGap

        FlatButton {
            // Named from `kind` for the reason the draft field is.
            objectName: root.kind + "SubmitButton"
            // "Publish", not "Send". The word is the claim: nothing here sends
            // anything, and the success message says so too.
            text: root.kind === "reply" ? "Publish the reply" : "Publish the post"
            kind: "primary"
            // **Not rendered while a publish is outstanding**, which is the same
            // answer this component already gives an unsubmittable draft: the
            // affordance is absent rather than present-and-inert.
            //
            // Disabling is a property of the CONTROL and not a message asking
            // the user to wait — a prompt not to double-tap relies on the person
            // reading it in the moment they are least likely to.
            visible: root.submittable && !root.publishing
            onClicked: root.submit()
        }

        Item { Layout.fillWidth: true }
    }

    // ---- what happened --------------------------------------------------

    DPublishOutcome {
        // Named from `kind` for the reason the draft field is: the feed and
        // the thread each mount one composer, so "postOutcomeMessage" and
        // "replyOutcomeMessage" each name exactly one element. Not
        // "…PublishOutcome": `check_qml_names.py` reads that string as a bare
        // reference to this type's undecorated name.
        objectName: root.kind + "OutcomeMessage"
        outcome: root.outcome
        detail: root.outcomeDetail
        subject: root.kind
        Layout.fillWidth: true
    }
}
