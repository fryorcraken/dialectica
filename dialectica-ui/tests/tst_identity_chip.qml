import QtQuick
import QtTest
import "../src/qml"

// The footer chip. Two states, and the interesting assertions are the ones
// about what each state must NOT show: a chip that leaks the identity row into
// the no-identity state answers "who am I posting as" with someone who does not
// exist on this machine.
TestCase {
    name: "DIdentityChip"

    Component {
        id: chipFactory
        DIdentityChip {}
    }

    readonly property string addr:
        "k:0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"

    function chip(props) {
        var c = chipFactory.createObject(null, props === undefined ? {} : props);
        verify(c !== null, "DIdentityChip failed to instantiate");
        return c;
    }

    // Only what a user can actually see: an invisible child's text is not on
    // screen, and a walker that ignores visibility would report the
    // no-identity state as showing an identity.
    function visibleText(item) {
        var found = [];
        function walk(node, ancestorsVisible) {
            if (!node)
                return;
            var here = ancestorsVisible && node.visible !== false;
            if (here && typeof node.text === "string" && node.text !== "")
                found.push(node.text);
            var kids = node.children;
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here);
        }
        walk(item, true);
        return found;
    }

    function joined(item) {
        return visibleText(item).join(" | ");
    }

    // ---- identity present ------------------------------------------------

    function test_a_held_identity_shows_the_label_the_name_and_the_address() {
        var c = chip({ hasIdentity: true,
                       generatedName: "kappa delta omicron",
                       identityAddress: addr });
        var texts = visibleText(c);
        var all = texts.join(" | ");

        verify(all.indexOf("CURRENT IDENTITY") >= 0,
               "the mark is unlabelled — see Identicon.qml's placement rule: " + all);
        verify(all.indexOf("kappa delta omicron") >= 0,
               "the generated name is not shown: " + all);

        // The address is ON SCREEN, not one click away. Keyed on the first
        // eight hex characters, which every correct abbreviation and every
        // wrong one must render to be an abbreviation of this address at all.
        verify(all.indexOf("01020304") >= 0,
               "the address is not on screen: " + all);
        c.destroy();
    }

    // The label is not decoration. `Identicon` dropped the person/Stoa contour
    // split on the grounds that POSITION carries it, and attached an obligation
    // to that: a placement the context does not disambiguate must label itself.
    // A footer chip is such a placement. This is the assertion that fails if
    // someone removes the label to save a row.
    function test_the_mark_is_labelled_because_position_alone_cannot_say_whose_it_is() {
        var c = chip({ hasIdentity: true, generatedName: "n", identityAddress: addr });
        verify(joined(c).indexOf("CURRENT IDENTITY") >= 0,
               "the mark must be labelled: " + joined(c));
        c.destroy();
    }

    function identiconsOf(item) {
        var found = [];
        function walk(node, ancestorsVisible) {
            if (!node)
                return;
            var here = ancestorsVisible && node.visible !== false;
            // An Identicon is the only thing here with `address` AND a canvas
            // `size`; AddressLabel has `address` but no `size`.
            if (here && node.address !== undefined && node.size !== undefined
                     && node.muted !== undefined)
                found.push(node);
            var kids = node.children;
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here);
        }
        walk(item, true);
        return found;
    }

    // SPEC.md:30 — "Full address on the slate (the user is choosing a key).
    // Abbreviated elsewhere." A footer chip is elsewhere.
    //
    // THIS WAS UNPINNED, and the gap is not theoretical: setting `full: true`
    // on the chip's `AddressLabel` put all 64 hex characters in the footer and
    // the whole file stayed green — 15 passed, 0 failed, measured. Nothing else
    // here could see it, because
    // `test_a_held_identity_shows_the_label_the_name_and_the_address` keys on
    // the head group `01020304`, which a FULL address renders too. Two
    // explanations, one answer: "it abbreviated" and "it printed everything"
    // both satisfy that assertion.
    //
    // Asserted three ways, none of which the others cover:
    //
    //   * the ellipsis is PRESENT — the abbreviation ran at all;
    //   * a middle group is present, as three groups separated by two
    //     ellipses. Head-and-tail alone is the format `SPEC.md:32-33` says
    //     vanity generators are built to defeat, and it also renders an
    //     ellipsis, so the ellipsis check alone cannot distinguish them;
    //   * the full address is ABSENT. The positive checks would both pass on a
    //     label that printed the abbreviation AND the whole address.
    //
    // Keyed on `AddressLabel`'s rendered text rather than on its `full`
    // property, because what SPEC.md constrains is what reaches the screen: a
    // second abbreviation hand-rolled past the component would set no property
    // and is the defect `SPEC.md:33-34` names ("Implemented once, in
    // AddressLabel.qml — do not hand-roll elision anywhere else").
    function test_the_footer_abbreviates_rather_than_printing_the_whole_address() {
        var c = chip({ hasIdentity: true, generatedName: "n", identityAddress: addr });
        var all = joined(c);

        verify(all.indexOf("…") >= 0,
               "the footer renders no ellipsis — the address is not abbreviated, "
               + "and SPEC.md:30 reserves the full address for the identity "
               + "slate: " + all);

        // The bare hex, ellipses and separators stripped, so the middle group
        // is counted rather than inferred.
        var groups = all.split("…");
        verify(groups.length >= 3,
               "the abbreviation renders " + groups.length + " group(s) — "
               + "head-and-tail alone is the format SPEC.md:32-33 says vanity "
               + "generators are built to defeat: " + all);

        verify(all.indexOf(addr) < 0,
               "the whole address is on screen in the footer — SPEC.md:30 puts "
               + "it on the slate only: " + all);
        c.destroy();
    }

    // The mark is drawn from the SAME address the label prints. A chip whose
    // mark reads a different property renders a stranger's mark beside your
    // name, and nothing above would notice.
    function test_the_mark_is_drawn_from_the_address_the_chip_was_given() {
        var c = chip({ hasIdentity: true, generatedName: "n", identityAddress: addr });
        var marks = identiconsOf(c);
        compare(marks.length, 1, "expected exactly one mark, got " + marks.length);
        compare(marks[0].address, addr, "the mark reads a different address");
        c.destroy();
    }

    // `isPerson` was deliberately removed from `Identicon` — the angular/curved
    // split halved each address's vocabulary and restated what position says.
    // Passing an undeclared property to a QML component is SILENT, so nothing
    // would have failed if the bundle's `isPerson: true` had been carried over;
    // this asserts the property genuinely does not exist on the type.
    function test_the_mark_has_no_isPerson_channel() {
        var c = chip({ hasIdentity: true, generatedName: "n", identityAddress: addr });
        var marks = identiconsOf(c);
        compare(marks.length, 1);
        compare(marks[0].isPerson, undefined,
                "Identicon has an isPerson property again — the contour split "
                + "was removed deliberately (Identicon.qml)");
        c.destroy();
    }

    // ---- an identity claimed but not yet supplied ------------------------

    // THE STATE EVERY SCREEN PASSES THROUGH: `hasIdentity: true` with the
    // address not yet arrived from core.
    //
    // `Identicon` pads a short or malformed address to 64 hex characters so it
    // renders something stable rather than throwing, which is right for the
    // component and wrong here: an empty address pads to 64 zeros and draws a
    // perfectly valid, deterministic, RECOGNISABLE mark — the mark of the zero
    // address — while `CURRENT IDENTITY` sits beside it and `AddressLabel`
    // renders empty.
    //
    // So the chip would assert "this is who you are" over a mark belonging to
    // nobody, with the address that is supposed to be the real identifier
    // absent. That is precisely what `Identicon`'s own placement obligation and
    // this chip's header comment exist to prevent — "a reader who needs to know
    // WHO this is reads the address", and here there is no address to read.
    //
    // Asserted on the MARK rather than on a new property, because the mark is
    // the forgeable channel and the thing a reader would recognise.
    function test_an_identity_with_no_address_yet_draws_no_mark() {
        var c = chip({ hasIdentity: true, generatedName: "", identityAddress: "" });
        compare(identiconsOf(c).length, 0,
                "the chip draws a mark for an empty address — that is the "
                + "zero address's mark, which belongs to nobody, labelled "
                + "CURRENT IDENTITY");
        c.destroy();
    }

    // And it comes back when the address arrives, so the guard above is not
    // satisfied by a chip whose mark never draws.
    function test_the_mark_arrives_with_the_address() {
        var c = chip({ hasIdentity: true, generatedName: "", identityAddress: "" });
        compare(identiconsOf(c).length, 0);
        c.identityAddress = addr;
        compare(identiconsOf(c).length, 1,
                "the mark did not appear when the address arrived");
        compare(identiconsOf(c)[0].address, addr);
        c.destroy();
    }

    // ---- no identity -----------------------------------------------------

    // copy.json feed.readOnlyFooter, verbatim and hardcoded.
    function test_with_no_identity_the_chip_says_what_one_is_for() {
        var c = chip({ hasIdentity: false });
        var all = joined(c);
        verify(all.indexOf("Voting, posting and replying need an identity.") >= 0,
               "the no-identity sentence is missing: " + all);
        // NOT copy.json common.createIdentity — see DIdentityChip.qml, which
        // records why this diverges from the bundle. The chip's arm is layer
        // 2 (a per-Stoa identity), and the bundle's string names layer 1.
        verify(all.indexOf("Choose an identity for this Stoa") >= 0,
               "the choose affordance is missing: " + all);
        // The label must not name layer 1, the master key the user has
        // already created by the time this arm can render.
        verify(all.indexOf("Create an identity") < 0,
               "the chip offers to create a key, not to choose a path: " + all);
        c.destroy();
    }

    // THE ABSENCE HALF, and the one worth having. A chip that leaves the
    // identity row visible claims an identity this machine does not hold.
    function test_with_no_identity_nothing_claims_one() {
        var c = chip({ hasIdentity: false,
                       // Deliberately populated: the chip must not render these
                       // merely because it was handed them.
                       generatedName: "kappa delta omicron",
                       identityAddress: addr });
        var all = joined(c);
        verify(all.indexOf("CURRENT IDENTITY") < 0,
               "the chip claims a current identity with none held: " + all);
        verify(all.indexOf("kappa delta omicron") < 0,
               "the chip shows a name with no identity held: " + all);
        verify(all.indexOf("01020304") < 0,
               "the chip shows an address with no identity held: " + all);

        // And no mark, for the same reason: a mark is a claim about who you are.
        compare(identiconsOf(c).length, 0,
                "the chip draws a mark with no identity held");
        c.destroy();
    }

    // The converse: the prompt must not survive into the held state, or the
    // footer asks for an identity it already has.
    function test_with_an_identity_the_prompt_is_gone() {
        var c = chip({ hasIdentity: true, generatedName: "n", identityAddress: addr });
        var all = joined(c);
        verify(all.indexOf("Voting, posting and replying need an identity.") < 0,
               "the no-identity sentence survives into the held state: " + all);
        verify(all.indexOf("Choose an identity for this Stoa") < 0,
               "the choose button survives into the held state: " + all);
        c.destroy();
    }

    // The state is reactive, not read once at construction.
    function test_losing_an_identity_switches_the_chip_back() {
        var c = chip({ hasIdentity: true, generatedName: "n", identityAddress: addr });
        verify(joined(c).indexOf("CURRENT IDENTITY") >= 0);
        c.hasIdentity = false;
        var all = joined(c);
        verify(all.indexOf("CURRENT IDENTITY") < 0,
               "the identity row survived hasIdentity going false: " + all);
        verify(all.indexOf("Choose an identity for this Stoa") >= 0,
               "the prompt did not appear: " + all);
        c.destroy();
    }

    // ---- the border ------------------------------------------------------

    // The accent border is the chip asking to be read, and it is the ONE state
    // in which the footer wants attention. Asserted as a relation as well as
    // against the tokens, so editing both tokens to one value fails.
    function test_the_border_marks_the_state_that_needs_attention() {
        var held = chip({ hasIdentity: true, generatedName: "n", identityAddress: addr });
        var none = chip({ hasIdentity: false });

        compare(String(held.border.color), String(DTheme.rule2));
        compare(String(none.border.color), String(DTheme.accent));
        verify(String(held.border.color) !== String(none.border.color),
               "both states draw the same border — the prompt does not stand out");

        held.destroy(); none.destroy();
    }

    // ---- peer-supplied text ----------------------------------------------

    // Every element in the tree that both HOLDS text and declares a format,
    // returned as a list of offenders so a failure names which one drifted.
    //
    // Text.PlainText and TextEdit.PlainText are both 0. Anything else —
    // RichText (1), AutoText (2), StyledText (4), MarkdownText (8) — renders
    // its source as markup.
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

    // QML's default `Text.AutoText` sniffs its input and renders markup found
    // there, so every Text here declares a format.
    //
    // THE FIXTURE FEEDS A VALUE THE CONTRACT SAYS CANNOT EXIST, deliberately.
    // `generated-names/spec.md` requires that a name never travels and that
    // every wordlist entry is ASCII and lowercase, so `"<b>bold</b>&amp;"` is
    // not a name any conforming caller can produce. The assertion is still
    // worth having — it pins the format against a name arriving from somewhere
    // the contract did not anticipate — but it is defence in depth rather than
    // a wire-facing guard, and calling this string "peer-supplied" (as this
    // comment once did) describes a larger hole than the code has.
    //
    // ASSERTED ON `textFormat`, NOT ON THE RENDERED STRING, and the difference
    // was measured here rather than assumed. The obvious version — feed the
    // chip `<b>bold</b>` and check the string comes back verbatim — SURVIVES a
    // `textFormat: Text.RichText` mutation, because `Text.text` returns the
    // source string whatever the format is. Measured: 12 passed, 0 failed, with
    // the chip rendering the name as rich text. The `ui-stoa-list` piece was
    // caught by the same gap.
    function test_every_text_the_chip_renders_is_plain_text() {
        var states = [
            { hasIdentity: true, generatedName: "<b>bold</b>&amp;",
              identityAddress: addr },
            { hasIdentity: false, generatedName: "<b>bold</b>&amp;",
              identityAddress: addr }
        ];
        for (var i = 0; i < states.length; i++) {
            var c = chip(states[i]);
            var bad = nonPlainTextElements(c);
            compare(bad.length, 0,
                    "a markup-shaped name may reach a non-plain Text: "
                    + bad.join(" | "));
            c.destroy();
        }
    }

    // And the string itself is unaltered — the chip must not elide, truncate or
    // "clean" a name. Paired with the format assertion above, never alone.
    function test_the_name_is_rendered_verbatim() {
        var name = "<b>bold</b>&amp;";
        var c = chip({ hasIdentity: true, generatedName: name, identityAddress: addr });
        verify(joined(c).indexOf(name) >= 0,
               "the name was altered before rendering: " + joined(c));
        c.destroy();
    }
}
