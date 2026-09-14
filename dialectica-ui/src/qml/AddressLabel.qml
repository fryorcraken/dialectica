import QtQuick

// The one place a 32-byte identifier is abbreviated: head 8, middle 8, tail 6.
// Head-and-tail alone is the shape vanity generators are built to defeat;
// including the middle group makes a convincing near-match far more expensive.
// Full text is always what gets copied.
//
// **THE MIDDLE GROUP IS NOW LOAD-BEARING IN A SECOND WAY**, and it is worth
// knowing which is which. Its original job is the one above: an interior group
// an attacker cannot skip. Since issue #80 it also decides a byte allocation —
// for an AUTHOR this abbreviates the public key, the same 32 bytes the mark and
// the generated name read, so which bytes appear here is which bytes those two
// channels must avoid:
//
//     head 8 chars   -> key bytes 0..3      shown
//     mark                     4..11        hidden
//                              12..13       hidden, unallocated
//     middle 8 chars ->        14..17       shown
//     generated name           18..23       hidden
//                              24..28       hidden, unallocated
//     tail 6 chars   ->        29..31       shown
//
// **The three group sizes below therefore cannot be retuned freely.** Widening
// any of them walks a displayed group onto a byte the mark or the name reads,
// and a byte this label DISPLAYS is the worst kind to share: an attacker
// grinding a lookalike reads their progress straight off the screen. The middle
// group is CENTRED, so widening it walks outward in both directions at once —
// and **both directions land on something**, which is the half of this example a
// reader needs and the half it used to omit. At `middleChars: 20`,
// `start = floor((64 - 20) / 2) = 22`, so chars 22..41 are shown: key bytes
// **11..20**. That is byte 11 leftward, which the mark reads, AND bytes 18, 19,
// 20 rightward, which are the generated name's window — a displayed byte
// reaching a derived one, the case the paragraph above calls the worst kind to
// share. Checking only the leftward collision and moving the group left would
// look like a fix and would not be one.
//
// That is measured rather than supposed, and it is why `tst_identicon.qml`
// probes this component for what it displays instead of recomputing the
// arithmetic below: a test that restates `abbreviate` can only see its INPUTS
// move, never the arithmetic itself.
//
// **This still abbreviates a Stoa address too**, and for a Stoa none of the
// allocation above applies — there is no mark-and-name budget over a Stoa
// address. The shape is shared; the byte allocation is about author keys.
Text {
    id: root

    property string address: ""
    property bool   full: false       // true wherever a decision is being made
    property bool   emphasis: false   // e.g. a lookalike's key, in accent

    function abbreviate(a) {
        var m = a.match(/^([a-z]+:)?(.*)$/i);
        var prefix = m[1] || "";
        var body = m[2];
        var h = DTheme.headChars, mid = DTheme.middleChars, t = DTheme.tailChars;
        if (body.length <= h + mid + t + 4)
            return a;
        var start = Math.floor((body.length - mid) / 2);
        return prefix + body.substr(0, h) + "\u2026"
                      + body.substr(start, mid) + "\u2026"
                      + body.slice(-t);
    }

    property string copyText: address

    text: full ? address : abbreviate(address)
    font: full ? DTheme.addressBig : DTheme.address
    color: emphasis ? DTheme.accent : DTheme.inkMuted
    wrapMode: full ? Text.WrapAnywhere : Text.NoWrap
    textFormat: Text.PlainText

    MouseArea {
        anchors.fill: parent
        acceptedButtons: Qt.RightButton | Qt.LeftButton
        cursorShape: Qt.IBeamCursor
        onClicked: root.copyRequested()
    }
    signal copyRequested()
}
