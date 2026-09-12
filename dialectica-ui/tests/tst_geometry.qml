import QtQuick
import QtQuick.Layouts
import QtTest
import "../src/qml"
import "geometry.js" as Geometry

// The card chassis, measured — and the measuring helpers, tested.
//
// ## What this file is for
//
// Two things that belong together, because neither is worth much alone.
//
// 1. **`ScreenFrame` must report a height that contains its children.** This is
//    the defect that made the first basecamp launch one illegible pile: the card
//    is a `Rectangle`, not a layout, so nothing derived its height from its
//    content, and it answered 0 while holding several hundred pixels. Basecamp
//    hosts a `ui_qml` view INSIDE a layout, so the view is sized by its parent
//    asking how tall it wants to be; a root that cannot answer gets a size it did
//    not choose. The fix was an `implicitHeight`, and this is what holds it down.
//
//    The feed screen that first exposed this is gone — it was the wrong first
//    screen, and it is not coming back in this shape. The CARD is not going
//    anywhere: every screen is built in one. So the coverage follows the card,
//    asserted against a stand-in content column rather than against any
//    particular screen's copy. That makes it a stronger test than the feed
//    version, not a weaker one: it cannot pass for a reason peculiar to one
//    screen's layout.
//
// 2. **The helpers in `geometry.js` must themselves be measured.** They are the
//    coverage class this project had none of, and the next screen's layout spec
//    will be written on top of them. A helper with no caller and no test is dead
//    code that rots quietly; worse, a helper that is subtly wrong makes every
//    spec built on it green for the wrong reason. So each one is exercised
//    against geometry whose answer is known independently — including the
//    failing case, which is the half that proves the helper can say "no".
Item {
    id: root

    width: 1200
    height: 1400

    property int hostWidth: 1200

    // Deliberately shaped like `Main.qml`: a ColumnLayout inside a clipping
    // Flickable, with the card given a preferred WIDTH only. Nothing supplies a
    // height, because the whole question is whether the card derives one from its
    // own content.
    //
    // `clip: true` is load-bearing: without it an overflowing child still reports
    // geometry and the intersection helpers have nothing to measure against.
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

            ScreenFrame {
                id: card
                objectName: "cardUnderTest"
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(DTheme.cardWidth,
                                                viewport.width - 2 * DTheme.cardPaddingX)

                // A stand-in for whatever screen is eventually built in here:
                // blocks of a known height, so the card's reported height can be
                // checked against a number this spec controls rather than against
                // a screen's copy. `blockCount` varies to prove the card TRACKS
                // its content instead of happening to be tall enough once.
                Repeater {
                    model: root.blockCount

                    Rectangle {
                        objectName: "block"
                        Layout.fillWidth: true
                        implicitHeight: root.blockHeight
                        color: DTheme.paper
                        border.width: DTheme.hairline
                        border.color: DTheme.ink
                    }
                }
            }
        }
    }

    property int blockCount: 3
    property int blockHeight: 60

    TestCase {
        id: spec
        name: "Geometry"
        when: windowShown

        function init() {
            root.hostWidth = 1200
            root.blockCount = 3
            root.blockHeight = 60
        }

        /// Let the layout actually run before measuring it.
        ///
        /// A layout re-arranges on a render pass, not on the assignment that
        /// changed its width, so reading geometry straight after setting
        /// `hostWidth` reports the PREVIOUS width's arrangement — which would
        /// both miss real failures and manufacture false ones.
        ///
        /// ONLY CALL THIS AFTER CHANGING SOMETHING. `waitForRendering` waits for
        /// the next frame, and when nothing is dirty no frame is scheduled, so it
        /// blocks for its whole 5s timeout and then returns false. That is not
        /// merely slow — a `settle()` that timed out synchronised nothing, so the
        /// measurement after it is of whatever was on screen before. An earlier
        /// draft of this spec opened each test with a bare `settle()` and paid
        /// 5 seconds a time for no synchronisation at all.
        function settle() {
            verify(waitForRendering(viewport),
                   "waitForRendering timed out — nothing was dirty, so this "
                   + "settle() synchronised nothing and the geometry read after "
                   + "it is stale. Only call settle() after a change.")
        }

        /// Bring the fixture to a known geometry AND leave a frame pending, so
        /// the `settle()` that follows has something real to wait for.
        ///
        /// `init()` resets the same three properties, but a reset is not
        /// necessarily a CHANGE: when the previous test left them at these values
        /// the assignments are no-ops and schedule no frame. Nudging the width to
        /// a throwaway value first guarantees the dirty state that makes the
        /// subsequent wait meaningful.
        function reset(blocks, blockPx) {
            root.hostWidth = 900
            root.blockCount = blocks === undefined ? 3 : blocks
            root.blockHeight = blockPx === undefined ? 60 : blockPx
            waitForRendering(viewport)
            root.hostWidth = 1200
            settle()
        }

        // ---- the card must have a height of its own ----------------------

        /// THE LAYOUT DEFECT, stated as directly as it can be.
        function test_the_card_reports_a_height_that_contains_its_content() {
            reset()

            verify(card.implicitHeight > 0,
                   "the card's implicitHeight is 0 — a Rectangle does not derive "
                   + "a height from its children, so it must declare one")

            var body = Geometry.findByName(card, "screenBody")
            verify(body !== null, "the card has no screenBody column")
            verify(card.implicitHeight >= body.implicitHeight,
                   "the card reports " + card.implicitHeight
                   + "px while its content column is " + body.implicitHeight + "px")
        }

        /// Not merely non-zero once: the height must FOLLOW the content. A
        /// hard-coded height would pass the test above and fail this one.
        function test_the_card_height_tracks_the_amount_of_content() {
            root.blockCount = 1
            settle()
            var shortCard = card.implicitHeight

            root.blockCount = 5
            settle()
            var tallCard = card.implicitHeight

            verify(shortCard > 0 && tallCard > 0,
                   "the card is " + shortCard + "px with one block and "
                   + tallCard + "px with five")
            verify(tallCard > shortCard,
                   "the card did not grow with its content: " + shortCard
                   + "px with one block, " + tallCard + "px with five")
        }

        /// Every block must fall inside the card, at every width. This is the
        /// assertion the feed's version made, generalised off the feed.
        function test_content_stays_inside_the_card_at_every_width() {
            reset()

            // 1200 is a wide window; 640 is a narrow docked panel, below the
            // card's own width, so the card must shrink rather than overflow.
            // The sweep starts BELOW the width `reset()` leaves behind, so every
            // assignment in the loop is a real change and every settle() waits
            // for a real frame.
            var widths = [1000, 880, 760, 640, 1200]

            for (var w = 0; w < widths.length; w++) {
                root.hostWidth = widths[w]
                settle()

                var blocks = Geometry.findAllByName(card, "block")
                compare(blocks.length, root.blockCount,
                        "expected " + root.blockCount + " blocks at width "
                        + widths[w] + ", found " + blocks.length)

                for (var i = 0; i < blocks.length; i++) {
                    var shown = Geometry.visibleHeightIn(blocks[i], card)
                    compare(shown, blocks[i].height,
                            "at viewport width " + widths[w] + ", block " + i
                            + " is " + blocks[i].height + "px tall but only "
                            + shown + "px of it falls inside the "
                            + card.height + "px card")

                    var across = Geometry.visibleWidthIn(blocks[i], card)
                    compare(across, blocks[i].width,
                            "at viewport width " + widths[w] + ", block " + i
                            + " is " + blocks[i].width + "px wide but only "
                            + across + "px of it falls inside the "
                            + card.width + "px card")
                }
            }
        }

        /// The card itself must fit the viewport rather than overflowing it.
        function test_the_card_fits_its_viewport_at_every_width() {
            reset()

            var widths = [1000, 880, 760, 640, 1200]

            for (var w = 0; w < widths.length; w++) {
                root.hostWidth = widths[w]
                settle()

                var shown = Geometry.visibleWidthIn(card, viewport)
                compare(shown, card.width,
                        "at viewport width " + widths[w] + ", the card is "
                        + card.width + "px wide but only " + shown
                        + "px of it is on screen")
            }
        }

        /// Stacked blocks abut; they must never share pixels. This is the
        /// text-on-top-of-text symptom, asserted on the chassis.
        function test_no_two_blocks_share_pixels() {
            reset()

            var blocks = Geometry.findAllByName(card, "block")
            verify(blocks.length >= 2, "need at least two blocks to compare")

            for (var i = 0; i < blocks.length; i++) {
                for (var j = i + 1; j < blocks.length; j++) {
                    verify(!Geometry.verticallyOverlap(blocks[i], blocks[j], card),
                           "block " + i + " and block " + j
                           + " share pixels — they are drawn on top of each other")
                }
            }
        }

        // ---- the helpers themselves --------------------------------------
        //
        // Each helper is checked against geometry whose answer is known without
        // it, and — the half that matters — against a case where it must say no.
        // A helper only ever exercised on a passing layout is a helper nobody has
        // shown can fail.
        //
        // NO `settle()` BELOW THIS LINE, deliberately. The fixture at the bottom
        // of this file sets `x`, `y`, `width` and `height` directly, with no
        // layout governing them, so `mapToItem` reflects an assignment on the
        // spot — there is nothing to wait for. Waiting anyway would be waiting on
        // `viewport`, which these tests do not touch: the only honest options are
        // a wait that times out or a wait that succeeds for an unrelated reason,
        // and neither is synchronisation. This is exactly why the helpers get a
        // plain fixture instead of being probed through the card.

        /// An item fully inside its container shows all of itself; one pushed
        /// halfway out shows half; one pushed entirely out shows none. The last
        /// case is the whole reason the helper exists, because the item's own
        /// `width` and `visible` are unchanged in every one of the three.
        function test_visibleWidthIn_reports_the_intersection_not_the_width() {
            compare(Geometry.visibleWidthIn(probe, probeBox), 100,
                    "a fully-contained item should report its whole width")

            probe.x = 150
            compare(Geometry.visibleWidthIn(probe, probeBox), 50,
                    "an item hanging 50px over the right edge should report 50px")
            compare(probe.width, 100,
                    "the item's own width is unchanged — which is exactly why a "
                    + "property-based check cannot see this")
            verify(probe.visible,
                   "the item is still visible:true while half of it is off-screen")

            probe.x = 400
            compare(Geometry.visibleWidthIn(probe, probeBox), 0,
                    "an item entirely outside its container should report 0px")

            probe.x = -100
            compare(Geometry.visibleWidthIn(probe, probeBox), 0,
                    "an item entirely off the LEFT should also report 0px")

            probe.x = -50
            compare(Geometry.visibleWidthIn(probe, probeBox), 50,
                    "an item hanging 50px off the left should report 50px")

            probe.x = 0
        }

        /// The vertical twin, including the collapsed-container case: a child of
        /// a zero-height parent has no vertical intersection with it, and that is
        /// the shape of the defect this suite exists for.
        function test_visibleHeightIn_reports_the_intersection_not_the_height() {
            compare(Geometry.visibleHeightIn(probe, probeBox), 100,
                    "a fully-contained item should report its whole height")

            probe.y = 150
            compare(Geometry.visibleHeightIn(probe, probeBox), 50,
                    "an item hanging 50px below the bottom should report 50px")

            probe.y = 0
            probeBox.height = 0
            compare(Geometry.visibleHeightIn(probe, probeBox), 0,
                    "inside a COLLAPSED container nothing is visible — this is "
                    + "the ScreenFrame defect's signature")
            compare(probe.height, 100,
                    "and the child's own height is untouched, so every "
                    + "property-based assertion stays green")

            probeBox.height = 200
        }

        /// An invisible item shows nothing, whatever its geometry says. Without
        /// this both helpers would credit a hidden item with its full size.
        function test_an_invisible_item_shows_nothing() {
            probe.visible = false
            compare(Geometry.visibleWidthIn(probe, probeBox), 0,
                    "a hidden item cannot be seen")
            compare(Geometry.visibleHeightIn(probe, probeBox), 0,
                    "a hidden item cannot be seen")

            probe.visible = true
        }

        /// A null item is 0 rather than a crash — a finder that found nothing is
        /// a normal outcome in a spec that asserts absence.
        function test_a_missing_item_is_zero_rather_than_an_error() {
            compare(Geometry.visibleWidthIn(null, probeBox), 0)
            compare(Geometry.visibleHeightIn(null, probeBox), 0)
            compare(Geometry.verticallyOverlap(null, probe, probeBox), false)
            compare(Geometry.verticallyOverlap(probe, null, probeBox), false)
        }

        /// Overlap must be true when two items share pixels, false when they
        /// merely abut, and false when one has no height. Abutment is the case a
        /// naive implementation gets wrong, and it would report a defect on every
        /// correct ColumnLayout.
        function test_verticallyOverlap_separates_sharing_from_abutting() {
            probe.y = 0
            probe.height = 100
            other.y = 100
            other.height = 100
            compare(Geometry.verticallyOverlap(probe, other, probeBox), false,
                    "two items sharing an EDGE abut; they do not overlap")

            other.y = 50
            compare(Geometry.verticallyOverlap(probe, other, probeBox), true,
                    "two items sharing 50px of vertical span DO overlap")

            other.y = 0
            compare(Geometry.verticallyOverlap(probe, other, probeBox), true,
                    "two items at the same y overlap completely — the exact "
                    + "shape of the launch defect")

            other.y = 100
            other.height = 0
            compare(Geometry.verticallyOverlap(probe, other, probeBox), false,
                    "a zero-height item overlaps nothing")

            other.height = 100
            probe.height = 100
            probe.y = 0
        }

        /// The finder returns every match, not the first. "Drawn exactly once" is
        /// an assertion a first-hit finder structurally cannot make.
        function test_findAllByName_returns_every_match() {
            var all = Geometry.findAllByName(probeBox, "twin")
            compare(all.length, 2,
                    "two items share the objectName 'twin'; the finder must "
                    + "return both or a duplicate can never be detected")

            compare(Geometry.findAllByName(probeBox, "nothing-has-this-name").length, 0,
                    "a name nothing carries yields an empty list")

            var one = Geometry.findByName(probeBox, "twin")
            verify(one !== null, "findByName returns the first of several")
            compare(Geometry.findByName(probeBox, "nothing-has-this-name"), null,
                    "findByName returns null rather than undefined when nothing matches")
        }

        /// The finder must reach a nested descendant, not just direct children.
        function test_findByName_searches_the_whole_tree() {
            var deep = Geometry.findByName(probeBox, "buried")
            verify(deep !== null,
                   "a descendant several levels down must still be found")
            compare(deep.objectName, "buried")
        }
    }

    // ---- fixtures for the helper tests ----------------------------------
    //
    // A plain box with items whose geometry this spec sets directly, so each
    // helper's expected answer is arithmetic rather than a layout's opinion.
    Item {
        id: probeBox
        objectName: "probeBox"
        width: 200
        height: 200

        Rectangle {
            id: probe
            objectName: "twin"
            width: 100
            height: 100
            color: "transparent"
        }

        Rectangle {
            id: other
            objectName: "twin"
            width: 100
            height: 100
            color: "transparent"

            Item {
                Item {
                    objectName: "buried"
                }
            }
        }
    }
}
