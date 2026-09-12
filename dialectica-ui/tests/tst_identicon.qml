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
        "k:000102030405060708090a0b" +   // bytes 0..11, the name's range
        "0c0d0e0f10111213" +             // bytes 12..19, the mark's range
        "1415161718191a1b1c1d1e1f"       // bytes 20..31, unread

    function test_the_mark_reads_only_bytes_12_to_19() {
        var a = mark(fixed);
        // Changing a byte the mark does not read must not change any selector.
        var b = mark("k:ffffffffffffffffffffffff" + "0c0d0e0f10111213"
                     + "ffffffffffffffffffffffff");
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
    // So these pin the ladder's INDEXING against the named Theme roles, not
    // merely that the three inks differ. The rotation mutation was watched
    // failing here before this was called done.
    function test_a_fixed_address_selects_fixed_values() {
        var m = mark(fixed);
        compare(m._form(), 0x0c % 11, "form");
        compare(m._angleDeg(), (0x10 % 12) * 15, "angle");
        compare(m._pitch(), [2, 3, 4, 6][0x11 % 4], "pitch");
        compare(m._duty(), [0.30, 0.45, 0.62][0x12 % 3], "duty");
        compare(m._weave(), 0x13 % 3, "weave");

        // byte 14 = 0x0e = 14; 14 % 7 = 0 -> the first ink in the ladder.
        compare(String(m._inkA()), String(Theme.markInk), "ink A indexing");
        // byte 15 = 0x0f = 15; 15 % 6 = 3, so B = (0 + 1 + 3) % 7 = 4.
        compare(String(m._inkB()), String(Theme.markSteel), "ink B indexing");
        // byte 13 = 0x0d = 13; 13 % 5 = 3, so the outline walks four steps on
        // from B, skipping A's index 0: 5, 6, then 0 is skipped to 1, then 2.
        compare(String(m._outlineInk()), String(Theme.markRust), "outline indexing");
        m.destroy();
    }

    // The invariant that a guard used to be responsible for and is now
    // structural. All three inks must differ, or the outline vanishes into the
    // ground and the mark loses its contour.
    function test_the_three_inks_are_always_distinct() {
        for (var hi = 0; hi < 256; hi += 7) {
            for (var lo = 0; lo < 256; lo += 11) {
                var hx = (hi < 16 ? "0" : "") + hi.toString(16);
                var lx = (lo < 16 ? "0" : "") + lo.toString(16);
                // bytes 13, 14, 15 are the outline offset, ink A and ink B offset.
                var m = mark("k:000102030405060708090a0b" + "0c" + hx + lx
                             + "0f10111213" + "1415161718191a1b1c1d1e1f");
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
