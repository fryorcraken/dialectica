import QtQuick
import QtQuick.Layouts

// The footer chip that says who you are in this Stoa — or that you are nobody
// here yet. It lives centred in the screen footer on every feed, so the answer
// to "who am I posting as" is always in the same place rather than somewhere
// that depends on what the screen is doing.
//
// WHY IT LABELS THE MARK. `Identicon` no longer distinguishes a person from a
// Stoa by contour: that split halved the vocabulary any one address could reach
// and restated what position already said, so the distinction is carried by
// POSITION ALONE (see Identicon.qml). The obligation attached to that decision
// is that a placement the context does not disambiguate must label itself — and
// a footer chip is such a placement, since a reader arriving at it cold has no
// surrounding post to say whose mark this is. `CURRENT IDENTITY` is that label,
// and it carries that disambiguation rather than being decoration.
//
// The address is on screen here and not one click away. What is grounded, and
// what is not, kept apart deliberately:
//
//   * GROUNDED — what the mark is not. `docs/IDENTICON.md` records that the
//     public key is the identity and the mark only a recognition aid, "a second
//     forgeable channel, and a second forgeable channel is still forgeable",
//     and that uniqueness is unachievable by pigeonhole. `generated-names`
//     contracts the same of a name: "A display name SHALL NOT be treated as
//     unique, SHALL NOT be accepted anywhere an identity is named".
//
//   * NOT GROUNDED — any requirement to put an address beside a mark. No spec
//     pairs the two. `IDENTICON.md` says the identifying value must be on
//     screen "wherever recognition carries weight — above all wherever a
//     moderator is named", but sources that to a `SPEC.md` no longer in this
//     repo; and `op-format`'s "show the Stoa address alongside any name" is
//     scoped to STOAS, with the spec saying in terms that it must not be cited
//     as authority for an author address.
//
// So this placement is a local judgement, not a rule inherited from elsewhere.
//
// ---- FOR WHOEVER PLACES THIS CHIP IN A FOOTER ---------------------------
//
// **Never bind a raw probe field. Always `=== true`.** That part is not
// cosmetic and holds for either binding below: `"true"`, `1`, `null` and
// `undefined` are all not-`true`, and each is truthy-or-falsy in a way that
// does not match what it means. A chip handed any of them under a looser test
// renders the IDENTITY PRESENT arm, claiming an identity the machine does not
// have, with every gate green.
//
// **WHICH answer to bind depends on what the chip is for, and the two are not
// interchangeable.** `who_am_i` and `get_capabilities` answer different
// questions and `lib.rs:258` contracts that they can honestly disagree — a
// stored identity whose keystore permissions are too open is a real identity
// that cannot currently be used.
//
//   * **Bind `<whoAmI>.hasIdentity === true` where the chip offers the route to
//     CREATING an identity** — which is every placement that renders the
//     `createRequested()` arm, the feed footer included. Binding `canPost` there
//     collapses "no identity" and "an identity that cannot be used" into one
//     state and offers key creation to the second. That is the one irreversible
//     wrong answer available: `keep_identity` is refused where an identity
//     already exists, and replacing one discards every identity derived from it
//     while the ops they signed stay published. The user who most needs to be
//     told what is wrong is instead offered a new key.
//   * **Bind `<capability>.canPost === true` only where the chip reports whether
//     the user can ACT** and no creation route is offered from it.
//
// `FeedScreen.qml` normalises both at the boundary — `identityFrom()` and
// `capabilityFrom()` — so a screen that routes through either inherits the
// `=== true` rule. One that reads a probe reply directly does not. The three
// degenerate shapes are driven as fixtures in `tst_gate_affordance.qml` and
// `tst_vote_and_gate.qml`.
//
// **`generatedName` cannot be filled by any caller yet.** The QML sandbox
// holds no wordlists and cannot derive a name for itself; `generated-names`'
// spec records the derivation as reachable by no caller until an entry point
// exists, tracked as issue #81. Passing `""` is correct until it does, and is
// what the existing consumer passes.
Rectangle {
    id: root

    property bool hasIdentity: true
    property string generatedName: ""
    property string identityAddress: ""
    signal createRequested()

    implicitWidth:  row.implicitWidth + 24
    implicitHeight: row.implicitHeight + 8
    color: DTheme.field
    border.width: DTheme.hairline

    // The accent border when there is no identity is the chip asking for
    // attention: it is the one state in which the footer wants to be read.
    border.color: hasIdentity ? DTheme.rule2 : DTheme.accent

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: 9

        // ---- identity present -------------------------------------------
        // A RowLayout omits a `visible: false` child from its layout entirely,
        // so the two sets below never reserve space for each other.
        // GATED ON THE ADDRESS, not only on `hasIdentity`, and that extra
        // conjunct is load-bearing.
        //
        // `Identicon` pads a short or malformed address to 64 hex characters so
        // it renders something stable rather than throwing — correct for the
        // component, and wrong here: an empty address pads to 64 zeros and
        // draws a perfectly valid, deterministic, RECOGNISABLE mark. The mark
        // of the zero address, which belongs to nobody, under the label
        // CURRENT IDENTITY, beside an AddressLabel rendering nothing.
        //
        // That is the state every screen holds between appearing and core
        // answering the identity call, so it is not a corner case — and it
        // inverts this component's own rule, stated in the header above: the
        // mark is a recognition aid and the address is the identifier, so a
        // mark with no address to read beside it is the one arrangement the
        // chip must not produce.
        //
        // The label and the name below are NOT gated on the address: they claim
        // nothing forgeable, and hiding them would make the chip flicker
        // through a third layout on the way to being filled.
        Identicon {
            visible: root.hasIdentity && root.identityAddress !== ""
            address: root.identityAddress
            size: 17
        }
        Text {
            visible: root.hasIdentity
            text: "CURRENT IDENTITY"          // copy.json common.currentIdentity
            font: DTheme.label
            color: DTheme.inkMuted
            textFormat: Text.PlainText
        }
        Text {
            visible: root.hasIdentity

            // A generated name is derived from the key and is NOT an
            // identifier — two keys can produce names that look alike, and
            // arrival order differs per peer so they are never numbered apart.
            //
            // PlainText as DEFENCE IN DEPTH, and NOT because the name is
            // peer-supplied — which is what this comment used to say, and is
            // contradicted by the merged contract. `generated-names/spec.md`
            // requires that the name SHALL NOT travel ("no reply SHALL carry a
            // display name... a name is derived by whoever holds the key, at
            // the point of rendering") and that every wordlist entry is ASCII
            // and lowercase, precisely so "a generated name can never itself
            // carry a bidi override or a homoglyph — it removes the attack from
            // this surface rather than mitigating it."
            //
            // So the format is pinned against a name arriving from somewhere
            // the contract did not anticipate, not against the wire. Stating
            // the wrong mechanism describes a larger hole than the code has and
            // would send the next reader looking for a sanitiser this string
            // does not need.
            text: root.generatedName
            font: DTheme.bodySmall
            color: DTheme.ink
            textFormat: Text.PlainText
        }
        AddressLabel {
            visible: root.hasIdentity
            address: root.identityAddress
        }

        // ---- no identity on this machine ---------------------------------
        Text {
            visible: !root.hasIdentity

            // copy.json feed.readOnlyFooter, verbatim. It names what an
            // identity is FOR rather than asserting a restriction: reading
            // needs nothing, and the sentence says so by omission.
            text: "Voting, posting and replying need an identity."
            font: DTheme.bodySmall
            color: DTheme.ink
            textFormat: Text.PlainText
        }
        FlatButton {
            visible: !root.hasIdentity
            text: "Create an identity"        // copy.json common.createIdentity
            kind: "primary"
            onClicked: root.createRequested()
        }
    }
}
