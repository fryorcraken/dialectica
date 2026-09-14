import QtQuick

// The generated name's THIRD CHANNEL, present so the disjointness gate has
// something to measure. It is not a renderer and must never become one.
//
// WHAT THIS IS FOR, because it is not obvious and the obvious reading is wrong.
//
// Three channels tell identities apart — the generated name, the mark, and the
// abbreviated public key on screen — and since issue #80 all three read the SAME
// 32 bytes, with no hash between the key and any of them. Byte-disjointness is
// therefore the only thing keeping name-grinding and mark-grinding independent
// searches whose costs multiply rather than add. That makes the allocation a
// property to gate, not a detail.
//
// But two of the three channels are QML (Identicon, AddressLabel) and the third
// is Rust (dialectica-core's `names.rs`). Nothing in the repository computes all
// three, so the pairwise check has nowhere to live unless the name's window is
// reachable from QML. This component is that reachability and nothing else.
//
// WHY IT COMPUTES INDICES AND NOT A NAME. The wordlists are 10,240 entries and
// are consensus-critical; a second copy here would be a second thing to keep in
// step, and CLAUDE.md's core/UI split puts derivation in core. The indices are
// sufficient for the property being gated: the question is WHICH KEY BYTES REACH
// THE NAME, and an index moves if and only if a byte it reads moves. A name is
// still core's to compute and PostHeader still receives one as a string.
//
// WHY A COMPONENT AND NOT A CONSTANT. `tst_identicon.qml` measures the other two
// channels by PROBING them — vary a byte, watch the output move — because a
// version that recomputed `abbreviate`'s arithmetic in the test passed under a
// mutation to `abbreviate` itself and was deleted for it. A probe that read the
// name's window out of a constant would fail the same way one level further out:
// it would follow the constant wherever it went and could never report an
// overlap. Probing this component is a measurement; reading a number is not.
//
// THE DUPLICATION IS REAL AND IS THE TRADE-OFF. The reduction arithmetic now
// exists in two languages. What keeps it honest is that both sides are pinned
// against the same written-down window and the same draw values, so the two
// drifting apart fails on both sides rather than silently. The alternative —
// leaving the name out of the QML gate — leaves the pairwise property ungated
// where two of the three channels actually live, which is worse.
QtObject {
    id: root

    property string key: ""           // with or without a "k:" prefix

    // The hex body, prefix stripped, padded to 64 characters so a short or
    // malformed key still yields stable indices rather than NaN. Peer-supplied
    // strings reach this, so it must not assume a well-formed key. Identical in
    // shape to Identicon's `_body` for the same reason.
    //
    // **THIS DIVERGES FROM CORE DELIBERATELY, and the spec says which way.**
    //
    // Core REFUSES malformed key material where this pads it: `names.rs` returns
    // `NameError::NotAValidPublicKey` for `""`, `"k:"`, `"k:0"` and `"not-a-key"`,
    // where this returns three in-range indices. That looks like one of the two
    // being wrong, and it was unrecorded until review asked which.
    //
    // The answer is that the two rules apply to different things. *Malformed key
    // material is refused rather than crashed on* forbids a name derived from
    // truncated or padded input because such a name "would render as an ordinary
    // participant" — a hazard that exists only where a name is RENDERED. This
    // component renders nothing; it reports which bytes are read. So *The three
    // channels read pairwise disjoint bytes of the public key* requires the
    // opposite of it: accept any input and report in-range values, because a
    // measurement returning a non-value would report the channel as reading no
    // byte, and "disjoint" is satisfied by a measurement that found nothing.
    //
    // Anything that renders a name refuses malformed input; the thing that only
    // measures accepts it. **Giving this component a rendering consumer moves it
    // under the first rule and obliges it to refuse** — which is one more reason
    // it must never become a renderer.
    //
    // The alternatives, since a refusal has no representation in a function
    // returning a number: `NaN` (renders nothing and makes the measurement
    // meaningless rather than failing loudly), `-1` (a sentinel every caller must
    // remember to check), a separate validity flag (a second thing for a probe to
    // get wrong). Padding keeps the one job: three in-range indices, always.
    // `test_a_malformed_key_still_yields_indices_in_range` pins it.
    readonly property string _body: {
        var a = key.replace(/^[a-z]+:/i, "").replace(/[^0-9a-f]/gi, "").toLowerCase();
        return (a + "00000000000000000000000000000000000000000000000000000000000000000").slice(0, 64);
    }
    function _byte(i) { return parseInt(_body.substr(i * 2, 2), 16); }

    // A 16-bit big-endian draw at key byte `i`, matching core's
    // `u16::from_be_bytes([key[i], key[i + 1]])`.
    function _draw(i) { return _byte(i) * 256 + _byte(i + 1); }

    // The name's window is key bytes 18..23, which the `generated-names` spec
    // allocates and `names.rs` reads. The three slots take two bytes each.
    //
    // The list sizes are what make each reduction exactly uniform — 8,192 and
    // 1,024 both divide 65,536 — and they are part of the scheme rather than
    // tunable. They are written out here rather than imported because there is
    // nothing to import them from: the lists live in core.
    function adjectiveIndex() { return _draw(18) % 8192; }
    function nounIndex()      { return _draw(20) % 1024; }
    function placeIndex()     { return _draw(22) % 1024; }
}
