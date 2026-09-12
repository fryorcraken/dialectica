import QtQuick
import QtQuick.Layouts
import QtTest
import "../src/qml"

// Geometry, measured against container bounds.
//
// ## Why this file exists
//
// The feed's first launch in basecamp rendered with text drawn on top of text
// and status panels in the same pixels. The 41 property-based tests that existed
// were all green throughout, and they could not have been otherwise: every one
// of them asks what a property HOLDS, and none asks where anything is DRAWN.
//
// Radicle hit this class first and recorded the lesson this file is built on:
//
//   > Measure against the container's bounds, not against `item.width` — an
//   > overflowing layout child keeps its full width and its `visible` stays
//   > true, so every property-based check passes while the user sees nothing.
//
// Its `tst_layout.qml` also names the basecamp-specific mechanism, which is the
// one dialectica just met: basecamp puts a `ui_qml` view inside a layout, so
// the view is sized by its PARENT rather than by its own implicit size. A root
// whose height is not derived from its children is handed something it did not
// choose, every child draws at y=0, and the screen becomes one illegible line.
//
// ## The two defects pinned here
//
// 1. `ScreenFrame` was a `Rectangle` with `implicitWidth` but **no
//    `implicitHeight`**, and it is not a layout — so its height was not derived
//    from its children at all. `implicitHeight` measured 0 while children
//    summed to several hundred pixels, and `Main.qml`'s
//    `contentHeight: frame.implicitHeight + 2 * cardPaddingY` therefore scrolled
//    over nothing but padding.
//
// 2. Three mutually-contradictory panels laid out at once (see
//    tst_feed_states.qml for the state-selection half). Geometrically it showed
//    as two panels sharing y=0, which is what `test_no_two_panels_share_pixels`
//    measures directly.
Item {
    id: root

    width: 1200
    height: 1400

    // ---- measuring helpers ----------------------------------------------
    //
    // Ported from radicle's tst_header_width.qml. The intersection with the
    // container is the only number that says what a user can actually see: an
    // item pushed outside its parent keeps a sensible `width`, a plausible `x`
    // and `visible === true`.

    /// How much of `item` falls inside `container`, horizontally.
    function visibleWidthIn(item, container) {
        if (!item || !item.visible)
            return 0
        var left = item.mapToItem(container, 0, 0).x
        var right = left + item.width
        return Math.max(0, Math.min(right, container.width) - Math.max(left, 0))
    }

    /// The same in the vertical axis. This is the one that catches a collapsed
    /// container: children stacked inside a zero-height parent have no vertical
    /// intersection with it, while every horizontal assertion stays green.
    function visibleHeightIn(item, container) {
        if (!item || !item.visible)
            return 0
        var top = item.mapToItem(container, 0, 0).y
        var bottom = top + item.height
        return Math.max(0, Math.min(bottom, container.height) - Math.max(top, 0))
    }

    /// Do two items overlap in the vertical axis, within `container`?
    ///
    /// Text drawn on top of text is the user-visible symptom, and it is a
    /// rectangle intersection rather than a property. Two panels that both
    /// report `visible: true` at `y: 0` is exactly what the owner photographed.
    function verticallyOverlap(a, b, container) {
        if (!a || !b || !a.visible || !b.visible)
            return false
        if (a.height <= 0 || b.height <= 0)
            return false
        var aTop = a.mapToItem(container, 0, 0).y
        var bTop = b.mapToItem(container, 0, 0).y
        var aBottom = aTop + a.height
        var bBottom = bTop + b.height
        // A shared edge is abutment, not overlap, so the comparison is strict.
        return aTop < bBottom - 0.5 && bTop < aBottom - 0.5
    }

    /// Every descendant of `node` whose `objectName` is exactly `name`.
    ///
    /// Returns ALL of them rather than the first, because "drawn exactly once"
    /// is one of the things being asserted and a finder that stops at the first
    /// hit structurally cannot see a duplicate.
    function findAllByName(node, name) {
        var found = []
        if (!node)
            return found
        if (node.objectName === name)
            found.push(node)
        for (var i = 0; i < node.children.length; i++)
            found = found.concat(findAllByName(node.children[i], name))
        return found
    }

    function findByName(node, name) {
        var all = findAllByName(node, name)
        return all.length > 0 ? all[0] : null
    }

    // ---- the host -------------------------------------------------------
    //
    // Deliberately shaped like `Main.qml`: a ColumnLayout inside a Flickable,
    // with the screen given a preferred WIDTH only. Nothing here supplies a
    // height, because the whole question is whether the screen derives one from
    // its own content — which is what basecamp does and what the old
    // `ScreenFrame` did not do.
    //
    // `clip: true` on the viewport is load-bearing: without it an overflowing
    // child still reports geometry and the intersection helpers above have
    // nothing to measure against.
    property int hostWidth: 1200

    Flickable {
        id: viewport
        objectName: "viewport"
        width: root.hostWidth
        height: root.height
        contentWidth: width
        contentHeight: column.implicitHeight
        clip: true

        ColumnLayout {
            id: column
            width: viewport.width
            spacing: 0

            FeedScreen {
                id: feed
                objectName: "feedUnderTest"
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(DTheme.cardWidth,
                                                viewport.width - 2 * DTheme.cardPaddingX)
            }
        }
    }

    TestCase {
        id: spec
        name: "FeedLayout"
        when: windowShown

        property var savedBridge: undefined

        function initTestCase() {
            spec.savedBridge = Core.bridge
        }

        function cleanupTestCase() {
            Core.bridge = spec.savedBridge
        }

        /// Let the layout actually run before measuring it.
        ///
        /// A layout re-arranges on a render pass, not on the assignment that
        /// changed its width, so reading geometry straight after setting
        /// `hostWidth` reports the PREVIOUS width's arrangement — which would
        /// both miss real failures and manufacture false ones.
        function settle() {
            waitForRendering(viewport)
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

        /// Point the feed at a fake core and re-read, then let it lay out.
        function load(replies, address) {
            Core.bridge = bridgeFor(replies)
            feed.stoaAddress = address === undefined ? "ab".repeat(32) : address
            feed.stoaGenesis = "00ff"
            feed.stoaTitle = "Agora"
            feed.page = 0
            feed.reload()
            settle()
        }

        function init() {
            root.hostWidth = 1200
        }

        // A store holding two posts: the state with the most content, so the
        // frame has the most to fail to account for.
        readonly property var twoPosts: ({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":['
                + '{"thread":"t1","currentVersion":"t1","author":"'
                + "11".repeat(32) + '","body":{"text":"the first post",'
                + '"removed":0,"marked":0},"attachments":[],'
                + '"isRevised":false,"isHidden":false},'
                + '{"thread":"t2","currentVersion":"v2","author":"'
                + "22".repeat(32) + '","body":{"text":"the second post",'
                + '"removed":0,"marked":0},"attachments":[],'
                + '"isRevised":true,"isHidden":false}'
                + '],"page":0,"hasMore":false}'
        })

        readonly property var storeBroken: ({
            "get_capabilities": '{"canPost":false,"reason":"No keystore found."}',
            "list_threads": '{"error":"the database at /x/ops.sqlite is locked"}'
        })

        readonly property var storeEmpty: ({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })

        // ---- defect 1: the frame must have a height of its own -----------

        /// THE LAYOUT DEFECT, stated as directly as it can be.
        ///
        /// `ScreenFrame` is the card every screen is built in, and it was a
        /// `Rectangle` — not a layout — carrying `implicitWidth` and no
        /// `implicitHeight`. A Rectangle does not derive a size from its
        /// children, so the card reported no height at all while holding several
        /// hundred pixels of content.
        ///
        /// Asserted against the CONTENT's height rather than against a
        /// hard-coded number, so the test stays true when the copy changes.
        function test_the_frame_reports_a_height_that_contains_its_content() {
            load(spec.twoPosts)

            verify(feed.implicitHeight > 0,
                   "the feed's implicitHeight is " + feed.implicitHeight
                   + " — a ScreenFrame that reports no height cannot be sized "
                   + "by a parent that asks it how tall it is, and basecamp "
                   + "asks. Every child then draws at y=0.")

            // The card must contain its content column, which since the
            // apparatus column was removed is the whole of its content.
            var body = root.findByName(feed, "screenBody")
            verify(body !== null, "the content column must be findable")

            verify(feed.height >= body.height - 1,
                   "the card is " + feed.height + "px tall but its content "
                   + "column is " + body.height + "px — the column is taller "
                   + "than the card containing it")
        }

        /// The card's height must actually CONTAIN every direct child of its
        /// content column, not merely be non-zero.
        ///
        /// This is the assertion that would have caught the launch: with the
        /// old frame each panel kept its real height and its real `visible`,
        /// and only the comparison against the containing card's bounds shows
        /// that they had nowhere to be drawn.
        function test_every_content_child_is_inside_the_card() {
            load(spec.twoPosts)

            var body = root.findByName(feed, "screenBody")
            verify(body !== null, "the content column must be findable")

            for (var i = 0; i < body.children.length; i++) {
                var c = body.children[i]
                if (!c.visible || c.height <= 0)
                    continue
                compare(root.visibleHeightIn(c, feed), c.height,
                        "content child " + i + " is " + c.height + "px tall but "
                        + "only " + root.visibleHeightIn(c, feed) + "px of it "
                        + "falls inside the " + feed.height + "px card — it is "
                        + "drawn outside the frame that is supposed to hold it")
            }
        }

        /// The frame grows with its content rather than reporting a constant.
        ///
        /// The counter-pressure that makes the test above a specification
        /// rather than an observation: a frame hard-coded to some large height
        /// would satisfy "contains its children" while being just as wrong, and
        /// would clip the moment the content outgrew the guess.
        ///
        /// Asserted on the CARD, which since the apparatus column was removed is
        /// a single column and therefore tracks its content directly. While the
        /// two-column card existed this had to be measured on the content column
        /// instead, because the apparatus column's three margin notes were
        /// taller than a two-post feed and pinned the card's height to their own
        /// word count.
        function test_the_card_height_tracks_the_amount_of_content() {
            load(spec.storeEmpty)
            var emptyHeight = feed.implicitHeight

            load(spec.twoPosts)
            var postsHeight = feed.implicitHeight

            verify(postsHeight > emptyHeight,
                   "the card is " + postsHeight + "px with two posts and "
                   + emptyHeight + "px with none — it is not deriving its "
                   + "height from its content")
        }

        // ---- defect 2: one state at a time, geometrically ----------------

        /// Text must never be drawn on top of text.
        ///
        /// The owner's screenshot showed "The store could not be read…" printed
        /// over "You cannot reply in this Stoa yet.". That is two panels
        /// occupying one rectangle, and no property-based assertion can see it:
        /// both panels report a height, a position and `visible: true`.
        ///
        /// Run across all three read states, because the panels differ per
        /// state and a single-state check would pick one and miss the others.
        function test_no_two_panels_share_pixels() {
            var cases = [
                { name: "a broken store", replies: spec.storeBroken, address: undefined },
                { name: "an empty store", replies: spec.storeEmpty, address: undefined },
                { name: "two posts", replies: spec.twoPosts, address: undefined },
                { name: "no address at all", replies: spec.storeEmpty, address: "" }
            ]

            for (var c = 0; c < cases.length; c++) {
                load(cases[c].replies, cases[c].address)

                var body = root.findByName(feed, "screenBody")
                verify(body !== null)

                // Only the direct children of the content column are compared:
                // they are siblings in one vertical stack, so any vertical
                // overlap between two of them is a layout failure by
                // definition.
                var kids = []
                for (var i = 0; i < body.children.length; i++) {
                    var k = body.children[i]
                    if (k.visible && k.height > 0)
                        kids.push(k)
                }

                for (var a = 0; a < kids.length; a++) {
                    for (var b = a + 1; b < kids.length; b++) {
                        verify(!root.verticallyOverlap(kids[a], kids[b], feed),
                               "with " + cases[c].name + ", content children "
                               + a + " and " + b + " overlap vertically: one at "
                               + "y=" + kids[a].mapToItem(feed, 0, 0).y + " h="
                               + kids[a].height + ", the other at y="
                               + kids[b].mapToItem(feed, 0, 0).y + " h="
                               + kids[b].height
                               + " — this is text drawn on top of text")
                    }
                }
            }
        }

        /// Exactly one of the three read-state panels is ever on screen.
        ///
        /// The geometric companion to the state-selection tests in
        /// tst_feed_states.qml. That file asserts `readState` holds one value;
        /// this one asserts the SCREEN agrees, which is a different claim — the
        /// launch had a correct `readState` of "failed" and rendered the empty
        /// copy anyway.
        function test_exactly_one_read_state_panel_is_visible() {
            var cases = [
                { name: "a broken store", replies: spec.storeBroken, address: undefined, expect: "failedPanel" },
                { name: "an empty store", replies: spec.storeEmpty, address: undefined, expect: "emptyPanel" },
                { name: "no address at all", replies: spec.storeEmpty, address: "", expect: "failedPanel" }
            ]

            for (var c = 0; c < cases.length; c++) {
                load(cases[c].replies, cases[c].address)

                var panels = ["failedPanel", "emptyPanel"]
                var shown = []
                for (var i = 0; i < panels.length; i++) {
                    var p = root.findByName(feed, panels[i])
                    if (p !== null && p.visible && p.height > 0)
                        shown.push(panels[i])
                }

                compare(shown.length, 1,
                        "with " + cases[c].name + " the panels on screen are ["
                        + shown.join(", ") + "] — exactly one of the read-state "
                        + "panels may render, because an empty store and an "
                        + "unreadable one mean opposite things (UI-BRIEF "
                        + "obligation 5)")
                compare(shown[0], cases[c].expect,
                        "with " + cases[c].name + " the panel on screen is "
                        + shown[0] + " rather than " + cases[c].expect)
            }
        }

        /// Nothing is drawn twice.
        ///
        /// Radicle's `tst_layout.qml` asserts its header is drawn exactly once;
        /// this is the same guard over the elements a duplicated card or a
        /// double-instantiated panel would reveal. A second copy of the header
        /// is precisely what "labels colliding, overprinted" looks like.
        function test_each_singular_element_is_drawn_exactly_once() {
            load(spec.twoPosts)

            // A `Repeater` is not an Item and never appears in a `children`
            // walk, so the ordering row is deliberately absent from this list —
            // asserting on it would assert on nothing.
            var singular = ["screenBody", "feedHeader"]
            for (var i = 0; i < singular.length; i++) {
                var hits = root.findAllByName(feed, singular[i])
                compare(hits.length, 1,
                        singular[i] + " appears " + hits.length + " times in "
                        + "the card — a duplicated element paints over its twin")
            }
        }

        // The apparatus column had its own geometry tests here — its notes had
        // to stay inside it and not overprint each other, which was the smeared
        // right-hand column the owner photographed. The column has since been
        // removed from the rendered UI (it was the design bundle's annotation of
        // itself, addressed to someone reading the mockup), so there is nothing
        // left for those assertions to measure.

        // ---- across widths ----------------------------------------------

        /// The whole card stays on screen at every plausible window width.
        ///
        /// Radicle's lesson about driving the width rather than testing one
        /// size: every fixture in this repo was a single comfortable width,
        /// which is exactly why an overflow shipped. 1200 is a wide window; 640
        /// is a narrow docked panel, below the 1000px card width, so the card
        /// must shrink rather than overflow.
        readonly property var probeWidths: [1200, 1000, 900, 760, 640]

        function test_the_card_fits_its_viewport_at_every_width() {
            load(spec.twoPosts)

            for (var i = 0; i < spec.probeWidths.length; i++) {
                var w = spec.probeWidths[i]
                root.hostWidth = w
                settle()

                compare(root.visibleWidthIn(feed, viewport), feed.width,
                        "at viewport width " + w + " the card is " + feed.width
                        + "px wide but only "
                        + root.visibleWidthIn(feed, viewport)
                        + "px of it is on screen — it is overflowing the "
                        + "viewport rather than fitting inside it")
            }
        }

        /// Content stays inside the card at every width too.
        ///
        /// A card that fits its viewport while its own children hang out of it
        /// is the same bug one level down, and the horizontal assertion above
        /// cannot see it.
        function test_content_stays_inside_the_card_at_every_width() {
            load(spec.twoPosts)

            for (var i = 0; i < spec.probeWidths.length; i++) {
                var w = spec.probeWidths[i]
                root.hostWidth = w
                settle()

                var body = root.findByName(feed, "screenBody")
                compare(root.visibleWidthIn(body, feed), body.width,
                        "at viewport width " + w + " the content column shows "
                        + root.visibleWidthIn(body, feed) + "px of its "
                        + body.width + "px inside the card")

                // The header is the row with the most incompressible parts in
                // it — an identicon, a title, a NoWrap-abbreviated address, the
                // ordering label and SHOW HIDDEN — so it is the element that
                // overflows first when the card narrows. A RowLayout whose
                // minimums do not fit does not shrink, it OVERFLOWS.
                var header = root.findByName(feed, "feedHeader")
                compare(root.visibleWidthIn(header, feed), header.width,
                        "at viewport width " + w + " the header shows "
                        + root.visibleWidthIn(header, feed) + "px of its "
                        + header.width + "px inside the card — the row could not "
                        + "shrink to fit and overflowed instead")
            }
        }

        /// The retry button stays reachable, and a real click reaches it.
        ///
        /// Geometry and clickability are different claims, and radicle has been
        /// bitten by each without the other. The retry button is the only way
        /// out of the failed state, so it is this screen's "Settings chip".
        function test_the_retry_button_is_clickable_at_every_width() {
            load(spec.storeBroken)

            for (var i = 0; i < spec.probeWidths.length; i++) {
                var w = spec.probeWidths[i]
                root.hostWidth = w
                settle()

                var retry = root.findByName(feed, "retryButton")
                verify(retry !== null, "the retry button must be findable")
                verify(retry.visible, "at width " + w + " retry is not visible")

                compare(root.visibleWidthIn(retry, feed), retry.width,
                        "at viewport width " + w + " the retry button shows "
                        + root.visibleWidthIn(retry, feed) + "px of its "
                        + retry.width + "px — the only way out of the failed "
                        + "state is off the edge of the card")
                compare(root.visibleHeightIn(retry, feed), retry.height,
                        "at viewport width " + w + " the retry button shows "
                        + root.visibleHeightIn(retry, feed) + "px of its "
                        + retry.height + "px vertically")
            }
        }
    }
}
