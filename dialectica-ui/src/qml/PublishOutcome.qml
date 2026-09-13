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

    readonly property bool isRefusal: root.outcome === "refused"

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
        text: root.outcome === "stored"
                ? "Your " + root.subject + " was saved on this machine."
            : root.outcome === "existing"
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
    // For a success, what a success does and does not mean. The sentence exists
    // because the headline's claim is deliberately weaker than users expect, and
    // an unexplained weak claim reads as a hedge rather than as a fact.
    //
    // It must not direct the reader to the content's position on screen: a
    // reply is not a thread head and has no row in a feed of thread heads, so
    // "it is now below" is a claim the view cannot establish.
    Text {
        visible: !root.isRefusal
        text: root.outcome === "existing"
                ? "The identical content is already in this machine's log, under the same op id. Nothing new was written."
                : "It is in this machine's log. Whether any other peer has received it is not something this software can tell you yet."
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
