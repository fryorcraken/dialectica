import QtQuick
import QtTest
import "../src/qml"

// The Stoa list and the join preview, driven through a fake bridge.
//
// What is under test is the screens' own state machines, not a re-implementation
// of them — the shape tst_feed_states.qml established, and the reason for it is
// that a test asserting against its own copy of the logic passes whatever the
// screen does.
//
// Three things this file is written to catch, because each has a plausible bug
// that leaves every other assertion green:
//
//   1. **Empty membership rendered as a failed read, or the reverse.** A user
//      told they belong to nothing will re-join Stoas they are already in.
//   2. **A join that happened without the user asking.** The fake bridge records
//      every method it was called with, so "no join call has been made" is an
//      assertion about a log rather than about a screen's appearance.
//   3. **A share affordance offered for a Stoa whose record the view has not
//      got.** What it would produce is a string that fails to verify on somebody
//      else's machine, as a refusal they cannot explain.
//
// ---- the defect family this file kept producing --------------------------
//
// **Assert the property the user is affected by, not the value feeding it.**
// Five instances have now been found in this one file, four of them by someone
// other than its author, and every one left the whole suite green:
//
//   - a title's markup safety asserted via `Text.text`, which is the SOURCE
//     string and is unchanged by `textFormat` — passed against StyledText;
//   - a share affordance's absence asserted via `canShare()`, the computation
//     feeding the `visible:` binding rather than the binding;
//   - a joined panel's presence asserted via `joinState`, the string feeding it;
//   - a feed's absence asserted via `screenShown`, a derived string;
//   - and the subtlest, found here by mutation: the per-Stoa-identity absence
//     asserted over a FRESH PREVIEW, whose body says nothing whatever about
//     what joining does. Planting "generates you an identity for it alone" in
//     the joined panel — the text a user actually reads — left that assertion
//     PASSING, because the panel was not rendered in the state being scanned.
//
// What the last one shows is that an absence assertion is only as strong as its
// corpus: one with no candidate in it proves nothing. The absence assertions in
// this file are written accordingly — each states what it IS scanning before
// saying what is not there — and
// `test_the_absence_assertions_scan_the_body_and_not_only_the_apparatus`
// pins that corpus so it cannot silently shrink.
//
// One reason it can shrink is already scheduled: the APPARATUS column is
// annotation explaining the design, not interface, and a separate piece is
// removing it from the shipped view. Most of what a whole-screen scan sees on
// the join screen is margin note rather than anything a user acts on — the
// measurement is at `bodyText`, where it justifies the subtraction, and is
// stated once so it cannot go half stale when the column goes. `bodyText`
// exists to scan only what ships. Verified by running this file against a
// hidden apparatus column: all tests pass, because none of them depends on
// annotation.
//
// ---- the second family: a literal where a meaning was required -------------
//
// Distinct from the one above and worth keeping separate, because the fix is
// different. There the assertion read the wrong VALUE; here it reads the right
// value and asks the wrong QUESTION — matching a phrase where the requirement
// is about what a sentence claims. Such a test fails on an innocent reword and
// passes on a fluent lie, which is the worst pair of properties available.
//
//   - three refusal messages asserted to be three DIFFERENT strings. A tester
//     reworded one to actively misinform and both distinguishability tests
//     stayed green: misinforming strings are still distinct strings.
//   - the malformed-paste refusal blocking three exact phrasings. A reviewer
//     measured two equivalents that passed — "could not be confirmed against
//     its address", "failed the check against the address it names" — each
//     telling a user who truncated their own paste to blame their sender.
//   - the address note pinned by three required substrings. A note carrying all
//     three and reading "…which confirms this is the Stoa you were sent"
//     passed, asserting the single thing the spec says the copy may never claim.
//
// So those assertions now test STRUCTURE — a verb of checking together with the
// address as its object; an explicit denial of provenance — rather than
// vocabulary. And two absence assertions that used bare `indexOf` gained word
// boundaries, because `members` matches `membership` and the honest copy
// "Your membership was read without error" is one screen-state change from
// making the count assertion fail for a reason that is not a defect.
//
// Both fixes were verified in BOTH directions, which is the part that is easy
// to skip: a regex that never matches is indistinguishable from a repaired
// assertion until you plant the thing it must still catch.
TestCase {
    id: spec
    name: "StoaScreens"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    // A bridge that answers each method from a map AND records every call.
    //
    // The recording half is not incidental — it is the only way to assert that a
    // call was NOT made. A screen that renders a preview without joining and a
    // screen that joins silently look identical from the outside; the difference
    // is entirely in what reached the bridge.
    //
    // A reply may be a FUNCTION of the request, for a test that needs different
    // answers to different requests. A fake answering every request alike
    // cannot tell "answered for this reference" from "still showing the last
    // one", which is exactly the difference the lookup tests exist to see.
    function bridgeFor(replies) {
        return {
            calls: [],
            callModule: function (module, method, args) {
                this.calls.push({ method: method, args: args })
                var reply = replies[method]
                if (reply === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                if (typeof reply === "function")
                    return reply(JSON.parse(args[0]))
                return reply
            }
        }
    }

    // A `get_stoa` success, as the core spells it.
    function getStoaReply(stoa, isGenesisFallback, title, description) {
        return JSON.stringify({
            stoa: stoa, title: title, description: description,
            policy: "open", isGenesisFallback: isGenesisFallback
        })
    }

    function callsTo(bridge, method) {
        var n = 0
        for (var i = 0; i < bridge.calls.length; i++)
            if (bridge.calls[i].method === method)
                n++
        return n
    }

    function lastArgsTo(bridge, method) {
        for (var i = bridge.calls.length - 1; i >= 0; i--)
            if (bridge.calls[i].method === method)
                return bridge.calls[i].args
        return null
    }

    // Every descendant carrying `name`, visible or not.
    //
    // Distinct from `visibleNamed` on purpose. Absence assertions must use the
    // visible form — that is the property a user is affected by. But a test that
    // needs to REACH a screen in order to drive it needs it whether or not it is
    // currently on screen: `Main.qml` mounts all three screens and toggles
    // `visible`, so the join screen exists from startup and is hidden until a
    // reference is previewed. Using the visible walker to find it returned
    // nothing and produced "Cannot call method 'join' of undefined" — a test
    // failing for a reason unrelated to the defect it was written for.
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

    // Every visible descendant of `item` carrying `name`, at any depth.
    //
    // `visible` is checked on each ancestor rather than trusting the element's
    // own flag: QML's `visible` is not inherited into the property, so a button
    // inside a hidden panel still reports `visible: true` for itself. An absence
    // assertion that missed that would pass on a screen showing the affordance.
    function visibleNamed(item, name) {
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (node.objectName === name && here)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    // Collect the `text` of every visible Text-ish descendant, so a claim about
    // what a screen does NOT say can be checked against everything it does say.
    function visibleText(item) {
        var out = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && typeof node.text === "string" && node.text !== "")
                out.push(node.text)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return out.join("\n")
    }

    // Every visible element a user can type into.
    //
    // **Keyed on behaviour, not on a name or a placeholder.** A walker looking
    // for `objectName: "creatorKeyField"` pins one spelling of the defect; the
    // next one is called `identityPicker`. And a walker keyed on
    // `placeholderText` finds nothing even on the clean tree — the shipped title
    // field is a bare `TextInput` without one, which is the trap the spec-test
    // reviewer flagged after hitting it.
    //
    // These three properties are what `TextInput` and `TextEdit` expose and a
    // `Text` does not, so the set is "things that accept typing" rather than
    // "things that look like a field". `readOnly` is checked because a read-only
    // TextEdit is a display element, not somewhere a user enters a key — the
    // DClipboardSink is exactly that and must not be counted.
    function editableInputs(item) {
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && node.echoMode !== undefined
                     && node.cursorPosition !== undefined
                     && node.readOnly === false)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    // What those inputs currently hold, for a failure message that names which
    // field is the unexpected one rather than only how many there were.
    function describeInputs(inputs) {
        var out = []
        for (var i = 0; i < inputs.length; i++)
            out.push("<" + String(inputs[i].text) + ">")
        return out.join(" ")
    }

    // Every visible element that RENDERS TEXT derived from `address`.
    //
    // Keyed on the rendered text rather than on an `address` property, for two
    // reasons. A hand-rolled elision written as a bare `Text` has no `address`
    // property at all, and would simply not be found — an absence that reads as
    // "nothing renders the address" rather than as the defect it is. And the
    // `Identicon` beside every address DOES carry `address`, so a property-keyed
    // walker returns two elements where one renders characters.
    //
    // So: anything whose text contains the address's first eight characters,
    // which both the right form and every wrong one must render to be an
    // abbreviation of it at all.
    function addressElementsFor(item, address) {
        var head = address.slice(0, 8)
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && typeof node.text === "string" && node.text.indexOf(head) >= 0)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    // The text of the apparatus column alone.
    //
    // **The apparatus is annotation explaining the design, not interface**, and
    // a separate piece is removing it from the shipped view. That matters to
    // every absence assertion in this file: `visibleText` walks the whole
    // ScreenFrame, so more than half of what those assertions scan on the join
    // screen is margin note rather than anything a user acts on. Measured, not
    // estimated — 747 of 1371 characters on a rendered preview.
    //
    // An absence assertion whose corpus shrinks under it does not fail; it
    // quietly starts proving less while its name goes on claiming the same
    // thing. So the two assertions that need a corpus check use this to state
    // what they are scanning, and `bodyText` below to scan only what ships.
    // Note it walks each note through `visibleText`, which checks the note's own
    // ancestors — so a column hidden by its container yields "" here, and
    // `bodyText` then correctly reports the whole screen as body. That is the
    // behaviour the apparatus's removal needs, and it was got wrong first time:
    // an earlier version returned the notes' text whether or not anything
    // rendered them, which made a hidden column look like a column still there.
    function apparatusText(screen) {
        var out = []
        var app = screen.apparatus
        if (app === undefined || app === null)
            return ""
        for (var i = 0; i < app.length; i++) {
            var t = visibleText(app[i])
            if (t !== "")
                out.push(t)
        }
        return out.join("\n")
    }

    // Everything the screen renders EXCEPT the apparatus column — that is, the
    // part that survives the apparatus's removal and the part a user reads.
    //
    // Derived by subtraction rather than by walking a named child, deliberately:
    // `ScreenFrame` exposes its body as a default property alias with no
    // objectName to find, and a lookup keyed on internal structure would break
    // silently when that structure changes. Subtraction breaks loudly instead —
    // if the apparatus stops being separable, `bodyText` returns the whole
    // screen and the corpus assertions below catch it.
    function bodyText(screen) {
        var whole = visibleText(screen)
        var app = apparatusText(screen)
        if (app === "")
            return whole
        var at = whole.indexOf(app)
        return at < 0 ? whole : (whole.slice(0, at) + whole.slice(at + app.length))
    }

    Component { id: listComponent; DStoaListScreen {} }
    Component { id: joinComponent; DJoinScreen {} }
    Component { id: sinkComponent; DClipboardSink {} }

    function makeList(replies, props) {
        Core.bridge = bridgeFor(replies)
        return listComponent.createObject(null, props === undefined ? {} : props)
    }

    // The list screen inside a parentless host Item, for the tests that hide
    // and re-show it.
    //
    // **Neither obvious parent works, both observed on Qt 6.10.3.** Created
    // with no parent (`makeList`), the screen reads `visible` true at
    // completion, but after `visible = false` a `visible = true` leaves it
    // invisible and emits no `visibleChanged` — a parentless item's effective
    // visibility needs a parent to become true again. Created under the
    // TestCase, it reads `visible` false from the start, because the runner
    // never shows the TestCase. A parentless host is the shape that behaves
    // like `Main.qml`, which mounts the screen in an always-shown layout: the
    // host is never toggled, so its own visibility stays true.
    Component { id: hostComponent; Item {} }

    function makeShownList(replies) {
        Core.bridge = bridgeFor(replies)
        var host = hostComponent.createObject(null, {})
        return listComponent.createObject(host, {})
    }

    function makeJoin(replies, props) {
        Core.bridge = bridgeFor(replies)
        return joinComponent.createObject(null, props)
    }

    // ---- the three read states ------------------------------------------

    function test_an_empty_membership_is_the_ok_state_and_reports_no_failure() {
        var screen = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })

        compare(screen.readState, "ok",
                "a membership that answered with nothing is a SUCCESS, not a failure")
        compare(screen.visibleRows.length, 0)
        compare(screen.failure, "", "an empty read has nothing to report")
        screen.destroy()
    }

    function test_an_unreadable_membership_is_a_failure_carrying_the_cores_words() {
        var screen = makeList({ "list_stoas": '{"error":"the membership store at /x/membership.sqlite is locked"}' })

        compare(screen.readState, "failed")
        compare(screen.visibleRows.length, 0, "a failure must not leave rows behind")
        verify(screen.failure.indexOf("locked") >= 0,
               "core's reason names a fix and must reach the screen unreworded, got: "
               + screen.failure)
        screen.destroy()
    }

    function test_the_empty_state_and_the_failed_state_are_different_states() {
        // Stated directly rather than inferred from the two tests above: the
        // same request against two stores must not leave the screen in one
        // state.
        var empty = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var broken = makeList({ "list_stoas": '{"error":"membership is locked"}' })

        verify(empty.readState !== broken.readState,
               "in-no-Stoas and cannot-read must never reach the same state")
        compare(empty.readState, "ok")
        compare(broken.readState, "failed")

        // And what they RENDER differs, not only what they hold. A pair of
        // states rendered through the same branch would satisfy the compare
        // above and still show the user one screen.
        verify(visibleText(empty) !== visibleText(broken),
               "the two states must not render the same text")
        empty.destroy()
        broken.destroy()
    }

    function test_a_success_without_an_items_array_is_a_named_failure() {
        var noItems = makeList({ "list_stoas": '{"page":0,"hasMore":false}' })
        compare(noItems.readState, "failed",
                "a success with no items array must not render as an empty membership")
        verify(noItems.failure.length > 0, "the failure must name what was wrong")
        noItems.destroy()

        // `items` present but not an array — a shape that passes an
        // `=== undefined` check and then has `.length` read off it.
        var notArray = makeList({ "list_stoas": '{"items":"lots","page":0,"hasMore":false}' })
        compare(notArray.readState, "failed", "items must be an ARRAY, not merely present")
        notArray.destroy()
    }

    function test_an_unreachable_core_is_a_failure_rather_than_an_empty_list() {
        Core.bridge = null
        var screen = listComponent.createObject(null, {})
        compare(screen.readState, "failed")
        verify(screen.failure.toLowerCase().indexOf("not reachable") >= 0,
               "the screen must say the core is unreachable, got: " + screen.failure)
        screen.destroy()
    }

    function test_a_reply_that_is_not_json_is_a_failure_rather_than_a_value() {
        var screen = makeList({ "list_stoas": '<html>gateway timeout</html>' })
        compare(screen.readState, "failed")
        compare(screen.visibleRows.length, 0, "no Stoa may be obtained from a non-JSON reply")
        screen.destroy()
    }

    // ---- what a row carries ---------------------------------------------

    function test_a_row_carries_the_address_as_well_as_the_title() {
        var addr = "7f3a91c4" + "00".repeat(28)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Nym Research"}],'
                        + '"page":0,"hasMore":false}'
        })

        compare(screen.readState, "ok")
        var shown = visibleText(screen)
        verify(shown.indexOf("Nym Research") >= 0, "the founding title must be rendered")
        // The abbreviation is AddressLabel's, so the head and tail are what is on
        // screen — the assertion is that the ADDRESS is there in some form, not
        // that a second elision was written.
        verify(shown.indexOf("7f3a91c4") >= 0,
               "the row must carry the address, not the title alone: " + shown)
        screen.destroy()
    }

    function test_two_stoas_with_the_same_title_render_differently() {
        var a = "7f3a91c4" + "11".repeat(28)
        var b = "b02d5e77" + "22".repeat(28)
        var screen = makeList({
            "list_stoas": '{"items":['
                        + '{"stoa":"' + a + '","foundingTitle":"Nym Research"},'
                        + '{"stoa":"' + b + '","foundingTitle":"Nym Research"}'
                        + '],"page":0,"hasMore":false}'
        })

        compare(screen.visibleRows.length, 2, "both rows must be rendered")
        var shown = visibleText(screen)
        // The prefixes are hardcoded rather than derived from the fixture: a
        // check that merely asserted "the two rows differ" would pass on two
        // rows differing in a whitespace character.
        verify(shown.indexOf("7f3a91c4") >= 0, "the first address must be on screen")
        verify(shown.indexOf("b02d5e77") >= 0, "the second address must be on screen")
        screen.destroy()
    }

    function test_an_empty_founding_title_still_gets_a_row_with_its_address() {
        var addr = "1ce0aa38" + "33".repeat(28)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":""}],'
                        + '"page":0,"hasMore":false}'
        })

        compare(screen.readState, "ok")
        compare(screen.visibleRows.length, 1, "an empty title is legal and must not be omitted")
        var shown = visibleText(screen)
        verify(shown.indexOf("1ce0aa38") >= 0, "the row must carry its address")
        // No substitute title. "Untitled" is a title no peer agrees on, and a
        // row showing one would be asserting something the record does not say.
        verify(shown.toLowerCase().indexOf("untitled") < 0,
               "no substitute title may stand in for an empty one")
        verify(shown.indexOf("(no title)") < 0)
        screen.destroy()
    }

    function test_a_title_containing_markup_is_rendered_as_literal_characters() {
        var addr = "aa".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '",'
                        + '"foundingTitle":"<b>bold</b> &amp; <img src=x>"}],'
                        + '"page":0,"hasMore":false}'
        })

        // **The element's `textFormat`, not its `text`.** An earlier version of
        // this test asserted only that `text` contained "<b>bold</b>", and it
        // PASSED against an implementation switched to `Text.StyledText` —
        // proved by mutation. `text` is the SOURCE string and is unchanged by
        // the format, so a test reading it cannot see the rendering at all,
        // which is precisely the property under test. The check has to be on the
        // format, and the source string is asserted alongside it so that a
        // renamed or vanished element fails rather than trivially satisfying it.
        var titles = spec.titleElementsFor(screen, "<b>bold</b> &amp; <img src=x>")
        compare(titles.length, 1, "the title element must be found by its content")
        compare(titles[0].textFormat, Text.PlainText,
                "a peer-supplied title must never reach a markup-interpreting path")
        // AutoText is the trap specifically: it SNIFFS its input and switches to
        // rich text when the string looks like markup, so an element left on the
        // default renders this title as markup while a `text` assertion passes.
        verify(titles[0].textFormat !== Text.AutoText)
        verify(titles[0].textFormat !== Text.StyledText)
        verify(titles[0].textFormat !== Text.RichText)
        screen.destroy()
    }

    // Every visible descendant whose `text` is exactly `value` and which has a
    // `textFormat` — the Text elements carrying peer-supplied content.
    function titleElementsFor(item, value) {
        var found = []
        function walk(node, ancestorsVisible) {
            if (!node)
                return
            var here = ancestorsVisible && node.visible !== false
            if (here && node.text === value && node.textFormat !== undefined)
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i], here)
        }
        walk(item, true)
        return found
    }

    // ---- the row treatment ------------------------------------------------
    //
    // `stoa-navigation-view`: "Where one listed Stoa ends and the next begins is
    // rendered, not left to spacing". The requirement is about which half of a
    // row belongs to which Stoa — the same concern as "a row carries the address
    // as well as the title", read one level out — so these assertions are about
    // the COUNT of boundaries against the count of rows, never about how a
    // boundary is drawn. The capability's Purpose puts "colours, type, metrics"
    // outside itself by name, and pinning the hairline's thickness or colour
    // here would contract something that spec declines to.

    function test_every_rendered_row_is_separated_from_the_next() {
        // THREE rows, not one. A screen that drew a single separator for the
        // whole list — or one that dropped it on the last row, which is the
        // common convention and the thing the requirement rules out — passes any
        // assertion phrased as "a separator exists". Only the count against the
        // row count distinguishes those, so that is what is asserted: N rows
        // produce N boundaries, and adding a row adds one.
        var screen = makeList({
            "list_stoas": '{"items":['
                        + '{"stoa":"7f3a91c4' + "11".repeat(28) + '","foundingTitle":"Nym Research"},'
                        + '{"stoa":"b02d5e77' + "22".repeat(28) + '","foundingTitle":"Transport Notes"},'
                        + '{"stoa":"1ce0aa38' + "33".repeat(28) + '","foundingTitle":"Keystore Notes"}'
                        + '],"page":0,"hasMore":false}'
        })

        compare(screen.readState, "ok", "the fixture's own precondition")
        compare(screen.visibleRows.length, 3, "all three rows must be rendered")

        var rules = spec.visibleNamed(screen, "rowSeparator")
        compare(rules.length, 3,
                "each of the 3 rows must carry a boundary, the last included; "
                + "found " + rules.length)
        screen.destroy()
    }

    // One screen, created, measured and destroyed before returning, so no two
    // component trees are ever live at once. The earlier single-function form
    // of the two tests below read a count from one screen, called `destroy()`
    // on it — which only QUEUES the deletion onto the event loop — and then
    // built a second screen through `makeList`, which reassigns `Core.bridge`.
    // The two trees briefly coexisted, and correctness rested on `Repeater`
    // populating synchronously and on the walk beating the queued deletion:
    // timing assumptions the test neither stated nor pinned. Measuring inside
    // a helper that owns the whole lifetime removes the overlap rather than
    // documenting it.
    //
    // The count is captured while the object is live and only the NUMBER
    // outlives it, so nothing here walks a destroyed tree.
    function separatorsForRowCount(replies, expectedRows) {
        var screen = makeList(replies)
        compare(screen.readState, "ok", "the fixture's own precondition")
        compare(screen.visibleRows.length, expectedRows,
                "the fixture must render the row count it claims")
        var count = spec.visibleNamed(screen, "rowSeparator").length
        screen.destroy()
        return count
    }

    // The two halves of what was one function. Neither alone is the relation:
    // the one-row case is what a separator dropped on the last row fails (1
    // expected, 0 found), and the four-row case is what a separator hoisted out
    // of the delegate fails — a hoisted rule renders a FIXED number however many
    // rows there are, so it can satisfy any single fixture whose row count it
    // happens to equal and cannot satisfy two that disagree. Keeping the counts
    // different (1 and 4, neither of them 3) is what makes the pair a relation
    // rather than two independently tunable constants; the arithmetic is
    // asserted in the second, against the first re-measured from its own screen.
    function test_one_row_draws_exactly_one_boundary() {
        compare(separatorsForRowCount({
            "list_stoas": '{"items":[{"stoa":"' + "aa".repeat(32) + '","foundingTitle":"One"}],'
                        + '"page":0,"hasMore":false}'
        }, 1), 1, "one row must draw exactly one boundary")
    }

    function test_the_row_count_and_the_separator_count_move_together() {
        var afterOne = separatorsForRowCount({
            "list_stoas": '{"items":[{"stoa":"' + "aa".repeat(32) + '","foundingTitle":"One"}],'
                        + '"page":0,"hasMore":false}'
        }, 1)
        var afterFour = separatorsForRowCount({
            "list_stoas": '{"items":['
                        + '{"stoa":"' + "a1".repeat(32) + '","foundingTitle":"One"},'
                        + '{"stoa":"' + "a2".repeat(32) + '","foundingTitle":"Two"},'
                        + '{"stoa":"' + "a3".repeat(32) + '","foundingTitle":"Three"},'
                        + '{"stoa":"' + "a4".repeat(32) + '","foundingTitle":"Four"}'
                        + '],"page":0,"hasMore":false}'
        }, 4)

        compare(afterFour, 4, "four rows must draw exactly four boundaries")
        compare(afterFour - afterOne, 3,
                "three further rows must add three further boundaries, got "
                + (afterFour - afterOne))
    }

    function test_an_empty_list_draws_no_row_boundary() {
        // The other direction, and not a formality: a separator hoisted to the
        // list container rather than into the delegate renders on the empty
        // state too, where it is a rule belonging to a row that is not there.
        // Both count assertions above are blind to that — neither instantiates
        // an empty list.
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}'
        })

        compare(screen.readState, "ok",
                "an empty membership is the ok state, not a failure")
        compare(screen.visibleRows.length, 0, "the fixture's own precondition")
        compare(spec.visibleNamed(screen, "rowSeparator").length, 0,
                "an empty list must draw no row boundary")
        screen.destroy()
    }

    // NO SPEC: the type a row title is set at is NOT contracted anywhere.
    // `stoa-navigation-view`'s Purpose puts the visual system — "colours, type,
    // metrics, the mark" — outside that capability by name, so the delta this
    // change adds deliberately covers the row boundary and stops there. The
    // design reference sets a row title at 19px against body's 15px, and this
    // pins the RELATION that establishes — a row title outweighs the prose
    // around it without reaching the screen's own heading — rather than the
    // literal 19, which a reword of the reference may legitimately change and
    // which no requirement makes binding.
    function test_a_row_title_is_set_apart_from_body_prose_without_reaching_a_heading() {
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + "7f3a91c4" + "00".repeat(28)
                        + '","foundingTitle":"Nym Research"}],"page":0,"hasMore":false}'
        })

        // Found by its rendered content, so a title element that vanished or was
        // renamed fails here rather than trivially satisfying the assertion.
        var titles = spec.titleElementsFor(screen, "Nym Research")
        compare(titles.length, 1, "the title element must be found by its content")

        // Against the TOKENS, not against a hardcoded 19. The claim under test
        // is the ordering between three roles in the type scale; a suite that
        // pinned the number would fail on a reference revision that kept the
        // ordering, and would pass on a DTheme where body had been raised to 19
        // and the distinction collapsed.
        compare(titles[0].font.pixelSize, DTheme.rowTitle.pixelSize,
                "the row title must be set at the rowTitle token")
        verify(DTheme.rowTitle.pixelSize > DTheme.body.pixelSize,
               "a row title must outweigh the prose around it: rowTitle "
               + DTheme.rowTitle.pixelSize + " vs body " + DTheme.body.pixelSize)
        verify(DTheme.rowTitle.pixelSize < DTheme.heading.pixelSize,
               "a row title must not compete with the screen's own heading: "
               + "rowTitle " + DTheme.rowTitle.pixelSize + " vs heading "
               + DTheme.heading.pixelSize)
        screen.destroy()
    }

    // **Narrowed by the `moderation-screen` change's amendment**, and what it
    // still forbids is the whole of what had a permanent reason. The owner
    // amended `stoa-navigation-view` to admit a MARKED PLACEHOLDER in the count
    // position; every assertion below survives that amendment unchanged, because
    // a placeholder rendering no number satisfies all of them. If a later change
    // renders a numeral there, this test and
    // `test_no_digit_is_rendered_that_the_reply_did_not_supply` both fail — which
    // is the intended outcome, since the amendment permits a placeholder rather
    // than a number.
    function test_no_row_renders_a_count_of_held_posts_or_anything_global() {
        // A thread listing is answered too, so a screen tempted to render some
        // other call's page length in the row's margin has one available.
        var addr = "cc".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Transport Notes"}],'
                        + '"page":0,"hasMore":false}',
            "list_threads": '{"items":[{"thread":"t1"},{"thread":"t2"},{"thread":"t3"}],'
                          + '"page":0,"hasMore":true}'
        })

        var shown = visibleText(screen)
        verify(shown.indexOf("posts received here") < 0,
               "no per-row held-post count is available and none may be substituted")
        verify(shown.indexOf("nothing received yet") < 0,
               "that phrase is a claim about a count nothing computed")
        // Word-boundary, not a bare substring. `indexOf("members")` also matches
        // `membership`, and **"Your membership was read without error" is
        // deliberate honest copy** in the list's empty state
        // (DStoaListScreen.qml) — the sentence that stops a user re-joining Stoas
        // they are already in. The bare form passes here only because this
        // fixture puts a row on screen, which hides that panel; add a row to the
        // empty-state fixture, or move that sentence into a row, and the test
        // fails for a reason that is not a defect.
        //
        // The two words mean opposite things: `members` is a global count no
        // peer can observe and the spec forbids permanently; `membership` is
        // what this machine recorded and is the honest thing to say.
        verify(!/\bmembers\b/.test(shown) && shown.indexOf("peers reachable") < 0,
               "no global count may appear: " + shown)
        // The specific substitution the spec forbids by name: another call's
        // page length rendered as though it were a total.
        verify(shown.indexOf("3 posts") < 0 && shown.indexOf("3 POSTS") < 0,
               "a page length from another call must not appear in a row")
        screen.destroy()
    }

    // ---- the record comes from the reply ----------------------------------
    //
    // The owner's defect. Every fixture below hands the screen a reply and then
    // asserts on what it holds — none of them writes `genesisByStoa` directly,
    // which is what the older tests in this file do and is exactly why the defect
    // survived them: a test that seeds the map is a test that assumes the very
    // thing that was missing.

    function test_a_listed_stoa_is_openable_from_the_reply_alone() {
        // The click that failed. "Open" passes `genesisFor(stoa)`, and when the
        // listing carried no record that was "" — zero bytes, which the core
        // refuses as "genesis record ended mid-field". Nothing is seeded here.
        var addr = "aa".repeat(32)
        var genesis = "01" + "cc".repeat(32) + "00" + "0000000A" + "74657374"
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Held",'
                        + '"genesis":"' + genesis + '"}],"page":0,"hasMore":false}'
        })

        compare(screen.genesisFor(addr), genesis,
                "the record the listing reported must be what Open hands over")
        verify(screen.genesisFor(addr) !== "",
               "an empty record is the input that produces 'ended mid-field'")
        screen.destroy()
    }

    // In the key-held state and driven through the create BUTTON, because that
    // is the only place a user can create from: in the could-not-be-read state
    // the fake's unanswered `get_master_key` would otherwise put this in, the
    // affordance is not in the element tree. The button lookup is what makes
    // the fixture's key state load-bearing — drop the `get_master_key` reply
    // and this fails at the lookup rather than passing on a bare `create()`.
    function test_a_created_stoa_is_openable_from_the_creation_reply_alone() {
        var addr = "dd".repeat(32)
        var genesis = "01" + "ee".repeat(32) + "00" + "00000004" + "74657374"
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false),
            "create_stoa": '{"stoa":"' + addr + '","foundingTitle":"New",'
                         + '"policy":"open","genesis":"' + genesis + '"}'
        })
        var create = spec.visibleNamed(screen, "createStoaButton")
        compare(create.length, 1,
                "creation is reachable only with a key held, so that is where "
                + "this is driven from")
        create[0].clicked()

        compare(screen.createState, "created")
        compare(screen.genesisFor(addr), genesis,
                "a Stoa just created must be openable without waiting for a reload")
        compare(screen.canShare(addr), true,
                "and shareable — the same absence hid both affordances")
        screen.destroy()
    }

    function test_a_record_survives_paging_away_from_the_stoa_that_carried_it() {
        // Additive rather than replacing. A peer with more Stoas than fit a page
        // would otherwise lose the record for everything not on the current page,
        // so Open would work on page 0 and fail on page 1.
        var first = "11".repeat(32)
        var second = "22".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + first + '","foundingTitle":"One",'
                        + '"genesis":"aabb"}],"page":0,"hasMore":true}'
        })
        compare(screen.genesisFor(first), "aabb")

        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[{"stoa":"' + second + '","foundingTitle":"Two",'
                        + '"genesis":"ccdd"}],"page":1,"hasMore":false}'
        })
        screen.page = 1
        screen.reload()

        compare(screen.genesisFor(second), "ccdd", "the new page's record is recorded")
        compare(screen.genesisFor(first), "aabb",
                "and the previous page's record is NOT dropped")
        screen.destroy()
    }

    function test_an_item_short_of_its_record_is_not_recorded_as_an_empty_one() {
        // The guard that keeps the defect from coming back wearing a success. An
        // "" written into the map would make `canShare` true and would send ""
        // straight to `read_feed` — the original failure, now with a visible
        // share button promising a reference that cannot be built.
        var missing = "33".repeat(32)
        var empty = "44".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":['
                        + '{"stoa":"' + missing + '","foundingTitle":"No field"},'
                        + '{"stoa":"' + empty + '","foundingTitle":"Empty","genesis":""}'
                        + '],"page":0,"hasMore":false}'
        })

        // **Asserted against the map's own keys, not against `genesisFor`.**
        // `genesisFor` returns "" both when the key is absent and when it holds
        // an explicit "", so every assertion phrased through it passes whichever
        // branch `rememberGenesis` takes — measured: removing the `genesis === ""`
        // half of the guard left 81/81 green. `hasOwnProperty` is the only
        // accessor that tells "never written" from "written as empty", which is
        // the distinction the guard exists to make.
        verify(!screen.genesisByStoa.hasOwnProperty(missing),
               "an absent field must leave NO key behind, not a key holding ''")
        verify(!screen.genesisByStoa.hasOwnProperty(empty),
               "an empty field must leave no key either — writing '' in is the "
               + "regression this guard prevents, and it is invisible to genesisFor")

        compare(screen.genesisFor(missing), "", "an absent field records nothing")
        compare(screen.genesisFor(empty), "", "and an empty one is not a record either")
        compare(screen.canShare(missing), false,
                "no share may be offered for a reference that cannot be built")
        compare(screen.canShare(empty), false)
        // The listing itself still succeeded: "which Stoas am I in" was answered.
        compare(screen.readState, "ok",
                "an item short of its record is not a failed read")
        screen.destroy()
    }

    // ---- sharing ---------------------------------------------------------

    function test_a_share_is_offered_only_where_the_view_holds_the_record() {
        var withRecord = "aa".repeat(32)
        var without = "bb".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":['
                        + '{"stoa":"' + withRecord + '","foundingTitle":"Held"},'
                        + '{"stoa":"' + without + '","foundingTitle":"Not held"}'
                        + '],"page":0,"hasMore":false}'
        })
        // One of the two rows has a record; the other does not. A single fixture
        // for both halves is deliberate: **an absence assertion alone proves
        // nothing**, because a renamed or deleted button makes it pass. Here the
        // POSITIVE half uses the same lookup, so a rename breaks this test rather
        // than silently satisfying it.
        var held = {}
        held[withRecord] = "00ff00ff"
        screen.genesisByStoa = held
        screen.reload()

        compare(screen.canShare(withRecord), true,
                "a Stoa whose record the view holds can be shared")
        compare(screen.canShare(without), false,
                "a Stoa whose record the view has NOT got must offer no share")

        // And nothing is produced for it, not merely nothing offered.
        compare(DStoaReference.shareText(without, screen.genesisFor(without)), "",
                "nothing carrying the address without a record may be produced")
        screen.destroy()
    }

    function test_a_share_carries_both_halves_and_the_address_in_full() {
        var addr = "b02d5e77a41c6b9013c6a9408ff4af235d7e1b06c92a84f13be057dc6104a8bf"
        var genesis = "0102030405060708"
        var text = DStoaReference.shareText(addr, genesis)

        verify(text.indexOf(addr) >= 0,
               "the whole address must be present, unabbreviated: " + text)
        verify(text.indexOf(genesis) >= 0, "the genesis record must be present")
        // Unabbreviated, stated as the relation rather than as a length: an
        // ellipsis anywhere means an abbreviation reached the shareable thing.
        verify(text.indexOf("…") < 0, "no elision may appear in a shareable reference")
    }

    function test_what_a_share_produces_is_what_a_paste_accepts() {
        // The round trip, which is the whole reason the two live in one file. A
        // share whose output the paste field cannot read produces a string whose
        // recipient can do nothing with it.
        var addr = "b02d5e77" + "44".repeat(28)
        var genesis = "deadbeefcafe"
        var parsed = DStoaReference.parse(DStoaReference.shareText(addr, genesis))

        compare(parsed.ok, true, "a share's own output must parse: ")
        compare(parsed.stoa, addr, "the preview must name the same Stoa")
        compare(parsed.genesis, genesis)
    }

    function test_a_share_string_reaches_the_clipboard_sink_verbatim() {
        var sink = sinkComponent.createObject(null, {})
        var addr = "aa".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Held"}],'
                        + '"page":0,"hasMore":false}'
        }, { clipboard: sink })
        var held = {}
        held[addr] = "00ff"
        screen.genesisByStoa = held
        screen.reload()

        var buttons = visibleNamed(screen, "shareButton")
        compare(buttons.length, 1, "exactly one share affordance for the one held record")
        buttons[0].clicked()

        // **What is asserted is what the sink was ASKED to copy.** Under
        // QT_QPA_PLATFORM=offscreen there is no system clipboard, so no test
        // anywhere can check what actually landed on it — TextEdit.copy() writes
        // nowhere readable. That half is unverified, here and in CI, and is
        // stated rather than papered over.
        compare(sink.lastCopied, DStoaReference.shareText(addr, "00ff"))
        verify(sink.lastCopied.indexOf(addr) >= 0)
        verify(sink.lastCopied.indexOf("00ff") >= 0)
        screen.destroy()
        sink.destroy()
    }

    // ---- pasting ---------------------------------------------------------

    function test_text_that_is_not_a_stoa_reference_is_refused_before_any_call() {
        var screen = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var bridge = Core.bridge
        var before = bridge.calls.length

        screen.pasted = "stoa:b02d5e77a41c6b9013c6a9408ff4af23"
        screen.preview()

        verify(screen.pasteFailure.length > 0, "the screen must say what was pasted is not a reference")
        compare(spec.callsTo(bridge, "join_stoa"), 0,
                "a malformed paste must reach no join call")
        compare(bridge.calls.length, before,
                "a malformed paste must reach NO call at all")
        screen.destroy()
    }

    function test_an_address_with_no_record_is_named_as_the_missing_half() {
        var parsed = DStoaReference.parse('{"stoa":"' + "aa".repeat(32) + '"}')
        compare(parsed.ok, false)
        verify(parsed.reason.toLowerCase().indexOf("record") >= 0,
               "the refusal must name the half that is missing: " + parsed.reason)
    }

    function test_a_well_formed_reference_previews_and_joins_nothing() {
        var screen = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var bridge = Core.bridge
        var seen = { stoa: "", genesis: "", count: 0 }
        screen.previewRequested.connect(function (stoa, genesis) {
            seen.stoa = stoa
            seen.genesis = genesis
            seen.count++
        })

        var addr = "b02d5e77" + "55".repeat(28)
        screen.pasted = DStoaReference.shareText(addr, "00ff")
        screen.preview()

        compare(seen.count, 1, "the preview must be requested")
        compare(seen.stoa, addr)
        compare(screen.pasteFailure, "", "a well-formed reference is not a paste failure")
        compare(spec.callsTo(bridge, "join_stoa"), 0,
                "acting on an address must PREVIEW and never join")
        screen.destroy()
    }

    function test_a_display_prefix_is_stripped_before_anything_is_sent() {
        // `stoa:` is a reading aid the design bundle uses. The core writes and
        // accepts bare hex, so a prefix that survived into join_stoa would be a
        // hash verifying against nothing — surfacing as a VERIFICATION failure
        // rather than as the malformed paste it actually is, which is the one
        // confusion the three outcomes exist to prevent.
        var addr = "b02d5e77" + "66".repeat(28)
        var parsed = DStoaReference.parse('{"stoa":"stoa:' + addr + '","genesis":"00ff"}')
        compare(parsed.ok, true)
        compare(parsed.stoa, addr, "the prefix must not reach the core")
        verify(DStoaReference.shareText(addr, "00ff").indexOf("stoa:") < 0,
               "and must never be added on the way out")
    }

    function test_a_repeated_display_prefix_is_stripped_rather_than_forwarded() {
        // **A single-pass strip is the defect, and the test above could not see
        // it** because it only ever supplied one prefix. A doubled `stoa:`
        // survived, reached `join_stoa` as part of the address, and came back as
        // "the genesis record does not hash to this address" — turning a
        // malformed paste into a VERIFICATION ACCUSATION and pointing the user
        // at their sender instead of at their own paste.
        //
        // That is the exact confusion the three paste outcomes are designed to
        // keep apart, and both design.md and DStoaReference's own source comment
        // assert it cannot happen. The invariant the comment claimed was not the
        // one the code enforced.
        var addr = "b02d5e77" + "88".repeat(28)
        var parsed = DStoaReference.parse('{"stoa":"stoa:stoa:' + addr + '","genesis":"00ff"}')
        compare(parsed.ok, true)
        compare(parsed.stoa, addr, "every prefix must be stripped, not just one")

        // Whitespace between them too — a paste that picked up a stray space is
        // the same user error with the same right answer.
        compare(DStoaReference.parse('{"stoa":"stoa: stoa:' + addr + '","genesis":"00ff"}').stoa,
                addr)
        compare(DStoaReference.parse('{"stoa":"  stoa:stoa:stoa:' + addr + '  ","genesis":"00ff"}').stoa,
                addr, "three, with surrounding whitespace")

        // And the end-to-end consequence: nothing carrying a prefix reaches the
        // core, so no verification refusal can be provoked by a paste artefact.
        Core.bridge = bridgeFor({ "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"t"}' })
        var bridge = Core.bridge
        Core.joinStoa(parsed.stoa, parsed.genesis)
        var sent = String(spec.lastArgsTo(bridge, "join_stoa"))
        verify(sent.indexOf("stoa:") < 0,
               "no prefix may reach join_stoa: " + sent)
    }

    // ---- the join preview ------------------------------------------------

    function test_the_preview_makes_no_join_call_until_the_user_acts() {
        var addr = "b02d5e77" + "77".repeat(28)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff", foundingTitle: "Nym Research" })
        var bridge = Core.bridge

        // Rendered, and nothing JOINED. This is the assertion the recording
        // bridge exists for: a screen that previews and a screen that joins
        // silently are indistinguishable from the outside.
        //
        // It used to assert NO call at all. The preview now asks `getStoa`
        // what the Stoa is called, which joins nothing, so the assertion is
        // narrowed to the calls a preview may make rather than dropped.
        compare(screen.joinState, "previewing")
        compare(spec.callsTo(bridge, "join_stoa"), 0,
                "rendering a preview must make no join call")
        compare(bridge.calls.length, spec.callsTo(bridge, "get_stoa"),
                "and the only call it makes is the lookup")

        screen.join()

        compare(spec.callsTo(bridge, "join_stoa"), 1,
                "the join call is made when, and only when, the user acts")
        compare(screen.joinState, "joined")
        screen.destroy()
    }

    // ---- a second preview inherits nothing from the first -----------------
    //
    // **These drive `Main.qml`, not a fresh `DJoinScreen`, and that is the whole
    // point of them.** Every other join-state test in this file constructs its
    // own screen — and `Main.qml` ships ONE reused instance, which is the only
    // configuration a user ever meets. So the suite could be entirely green
    // while a hostile reference rendered under a "Joined." panel it never
    // earned, and it was — every test in every spec file passed with that
    // defect present.
    //
    // That is the corpus lesson at integration scale. A test exercising a
    // component in a shape the app does not use is testing something the app
    // does not do.

    function test_a_second_preview_does_not_inherit_the_first_joined_state() {
        var a = "aaaaaaaa" + "11".repeat(28)
        var b = "bbbbbbbb" + "22".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "join_stoa": '{"stoa":"' + a + '","foundingTitle":"Nym Research","policy":"open"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        // Preview A and join it, the legitimate half of the sequence.
        view.previewing = { stoa: a, genesis: "00ff" }
        join.join()
        compare(join.joinState, "joined", "A was genuinely joined")
        compare(spec.callsTo(bridge, "join_stoa"), 1)

        // Now the attacker's reference, with NO Cancel in between — an ordinary
        // two-paste sequence, not a contrived one.
        view.previewing = { stoa: b, genesis: "00ff" }

        compare(join.stoaAddress, b, "the attacker's address is what is on screen")
        compare(spec.callsTo(bridge, "join_stoa"), 1,
                "no join call was made for B")
        compare(join.joinState, "previewing",
                "so B must NOT read as joined — a join is reported from the "
                + "core's reply, never inherited from another Stoa's")

        // The user-visible consequence, not only the state string feeding it.
        compare(spec.visibleNamed(join, "joinedPanel").length, 0,
                "no joined panel for a Stoa the core never saw")
        compare(spec.visibleNamed(join, "joinButton").length, 1,
                "and the join affordance must be back, or the user cannot act")
        var body = spec.bodyText(join)
        verify(body.indexOf("started collecting") < 0,
               "nothing may claim this machine began collecting B's records: " + body)
        view.destroy()
    }

    function test_a_second_preview_does_not_inherit_the_first_founding_title() {
        var a = "aaaaaaaa" + "33".repeat(28)
        var b = "bbbbbbbb" + "44".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "join_stoa": '{"stoa":"' + a + '","foundingTitle":"Nym Research","policy":"open"}'
        })
        var view = mainComponent.createObject(null, {})
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        view.previewing = { stoa: a, genesis: "00ff" }
        join.join()
        compare(join.foundingTitle, "Nym Research", "A's title came from A's reply")

        view.previewing = { stoa: b, genesis: "00ff" }

        // **Separable from the state carryover, and it must be asserted
        // separately.** Clearing `joinState` alone would still leave A's title
        // captioned "FIXED FOREVER" above B's address — a trusted name lent to
        // an untrusted address, which is precisely the impersonation the
        // lookalike requirement exists to expose, delivered by the view itself.
        // The lookalike panel cannot catch it: the two titles being compared
        // are the same string from the same source.
        compare(join.foundingTitle, "",
                "B's title is unknown until B's record is decoded, so nothing "
                + "may be rendered as B's founding title")
        var body = spec.bodyText(join)
        verify(body.indexOf("Nym Research") < 0,
               "A's title must not decorate B's address: " + body)
        view.destroy()
    }

    function test_a_second_preview_does_not_inherit_the_first_refusal() {
        var a = "aaaaaaaa" + "55".repeat(28)
        var b = "bbbbbbbb" + "66".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "join_stoa": '{"error":"the genesis record does not hash to this address"}'
        })
        var view = mainComponent.createObject(null, {})
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        view.previewing = { stoa: a, genesis: "00ff" }
        join.join()
        compare(join.joinState, "failed", "A was refused by the core")

        view.previewing = { stoa: b, genesis: "00ff" }

        // The mirror of the joined case and just as wrong: a user reads "the
        // record you were sent is wrong" about a reference the core has not
        // seen. Listed separately by the reviewer because fixing either of the
        // two above does not fix this one.
        compare(join.joinState, "previewing")
        compare(join.failure, "", "A's refusal says nothing about B")
        compare(spec.visibleNamed(join, "joinFailurePanel").length, 0,
                "no refusal may be attached to an address never submitted")
        view.destroy()
    }

    function test_an_outcome_survives_re_previewing_the_very_same_reference() {
        // **The other side of the rule, and I got this one wrong first.**
        //
        // My initial version of this test asserted that re-previewing the SAME
        // reference after joining it must read `previewing` again — that Cancel
        // should wipe the outcome. It failed against the fix, and the fix was
        // right: the core genuinely answered for this exact (stoa, genesis)
        // pair, so reporting it is reporting a fact, not inheriting another
        // Stoa's. Making the screen forget a true answer would have been a
        // second defect dressed as caution, and would have cost a real join call
        // every time a user glanced back at a Stoa they already hold.
        //
        // The invariant is not "clear on navigation" — that is the reset shape
        // the fix deliberately avoided. It is "an outcome describes exactly the
        // reference it was returned for", which is true here and false for the
        // three tests above. Recorded so nobody later "fixes" this into a wipe.
        var a = "aaaaaaaa" + "77".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "join_stoa": '{"stoa":"' + a + '","foundingTitle":"Nym Research","policy":"open"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        view.previewing = { stoa: a, genesis: "00ff" }
        join.join()
        join.cancelled()
        compare(view.previewing, null)

        view.previewing = { stoa: a, genesis: "00ff" }
        compare(join.joinState, "joined",
                "the core answered for THIS reference, so the answer stands")
        compare(join.foundingTitle, "Nym Research",
                "and the title it returned belongs to this Stoa")
        compare(spec.callsTo(bridge, "join_stoa"), 1,
                "and no second call was needed to say so")

        // A DIFFERENT genesis for the same address is a different reference, and
        // must not inherit: the address alone does not identify what was
        // verified, since the pair is what the core checked.
        view.previewing = { stoa: a, genesis: "ffff" }
        compare(join.joinState, "previewing",
                "a different record against the same address is a different "
                + "reference and carries no outcome")
        compare(join.foundingTitle, "")
        view.destroy()
    }

    function test_the_preview_renders_the_whole_address_not_an_abbreviation() {
        var addr = "b02d5e77a41c6b9013c6a9408ff4af235d7e1b06c92a84f13be057dc6104a8bf"
        var screen = makeJoin({}, { stoaAddress: addr, stoaGenesis: "00ff" })

        var labels = spec.visibleNamed(screen, "previewAddress")
        compare(labels.length, 1)
        compare(labels[0].full, true, "a decision is made here; the full form is required")
        compare(labels[0].text, addr,
                "the whole address must be rendered rather than an abbreviation")
        verify(labels[0].text.indexOf("…") < 0, "no elision on this screen")
        screen.destroy()
    }

    // **This drives a state the shipped screen reaches only AFTER a join**, and
    // the route it takes there cannot happen at all.
    //
    // `foundingTitle` is `readonly` and derived from `currentOutcome`; QML
    // accepts a `createObject` props value as a readonly property's INITIAL
    // value, so this yields a title with a `null` outcome — a pair `Main.qml`
    // never produces. The assertion below is still worth making: WHEN a title
    // exists it must be labelled as founding, and that is the requirement. But
    // it says nothing about whether a title ever exists on a preview, and a
    // design review found it does not — see
    // `test_a_preview_before_a_join_carries_no_founding_title_and_says_so`,
    // which goes through the real paste route, and design.md constraint 4.
    function test_the_founding_title_is_labelled_as_founding() {
        var screen = makeJoin({}, {
            stoaAddress: "aa".repeat(32),
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research"
        })

        var shown = spec.visibleText(screen)
        verify(shown.indexOf("Nym Research") >= 0, "the founding title must be rendered")
        verify(shown.indexOf("FOUNDING TITLE") >= 0,
               "and identified as the FOUNDING value: " + shown)

        // Same exposure as a list row: this title is peer-supplied and freely
        // chosen. Asserted on the FORMAT rather than on the text, for the reason
        // spelled out on the list's markup test — `text` cannot see the
        // rendering, which is the property in question.
        var titles = spec.visibleNamed(screen, "foundingTitleText")
        compare(titles.length, 1)
        compare(titles[0].textFormat, Text.PlainText,
                "the founding title must never reach a markup-interpreting path")
        screen.destroy()
    }

    function test_no_current_title_is_rendered_while_nothing_resolves_one() {
        var screen = makeJoin({}, {
            stoaAddress: "aa".repeat(32),
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research"
        })

        var shown = spec.visibleText(screen)
        verify(shown.indexOf("CURRENT TITLE") < 0,
               "the founding value must not be captioned as a current title: " + shown)
        verify(shown.toLowerCase().indexOf("chosen by a moderator") < 0,
               "nothing may attribute this title to a moderator")
        // Rendered ONCE. Twice under two captions would be the same assertion
        // made in the layout instead of the copy.
        compare(spec.visibleNamed(screen, "currentTitleText").length, 0)
        compare(spec.visibleNamed(screen, "foundingTitleText").length, 1)

        // **The two literals above are not the requirement**, and a reviewer
        // proved it: extending the founding caption to `FOUNDING TITLE — FIXED
        // FOREVER — AND THIS STOA'S PRESENT NAME, AS ITS MODERATOR HAS IT TODAY`
        // carries neither blocked string and asserts both things R6 forbids. All
        // 100 tests passed. Same family as the address note — the assertion read
        // the right value and asked the wrong question.
        //
        // So: the *claim* is refused, however phrased. Two conjunctions, because
        // the requirement forbids two distinct things and either alone is the
        // harm. On a build where metadata resolution is not implemented, copy
        // saying this title is what the Stoa is called NOW asserts that no
        // moderator has renamed it — a fact no peer here has checked and which
        // is false for every Stoa that has been renamed.
        var low = shown.toLowerCase()

        // (a) a present-tense naming word attached to a title.
        verify(!/\b(present|current|now|today|at the moment|as it stands|presently)\b[^.]{0,80}\b(name|title|called|known as)\b/.test(low)
               && !/\b(name|title|called|known as)\b[^.]{0,80}\b(present|current|now|today|at the moment|as it stands|presently)\b/.test(low),
               "nothing may present a title as what the Stoa is called now — "
               + "nothing on this build resolves a current title, so such a "
               + "caption asserts no moderator has renamed it: " + shown)

        // (b) a title attributed to a moderator, in any voice.
        verify(!/\bmoderator\b[^.]{0,80}\b(name|title|called|renamed|has it|chose|chosen)\b/.test(low)
               && !/\b(name|title|called|renamed)\b[^.]{0,80}\bmoderator\b/.test(low),
               "nothing may attribute this title to a moderator: " + shown)
        screen.destroy()
    }

    function test_a_resolved_current_title_fills_that_position_when_one_exists() {
        // The panel arrives WITH the value. This is the forward half of the rule
        // above: the prohibition is on claiming, not on the layout.
        //
        // Driven through the two replies that produce this state, rather than
        // through `createObject` props: a non-fallback lookup supplies the
        // current title, and only a join supplies the founding one beside it.
        var addr = "aa".repeat(32)
        var screen = makeJoin({
            "get_stoa": spec.getStoaReply(addr, false, "Nym Research Archive", ""),
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff" })
        screen.join()

        var shown = spec.visibleText(screen)
        verify(shown.indexOf("Nym Research Archive") >= 0, "the resolved title must render")
        verify(shown.indexOf("FOUNDING TITLE") >= 0, "and the founding one stay labelled")
        verify(shown.indexOf("CURRENT TITLE") >= 0,
               "each labelled as the value it is, so the reader can tell which was fixed")
        screen.destroy()
    }

    function test_the_explanation_claims_a_hash_match_and_names_the_unverified_rest() {
        var screen = makeJoin({}, { stoaAddress: "aa".repeat(32), stoaGenesis: "00ff" })
        var notes = spec.visibleNamed(screen, "addressNote")
        compare(notes.length, 1)
        var note = notes[0].text

        verify(note.indexOf("this address names") >= 0,
               "what is stated is that the record shown is the one the address names")
        verify(note.toLowerCase().indexOf("unverified") >= 0
               || note.toLowerCase().indexOf("not checked") >= 0,
               "the unverified remainder must be named: " + note)
        // And the overreach the bundle's own string carries is absent: nothing
        // says pasting IS the verification, full stop.
        verify(note.indexOf("is itself the verification") < 0,
               "the copy must not present pasting as THE verification")
        verify(note.toLowerCase().indexOf("registry") >= 0,
               "and must say the address was checked against no registry")
        screen.destroy()
    }

    function test_nothing_on_the_preview_promises_a_per_stoa_identity() {
        var addr = "aa".repeat(32)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff" })

        // **Driven through the JOINED state, and that is the point of the test
        // rather than incidental setup.** An earlier version asserted over a
        // fresh preview, whose card body says nothing whatever about what
        // joining does — the only sentence on that subject lived in the
        // apparatus column, which is annotation and is being removed. So the
        // assertion was scanning a screen that could not plausibly have carried
        // the claim: an absence proved over a corpus with no candidate in it,
        // which is the same "passes for the wrong reason" shape as asserting a
        // binding's input instead of its output.
        //
        // After a join the body DOES speak about what joining did, so this now
        // examines the text where the false promise would actually appear.
        screen.join()
        compare(screen.joinState, "joined", "the state that says what joining did")

        var body = spec.bodyText(screen)
        verify(body.indexOf("started collecting") >= 0,
               "the body must be saying what joining did, or this proves nothing "
               + "about the screen a user reads: " + body)

        var shown = body.toLowerCase()

        // The bundle's apparatus note ends "and generates you an identity for it
        // alone". One key signs in every Stoa in this release, so that sentence
        // promises an unlinkability property the software does not have — the
        // one false claim on these screens that could actually harm somebody.
        verify(shown.indexOf("identity for it alone") < 0,
               "no per-Stoa identity may be promised")
        verify(shown.indexOf("an identity for that stoa") < 0)
        verify(shown.indexOf("identity for that stoa alone") < 0)
        // The claim need not use the bundle's exact words to do the harm, so the
        // word itself is refused anywhere in the body. Nothing this screen
        // legitimately says uses it.
        // Word-boundary, for the same reason as `\bmembers\b` on the list's count
        // assertion. `identical` does not in fact contain `identity` — this one
        // is safe by luck of spelling today — but the words it WOULD catch are
        // `identities`, `identify` and `identifier`, and the lookalike panel is
        // one copy edit from saying "the titles are identical" on screen. A
        // false alarm naming a per-Stoa-identity promise that was never made is
        // how a real one later gets waved through.
        verify(!/\bidentity\b/.test(shown),
               "the body must not raise identity at all, however phrased: " + body)
        verify(shown.indexOf("you moderate") < 0, "no moderator status may be asserted")
        verify(shown.indexOf("membership list") < 0 || shown.indexOf("no membership list") >= 0,
               "membership may be denied but never asserted")
        screen.destroy()
    }

    // The corpus these absence assertions run over, asserted rather than assumed.
    //
    // **An absence assertion cannot tell you its corpus shrank.** The apparatus
    // column is annotation that a separate piece is removing from the shipped
    // view, and more than half the text `visibleText` returns for a join screen
    // comes from it. Without this, that removal would silently narrow two tests
    // above while their names went on claiming the same coverage — so the
    // relationship is pinned here, and it fails loudly whichever way it breaks.
    function test_the_absence_assertions_scan_the_body_and_not_only_the_apparatus() {
        var addr = "aa".repeat(32)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff", foundingTitle: "Nym Research" })
        screen.join()

        var body = spec.bodyText(screen)
        var app = spec.apparatusText(screen)

        // The body carries real, on-topic sentences of its own. If the apparatus
        // were ever the only place a subject was discussed, an absence assertion
        // over the body would be vacuous — which is exactly the defect this
        // pins.
        verify(body.length > 0, "the body must carry text of its own")
        verify(body.indexOf("started collecting") >= 0,
               "what joining did is stated in the BODY, not only in a margin note")
        verify(body.indexOf("Nym Research") >= 0, "as is the founding title")

        // **The apparatus column is GONE, and this assertion now pins that
        // rather than the subtraction it was written for.** `piece/drop-apparatus`
        // deleted both components and `ScreenFrame`'s `apparatus` alias, so
        // `screen.apparatus` is `undefined`, `apparatusText` returns "" for every
        // screen, and `bodyText` is the identity function.
        //
        // That matters more than it sounds. The subtraction below was the guard
        // that kept these absence assertions honest while half the scanned text
        // was annotation — and a walker narrowed until it returns nothing passes
        // every assertion written over it, which is this repo's recorded defect
        // family. So the branch is asserted CLOSED rather than left as a
        // conditional nobody notices is dead: if an apparatus-shaped property
        // ever comes back, this fails and whoever brought it back has to decide
        // what the body-versus-annotation split means again.
        compare(app, "",
                "no screen has an apparatus column any more, so bodyText is the "
                + "whole screen. If this fails, annotation has returned to the "
                + "shipped view and every absence assertion in this file is "
                + "scanning a corpus that is partly margin note again.")
        compare(body, spec.visibleText(screen),
                "and with nothing to subtract, the body IS the screen")
        screen.destroy()
    }

    // ---- the two failures, kept apart -------------------------------------
    //
    // **`test_a_record_that_does_not_verify_is_a_failure_distinct_from_a_bad_paste`
    // used to live here and has been REMOVED, not merely superseded.** It
    // asserted that the two refusals differ — which this file's header records
    // as insufficient, because misinforming strings are still distinct strings,
    // and a tester demonstrated exactly that on a sibling piece.
    //
    // Deleted rather than labelled. A label is a weaker guard than absence: the
    // repo's own memory records that a known-weak test is the template the next
    // one gets written against, and a reader arriving here would have met the
    // rule the file elsewhere says is not enough, 600 lines before meeting the
    // replacement.
    //
    // Nothing was lost with it, which is why removal was available:
    // `test_a_malformed_paste_and_an_unverified_record_say_different_things_to_do`
    // below is built on the same fixture and the same core error string, renders
    // the refusal through `joinFailureText` rather than through a whole-screen
    // scan, keeps the non-containment check, and adds what the old one could not
    // ask — what each refusal must and must not IMPLY. The `joinState === "failed"`
    // assertion it also carried is pinned directly below, in
    // `test_the_view_does_not_report_a_pair_as_joinable_before_the_core_answers`.

    function test_the_view_does_not_report_a_pair_as_joinable_before_the_core_answers() {
        // A well-formed pair whose record does not name the address. The view's
        // own check is limited to whether both halves are present — a view that
        // re-derived an address would be a second implementation of the one
        // check this design rests on.
        var addr = "b02d5e77" + "99".repeat(28)
        var parsed = DStoaReference.parse(DStoaReference.shareText(addr, "00"))
        compare(parsed.ok, true,
                "the view must accept a well-formed pair without judging it")

        var screen = makeJoin({
            "join_stoa": '{"error":"the genesis record does not hash to this address"}'
        }, { stoaAddress: addr, stoaGenesis: "00" })

        compare(screen.joinState, "previewing",
                "nothing is reported as joinable before the core has answered")
        screen.join()
        compare(screen.joinState, "failed",
                "the outcome comes from the core's reply, not from a check in the view")
        screen.destroy()
    }

    function test_a_failed_join_does_not_report_the_stoa_as_joined() {
        var joinedFired = 0
        var screen = makeJoin({
            "join_stoa": '{"error":"the membership store is read-only"}'
        }, { stoaAddress: "aa".repeat(32), stoaGenesis: "00ff" })
        screen.joined.connect(function () { joinedFired++ })

        screen.join()

        compare(screen.joinState, "failed")
        compare(joinedFired, 0, "a failure must not emit the joined signal")
        verify(spec.visibleText(screen).indexOf("read-only") >= 0,
               "the core's failure must be rendered")
        compare(spec.visibleNamed(screen, "joinedPanel").length, 0,
                "and the joined panel must not be on screen")
        screen.destroy()
    }

    function test_joining_a_stoa_already_held_is_success_with_no_warning() {
        var addr = "7f3a91c4" + "aa".repeat(28)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, {
            stoaAddress: addr,
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research",
            // The very same Stoa is already in the held listing — which is the
            // input a screen inferring newness would reach for.
            heldStoas: [{ stoa: addr, foundingTitle: "Nym Research" }]
        })

        screen.join()

        compare(screen.joinState, "joined", "a repeat join is SUCCESS")

        // The BODY, not the whole frame. The apparatus column is annotation
        // being removed from the shipped view, and scanning it would let this
        // assertion quietly narrow when that lands. The corpus is asserted
        // first so the absence below is over text that exists.
        var body = spec.bodyText(screen)
        verify(body.indexOf("Joined.") >= 0,
               "the outcome must be on screen, or the absences prove nothing: " + body)

        var shown = body.toLowerCase()
        verify(shown.indexOf("already") < 0, "no warning about a repeat")
        verify(shown.indexOf("collision") < 0 && shown.indexOf("duplicate") < 0,
               "and no collision to resolve: " + shown)
        // Word-boundary, not a bare substring: a plain `indexOf("again")` fires
        // on "not checked AGAINst any registry", which is legitimate copy in the
        // address note. Caught by this assertion failing on first run, which is
        // the argument for running a new assertion before believing it.
        verify(!/\bagain\b/.test(shown), "nor any nod to this being a second attempt")
        verify(!/\bonce more\b/.test(shown))

        // The user-visible consequence, not the state string that feeds it: the
        // failure panel must not be ON SCREEN. `visibleNamed` checks every
        // ancestor's visibility, so a panel inside a hidden parent counts as
        // absent and a panel bound `visible: true` counts as present.
        compare(spec.visibleNamed(screen, "joinFailurePanel").length, 0,
                "no failure panel may be rendered for a repeat join")
        compare(spec.visibleNamed(screen, "joinedPanel").length, 1,
                "and the joined panel must actually be rendered")
        screen.destroy()
    }

    // ---- the lookalike ----------------------------------------------------

    // Same caveat as the founding-title test above, and here it is sharper: the
    // props route gives this screen a title with a `null` outcome, which is the
    // state a JOINED screen is in and not the state a preview is in. So this
    // pins that the comparison WORKS when a title exists — which it does — and
    // proves nothing about when that is. Through the real route it is only after
    // a join; `test_the_lookalike_warning_cannot_be_claimed_before_a_join_happens`
    // is the one that measures the timing.
    function test_a_same_title_stoa_already_held_is_shown_beside_the_preview() {
        var previewed = "b02d5e77" + "bb".repeat(28)
        var held = "7f3a91c4" + "cc".repeat(28)
        var screen = makeJoin({}, {
            stoaAddress: previewed,
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research",
            heldStoas: [{ stoa: held, foundingTitle: "Nym Research" }]
        })

        compare(screen.lookalikes.length, 1)
        var shown = spec.visibleText(screen)
        // Both addresses. The titles are identical, so the addresses are the
        // only thing telling the reader these are two Stoas.
        verify(shown.indexOf("b02d5e77") >= 0, "the previewed address must be on screen")
        verify(shown.indexOf("7f3a91c4") >= 0, "the held one's address too")
        verify(shown.toLowerCase().indexOf("duplicate") < 0
               && shown.toLowerCase().indexOf("conflict") < 0,
               "neither is a duplicate or a conflict: " + shown)
        screen.destroy()
    }

    // ---- the two states a real preview can actually be in ------------------
    //
    // **Every lookalike and founding-title test above reaches the screen through
    // `createObject` props, and that route can build a state the shipped screen
    // cannot.** `foundingTitle` is `readonly` and derived from `currentOutcome`;
    // QML accepts a props value as a readonly property's INITIAL value, which
    // yields a title with a `null` outcome — a pair `Main.qml` never produces.
    // Measured: after `makeJoin({}, {foundingTitle: "Nym Research"})` the screen
    // reports that title while `outcome` is `null`.
    //
    // So those tests prove something about a state that does not exist, and a
    // design review found the consequence by driving the real thing instead: the
    // lookalike panel — whose whole purpose is to warn BEFORE the user commits —
    // could not render until after they had committed, and the founding-title
    // caption rendered over nothing. The two below go through `Main.qml`'s real
    // paste route, which is the only route a user has.

    function test_a_preview_before_a_join_carries_no_founding_title_and_says_so() {
        // The constraint, pinned so nobody later reads the empty panel as a bug
        // and fills it with something. Before a join, `getStoa` answers a
        // founding title only when it falls back. Here it answers a CURRENT
        // title, so no founding title is available, and decoding the genesis
        // record in QML would be a second implementation of core's encoding.
        var attacker = "b02d5e77" + "bb".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_stoa": spec.getStoaReply(attacker, false, "Renamed Since", "")
        })
        var view = mainComponent.createObject(null, {})
        var list = spec.namedAnywhere(view, "stoaList")[0]
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        list.pasted = JSON.stringify({ stoa: attacker, genesis: "00ff" })
        list.preview()
        compare(view.screenShown, "join", "the real route reaches the preview")

        compare(join.foundingTitle, "",
                "a non-fallback reply answers no founding title for this reference")

        // **The user-visible half, which is the finding.** A caption reading
        // FOUNDING TITLE — FIXED FOREVER above an empty value tells a reader the
        // Stoa's founding title is blank. It is not; it is unknown here.
        var body = spec.bodyText(join)
        var titles = spec.visibleNamed(join, "foundingTitleText")
        verify(titles.length === 0 || titles[0].text !== "",
               "no title element may render empty under a title caption: <"
               + (titles.length > 0 ? titles[0].text : "absent") + ">")
        verify(body.indexOf("FOUNDING TITLE — FIXED FOREVER") < 0,
               "and the caption must not stand over nothing: " + body)

        // Something has to say WHY, or the absence reads as a broken screen
        // rather than as a thing this build cannot know.
        //
        // **Asserted as a conjunction of claims on ONE element's text**, and
        // both halves of that shape were forced by a failure.
        //
        // The conjunction, rather than one ordered phrase: an earlier version
        // required `title` and `join` inside one sentence, which the honest copy
        // fails the moment the writer splits the sentence in two — red on a
        // reword, green on a lie, this file's second defect family.
        //
        // The single element, rather than `bodyText`: with the note HIDDEN a
        // whole-body version of this still passed, because the apparatus
        // column's `ON THE TITLE` margin note carries "title" and "chose it
        // freely" and the address note carries "not checked". A corpus with a
        // decoy in it proves nothing — and the apparatus is annotation that a
        // separate piece is removing (design.md D7), so an assertion resting on
        // it is one that quietly stops proving anything.
        var notes = spec.visibleNamed(join, "titleUnknownText")
        compare(notes.length, 1,
                "the preview must carry a visible statement of why no title is "
                + "shown, or the empty space reads as a broken screen")
        var low = notes[0].text.toLowerCase()
        verify(/\b(founding )?title\b/.test(low),
               "the copy must name the thing that is missing as a title: " + notes[0].text)
        verify(/\b(cannot|can't|not|no)\b/.test(low),
               "and state an absence rather than describing a feature")
        verify(/\bjoin(ing|ed)?\b/.test(low),
               "and name joining as what would supply it: " + notes[0].text)
        // The misinformation this must still catch, verified by planting it: a
        // caption saying the title is blank, or that there is none. A preview
        // saying "this Stoa has no title" states a fact about the Stoa that
        // nothing here checked.
        verify(!/\b(has no|carries no|without a) (founding )?title\b/.test(low),
               "the title is unknown here, not known to be absent: " + notes[0].text)
        // And with a current title on screen, nothing may say that what the
        // Stoa is called is unknown: the note's old heading did exactly that.
        verify(spec.visibleText(join).toLowerCase().indexOf("knows what this stoa is called") < 0,
               "a current title is known here, so what it is called is not unknown")

        // And the address — the half that IS trustworthy — is still on screen,
        // so what the preview shows is the thing worth deciding on.
        verify(body.indexOf(attacker) >= 0, "the address is rendered in full")
        view.destroy()
    }

    function test_the_lookalike_warning_cannot_be_claimed_before_a_join_happens() {
        // **Where no founding title is available, the impersonation defence
        // does not run, and the screen must not imply that it did.** Before a
        // join that is every reference `getStoa` does not answer as a fallback.
        // Here it answers a CURRENT title equal to the held Stoa's founding
        // title, which is the renamed-to-match case the owner ruled out of
        // scope on #143: the comparison is over founding titles, so it still
        // does not run.
        //
        // Measured through the real paste route before a lookup existed:
        //   foundingTitle=<>  lookalikes=0  panel=0   with heldStoas=1
        //   after join():     foundingTitle=<Nym Research>  lookalikes=1  panel=1
        //
        // This test does not assert the panel appears early. It asserts the
        // screen does not silently present an unrun check as a clean result.
        var held = "7f3a91c4" + "cc".repeat(28)
        var attacker = "b02d5e77" + "bb".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[{"stoa":"' + held + '","foundingTitle":"Nym Research"}],'
                        + '"page":0,"hasMore":false}',
            "get_stoa": spec.getStoaReply(attacker, false, "Nym Research", ""),
            "join_stoa": '{"stoa":"' + attacker + '","foundingTitle":"Nym Research",'
                       + '"policy":"open"}'
        })
        var view = mainComponent.createObject(null, {})
        var list = spec.namedAnywhere(view, "stoaList")[0]
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        list.pasted = JSON.stringify({ stoa: attacker, genesis: "00ff" })
        list.preview()

        compare(join.heldStoas.length, 1,
                "the listing IS available — the comparison's other half is there")
        compare(join.currentTitle, "Nym Research",
                "the lookup's current title IS on screen, and equals the held one")
        compare(join.lookalikes.length, 0,
                "and the comparison still does not run, because a current title "
                + "is not a founding title")
        compare(spec.visibleNamed(join, "lookalikePanel").length, 0)

        // The load-bearing assertion: the preview must state that the comparison
        // has not been made. Silence here is a reader concluding from an absent
        // warning that they were warned and there was nothing to warn about.
        //
        // The comparison is named — a word for comparing, near a word for the
        // thing compared — and negated. Two separate regexes rather than one
        // ordered pattern, because either order is honest English and an ordered
        // one fails on the innocent half of the reword.
        //
        // **Asserted on ONE element's text, not on the whole body.** A
        // whole-body scan passed with the note hidden, because the apparatus
        // column's `ON THE TITLE` margin note already contains "title" and
        // "matched against nothing" — a corpus carrying a decoy that satisfies
        // the pattern without the screen saying the thing. Proved by mutation:
        // hiding the note left a body-scanned version of this assertion green.
        // The apparatus is annotation and is being removed (design.md D7), so
        // an assertion resting on it proves nothing about what ships.
        var notes = spec.visibleNamed(join, "titleUnknownText")
        compare(notes.length, 1,
                "the preview must carry a visible statement of what it could not "
                + "check")
        var low = notes[0].text.toLowerCase()
        verify(/\b(compar\w*|check\w*|match\w*)\b/.test(low),
               "which must name the comparison: " + notes[0].text)
        verify(/\bsame[- ]title\b|\btitle\b/.test(low),
               "and name what is compared")
        verify(/\b(has not been made|not been made|cannot|has not|is not|nothing)\b/.test(low),
               "and say it has NOT been made — an unrun check read as a clean "
               + "result is the impersonation this screen is written against: "
               + notes[0].text)

        // After the join the comparison DOES run, which is what makes the
        // absence above a timing fact rather than a missing feature.
        join.join()
        compare(join.lookalikes.length, 1,
                "the comparison runs once a title exists — it is late, not absent")
        compare(spec.visibleNamed(join, "lookalikePanel").length, 1)
        view.destroy()
    }

    function test_the_same_address_is_not_shown_beside_itself_as_a_second_stoa() {
        var addr = "b02d5e77" + "dd".repeat(28)
        var screen = makeJoin({}, {
            stoaAddress: addr,
            stoaGenesis: "00ff",
            foundingTitle: "Nym Research",
            heldStoas: [{ stoa: addr, foundingTitle: "Nym Research" }]
        })

        compare(screen.lookalikes.length, 0,
                "a Stoa already held at the SAME address is not a second Stoa")
        compare(spec.visibleNamed(screen, "lookalikePanel").length, 0)
        screen.destroy()
    }

    // ---- what the preview asks the core -----------------------------------
    //
    // Written by the implementer as the lookup was built; the tester owns the
    // final suite. Each fake below answers from the REQUEST where the answer
    // matters, so a screen showing a stale answer is distinguishable from one
    // showing the right one.

    // A join screen whose lookup is answered with `reply`, a get_stoa JSON
    // string, for a fixed reference.
    function previewAnswered(reply, extraProps) {
        var props = { stoaAddress: "b02d5e77" + "ab".repeat(28), stoaGenesis: "00ff" }
        if (extraProps !== undefined)
            for (var k in extraProps)
                props[k] = extraProps[k]
        return makeJoin({ "get_stoa": reply }, props)
    }

    function test_previewing_a_reference_looks_it_up_with_what_a_join_would_send() {
        var addr = "b02d5e77" + "12".repeat(28)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_stoa": function (r) { return spec.getStoaReply(r.stoa, true, "Agora", "") },
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Agora","policy":"open"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        var list = spec.namedAnywhere(view, "stoaList")[0]
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        // The display prefix, so "what a join would send" is not trivially the
        // pasted string.
        list.pasted = JSON.stringify({ stoa: "stoa:" + addr, genesis: "00ff" })
        list.preview()

        compare(spec.callsTo(bridge, "get_stoa"), 1, "looked up without the user acting")
        compare(spec.callsTo(bridge, "join_stoa"), 0, "and joined nothing")

        join.join()
        var asked = JSON.parse(spec.lastArgsTo(bridge, "get_stoa")[0])
        var joined = JSON.parse(spec.lastArgsTo(bridge, "join_stoa")[0])
        compare(asked.stoa, joined.stoa, "the lookup carries the join's address")
        compare(asked.genesis, joined.genesis, "and the join's record")
        compare(asked.stoa, addr, "with no display prefix")
        view.destroy()
    }

    function test_no_lookup_is_made_without_a_reference_or_for_a_malformed_paste() {
        Core.bridge = bridgeFor({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        compare(spec.callsTo(bridge, "get_stoa"), 0, "nothing is previewed at startup")

        var list = spec.namedAnywhere(view, "stoaList")[0]
        list.pasted = "stoa:b02d5e77a41c6b9013c6a9408ff4af23"
        list.preview()
        compare(spec.callsTo(bridge, "get_stoa"), 0, "a malformed paste looks nothing up")
        view.destroy()
    }

    function test_a_fallback_title_fills_the_founding_position_and_says_no_moderator_title_is_held() {
        var screen = previewAnswered(spec.getStoaReply("x", true, "Agora", ""))

        compare(spec.visibleNamed(screen, "foundingTitlePanel").length, 1)
        compare(spec.visibleNamed(screen, "foundingTitleText")[0].text, "Agora")
        compare(spec.visibleNamed(screen, "currentTitlePanel").length, 0,
                "a fallback title is not a current title")
        compare(spec.visibleNamed(screen, "titleUnknownNote").length, 0,
                "a founding title IS available here")

        var notes = spec.visibleNamed(screen, "fallbackNote")
        compare(notes.length, 1, "the fallback is stated, not only used to pick a panel")
        var low = notes[0].text.toLowerCase()
        verify(/\bmoderator\b/.test(low) && /\bno\b/.test(low),
               "it says no moderator-set title is held here: " + notes[0].text)
        // The word "not" may be modified by an adverb ("has definitely not
        // been renamed"), so the earlier form of this check —
        // `/\b(has not|hasn't|never) been renamed\b/` — is defeated by a
        // single inserted word while still asserting exactly the false claim
        // the requirement forbids. `\bnot\b[^.]{0,20}\brenamed\b` catches
        // "not" and "renamed" within twenty characters of each other in
        // EITHER order, which a synonym-only rewrite cannot dodge by
        // rearranging or padding.
        verify(!/\bnot\b[^.]{0,20}\brenamed\b/.test(low)
               && !/\brenamed\b[^.]{0,20}\bnot\b/.test(low),
               "and never that the Stoa has not been renamed, however phrased: "
               + notes[0].text)
        screen.destroy()
    }

    function test_a_non_fallback_title_fills_the_current_position_and_never_the_founding_one() {
        var screen = previewAnswered(spec.getStoaReply("x", false, "Stoa Poikile", ""))

        compare(spec.visibleNamed(screen, "currentTitleText")[0].text, "Stoa Poikile")
        verify(spec.visibleText(screen).indexOf("CHOSEN BY A MODERATOR") >= 0,
               "labelled as the moderator's")
        compare(spec.visibleNamed(screen, "foundingTitlePanel").length, 0,
                "a current title is never rendered as the founding one")
        compare(spec.visibleNamed(screen, "fallbackNote").length, 0)
        compare(spec.visibleNamed(screen, "titleUnknownNote").length, 1,
                "no founding title is available, and the screen says so")
        screen.destroy()
    }

    function test_a_description_is_rendered_only_from_a_non_fallback_reply_and_only_when_non_empty() {
        var described = previewAnswered(spec.getStoaReply("x", false, "Stoa Poikile", "the painted porch"))
        compare(spec.visibleNamed(described, "currentDescriptionText")[0].text, "the painted porch")
        compare(spec.visibleNamed(described, "currentDescriptionCaption").length, 1)
        described.destroy()

        var empty = previewAnswered(spec.getStoaReply("x", false, "Stoa Poikile", ""))
        compare(spec.visibleNamed(empty, "currentDescriptionCaption").length, 0,
                "no caption over an empty description")
        empty.destroy()

        // A fallback reply carrying a description is not one the core sends; the
        // view must still render none from it.
        var fallback = previewAnswered(spec.getStoaReply("x", true, "Agora", "not a moderator's"))
        verify(spec.visibleText(fallback).indexOf("not a moderator's") < 0,
               "no description from a fallback reply")
        fallback.destroy()
    }

    function test_a_refused_lookup_renders_the_cores_reason_and_withdraws_no_join() {
        var addr = "b02d5e77" + "34".repeat(28)
        var screen = makeJoin({
            "get_stoa": '{"error":"the op log\'s storage could not be used: disk"}',
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Agora","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff" })

        var failures = spec.visibleNamed(screen, "lookupFailureText")
        compare(failures.length, 1)
        compare(failures[0].text, "the op log's storage could not be used: disk", "unreworded")
        compare(spec.visibleNamed(screen, "foundingTitlePanel").length, 0)
        compare(spec.visibleNamed(screen, "currentTitlePanel").length, 0)
        compare(spec.visibleNamed(screen, "fallbackNote").length, 0,
                "a failure is not a fallback")
        compare(spec.visibleNamed(screen, "joinFailurePanel").length, 0,
                "and not a refused join")
        compare(spec.visibleNamed(screen, "joinButton").length, 1, "the join is still offered")

        screen.join()
        compare(screen.joinState, "joined", "and the join's own reply decides")
        compare(spec.visibleNamed(screen, "joinedPanel").length, 1)
        screen.destroy()
    }

    function test_a_misshapen_lookup_reply_is_a_failure_of_the_views_own() {
        var shapes = [
            '{"stoa":"x","title":"Agora","description":"","policy":"open"}',
            '{"stoa":"x","title":"Agora","description":"","policy":"open","isGenesisFallback":"true"}',
            '{"stoa":"x","title":7,"description":"","policy":"open","isGenesisFallback":true}',
            '{"stoa":"x","title":"Agora","description":null,"policy":"open","isGenesisFallback":false}'
        ]
        for (var i = 0; i < shapes.length; i++) {
            var screen = previewAnswered(shapes[i])
            var failures = spec.visibleNamed(screen, "lookupFailureText")
            compare(failures.length, 1, "shape " + i + " is a failure")
            verify(failures[0].text.length > 0, "carrying a reason of the view's own")
            compare(spec.visibleNamed(screen, "foundingTitlePanel").length, 0, "shape " + i)
            compare(spec.visibleNamed(screen, "currentTitlePanel").length, 0, "shape " + i)
            compare(spec.visibleNamed(screen, "joinButton").length, 1, "shape " + i)
            screen.destroy()
        }
    }

    function test_a_blank_looked_up_title_is_a_failure_naming_it_blank() {
        var blanks = ["", " ​　"]
        for (var i = 0; i < blanks.length; i++) {
            for (var f = 0; f < 2; f++) {
                var screen = previewAnswered(
                    spec.getStoaReply("x", f === 0, blanks[i], "a description"))
                var failures = spec.visibleNamed(screen, "lookupFailureText")
                compare(failures.length, 1, "blank " + i + ", fallback " + (f === 0))
                verify(/\bblank\b/.test(failures[0].text.toLowerCase()),
                       "the reason names the title as blank: " + failures[0].text)
                compare(spec.visibleNamed(screen, "foundingTitlePanel").length, 0)
                compare(spec.visibleNamed(screen, "currentTitlePanel").length, 0)
                compare(spec.visibleNamed(screen, "fallbackNote").length, 0)
                verify(spec.visibleText(screen).indexOf("a description") < 0,
                       "no description from that reply")
                compare(spec.visibleNamed(screen, "titleUnknownNote").length, 1,
                        "no founding title is available")
                compare(spec.visibleNamed(screen, "joinButton").length, 1)
                screen.destroy()
            }
        }
    }

    function test_the_views_blank_list_is_the_thirty_the_core_lists() {
        // Hardcoded, not read back from `Core.blankCodeUnits`: an assertion
        // against the list itself passes whatever it holds. The core's
        // `the_blank_characters_are_exactly_the_thirty_the_spec_lists` pins the
        // same values on the other side.
        var listed = [
            0x0009, 0x000A, 0x000B, 0x000C, 0x000D, 0x0020, 0x0085, 0x00A0,
            0x1680, 0x2000, 0x2001, 0x2002, 0x2003, 0x2004, 0x2005, 0x2006,
            0x2007, 0x2008, 0x2009, 0x200A, 0x2028, 0x2029, 0x202F, 0x205F,
            0x3000, 0x200B, 0x200C, 0x200D, 0x2060, 0xFEFF
        ]
        compare(Core.blankCodeUnits.length, 30)
        for (var i = 0; i < listed.length; i++)
            compare(Core.blankCodeUnits[i], listed[i], "code point " + i)

        // Each alone, through the screen a user reads.
        for (var j = 0; j < listed.length; j++) {
            var screen = previewAnswered(
                spec.getStoaReply("x", true, String.fromCharCode(listed[j]), ""))
            compare(spec.visibleNamed(screen, "lookupFailureText").length, 1,
                    "U+" + listed[j].toString(16) + " alone is blank")
            compare(spec.visibleNamed(screen, "foundingTitlePanel").length, 0)
            screen.destroy()
        }

        // And no wider: two characters that render as nothing and are NOT in
        // the list.
        var outside = ["‎", "᠎"]
        for (var k = 0; k < outside.length; k++) {
            var ok = previewAnswered(spec.getStoaReply("x", true, outside[k], ""))
            compare(spec.visibleNamed(ok, "lookupFailureText").length, 0, "not blank: " + k)
            compare(spec.visibleNamed(ok, "foundingTitleText")[0].text, outside[k])
            ok.destroy()
        }
    }

    function test_one_visible_letter_among_blank_characters_is_a_title() {
        var title = " ​a　﻿"
        var founding = previewAnswered(spec.getStoaReply("x", true, title, ""))
        compare(spec.visibleNamed(founding, "lookupFailureText").length, 0)
        compare(spec.visibleNamed(founding, "foundingTitleText")[0].text, title,
                "rendered in the founding position, untrimmed")
        founding.destroy()

        var current = previewAnswered(spec.getStoaReply("x", false, title, ""))
        compare(spec.visibleNamed(current, "lookupFailureText").length, 0)
        compare(spec.visibleNamed(current, "currentTitleText")[0].text, title,
                "rendered in the current position, untrimmed")
        current.destroy()
    }

    function test_looked_up_text_is_not_interpreted_as_markup() {
        var markup = "<b>Agora</b> &amp;"
        var founding = previewAnswered(spec.getStoaReply("x", true, markup, ""))
        compare(spec.visibleNamed(founding, "foundingTitleText")[0].textFormat, Text.PlainText)
        founding.destroy()

        var current = previewAnswered(spec.getStoaReply("x", false, markup, markup))
        compare(spec.visibleNamed(current, "currentTitleText")[0].textFormat, Text.PlainText)
        compare(spec.visibleNamed(current, "currentDescriptionText")[0].textFormat,
                Text.PlainText)
        current.destroy()
    }

    function test_a_second_reference_renders_nothing_from_the_first_lookup() {
        var a = "aaaaaaaa" + "56".repeat(28)
        var b = "bbbbbbbb" + "78".repeat(28)
        // Answered FROM THE REQUEST: A falls back, B is refused. A fake giving
        // one answer to both could not tell a stale title from a fresh one.
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_stoa": function (r) {
                return r.stoa === a
                    ? spec.getStoaReply(a, true, "Nym Research", "")
                    : '{"error":"the genesis record does not hash to this address"}'
            }
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        var join = spec.namedAnywhere(view, "joinScreen")[0]

        view.previewing = { stoa: a, genesis: "00ff" }
        compare(join.foundingTitle, "Nym Research", "A's lookup answered A")

        view.previewing = { stoa: b, genesis: "00ff" }
        compare(JSON.parse(spec.lastArgsTo(bridge, "get_stoa")[0]).stoa, b,
                "B was looked up")
        var body = spec.bodyText(join)
        verify(body.indexOf("Nym Research") < 0, "A's title is not rendered for B: " + body)
        compare(spec.visibleNamed(join, "fallbackNote").length, 0,
                "nor A's fallback statement")
        compare(spec.visibleNamed(join, "lookupFailureText").length, 1,
                "B's own answer is what is shown")
        view.destroy()
    }

    // **A state `Main.qml` does not produce, on purpose.** Through the shipped
    // routes every reference on screen gets a fresh lookup synchronously, so a
    // stale one is always overwritten before anything renders, and no
    // Main-driven test can see the reference filter on `currentLookup`. Here the
    // address moves while the record goes empty, so no lookup is made for the
    // new pair and the old answer is still stored. Deleting the filter turns
    // this test red and nothing else. It is kept because a later change that
    // made lookups asynchronous would make the stale state reachable.
    function test_a_lookup_answered_for_one_address_is_not_rendered_over_another() {
        var screen = previewAnswered(spec.getStoaReply("x", true, "Nym Research", ""))
        compare(screen.foundingTitle, "Nym Research", "the first reference's answer")

        screen.stoaAddress = "cccccccc" + "01".repeat(28)
        screen.stoaGenesis = ""
        compare(screen.foundingTitle, "", "not rendered over another address")
        compare(spec.visibleNamed(screen, "fallbackNote").length, 0)
        screen.destroy()
    }

    // `stoa-navigation-view`'s "Joining shows what is being joined" requirement:
    // "Where both a successful join reply and a fallback reply carry a founding
    // title for the reference on screen, the join reply's is the one rendered."
    // Neither existing test drives BOTH a non-blank fallback lookup AND a
    // subsequent non-blank join reply for the SAME reference —
    // `test_a_resolved_current_title_fills_that_position_when_one_exists` starts
    // from a NON-fallback lookup (no founding title available before the join),
    // and `test_a_blank_founding_title_from_a_join_is_not_a_founding_title` also
    // starts from a non-fallback lookup. This is the precedence scenario itself.
    function test_a_join_replys_founding_title_takes_the_place_of_a_fallback_replys() {
        var addr = "b02d5e77" + "cd".repeat(28)
        var screen = makeJoin({
            "get_stoa": spec.getStoaReply(addr, true, "Agora", ""),
            "join_stoa": JSON.stringify({
                stoa: addr, foundingTitle: "Agora Renamed At Founding", policy: "open"
            })
        }, { stoaAddress: addr, stoaGenesis: "00ff" })

        // Before the join: the fallback's title fills the founding position.
        compare(screen.foundingTitle, "Agora", "the fallback fills the position first")

        screen.join()

        compare(screen.joinState, "joined")
        compare(screen.foundingTitle, "Agora Renamed At Founding",
                "the join reply's founding title takes the place of the fallback's")
        compare(spec.visibleNamed(screen, "foundingTitleText")[0].text,
                "Agora Renamed At Founding")
        verify(spec.visibleText(screen).indexOf("Agora Renamed At Founding") >= 0)
        // The fallback's own title must not still be on screen anywhere,
        // which is what "the two MUST NOT both be rendered" requires.
        verify(spec.bodyText(screen).indexOf("Agora") < 0 ||
               spec.bodyText(screen).indexOf("Agora Renamed At Founding") >= 0,
               "the fallback's bare title must not linger beside the join's")
        screen.destroy()
    }

    // The other half of the same precedence rule: "A fallback reply's `title`
    // MUST fill the founding-title position only while no successful join reply
    // for that reference carries a founding title that is not blank." So when
    // the join succeeds with a BLANK founding title, the fallback's earlier,
    // non-blank title must stay rather than being cleared.
    function test_a_fallback_replys_title_stays_when_the_join_replys_is_blank() {
        var addr = "b02d5e77" + "ef".repeat(28)
        var blanks = ["", " ​"]
        for (var i = 0; i < blanks.length; i++) {
            var screen = makeJoin({
                "get_stoa": spec.getStoaReply(addr, true, "Nym Research", ""),
                "join_stoa": JSON.stringify({ stoa: addr, foundingTitle: blanks[i], policy: "open" })
            }, { stoaAddress: addr, stoaGenesis: "00ff" })

            compare(screen.foundingTitle, "Nym Research", "the fallback fills it first")

            screen.join()

            compare(screen.joinState, "joined", "blank " + i + ": the join itself succeeded")
            compare(screen.foundingTitle, "Nym Research",
                    "blank " + i + ": the fallback's title stays when the join's is blank")
            compare(spec.visibleNamed(screen, "foundingTitlePanel").length, 1,
                    "blank " + i + ": the founding panel is still rendered")
            compare(spec.visibleNamed(screen, "foundingTitleText")[0].text, "Nym Research")
            compare(spec.visibleNamed(screen, "titleUnknownNote").length, 0,
                    "blank " + i + ": a founding title IS available, from the fallback")
            screen.destroy()
        }
    }

    function test_a_blank_founding_title_from_a_join_is_not_a_founding_title() {
        var addr = "b02d5e77" + "9a".repeat(28)
        var blanks = ["", " ​"]
        for (var i = 0; i < blanks.length; i++) {
            var screen = makeJoin({
                "get_stoa": spec.getStoaReply(addr, false, "Renamed", ""),
                "join_stoa": JSON.stringify({ stoa: addr, foundingTitle: blanks[i], policy: "open" })
            }, { stoaAddress: addr, stoaGenesis: "00ff" })
            screen.join()

            compare(screen.joinState, "joined", "the join itself succeeded")
            compare(spec.visibleNamed(screen, "foundingTitlePanel").length, 0,
                    "blank " + i + " is not rendered as a founding title")
            var notes = spec.visibleNamed(screen, "titleUnknownText")
            compare(notes.length, 1, "no founding title is available here")
            verify(!/\bjoining is what would\b/.test(notes[0].text.toLowerCase()),
                   "and the note no longer says joining would supply one: " + notes[0].text)
            screen.destroy()
        }
    }

    function test_a_blank_title_matches_no_held_stoa() {
        var held = "7f3a91c4" + "bc".repeat(28)
        var blanks = ["", " ​"]
        for (var i = 0; i < blanks.length; i++) {
            // A fallback lookup carrying the same blank title the held Stoa has.
            var screen = previewAnswered(
                spec.getStoaReply("x", true, blanks[i], ""),
                { heldStoas: [{ stoa: held, foundingTitle: blanks[i] }] })

            // The load-bearing assertion for the EMPTY case: `foundingTitle`
            // is "" whether the normaliser's blank check ran and refused the
            // lookup, or was deleted and let an empty-titled fallback through
            // — an empty fallback title is ALSO "". Only the lookup's own `ok`
            // flag tells the two apart, and only this assertion depends on it:
            // deleting `Core.stoaMetadataFrom`'s blank check turns this
            // green-either-way for blank "" without it.
            compare(screen.currentLookup.ok, false,
                    "blank " + i + " must be a FAILED lookup, not a fallback " +
                    "whose title happens to be empty")

            compare(screen.lookalikes.length, 0, "blank " + i + " matches nothing")
            compare(spec.visibleNamed(screen, "lookalikePanel").length, 0)
            compare(spec.visibleNamed(screen, "titleUnknownText").length, 1,
                    "and the screen says the comparison has not been made")
            screen.destroy()
        }
    }

    function test_titles_among_blank_characters_are_compared_untrimmed() {
        var held = "7f3a91c4" + "de".repeat(28)
        var title = " ​a　"
        var same = previewAnswered(spec.getStoaReply("x", true, title, ""),
                                   { heldStoas: [{ stoa: held, foundingTitle: title }] })
        compare(same.lookalikes.length, 1, "the same four characters match")
        same.destroy()

        var trimmed = previewAnswered(spec.getStoaReply("x", true, title, ""),
                                      { heldStoas: [{ stoa: held, foundingTitle: "a" }] })
        compare(trimmed.lookalikes.length, 0, "and `a` alone does not: nothing is trimmed")
        trimmed.destroy()
    }

    function test_a_fallback_title_runs_the_lookalike_comparison_before_a_join() {
        // The comparison is late only where no founding title is available. A
        // fallback reply makes one available at preview time, so the warning
        // arrives before the decision it exists to inform.
        var held = "7f3a91c4" + "ef".repeat(28)
        var screen = previewAnswered(spec.getStoaReply("x", true, "Nym Research", ""),
                                     { heldStoas: [{ stoa: held, foundingTitle: "Nym Research" }] })
        compare(screen.joinState, "previewing", "nothing has been joined")
        compare(screen.lookalikes.length, 1)
        compare(spec.visibleNamed(screen, "lookalikePanel").length, 1)
        screen.destroy()
    }

    // ---- creation ---------------------------------------------------------

    // Every creation test below is in the key-held state, because that is the
    // only state the create affordance exists in. `get_master_key` is in each
    // fixture for that reason; without it the fake answers the error shape and
    // the screen is, correctly, in its could-not-be-read state.

    function test_the_create_affordance_is_present_and_usable_when_a_key_is_held() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false)
        })

        var buttons = spec.visibleNamed(screen, "createStoaButton")
        compare(buttons.length, 1, "the create affordance must be present and actionable")
        compare(buttons[0].enabled, true)
        screen.destroy()
    }

    function test_the_create_affordance_is_not_instantiated_when_no_key_is_held() {
        // Inverted from the removed "always offered" requirement. NOT HIDDEN:
        // `namedAnywhere` walks visible and invisible elements alike, so a
        // `visible: false` button would still be found and fail this.
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        })

        compare(spec.namedAnywhere(screen, "createStoaButton").length, 0,
                "no create action may exist in the element tree with no key held")
        compare(spec.namedAnywhere(screen, "createTitleField").length, 0,
                "nor a title field for a new Stoa")
        screen.destroy()
    }

    function test_a_creation_refused_for_want_of_a_key_renders_the_cores_reason() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false),
            "create_stoa": '{"error":"No keystore found. Create one before posting; '
                         + 'the reply box comes back when a reply would actually send."}'
        })
        screen.createTitle = "Transport Notes"
        screen.create()

        compare(screen.createState, "failed")
        compare(screen.created, null, "nothing may be reported as created")
        verify(spec.visibleText(screen).indexOf("No keystore found.") >= 0,
               "the keystore's own reason, unreworded")
        screen.destroy()
    }

    function test_every_blank_title_reaches_the_core_rather_than_being_refused_here() {
        // The core refuses a blank title and the view does not: the core is the
        // one place titles are judged, and a second check here would be a second
        // copy of the blank list that could drift from it. So the call is MADE,
        // carrying exactly what was typed, and the core's refusal is rendered.
        //
        // The fake's refusal names the title it was sent, so an assertion that
        // the reason is on screen also shows it was this request's reason.
        var refusal = function (request) {
            return JSON.stringify({ error: "title: title is blank <" + request.title + ">" })
        }
        var typed = ["", "   ", "​　"]
        for (var i = 0; i < typed.length; i++) {
            var screen = makeList({
                "list_stoas": '{"items":[],"page":0,"hasMore":false}',
                "get_master_key": spec.heldKeyReply(aKeyHex(), false),
                "create_stoa": refusal
            })
            var bridge = Core.bridge
            screen.createTitle = typed[i]
            screen.create()

            compare(spec.callsTo(bridge, "create_stoa"), 1,
                    "the create call must be MADE — the reply decides, not a check here")
            compare(JSON.parse(spec.lastArgsTo(bridge, "create_stoa")[0]).title, typed[i],
                    "and the title sent is exactly what was typed")
            compare(screen.createState, "failed")
            compare(screen.created, null, "nothing may be reported as created")
            verify(spec.visibleText(screen).indexOf("title is blank <" + typed[i] + ">") >= 0,
                   "the core's reason, unreworded")
            compare(spec.visibleNamed(screen, "createdAddress").length, 0,
                    "and no address is rendered as a Stoa just created")
            screen.destroy()
        }
    }

    function test_the_created_address_is_rendered() {
        var addr = "b02d5e77a41c6b9013c6a9408ff4af235d7e1b06c92a84f13be057dc6104a8bf"
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false),
            "create_stoa": '{"stoa":"' + addr + '","foundingTitle":"Transport Notes","policy":"open"}'
        })
        screen.createTitle = "Transport Notes"
        screen.create()

        var labels = spec.visibleNamed(screen, "createdAddress")
        compare(labels.length, 1, "there is no registry to look a Stoa up in later")
        compare(labels[0].text, addr, "the address the core returned, in full")
        screen.destroy()
    }

    function test_the_same_title_twice_is_one_stoa_reported_twice() {
        var addr = "b02d5e77" + "ff".repeat(28)
        var reply = '{"stoa":"' + addr + '","foundingTitle":"Transport Notes","policy":"open"}'
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false),
            "create_stoa": reply
        })
        var bridge = Core.bridge

        screen.createTitle = "Transport Notes"
        screen.create()
        var firstAddress = screen.created.stoa

        screen.create()

        compare(screen.createState, "created", "the second outcome is SUCCESS")
        compare(screen.created.stoa, firstAddress,
                "and names the Stoa the first creation returned")
        var shown = spec.visibleText(screen).toLowerCase()
        verify(shown.indexOf("collision") < 0 && shown.indexOf("duplicate") < 0
               && shown.indexOf("already exists") < 0,
               "nothing may read as a collision: " + shown)

        // The title submitted is the title the user typed, both times. A suffix
        // appended to obtain a different Stoa mints one with a title they did
        // not choose, permanently, at an address nobody can withdraw.
        var sent = String(spec.lastArgsTo(bridge, "create_stoa"))
        verify(sent.indexOf('"title":"Transport Notes"') >= 0,
               "nothing may be appended to the user's title: " + sent)
        screen.destroy()
    }

    // ---- what the view holds of its own -----------------------------------

    function test_the_view_supplies_no_stoa_of_its_own_before_one_is_chosen() {
        // The top-level view used to carry stoaAddress/stoaTitle/stoaGenesis as
        // developer-filled properties. A second source for the one value these
        // screens exist to supply is a build that ships a hardcoded Stoa.
        Core.bridge = bridgeFor({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var view = mainComponent.createObject(null, {})

        compare(view.chosen, null, "no Stoa is chosen at startup")
        compare(view.screenShown, "list", "and no feed is rendered for any Stoa")
        verify(view.stoaAddress === undefined,
               "the view must not carry a Stoa address property of its own")
        verify(view.stoaGenesis === undefined,
               "nor a genesis record property")
        view.destroy()
    }

    Component { id: mainComponent; Main {} }

    function test_the_feed_is_given_the_address_and_the_record_from_the_chosen_row() {
        var addr = "aa".repeat(32)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Held"}],'
                        + '"page":0,"hasMore":false}',
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        var view = mainComponent.createObject(null, {})
        var list = spec.visibleNamed(view, "stoaList")[0]
        var held = {}
        held[addr] = "00ff00ff"
        list.genesisByStoa = held

        list.stoaChosen(addr, "Held", "00ff00ff")

        compare(view.screenShown, "feed")
        compare(view.chosen.stoa, addr)
        compare(view.chosen.genesis, "00ff00ff",
                "the record must travel with the address")
        view.destroy()
    }

    function test_a_failed_reload_makes_the_stale_listing_unreadable_not_merely_unrendered() {
        // The previous shape kept the last page in `rows` after a failed reload
        // — correct, because a failure must not blank a good listing underneath
        // a banner — and relied on every READER remembering
        // `readState === "ok" ? rows : []`. That guard was written twice, in two
        // files, which is the point at which the data should absorb it.
        //
        // This asserts the absorbed form: after a failure the rows are gone from
        // what any caller can read, while the raw listing is still there for the
        // screen's own purposes.
        var addr = "aa".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Held"}],'
                        + '"page":0,"hasMore":false}'
        })
        compare(screen.readState, "ok")
        compare(screen.visibleRows.length, 1, "the good listing is readable")

        Core.bridge = bridgeFor({ "list_stoas": '{"error":"the membership store is locked"}' })
        screen.reload()

        compare(screen.readState, "failed")
        compare(screen.visibleRows.length, 0,
                "a caller reading the rows after a failed read gets none, "
                + "without having to remember a guard")
        compare(screen.lastListing.length, 1,
                "while the previous page is still held, so a retry need not refetch "
                + "and a failure does not destroy a good listing")

        // And nothing stale is on screen, which is the user-visible half.
        var body = spec.bodyText(screen)
        verify(body.indexOf("Held") < 0, "no stale row may render: " + body)
        verify(body.indexOf("locked") >= 0, "the core's reason is what shows instead")
        screen.destroy()
    }

    function test_a_user_who_opened_a_stoa_can_return_to_the_list() {
        // **Arriving somewhere is half a transition.** Nothing cleared `chosen`,
        // and `FeedScreen` declared no signals at all — so the first row a user
        // opened was the last screen they saw until they restarted the app. The
        // list, the share affordance and the join field, which are the whole of
        // what this piece added, all became unreachable after one click.
        //
        // 103 of 103 tests passed while that held, because the suite asserted up
        // to the transition and nothing after it. A one-way trip satisfies every
        // scenario about arriving.
        var addr = "aa".repeat(32)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Held"}],'
                        + '"page":0,"hasMore":false}',
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        var view = mainComponent.createObject(null, {})
        var list = spec.visibleNamed(view, "stoaList")[0]
        list.stoaChosen(addr, "Held", "00ff00ff")
        compare(view.screenShown, "feed", "the user is on the feed")

        // The affordance a user acts on, found on screen rather than assumed —
        // a `closed` signal nothing renders a control for is a route only a test
        // can take.
        var backs = spec.visibleNamed(view, "feedBackButton")
        compare(backs.length, 1, "a visible affordance must offer the return")
        backs[0].clicked()

        compare(view.screenShown, "list", "and acting on it renders the list")
        compare(view.chosen, null, "with no Stoa still chosen")

        // The list is genuinely usable again, not merely nominally rendered:
        // the row is back and so is the paste field's action.
        var body = spec.bodyText(spec.visibleNamed(view, "stoaList")[0])
        verify(body.indexOf("Held") >= 0,
               "the list's rows are on screen again: " + body)
        view.destroy()
    }

    function test_a_preview_requested_while_a_feed_is_open_is_not_swallowed() {
        // `screenShown` is an ordered ternary testing `chosen` first, so before
        // the fix a preview requested while a feed was open left `screenShown`
        // at "feed" — the DJoinScreen rebound to the new reference behind an
        // invisible panel, and nothing shown.
        //
        // Latent rather than live: nothing on the feed emits a preview request
        // today. But the spec's model is that an address inside a post is an
        // affordance a reader acts on, and a post lives on the feed — so the
        // piece adding that affordance would have hit exactly this. It is pinned
        // now so that piece inherits the behaviour instead of the silence.
        var addr = "aa".repeat(32)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        var view = mainComponent.createObject(null, {})

        view.open(addr, "Held", "00ff00ff")
        compare(view.screenShown, "feed")

        view.preview("bb".repeat(32), "00ff")

        compare(view.screenShown, "join",
                "a preview requested from anywhere must be shown, not swallowed")
        compare(view.chosen, null,
                "and the state (chosen, previewing) both set must not exist — "
                + "the ternary renders the state rather than resolving a clash")

        // And the reverse direction, so the exclusion is not one-way.
        view.open(addr, "Held", "00ff00ff")
        compare(view.screenShown, "feed")
        compare(view.previewing, null, "opening a Stoa clears a pending preview")
        view.destroy()
    }

    function test_reopening_a_stoa_after_returning_still_carries_its_record() {
        // The return must not cost the record. `chosen` is cleared on the way
        // out, and `genesisByStoa` lives on the list rather than on the feed, so
        // a second open is as complete as the first — which is what makes the
        // return safe to take rather than a thing to avoid.
        var addr = "aa".repeat(32)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Held"}],'
                        + '"page":0,"hasMore":false}',
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        var view = mainComponent.createObject(null, {})
        var list = spec.visibleNamed(view, "stoaList")[0]
        var held = {}
        held[addr] = "00ff00ff"
        list.genesisByStoa = held

        list.stoaChosen(addr, "Held", "00ff00ff")
        spec.visibleNamed(view, "feedBackButton")[0].clicked()
        compare(view.screenShown, "list")

        list.stoaChosen(addr, "Held", list.genesisFor(addr))
        compare(view.screenShown, "feed")
        compare(view.chosen.genesis, "00ff00ff",
                "the record still travels on a second open")
        view.destroy()
    }

    function test_no_record_is_invented_for_a_row_the_view_has_none_for() {
        var addr = "bb".repeat(32)
        Core.bridge = bridgeFor({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Not held"}],'
                        + '"page":0,"hasMore":false}',
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"error":"a genesis record is required for this Stoa"}'
        })
        var view = mainComponent.createObject(null, {})
        var list = spec.visibleNamed(view, "stoaList")[0]

        list.stoaChosen(addr, "Not held", list.genesisFor(addr))

        compare(view.chosen.genesis, "",
                "no placeholder or fabricated record may be sent")
        // And whatever the core then refuses is rendered as the failure it is,
        // which is distinguishable from a Stoa holding nothing.
        var feed = spec.visibleNamed(view, "feed")[0]
        compare(feed.readState, "failed")
        view.destroy()
    }

    // ---- behaviour the spec did not decide --------------------------------

    // NO SPEC: the spec requires that what is shared carry both halves and says
    // nothing about the encoding. A one-line JSON object was chosen — see
    // design.md D1 — and this pins the shape so a later change to it is a
    // deliberate one. Anyone who has already copied a reference holds a string
    // in this format, so changing it strands them.
    function test_the_reference_format_is_a_json_object_with_two_hex_fields() {
        var text = DStoaReference.shareText("aabb", "ccdd")
        var obj = JSON.parse(text)
        compare(obj.stoa, "aabb")
        compare(obj.genesis, "ccdd")
        compare(Object.keys(obj).length, 2, "no third field, so nothing to ignore")
    }

    // NO SPEC: the spec's three paste outcomes do not cover an EMPTY field. It
    // is refused as not-a-reference rather than silently doing nothing, on the
    // grounds that a button that appears to do nothing is worse than one that
    // says why — but the spec does not require either.
    function test_an_empty_paste_is_refused_rather_than_ignored() {
        var screen = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var bridge = Core.bridge
        screen.pasted = ""
        screen.preview()
        verify(screen.pasteFailure.length > 0, "an empty paste says why nothing happened")
        compare(spec.callsTo(bridge, "join_stoa"), 0)
        screen.destroy()
    }

    // NO SPEC: the spec requires the created address be rendered and does not
    // say what happens when a success arrives WITHOUT one. It is treated as a
    // failure here rather than rendering a created Stoa with no address, which
    // would be a Stoa the user cannot name, share or read.
    //
    // Driven from the key-held state through the create button, for the reason
    // `test_a_created_stoa_is_openable_from_the_creation_reply_alone` gives.
    function test_a_creation_success_carrying_no_address_is_a_failure() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false),
            "create_stoa": '{"foundingTitle":"Transport Notes","policy":"open"}'
        })
        var create = spec.visibleNamed(screen, "createStoaButton")
        compare(create.length, 1,
                "creation is reachable only with a key held, so that is where "
                + "this is driven from")
        screen.createTitle = "Transport Notes"
        create[0].clicked()
        compare(screen.createState, "failed")
        compare(screen.created, null)
        screen.destroy()
    }

    // NO SPEC: the spec says the view's check is limited to "whether the input
    // carries the two halves at all" and does not say what a half that is
    // present but not a string does. A number or an object is refused rather
    // than coerced, so nothing but a string can reach JSON.stringify and the
    // core.
    function test_a_half_that_is_not_a_string_is_refused_rather_than_coerced() {
        compare(DStoaReference.parse('{"stoa":12345,"genesis":"00ff"}').ok, false)
        compare(DStoaReference.parse('{"stoa":"aa","genesis":{"x":1}}').ok, false)
        compare(DStoaReference.parse('["aa","00ff"]').ok, false,
                "an array is not a reference either")
    }

    // ---- rendering, where a property was standing in for it ----------------
    //
    // Every test in this section exists because a property assertion elsewhere
    // in this file passed against a mutation that broke the rendering. The
    // pattern is the one that moved the markup test from `text` to `textFormat`:
    // asking the implementation what it computed and agreeing, where the
    // requirement is about what reaches the screen. Each was proved by mutating
    // a `visible:` binding and watching the whole suite stay green.

    // A share affordance offered for a Stoa the view holds no record for
    // produces a string that fails to verify on somebody else's machine — the
    // third failure this file's header names. `canShare()` answering correctly
    // is not that requirement: the requirement is that no BUTTON is on screen.
    //
    // Proved: `visible: screen.canShare(row.rowStoa)` → `visible: true` on the
    // share button left the whole suite green with the defect present.
    function test_no_share_button_is_on_screen_for_a_row_whose_record_is_not_held() {
        var withRecord = "aa".repeat(32)
        var without = "bb".repeat(32)
        var screen = makeList({
            "list_stoas": '{"items":['
                        + '{"stoa":"' + withRecord + '","foundingTitle":"Held"},'
                        + '{"stoa":"' + without + '","foundingTitle":"Not held"}'
                        + '],"page":0,"hasMore":false}'
        })
        var held = {}
        held[withRecord] = "00ff00ff"
        screen.genesisByStoa = held
        screen.reload()

        // Two rows, one record. Both halves from one fixture, and the count is
        // hardcoded rather than derived: `<= 1` would pass on zero buttons, and
        // zero is a different defect that this same assertion must catch.
        compare(screen.visibleRows.length, 2, "the fixture must put two rows on screen")
        compare(spec.visibleNamed(screen, "shareButton").length, 1,
                "exactly one share affordance is on screen — the held row's — "
                + "and the unheld row must offer none")
        screen.destroy()
    }

    // The joined outcome has to be REPORTED, which is a thing on screen and not
    // a string in a property. A screen holding `joinState === "joined"` while
    // rendering nothing has told the user nothing.
    //
    // Proved: `visible: screen.joinState === "joined"` → `visible: false` on the
    // joined panel left the whole suite green with the defect present —
    // including the two tests that assert a repeat join is success.
    function test_the_joined_outcome_is_reported_on_screen_and_not_only_in_a_property() {
        var addr = "7f3a91c4" + "ee".repeat(28)
        var screen = makeJoin({
            "join_stoa": '{"stoa":"' + addr + '","foundingTitle":"Nym Research","policy":"open"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff", foundingTitle: "Nym Research" })

        compare(spec.visibleNamed(screen, "joinedPanel").length, 0,
                "nothing reports a join before the user acts")

        screen.join()

        compare(spec.visibleNamed(screen, "joinedPanel").length, 1,
                "a successful reply must put the outcome on screen, not only in joinState")
        // And the failure panel is not up alongside it. One outcome, not two.
        compare(spec.visibleNamed(screen, "joinFailurePanel").length, 0)
        screen.destroy()
    }

    // "No feed is rendered for any Stoa" before one is chosen. `screenShown` is
    // a derived string; what the spec forbids is a feed on screen.
    //
    // Proved: `visible: root.screenShown === "feed"` → `visible: true` on the
    // FeedScreen left the whole suite green with the defect present, the feed
    // rendering for the empty address at startup.
    function test_no_feed_is_on_screen_before_a_stoa_has_been_chosen() {
        Core.bridge = bridgeFor({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var view = mainComponent.createObject(null, {})

        compare(spec.visibleNamed(view, "feed").length, 0,
                "no feed may be on screen before a Stoa is chosen")
        compare(spec.visibleNamed(view, "joinScreen").length, 0,
                "nor a join preview, which nothing has been previewed for")
        compare(spec.visibleNamed(view, "stoaList").length, 1,
                "the list is the entry point — asserted so that a view rendering "
                + "NOTHING cannot satisfy the two absences above")
        view.destroy()
    }

    // ---- no number appears that nothing computed ---------------------------

    // Every run of digits in what a row renders, in source order.
    function digitRunsIn(text) {
        return text.match(/[0-9]+/g) || []
    }

    // The strengthened count assertion. The dev-writer's version was a blocklist
    // of the mockup's phrasings — `posts received here`, `nothing received yet`,
    // `3 posts` — and said so: it could not catch a count rendered as a bare
    // `31` in a row's margin, which is the shape the mockup actually draws.
    //
    // This is that same requirement stated as a relation instead: **every digit
    // on this screen must be traceable to something the fixture supplied.** The
    // fixture's address carries digits deliberately, so the test is not passing
    // merely because nothing anywhere renders a digit — remove the address from
    // the row and the allowed set shrinks rather than the assertion vanishing.
    //
    // What it still cannot see: a count rendered in a glyph that is not a digit,
    // or spelled out in words. That residue is smaller than a phrase blocklist
    // and is stated rather than left to be assumed.
    function test_no_digit_is_rendered_that_the_reply_did_not_supply() {
        // Digits in the address (4 0 3 9 1 …) and in the title, so the allowed
        // set is non-empty and the test has something to distinguish.
        var addr = "7f3a91c4" + "40".repeat(28)
        var title = "Transport Notes 1972"
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"' + title + '"}],'
                        + '"page":0,"hasMore":false}',
            // A thread listing is answered too, so a screen tempted to render
            // some OTHER call's page length has one available to render.
            "list_threads": '{"items":[{"thread":"t1"},{"thread":"t2"},{"thread":"t3"}],'
                          + '"page":0,"hasMore":true}'
        })
        compare(screen.readState, "ok")
        compare(screen.visibleRows.length, 1, "the fixture must put a row on screen")

        var shown = spec.visibleText(screen)
        var runs = spec.digitRunsIn(shown)
        // Non-empty, or the assertion below is vacuous: a screen rendering no
        // digits at all would satisfy "every digit is traceable" for free, and
        // that is exactly the self-satisfying shape this file is written against.
        verify(runs.length > 0,
               "the fixture's own digits must reach the screen, or this test "
               + "proves nothing: " + shown)

        for (var i = 0; i < runs.length; i++) {
            var run = runs[i]
            var fromAddress = addr.indexOf(run) >= 0
            var fromTitle = title.indexOf(run) >= 0
            verify(fromAddress || fromTitle,
                   "the screen renders the number '" + run + "', which nothing "
                   + "in the reply supplied. No call answers how many posts this "
                   + "peer holds for a Stoa, so any number here was invented or "
                   + "taken from another call's page length. Rendered: " + shown)
        }
        screen.destroy()
    }

    // ---- the count placeholder --------------------------------------------

    // The amended requirement permits a placeholder and requires that it be
    // derived from no reply. **The assertion is that it does not MOVE**, which
    // is the property that distinguishes a placeholder from a page length
    // rendered in the count position — the failure the requirement names.
    //
    // A null implementation that rendered a constant would satisfy "does not
    // move" for free, so the fixtures below differ in every quantity a screen
    // could reach for: the number of Stoas listed, and the number of threads the
    // other call answered. If the position were bound to either, the two runs
    // would differ.
    // Both halves of the amended requirement, in one function.
    //
    // **They were written as two and one of them did not appear in the run**, at
    // two different names, with no warning of any kind. What resolved it was
    // folding them together; what the cause was is NOT established, and saying
    // so is the honest version — an earlier draft of this comment blamed a
    // runner enumeration limit and that was a guess dressed as a measurement.
    // The count that looked like evidence for it was my own miscount: a
    // `grep -c "    function test_"` reads the words "function test_" inside a
    // comment as a declaration, so the file's real total was one lower than the
    // number the claim rested on.
    //
    // The diagnostic that actually works, if a test here stops appearing:
    // list the declared names and the run names and compare them, rather than
    // comparing two totals. Two totals cannot say which one is missing, and one
    // of them is easy to get wrong.
    //
    // **That diagnostic is now a gate and runs on every spec.**
    // `check_every_test_ran` in `run-qml-tests.sh` does exactly this comparison
    // and fails the run on a declared test that did not execute — so a
    // recurrence here, or in any sibling file, is now loud rather than silent.
    // It needs no root cause to work, which is what makes it the right answer
    // to an incident whose cause was never established.
    //
    // ONE mechanism for silent test loss has since been reproduced, though it
    // is NOT established as the cause of the incident above — no `_data` name
    // appears anywhere in this file's history. QtTest treats `test_foo_data()`
    // as the DATA PROVIDER for `test_foo()`, so declaring both removes BOTH
    // from the run: measured on Qt 6.10.3 as `3 passed, 0 failed` with neither
    // function executed, the only trace a `WARNING: ... no data supplied` line.
    // Recorded here because it is the same defect SHAPE, and because the next
    // person to lose a test in this file should check that name pattern first.
    function test_the_row_count_placeholder_claims_no_measurement() {
        // ---- it does not read as a measurement ---------------------------
        //
        // A placeholder saying "0 posts" would be constant AND false about
        // every Stoa on the list, so the two halves below are both needed.
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + "cc".repeat(32)
                        + '","foundingTitle":"Transport Notes"}],'
                        + '"page":0,"hasMore":false}'
        })
        var text = spec.namedAnywhere(screen, "rowCountPlaceholder")[0].text

        verify(spec.digitRunsIn(text).length === 0,
               "a placeholder carrying a numeral reads as a count: " + text)
        // NO SPEC: the requirement says a placeholder must not be presented as
        // this peer's measurement and does not fix the wording. This pins that
        // it does not assert emptiness, which is the one substitute value that
        // would be both digit-free and false.
        verify(text.toLowerCase().indexOf("nothing received") < 0
               && text.toLowerCase().indexOf("no posts") < 0,
               "asserting emptiness is a claim about a count nothing computed: "
               + text)
        screen.destroy()

        // ---- it does not MOVE with the data ------------------------------
        //
        // The property that distinguishes a placeholder from another call's
        // page length rendered in the count position, which is the failure the
        // requirement names. The two fixtures differ in every quantity a screen
        // could reach for — the number of Stoas listed and the number of threads
        // the other call answered — so a position bound to either would differ
        // between them.
        var addrOne = "aa".repeat(32)
        var addrTwo = "bb".repeat(32)

        var thin = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addrOne + '","foundingTitle":"One"}],'
                        + '"page":0,"hasMore":false}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        compare(thin.visibleRows.length, 1)
        var firstFound = spec.namedAnywhere(thin, "rowCountPlaceholder")
        compare(firstFound.length, 1, "the row carries the placeholder")
        var placeholder = firstFound[0].text
        verify(placeholder !== "", "and it renders something")
        thin.destroy()

        var thick = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addrOne + '","foundingTitle":"One"},'
                        + '{"stoa":"' + addrTwo + '","foundingTitle":"Two"}],'
                        + '"page":0,"hasMore":true}',
            "list_threads": '{"items":[{"thread":"t1"},{"thread":"t2"},'
                          + '{"thread":"t3"},{"thread":"t4"}],'
                          + '"page":0,"hasMore":true}'
        })
        compare(thick.visibleRows.length, 2, "a different amount of data")
        var bothFound = spec.namedAnywhere(thick, "rowCountPlaceholder")
        compare(bothFound.length, 2, "one placeholder per row")

        for (var i = 0; i < bothFound.length; i++)
            compare(bothFound[i].text, placeholder,
                    "the placeholder is the same string whatever the data, so "
                    + "it is not another call's page length in disguise")
        thick.destroy()
    }

    // ---- the two paste outcomes, pinned by meaning rather than by difference

    // The sibling `thread-read` piece shipped three refusal messages asserted to
    // be three DIFFERENT strings; a tester reworded one to actively misinform
    // and both distinguishability tests stayed green, because three misinforming
    // strings are still three distinct strings. So this asserts what each of the
    // two paste outcomes must and must not IMPLY, and the difference between
    // them falls out of that rather than standing in for it.
    //
    // The two mean opposite things about what to do next:
    //   malformed  — the paste went wrong; paste it again.
    //   unverified — somebody handed over a record that is not the one that
    //                address names; do NOT try again.
    function test_a_malformed_paste_and_an_unverified_record_say_different_things_to_do() {
        // 1. Text that is not a reference at all.
        var list = makeList({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        list.pasted = "not a reference"
        list.preview()
        var malformed = list.pasteFailure
        verify(malformed.length > 0, "a malformed paste must say something")

        var m = malformed.toLowerCase()
        // It must name the INPUT as the problem — what was pasted — so the
        // reader knows to look at what they pasted.
        verify(m.indexOf("pasted") >= 0 || m.indexOf("paste") >= 0,
               "a malformed paste must name what was pasted as the problem: " + malformed)
        // And it must NOT describe a comparison against the address. A malformed
        // paste has been compared against nothing; saying it failed a check would
        // send a reader who truncated their own paste to blame their sender.
        //
        // **Asserted as a structure rather than as a blocklist.** Three literal
        // phrasings were blocked here before, and a reviewer measured two
        // equivalents that passed — "could not be confirmed against its address"
        // and "failed the check against the address it names" — each carrying
        // exactly the misinformation the blocklist existed to stop. Adding those
        // two would leave a third.
        //
        // A claim of comparison needs both halves: a verb of checking, and the
        // address as the thing checked against. The shipped copy mentions the
        // address freely — it explains what a reference is — so neither half
        // alone can be forbidden. It is the conjunction that is the lie.
        var checkVerb = /\b(hash(es|ed)?|verif(y|ies|ied|ication)|match(es|ed)?|confirm(s|ed)?|check(s|ed)?|validat(e|es|ed|ion))\b/
        var againstAddress = /\b(against|to|with)\b[^.]{0,40}\baddress\b/
        verify(!(checkVerb.test(m) && againstAddress.test(m)),
               "nothing was compared against the address here, so nothing may "
               + "report that a comparison failed — a reader who truncated their "
               + "own paste would be sent to blame their sender: " + malformed)

        // 2. A well-formed pair the core refuses because it does not verify.
        var addr = "b02d5e77" + "88".repeat(28)
        var join = makeJoin({
            "join_stoa": '{"error":"the genesis record does not hash to this address; '
                       + 'whoever sent it did not send the record this address names"}'
        }, { stoaAddress: addr, stoaGenesis: "00ff" })
        join.join()

        var panels = spec.visibleNamed(join, "joinFailureText")
        compare(panels.length, 1, "the refusal must be on screen, not only in a property")
        var refusal = panels[0].text

        // The core's own words, unreworded — they name what was wrong, and that
        // is the difference between a user who knows the record they were sent
        // is wrong and one who thinks the app is broken.
        verify(refusal.indexOf("does not hash to this address") >= 0,
               "the core's refusal must be rendered as it came: " + refusal)
        // It must NOT invite a retry. Pressing again produces the same refusal,
        // and an interface offering one teaches the reader to mistake a
        // permanent answer for a transient fault.
        var retryButtons = spec.visibleNamed(join, "joinRetryButton")
        compare(retryButtons.length, 0, "a verification refusal offers no retry")
        var joinShown = spec.visibleText(join).toLowerCase()
        verify(joinShown.indexOf("try again") < 0 && joinShown.indexOf("retry") < 0,
               "nor may anything on screen suggest one: " + joinShown)

        // And only NOW the difference, which is a consequence of the two
        // meanings above rather than the whole of the assertion.
        verify(refusal.indexOf(malformed) < 0 && malformed.indexOf(refusal) < 0,
               "neither refusal may contain the other")
        join.destroy()
        list.destroy()
    }

    // ---- what the address proves, pinned by what a simplification would lose

    // The spec deliberately does not pin this copy's wording, because the thing
    // it has to convey is a distinction: the check is a hash comparison between
    // TWO INPUTS THE USER SUPPLIED, so a reader who pasted a hostile address and
    // saw a verified record has verified the attacker's record against the
    // attacker's address, perfectly successfully.
    //
    // The dev-writer's version pins three substrings, which fails on a reword
    // and passes on a misinformation. This asserts the two halves the copy
    // cannot lose — the scope of the proof, and the named remainder — and
    // requires that no unqualified claim of verification stands anywhere on the
    // screen. Someone simplifying this copy to "Verified." satisfies none of
    // the three.
    function test_the_address_note_cannot_be_simplified_into_an_unqualified_verified() {
        var screen = makeJoin({}, { stoaAddress: "aa".repeat(32), stoaGenesis: "00ff",
                                    foundingTitle: "Nym Research" })
        var notes = spec.visibleNamed(screen, "addressNote")
        compare(notes.length, 1, "the explanation must be on screen")
        var note = notes[0].text
        var n = note.toLowerCase()

        // 1. The proof is SCOPED to the record this address names. Not to the
        //    Stoa, its creator, or its reachability.
        verify(n.indexOf("this address names") >= 0 || n.indexOf("the record shown") >= 0,
               "the note must say what is proved: that the record shown is the "
               + "one this address names. Got: " + note)

        // 2. The remainder is NAMED, not merely unmentioned. An absence
        //    assertion here would pass on copy that said nothing at all, which
        //    is the failure: a reader told nothing assumes everything checked.
        verify(n.indexOf("unverified") >= 0 || n.indexOf("not verified") >= 0
               || n.indexOf("not checked") >= 0,
               "the note must NAME the unverified remainder rather than leave it "
               + "unmentioned. Got: " + note)

        // 3. And the scope is bounded by an explicit negation. Copy that states
        //    the positive and stops has told the reader they finished checking.
        verify(n.indexOf("nothing more") >= 0 || n.indexOf("not checked against") >= 0
               || n.indexOf("no registry") >= 0 || n.indexOf("any registry") >= 0,
               "the note must bound what the check consulted — no registry, no "
               + "peer, no third party. Got: " + note)

        // 3b. **Provenance is denied, in so many words.** This is the check the
        //     four above could not make, and a reviewer proved it: the note
        //     "The record shown is the one this address names, which confirms
        //     this is the Stoa you were sent. Nothing more is needed. No registry
        //     was consulted because none is needed; only the founding title is
        //     unverified." satisfies every one of them — it carries `this address
        //     names`, `unverified`, `no registry`, and dodges the three literal
        //     phrases below — while telling the reader the address is the one
        //     they were meant to receive.
        //
        //     That is THE claim spec.md's "What the address proves is stated
        //     exactly, and nothing broader" forbids, because it is the whole of
        //     the residual risk: a reader who pastes a hostile address and is
        //     shown a verified record has verified the attacker's record against
        //     the attacker's address, perfectly successfully. Assembling more
        //     required substrings cannot catch it — the hostile note contains
        //     every one. Only the negation can, so the negation is required.
        verify(/\bnothing\b[^.]{0,60}\b(says|establishes|proves|shows|tells)\b/.test(n)
               || /\bdoes not\b[^.]{0,60}\b(establish|prove|show|confirm|mean)\b/.test(n)
               || /\bcannot\b[^.]{0,60}\b(establish|prove|show|confirm|tell)\b/.test(n),
               "the note must explicitly DENY provenance — that nothing here "
               + "establishes this is the address you were meant to receive — "
               + "rather than merely omitting the claim. Got: " + note)
        // And the affirmative form of that claim is refused outright, however
        // the sentence around it is built.
        verify(!/\b(confirms|proves|establishes|means)\b[^.]{0,60}\b(you were sent|meant to receive|the right|the one you wanted)\b/.test(n),
               "nothing may claim this is the address the reader was sent or "
               + "meant to receive. Got: " + note)

        // 4. Nothing anywhere on the screen makes an unqualified claim of
        //    verification. This is the assertion a "Verified." simplification
        //    fails: the founding title is on screen beside it, freely chosen and
        //    matched against nothing, so a bare "verified" attaches to it.
        //
        //    This scan covers every visible Text on the screen, which today
        //    includes the apparatus column. **Nothing in the apparatus is what
        //    satisfies these assertions** — checks 1 to 3 read `addressNote` by
        //    name, and no apparatus note contains "verified" — so removing the
        //    apparatus column weakens none of them. Said explicitly because a
        //    whole-screen scan otherwise leaves that unanswerable by reading.
        var shown = spec.visibleText(screen).toLowerCase()
        verify(shown.indexOf("verified") < 0 || shown.indexOf("unverified") >= 0
               || shown.indexOf("not verified") >= 0,
               "the word 'verified' may not stand on this screen without the "
               + "remainder being named alongside it: " + shown)
        verify(shown.indexOf("this stoa is verified") < 0
               && shown.indexOf("verified stoa") < 0
               && shown.indexOf("verified record") < 0,
               "nothing may present the Stoa or its record as verified outright: " + shown)
        screen.destroy()
    }

    // ---- scanned over the screen the requirement names ---------------------
    //
    // R13 forbids a moderator or per-Stoa-identity claim on THESE SCREENS, and
    // its scenario (spec.md:611) names two: the list rendering a Stoa, and the
    // creation outcome rendering a newly created one. The only test on the
    // subject scanned the JOIN screen — a third screen the scenario does not
    // mention — so the requirement was pinned over the wrong corpus entirely.
    //
    // Measured by a reviewer: captioning the creation outcome `CREATED — THIS IS
    // ITS ADDRESS. You moderate this Stoa, and joining it generates you an
    // identity for it alone.` left **all 100 tests passing**, with the sentence
    // confirmed rendered. Both claims are forbidden, and the second is the one
    // the spec calls the only failure here that could actually harm someone.
    //
    // This is the corpus defect the file header documents, recurring in the
    // requirement the header calls the most harmful to get wrong. Hence the
    // corpus assertion below, before any absence: it is the lesson applied to
    // its own fix.
    function test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_identity() {
        var addr = "b02d5e77" + "a1".repeat(28)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Transport Notes"}],'
                        + '"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false),
            "create_stoa": '{"stoa":"' + addr + '","foundingTitle":"Transport Notes","policy":"open"}'
        })
        screen.createTitle = "Transport Notes"
        screen.create()
        compare(screen.createState, "created", "the creation outcome must be on screen")

        var body = spec.bodyText(screen)

        // **The corpus, asserted before the absences.** Both screens the
        // scenario names must actually be rendering: the list's row, and the
        // creation outcome. Without this the test would pass on a blank screen,
        // which is exactly how its sibling proved nothing for so long.
        verify(body.indexOf("CREATED") >= 0,
               "the creation outcome must be in the scanned corpus, or the "
               + "absences below prove nothing: " + body)
        verify(body.indexOf("Transport Notes") >= 0,
               "and the list's row with it: " + body)

        var shown = body.toLowerCase()

        // The same two predicates the join screen uses, over the screens R13's
        // scenario actually names. Deliberately the same words: one requirement,
        // one vocabulary, so a reviewer comparing the two reads them as the pair
        // they are.
        verify(!/\bidentity\b/.test(shown),
               "neither the list nor the creation outcome may raise identity — "
               + "one key signs in every Stoa in this release, so a per-Stoa "
               + "identity promise offers an unlinkability property the "
               + "software does not have: " + body)
        verify(shown.indexOf("you moderate") < 0,
               "nor may either claim the user moderates a Stoa — the creator key "
               + "in the record is never re-checked against this peer's current "
               + "signing key, so it is a question these screens cannot answer: "
               + body)
        // A created Stoa is the tempting case for the moderation claim, since
        // this peer did create it. Refused in the other voice too.
        verify(!/\byou (are|become|are now)\b[^.]{0,40}\bmoderator\b/.test(shown),
               "including of a Stoa this peer just created: " + body)
        screen.destroy()
    }

    // R11's central prohibition — "it offers no field for a creator key or an
    // identity to create under" — was pinned by nothing. The only test on the
    // create affordance asserts the button exists and is enabled, which is the
    // scenario's FIRST clause and never its second.
    //
    // Measured by a reviewer: a visible `TextInput` labelled `CREATOR KEY` beside
    // the title field left all 100 tests passing, confirmed rendered.
    //
    // The stake is why this is not a nicety: the creator key is fixed inside the
    // address preimage forever, so a field for one mints a Stoa nobody can
    // moderate at an address that cannot be un-minted.
    function test_the_create_affordance_offers_exactly_one_field_and_it_is_not_a_key() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false)
        })

        // **Counted, not searched by name.** A test looking for an element named
        // `creatorKeyField` pins one spelling of the defect and nothing else;
        // the next one would be called `identityPicker`. What the requirement
        // bounds is how many things the user can type into at all.
        //
        // The reviewer's warning applies here and shaped this: the shipped title
        // field is a bare `TextInput` with no `placeholderText`, so a probe keyed
        // on that property finds nothing even on a clean tree. These three
        // properties are what an editable text input exposes and a `Text` does
        // not — verified by walking the shipped screen.
        var inputs = spec.editableInputs(screen)

        // Two: the create title, and the paste field. Hardcoded rather than
        // `<= 2`, because zero is a different defect — a create affordance with
        // no title field at all — and an inequality would pass on it.
        compare(inputs.length, 2,
                "exactly two things on this screen accept typing: the Stoa title "
                + "and the paste field. A third is a field for something the "
                + "create call does not take — and the only candidates are a "
                + "creator key or an identity, neither of which is a parameter "
                + "and neither of which can be: " + spec.describeInputs(inputs))

        // And nothing captions a field as a key or an identity, which catches a
        // key field built from something other than a TextInput.
        var shown = spec.visibleText(screen).toLowerCase()
        verify(!/\b(creator key|signing key|private key|secret key)\b/.test(shown),
               "no field may be captioned as a key: " + shown)
        verify(!/\b(identity|identities)\b[^.]{0,30}\b(to create|create under|choose|select|pick)\b/.test(shown),
               "nor may an identity be offered to create under: " + shown)

        // The affordance is still THERE — asserted alongside, so a screen that
        // rendered no create row at all could not satisfy the absences above.
        compare(spec.visibleNamed(screen, "createStoaButton").length, 1,
                "the create affordance itself must be present")
        screen.destroy()
    }

    // R1: "A second abbreviation MUST NOT be written." The existing row test
    // says in its own comment that it asserts "the ADDRESS is there in some
    // form, not that a second elision was written" — so the prose requirement
    // was knowingly unpinned, and a reviewer confirmed it: replacing the row's
    // `AddressLabel` with a hand-rolled `head8 + "…" + tail6` left all 100 tests
    // passing. It passes because both the right and the wrong implementation
    // render the head `7f3a91c4`, which is the only thing asserted — the repo's
    // defect family exactly, a question both implementations answer alike.
    //
    // The requirement says why it matters: "an elision that keeps only a head
    // and a tail is the shape vanity-address generators are built to defeat".
    function test_a_row_abbreviates_through_the_one_component_and_keeps_a_middle_group() {
        // Every 8-character window of this address is distinct, so "the middle
        // group came from the middle" is a real check rather than one satisfied
        // by a repeated byte matching an earlier offset by accident.
        var addr = "7f3a91c4" + "0123456789abcdef" + "fedcba9876543210"
                 + "13579bdf02468ace" + "cafebabe"
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Transport Notes"}],'
                        + '"page":0,"hasMore":false}'
        })
        compare(screen.visibleRows.length, 1, "the fixture must put a row on screen")

        // The row's address element, found by carrying the address rather than
        // by a name the mutation could keep.
        var labels = spec.addressElementsFor(screen, addr)
        compare(labels.length, 1,
                "exactly one element renders this row's address: "
                + spec.visibleText(screen))
        var label = labels[0]

        // **Two assertions, and they fail for different reasons.**
        //
        // (a) The rendered form keeps a MIDDLE group. This is the requirement
        //     itself rather than a proxy for it: head-8 middle-8 tail-6 renders
        //     three groups, a head-and-tail elision renders two. A correct
        //     reimplementation would satisfy this and a weak one cannot,
        //     whatever component it lives in.
        var parts = label.text.split("…")
        compare(parts.length, 3,
                "the abbreviation must render THREE groups separated by two "
                + "ellipses — head, middle, tail. A head-and-tail form is the "
                + "shape vanity generators are built to defeat. Got: " + label.text)
        verify(parts[1].length > 0, "the middle group must not be empty")
        // The middle group is drawn from the middle of the address, so it is
        // neither the head nor the tail — asserted as a relation, because
        // hardcoding the offset would re-implement AddressLabel's arithmetic
        // here and agree with it by construction.
        verify(addr.indexOf(parts[1]) > parts[0].length,
               "the middle group must come from the middle of the address, "
               + "not be a second copy of the head: " + label.text)

        // (b) And it is rendered through the component that OWNS that form.
        //     `full` is a property AddressLabel declares and a bare Text does
        //     not, so this is the "no second implementation" clause: a screen
        //     that grew its own elision fails here even if it happened to keep
        //     three groups.
        verify(label.full !== undefined,
               "the row's address must render through AddressLabel, which owns "
               + "the 8-8-6 form — a second implementation is how one screen "
               + "quietly acquires the weaker one")
        compare(label.full, false, "and a row abbreviates rather than showing it whole")
        screen.destroy()
    }

    // ---- every call goes through the one wrapper --------------------------

    function test_each_core_method_is_named_once_and_reached_through_the_wrapper() {
        // The wrapper is what normalises every reply into one of two shapes, and
        // it is the only thing that makes these screens testable at all — a
        // screen calling the host directly is a screen whose failure paths
        // cannot be exercised anywhere. If these calls did not go through
        // Core.bridge, every fake above would have been ignored and every test
        // in this file would be asserting against a real host that is not there.
        Core.bridge = bridgeFor({ "list_stoas": '{"items":[],"page":0,"hasMore":false}' })
        var bridge = Core.bridge
        var screen = listComponent.createObject(null, {})
        compare(spec.callsTo(bridge, "list_stoas"), 1)
        screen.destroy()

        Core.bridge = bridgeFor({ "create_stoa": '{"stoa":"aa","foundingTitle":"t"}' })
        bridge = Core.bridge
        var out = Core.createStoa("t")
        compare(spec.callsTo(bridge, "create_stoa"), 1)
        compare(out.ok, true)

        Core.bridge = bridgeFor({ "join_stoa": '{"stoa":"aa","foundingTitle":"t"}' })
        bridge = Core.bridge
        Core.joinStoa("aa", "00ff")
        compare(spec.callsTo(bridge, "join_stoa"), 1)
        verify(String(spec.lastArgsTo(bridge, "join_stoa")).indexOf('"genesis":"00ff"') >= 0,
               "both halves must reach the core")

        Core.bridge = bridgeFor({
            "create_identity": '{"publicKey":"aa","encrypted":false,"wasNew":true}'
        })
        bridge = Core.bridge
        var minted = Core.createIdentity()
        compare(spec.callsTo(bridge, "create_identity"), 1)
        compare(minted.ok, true)

        Core.bridge = bridgeFor({ "get_master_key": '{"hasMasterKey":false}' })
        bridge = Core.bridge
        var asked = Core.getMasterKey()
        compare(spec.callsTo(bridge, "get_master_key"), 1)
        compare(asked.ok, true)
    }

    // ---- this machine's key: the three key states ---------------------------
    //
    // `stoa-navigation-view`'s key-state requirements, from `home-screen-key-
    // states`. The key state is decided by `get_master_key`'s reply, so every
    // fixture below names one — and the fakes answer DIFFERENTLY per test
    // (a key, no key, a failure, a malformed reply), because a fake answering
    // the same for every input cannot tell "asked" from "never asked".

    /// A key hex of the right length, so `AddressLabel` renders it as it would a
    /// real one rather than falling into a short-string path.
    function aKeyHex() {
        return "7f3c19d84ba2e05c6178fd4390ab2ec5518d7a6f30b94c2e81df05a7c63e14b2"
    }

    /// A second key whose every 8-character window differs from the first's,
    /// so a screen rendering the wrong one is caught by its head group alone.
    function anotherKeyHex() {
        return "c4a1e07b93d25f6810bb7c3e5d49f2a6071e8c35b9d4a26f0e17c83b5a92d6e4"
    }

    function heldKeyReply(key, encrypted) {
        return JSON.stringify({ hasMasterKey: true, publicKey: key, encrypted: encrypted })
    }

    function noKeyReply() {
        return '{"hasMasterKey":false}'
    }

    // The key block's explanation, verbatim from copy.json.
    readonly property string keyExplanationCopy:
        "A Stoa records its creator's key, so this machine needs one before it "
        + "can create or post. Making it writes a key here and tells nobody. The "
        + "same key signs in every Stoa you hold."

    readonly property string unencryptedCopy:
        "Stored unencrypted on this machine. Anyone who can read the file can post as you."

    function test_a_reply_stating_no_key_puts_the_screen_in_the_no_key_state() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        })

        compare(screen.machineKey.state, "none")
        compare(spec.visibleNamed(screen, "createKeyButton").length, 1,
                "the key block's action is the task on this screen")
        compare(spec.visibleNamed(screen, "keyExplanation").length, 1)
        screen.destroy()
    }

    function test_a_reply_naming_a_held_key_puts_the_screen_in_the_key_held_state() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false)
        })

        compare(screen.machineKey.state, "held")
        compare(screen.machineKey.publicKey, aKeyHex())
        compare(spec.visibleNamed(screen, "keyLine").length, 1)
        screen.destroy()
    }

    function test_a_failed_query_is_the_could_not_be_read_state_carrying_the_cores_words() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": '{"error":"keystore permissions are too open (mode 0644)"}'
        })

        compare(screen.machineKey.state, "unreadable")
        var reasons = spec.visibleNamed(screen, "keyUnreadableReason")
        compare(reasons.length, 1)
        compare(reasons[0].text, "keystore permissions are too open (mode 0644)",
                "the core's message, unreworded")
        compare(spec.namedAnywhere(screen, "createKeyButton").length, 0,
                "a key that may exist is not offered to be made")
        compare(spec.namedAnywhere(screen, "createStoaButton").length, 0,
                "nor is creation, which needs a key that could not be read")
        compare(spec.namedAnywhere(screen, "createTitleField").length, 0)
        compare(spec.namedAnywhere(screen, "keyLine").length, 0, "and no key line")
        var shown = spec.visibleText(screen)
        verify(shown.indexOf(spec.keyExplanationCopy) < 0,
               "the no-key explanation must not be rendered: " + shown)
        verify(!/\bno key\b|\bholds? none\b|\bhas no key\b/i.test(shown),
               "nothing may state that no key is held: " + shown)
        screen.destroy()
    }

    // "Pasting stays available when the key state could not be read"
    // (stoa-navigation-view, spec.md lines 416-419). Asserts presence AND that
    // the action still reaches the preview signal — the shape this file's own
    // defect-family note asks for, not just an element existing in the tree.
    function test_pasting_stays_available_when_the_key_state_could_not_be_read() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": '{"error":"keystore permissions are too open (mode 0644)"}'
        })
        compare(screen.machineKey.state, "unreadable", "the fixture is the could-not-be-read state")

        compare(spec.visibleNamed(screen, "pasteSection").length, 1)
        compare(spec.visibleNamed(screen, "pasteField").length, 1)
        compare(spec.visibleNamed(screen, "pasteButton").length, 1)

        var asked = []
        screen.previewRequested.connect(function (stoa, genesis) { asked.push(stoa) })
        var stoa = "ab".repeat(32)
        screen.pasted = DStoaReference.shareText(stoa, "00ff")
        spec.visibleNamed(screen, "pasteButton")[0].clicked()

        compare(asked.length, 1, "the action still reaches the preview signal")
        compare(asked[0], stoa)
        screen.destroy()
    }

    function test_a_reply_claiming_a_key_without_naming_one_is_not_the_key_held_state() {
        var replies = ['{"hasMasterKey":true}', '{"hasMasterKey":true,"publicKey":""}',
                       '{"hasMasterKey":true,"publicKey":7}']
        for (var i = 0; i < replies.length; i++) {
            var screen = makeList({
                "list_stoas": '{"items":[],"page":0,"hasMore":false}',
                "get_master_key": replies[i]
            })
            compare(screen.machineKey.state, "unreadable", "for " + replies[i])
            compare(spec.namedAnywhere(screen, "keyLine").length, 0, "for " + replies[i])
            compare(spec.namedAnywhere(screen, "createStoaButton").length, 0, "for " + replies[i])
            verify(spec.visibleNamed(screen, "keyUnreadableReason")[0].text !== "",
                   "the failure must name what was wrong with the reply")
            screen.destroy()
        }
    }

    function test_a_reply_stating_neither_outcome_is_not_the_no_key_state() {
        // `=== false` and nothing looser. Each of these is falsy or absent, and
        // each would put a `!hasMasterKey` screen in the no-key state, drawing
        // "Create this machine's key" for a peer the core said nothing about.
        var replies = ['{}', '{"hasMasterKey":null}', '{"hasMasterKey":0}',
                       '{"hasMasterKey":"false"}', '{"hasMasterKey":"true","publicKey":"aa"}']
        for (var i = 0; i < replies.length; i++) {
            var screen = makeList({
                "list_stoas": '{"items":[],"page":0,"hasMore":false}',
                "get_master_key": replies[i]
            })
            compare(screen.machineKey.state, "unreadable", "for " + replies[i])
            compare(spec.namedAnywhere(screen, "createKeyButton").length, 0, "for " + replies[i])
            screen.destroy()
        }
    }

    // ---- could not be read: what failed, and reading the key again ----------
    //
    // "A key state that could not be read is told apart from both others".
    // Every fixture here changes the query's answer between the arrival and the
    // press, because a fake answering the same both times cannot tell "asked
    // again" from "kept the first answer".

    readonly property string unreadableStatement:
        "Whether this machine holds a key could not be read."
    readonly property string readAgainLabel: "Try reading the key again"

    /// Every element carrying `label` that can be acted on, visible or not.
    ///
    /// Keyed on the label AND on having a `clicked` signal, so the thing found
    /// is the action the user reads rather than an element named for it: a
    /// button whose objectName survived a relabelling would not be found, and
    /// a caption carrying the words without an action behind it is not one.
    /// Walks invisible elements too, because the spec's "not rendered" is met
    /// here by the action not existing, and a hidden one would still be found.
    function actionsLabelled(item, label) {
        var found = []
        function walk(node) {
            if (!node)
                return
            if (node.text === label && typeof node.clicked === "function")
                found.push(node)
            var kids = node.children
            if (kids !== undefined)
                for (var i = 0; i < kids.length; i++)
                    walk(kids[i])
        }
        walk(item)
        return found
    }

    function test_the_could_not_be_read_state_states_what_failed_above_the_reason() {
        // Both routes into the state: the failure shape, and a success stating
        // neither boolean. The statement is the same for both.
        var replies = ['{"error":"keystore permissions are too open (mode 0644)"}',
                       '{"hasMasterKey":"maybe"}']
        for (var i = 0; i < replies.length; i++) {
            var screen = makeList({
                "list_stoas": '{"items":[],"page":0,"hasMore":false}',
                "get_master_key": replies[i]
            })
            var statements = spec.visibleNamed(screen, "keyUnreadableStatement")
            compare(statements.length, 1, "for " + replies[i])
            compare(statements[0].text, spec.unreadableStatement, "verbatim, for " + replies[i])
            var reason = spec.visibleNamed(screen, "keyUnreadableReason")[0]
            verify(reason.text !== "" && reason.text !== spec.unreadableStatement,
                   "the reason is a second element, not the statement: " + reason.text)
            verify(statements[0].mapToItem(screen, 0, 0).y < reason.mapToItem(screen, 0, 0).y,
                   "the statement sits above the reason, for " + replies[i])
            screen.destroy()
        }
    }

    function test_reading_the_key_again_asks_the_query_and_mints_nothing() {
        var replies = {
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": '{"error":"keystore could not be read"}',
            "create_identity": '{"publicKey":"' + aKeyHex() + '","encrypted":false,"wasNew":true}'
        }
        var screen = makeList(replies)
        var bridge = Core.bridge
        var before = spec.callsTo(bridge, "get_master_key")

        var actions = spec.actionsLabelled(screen, spec.readAgainLabel)
        compare(actions.length, 1, "the could-not-be-read state offers the action")
        actions[0].clicked()

        compare(spec.callsTo(bridge, "get_master_key"), before + 1, "asked exactly once more")
        compare(spec.callsTo(bridge, "create_identity"), 0,
                "and nothing minted, although the fake would have answered a mint")
        screen.destroy()
    }

    function test_reading_the_key_again_reaches_the_key_held_state_once_the_key_can_be_read() {
        var earlier = "keystore permissions are too open (mode 0644)"
        var replies = {
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": '{"error":"' + earlier + '"}'
        }
        var screen = makeList(replies)
        compare(screen.machineKey.state, "unreadable", "the fixture starts unreadable")

        replies["get_master_key"] = spec.heldKeyReply(anotherKeyHex(), true)
        spec.actionsLabelled(screen, spec.readAgainLabel)[0].clicked()

        compare(screen.machineKey.state, "held")
        compare(spec.visibleNamed(screen, "keyLineAddress")[0].address, anotherKeyHex(),
                "naming the key the second answer named")
        var shown = spec.visibleText(screen)
        verify(shown.indexOf(spec.unreadableStatement) < 0, shown)
        verify(shown.indexOf(earlier) < 0, "nor the earlier reason: " + shown)
        screen.destroy()
    }

    function test_reading_the_key_again_reaches_the_no_key_state_when_none_is_held() {
        // The requirement names this outcome alongside the other two ("a reply
        // stating no key is held puts it in its no-key state"); it has no
        // scenario of its own.
        var replies = {
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": '{"error":"keystore could not be read"}'
        }
        var screen = makeList(replies)

        replies["get_master_key"] = spec.noKeyReply()
        spec.actionsLabelled(screen, spec.readAgainLabel)[0].clicked()

        compare(screen.machineKey.state, "none")
        compare(spec.visibleNamed(screen, "createKeyButton").length, 1)
        verify(spec.visibleText(screen).indexOf(spec.unreadableStatement) < 0)
        screen.destroy()
    }

    function test_reading_the_key_again_that_fails_again_renders_the_new_reason() {
        var replies = {
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": '{"error":"keystore permissions are too open (mode 0644)"}'
        }
        var screen = makeList(replies)

        replies["get_master_key"] = '{"error":"the keystore is encrypted and no passphrase is set"}'
        spec.actionsLabelled(screen, spec.readAgainLabel)[0].clicked()

        compare(screen.machineKey.state, "unreadable")
        var reasons = spec.visibleNamed(screen, "keyUnreadableReason")
        compare(reasons.length, 1)
        compare(reasons[0].text, "the keystore is encrypted and no passphrase is set",
                "the new message, unreworded")
        verify(spec.visibleText(screen).indexOf("permissions are too open") < 0,
               "and the earlier one is gone")
        compare(spec.actionsLabelled(screen, spec.readAgainLabel).length, 1,
                "the action is still offered, so a second fix can be read too")
        screen.destroy()
    }

    function test_the_read_again_action_belongs_to_the_could_not_be_read_state_alone() {
        var replies = [spec.noKeyReply(), spec.heldKeyReply(aKeyHex(), false)]
        for (var i = 0; i < replies.length; i++) {
            var screen = makeList({
                "list_stoas": '{"items":[],"page":0,"hasMore":false}',
                "get_master_key": replies[i]
            })
            verify(screen.machineKey.state !== "unreadable", "the fixture is a readable state")
            compare(spec.actionsLabelled(screen, spec.readAgainLabel).length, 0,
                    "for " + replies[i])
            verify(spec.visibleText(screen).indexOf(spec.readAgainLabel) < 0,
                   "nor the words, for " + replies[i])
            screen.destroy()
        }
    }

    function test_arriving_asks_the_query_once_and_mints_nothing() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        })
        var bridge = Core.bridge

        compare(spec.callsTo(bridge, "get_master_key"), 1)
        compare(spec.callsTo(bridge, "create_identity"), 0,
                "nothing may be minted before the user presses the button")
        verify(String(spec.lastArgsTo(bridge, "get_master_key")).indexOf("stoa") < 0,
               "the query names no Stoa — a fresh install has none")
        screen.destroy()
    }

    function test_the_key_state_is_asked_again_on_each_showing() {
        // The fake's answer CHANGES between showings, which is what makes this
        // able to fail: a screen that kept its first answer stays in "none".
        var replies = {
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        }
        var screen = makeShownList(replies)
        var bridge = Core.bridge
        compare(screen.machineKey.state, "none")

        screen.visible = false
        replies["get_master_key"] = spec.heldKeyReply(aKeyHex(), false)
        screen.visible = true

        compare(spec.callsTo(bridge, "get_master_key"), 2, "once for each showing")
        compare(screen.machineKey.state, "held", "the earlier answer notwithstanding")
        compare(spec.namedAnywhere(screen, "createKeyButton").length, 0)
        screen.destroy()
    }

    function test_returning_home_from_a_feed_asks_the_key_state_again() {
        // The re-read through the real navigator: the key changes while a feed
        // is open — in this release from outside the view, since the per-Stoa
        // keep that used to write it is not mounted (design.md, Decision 8) —
        // so the list the user returns to must not still be offering to make
        // one.
        var replies = {
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        }
        Core.bridge = bridgeFor(replies)
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        var list = spec.namedAnywhere(view, "stoaList")[0]
        compare(list.machineKey.state, "none")

        view.open("ab".repeat(32), "Agora", "00ff")
        replies["get_master_key"] = spec.heldKeyReply(aKeyHex(), false)
        view.closeFeed()

        compare(spec.callsTo(bridge, "get_master_key"), 2, "asked on arrival and on return")
        compare(list.machineKey.state, "held")
        view.destroy()
    }

    function test_hiding_the_screen_asks_nothing() {
        var screen = makeShownList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        })
        var bridge = Core.bridge
        screen.visible = false
        compare(spec.callsTo(bridge, "get_master_key"), 1,
                "a hide is not a showing")
        screen.destroy()
    }

    function test_the_key_state_does_not_follow_the_listing() {
        // Each key state against a FAILED listing. The listing and the key are
        // answered by different calls, and the one that failed says nothing
        // about the other.
        var cases = [
            { reply: spec.noKeyReply(), state: "none", present: "createKeyButton" },
            { reply: spec.heldKeyReply(aKeyHex(), false), state: "held", present: "createStoaButton" }
        ]
        for (var i = 0; i < cases.length; i++) {
            var screen = makeList({
                "list_stoas": '{"error":"the membership store is locked"}',
                "get_master_key": cases[i].reply
            })
            compare(screen.readState, "failed", "the fixture must be in the failed state")
            compare(screen.machineKey.state, cases[i].state)
            compare(spec.visibleNamed(screen, cases[i].present).length, 1,
                    cases[i].state + ": a failed listing changes nothing about the key")
            screen.destroy()
        }
    }

    function test_no_other_identity_probe_is_made_on_arrival() {
        // `who_am_i` and `get_capabilities` both take a Stoa, and there is no
        // Stoa on this screen. The key question is `get_master_key`'s.
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        })
        var bridge = Core.bridge

        compare(spec.callsTo(bridge, "who_am_i"), 0)
        compare(spec.callsTo(bridge, "get_capabilities"), 0)
        screen.destroy()
    }

    // ---- no key held (0A) --------------------------------------------------

    function test_the_no_key_state_renders_the_key_block_above_the_paste_section() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        })

        var block = spec.visibleNamed(screen, "keyBlock")
        var paste = spec.visibleNamed(screen, "pasteSection")
        compare(block.length, 1)
        compare(paste.length, 1)
        compare(spec.visibleNamed(screen, "pasteField").length, 1)
        compare(spec.visibleNamed(screen, "pasteButton").length, 1)
        verify(block[0].mapToItem(screen, 0, 0).y < paste[0].mapToItem(screen, 0, 0).y,
               "the key block sits above the paste section")
        screen.destroy()
    }

    function test_a_reference_can_be_previewed_with_no_key_held() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        })
        var bridge = Core.bridge
        var asked = []
        screen.previewRequested.connect(function (stoa, genesis) { asked.push(stoa) })

        var stoa = "ab".repeat(32)
        screen.pasted = DStoaReference.shareText(stoa, "00ff")
        spec.visibleNamed(screen, "pasteButton")[0].clicked()

        compare(asked.length, 1, "a preview is requested")
        compare(asked[0], stoa)
        compare(spec.callsTo(bridge, "join_stoa"), 0, "and nothing is joined")
        screen.destroy()
    }

    function test_creating_the_key_calls_the_mint_once_and_names_no_stoa() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply(),
            "create_identity": '{"publicKey":"' + aKeyHex() + '","encrypted":false,"wasNew":true}'
        })
        var bridge = Core.bridge

        spec.visibleNamed(screen, "createKeyButton")[0].clicked()

        compare(spec.callsTo(bridge, "create_identity"), 1)
        var sent = String(spec.lastArgsTo(bridge, "create_identity"))
        verify(sent.indexOf("stoa") < 0,
               "the mint must name no Stoa — a fresh install has none: " + sent)
        screen.destroy()
    }

    function test_a_successful_mint_moves_the_screen_to_the_key_held_state() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply(),
            "create_identity": '{"publicKey":"' + anotherKeyHex() + '","encrypted":false,"wasNew":true}'
        })
        spec.visibleNamed(screen, "createKeyButton")[0].clicked()

        compare(screen.machineKey.state, "held")
        compare(spec.namedAnywhere(screen, "keyBlock").length, 0,
                "the key block is no longer in the element tree")
        compare(spec.visibleNamed(screen, "createStoaButton").length, 1,
                "and the create affordance is present")
        compare(spec.visibleNamed(screen, "keyLineAddress")[0].address, anotherKeyHex(),
                "naming the key the MINT returned")
        screen.destroy()
    }

    function test_a_refused_mint_renders_the_cores_reason_and_stays_in_the_no_key_state() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply(),
            "create_identity": '{"error":"the keystore directory is not writable"}'
        })
        spec.visibleNamed(screen, "createKeyButton")[0].clicked()

        compare(screen.machineKey.state, "none")
        var shown = spec.visibleText(screen)
        verify(shown.indexOf("No key was created.") >= 0, "says no key was created: " + shown)
        compare(spec.visibleNamed(screen, "mintFailureText")[0].text,
                "the keystore directory is not writable", "the core's reason, unreworded")
        compare(spec.namedAnywhere(screen, "keyLine").length, 0, "no key is rendered as held")
        screen.destroy()
    }

    function test_a_mint_success_naming_no_key_is_not_a_key() {
        var replies = ['{"encrypted":false,"wasNew":true}',
                       '{"publicKey":"","encrypted":false,"wasNew":true}']
        for (var i = 0; i < replies.length; i++) {
            var screen = makeList({
                "list_stoas": '{"items":[],"page":0,"hasMore":false}',
                "get_master_key": spec.noKeyReply(),
                "create_identity": replies[i]
            })
            spec.visibleNamed(screen, "createKeyButton")[0].clicked()

            verify(screen.machineKey.state !== "held", "for " + replies[i])
            compare(spec.namedAnywhere(screen, "keyLine").length, 0, "for " + replies[i])
            verify(spec.visibleNamed(screen, "mintFailureText")[0].text !== "",
                   "a failure naming what was wrong with the reply")
            screen.destroy()
        }
    }

    // "A failure rendered after a press belongs to the showing in which the
    // press was made", and its scenario "A refused mint is not rendered on a
    // later showing". The query answers no key both times, so the refusal is
    // gone because the showing changed, not because the key state did.
    function test_a_mint_failure_does_not_outlive_the_showing_it_happened_in() {
        var screen = makeShownList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply(),
            "create_identity": '{"error":"the keystore directory is not writable"}'
        })
        spec.visibleNamed(screen, "createKeyButton")[0].clicked()
        compare(spec.visibleNamed(screen, "mintFailureText")[0].text,
                "the keystore directory is not writable",
                "the fixture must render the refusal before the re-showing")

        screen.visible = false
        screen.visible = true

        compare(screen.machineKey.state, "none")
        var shown = spec.visibleText(screen)
        verify(shown.indexOf("No key was created.") < 0, shown)
        verify(shown.indexOf("the keystore directory is not writable") < 0, shown)
        screen.destroy()
    }

    // ---- key held (0B) -----------------------------------------------------

    function test_the_key_block_is_not_instantiated_when_a_key_is_held() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false)
        })

        compare(spec.namedAnywhere(screen, "createKeyButton").length, 0,
                "no create-key action, visible or invisible")
        compare(spec.namedAnywhere(screen, "keyExplanation").length, 0,
                "no element carrying the explanation")
        screen.destroy()
    }

    function test_creating_a_stoa_sits_above_pasting_and_the_key_line_below_both() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false)
        })

        function y(name) { return spec.visibleNamed(screen, name)[0].mapToItem(screen, 0, 0).y }
        verify(y("createBlock") < y("pasteSection"), "create above paste")
        verify(y("pasteSection") < y("keyLine"), "the key line below the paste section")
        verify(y("createBlock") < y("keyLine"), "and below the create affordance")
        screen.destroy()
    }

    function test_the_key_line_abbreviates_the_reported_key_through_the_one_component() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(anotherKeyHex(), false)
        })

        var labels = spec.addressElementsFor(screen, anotherKeyHex())
        compare(labels.length, 1, "exactly one element renders the key")
        verify(labels[0].full !== undefined, "rendered by AddressLabel")
        compare(labels[0].full, false, "abbreviated, never in full")
        compare(labels[0].address, anotherKeyHex(), "carrying the reported key")
        compare(labels[0].text.split("…").length, 3, "head, middle and tail: " + labels[0].text)
        screen.destroy()
    }

    function test_an_unprotected_key_carries_the_warning_in_the_accent_colour() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false)
        })

        var warnings = spec.visibleNamed(screen, "unencryptedWarning")
        compare(warnings.length, 1)
        compare(warnings[0].text, spec.unencryptedCopy)
        verify(Qt.colorEqual(warnings[0].color, DTheme.accent), "in the accent colour")
        screen.destroy()
    }

    function test_a_protected_key_carries_no_unencrypted_warning() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), true)
        })

        compare(spec.visibleNamed(screen, "unencryptedWarning").length, 0)
        screen.destroy()
    }

    function test_an_omitted_protection_field_makes_no_claim_either_way() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": '{"hasMasterKey":true,"publicKey":"' + aKeyHex() + '"}'
        })

        compare(screen.machineKey.state, "held")
        compare(spec.visibleNamed(screen, "unencryptedWarning").length, 0)
        var shown = spec.visibleText(screen).toLowerCase()
        verify(!/\b(encrypted|protected)\b/.test(shown),
               "nothing may claim the key is protected either: " + shown)
        screen.destroy()
    }

    function test_the_key_held_state_renders_the_same_however_it_was_reached() {
        function heldText(replies, press) {
            var screen = makeList(replies)
            if (press)
                spec.visibleNamed(screen, "createKeyButton")[0].clicked()
            compare(screen.machineKey.state, "held")
            var text = spec.visibleText(screen)
            screen.destroy()
            return text
        }
        var list = '{"items":[],"page":0,"hasMore":false}'
        var fromQuery = heldText({ "list_stoas": list,
                                   "get_master_key": spec.heldKeyReply(aKeyHex(), false) }, false)
        var fromNewMint = heldText({ "list_stoas": list, "get_master_key": spec.noKeyReply(),
                                     "create_identity": '{"publicKey":"' + aKeyHex()
                                                      + '","encrypted":false,"wasNew":true}' }, true)
        var fromOldMint = heldText({ "list_stoas": list, "get_master_key": spec.noKeyReply(),
                                     "create_identity": '{"publicKey":"' + aKeyHex()
                                                      + '","encrypted":false,"wasNew":false}' }, true)

        compare(fromNewMint, fromQuery)
        compare(fromOldMint, fromQuery)
        verify(!/already had a key|nothing was replaced/i.test(fromOldMint),
               "a mint that found a key says nothing about it: " + fromOldMint)
    }

    // ---- the copy ----------------------------------------------------------

    function test_the_no_key_state_renders_its_copy_verbatim() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        })
        var shown = spec.visibleText(screen).split("\n")
        var expected = ["Stoas you joined", "NO DIRECTORY EXISTS · JOIN BY ADDRESS",
                        "THIS MACHINE'S KEY", spec.keyExplanationCopy,
                        "Create this machine's key",
                        "PASTE A STOA REFERENCE — THE ADDRESS AND ITS FOUNDING RECORD",
                        "Look at it first"]
        for (var i = 0; i < expected.length; i++)
            verify(shown.indexOf(expected[i]) >= 0, "missing verbatim: " + expected[i])
        screen.destroy()
    }

    function test_the_key_held_state_renders_its_copy_verbatim() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false)
        })
        var shown = spec.visibleText(screen).split("\n")
        var expected = ["Stoas you joined", "NO DIRECTORY EXISTS · JOIN BY ADDRESS",
                        "CREATE A STOA", "Create it",
                        "PASTE A STOA REFERENCE — THE ADDRESS AND ITS FOUNDING RECORD",
                        "Look at it first", "THIS MACHINE'S KEY", spec.unencryptedCopy]
        for (var i = 0; i < expected.length; i++)
            verify(shown.indexOf(expected[i]) >= 0, "missing verbatim: " + expected[i])
        screen.destroy()
    }

    function test_a_listed_row_renders_its_actions_verbatim() {
        var addr = "b02d5e77" + "c3".repeat(28)
        var screen = makeList({
            "list_stoas": '{"items":[{"stoa":"' + addr + '","foundingTitle":"Transport Notes",'
                        + '"genesis":"00ff"}],"page":0,"hasMore":false}',
            "get_master_key": spec.noKeyReply()
        })
        var shown = spec.visibleText(screen).split("\n")
        verify(shown.indexOf("Copy a shareable reference") >= 0)
        verify(shown.indexOf("Open") >= 0)
        screen.destroy()
    }

    function test_the_title_placeholder_shows_only_while_the_field_is_empty() {
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false)
        })

        compare(spec.visibleNamed(screen, "createTitlePlaceholder").length, 1)
        compare(spec.visibleNamed(screen, "createTitlePlaceholder")[0].text, "Title of the new Stoa")

        spec.namedAnywhere(screen, "createTitleField")[0].text = "Agora"
        compare(spec.visibleNamed(screen, "createTitlePlaceholder").length, 0,
                "the placeholder goes once text is entered")
        screen.destroy()
    }

    function test_the_placeholder_is_never_submitted_as_a_title() {
        // The empty title is blank, and `stoa-membership` refuses a blank title,
        // so the core answers with the error shape. A fixture answering success
        // here would encode a reply the core may not give.
        var screen = makeList({
            "list_stoas": '{"items":[],"page":0,"hasMore":false}',
            "get_master_key": spec.heldKeyReply(aKeyHex(), false),
            "create_stoa": '{"error":"title: title is blank"}'
        })
        var bridge = Core.bridge
        spec.visibleNamed(screen, "createStoaButton")[0].clicked()

        var sent = String(spec.lastArgsTo(bridge, "create_stoa"))
        verify(sent.indexOf('"title":""') >= 0, "the empty string was sent: " + sent)
        verify(sent.indexOf("Title of the new Stoa") < 0, "not the placeholder: " + sent)
        compare(screen.created, null, "no Stoa may be reported as created")
        compare(spec.visibleNamed(screen, "createdAddress").length, 0,
                "and no address is rendered as a Stoa just created")
        screen.destroy()
    }

    function test_none_of_the_key_states_claim_identity() {
        // **The key blocks must not say "identity" to do their job.** This
        // screen is banned from that word by
        // `test_neither_the_list_nor_the_creation_outcome_claims_moderation_or_
        // identity`: one key signs in every Stoa in this release, so raising
        // identity here offers an unlinkability property the software lacks.
        // Asserted over all three key states, because that sibling drives only
        // creation, so the copy these states render is a corpus it never scans.
        var replies = [spec.noKeyReply(), spec.heldKeyReply(aKeyHex(), false),
                       '{"error":"keystore could not be read"}']
        for (var i = 0; i < replies.length; i++) {
            var screen = makeList({
                "list_stoas": '{"items":[],"page":0,"hasMore":false}',
                "get_master_key": replies[i]
            })
            var shown = spec.visibleText(screen)
            verify(shown.indexOf("Look at it first") >= 0, "the corpus is the rendered screen")
            verify(!/\bidentity\b/i.test(shown),
                   "no key state may raise identity: " + shown)
            screen.destroy()
        }
    }

    function test_the_mint_reports_protection_from_its_own_reply() {
        // The mint's `encrypted` decides the warning when the key-held state was
        // reached through it — both directions, so a warning drawn
        // unconditionally fails the second.
        var cases = [{ encrypted: false, warnings: 1 }, { encrypted: true, warnings: 0 }]
        for (var i = 0; i < cases.length; i++) {
            var screen = makeList({
                "list_stoas": '{"items":[],"page":0,"hasMore":false}',
                "get_master_key": spec.noKeyReply(),
                "create_identity": JSON.stringify({ publicKey: aKeyHex(),
                                                    encrypted: cases[i].encrypted,
                                                    wasNew: true })
            })
            spec.visibleNamed(screen, "createKeyButton")[0].clicked()
            compare(spec.visibleNamed(screen, "unencryptedWarning").length, cases[i].warnings,
                    "encrypted: " + cases[i].encrypted)
            screen.destroy()
        }
    }
}
