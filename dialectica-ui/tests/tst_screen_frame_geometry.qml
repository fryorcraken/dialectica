import QtQuick
import QtQuick.Layouts
import QtTest
import "../src/qml"

// `ScreenFrame`'s two height bindings, and the interface text this change moved
// into `FeedScreen`'s body. Four review boxes on `piece/drop-apparatus` are
// closed here; each is named at the test that closes it.
//
// **"QtTest cannot measure geometry" is false, and this file is the counter-
// example.** No test in this suite observed geometry before it: every spec built
// its subject with `createObject(null, {})` — no parent, no size — so a screen
// 1000px wide and a screen 0px wide were indistinguishable to the whole gate.
// That is the blind spot that let a zero-height card ship green, and the change
// whose subject is measured layout is the right place to close it.
//
// **Two recipes, and which one a property needs is not a matter of taste.**
// Measured on Qt 6.10.3, both ingredients are required for a Repeater's rows to
// reach a layout's implicit height, and NEITHER alone is enough:
//
// | recipe | ScreenFrame's own bindings | FeedScreen implicitHeight, 0/1/5/30 rows |
// |---|---|---|
// | `createObject(null, {width, height})` | correct | 385 385 385 385 — FLAT |
// | parented to a shown TestCase, no render wait | — | 385 385 385 385 — FLAT |
// | parentless + `wait(50)` | — | 385 385 385 385 — FLAT |
// | parented to a shown TestCase + `waitForRendering` | correct | 385 530 954 3604 |
//
// A polish pass runs only for an item inside a rendered window, so a delegate
// built by a Repeater contributes nothing to `implicitHeight` until both hold.
// The light recipe is sufficient for `ScreenFrame` measured directly — it
// reproduces every figure `ScreenFrame.qml`'s comment block records — and it is
// used below wherever it suffices, because it is faster and needs no window.
// **A row-count assertion written with the light recipe would be flat at 385 in
// every case and would pass with the Repeater deleted**, which is precisely the
// "two explanations, one green" shape this repo keeps finding.
TestCase {
    id: spec
    name: "ScreenFrameGeometry"
    width: 1000
    height: 800
    when: windowShown

    property var savedBridge: undefined

    function init() { spec.savedBridge = Core.bridge }
    function cleanup() { Core.bridge = spec.savedBridge }

    // ---- subjects --------------------------------------------------------

    // A bare Rectangle has implicitHeight 0, so it can only be non-zero here by
    // being granted slack. That is what makes it the right probe for the
    // `body.height` binding and a poor one for anything else.
    Component {
        id: framedFiller
        ScreenFrame {
            Rectangle {
                objectName: "filler"
                Layout.fillWidth: true
                Layout.fillHeight: true
            }
        }
    }

    Component {
        id: framedRows
        ScreenFrame {
            Rectangle {
                objectName: "rowA"
                Layout.fillWidth: true
                Layout.preferredHeight: 40
            }
            Rectangle {
                objectName: "rowB"
                Layout.fillWidth: true
                Layout.preferredHeight: 40
            }
        }
    }

    Component { id: feedComponent; FeedScreen {} }

    function findByName(item, name) {
        if (item === null || item === undefined)
            return null
        if (item.objectName === name)
            return item
        var kids = item.children
        for (var i = 0; i < kids.length; ++i) {
            var hit = findByName(kids[i], name)
            if (hit !== null)
                return hit
        }
        return null
    }

    function bridgeFor(replies) {
        return {
            callModule: function (module, method, args) {
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    function rowsJson(n) {
        var parts = []
        for (var i = 0; i < n; ++i) {
            parts.push('{"thread":"t' + i + '","currentVersion":"t' + i + '",'
                + '"author":"' + "cd".repeat(32) + '",'
                + '"body":{"text":"post ' + i + '","removed":0,"marked":0},'
                + '"attachments":[],"isRevised":false,"isHidden":false}')
        }
        return parts.join(",")
    }

    // `stoaAddress` must be non-empty or `reload()` short-circuits before it
    // calls the bridge, and every screen here would be the unread state.
    //
    // Parented and rendered, per the table above: these subjects are measured
    // for height, and an unrendered FeedScreen reports 385 at every row count.
    function renderedFeed(rows, hasMore) {
        Core.bridge = bridgeFor({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[' + rowsJson(rows) + '],"page":0,'
                + '"hasMore":' + (hasMore ? "true" : "false") + '}'
        })
        var screen = createTemporaryObject(feedComponent, spec, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            stoaTitle: "Agora",
            width: 1000
        })
        waitForRendering(screen)
        return screen
    }

    // ---- `body.height`: the fillHeight guard ------------------------------

    // `findings/spec-test.md` box 1. The `body.height` binding was added to fix
    // a defect review found, and deleting it left the whole suite green while a
    // `Layout.fillHeight` child silently collapsed to zero — a blank region with
    // every gate passing. A guard nothing tests is a guard that gets removed.
    //
    // **The assertion is on the CHILD, not on the frame.** Both frames below are
    // 600 high whether or not the binding exists, so any assertion about
    // `frame.height` would pass under the mutation it exists to catch.
    function test_a_fillHeight_child_of_a_sized_card_is_given_the_cards_slack() {
        var frame = framedFiller.createObject(null, { width: 1000, height: 600 })
        var filler = findByName(frame, "filler")
        verify(filler !== null, "the probe child must be found at all")

        // Hardcoded from the geometry, not read back from the frame: the card is
        // 600 high and gives up `cardPaddingY` at the top and bottom, so the one
        // child claiming the slack gets 600 - 2*28 = 544. Deriving this from
        // `frame.height - 2 * DTheme.cardPaddingY` would be the implementation
        // told back to itself, and would agree with a broken binding as readily
        // as with a correct one.
        compare(filler.height, 544,
                "a child declaring Layout.fillHeight in a card given an explicit "
                + "height must receive the card's slack. It measures 0 without "
                + "ScreenFrame's `body.height` binding — rendering nothing, "
                + "logging nothing, raising no binding loop and passing qmllint. "
                + "An obligation discharged by a zero-height element is an "
                + "obligation not discharged.")
        frame.destroy()
    }

    // The scope of the number above, which `ScreenFrame.qml` states at length
    // and which is easy to over-read as a promise the shell does not make.
    //
    // **This is the case that passes for the wrong reason if written carelessly.**
    // Deleting the `body.height` binding ALSO makes the filler 0 here — so a test
    // asserting only this would report green on the mutation the test above
    // exists to catch. It is meaningful only beside that test: together they say
    // the binding changes the sized case and deliberately does not change this
    // one. Asserted so that a future author who "fixes" the content-sized case
    // has to come back and change a test that says it was a decision.
    function test_a_content_sized_card_has_no_slack_to_give_a_fillHeight_child() {
        var frame = framedFiller.createObject(null, { width: 1000 })
        var filler = findByName(frame, "filler")

        compare(filler.height, 0,
                "a card sized from its own content has no spare height to "
                + "distribute, so a fillHeight child measures 0 even with the "
                + "body.height binding. docs/UI-BRIEF.md's first draft promised "
                + "the opposite; a screen needing a full-height region cannot "
                + "get one from this shell as it stands.")

        // And the card is exactly its content plus the padding — 2 * 28 with a
        // zero-height child. The pair is what distinguishes "no slack" from
        // "no card": a frame that had collapsed entirely would also report a
        // zero-height child.
        compare(frame.implicitHeight, 56,
                "the card is its content plus cardPaddingY above and below")
        frame.destroy()
    }

    // ---- `implicitHeight`: the card reports its own size -------------------

    // `findings/correctness.md` box 4, first half. `Main.qml` reads
    // `implicitHeight` to size the Flickable's contentHeight, so a card that
    // does not report its height is a feed that does not scroll — which is how
    // the defect survived on `main`, where a thirty-row feed reported 0.
    function test_a_card_reports_the_height_of_its_content() {
        var frame = framedRows.createObject(null, { width: 1000 })

        // Hardcoded. Two 40px rows, one 20px gap between them, 28px padding above
        // and below: 40 + 20 + 40 + 2*28 = 156. A `> 0` floor would also pass on
        // a card reporting one row's height, or a constant.
        compare(frame.implicitHeight, 156,
                "the card must report its content's height plus its padding")

        // Stacked from the top, which is the layout a card sized by its own
        // content gets. The scatter case is the one below.
        var a = findByName(frame, "rowA")
        var b = findByName(frame, "rowB")
        compare(a.y, 0, "the first row starts at the top of the body")
        compare(b.y, 60, "and the second follows one blockGap after it")
        frame.destroy()
    }

    // The trade `ScreenFrame.qml`'s comment block names, pinned so that the
    // comment and the behaviour cannot drift apart. A card given an explicit
    // height with no child claiming the slack spreads it among the rows instead.
    //
    // No caller does this today, which is exactly why it is worth a test: the
    // comment is the only thing standing between the next author and a
    // surprise, and comments are not executed.
    function test_a_sized_card_with_no_claimant_scatters_its_rows() {
        var frame = framedRows.createObject(null, { width: 1000, height: 600 })
        var a = findByName(frame, "rowA")
        var b = findByName(frame, "rowB")

        compare(a.y, 111, "slack with no claimant is distributed between the rows")
        compare(b.y, 393, "which is why a card with an explicit height needs a "
                + "child declaring Layout.fillHeight to absorb it")

        // And the card still reports its CONTENT's height, unchanged by being
        // given a larger one — the property `Main.qml` depends on.
        compare(frame.implicitHeight, 156,
                "an explicit height must not change what the card reports as its "
                + "implicit one")
        frame.destroy()
    }

    // `findings/correctness.md` box 4, second half: that `implicitHeight` tracks
    // the content rather than merely being non-zero once.
    //
    // **Strictly monotonic, not `> 0`.** A floor passes on a card that reports a
    // constant — which is what `main` would have done had its zero been any other
    // number, and what an unrendered FeedScreen does report: 385 at 0 rows and
    // 385 at 30. The growth is the property `Main.qml` relies on, so the growth
    // is what is asserted.
    function test_a_feeds_reported_height_grows_with_the_rows_it_holds() {
        var heights = []
        var counts = [0, 1, 5, 30]
        for (var i = 0; i < counts.length; ++i)
            heights.push(renderedFeed(counts[i], false).implicitHeight)

        verify(heights[0] > 0,
               "an empty feed still reports the height of the copy it renders, "
               + "got " + heights[0])

        for (var j = 1; j < heights.length; ++j) {
            verify(heights[j] > heights[j - 1],
                   "a feed holding " + counts[j] + " posts must report a greater "
                   + "height than one holding " + counts[j - 1] + ": Main.qml "
                   + "sizes the Flickable's contentHeight from this, so a card "
                   + "whose height does not track its content is a feed that "
                   + "does not scroll. Measured " + heights.join(", ")
                   + " for " + counts.join(", ") + " rows.")
        }
    }
}
