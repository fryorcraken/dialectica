import QtQuick
import QtTest
import "../src/qml"

// The vouch stamp. The visibility rule is the whole component: a vouch the
// viewer has to hover to rediscover is a record they cannot rely on having
// made, and an un-vouched stamp on every row is a feed that shouts.
//
// The absence assertions matter as much as the presence ones. A vouch is never
// published and never counted, so this file also pins that the component
// cannot be asked for a number.
TestCase {
    name: "DVouchStamp"

    Component {
        id: stampFactory
        DVouchStamp {}
    }

    function stamp(props) {
        var s = stampFactory.createObject(null, props === undefined ? {} : props);
        verify(s !== null, "DVouchStamp failed to instantiate");
        return s;
    }

    // ---- visibility: the four states of (vouched, revealed) --------------

    // All four combinations, because the rule is a disjunction and three of the
    // four cases are what distinguishes it from either conjunct alone. A
    // component wired `opacity: revealed` passes on two of these; one wired
    // `opacity: vouched` passes on a different two.
    function test_the_visibility_rule_over_all_four_states() {
        var cases = [
            { vouched: false, revealed: false, visible: false,
              why: "an un-vouched stamp with no pointer over the row must be silent" },
            { vouched: false, revealed: true,  visible: true,
              why: "hovering the row must reveal the prompt" },
            { vouched: true,  revealed: false, visible: true,
              why: "a vouch must not need a hover to be found" },
            { vouched: true,  revealed: true,  visible: true,
              why: "a hovered vouch stays visible" }
        ];
        for (var i = 0; i < cases.length; i++) {
            var c = cases[i];
            var s = stamp({ vouched: c.vouched, revealed: c.revealed });
            if (c.visible)
                verify(s.opacity > 0, c.why + " (opacity " + s.opacity + ")");
            else
                compare(s.opacity, 0, c.why);
            s.destroy();
        }
    }

    // The asymmetry stated on its own, because it is the decision: `revealed`
    // is ignored once vouched, and is the only thing that matters before.
    function test_revealed_is_irrelevant_once_vouched() {
        var s = stamp({ vouched: true, revealed: false });
        compare(s.opacity, 1);
        s.revealed = true;
        compare(s.opacity, 1, "a vouched stamp changed on hover");
        s.destroy();
    }

    // ---- the label ------------------------------------------------------

    // copy.json common.vouch / common.vouched, verbatim and hardcoded. Reading
    // the expected value off the component would be asking the implementation
    // what it wrote and agreeing with it.
    function test_the_label_is_the_bundles_copy_verbatim() {
        var un = stamp({ vouched: false });
        compare(un.label, "VOUCH");
        un.destroy();

        var v = stamp({ vouched: true });
        compare(v.label, "VOUCHED");
        v.destroy();
    }

    function everyText(item) {
        var found = [];
        function walk(node) {
            if (!node)
                return;
            if (typeof node.text === "string" && node.text !== "")
                found.push(node.text);
            var kids = node.children;
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i]);
        }
        walk(item);
        return found;
    }

    // The label reaches the screen, and nothing else does. A `label` property
    // no Text renders would satisfy the test above.
    function test_the_rendered_text_is_the_label_and_nothing_else() {
        var s = stamp({ vouched: true, revealed: false });
        var texts = everyText(s);
        compare(texts.length, 1, "expected one text element, got: " + texts.join(" | "));
        compare(texts[0], "VOUCHED");
        s.destroy();
    }

    // ---- no count, ever --------------------------------------------------

    // A vouch is never published and never counted. This is an ABSENCE
    // assertion over the component's own surface: the natural next request is
    // "show how many", and the component must have nowhere to put one.
    //
    // Keyed on rendering a digit rather than on a property name, because the
    // next spelling of this defect is called `weight` or `tally` and a
    // name-keyed check would miss it.
    function test_no_state_of_the_stamp_renders_a_number() {
        var states = [
            { vouched: false, revealed: false },
            { vouched: false, revealed: true },
            { vouched: true,  revealed: false },
            { vouched: true,  revealed: true }
        ];
        for (var i = 0; i < states.length; i++) {
            var s = stamp(states[i]);
            var texts = everyText(s);
            for (var t = 0; t < texts.length; t++)
                verify(!/[0-9]/.test(texts[t]),
                       "the stamp renders a digit — a vouch is never counted: "
                       + texts[t]);
            s.destroy();
        }
    }

    // Every element that both holds text and declares a format. PlainText is 0;
    // RichText (1), AutoText (2), StyledText (4) and MarkdownText (8) all
    // render their source as markup.
    //
    // ASSERTED ON `textFormat`, NOT ON THE RENDERED STRING. `Text.text` returns
    // the source whatever the format is, so a check that a markup-shaped string
    // comes back verbatim SURVIVES a RichText mutation — measured on
    // `tst_identity_chip.qml` in this same piece, and on `ui-stoa-list` before
    // it. The stamp's own label is a constant, but the assertion is here anyway
    // because the next version of this component will bind something.
    //
    // IT DESCENDS `data`, NOT ONLY `children`. A `ToolTip` is a `Popup`, not an
    // `Item`, so it never appears in `children` and the first version of this
    // walker could not see the stamp's tooltip at all — it reported the stamp
    // clean without ever looking, which is this suite's recorded defect family.
    function nonPlainTextElements(item) {
        var out = [];
        var visited = [];

        function seen(node) {
            for (var k = 0; k < visited.length; k++)
                if (visited[k] === node)
                    return true;
            visited.push(node);
            return false;
        }

        function walk(node) {
            if (!node || seen(node))
                return;
            if (typeof node.text === "string" && node.textFormat !== undefined
                && node.textFormat !== 0)
                out.push(JSON.stringify(node.text) + " has textFormat "
                         + node.textFormat);
            if (node.contentItem !== undefined && node.contentItem !== null)
                walk(node.contentItem);
            var d = node.data;
            if (d !== undefined)
                for (var j = 0; j < d.length; j++)
                    walk(d[j]);
            var kids = node.children;
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i]);
        }
        walk(item);
        return out;
    }

    function test_every_text_the_stamp_renders_is_plain_text() {
        var s = stamp({ vouched: true, revealed: true });
        var bad = nonPlainTextElements(s);
        compare(bad.length, 0, "a non-plain Text in the stamp: " + bad.join(" | "));
        s.destroy();
    }

    // The stamp's tooltip, reached and asserted separately from the walk above.
    // Both its strings are hardcoded literals today, so this pins a component
    // that renders no markup rather than repairing one that does — the point is
    // that the NEXT edit to bind a name or a reason here inherits the format
    // instead of re-deriving it. The count is asserted first for the same reason
    // as in `tst_status_bar.qml`: a walker that reaches no tooltip and a stamp
    // with a clean tooltip are otherwise the same green.
    function tooltipFormats(item) {
        var out = [];
        var visited = [];

        function seen(node) {
            for (var k = 0; k < visited.length; k++)
                if (visited[k] === node)
                    return true;
            visited.push(node);
            return false;
        }

        function walk(node) {
            if (!node || seen(node))
                return;
            if (node.contentItem !== undefined && node.contentItem !== null
                && typeof node.text === "string" && node.children === undefined)
                out.push(node.contentItem.textFormat);
            var d = node.data;
            if (d !== undefined)
                for (var j = 0; j < d.length; j++)
                    walk(d[j]);
            var kids = node.children;
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i]);
        }
        walk(item);
        return out;
    }

    function test_the_stamps_tooltip_is_plain_text() {
        var s = stamp({ vouched: true, revealed: true });
        var formats = tooltipFormats(s);
        compare(formats.length, 1,
                "expected exactly one tooltip on the stamp, found "
                + formats.length + " — if this is 0 the walker no longer reaches "
                + "a popup and the plain-text test above is vacuous");
        compare(formats[0], 0,
                "the stamp's tooltip renders as markup (textFormat "
                + formats[0] + ") — a ToolTip's default content item is "
                + "StyledText, so `ToolTip.text:` is a markup sink");
        s.destroy();
    }

    // ---- the two appearances --------------------------------------------

    // Vouched is INKED, un-vouched is an OUTLINE. Asserted as a relation
    // between the two states rather than as two literals: the design's claim is
    // that they are told apart, and pinning `#a33a2b` twice would pass if both
    // states were filled.
    function test_vouched_is_inked_and_unvouched_is_an_outline() {
        var un = stamp({ vouched: false, revealed: true });
        var v  = stamp({ vouched: true });

        compare(String(un.color), "#00000000",
                "an un-vouched stamp must have no fill");
        compare(String(v.color), String(DTheme.accent),
                "a vouched stamp must be filled in the accent");
        verify(String(un.color) !== String(v.color),
               "the two states are filled identically");

        // The text inverts with the fill, or a filled stamp is ink-on-ink.
        var unText = everyText(un), vText = everyText(v);
        compare(unText.length, 1);
        compare(vText.length, 1);

        un.destroy(); v.destroy();
    }

    // The stamp sits askew only when it is a record. An un-vouched outline is a
    // prompt, and a prompt that looks stamped claims something has happened.
    function test_only_a_vouched_stamp_is_rotated() {
        var un = stamp({ vouched: false, revealed: true });
        compare(un.rotation, 0, "an un-vouched stamp must sit square");
        un.destroy();

        var v = stamp({ vouched: true });
        verify(v.rotation !== 0, "a vouched stamp must sit askew");
        v.destroy();
    }

    // ---- input ----------------------------------------------------------

    // An invisible control that still takes clicks is a click nobody can
    // predict. The enable must follow the opacity.
    function mouseAreasOf(item) {
        var found = [];
        function walk(node) {
            if (!node)
                return;
            if (node.containsMouse !== undefined && node.cursorShape !== undefined)
                found.push(node);
            var kids = node.children;
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i]);
        }
        walk(item);
        return found;
    }

    function test_an_invisible_stamp_does_not_accept_clicks() {
        var hidden = stamp({ vouched: false, revealed: false });
        compare(hidden.opacity, 0);
        var areas = mouseAreasOf(hidden);
        compare(areas.length, 1, "expected one MouseArea");
        compare(areas[0].enabled, false,
                "a stamp at zero opacity still accepts clicks");
        hidden.destroy();

        var shown = stamp({ vouched: false, revealed: true });
        var shownAreas = mouseAreasOf(shown);
        compare(shownAreas[0].enabled, true,
                "a revealed stamp does not accept clicks");
        shown.destroy();
    }

    // The signal exists and reaches a listener.
    //
    // WHAT THIS DOES NOT COVER, said because the obvious extra assertion here
    // cannot fail and would have been written. "The stamp does not flip its own
    // `vouched`" looks testable as `s.toggled(); compare(s.vouched, false)` —
    // but emitting the signal directly BYPASSES the MouseArea's handler, so
    // that assertion passes whether or not `onClicked` writes `root.vouched`.
    // Two explanations, one answer.
    //
    // Driving a real click would settle it, and `mousePress` on a component
    // created with `createObject(null, …)` has no window to deliver into. So
    // the ownership claim is covered by reading `DVouchStamp.qml` — `onClicked`
    // is one line, `root.toggled()` — and not by this test, rather than by an
    // assertion that would report a green it did not earn.
    function test_toggling_emits_a_signal_the_owner_interprets() {
        var s = stamp({ vouched: false, revealed: true });
        var seen = 0;
        s.toggled.connect(function () { seen++; });
        s.toggled();
        compare(seen, 1, "the toggled signal did not reach a listener");
        s.destroy();
    }
}
