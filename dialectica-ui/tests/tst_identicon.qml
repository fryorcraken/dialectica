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
    //
    // This is the MARK side, and it hardcodes which bytes the abbreviation
    // shows, so it cannot see the abbreviation widening onto the mark. The
    // abbreviation side is `test_the_mark_and_the_abbreviation_share_no_byte`
    // further down, which measures both windows instead; both are needed, and
    // for a while only this one existed.
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

    // ─── Disjointness, measured across all THREE channels ──────────────────
    //
    // `test_no_byte_the_abbreviation_displays_reaches_the_mark` above pins this
    // from the mark's side, but it hardcodes WHICH bytes the abbreviation shows
    // (0..3, 14..17, 29..31) in its fixture string. That set is not a constant:
    // `abbreviate()` derives it from `DTheme.headChars`, `DTheme.middleChars`
    // and `DTheme.tailChars`, and the middle group is CENTRED, so widening it
    // walks the group outward from the middle in both directions.
    //
    // So a DTheme edit can slide the middle group onto a byte the mark reads
    // while that test keeps passing — it would go on flipping bytes 14..17,
    // which by then are no longer the group. Measured, not supposed: with
    // `middleChars: 20` the group becomes bytes 11..20 and overlaps the mark's
    // byte 11 (`_weave`), and every test in this file still passed.
    //
    // These ask each component what it ACTUALLY reads or displays and compare
    // the three measured sets. No set is written down except in the meta-test
    // below, so moving ANY window — the mark's offsets, the name's window, or
    // any of the three DTheme group sizes — fails these rather than silently
    // removing the property.
    //
    // **THERE ARE THREE CHANNELS HERE NOW, AND THAT IS THE CHANGE.** Until issue
    // #80 the name derived from H(NAME_PREFIX || public_key) while the mark and
    // the abbreviation read the AUTHOR ADDRESS, a different digest of the same
    // key. Two digests cannot overlap, so the name's independence came from
    // domain separation whatever bytes it read, and it had no place in a byte
    // gate — which is exactly why these tests were a pair rather than a triple.
    //
    // With the author address deleted and no hash between the key and any
    // channel, all three read the SAME 32 bytes. Byte-disjointness stops being
    // decorative and becomes the only thing making the three grinding searches
    // independent, so the name joins the gate. `DKeyNameWindow` exists for no
    // other reason than to give this file a third thing to probe.

    Component {
        id: labelFactory
        AddressLabel {}
    }

    Component {
        id: nameWindowFactory
        DKeyNameWindow {}
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

    // The byte indices the GENERATED NAME reads, discovered the same way: vary a
    // byte and see whether any of the three draw indices moves.
    //
    // **THIS IS THE THIRD CHANNEL, and before issue #80 it had no place here.**
    // While the name derived from H(NAME_PREFIX || public_key) and the mark read
    // the ADDRESS, the two read different digests and could not overlap — their
    // independence came from domain separation whatever bytes each happened to
    // read, and a byte allocation between them was neither required nor
    // possible. With the author address deleted and no hash between the key and
    // any channel, all three read the same 32 bytes and byte-disjointness is the
    // ONLY thing making the three grinding searches independent.
    //
    // **Measured, never restated.** `DKeyNameWindow` exists so that this can be
    // a measurement: probing it follows the arithmetic wherever it goes, where a
    // probe that read the window out of a constant would follow the constant and
    // could never report an overlap. That is the failure mode that got the
    // computed version of these tests deleted — see the note below.
    function _nameBytes() {
        var base = "";
        for (var i = 0; i < 32; i++) base += "00";
        var win = nameWindowFactory.createObject(null, { key: "k:" + base });
        function draws() {
            return [win.adjectiveIndex(), win.nounIndex(), win.placeIndex()].join("|");
        }
        var reference = draws();
        var read = [];
        for (var b = 0; b < 32; b++) {
            for (var v = 0; v < probeValues.length; v++) {
                var hex = base.substr(0, b * 2) + probeValues[v]
                        + base.substr(b * 2 + 2);
                win.key = "k:" + hex;
                if (draws() !== reference) { read.push(b); break; }
            }
            win.key = "k:" + base;
        }
        win.destroy();
        return read;
    }

    // The measurement itself must be sound, or "disjoint" is satisfied by a
    // probe that finds nothing. This pins all three measurements against their
    // known windows — the one place in these tests where the windows ARE written
    // down, so that a probe which silently stopped detecting bytes fails here
    // rather than reporting a false all-clear from the tests below.
    function test_the_byte_probes_find_the_windows_they_should() {
        compare(_markBytes().join(","), "4,5,6,7,8,9,10,11",
                "the mark probe does not see the mark's documented window");
        compare(_displayedBytes().join(","), "0,1,2,3,14,15,16,17,29,30,31",
                "the abbreviation probe does not see the documented groups");
        compare(_nameBytes().join(","), "18,19,20,21,22,23",
                "the name probe does not see the name's documented window");
    }

    // The security property itself: grinding for a lookalike mark and grinding
    // for a lookalike abbreviated address must be independent searches, whose
    // costs multiply rather than add. A byte in both sets is worse than merely
    // shared — it is a byte the attacker can target while reading their progress
    // off the screen.
    //
    // **A THIRD test of this property was deleted rather than kept**, and the
    // reason is worth recording because it is the shape that will be reinvented.
    // It computed the displayed set from `DTheme.headChars` / `middleChars` /
    // `tailChars` by REDOING `abbreviate`'s arithmetic in the test, instead of
    // asking `AddressLabel` what it displays. That catches a DTheme widening —
    // both `headChars: 24` and `middleChars: 20` fail it — but it is blind to a
    // change in `abbreviate` itself. Measured: shifting the head group to
    // `body.substr(8, h)` with DTheme untouched puts mark byte 4 on screen, and
    // the mirroring test PASSED while the two probe tests here failed. A test
    // that restates the production arithmetic can only see its inputs move, not
    // the arithmetic; these measure the component.
    function test_the_three_channels_read_pairwise_disjoint_bytes() {
        var channels = [
            { name: "the abbreviation", bytes: _displayedBytes() },
            { name: "the mark",         bytes: _markBytes() },
            { name: "the generated name", bytes: _nameBytes() }
        ];

        // Every set must be non-empty, or "disjoint" would be satisfied by a
        // measurement that found nothing — the two-explanations-one-answer
        // shape. Checked per channel rather than as a total, because a total
        // stays comfortably large while one channel measures zero.
        for (var c = 0; c < channels.length; c++) {
            verify(channels[c].bytes.length > 0,
                   channels[c].name + " reads no byte at all, so every "
                   + "disjointness assertion below is vacuous for it");
        }

        // PAIRWISE, which is what the spec says and is not the same as "the
        // union has 25 distinct entries". A union count passes when a set
        // measures empty; a pairwise sweep with the non-emptiness precondition
        // above states what is actually required, and names the overlapping byte
        // when it fails.
        for (var i = 0; i < channels.length; i++) {
            for (var j = i + 1; j < channels.length; j++) {
                var a = channels[i], b = channels[j];
                for (var k = 0; k < a.bytes.length; k++) {
                    verify(b.bytes.indexOf(a.bytes[k]) === -1,
                           "key byte " + a.bytes[k] + " is read by BOTH "
                           + a.name + " and " + b.name + " — "
                           + a.name + " reads [" + a.bytes.join(",") + "], "
                           + b.name + " reads [" + b.bytes.join(",") + "], at "
                           + "head " + DTheme.headChars + ", middle "
                           + DTheme.middleChars + ", tail " + DTheme.tailChars
                           + ". The three channels read the same 32 bytes, so a "
                           + "shared byte makes two grinding searches partly "
                           + "coincide and their costs add instead of "
                           + "multiplying");
                }
            }
        }
    }

    // A byte the abbreviation DISPLAYS is worse than a merely shared one, and
    // the spec says so in those words: an attacker reads their progress off the
    // rendered key while grinding it, so it contributes nothing an observer
    // could not have been handed directly.
    //
    // This is a SEPARATE test from the pairwise sweep above, not a duplicate of
    // two of its pairs. The sweep would fail on any overlap and report it as one
    // of three symmetrical cases; this one says which overlap is the severe one,
    // so a reader looking at a failure knows whether a hidden byte leaked into
    // two channels or a displayed byte reached a derived one.
    function test_no_byte_on_screen_reaches_the_mark_or_the_name() {
        var shown = _displayedBytes();
        verify(shown.length > 0, "the abbreviation displays no byte at all");

        var derived = [
            { name: "the mark", bytes: _markBytes() },
            { name: "the generated name", bytes: _nameBytes() }
        ];
        for (var d = 0; d < derived.length; d++) {
            verify(derived[d].bytes.length > 0,
                   derived[d].name + " reads no byte at all");
            for (var i = 0; i < shown.length; i++) {
                verify(derived[d].bytes.indexOf(shown[i]) === -1,
                       "key byte " + shown[i] + " is DISPLAYED by the "
                       + "abbreviation and also read by " + derived[d].name
                       + " — an attacker grinding a lookalike can read their "
                       + "progress off the rendered key");
            }
        }
    }

    // **The reduction arithmetic now exists in two languages**, and this is the
    // pin that keeps them from drifting apart silently.
    //
    // `DKeyNameWindow` reproduces what `dialectica-core`'s `names.rs` does — the
    // same window, the same big-endian draws, the same moduli — because the
    // pairwise gate above needs a third channel to probe and core is not
    // reachable from `qmltestrunner`. That duplication is the design's accepted
    // trade-off, and an unpinned duplicate is how two implementations of one
    // scheme quietly diverge.
    //
    // **Both the key and the expected indices are WRITTEN DOWN**, produced by
    // `dialectica-core/examples/pin_name.rs`, which reads the wordlists from the
    // text files and does not link the derivation. The same pair is pinned on
    // the Rust side in `names.rs`'s `PINNED_CASES`, so the two sides agree with
    // a third party rather than with each other.
    //
    // If this fails, do NOT adjust it to match. Either this file's arithmetic or
    // core's has moved, and the two are now deriving different names from the
    // same key on the same build.
    // **BOTH of `names.rs`'s pinned cases, and the second one is not redundant.**
    //
    // A single case was measured insufficient. In the first key below, the noun
    // slot's two bytes are BOTH 0xbe, so `be16` and `le16` give the same 16-bit
    // value and that slot's index is identical under either byte order. Reading
    // the noun slot's bytes in the WRONG ORDER — `_byte(21) * 256 + _byte(20)` —
    // left this file's entire suite green, while QML and core would have derived
    // different names for almost every other key on the same build. Two
    // explanations, one answer: this repo's named test defect, found live here.
    //
    // The second key's three slots each hold two DIFFERENT bytes
    // (0xef/0x1a, 0x06/0xad, 0xa6/0x6d), so every slot is byte-order sensitive
    // and a per-slot endian flip cannot coincide in any of them.
    //
    // That precondition is asserted below rather than trusted, because it is a
    // property of the FIXTURE and a fixture property nobody checks is one that
    // decays: someone re-derives a pin against a new key, the pair of bytes
    // happens to repeat, and the pin goes quietly green about a byte order that
    // moved.
    readonly property var pinnedCases: [
        // key hex, adjective, noun, place. Produced by
        // `dialectica-core/examples/pin_name.rs`, which reads the wordlists from
        // the text files and does not link the derivation — so these agree with a
        // third party rather than with either implementation. The same two keys
        // are pinned by name in `names.rs`'s `PINNED_CASES`.
        { key: "ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c",
          adjective: 5806, noun: 702, place: 914,
          name: "quartzous paris of sypalettos" },
        { key: "66be7e332c7a453332bd9d0a7f7db055f5c5ef1a06ada66d98b39fb6810c473a",
          adjective: 3866, noun: 685, place: 621,
          name: "inpardonable paidagogos of myriandros" }
    ];

    function test_the_name_window_agrees_with_cores_pinned_case() {
        for (var c = 0; c < pinnedCases.length; c++) {
            var p = pinnedCases[c];
            var win = nameWindowFactory.createObject(null, { key: "k:" + p.key });

            compare(win.adjectiveIndex(), p.adjective, p.name + ": adjective index");
            compare(win.nounIndex(), p.noun, p.name + ": noun index");
            compare(win.placeIndex(), p.place, p.name + ": place index");

            // The prefix must not shift the offsets, for the same reason it must
            // not for the mark: a "k:" key and a bare one are one identity.
            win.key = p.key;
            compare(win.adjectiveIndex(), p.adjective,
                    p.name + ": adjective index without a prefix");
            compare(win.nounIndex(), p.noun, p.name + ": noun index without a prefix");
            compare(win.placeIndex(), p.place, p.name + ": place index without a prefix");

            win.destroy();
        }
    }

    // The precondition the pins above rest on, asserted rather than assumed.
    //
    // A slot whose two key bytes are EQUAL reads the same 16-bit value
    // big-endian or little-endian, so that slot's pin cannot fail on a byte-order
    // change. The first pinned key has exactly that property in its noun slot
    // (0xbe, 0xbe), which is how a wrong-order noun slot passed this whole file.
    // At least one pinned case must therefore be sensitive in EVERY slot, or the
    // cross-language pin has a blind spot in whichever slot coincides.
    //
    // Stated as "some case is sensitive everywhere" rather than "every case is",
    // because the first case is kept deliberately: it is the one `names.rs`
    // reproduces by hand, and its noun coincidence is a fact about that key
    // rather than a defect in it.
    function test_some_pinned_case_is_byte_order_sensitive_in_every_slot() {
        function _b(hex, i) { return parseInt(hex.substr(i * 2, 2), 16); }

        var anyFullySensitive = false;
        for (var c = 0; c < pinnedCases.length; c++) {
            var hex = pinnedCases[c].key;
            var sensitive = true;
            // The three slots start at key bytes 18, 20 and 22.
            for (var s = 18; s <= 22; s += 2) {
                if (_b(hex, s) === _b(hex, s + 1)) sensitive = false;
            }
            if (sensitive) anyFullySensitive = true;
        }
        verify(anyFullySensitive,
               "no pinned case holds two DIFFERENT bytes in all three slots, so "
               + "a slot reading its two bytes in the wrong order would reach the "
               + "same index in every pinned case and the cross-language pin "
               + "would stay green while QML and core derived different names "
               + "from the same key");
    }

    // Peer-supplied strings reach this component too, so a short or malformed
    // key must yield stable in-range indices rather than NaN — the same
    // obligation `test_a_malformed_address_still_selects_valid_values` places on
    // the mark. An index of NaN would index a wordlist with `undefined` in core's
    // place and render nothing, which is a name attributable to nobody.
    function test_a_malformed_key_still_yields_indices_in_range() {
        var cases = ["", "k:", "stoa:zzzz", "k:0", "not-a-key"];
        for (var i = 0; i < cases.length; i++) {
            var win = nameWindowFactory.createObject(null, { key: cases[i] });
            var a = win.adjectiveIndex(), n = win.nounIndex(), p = win.placeIndex();
            verify(!isNaN(a) && a >= 0 && a < 8192,
                   "adjective index out of range for " + cases[i] + ": " + a);
            verify(!isNaN(n) && n >= 0 && n < 1024,
                   "noun index out of range for " + cases[i] + ": " + n);
            verify(!isNaN(p) && p >= 0 && p < 1024,
                   "place index out of range for " + cases[i] + ": " + p);
            win.destroy();
        }
    }

    // Disjointness stated as the CONSEQUENCE a reader can check, rather than as
    // a set comparison: vary a byte exactly one channel reads, and that
    // channel's output must move while the other two stand still.
    //
    // **This is a different claim from the pairwise sweep**, not a restatement
    // of it. The sweep compares three measured sets and would pass on three
    // channels that each read nothing at all except for the non-emptiness
    // precondition; this one exhibits the channels actually responding, one byte
    // at a time, which is the property a reader cares about. The two together
    // are what make "independent channels" mean something.
    //
    // One byte per channel rather than the whole window, because the point is
    // the ISOLATION rather than the coverage — and the windows themselves are
    // already pinned by the meta-test above.
    function test_a_byte_one_channel_reads_moves_only_that_channel() {
        var base = "";
        for (var i = 0; i < 32; i++) base += "00";

        function outputs(hex) {
            var m = mark("k:" + hex);
            var label = labelFactory.createObject(null, { address: "k:" + hex });
            var win = nameWindowFactory.createObject(null, { key: "k:" + hex });
            var out = {
                mark: [m._form(), String(m._inkA()), String(m._inkB()),
                       String(m._outlineInk()), m._angleDeg(), m._pitch(),
                       m._duty(), m._weave()].join("|"),
                shown: label.text,
                name: [win.adjectiveIndex(), win.nounIndex(),
                       win.placeIndex()].join("|")
            };
            m.destroy(); label.destroy(); win.destroy();
            return out;
        }

        var reference = outputs(base);

        // One byte from each channel's own window: 0 is the abbreviation's
        // head, 4 the mark's first dimension, 18 the name's adjective slot.
        // `probeValues` rather than a single flip, for the reason given above
        // it: `0x00 % 3` and `0xff % 3` are both 0, so one value can look like
        // no change at all.
        var cases = [
            { byte: 0,  moves: "shown" },
            { byte: 4,  moves: "mark" },
            { byte: 18, moves: "name" }
        ];
        var fields = ["shown", "mark", "name"];

        for (var c = 0; c < cases.length; c++) {
            var b = cases[c].byte;
            var moved = false;
            for (var v = 0; v < probeValues.length && !moved; v++) {
                var hex = base.substr(0, b * 2) + probeValues[v]
                        + base.substr(b * 2 + 2);
                var got = outputs(hex);

                // The other two must be untouched for EVERY probe value, not
                // merely for the one that moved the target channel — a channel
                // that leaked on some values and not others would otherwise pass
                // as soon as one clean value was found.
                for (var f = 0; f < fields.length; f++) {
                    if (fields[f] === cases[c].moves) continue;
                    compare(got[fields[f]], reference[fields[f]],
                            "key byte " + b + " belongs to " + cases[c].moves
                            + " but moved " + fields[f] + " as well");
                }
                if (got[cases[c].moves] !== reference[cases[c].moves])
                    moved = true;
            }
            verify(moved,
                   "key byte " + b + " moved " + cases[c].moves
                   + " for no probe value, so that channel does not read the "
                   + "byte the allocation gives it");
        }
    }

    // The seven bytes no channel reads: 12..13 and 24..28. They are UNALLOCATED
    // rather than reserved — nothing depends on their value — and this asserts
    // only that none of the three has quietly been extended onto them.
    //
    // Without it the pairwise test alone is satisfied by three channels that had
    // each grown into the gaps in different directions, which would still be
    // disjoint and would still have spent the budget the allocation left spare.
    function test_no_channel_reads_an_unallocated_byte() {
        var unallocated = [12, 13, 24, 25, 26, 27, 28];
        var channels = [
            { name: "the abbreviation", bytes: _displayedBytes() },
            { name: "the mark",         bytes: _markBytes() },
            { name: "the generated name", bytes: _nameBytes() }
        ];
        for (var c = 0; c < channels.length; c++) {
            for (var u = 0; u < unallocated.length; u++) {
                verify(channels[c].bytes.indexOf(unallocated[u]) === -1,
                       "key byte " + unallocated[u] + " is unallocated but "
                       + channels[c].name + " reads it; extending a channel is "
                       + "a change to the generated-names spec");
            }
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
