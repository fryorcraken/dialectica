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

    // DERIVED FROM THE TABLE, not restated beside it. This was a hand-written
    // array of the five keys, which is the `hand-maintained sweep lists go stale
    // silently` trap this repo has already paid for: a sixth kind added to
    // `FlatButton.kinds` and not to the array was swept by NOTHING, and the
    // suite stayed green. Measured — a deliberately incomplete sixth entry
    // passed this whole file.
    //
    // It also defeated the stated reason for the table. D5 says a sixth kind is
    // "one entry, not four edits in four places that must agree"; the array was
    // a fifth place that had to agree.
    //
    // `Object.keys` on the component's own table means a kind that exists is a
    // kind that is swept, by construction. The assertion below that the derived
    // list is non-empty is what stops a `kinds` that went missing from turning
    // every sweep into a vacuous pass over zero kinds.
    function kindsOf() {
        var b = buttonFactory.createObject(null, { text: "ACT" });
        verify(b !== null, "FlatButton failed to instantiate");
        var keys = Object.keys(b.kinds);
        b.destroy();
        return keys;
    }

    readonly property var declaredKinds: kindsOf()

    // The sweeps above and below are only worth their green if the list they
    // sweep is real. A `kinds` table that went missing, or an `Object.keys` that
    // stopped returning anything, would make every one of them pass over zero
    // kinds — the vacuous-sweep shape this suite is told to watch for.
    function test_the_kind_sweep_has_a_corpus_to_sweep() {
        verify(declaredKinds.length >= 5,
               "the derived kind list holds " + declaredKinds.length
               + " kinds — every sweep in this file is vacuous below five");
        verify(declaredKinds.indexOf("primary") !== -1, "primary is missing");
        verify(declaredKinds.indexOf("secondary") !== -1, "secondary is missing");
        verify(declaredKinds.indexOf("destructive") !== -1,
               "destructive is missing");
    }

    // The set of fields the table means every kind to have, derived as the
    // UNION of what each kind declares. See the comment on the test below for
    // why it is derived rather than restated, and for what it cannot reach.
    function requiredFields(kindsTable, keys) {
        var seen = {};
        var out = [];
        for (var i = 0; i < keys.length; i++) {
            var spec = kindsTable[keys[i]];
            for (var f in spec)
                if (seen[f] === undefined) {
                    seen[f] = true;
                    out.push(f);
                }
        }
        return out;
    }

    // EVERY FIELD OF EVERY KIND IS PRESENT, and this exists because the table's
    // stated guarantee was only half true.
    //
    // `FlatButton`'s comment claims "a missing field is visible where the kind
    // is defined". Measured, and it holds for the COLOUR fields only: a kind
    // missing `textInk` raises `Unable to assign [undefined] to QColor`, which
    // `run-qml-tests.sh`'s `check_bindings` turns into a failure. A kind missing
    // `padY` raises NOTHING — `implicitHeight` becomes `NaN` in total silence,
    // no warning, no binding loop, and `check_bindings` reports clean. A
    // NaN-height button in a RowLayout is a control nobody can see that still
    // accepts clicks, which is the exact failure the fallback to `secondary` was
    // chosen to prevent.
    //
    // The numeric fields do not fail loudly because `NaN` is a valid `real`;
    // the colour ones do because QML type-checks a QColor assignment. So the
    // guarantee needs an assertion rather than a type.
    // THE REQUIRED SET IS DERIVED FROM THE TABLE, not restated beside it — the
    // same correction `declaredKinds` already took, applied to the other axis.
    //
    // It was a literal `["fill", "stroke", "textInk", "font", "padX", "padY"]`,
    // which is the `hand-maintained sweep lists go stale silently` trap in its
    // second dimension: `declaredKinds` made a sixth KIND enter the sweep for
    // free, while a seventh FIELD still had to be typed in here by hand.
    //
    // Measured, and the gap is real. Adding `spec.extraY` to `implicitHeight`
    // with no kind declaring it left THIS test green — it swept the six names
    // it was given and never looked for a seventh. What failed instead was
    // `test_no_kind_renders_with_a_nan_dimension`, which measures the outcome
    // rather than the list, on three kinds at once.
    //
    // So the outcome test is the one that is total over numeric fields, and it
    // is why the gap was not a hole in the suite. This test still earns its
    // place — it names WHICH kind and WHICH field, where the outcome test says
    // only "NaN" — and deriving the set is what makes the name honest.
    //
    // The union across every kind is the right derivation: a field that any
    // kind declares is a field the table means to have, so a sibling missing it
    // fails here. It cannot catch a field NO kind declares (the `extraY` case
    // above) — nothing keyed on the table can, since the table is where the
    // knowledge would have to be — and the outcome tests cover that direction.
    function test_every_kind_declares_every_field() {
        var b = buttonFactory.createObject(null, { text: "ACT" });
        var required = requiredFields(b.kinds, declaredKinds);

        // The derivation must have found a corpus, for the same reason
        // `declaredKinds` is checked: a `requiredFields` that returned nothing
        // would make the loop below a pass over zero fields.
        verify(required.length >= 6,
               "the derived field set holds " + required.length
               + " fields — this sweep is vacuous below six");

        for (var i = 0; i < declaredKinds.length; i++) {
            var spec = b.kinds[declaredKinds[i]];
            for (var f = 0; f < required.length; f++)
                verify(spec[required[f]] !== undefined,
                       declaredKinds[i] + " has no `" + required[f] + "` — a "
                       + "missing colour field raises a binding error, but a "
                       + "missing padX/padY yields implicitHeight NaN in "
                       + "silence: an invisible control that accepts clicks");
        }
        b.destroy();
    }

    // And the geometry each kind actually produces is a number. The field-
    // presence check above would pass a `padY: undefined` spelled as a typo'd
    // key; this catches what that produces, which is the thing a user meets.
    function test_no_kind_renders_with_a_nan_dimension() {
        for (var i = 0; i < declaredKinds.length; i++) {
            var b = button(declaredKinds[i]);
            verify(!isNaN(b.implicitHeight),
                   declaredKinds[i] + " has implicitHeight NaN — it lays out as "
                   + "an invisible control that still accepts clicks");
            verify(!isNaN(b.implicitWidth),
                   declaredKinds[i] + " has implicitWidth NaN");
            verify(b.implicitHeight > 0,
                   declaredKinds[i] + " has implicitHeight "
                   + b.implicitHeight);
            b.destroy();
        }
    }

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

        // AND IT IS SET AT LABEL TYPE, which the two size assertions above
        // cannot see. Both shrink from `padX`/`padY` alone, so changing this
        // kind's `font` from `DTheme.label` to `DTheme.body` left this test —
        // and the whole suite — green, measured. `font` was the one field of
        // the six with no assertion anywhere, which made
        // `test_every_kind_is_either_filled_or_outlined`'s promise that "a
        // sixth entry with a field missing fails here" true of five fields and
        // false of the sixth.
        //
        // Asserted as a RELATION to `secondary` as well as against the token:
        // the token comparison alone passes when both read `undefined`, which
        // is the mechanism that let a renamed colour token through elsewhere in
        // this piece.
        var microLabel = labelOf(micro);
        var fullLabel = labelOf(full);
        compare(microLabel.font.pixelSize, DTheme.label.pixelSize,
                "secondary-micro is not set at label type");
        verify(microLabel.font.pixelSize < fullLabel.font.pixelSize,
               "secondary-micro's type is not smaller than secondary's ("
               + microLabel.font.pixelSize + " vs "
               + fullLabel.font.pixelSize + ")");

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
    //
    // THE SIGNATURE MUST COVER EVERY FIELD THE COMPONENT READS, and an earlier
    // version covered four of the six. It was `color / border.width /
    // border.color / labelColor / implicitHeight`, which sees `fill`, `stroke`,
    // `textInk` and `padY` — and is blind to `font` and `padX` except through
    // the height and width they happen to move.
    //
    // Measured, because the gap is not obvious from reading: a sixth kind
    // identical to `secondary` but set at `DTheme.label` (9px against 15px) with
    // `padY` raised from 14 to 22 so the heights coincide at 35 produced the
    // SAME signature as `secondary` and was reported as "renders identically" —
    // of a button whose text is visibly two-thirds the size. The old signature
    // could only err strict here, never pass a true duplicate, but it named a
    // property it did not measure and its failure message would send a reader
    // hunting a duplicate that is not there.
    //
    // `implicitWidth` and the label's `pixelSize` are added rather than the raw
    // table fields, because what a reader tells apart is what is drawn: two
    // kinds whose different `padX` values produce the same width ARE the same
    // button. The signature is now total over the six fields — `fill`,
    // `stroke`, `textInk` through the colours; `padX`, `padY` and `font`
    // through the two dimensions and the type size.
    function test_no_two_kinds_render_identically() {
        var seen = {};
        for (var i = 0; i < declaredKinds.length; i++) {
            var b = button(declaredKinds[i]);
            var sig = String(b.color) + "/" + b.border.width + "/"
                    + String(b.border.color) + "/" + String(labelOf(b).color)
                    + "/" + b.implicitHeight + "/" + b.implicitWidth
                    + "/" + labelOf(b).font.pixelSize;
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
    //
    // THE CORPUS INCLUDES SEVEN `Object.prototype` MEMBER NAMES, and they are
    // the half that was missing. `kinds[kind] !== undefined` is a lookup that
    // reaches the prototype chain, so `kind: "constructor"` resolves to
    // `Object.prototype.constructor` — a Function, not `undefined` — and the
    // guard passes. `spec` is then that Function, every field read off it is
    // `undefined`, and the button renders white-on-black at NaN size: the same
    // invisible-clickable-control class this fallback exists to close, through
    // a different door.
    //
    // Seven plain typo strings could not see it. They are all still here,
    // because the ordinary typo is the ordinary case; the prototype names are
    // added beside them rather than instead.
    function test_an_unrecognised_kind_falls_back_to_secondary() {
        var secondary = button("secondary");
        var unknown = ["", "Primary", "destuctive", "micro", "ghost",
                       "secondary micro", "undefined",
                       // Object.prototype members — see above.
                       "constructor", "toString", "valueOf", "hasOwnProperty",
                       "__proto__", "isPrototypeOf", "propertyIsEnumerable"];
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
            // The prototype-name case fails here first: a Function `spec`
            // yields `undefined` padding and so a NaN dimension.
            verify(!isNaN(b.implicitHeight) && !isNaN(b.implicitWidth),
                   "'" + unknown[i] + "' has a NaN dimension — the fallback "
                   + "did not fire and `spec` is not a kind");
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
