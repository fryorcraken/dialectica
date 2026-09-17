import QtQuick
import QtTest
import "../src/qml"

// `muted` — the mark for an author already in the moderated list.
//
// The assertion that carries the weight is the SECOND one: muting must not
// reach a selector. If it did, what a mark looks like would depend on the
// context it is drawn in, so two peers rendering the same person in different
// lists would disagree — which is the failure the determinism contract in
// `Identicon.qml` exists to forbid. `tst_identicon.qml` pins the selectors
// against a fixed address; this pins that `muted` leaves every one of them
// alone.
TestCase {
    name: "IdenticonMuted"

    Component {
        id: markFactory
        Identicon {}
    }

    function mark(props) {
        var m = markFactory.createObject(null, props);
        verify(m !== null, "Identicon failed to instantiate");
        return m;
    }

    // The same fixed address tst_identicon.qml uses: distinct bytes in every
    // position the mark reads, so a selector reading a neighbouring byte shows
    // as a changed value rather than coinciding.
    readonly property string fixed:
        "k:000102030405060708090a0b" +   // bytes 0..11, the name's range
        "0c0d0e0f10111213" +             // bytes 12..19, the mark's range
        "1415161718191a1b1c1d1e1f"       // bytes 20..31, unread

    // ---- the default ----------------------------------------------------

    // A mark is opaque unless asked otherwise. Without this, `muted: true` as
    // the default would satisfy every other test here.
    function test_a_mark_is_fully_opaque_by_default() {
        var m = mark({ address: fixed });
        compare(m.muted, false, "muted must default to false");
        compare(m.opacity, 1, "an unmuted mark must be fully opaque");
        m.destroy();
    }

    // ---- the de-emphasis ------------------------------------------------

    // NO SPEC: the bundle asks for `muted` and gives no value for how muted it
    // is. 0.45 is the choice recorded in design.md D4.
    //
    // ASSERTED AS A RELATION AND AS A LITERAL. The relation (strictly less
    // opaque, and still visible) is what "recessive" means and survives a
    // retune of the number; the literal catches the token going missing, which
    // the relation alone would not — an `undefined` opacity is neither less
    // than 1 nor greater than 0 in a way a comparison reports usefully.
    function test_a_muted_mark_recedes_without_disappearing() {
        var plain = mark({ address: fixed });
        var dim = mark({ address: fixed, muted: true });

        verify(dim.opacity < plain.opacity,
               "a muted mark is not less opaque than an unmuted one ("
               + dim.opacity + " vs " + plain.opacity + ")");
        verify(dim.opacity > 0,
               "a muted mark is invisible — it must recede, not vanish");
        compare(dim.opacity, DTheme.markMutedAlpha,
                "the muted mark does not use the theme's token");
        compare(dim.opacity, 0.45, "markMutedAlpha is not 0.45");

        plain.destroy(); dim.destroy();
    }

    // Reactive, not read once at construction — a row that becomes moderated
    // while on screen must dim.
    function test_muting_an_existing_mark_dims_it() {
        var m = mark({ address: fixed });
        compare(m.opacity, 1);
        m.muted = true;
        compare(m.opacity, DTheme.markMutedAlpha,
                "the mark did not follow `muted` going true");
        m.muted = false;
        compare(m.opacity, 1, "the mark did not come back");
        m.destroy();
    }

    // ---- muting cannot reach a selector ---------------------------------

    // THE ASSERTION THIS FILE EXISTS FOR. Every selector the determinism
    // contract covers must be byte-identical with `muted` set. A `muted` branch
    // inside `_inkA()` or `_form()` — the implementation this design rejected —
    // fails here.
    //
    // Swept over many addresses rather than one, because a context-dependent
    // branch could easily be keyed on a byte value and miss a single fixture.
    function test_muting_changes_no_selector_for_any_address() {
        for (var hi = 0; hi < 256; hi += 23) {
            var hx = (hi < 16 ? "0" : "") + hi.toString(16);
            var addr = "k:000102030405060708090a0b" + hx + "0d0e0f10111213"
                     + "1415161718191a1b1c1d1e1f";
            var plain = mark({ address: addr });
            var dim = mark({ address: addr, muted: true });

            compare(dim._form(), plain._form(), "form changed under muted @" + hx);
            compare(String(dim._inkA()), String(plain._inkA()),
                    "ink A changed under muted @" + hx);
            compare(String(dim._inkB()), String(plain._inkB()),
                    "ink B changed under muted @" + hx);
            compare(String(dim._outlineInk()), String(plain._outlineInk()),
                    "outline changed under muted @" + hx);
            compare(dim._angleDeg(), plain._angleDeg(),
                    "angle changed under muted @" + hx);
            compare(dim._pitch(), plain._pitch(), "pitch changed under muted @" + hx);
            compare(dim._duty(), plain._duty(), "duty changed under muted @" + hx);
            compare(dim._weave(), plain._weave(), "weave changed under muted @" + hx);

            plain.destroy(); dim.destroy();
        }
    }

    // The size channels are untouched too: a muted mark occupies the same room,
    // or a moderated row reflows when it is moderated.
    function test_muting_changes_no_geometry() {
        var plain = mark({ address: fixed, size: 19 });
        var dim = mark({ address: fixed, size: 19, muted: true });
        compare(dim.width, plain.width, "width changed under muted");
        compare(dim.height, plain.height, "height changed under muted");
        compare(dim.stroke, plain.stroke, "stroke changed under muted");
        plain.destroy(); dim.destroy();
    }

    // A peer-supplied address is still handled safely when muted — muting must
    // not open a path that a malformed address reaches differently.
    function test_a_malformed_address_mutes_without_throwing() {
        var cases = ["", "k:", "stoa:zzzz", "k:0", "not-an-address"];
        for (var i = 0; i < cases.length; i++) {
            var m = mark({ address: cases[i], muted: true });
            compare(m.opacity, DTheme.markMutedAlpha,
                    "muted opacity wrong for " + cases[i]);
            var f = m._form();
            verify(f >= 0 && f < 11, "form out of range for " + cases[i]);
            m.destroy();
        }
    }
}
