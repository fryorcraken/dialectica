import QtQuick
import QtTest
import "../src/qml"

// The mark's contract is "the same address selects the same appearance on every
// peer, forever". Nothing pinned that, so a refactor could have changed every
// identity's mark and the only symptom would have been two peers disagreeing
// about the same person — which nobody would attribute to a rendering change.
//
// These assert the SELECTORS, not the pixels. Pixel-identity is deliberately
// not the contract (device pixel ratio and antialiasing of rotated fills both
// move pixels without changing which shape or inks are chosen), so asserting it
// would be a test that fails for reasons the design does not care about.
TestCase {
    name: "Identicon"

    Component {
        id: markFactory
        Identicon {}
    }

    function mark(address) {
        return markFactory.createObject(null, { address: address });
    }

    // A fixed address with distinct bytes in every position the mark reads, so
    // a selector accidentally reading a neighbouring byte shows up as a changed
    // value rather than coinciding.
    readonly property string fixed:
        "k:00010203" +                   // bytes 0..3, the abbreviation's head
        "0405060708090a0b" +             // bytes 4..11, the mark's range
        "0c0d0e0f1011121314151617" +     // bytes 12..23 — 14..17 is the middle
        "18191a1b1c1d1e1f"               // bytes 24..31 — 29..31 is the tail

    // The mark reads bytes 4..11 and NOTHING the abbreviation displays. The
    // abbreviation shows bytes 0..3 (head), 14..17 (middle) and 29..31 (tail),
    // so every one of those must be free to change without moving a selector.
    //
    // This window was 12..19 and overlapped the middle group on {14, 15, 16, 17}
    // — half of what the mark read was already on screen. An attacker grinding
    // for a lookalike could watch their progress on those four bytes in the
    // rendered address. Moving the mark rather than the abbreviation keeps the
    // 8-8-6 shape and its vanity-defeating middle group intact.
    function test_the_mark_reads_only_bytes_4_to_11() {
        var a = mark(fixed);
        // Changing every byte the mark does not read — INCLUDING all three
        // groups the abbreviation displays — must not change any selector.
        var b = mark("k:ffffffff" + "0405060708090a0b"
                     + "ffffffffffffffffffffffffffffffffffffffff");
        compare(b._form(), a._form(), "form changed on an unread byte");
        compare(b._inkA(), a._inkA(), "ink A changed on an unread byte");
        compare(b._inkB(), a._inkB(), "ink B changed on an unread byte");
        compare(b._outlineInk(), a._outlineInk(), "outline changed on an unread byte");
        compare(b._angleDeg(), a._angleDeg(), "angle changed on an unread byte");
        compare(b._pitch(), a._pitch(), "pitch changed on an unread byte");
        compare(b._duty(), a._duty(), "duty changed on an unread byte");
        compare(b._weave(), a._weave(), "weave changed on an unread byte");
        a.destroy(); b.destroy();
    }

    // Pins the actual derived values. If a modulus, a byte offset or the ink
    // ordering changes, this fails — which is the point: every identity's mark
    // would have changed, and that is a decision to take deliberately.
    //
    // THE INK ASSERTIONS ARE THE IMPORTANT HALF and were missing in the first
    // version of this file. Without them, rotating every ink assignment by a
    // constant — `_byte(14) % 6` becoming `(_byte(14) + 3) % 6` applied
    // consistently across all three selectors — passed every test green while
    // changing what every identity in the system looks like. The distinctness
    // sweep below cannot catch it, because a rotation preserves distinctness;
    // and the other selectors here do not touch inks at all. Two explanations,
    // one answer: the classic shape of a test that proves nothing.
    //
    // So these pin the ladder's INDEXING against the named DTheme roles, not
    // merely that the three inks differ. The rotation mutation was watched
    // failing here before this was called done.
    function test_a_fixed_address_selects_fixed_values() {
        var m = mark(fixed);
        // In `fixed`, byte i holds the value i, so each expected value below is
        // the selector's arithmetic on its own byte, written out rather than
        // recomputed from the code under test.
        compare(m._form(), 4, "form");                  // byte 4 = 0x04; 4 % 11 = 4
        compare(m._angleDeg(), 120, "angle");           // byte 8 = 0x08; (8 % 12) * 15
        compare(m._pitch(), 3, "pitch");                // byte 9 = 0x09; [2,3,4,6][9 % 4 = 1]
        compare(m._duty(), 0.45, "duty");               // byte 10 = 0x0a; [...][10 % 3 = 1]
        compare(m._weave(), 2, "weave");                // byte 11 = 0x0b; 11 % 3 = 2

        // byte 6 = 0x06; 6 % 7 = 6 -> the seventh ink in the ladder.
        compare(String(m._inkA()), String(DTheme.markSage), "ink A indexing");
        // byte 7 = 0x07; 7 % 6 = 1, so B = (6 + 1 + 1) % 7 = 1.
        compare(String(m._inkB()), String(DTheme.markIndigo), "ink B indexing");
        // byte 5 = 0x05; 5 % 5 = 0, so the outline walks ONE step on from B's
        // index 1 to 2, and 2 is not A's index 6, so it stays.
        compare(String(m._outlineInk()), String(DTheme.markRust), "outline indexing");
        m.destroy();
    }

    // The disjointness the whole window move exists to produce, asserted as a
    // RELATION rather than as a restatement of the window. Changing any byte the
    // abbreviation displays must leave every selector alone — which is what makes
    // the mark a genuinely second channel rather than a restatement of what is
    // already on screen.
    function test_no_byte_the_abbreviation_displays_reaches_the_mark() {
        var a = mark(fixed);
        // Each group the abbreviation displays is flipped to ff, and the mark's
        // window is held at its `fixed` value. Byte counts, so a miscount cannot
        // hide a passing test: 4 + 8 + 2 + 4 + 11 + 3 = 32.
        var altered = "k:"
                    + "ffffffff"                   // bytes  0.. 3  head, flipped
                    + "0405060708090a0b"           // bytes  4..11  the mark, held
                    + "0c0d"                       // bytes 12..13  hidden
                    + "ffffffff"                   // bytes 14..17  middle, flipped
                    + "12131415161718191a1b1c"     // bytes 18..28  hidden
                    + "ffffff";                    // bytes 29..31  tail, flipped
        var b = mark(altered);
        compare(b._form(), a._form(), "form moved on a displayed byte");
        compare(b._inkA(), a._inkA(), "ink A moved on a displayed byte");
        compare(b._inkB(), a._inkB(), "ink B moved on a displayed byte");
        compare(b._outlineInk(), a._outlineInk(), "outline moved on a displayed byte");
        compare(b._angleDeg(), a._angleDeg(), "angle moved on a displayed byte");
        compare(b._pitch(), a._pitch(), "pitch moved on a displayed byte");
        compare(b._duty(), a._duty(), "duty moved on a displayed byte");
        compare(b._weave(), a._weave(), "weave moved on a displayed byte");
        a.destroy(); b.destroy();
    }

    // The invariant that a guard used to be responsible for and is now
    // structural. All three inks must differ, or the outline vanishes into the
    // ground and the mark loses its contour.
    // It sweeps bytes 6 and 7 — ink A's index and ink B's offset — which are the
    // two that decide all three inks, with byte 5 (the outline offset) held. It
    // MUST sweep bytes the mark actually reads: pointed at bytes the mark
    // ignores, every iteration would build the same three inks and the sweep
    // would pass while proving nothing.
    function test_the_three_inks_are_always_distinct() {
        for (var hi = 0; hi < 256; hi += 7) {
            for (var lo = 0; lo < 256; lo += 11) {
                var hx = (hi < 16 ? "0" : "") + hi.toString(16);
                var lx = (lo < 16 ? "0" : "") + lo.toString(16);
                //                bytes 0..4      byte 5   bytes 6,7   bytes 8..31
                var m = mark("k:" + "0001020304" + "05" + hx + lx
                             + "08090a0b0c0d0e0f1011121314151617"
                             + "18191a1b1c1d1e1f");
                var a = m._inkA(), b = m._inkB(), o = m._outlineInk();
                verify(a !== b, "ink A equals ink B at " + hx + "/" + lx);
                verify(a !== o, "outline equals ink A at " + hx + "/" + lx);
                verify(b !== o, "outline equals ink B at " + hx + "/" + lx);
                m.destroy();
            }
        }
    }

    // A malformed or short address is peer-supplied and must render something
    // stable rather than throwing or producing NaN indices.
    function test_a_malformed_address_still_selects_valid_values() {
        var cases = ["", "k:", "stoa:zzzz", "k:0", "not-an-address"];
        for (var i = 0; i < cases.length; i++) {
            var m = mark(cases[i]);
            var f = m._form();
            verify(f >= 0 && f < 11, "form out of range for " + cases[i]);
            verify(!isNaN(m._angleDeg()), "angle is NaN for " + cases[i]);
            verify(m._pitch() > 0, "pitch not positive for " + cases[i]);
            m.destroy();
        }
    }

    // ─── Disjointness, pinned from the ABBREVIATION's side ─────────────────
    //
    // `test_no_byte_the_abbreviation_displays_reaches_the_mark` above pins this
    // from the mark's side, but it hardcodes WHICH bytes the abbreviation shows
    // (0..3, 14..17, 29..31) in its fixture string. That set is not a constant:
    // `abbreviate()` derives it from `Theme.headChars`, `Theme.middleChars` and
    // `Theme.tailChars`, and the middle group is CENTRED, so widening it walks
    // the group outward from the middle in both directions.
    //
    // So a Theme edit can slide the middle group onto a byte the mark reads
    // while that test keeps passing — it would go on flipping bytes 14..17,
    // which by then are no longer the group. Measured, not supposed: with
    // `middleChars: 20` the group becomes bytes 11..20 and overlaps the mark's
    // byte 11 (`_weave`), and all eight tests in this file still passed.
    //
    // These two ask `AddressLabel` what it actually displays and compare that
    // against what `Identicon` actually reads. Neither set is written down here,
    // so moving EITHER window — the mark's offsets or any of the three Theme
    // group sizes — fails these rather than silently removing the property.

    Component {
        id: labelFactory
        AddressLabel {}
    }

    // Several probe values per byte, NOT one.
    //
    // A single flip to `ff` gives false negatives, and that is not hypothetical:
    // `_weave()` is `_byte(11) % 3`, and `0x00 % 3` and `0xff % 3` are both 0, so
    // a one-value probe concludes the mark does not read byte 11. The first
    // version of these tests did exactly that and passed under a mutation that
    // genuinely broke disjointness. Coprime-ish spread across the byte range, so
    // no modulus in either component can collide on all of them.
    readonly property var probeValues: ["ff", "01", "02", "05", "07", "3b", "91"]

    // The byte indices the abbreviation puts on screen, discovered by varying
    // one byte at a time and watching the rendered text. This is a MEASUREMENT
    // of AddressLabel rather than a restatement of Theme's arithmetic: if
    // `abbreviate` changes shape, this follows it.
    function _displayedBytes() {
        var base = "";
        for (var i = 0; i < 32; i++) base += "00";
        var label = labelFactory.createObject(null, { address: "k:" + base });
        var reference = label.text;
        var shown = [];
        for (var b = 0; b < 32; b++) {
            for (var v = 0; v < probeValues.length; v++) {
                var hex = base.substr(0, b * 2) + probeValues[v]
                        + base.substr(b * 2 + 2);
                label.address = "k:" + hex;
                if (label.text !== reference) { shown.push(b); break; }
            }
        }
        label.destroy();
        return shown;
    }

    // The byte indices the mark reads, discovered the same way: vary a byte and
    // see whether any selector moves. Derived from Identicon, not written down.
    function _markBytes() {
        var base = "";
        for (var i = 0; i < 32; i++) base += "00";
        function selectors(addr) {
            var m = mark(addr);
            var s = [m._form(), String(m._inkA()), String(m._inkB()),
                     String(m._outlineInk()), m._angleDeg(), m._pitch(),
                     m._duty(), m._weave()].join("|");
            m.destroy();
            return s;
        }
        var reference = selectors("k:" + base);
        var read = [];
        for (var b = 0; b < 32; b++) {
            for (var v = 0; v < probeValues.length; v++) {
                var hex = base.substr(0, b * 2) + probeValues[v]
                        + base.substr(b * 2 + 2);
                if (selectors("k:" + hex) !== reference) { read.push(b); break; }
            }
        }
        return read;
    }

    // The measurement itself must be sound, or "disjoint" is satisfied by a
    // probe that finds nothing. This pins both measurements against their known
    // windows — the one place in these tests where the windows ARE written down,
    // so that a probe which silently stopped detecting bytes fails here rather
    // than reporting a false all-clear from the two tests below.
    function test_the_byte_probes_find_the_windows_they_should() {
        compare(_markBytes().join(","), "4,5,6,7,8,9,10,11",
                "the mark probe does not see the mark's documented window");
        compare(_displayedBytes().join(","), "0,1,2,3,14,15,16,17,29,30,31",
                "the abbreviation probe does not see the documented groups");
    }

    // The security property itself: grinding for a lookalike mark and grinding
    // for a lookalike abbreviated address must be independent searches, whose
    // costs multiply rather than add. A byte in both sets is worse than merely
    // shared — it is a byte the attacker can target while reading their progress
    // off the screen.
    function test_the_mark_and_the_abbreviation_share_no_byte() {
        var shown = _displayedBytes();
        var read = _markBytes();

        // Both windows must be non-empty, or "disjoint" would be satisfied by a
        // measurement that found nothing — the two-explanations-one-answer shape.
        verify(shown.length > 0, "the abbreviation displays no byte at all");
        verify(read.length > 0, "the mark reads no byte at all");

        for (var i = 0; i < read.length; i++) {
            verify(shown.indexOf(read[i]) === -1,
                   "byte " + read[i] + " is both read by the mark and displayed "
                   + "by the abbreviation (mark reads [" + read.join(",")
                   + "], abbreviation shows [" + shown.join(",") + "])");
        }
    }

    // The middle group is the half a vanity generator is built to defeat, so its
    // presence is a requirement rather than a detail of the current Theme. Head
    // and tail alone would satisfy "disjoint" trivially while removing exactly
    // the protection the shape exists for.
    function test_the_abbreviation_keeps_a_middle_group() {
        var shown = _displayedBytes();
        verify(shown.length > 0, "the abbreviation displays no byte at all");

        // A group drawn from the INTERIOR: some displayed byte is neither at the
        // head nor at the tail, with unshown bytes on both sides of it.
        var interior = false;
        for (var i = 0; i < shown.length; i++) {
            var b = shown[i];
            if (shown.indexOf(b - 1) === -1 && shown.indexOf(b + 1) !== -1
                && b > 0 && b < 31) {
                // The start of a run that does not begin at byte 0.
                interior = true;
            }
        }
        verify(interior,
               "the abbreviation shows no interior group — head and tail alone "
               + "is the shape a vanity generator is built to defeat. Displayed: ["
               + shown.join(",") + "]");
    }

    // The prefix must not shift the byte offsets, or a "k:" address and a
    // bare one would render as different identities.
    function test_the_prefix_does_not_shift_the_offsets() {
        var withPrefix = mark(fixed);
        var bare = mark(fixed.substring(2));
        compare(bare._form(), withPrefix._form(), "form");
        compare(bare._inkA(), withPrefix._inkA(), "ink A");
        compare(bare._weave(), withPrefix._weave(), "weave");
        withPrefix.destroy(); bare.destroy();
    }
}
