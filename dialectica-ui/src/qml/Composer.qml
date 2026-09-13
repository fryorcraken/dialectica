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
    // `Composer { kind: "post"; parentOp: "deadbeef" }` was accepted and
    // silently dropped the parent: no warning, no refusal, no test. A sentence
    // that reads as a constraint the component enforces, when it enforces
    // nothing, is worse than no sentence — the next person writes a call site
    // trusting it.
    //
    // `replyParent` below is what makes it true rather than merely stated: a
    // parent belonging to a post is not "ignored", it is not reachable.
    property string parentOp: ""

    // The parent that actually reaches core, which is "" for a post whatever
    // `parentOp` holds. One expression rather than a rule to remember at each
    // call site, so a stray parent on a post is unrepresentable downstream
    // instead of dropped somewhere the caller cannot see.
    readonly property string replyParent: root.kind === "reply" ? root.parentOp : ""

    // The draft, and **the field is the single place it lives**.
    //
    // An alias rather than a separate property with a two-way binding: `text:
    // root.draft` plus `onTextChanged: root.draft = text` looks symmetric and is
    // not — the first keystroke breaks the declarative binding, after which a
    // programmatic write to `draft` updates nothing on screen. That failure is
    // silent and looks exactly like the clear-on-success not working.
    //
    // Nothing in this component ever rewrites this text, which is the whole of
    // "the composer never alters what the user typed": no trim, no normalise, no
    // strip anywhere on the path from here to `Core.publishPost`. An op is
    // signed over its bytes, so a composer that altered a draft would publish,
    // permanently and under the user's signature, something they did not write.
    // The one write is `clearDraft()`, which sets it to empty after a store.
    property alias draft: field.text

    function clearDraft() {
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
    // "" | "stored" | "existing" | "refused". See PublishOutcome.qml for why
    // this is a string rather than a set of booleans.
    property string outcome: ""
    property string outcomeDetail: ""

    // ---- what the draft costs and what is hiding in it ------------------

    readonly property int draftBytes: root.utf8Length(root.draft)
    readonly property bool overLimit: root.draftBytes > root.bodyByteLimit
    readonly property int invisibleCount: root.countInvisible(root.draft)

    // Empty drafts are not submitted. Core would refuse one on its own terms;
    // offering a button that cannot do anything is the affordance equivalent of
    // a dead text field.
    readonly property bool submittable: root.draft.length > 0 && !root.overLimit

    spacing: Theme.itemGap

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

    // ---- submitting -----------------------------------------------------
    //
    // One function, and every exit from it sets `outcome` to exactly one of
    // three values. There is no path that leaves a progress state on screen:
    // the call is synchronous, so there is no window in which a spinner could
    // resolve into nothing.
    function submit() {
        if (!root.submittable)
            return

        var reply = root.kind === "reply"
            ? Core.publishReply(root.stoaAddress, root.replyParent, root.draft)
            : Core.publishPost(root.stoaAddress, root.draft)

        root.applyReply(reply)
    }

    // The reply, turned into an outcome. Separate from `submit()` because it is
    // a separate job — deciding what a reply MEANS is not deciding what to send
    // — and because it is the half worth testing against shapes a real core
    // would not produce.
    function applyReply(reply) {
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
            root.clearDraft()
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
        color: Theme.field
        border.width: Theme.hairline
        border.color: root.overLimit ? Theme.accent : Theme.rule2

        TextEdit {
            id: field
            anchors.fill: parent
            anchors.margins: Theme.itemGap
            font: Theme.body
            color: Theme.ink
            wrapMode: TextEdit.Wrap
            selectByMouse: true

            // PlainText, always, and for a second reason beyond the one that
            // applies to peer text: a rich-text editor normalises what is typed
            // into it, and normalising is the one thing this component must not
            // do to a draft.
            textFormat: TextEdit.PlainText
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
        font: Theme.label
        color: root.overLimit ? Theme.accent : Theme.inkMuted
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
        font: Theme.bodySmall
        color: Theme.accent
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
        font: Theme.bodySmall
        color: Theme.accent
        wrapMode: Text.WordWrap
        lineHeight: 1.55
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- submit ---------------------------------------------------------

    RowLayout {
        Layout.fillWidth: true
        spacing: Theme.itemGap

        FlatButton {
            // "Publish", not "Send". The word is the claim: nothing here sends
            // anything, and the success message says so too.
            text: root.kind === "reply" ? "Publish the reply" : "Publish the post"
            kind: "primary"
            visible: root.submittable
            onClicked: root.submit()
        }

        Item { Layout.fillWidth: true }
    }

    // ---- what happened --------------------------------------------------

    PublishOutcome {
        outcome: root.outcome
        detail: root.outcomeDetail
        subject: root.kind
        Layout.fillWidth: true
    }
}
