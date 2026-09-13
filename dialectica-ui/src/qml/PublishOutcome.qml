import QtQuick
import QtQuick.Layouts

// What happened to a publish, rendered from ONE value.
//
// Three outcomes, and the spec requires all three to be mutually
// distinguishable: a newly stored op, an op that already existed, and a
// refusal. Written as booleans that would be four variables and sixteen
// combinations, thirteen of which are nonsense — and the one that eventually
// renders is a success message inside an error border.
//
// So `outcome` is one string, the way `FeedScreen.readState` is one string for
// the same reason. Two of these cannot be on screen at once because one
// variable cannot hold two values.
//
//   ""          nothing submitted yet — renders nothing
//   "stored"    the op was newly stored on this machine
//   "existing"  the op already existed; nothing new was written
//   "refused"   the publish did not happen; `detail` says what core said
//
// **Anything else is rendered as a refusal**, and that totality is this
// component's own, not its caller's. See `state` below.
ColumnLayout {
    id: root

    property string outcome: ""

    // Core's own message, for a refusal. Rendered verbatim: core's errors are
    // written to name a fix, and nothing here selects what is displayed by
    // matching on this text — a view branching on prose turns every improvement
    // to that prose into a silent breaking change.
    property string detail: ""

    // What was published, for the wording. "post" or "reply" — this is the
    // view's own word for its own affordance, never anything core supplied.
    property string subject: "post"

    // ---- the classification, made ONCE ----------------------------------
    //
    // **Every element below keys on this, and none of them re-derives it.**
    // That is the fix for a defect review found, and the defect is worth
    // recording because the shape is subtle and the code looked right.
    //
    // `outcome` used to be partitioned TWICE, along different seams: the
    // headline asked "is it one of the two successes?" and fell through to the
    // refusal wording, while `isRefusal` asked "is it the one refusal?" and the
    // sentences below keyed on its negation. Those are different partitions of
    // one value space, so a value in neither positive case — `"deferred"`,
    // `"Refused"`, `"refused "` — satisfied the refusal headline AND both
    // success sentences, rendering "Your post was not published." directly above
    // "It is in this machine's log." That is the exact contradiction the
    // one-string design was adopted to make unrepresentable, and `detail` was
    // suppressed at the same time because it was gated the other way — so the
    // one thing that would explain the state was the one thing withheld.
    //
    // **The invariant belonged here and was living in the caller.**
    // `Composer.applyReply` is total and writes only the four strings above, so
    // nothing reached this on the tree as shipped — but this is a separately
    // registered QML type with `outcome` as a public writable property, and a
    // second caller (the thread screen, when the reply composer lands there)
    // would inherit the rendering without inheriting the guarantee. A component
    // whose correctness is a property of who calls it is a component that is
    // correct by luck.
    //
    // So the classification is a value, computed once, and it is **total**: the
    // fall-through is "refused" rather than an error case, because an outcome
    // this component does not understand is one where it cannot honestly claim
    // anything was stored. Claiming less than happened is recoverable; claiming
    // storage that did not happen is not.
    readonly property string state: root.outcome === "stored"
                                 || root.outcome === "existing"
                                 || root.outcome === "refused"
        ? root.outcome
        : "refused"

    readonly property bool isRefusal: root.state === "refused"

    visible: root.outcome !== ""
    spacing: 6

    // ---- the headline ---------------------------------------------------
    //
    // **Three distinct strings, and the distinctness is the requirement.**
    //
    // "stored" says SAVED ON THIS MACHINE and nothing else. It must not say
    // sent, delivered, received, propagated, published to the Stoa, or that
    // anyone else can see it: the reply carries no delivery outcome by design,
    // delivery's result arrives asynchronously long after the call returns, and
    // nothing in this system checks it. A message claiming delivery would be
    // claiming something no part of the software has established.
    //
    // "existing" is neither a failure nor a fresh success. Nothing failed, so a
    // refusal is wrong; nothing new was written, so reporting a fresh success
    // leaves the user looking for a post that will never appear.
    Text {
        text: root.state === "stored"
                ? "Your " + root.subject + " was saved on this machine."
            : root.state === "existing"
                ? "This " + root.subject + " was already published."
            : "Your " + root.subject + " was not published."
        font: Theme.body
        color: root.isRefusal ? Theme.accent : Theme.ink
        wrapMode: Text.WordWrap
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- the qualifier --------------------------------------------------
    //
    // For a success, what a success does and does not mean — the part that
    // differs between the two successes. The sentence exists because the
    // headline's claim is deliberately weaker than users expect, and an
    // unexplained weak claim reads as a hedge rather than as a fact.
    //
    // It must not direct the reader to the content's position on screen: a
    // reply is not a thread head and has no row in a feed of thread heads, so
    // "it is now below" is a claim the view cannot establish.
    Text {
        visible: !root.isRefusal
        text: root.state === "existing"
                ? "The identical content is already in this machine's log, under the same op id. Nothing new was written."
                : "It is in this machine's log."
        font: Theme.bodySmall
        color: Theme.inkSoft
        wrapMode: Text.WordWrap
        lineHeight: 1.55
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- the delivery denial, owed by EVERY success ---------------------
    //
    // **Its own element rather than a clause inside the qualifier above, and
    // that is the fix rather than an incidental tidy-up.** The denial was
    // originally the tail of the `stored` arm of that ternary, which made it
    // one branch's copy — so the `existing` branch, which is a success by this
    // component's own design (`isRefusal` is false and nothing failed), carried
    // no denial at all. Hanging a requirement off one arm of a conditional is
    // how it goes missing from the other; hanging it off `!isRefusal` is how it
    // cannot.
    //
    // **Silence is not compliance here.** Every other delivery rule in this
    // capability is a prohibition, and a prohibition is discharged by saying
    // nothing. That is the wrong answer for this one: a reader who sees a forum
    // post submit successfully assumes it went somewhere, so an interface that
    // merely declines to mention delivery leaves that assumption standing while
    // being fully compliant with every prohibition.
    //
    // The stakes are specific rather than general good manners. A publish reply
    // carries no delivery outcome by design, delivery is not wired at all, and a
    // body that is legal but near the cap encodes past what the transport will
    // carry — so it is stored locally and silently refused by every receiving
    // peer. An author cannot tell a post nobody received from one everybody did,
    // and this sentence is the only place in the system that fact can be told to
    // them.
    //
    // It reads "with the success" because it is inside this component, which is
    // rendered directly beneath the headline. It is deliberately NOT in the
    // apparatus column: an obligation expressed as "this text appears in that
    // column" disappears with the column, silently, while still being required.
    Text {
        visible: !root.isRefusal
        text: "Whether any other peer has received it is not something this software can tell you yet."
        font: Theme.bodySmall
        color: Theme.inkSoft
        wrapMode: Text.WordWrap
        lineHeight: 1.55
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- core's message, for a refusal ----------------------------------
    //
    // Verbatim, in the address face, the way FeedScreen renders a read failure.
    // Nothing above or below it reads its text.
    Text {
        visible: root.isRefusal && root.detail !== ""
        text: root.detail
        font: Theme.address
        color: Theme.ink
        wrapMode: Text.WrapAnywhere
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }

    // ---- what a refusal does NOT mean -----------------------------------
    //
    // The view cannot tell WHICH refusal it received. Core distinguishes "no
    // such op is held" from "the op held is not a post", but that distinction
    // arrives only as differing prose inside one error shape with no
    // machine-readable discriminant — and a caller must not branch on message
    // wording.
    //
    // So this sentence is the reading that is safe under either: the draft is
    // kept and a retry is available. The asymmetry decides it — offering a retry
    // that cannot succeed costs one press, while withholding one from the
    // parent-has-not-arrived-yet case discards the thing most likely to work and
    // reports an ordinary, temporary, nobody's-fault condition as permanent.
    //
    // It therefore must not say the user caused this or that it is permanent,
    // because for the common case neither is true.
    Text {
        visible: root.isRefusal
        text: "Nothing was published and what you wrote is still here. You can try again."
        font: Theme.bodySmall
        color: Theme.inkSoft
        wrapMode: Text.WordWrap
        lineHeight: 1.55
        textFormat: Text.PlainText
        Layout.fillWidth: true
    }
}
