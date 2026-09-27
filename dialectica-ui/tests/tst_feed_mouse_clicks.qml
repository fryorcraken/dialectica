import QtQuick
import QtTest
import "../src/qml"

// `readThreadArea` and `moderateArea` clicked for real, with `mouseClick`
// against a shown window — not by calling the signal each one raises.
//
// `tasks.md`'s tests row said these two controls "genuinely have no
// component-test click" because `tst_navigation.qml` builds no window for
// `mouseClick` to act on. `findings/spec-test.md` box 1 challenged the
// "genuinely...no" framing: this suite already has the windowed recipe that
// would be needed — `tst_render_probe.qml` and `tst_screen_frame_geometry.qml`
// both build a `TestCase { when: windowShown }` and drive a real render pass
// — and nothing in `tasks.md` had tried it against these two `MouseArea`s.
// This file is that attempt, and it needed a THIRD data point about the
// recipe that neither of those two files states outright:
//
// **`TestCase` itself reports `visible: false`, even shown, even with
// `when: windowShown`** — measured here by walking the parent chain of a
// clicked item and printing each ancestor's own `visible` (Qt 6.10.3;
// `tst_zz_visible_probe.qml`, deleted after this was confirmed, pinned the
// underlying fact: a plain child Item's `visible` getter reflects its
// ancestor chain, not only its own local flag). `tst_screen_frame_geometry.qml`
// parents its subject DIRECTLY under `TestCase` and that is fine for the
// geometry it reads there — a layout polish pass runs for any item inside a
// rendered window regardless of the chain's own visible flag — but Qt Quick's
// input delivery DOES gate on effective visibility, so a `MouseArea` nested
// under an invisible `TestCase` never receives a synthesized click: it
// silently lands nowhere, and the screen stays exactly where it was.
//
// So this file follows `tst_render_probe.qml`'s shape instead of
// `tst_screen_frame_geometry.qml`'s: an `Item` is the file root, `TestCase` is
// a SIBLING parked clear of the subject, and `Main` is created into its own
// cell `Item` — never as a child of `TestCase`. That is what makes the two
// tests below able to receive a click at all, and it is why a lighter version
// of this file (tried first, kept only in this comment) failed both tests with
// `Main` reporting `visible: false` all the way up to the window.
//
// Every other navigation test in this suite (`tst_navigation.qml`) drives the
// transition by calling `feed.threadOpened(...)` or
// `feed.moderationRequested()` directly. That proves the WIRING from a signal
// to a screen transition. It cannot prove the MouseArea itself is reachable
// by a click — a `MouseArea` with the wrong `objectName`, the wrong
// `anchors.fill`, or sitting under something else that steals the event would
// pass every test in that file unchanged, because none of them touch the
// MouseArea. This file exists to close exactly that gap, by finding the real
// item in a real, shown window and clicking it.
Item {
    id: stage
    width: 1200
    height: 900

    // Parked well clear of `mainCell` below, per `tst_render_probe.qml`'s note
    // 2: a `TestCase` positioned over the subject would compete with it for
    // events and grabs, even though this file grabs no pixels.
    TestCase {
        id: spec
        name: "FeedMouseClicks"
        when: windowShown
        width: 150
        height: 100
        x: 1020
        y: 780

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

        // Depth-first search by `objectName`, same shape as
        // `tst_screen_frame_geometry.qml`'s `findByName` — this file needs
        // the same walk, not a variant of it.
        function findByName(item, name) {
            if (item === null || item === undefined)
                return null
            if (item.objectName === name)
                return item
            var kids = item.children
            if (kids === undefined)
                return null
            for (var i = 0; i < kids.length; ++i) {
                var hit = findByName(kids[i], name)
                if (hit !== null)
                    return hit
            }
            return null
        }

        readonly property string stoaA:
            "aaaaaaaa11111111111111111111111111111111111111111111111111111111"
        readonly property string keyA:
            "k:0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
        readonly property string recordA: "beef"

        readonly property string oneStoa:
            '{"items":[{"stoa":"' + spec.stoaA + '","foundingTitle":"Nym Research"}],'
            + '"page":0,"hasMore":false}'

        // ---- readThreadArea -------------------------------------------
        //
        // A row naming a thread, so `readThreadArea` is rendered and its
        // `visible target !== ""` guard is open (`FeedScreen.qml`'s
        // `threadTarget`).
        function test_a_real_click_on_readThreadArea_opens_that_rows_thread() {
            var op = "cc" + "11".repeat(31)
            Core.bridge = spec.bridgeFor({
                "list_stoas": spec.oneStoa,
                "list_threads": '{"items":[{"thread":"' + op
                    + '","currentVersion":"' + op
                    + '","author":"' + spec.keyA + '","body":{"text":"hi"},'
                    + '"attachments":[],"isRevised":false,"isHidden":false}],'
                    + '"page":0,"hasMore":false}',
                "read_thread": '{"items":[],"page":0,"hasMore":false}',
                "who_am_i": '{"hasIdentity":false,"reason":"none"}',
                "get_capabilities": '{"canPost":false,"reason":"none"}'
            })
            var view = mainComponent.createObject(mainCell,
                { width: 1000, height: 800 })
            view.open(spec.stoaA, "Nym Research", spec.recordA)
            waitForRendering(view)

            var area = spec.findByName(view, "readThreadArea")
            verify(area !== null,
                   "readThreadArea must exist on a feed with a row naming "
                   + "a thread — if this is null the click below tests "
                   + "nothing")
            verify(area.width > 0 && area.height > 0,
                   "readThreadArea must have real, rendered geometry for "
                   + "mouseClick to land on — got " + area.width + "x"
                   + area.height)
            verify(area.visible,
                   "readThreadArea must be effectively visible (its own "
                   + "flag and every ancestor's) or a real click cannot "
                   + "reach it")

            compare(view.screenShown, "feed", "precondition: still on the feed")

            mouseClick(area)

            compare(view.screenShown, "thread",
                    "a real click on readThreadArea must open the thread "
                    + "its row names — this is checking the MouseArea and "
                    + "its handler are reachable, not merely that "
                    + "threadOpened() is wired to openThread()")
            view.destroy()
        }

        // ---- moderateArea -----------------------------------------------
        //
        // Rendered unconditionally in the feed's header (`FeedScreen.qml`'s
        // header RowLayout carries no `visible` guard on `moderateLink`), so
        // no row or read state is needed for the control to exist.
        function test_a_real_click_on_moderateArea_opens_the_moderation_screen() {
            Core.bridge = spec.bridgeFor({
                "list_stoas": spec.oneStoa,
                "list_threads": '{"items":[],"page":0,"hasMore":false}',
                "who_am_i": '{"hasIdentity":false,"reason":"none"}',
                "get_capabilities": '{"canPost":false,"reason":"none"}'
            })
            var view = mainComponent.createObject(mainCell,
                { width: 1000, height: 800 })
            view.open(spec.stoaA, "Nym Research", spec.recordA)
            waitForRendering(view)

            var area = spec.findByName(view, "moderateArea")
            verify(area !== null,
                   "moderateArea must exist on the feed's header — if this "
                   + "is null the click below tests nothing")
            verify(area.width > 0 && area.height > 0,
                   "moderateArea must have real, rendered geometry for "
                   + "mouseClick to land on — got " + area.width + "x"
                   + area.height)
            verify(area.visible,
                   "moderateArea must be effectively visible (its own flag "
                   + "and every ancestor's) or a real click cannot reach it")

            compare(view.screenShown, "feed", "precondition: still on the feed")

            mouseClick(area)

            compare(view.screenShown, "moderation",
                    "a real click on moderateArea must open the moderation "
                    + "screen for the Stoa the feed is showing — this is "
                    + "checking the MouseArea and its handler are "
                    + "reachable, not merely that moderationRequested() is "
                    + "wired to moderateIn()")
            view.destroy()
        }
    }

    Component { id: mainComponent; Main {} }

    // The one cell `Main` is created into, each test destroying its own
    // instance before the next runs. Sized to give `Main`'s
    // `Layout.preferredWidth: Math.min(DTheme.cardWidth, root.width - ...)`
    // bindings (`Main.qml`) room, the same 1000x800 `tst_thread_navigation.qml`
    // and `tst_screen_frame_geometry.qml` already use.
    Item { id: mainCell; x: 0; y: 0; width: 1000; height: 800 }
}
