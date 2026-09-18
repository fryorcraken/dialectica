import QtQuick
import QtTest
import "../src/qml"

// **Do the screens paint any CONTENT?**
//
// Nothing else in this suite asks. Every other spec reads properties, counts
// rows and inspects text — all of which a component satisfies perfectly while
// painting nothing. `run-qml-tests.sh`'s `check_bindings` closes the case where
// a binding evaluates to `undefined`, and `check_qml_names.py` closes the case
// where a type name collides with basecamp's. NEITHER can see a component whose
// children never enter the scene graph: nothing warns, no binding is undefined,
// no name collides, every gate stays green, and the symptom is a blank view —
// which CLAUDE.md records as indistinguishable from a plugin that failed to
// load.
//
// The defect is on record in the sibling repo rather than hypothetical.
// `radicle-ui`'s CommitView.qml declared a property named `data`, shadowing
// Item's default property, so every declared child silently failed to become a
// scene-graph child and the whole view rendered blank. `RenderProbeShadowed.qml`
// reproduces it here.
//
// ===========================================================================
// FOUR THINGS BELOW LOOK SIMPLIFIABLE TOWARD THE SUITE'S HOUSE STYLE. NONE IS.
// Each was measured on Qt 6.10.3 on this tree; each has a recorded number.
// ===========================================================================
//
// 1. **The root is an `Item`, and the `TestCase` is a sibling of the subjects.**
//    Every other spec here makes `TestCase` the file root. Written that way,
//    `grabImage` returns a UNIFORMLY WHITE BUFFER whatever was painted:
//
//      | shape                          | subject                     | grab             |
//      |--------------------------------|-----------------------------|------------------|
//      | `TestCase` as file root        | red+blue Rectangle          | #ffffff, 0 diff  |
//      | `TestCase` as file root        | same, createTemporaryObject | #ffffff, 0 diff  |
//      | `TestCase` as file root        | the TestCase itself         | #ffffff, 0 diff  |
//      | `Item` root, subject a sibling | same red+blue Rectangle     | #ff0000, 400 diff|
//
//    So a probe in the natural shape FAILS ON A HEALTHY SCREEN, and the natural
//    repair — loosening the assertion until it passes — yields a probe that can
//    never fail. The precise ingredient is the TestCase being the FILE ROOT, not
//    merely being the subject's parent.
//
// 2. **No two subjects overlap, and the TestCase is parked clear of all of
//    them.** `grabImage(item)` captures the WINDOW REGION at that item's
//    geometry, so a subject with anything behind it is grabbed together with
//    whatever that thing painted. Measured: a blank Item over a painted
//    Rectangle reported #008000 and 900 differing pixels — passing the probe on
//    another item's paint. The same blank Item alone reported #ffffff, 0.
//
// 3. **THE GRABBED REGION IS THE SCREEN'S CONTENT AREA, NOT THE WHOLE CARD.**
//    This is the one that makes the probe mean anything, and the version of
//    this file that grabbed the whole card was a FALSE GREEN that nearly
//    shipped.
//
//    `ScreenFrame`'s root is a Rectangle painting `DTheme.paper` with a
//    `DTheme.ink` hairline border, so THE CARD PAINTS TWO COLOURS WHETHER OR NOT
//    ANY CONTENT SURVIVES. Measured: a ScreenFrame containing NOTHING AT ALL —
//    56px tall, no children — grabbed over a full cell reports 38,487 differing
//    pixels and sails through the probe. Every screen probe here would have
//    passed on chrome, and the defect the file exists to catch would have been
//    invisible to it.
//
//    So each screen is grabbed through a region inset past the card's padding
//    (`DTheme.cardPaddingX` 34, `cardPaddingY` 28, `hairline` 1), where only
//    content can paint. Measured with the inset: a populated FeedScreen's
//    content reports 1,116 differing; an empty ScreenFrame's content is FLAT.
//
// 4. **The content area is reached by SAMPLING a sub-rectangle of the cell's
//    grab, never by grabbing a nested Item placed over the content.** The
//    nested-Item form looks like the obvious way to express "just this region"
//    and it silently measures nothing: `grabImage` on a childless Item returns
//    that item's own empty subtree, not what is painted beneath it. Measured —
//    a content Item at x=40 y=34, 920x289, wholly inside a 357px-tall populated
//    card, grabbed FLAT #ffffff. The same region sampled out of the cell's own
//    grab reported 3,402 differing pixels.
//
//    This is the trap that makes the whole probe worthless if it is
//    reintroduced, because it fails in the passing direction for the four
//    screen tests and would be "fixed" by dropping the inset.
//
// 5. **The sampled rectangle is sized to ITS OWN screen's height**, not to the
//    cell's. A region taller than the screen reads PAST IT into whatever else
//    is in the cell. Measured, in an earlier shape: a populated feed and an
//    EMPTY frame both reported exactly 11,926 differing pixels — the identical
//    number is the tell, and it was the same out-of-card paint read twice.
//
// WHAT A PASS MEANS, AND WHAT IT DOES NOT. A passing probe is evidence that more
// than one colour was painted in a screen's content area, and evidence of
// NOTHING else — not that the layout, the colours, the text or the completeness
// are right. A screen rendering entirely wrong content in entirely wrong colours
// passes. Do not cite a green run here as showing a screen still renders
// correctly.
Item {
    id: stage
    width: 2400
    height: 1600

    readonly property int cellW: 1000
    readonly property int cellH: 620

    // Inset past the card's padding and border on every side, so the grabbed
    // region contains content and nothing else. Deliberately a few pixels
    // larger than DTheme's 34/28 rather than equal to it: a region flush with
    // the padding box catches the border's antialiased edge, which would paint
    // a second colour and hand the probe a pass it did not earn.
    readonly property int insetX: 40
    readonly property int insetY: 34

    // Fine enough that the smallest element any covered screen paints cannot
    // fall between sampled points: the smallest painted features here are 1px
    // borders and separators, and each runs the full width or height of a
    // block, so a 4px grid crosses them many times. No covered screen paints an
    // isolated feature smaller than 4x4.
    readonly property int stride: 4

    TestCase {
        id: spec
        name: "RenderProbe"
        when: windowShown
        width: 180
        height: 180
        // Parked in the grid's spare corner, clear of every subject cell. See
        // note 2 above: a TestCase over a subject would be grabbed with it.
        x: 2200
        y: 1400

        property var savedBridge: undefined

        function init() { spec.savedBridge = Core.bridge }
        function cleanup() { Core.bridge = spec.savedBridge }

        function bridgeFor(replies) {
            return {
                callModule: function (module, method, args) {
                    if (replies[method] === undefined)
                        return '{"error":"no fake reply for ' + method + '"}'
                    return replies[method]
                }
            }
        }

        // ---- the probe itself ------------------------------------------

        // THE PROBE. Sample a rectangle of an image on the stride, compare each
        // sample against the rectangle's FIRST sample, and stop at the first
        // difference.
        //
        // Narrowing to a rectangle — rather than grabbing a nested Item over the
        // region — is the only form that works; see note 4.
        //
        // Returning the sampled count as well as the verdict is what lets a test
        // assert a pass was reached WITHOUT a full scan: the spec requires the
        // stride, and a probe that quietly scanned everything would satisfy the
        // verdict while failing the requirement.
        function sampleRegionForDifference(img, rx, ry, rw, rh) {
            var first = img.pixel(rx, ry)
            var sampled = 0
            for (var y = ry; y < ry + rh; y += stage.stride) {
                for (var x = rx; x < rx + rw; x += stage.stride) {
                    sampled += 1
                    if (img.pixel(x, y) !== first)
                        return { flat: false, sampled: sampled, first: first }
                }
            }
            return { flat: true, sampled: sampled, first: first }
        }

        // The whole of an image, which is the common case for the subjects that
        // are not cards.
        function sampleForDifference(img) {
            return spec.sampleRegionForDifference(img, 0, 0,
                                                  img.width, img.height)
        }

        // Grab a screen's cell and sample only the screen's content area —
        // inset past the card's padding, and sized from the SCREEN's own
        // geometry (note 5) so a collapsed screen cannot read past itself.
        //
        // `minSide` guards the degenerate end: a region a few pixels tall is
        // trivially flat, and "flat because there was nothing to look at" must
        // never be reported as "flat because nothing painted".
        function probeContentOf(cell, screen, minSide) {
            var rw = screen.width - 2 * stage.insetX
            var rh = screen.height - 2 * stage.insetY

            verify(rw >= minSide && rh >= minSide,
                   "the content region is " + rw + "x" + rh + ", below the "
                   + minSide + "px floor. A region this small is flat whatever "
                   + "happened, so neither verdict would mean anything.")

            var img = spec.grabImage(cell)
            verify(img !== null && img !== undefined,
                   "grabImage returned nothing — the probe measured NO pixels, "
                   + "which must never be mistaken for a pass")
            verify(stage.insetX + rw <= img.width
                   && stage.insetY + rh <= img.height,
                   "the content rectangle (" + rw + "x" + rh + " at "
                   + stage.insetX + "," + stage.insetY + ") does not fit the "
                   + img.width + "x" + img.height + " grab, so it would sample "
                   + "outside the cell")
            return spec.sampleRegionForDifference(img, stage.insetX,
                                                  stage.insetY, rw, rh)
        }

        function assertContentPaints(cell, screen, label) {
            // 100px floor: every populated screen here is far taller, and an
            // empty ScreenFrame's content area is 12px — so this floor also
            // catches a screen that collapsed to chrome.
            var r = spec.probeContentOf(cell, screen, 100)
            verify(!r.flat,
                   label + " painted a single flat colour (" + r.first + ") "
                   + "across its whole content area over " + r.sampled
                   + " sampled pixels. Every sampled pixel equalled the first, "
                   + "which is what a screen whose children never reached the "
                   + "scene graph looks like. NOTE: the card's own paper and "
                   + "border are OUTSIDE this rectangle by construction, so "
                   + "chrome cannot rescue this assertion.")
            return r
        }

        // ---- the four screens -------------------------------------------
        //
        // Each is driven to a POPULATED state through the same faked module
        // boundary the rest of the suite uses. A screen probed in a state where
        // it is CORRECT for it to render nothing would fail for a reason that
        // is not a defect, so each test asserts the populated precondition
        // before it asserts the paint.

        function test_a_populated_feed_screen_paints_content() {
            Core.bridge = spec.bridgeFor({
                "get_capabilities": '{"canPost":true,"identity":"deadbeef"}',
                "list_threads": '{"items":['
                    + '{"thread":"t1","currentVersion":"t1","author":"'
                    + "aa".repeat(32) + '",'
                    + '"body":{"text":"A first thread body","removed":0,"marked":0},'
                    + '"attachments":[],"isRevised":false,"isHidden":false}'
                    + '],"page":0,"hasMore":false}'
            })
            // No explicit height: ScreenFrame is content-sized and its contract
            // forbids one — a Layout.fillHeight child in a card given a fixed
            // height measures 0.
            var screen = feedComponent.createObject(feedCell, {
                stoaAddress: "ab".repeat(32),
                stoaGenesis: "00ff",
                stoaTitle: "Agora",
                width: stage.cellW
            })

            compare(screen.readState, "ok", "the fixture's own precondition")
            compare(screen.rows.length, 1, "the screen must have a row to paint")

            waitForRendering(stage)
            spec.assertContentPaints(feedCell, screen, "FeedScreen")
            screen.destroy()
        }

        function test_a_populated_stoa_list_screen_paints_content() {
            Core.bridge = spec.bridgeFor({
                "list_stoas": '{"items":[{"stoa":"' + "7f3a91c4" + "00".repeat(28)
                    + '","foundingTitle":"Nym Research"}],"page":0,"hasMore":false}',
                "get_capabilities": '{"canPost":true,"identity":"deadbeef"}'
            })
            var screen = listComponent.createObject(listCell, {
                width: stage.cellW
            })

            compare(screen.readState, "ok", "the fixture's own precondition")
            compare(screen.visibleRows.length, 1,
                    "the screen must have a row to paint")

            waitForRendering(stage)
            spec.assertContentPaints(listCell, screen, "DStoaListScreen")
            screen.destroy()
        }

        function test_a_joined_join_screen_paints_content() {
            // Driven through join() rather than given `foundingTitle` in props.
            // That property is readonly and derived; QML accepts a props value
            // as a readonly property's INITIAL value, which would build a state
            // Main.qml never produces and probe a screen that cannot exist.
            var addr = "b02d5e77" + "dd".repeat(28)
            Core.bridge = spec.bridgeFor({
                "join_stoa": '{"stoa":"' + addr
                    + '","foundingTitle":"Nym Research","policy":"open"}'
            })
            var screen = joinComponent.createObject(joinCell, {
                stoaAddress: addr,
                stoaGenesis: "00ff",
                width: stage.cellW
            })
            screen.join()

            compare(screen.joinState, "joined", "the fixture's own precondition")

            waitForRendering(stage)
            spec.assertContentPaints(joinCell, screen, "DJoinScreen")
            screen.destroy()
        }

        function test_an_onboarding_screen_showing_a_slate_paints_content() {
            Core.bridge = spec.bridgeFor({
                "generate_identity_slate":
                    '{"slate":"feed01","count":2,"candidates":['
                    + '{"index":0,"path":0,"publicKey":"' + "11".repeat(32) + '"},'
                    + '{"index":1,"path":1,"publicKey":"' + "22".repeat(32) + '"}'
                    + ']}'
            })
            var screen = onboardingComponent.createObject(onboardingCell, {
                stoaAddress: "ab".repeat(32),
                width: stage.cellW
            })
            // The opening state deliberately calls nothing, so the slate has to
            // be asked for.
            screen.requestSlate()

            compare(screen.phase, "slate", "the fixture's own precondition")
            compare(screen.candidates.length, 2,
                    "the screen must have candidates to paint")

            waitForRendering(stage)
            spec.assertContentPaints(onboardingCell, screen,
                                     "DOnboardingScreen")
            screen.destroy()
        }

        // ---- the demonstrations that this probe can fail -----------------
        //
        // THE TESTS THE REST OF THE FILE RESTS ON. A probe that cannot fail
        // passes exactly as quietly as a correct one, which is this project's
        // most frequently recorded defect.

        // (a) The scene-graph defect, on subjects differing ONLY in that.

        function test_the_healthy_twin_paints() {
            waitForRendering(stage)
            var img = spec.grabImage(healthyCell)
            var r = spec.sampleForDifference(img)
            verify(!r.flat,
                   "the healthy twin must paint, else the pair below "
                   + "demonstrates nothing: first=" + r.first)
            compare(healthyTwin.children.length, 2,
                    "the healthy twin's children must reach the scene graph — "
                    + "that is the ONLY thing separating it from the shadowed one")
            verify(r.sampled < 120 * 80,
                   "a paint must be found on the stride rather than by scanning "
                   + "every pixel: sampled " + r.sampled)
        }

        function test_the_shadowed_twin_is_flat_and_so_fails_the_probe() {
            // The sibling repo's real bug: `property var data` shadows Item's
            // default property, so the declared Rectangles never become
            // scene-graph children. `check_probe_twins.sh` pins that the two
            // twins differ by exactly this one line.
            waitForRendering(stage)
            compare(shadowedTwin.children.length, 0,
                    "the shadowed twin's children must NOT reach the scene "
                    + "graph — if this is non-zero the demonstration is not "
                    + "demonstrating anything")

            var img = spec.grabImage(shadowedCell)
            var r = spec.sampleForDifference(img)
            verify(r.flat,
                   "THE PROBE CANNOT FAIL. A subject painting nothing was "
                   + "reported as non-flat, first=" + r.first + ". Every "
                   + "assertion in this file is worthless until this passes.")
            compare(r.first, "#ffffff",
                    "a subject painting nothing leaves the window ground")
        }

        // (b) THE DEMONSTRATION THAT THE CONTENT INSET EARNS ITS KEEP.
        //
        // An empty ScreenFrame is the null implementation for the four screen
        // tests: a card that paints its chrome and holds NO content. It must be
        // reported as flat through the same region those tests use, and this
        // test pins BOTH halves — chrome non-flat, content flat — because the
        // first is what made the whole-card version of this probe a false green.

        function test_an_empty_card_paints_chrome_but_no_content() {
            var bare = bareFrameComponent.createObject(bareCell, {
                width: stage.cellW
            })
            waitForRendering(stage)

            // The chrome: grabbing the whole cell, an empty card is NOT flat.
            // This is the false green that nearly shipped — 38,487 differing
            // pixels from a card containing nothing.
            var whole = spec.sampleForDifference(spec.grabImage(bareCell))
            verify(!whole.flat,
                   "an empty card's own paper and border must still paint — if "
                   + "this is flat, the inset below is no longer distinguishing "
                   + "anything and this test has stopped being a demonstration")

            // And the content side. An empty card does not merely paint a flat
            // content area — it HAS NO CONTENT AREA AT ALL: 56px tall against
            // 68px of padding, so the rectangle the four screen tests sample
            // comes out 920x-12.
            //
            // Asserted as a negative height rather than as flatness, because
            // that is the stronger statement and the honest one: there is no
            // region to be flat. `probeContentOf`'s floor is what turns that
            // into a failure rather than a vacuous pass, and the four screen
            // tests inherit the same guard — which is what stops a screen that
            // collapsed to chrome from being reported as "flat, therefore
            // measured".
            var contentHeight = bare.height - 2 * stage.insetY
            verify(contentHeight <= 0,
                   "an empty card must have NO content area; got "
                   + contentHeight + "px, which means the inset is no longer "
                   + "clearing the card's padding and the four screen probes "
                   + "may be reading chrome")

            bare.destroy()
        }

        // (c) The stride is real, and the flat verdict scans rather than
        // short-circuits. Both directions of `sampleRegionForDifference`,
        // measured on a synthetic image so the expected counts are arithmetic
        // rather than whatever the screens happen to paint.

        function test_the_sampler_stops_early_on_paint_and_scans_when_flat() {
            var img = spec.grabImage(healthyCell)

            // Non-flat: the difference is found well before the whole region
            // has been sampled. 120x80 on a stride of 4 is 30*20 = 600 samples
            // if it ran to the end.
            var painted = spec.sampleRegionForDifference(img, 0, 0, 120, 80)
            verify(!painted.flat, "the healthy twin must paint")
            verify(painted.sampled < 600,
                   "a difference must stop the scan early: sampled "
                   + painted.sampled + " of 600")

            // Flat: a 1x1 region cannot differ from itself, and must report
            // exactly one sample — the arithmetic floor of the sampler.
            var single = spec.sampleRegionForDifference(img, 0, 0, 1, 1)
            verify(single.flat, "a single pixel cannot differ from itself")
            compare(single.sampled, 1,
                    "a 1x1 region is exactly one sample on any stride")

            // And the flat path genuinely visits the whole region: a 120x80
            // region of the SHADOWED twin, which paints nothing, must cost the
            // full 600.
            var blank = spec.sampleRegionForDifference(
                spec.grabImage(shadowedCell), 0, 0, 120, 80)
            verify(blank.flat, "the shadowed twin paints nothing")
            compare(blank.sampled, 600,
                    "a flat verdict must have sampled the whole region on the "
                    + "stride — 30 columns x 20 rows")
        }

        // ---- the probe's own floor ---------------------------------------

        function test_an_empty_cell_is_flat() {
            // The control that keeps the four screen tests honest. All four
            // read the same ink colour because they share DTheme's tokens —
            // which is what a common background painted behind every cell would
            // also look like. This says it is not one: a cell in the same grid
            // with nothing created into it is flat white.
            waitForRendering(stage)
            var r = spec.sampleForDifference(spec.grabImage(emptyControlCell))
            verify(r.flat,
                   "an EMPTY cell reported paint (" + r.first + "), so the four "
                   + "screen probes above may be reading something other than "
                   + "the screens themselves")
            compare(r.first, "#ffffff")
        }
    }

    Component { id: feedComponent; FeedScreen {} }
    Component { id: listComponent; DStoaListScreen {} }
    Component { id: joinComponent; DJoinScreen {} }
    Component { id: onboardingComponent; DOnboardingScreen {} }
    Component { id: bareFrameComponent; ScreenFrame {} }

    // One cell per subject, none overlapping any other, and none overlapping
    // the TestCase. See note 2 at the top: this layout IS the non-overlap
    // requirement, and moving a cell onto another one defeats the probe
    // silently.
    //
    // A cell holds ONLY its screen. The content area is reached by sampling a
    // sub-rectangle of the cell's grab (note 4) — deliberately NOT by nesting a
    // content Item here, which grabs empty and would make every screen test a
    // false red, then a false green once someone "fixed" it by dropping the
    // inset.
    Item { id: feedCell;       x: 0;    y: 0;    width: stage.cellW; height: stage.cellH }
    Item { id: listCell;       x: 1100; y: 0;    width: stage.cellW; height: stage.cellH }
    Item { id: joinCell;       x: 0;    y: 700;  width: stage.cellW; height: stage.cellH }
    Item { id: onboardingCell; x: 1100; y: 700;  width: stage.cellW; height: stage.cellH }

    // The empty-card demonstration's cell.
    Item { id: bareCell; x: 0; y: 1400; width: stage.cellW; height: 120 }

    // The demonstration pair. Two cells, side by side, neither over anything.
    Item {
        id: healthyCell
        x: 1100; y: 1400; width: 120; height: 80
        RenderProbeHealthy { id: healthyTwin; anchors.fill: parent }
    }
    Item {
        id: shadowedCell
        x: 1300; y: 1400; width: 120; height: 80
        RenderProbeShadowed { id: shadowedTwin; anchors.fill: parent }
    }

    // The control for test_an_empty_cell_is_flat: same grid, nothing in it.
    Item { id: emptyControlCell; x: 1500; y: 1400; width: 400; height: 80 }
}
