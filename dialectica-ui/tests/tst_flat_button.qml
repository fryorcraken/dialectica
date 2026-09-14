import QtQuick
import QtTest
import "../src/qml"

// The five kinds. What matters is that they are TOLD APART — a destructive
// button that renders like a secondary is an irreversible publication dressed
// as a cancel — and that an unrecognised kind does something a person can see.
TestCase {
    name: "FlatButton"

    Component {
        id: buttonFactory
        FlatButton {}
    }

    function button(kind) {
        var b = buttonFactory.createObject(null, { text: "ACT", kind: kind });
        verify(b !== null, "FlatButton failed to instantiate for kind " + kind);
        return b;
    }

    readonly property var declaredKinds:
        ["primary", "secondary", "destructive",
         "destructive-outline", "secondary-micro"]

    function labelOf(item) {
        var found = null;
        function walk(node) {
            if (!node)
                return;
            if (typeof node.text === "string" && node.font !== undefined
                && node.textFormat !== undefined)
                found = node;
            var kids = node.children;
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i]);
        }
        walk(item);
        return found;
    }

    // ---- every kind renders something a person can see -------------------

    // THE INVARIANT THAT MATTERS MOST, and the one the previous implementation
    // broke for an unknown kind: a button must be VISIBLE. Either it has a
    // fill, or it has a border — a control with neither is paper text on
    // nothing, which is what the old ternary chains produced for any `kind`
    // they did not recognise.
    //
    // It is also asserted over every declared kind, so a sixth entry added to
    // the table with a field missing fails here rather than at a screen.
    function test_every_kind_is_either_filled_or_outlined() {
        for (var i = 0; i < declaredKinds.length; i++) {
            var b = button(declaredKinds[i]);
            var filled = String(b.color) !== "#00000000";
            var outlined = b.border.width > 0;
            verify(filled || outlined,
                   declaredKinds[i] + " renders with neither fill nor border — "
                   + "an invisible control that still accepts clicks");
            b.destroy();
        }
    }

    // And the text must contrast with whatever is behind it. A filled button
    // takes paper text; an outlined one takes ink (or accent) on the card.
    function test_no_kind_renders_its_label_in_the_colour_behind_it() {
        for (var i = 0; i < declaredKinds.length; i++) {
            var b = button(declaredKinds[i]);
            var label = labelOf(b);
            verify(label !== null, declaredKinds[i] + " renders no label");
            var behind = String(b.color) !== "#00000000"
                       ? String(b.color) : String(DTheme.paper);
            verify(String(label.color) !== behind,
                   declaredKinds[i] + " draws its label in the colour behind it: "
                   + String(label.color));
            b.destroy();
        }
    }

    // ---- the two new kinds ----------------------------------------------

    // `destructive-outline` sits beside a filled `destructive` on the
    // moderation confirmation. It must read as destructive — the accent — and
    // must NOT be a second filled red, or the two read as one decision offered
    // twice.
    function test_destructive_outline_is_accent_but_not_filled() {
        var outline = button("destructive-outline");
        var filled = button("destructive");

        compare(String(outline.color), "#00000000",
                "destructive-outline must not be filled");
        compare(String(outline.border.color), String(DTheme.accent),
                "destructive-outline must be bordered in the accent");
        compare(String(labelOf(outline).color), String(DTheme.accent),
                "destructive-outline's label must read as destructive");

        compare(String(filled.color), String(DTheme.accent),
                "destructive must be filled in the accent");
        verify(String(outline.color) !== String(filled.color),
               "the two destructive kinds are indistinguishable");

        outline.destroy(); filled.destroy();
    }

    // `secondary-micro` is `secondary` scaled down for a list row. It must
    // carry the same fill and border decisions — it is not a different kind of
    // button — and must be genuinely SMALLER, or it out-weighs the row it acts
    // on.
    function test_secondary_micro_is_a_smaller_secondary() {
        var micro = button("secondary-micro");
        var full = button("secondary");

        compare(String(micro.color), String(full.color),
                "secondary-micro and secondary must share a fill");
        compare(String(micro.border.color), String(full.border.color),
                "secondary-micro and secondary must share a border colour");

        verify(micro.implicitHeight < full.implicitHeight,
               "secondary-micro is not shorter than secondary ("
               + micro.implicitHeight + " vs " + full.implicitHeight + ")");
        verify(micro.implicitWidth < full.implicitWidth,
               "secondary-micro is not narrower than secondary ("
               + micro.implicitWidth + " vs " + full.implicitWidth + ")");

        micro.destroy(); full.destroy();
    }

    // ---- the three that existed ------------------------------------------

    // The kinds that predate this change keep their appearance. A reshape that
    // quietly restyled `primary` would be a change to every screen.
    function test_the_original_three_kinds_are_unchanged() {
        var primary = button("primary");
        compare(String(primary.color), String(DTheme.ink));
        compare(primary.border.width, 0, "primary must not be outlined");
        compare(String(labelOf(primary).color), String(DTheme.paper));
        primary.destroy();

        var secondary = button("secondary");
        compare(String(secondary.color), "#00000000");
        compare(secondary.border.width, DTheme.hairline);
        compare(String(secondary.border.color), String(DTheme.ink));
        compare(String(labelOf(secondary).color), String(DTheme.ink));
        secondary.destroy();

        var destructive = button("destructive");
        compare(String(destructive.color), String(DTheme.accent));
        compare(destructive.border.width, 0, "destructive must not be outlined");
        compare(String(labelOf(destructive).color), String(DTheme.paper));
        destructive.destroy();
    }

    // No two kinds render identically, or the distinction they exist to draw
    // is not on screen. The signature is what a reader can actually tell apart.
    function test_no_two_kinds_render_identically() {
        var seen = {};
        for (var i = 0; i < declaredKinds.length; i++) {
            var b = button(declaredKinds[i]);
            var sig = String(b.color) + "/" + b.border.width + "/"
                    + String(b.border.color) + "/" + String(labelOf(b).color)
                    + "/" + b.implicitHeight;
            verify(seen[sig] === undefined,
                   declaredKinds[i] + " renders identically to " + seen[sig]);
            seen[sig] = declaredKinds[i];
            b.destroy();
        }
    }

    // ---- an unrecognised kind --------------------------------------------

    // NO SPEC: the bundle does not say what an unknown `kind` does. This pins
    // the choice recorded in design.md D5 — it falls back to `secondary`.
    //
    // It is a BEHAVIOUR CHANGE, deliberately. Under the previous three ternary
    // chains an unknown kind was `filled` (`kind !== "secondary"`), matched
    // neither colour branch so had no fill, took no border, and drew
    // paper-coloured text: an INVISIBLE button that still accepted clicks. A
    // typo should look wrong, not look absent.
    function test_an_unrecognised_kind_falls_back_to_secondary() {
        var secondary = button("secondary");
        var unknown = ["", "Primary", "destuctive", "micro", "ghost",
                       "secondary micro", "undefined"];
        for (var i = 0; i < unknown.length; i++) {
            var b = button(unknown[i]);
            compare(String(b.color), String(secondary.color),
                    "'" + unknown[i] + "' does not fall back to secondary's fill");
            compare(b.border.width, secondary.border.width,
                    "'" + unknown[i] + "' does not fall back to secondary's border");
            compare(String(labelOf(b).color), String(labelOf(secondary).color),
                    "'" + unknown[i] + "' does not fall back to secondary's ink");
            verify(b.border.width > 0 || String(b.color) !== "#00000000",
                   "'" + unknown[i] + "' renders invisibly");
            b.destroy();
        }
        secondary.destroy();
    }

    // ---- text ------------------------------------------------------------

    function nonPlainTextElements(item) {
        var out = [];
        function walk(node) {
            if (!node)
                return;
            if (typeof node.text === "string" && node.textFormat !== undefined
                && node.textFormat !== 0)
                out.push(JSON.stringify(node.text) + " has textFormat "
                         + node.textFormat);
            var kids = node.children;
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i]);
        }
        walk(item);
        return out;
    }

    // Asserted on `textFormat` rather than on the rendered string: `Text.text`
    // returns its source whatever the format is, so a string-based check
    // survives a RichText mutation — measured elsewhere in this piece.
    function test_every_kind_renders_its_label_as_plain_text() {
        for (var i = 0; i < declaredKinds.length; i++) {
            var b = button(declaredKinds[i]);
            var bad = nonPlainTextElements(b);
            compare(bad.length, 0,
                    declaredKinds[i] + " renders a non-plain Text: " + bad.join(" | "));
            b.destroy();
        }
    }

    // The label is the text it was given, unaltered.
    function test_the_label_is_the_text_the_button_was_given() {
        var b = buttonFactory.createObject(null,
            { text: "Moderate all posts of this author", kind: "destructive-outline" });
        compare(labelOf(b).text, "Moderate all posts of this author");
        b.destroy();
    }

    // ---- input -----------------------------------------------------------

    // The signal exists and reaches a listener.
    //
    // WHAT THIS CANNOT SEE: emitting directly bypasses the MouseArea, so this
    // says nothing about whether `onClicked` is wired. Driving a real press
    // needs a window, which a component built with `createObject(null, …)` does
    // not have. The wiring is one line in `FlatButton.qml` and is covered by
    // reading it — said here rather than left as an assertion that would report
    // a green it did not earn.
    function test_clicking_emits_the_signal() {
        var b = button("primary");
        var seen = 0;
        b.clicked.connect(function () { seen++; });
        b.clicked();
        compare(seen, 1, "the clicked signal did not reach a listener");
        b.destroy();
    }
}
