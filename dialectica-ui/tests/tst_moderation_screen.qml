import QtQuick
import QtTest
import "../src/qml"

// What screen 07 claims, and what it must not.
//
// **The routing half is not here.** That the screen is reachable, that pressing
// its controls makes no core call, and that the way out works are all assertions
// about the navigator driving the real screen, and they live in
// `tst_navigation.qml` — a component suite instantiates the component it tests
// and therefore supplies the reachability whose absence is the defect.
//
// What IS here is what the screen says about itself, which is the half a
// component test can see: the standing notice, the ceiling on what moderating
// reaches, the two lists being two, and that no peer-supplied string on the
// screen is rendered through a markup-interpreting path.
TestCase {
    id: spec
    // `DModerationScreen`, not `ModerationScreen`: the name gate reads this
    // string too, and a bare `ModerationScreen` is the shape that resolves to a
    // host registration at runtime. The gate is right to refuse it here even
    // though this one is a test label — the rule is total over the spelling
    // rather than over where it appears, which is what makes it enforceable.
    name: "DModerationScreen"

    Component {
        id: screenComponent
        DModerationScreen {
            subjectName: "slow cobalt lamplighter"
            subjectAddress: "c04e77b1a92f3c8d4e17b6520fa9c3d1de51a92f7b408c6e35a1f2d98c3714ab"
            subjectExcerpt: "“Keystore mode 0644”"
        }
    }

    function namedAnywhere(item, name) {
        var found = []
        function walk(node) {
            if (!node)
                return
            if (node.objectName === name)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i])
        }
        walk(item)
        return found
    }

    // Every Text the screen renders, whatever its depth. Used by the sweeps
    // below, which must cover elements nobody thought to name.
    function everyText(item) {
        var found = []
        function walk(node) {
            if (!node)
                return
            if (node.text !== undefined && node.font !== undefined
                && node.textFormat !== undefined)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i])
        }
        walk(item)
        return found
    }

    // ---- the standing notice ---------------------------------------------

    // The screen must carry the inertness in what it RENDERS, not merely be
    // inert. A user who presses a destructive control and is told nothing
    // believes either that the post is hidden or that the app is broken, and the
    // first is the dangerous one — on this screen a successful moderation and a
    // control that does nothing look identical.
    function test_the_screen_states_that_it_publishes_nothing() {
        var screen = screenComponent.createObject(null, {})

        var body = spec.namedAnywhere(screen, "inertNoticeBody")
        compare(body.length, 1, "the notice is rendered")
        verify(body[0].text.indexOf("inert") >= 0,
               "it says the controls are inert")
        verify(body[0].visible, "and it is visible rather than merely present")
        screen.destroy()
    }

    // It must name the REASON — no core method — rather than leaving the reader
    // to conclude the feature is broken or half-built.
    function test_the_notice_names_the_missing_core_method_as_the_reason() {
        var screen = screenComponent.createObject(null, {})
        var body = spec.namedAnywhere(screen, "inertNoticeBody")[0]

        verify(body.text.indexOf("core module") >= 0
               && body.text.indexOf("publishing a moderation") >= 0,
               "the reason given is the absent core method: " + body.text)
        screen.destroy()
    }

    // And it must NOT read as a fault. The absence is a scope decision, nothing
    // here is retryable, and a screen presenting it as an error invites a user
    // to keep pressing.
    function test_the_notice_does_not_present_the_absence_as_a_fault() {
        var screen = screenComponent.createObject(null, {})
        var body = spec.namedAnywhere(screen, "inertNoticeBody")[0]

        // Hardcoded words rather than a check derived from the string itself:
        // a test asking the implementation what it wrote and agreeing proves
        // nothing. These are the words that would make it read as a fault.
        var faultWords = ["error", "failed", "failure", "try again", "retry",
                          "unavailable", "broken"]
        for (var i = 0; i < faultWords.length; i++)
            verify(body.text.toLowerCase().indexOf(faultWords[i]) < 0,
                   "the notice must not read as a fault, but contains '"
                   + faultWords[i] + "'")
        screen.destroy()
    }

    // The notice must also say the lists are examples. Without that clause a
    // reader takes them for this peer's moderation state, which is a claim
    // nothing backs — nothing enumerates what a Stoa has moderated at all.
    function test_the_notice_says_the_lists_are_examples() {
        var screen = screenComponent.createObject(null, {})
        var body = spec.namedAnywhere(screen, "inertNoticeBody")[0]

        verify(body.text.indexOf("examples") >= 0,
               "the notice calls the lists examples")
        verify(body.text.indexOf("not a record") >= 0,
               "and denies that they record what this peer has moderated")
        screen.destroy()
    }

    // ---- the ceiling on what moderating reaches --------------------------

    // A permanent property of the design rather than a limitation of this
    // build: a Stoa is peers exchanging signed ops, with no server holding the
    // only copy and no membership to revoke. The wording survives a real
    // publishing path unchanged, which is the test of whether it belongs in a
    // contract.
    function test_the_screen_states_what_moderating_cannot_do() {
        var screen = screenComponent.createObject(null, {})
        var reach = spec.namedAnywhere(screen, "reachStatement")
        compare(reach.length, 1)

        verify(reach[0].text.indexOf("hide this post") >= 0,
               "what it does: asks readers to hide")
        verify(reach[0].text.indexOf("does not delete") >= 0,
               "what it does not: delete")
        verify(reach[0].text.indexOf("nor block a user") >= 0,
               "what it does not: remove a person")
        screen.destroy()
    }

    // ---- the two lists are two -------------------------------------------

    // Moderating an author and moderating one of their posts are different
    // judgements with different scopes, independently reversible. A shared
    // control would make the smaller reversal unavailable.
    function test_the_two_lists_are_separate_with_their_own_controls() {
        var screen = screenComponent.createObject(null, {})

        compare(spec.namedAnywhere(screen, "moderatedAuthorList").length, 1)
        compare(spec.namedAnywhere(screen, "moderatedPostList").length, 1)

        var authorButtons = spec.namedAnywhere(screen, "unmoderateAuthorButton")
        var postButtons = spec.namedAnywhere(screen, "unmoderatePostButton")
        compare(authorButtons.length, screen.moderatedAuthors.length,
                "one control per author row, not one for the list")
        compare(postButtons.length, screen.moderatedPosts.length,
                "one control per post row, not one for the list")
        screen.destroy()
    }

    // The fixtures are `readonly`, so a caller cannot bind a core reply into
    // them and leave the notice claiming an example. That is the cheap half of
    // "no reply is rendered as one"; the expensive half — that nothing calls
    // anything — is `tst_navigation.qml`'s.
    function test_the_fixture_lists_cannot_be_written_from_outside() {
        var screen = screenComponent.createObject(null, {})
        var before = screen.moderatedAuthors.length
        verify(before > 0, "there is something to overwrite")

        // QML THROWS on a write to a readonly property rather than ignoring it,
        // which is the stronger of the two behaviours and the one to pin: a
        // silent refusal would let a caller believe a reply had been bound in.
        // Measured on Qt 6.10.3 — the first version of this test asserted the
        // silent form and failed with "Cannot assign to read-only property".
        var threw = false
        try {
            screen.moderatedAuthors = []
        } catch (e) {
            threw = true
        }
        verify(threw, "binding a reply into the fixture is refused loudly")
        compare(screen.moderatedAuthors.length, before,
                "and the fixture is unchanged, so the notice still describes "
                + "what is on screen")
        screen.destroy()
    }

    // ---- nothing peer-supplied is rendered as markup ----------------------

    // QML's default textFormat is AutoText, which SNIFFS its input and switches
    // to rich text when the string looks like markup. An element left on the
    // default is one dynamic binding away from rendering peer markup, and
    // nothing about that change would look like it touched rendering.
    //
    // A SWEEP over every Text rather than a list of the ones anyone named: this
    // screen renders author names, post excerpts and a subject, all of them
    // peer-supplied, and a hand-written list would go stale the first time a
    // row gained a field.
    function test_every_text_the_screen_renders_is_plain_text() {
        var screen = screenComponent.createObject(null, {})
        var texts = spec.everyText(screen)
        verify(texts.length > 8, "the sweep found the screen's text, "
               + "rather than silently matching nothing: " + texts.length)

        for (var i = 0; i < texts.length; i++)
            compare(texts[i].textFormat, Text.PlainText,
                    "a Text rendering '" + texts[i].text
                    + "' is not pinned to PlainText")
        screen.destroy()
    }

    // The sweep above passes trivially if `everyText` matches nothing, so this
    // drives a name containing markup through the screen and checks it survives
    // as literal characters. It asserts against a string the implementation did
    // not choose.
    function test_a_name_containing_markup_is_rendered_as_its_characters() {
        var screen = screenComponent.createObject(null, {
            subjectName: "<b>not bold</b>"
        })
        var name = spec.namedAnywhere(screen, "subjectName")[0]

        compare(name.text, "<b>not bold</b>",
                "the string reaches the element unchanged")
        compare(name.textFormat, Text.PlainText,
                "and is rendered as the literal characters it contains")
        screen.destroy()
    }

    // ---- addresses go through the view's one abbreviation ----------------

    // A second elision keeping only a head and a tail is the shape
    // vanity-address generators are built to defeat, and this screen is where a
    // reader decides who somebody is. Every address here must carry the middle
    // group, which only `AddressLabel` produces.
    function test_every_address_carries_the_middle_group() {
        var screen = screenComponent.createObject(null, {})
        var texts = spec.everyText(screen)

        // The subject address, abbreviated: head 8, middle 8, tail 6 of the
        // hex, so these three fragments must each appear somewhere on screen.
        // Hardcoded from the address this spec supplies rather than recomputed
        // from DTheme — recomputing would ask the implementation what it did
        // and agree with the answer.
        // The middle group starts at `(length - 8) / 2` = 28 for a 64-char
        // address, which is `AddressLabel`'s arithmetic. Hardcoded here as the
        // literal 28 rather than recomputed from `DTheme` — recomputing would
        // ask the implementation what it did and agree with the answer, which
        // is this repo's named test defect. If the abbreviation moves, this
        // fails and should be re-derived by hand from the new shape.
        var hex = "c04e77b1a92f3c8d4e17b6520fa9c3d1de51a92f7b408c6e35a1f2d98c3714ab"
        compare(hex.length, 64, "a 32-byte address in hex")
        var head = hex.substring(0, 8)
        var middle = hex.substring(28, 36)
        var tail = hex.substring(hex.length - 6)

        var joined = ""
        for (var i = 0; i < texts.length; i++)
            joined += texts[i].text + "\n"

        verify(joined.indexOf(head) >= 0, "the head group is rendered")
        verify(joined.indexOf(middle) >= 0,
               "the MIDDLE group is rendered — its absence is the "
               + "vanity-grindable head-and-tail form")
        verify(joined.indexOf(tail) >= 0, "the tail group is rendered")
        screen.destroy()
    }

    // ---- no authority is claimed -----------------------------------------

    // Whether this peer may moderate this Stoa is a question nothing answers:
    // `getModerationCapability` is designed in PLAN.md §9.1 and does not exist,
    // and `stoa-membership` states that a listed Stoa means the user chose it
    // rather than that the user governs it. A badge here would be the one claim
    // on this screen a user could act on to their cost.
    function test_the_screen_claims_no_moderator_authority() {
        var screen = screenComponent.createObject(null, {})
        var texts = spec.everyText(screen)

        var joined = ""
        for (var i = 0; i < texts.length; i++)
            joined += texts[i].text.toLowerCase() + "\n"

        // NO SPEC: `moderation-view` forbids the screen claiming a moderator
        // status but does not enumerate the words that would. These are the
        // phrasings the reference's MODERATOR badge and its neighbours use.
        var claims = ["you are a moderator", "you moderate", "your stoa",
                      "as a moderator"]
        for (var j = 0; j < claims.length; j++)
            verify(joined.indexOf(claims[j]) < 0,
                   "the screen must not claim moderator status: '"
                   + claims[j] + "'")
        screen.destroy()
    }
}
