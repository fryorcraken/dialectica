import QtQuick
import QtTest
import "../src/qml"

// The three lamps. What matters about them is not that they render — it is
// what a colour is allowed to CLAIM, and in which direction an unrecognised
// state resolves.
//
// WHAT NO TEST HERE CAN SEE, said because this suite has paid for an overbroad
// claim once: a host type-name collision. Under `qmltestrunner` basecamp is
// absent, so `DStatusBar` has no competitor to lose to and an assertion that it
// resolves proves only that the file is there and declared.
// `check_qml_names.py` is what covers the collision. Nothing below claims to.
TestCase {
    name: "DStatusBar"

    Component {
        id: barFactory
        DStatusBar {}
    }

    function bar(props) {
        var b = barFactory.createObject(null, props === undefined ? {} : props);
        verify(b !== null, "DStatusBar failed to instantiate");
        return b;
    }

    // ---- the state rule -------------------------------------------------

    // The three named states pass through untouched. Without this, a
    // `normalisedState` that returned "degraded" for EVERYTHING would satisfy
    // the unknown-state test below and be caught by nothing.
    function test_the_three_named_states_are_returned_unchanged() {
        var b = bar();
        compare(b.normalisedState("ok"), "ok");
        compare(b.normalisedState("degraded"), "degraded");
        compare(b.normalisedState("failed"), "failed");
        b.destroy();
    }

    // NO SPEC: the bundle enumerates ok / degraded / failed and says nothing
    // about a fourth value. This asserts the choice recorded in design.md D2 —
    // an unrecognised state degrades rather than reading as ok.
    //
    // THE DIRECTION IS THE WHOLE POINT. The obvious implementation
    // (`s === "failed" ? … : s === "degraded" ? … : ok`) reads every one of
    // these as GREEN, which is the UI claiming the machine is working on the
    // strength of a string it could not interpret.
    function test_an_unrecognised_state_degrades_rather_than_reading_as_ok() {
        var b = bar();
        var unknown = ["", "OK", "Ok", "okay", "ko", "healthy", "FAILED",
                       "degrade", "unknown", "null", "undefined", "0", "true"];
        for (var i = 0; i < unknown.length; i++) {
            compare(b.normalisedState(unknown[i]), "degraded",
                    "'" + unknown[i] + "' must degrade, not read as ok");
            compare(String(b.lampColor(unknown[i])), String(DTheme.statusDegraded),
                    "'" + unknown[i] + "' must be drawn in the degraded colour");
        }
        b.destroy();
    }

    // NO SPEC: the bundle defaults all three state properties to "ok" and says
    // nothing about what an unbound lamp should claim. This asserts the choice
    // recorded in design.md D2a — a bar nobody has bound yet renders degraded.
    //
    // IT IS THE SAME RULE AS THE TEST ABOVE, applied to the case that reaches
    // the screen first. An unrecognised state degrades because the UI cannot
    // back a green; a bar with NO state bound has even less to back one with,
    // and it is the state every screen passes through between appearing and
    // core answering. Three green lamps on no data at all is the strongest
    // version of the claim D2 exists to refuse.
    //
    // Asserted on the rendered dot and not only on the property, because a
    // default the constructor sets and the binding ignores would pass a
    // property-only check.
    function test_a_bar_nobody_has_bound_yet_claims_nothing() {
        var b = bar();
        compare(b.deliveryState, "degraded", "deliveryState defaults to ok");
        compare(b.storageState, "degraded", "storageState defaults to ok");
        compare(b.zoneState, "degraded", "zoneState defaults to ok");

        var dots = dotsOf(b);
        compare(dots.length, 3);
        for (var i = 0; i < dots.length; i++)
            compare(String(dots[i].color), String(DTheme.statusDegraded),
                    "lamp " + i + " renders green on no data — the interface is "
                    + "asserting this machine works on the strength of nothing");
        b.destroy();
    }

    // Case matters, and it is asserted separately because "OK" is the single
    // most likely value to arrive from a core that formats differently. A
    // case-insensitive comparison would be a reasonable design; it is not the
    // one recorded, and this pins which was chosen.
    function test_the_match_is_case_sensitive() {
        var b = bar();
        compare(b.normalisedState("OK"), "degraded");
        compare(b.normalisedState("ok"), "ok");
        b.destroy();
    }

    // ---- the colours ----------------------------------------------------

    // Each state maps to its OWN colour, and the three are distinct.
    //
    // THREE KINDS OF ASSERTION, and all three are needed. This suite has
    // shipped tests where two explanations give the same answer, so:
    //
    //   * against the TOKEN — catches a lamp wired to the wrong role;
    //   * against a HARDCODED LITERAL — catches the token itself going
    //     missing. Comparing `lampColor("degraded")` with
    //     `DTheme.statusDegraded` alone passes when BOTH are `undefined`,
    //     measured: renaming the token to `statusDegradedTYPO` left this
    //     function green while the lamp rendered white;
    //   * as a RELATION — catches all three tokens being edited to one value,
    //     which the literals would also catch but only by being re-read, where
    //     the relation fails on its own terms.
    function test_each_state_maps_to_its_own_distinct_colour() {
        var b = bar();

        compare(String(b.lampColor("ok")), String(DTheme.statusOk));
        compare(String(b.lampColor("degraded")), String(DTheme.statusDegraded));
        compare(String(b.lampColor("failed")), String(DTheme.statusFailed));

        compare(String(b.lampColor("ok")), "#4f6b3a", "ok is not the green");
        compare(String(b.lampColor("degraded")), "#b5731f",
                "degraded is not the orange");
        compare(String(b.lampColor("failed")), "#a33a2b", "failed is not the red");

        verify(String(DTheme.statusOk) !== String(DTheme.statusDegraded),
               "ok and degraded render identically");
        verify(String(DTheme.statusOk) !== String(DTheme.statusFailed),
               "ok and failed render identically");
        verify(String(DTheme.statusDegraded) !== String(DTheme.statusFailed),
               "degraded and failed render identically");
        b.destroy();
    }

    // The three colour tokens exist and are real colours. A token that is
    // missing reads `undefined`, which `check_bindings` would catch only if a
    // binding used it — this asserts it directly.
    function test_the_status_tokens_are_defined_colours() {
        var names = ["statusOk", "statusDegraded", "statusFailed"];
        for (var i = 0; i < names.length; i++) {
            var v = DTheme[names[i]];
            verify(v !== undefined, "DTheme." + names[i] + " is undefined");
            verify(String(v).charAt(0) === "#",
                   "DTheme." + names[i] + " is not a colour: " + String(v));
        }
    }

    // ---- the lamps ------------------------------------------------------

    function labelsOf(item) {
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

    // Three lamps, these three, IN THIS ORDER. The order is part of what the
    // footer means — a reader learns the position — so it is asserted as a
    // sequence and not as a set.
    function test_the_three_lamps_are_delivery_storage_zone_in_that_order() {
        var b = bar();
        var labels = labelsOf(b);
        compare(labels.length, 3, "expected exactly three lamps, got: "
                                  + labels.join(" "));
        compare(labels[0], "DELIVERY");
        compare(labels[1], "STORAGE");
        compare(labels[2], "ZONE");
        b.destroy();
    }

    // Every dot the bar draws, in lamp order. Keyed on `radius` because that is
    // what distinguishes a lamp dot from the pill border around it: both are
    // Rectangles, and only the dot is round.
    function dotsOf(item) {
        var found = [];
        function walk(node) {
            if (!node)
                return;
            if (node.radius !== undefined && node.radius > 0
                && node.color !== undefined && node.implicitWidth === 7)
                found.push(node);
            var kids = node.children;
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i]);
        }
        walk(item);
        return found;
    }

    // Each lamp is driven by its OWN state property. Without this, a component
    // wiring all three dots to `deliveryState` would pass every test above.
    function test_each_lamp_reads_its_own_state_property() {
        var b = bar({ deliveryState: "ok",
                      storageState: "failed",
                      zoneState: "degraded" });
        var dots = dotsOf(b);
        compare(dots.length, 3, "expected three lamp dots, got " + dots.length);
        compare(String(dots[0].color), String(DTheme.statusOk), "delivery dot");
        compare(String(dots[1].color), String(DTheme.statusFailed), "storage dot");
        compare(String(dots[2].color), String(DTheme.statusDegraded), "zone dot");
        b.destroy();
    }

    // A state change reaches the dot. A component that read its state once at
    // construction would pass the test above and fail here.
    function test_a_state_change_repaints_its_lamp() {
        var b = bar({ storageState: "ok" });
        var dots = dotsOf(b);
        compare(String(dots[1].color), String(DTheme.statusOk));
        b.storageState = "failed";
        compare(String(dots[1].color), String(DTheme.statusFailed),
                "the storage dot did not follow storageState");
        b.destroy();
    }

    // `lampState`, not `state`. The bundle's spelling shadows `QQuickItem`'s
    // built-in `state` — measured on Qt 6.10.3: the redeclaration instantiates,
    // holds the value, and the `states` machine never fires. This asserts the
    // built-in is still OURS to use, which is exactly what the rename bought.
    //
    // It fails if a lamp is renamed back to `state`: assigning "degraded" would
    // then set the item's state name, no `State` of that name exists, and the
    // built-in would report "degraded" rather than the "" this asserts.
    function test_a_lamp_does_not_shadow_the_item_state_machine() {
        var b = bar({ deliveryState: "degraded" });
        var dots = dotsOf(b);
        compare(dots.length, 3);
        // The lamp is the dot's parent's parent (dot -> RowLayout -> Lamp).
        var lamp = dots[0].parent.parent;
        compare(lamp.lampState, "degraded", "lampState does not hold the value");
        compare(String(lamp.state), "",
                "the item's built-in `state` was written to — a lamp property "
                + "is shadowing QQuickItem.state");
        b.destroy();
    }

    // Every element that both holds text and declares a format. PlainText is 0;
    // anything else renders its source as markup. Asserted on `textFormat` and
    // not on the rendered string, because `Text.text` returns the source
    // whatever the format is — measured in this piece, where the string-based
    // form survived a RichText mutation.
    //
    // IT DESCENDS `data`, NOT ONLY `children`, and that is the whole repair.
    // A `ToolTip` is a `Popup`, which is not an `Item` and so never appears in
    // `children` — the first version of this walker returned `[]` for a bar
    // rendering `"<b>OWNED</b> <img src=x>"` through a tooltip whose content
    // item was `StyledText`. Two explanations gave the same answer there ("no
    // markup rendered" and "the walker never looked"), which is this suite's
    // recorded defect family. `data` carries every child, visual or not, so a
    // popup's content item is now walked like any other element.
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
            // A popup (ToolTip, Menu, Dialog) hangs off `data` and holds its
            // Text in `contentItem`, which is not in its `children` either.
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

    // EVERY element this walker visits that declares a format, offending or
    // not. This is the corpus `nonPlainTextElements` filters, and it exists
    // because that function's emptiness is ambiguous on its own: "nothing
    // renders markup" and "the walker reached nothing" are the same `[]`, which
    // is this suite's recorded defect family exactly.
    function formatDeclaringElements(item) {
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
            if (typeof node.text === "string" && node.textFormat !== undefined)
                out.push(node);
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

    // THE COUNT IS ASSERTED BEFORE THE FORMATS, and it was not.
    //
    // Measured, and this is the original defect reproduced rather than a
    // hypothetical: with the attached `ToolTip.text:` binding restored to
    // `DStatusBar.qml` — the real markup sink this piece exists to close — and
    // this walker narrowed back to `children`, THIS TEST PASSED. Fourteen
    // passed, one failed, and the one that failed was
    // `test_every_tooltip_the_bar_opens_is_plain_text` on its own count.
    //
    // So the repair the author made to the walker was real, and the count
    // assertion that protects it was added to the OTHER tooltip test but not to
    // this one. This test is the one named in the finding, and until now it
    // still could not fail for the reason its name gives: a walker narrowed
    // back to `children` makes it report a clean bar without looking.
    //
    // Six is what the bar renders: three lamp labels and three tooltip content
    // items. Asserted as a floor rather than an equality so that adding a
    // fourth text element is not a test edit, but zero and three both fail —
    // three being what a `children`-only walker sees.
    function test_every_text_the_bar_renders_is_plain_text() {
        var b = bar({ deliveryText: "<b>OWNED</b> <img src=x>",
                      storageText: "<i>x</i>",
                      zoneText: "<b>y</b>" });

        var all = formatDeclaringElements(b);
        verify(all.length >= 6,
               "the walker reached " + all.length + " format-declaring elements, "
               + "expected at least 6 (three lamp labels and three tooltip "
               + "content items) — below that it is not reaching the popups, and "
               + "the format assertion below passes without looking");

        var bad = nonPlainTextElements(b);
        compare(bad.length, 0, "a non-plain Text in the bar: " + bad.join(" | "));
        b.destroy();
    }

    // THE WALKER ABOVE MUST ACTUALLY REACH A TOOLTIP, asserted separately
    // because the two claims fail independently: a walker that reaches nothing
    // reports a clean bar, and so does a bar that is genuinely clean. This
    // counts what was found, so a walker narrowed back to `children` fails here
    // with "found 0" rather than passing quietly.
    //
    // A lamp's `explanation` is the one free string this component takes — the
    // sentence core supplies to explain a degraded lamp, which is where a
    // peer-supplied fragment ends up — so this is the sink that matters.
    function textFormatsOfTooltips(item) {
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
            // A ToolTip: it has a content item and a text, and is not an Item
            // (no `children`).
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

    // Measured on Qt 6.10.3: a `ToolTip`'s DEFAULT content item is a `Text` at
    // `textFormat: Text.StyledText` (2), so `ToolTip.text: lamp.explanation`
    // renders markup. Against `"<b>OWNED</b> x"` the attached form painted at
    // contentWidth 56.66 where a PlainText element paints 102.02 — the tags
    // were consumed, not drawn. `contentItem.text` returns the raw source
    // either way, which is why this reads `textFormat` and not the string.
    function test_every_tooltip_the_bar_opens_is_plain_text() {
        var b = bar({ deliveryText: "<b>OWNED</b> <img src=x>",
                      storageText: "<i>x</i>",
                      zoneText: "<b>y</b>" });
        var formats = textFormatsOfTooltips(b);
        compare(formats.length, 3,
                "expected one tooltip per lamp, found " + formats.length
                + " — if this is 0 the walker no longer reaches a popup and "
                + "the plain-text test above is vacuous");
        for (var i = 0; i < formats.length; i++)
            compare(formats[i], 0,
                    "lamp " + i + "'s tooltip renders as markup (textFormat "
                    + formats[i] + ") — a lamp's explanation is peer-influenced "
                    + "text and a ToolTip's default content item is StyledText");
        b.destroy();
    }

    // ---- tooltips -------------------------------------------------------

    // The explanation is carried, not invented. A lamp given no text must not
    // substitute one — the tooltip is where the sentence that a lamp cannot
    // claim on its own lives, and a default would be the component claiming it.
    function test_a_lamp_given_no_explanation_invents_none() {
        var b = bar();
        compare(b.deliveryText, "");
        compare(b.storageText, "");
        compare(b.zoneText, "");
        var labels = labelsOf(b);
        for (var i = 0; i < labels.length; i++)
            verify(labels[i] === "DELIVERY" || labels[i] === "STORAGE"
                   || labels[i] === "ZONE",
                   "the bar renders text it was not given: " + labels[i]);
        b.destroy();
    }
}
