import QtQuick
import QtTest
import "../src/qml"

// Nesting, and the one property this screen exists to protect.
//
// **An item whose parent is not on the page must not be rendered as a direct
// reply to the root.** `thread.rs` derives thread membership from the parent
// chain precisely so an authentically-signed post cannot inject itself into a
// conversation by naming one, and returns a post whose chain cannot be completed
// under NO thread rather than placed by its claim. A view that re-parented such
// an item to the root would hand the attacker, at the render step, the placement
// core refused.
//
// These drive the real DThreadScreen through a fake bridge, so what is under
// test is the screen's own arithmetic rather than a re-implementation of it.
TestCase {
    id: spec
    name: "ThreadNesting"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
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

    // `stoaAddress` and `threadId` must both be non-empty or reload()
    // short-circuits before it reaches the bridge — which would make every test
    // below pass for the wrong reason.
    function makeScreen(replies) {
        Core.bridge = bridgeFor(replies)
        return threadComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            stoaTitle: "Agora",
            threadId: "root1"
        })
    }

    // A thread reply built from items, so each test states only the shape it is
    // about. `perPage`/`page` are core's; the view sends neither.
    function threadOf(items) {
        return JSON.stringify({ items: items, page: 0, hasMore: false })
    }

    // The root: the item reporting NO parent at all. `parent` is omitted rather
    // than nulled on the wire, and the distinction between "reports no parent"
    // and "reports a parent we cannot find" is the whole of what is tested here.
    function rootItem(id) {
        return {
            thread: id, id: id, currentVersion: id,
            author: "aa".repeat(32), isRevised: false,
            moderation: { state: "unmoderated" },
            position: "0",
            body: { text: "the root post", removed: 0, marked: 0 }
        }
    }

    function replyItem(id, parent, position) {
        return {
            thread: "root1", id: id, currentVersion: id, parent: parent,
            author: "bb".repeat(32), isRevised: false,
            moderation: { state: "unmoderated" },
            position: position,
            body: { text: "a reply", removed: 0, marked: 0 }
        }
    }

    Component {
        id: threadComponent
        DThreadScreen {}
    }

    // ---- the root is not a missing-parent case --------------------------

    function test_the_item_reporting_no_parent_is_the_root_at_depth_zero() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1")])
        })

        compare(screen.resolveDepth(screen.items[0]), 0,
                "the item reporting no parent is the root, at the outermost depth")
        verify(screen.resolveDepth(screen.items[0]) >= 0,
               "the root must never be the unresolvable state")
        screen.destroy()
    }

    // ---- ordinary nesting ------------------------------------------------

    function test_a_reply_is_one_deeper_than_the_root_it_names() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1"),
                                     replyItem("r2", "root1", "1")])
        })

        compare(screen.resolveDepth(screen.items[1]), 1)
        screen.destroy()
    }

    function test_a_reply_to_a_reply_is_deeper_than_its_own_parent() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1"),
                                     replyItem("r2", "root1", "1"),
                                     replyItem("r3", "r2", "2")])
        })

        var parentDepth = screen.resolveDepth(screen.items[1])
        var childDepth = screen.resolveDepth(screen.items[2])
        verify(childDepth > parentDepth,
               "a reply to a reply renders deeper than the item it names as parent")
        compare(childDepth, 2)
        screen.destroy()
    }

    // ---- THE SECURITY PROPERTY -------------------------------------------

    function test_an_item_whose_parent_is_absent_is_not_re_parented_to_the_root() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            // `ghost` is on no page the view holds. Core sent the item anyway,
            // because a parent is an op id and not a promise the parent is among
            // the items — it may be on an earlier page, or hidden.
            "read_thread": threadOf([rootItem("root1"),
                                     replyItem("orphan", "ghost", "1")])
        })

        var depth = screen.resolveDepth(screen.items[1])

        // The assertion that matters, and it is stated as a DENIAL of the
        // specific wrong answer rather than only as an equality: depth 0 is
        // where a re-parenting implementation would put it, and 1 is where a
        // "default to one below the root" implementation would.
        verify(depth !== 0,
               "an unresolvable parent must not render the item at the root's depth")
        verify(depth !== 1,
               "an unresolvable parent must not render the item as a direct reply to the root")
        compare(depth, -1,
                "the view reports that it cannot establish a depth, which is a "
                + "renderable state and not a failure")
        screen.destroy()
    }

    function test_the_other_items_still_render_when_one_parent_is_unresolvable() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1"),
                                     replyItem("orphan", "ghost", "1"),
                                     replyItem("r3", "root1", "2")])
        })

        compare(screen.readState, "ok",
                "an item the view cannot place does not fail the read that returned it")
        compare(screen.items.length, 3, "and it does not omit any item")
        compare(screen.resolveDepth(screen.items[2]), 1,
                "a sibling with a resolvable parent is unaffected")
        screen.destroy()
    }

    // ---- termination over peer-supplied parents --------------------------
    //
    // Parent references arrive from peers, so a chain that could be made not to
    // terminate would turn one malformed item into a hang in the view. The walk
    // carries a visited set, which `thread.rs` chose over a depth limit for the
    // same reason.

    function test_an_item_naming_itself_as_its_parent_terminates() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1"),
                                     replyItem("loop", "loop", "1")])
        })

        compare(screen.resolveDepth(screen.items[1]), -1,
                "an item inside a cycle has no establishable depth")
        screen.destroy()
    }

    function test_a_parent_cycle_of_two_terminates() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1"),
                                     replyItem("a", "b", "1"),
                                     replyItem("b", "a", "2")])
        })

        compare(screen.resolveDepth(screen.items[1]), -1)
        compare(screen.resolveDepth(screen.items[2]), -1)
        screen.destroy()
    }

    // A cycle must reach the SAME state as an absent parent, and not depth 0 —
    // otherwise the security property above is undone by a second route.
    function test_a_cycle_is_not_rendered_at_the_roots_depth() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1"),
                                     replyItem("loop", "loop", "1")])
        })

        verify(screen.resolveDepth(screen.items[1]) !== 0,
               "a cycle must not collapse to the root's depth")
        screen.destroy()
    }

    // ---- the indent is bounded, and bounds only the indent ---------------

    function test_a_deep_chain_stops_indenting_but_keeps_every_item() {
        var items = [rootItem("root1")]
        var parent = "root1"
        // Ten deep, comfortably past the default bound of six.
        for (var i = 1; i <= 10; i++) {
            items.push(replyItem("d" + i, parent, String(i)))
            parent = "d" + i
        }

        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf(items)
        })

        compare(screen.items.length, 11, "every item of the chain is rendered")

        // The computed DEPTH keeps growing — the bound is on pixels only.
        compare(screen.resolveDepth(screen.items[10]), 10,
                "the bound clamps the indent, never the computed depth")

        // The INDENT stops.
        var deepest = screen.indentFor(screen.resolveDepth(screen.items[10]))
        var atBound = screen.indentFor(screen.maxIndentDepth)
        compare(deepest, atBound, "no item is indented past the bound")
        verify(deepest <= screen.maxIndentDepth * screen.indentStep)
        screen.destroy()
    }

    function test_the_sequence_rendered_is_the_sequence_returned() {
        // A page whose returned order disagrees with what sorting by position
        // would produce: the view must leave it alone.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1"),
                                     replyItem("later", "root1", "9"),
                                     replyItem("earlier", "root1", "2")])
        })

        compare(screen.items[1].id, "later",
                "the returned sequence is preserved even where positions disagree with it")
        compare(screen.items[2].id, "earlier")
        screen.destroy()
    }

    // `position` is an opaque ordering token, not a quantity. A page whose
    // positions are not decimal numerals must render without the screen failing
    // — which is what proves nothing parsed them.
    function test_positions_that_are_not_numerals_are_not_parsed() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1"),
                                     replyItem("r2", "root1", "zz-opaque-zz")])
        })

        compare(screen.readState, "ok",
                "an opaque position is rendered, not parsed")
        compare(screen.items.length, 2)
        compare(screen.resolveDepth(screen.items[1]), 1,
                "and nesting is unaffected by it, being computed from parents")
        screen.destroy()
    }

    // ---- an item carrying no id --------------------------------------------
    //
    // Peer-supplied items are not validated element-by-element on this path. Two
    // items missing `id` would otherwise key the parent map on the JavaScript
    // value `undefined`, which stringifies to the single key "undefined" — so
    // each would resolve as the other's parent.
    function test_items_carrying_no_id_do_not_share_a_parent_slot() {
        var a = replyItem("", "ghost", "1")
        delete a.id
        var b = replyItem("", "ghost", "2")
        delete b.id

        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"no keystore"}',
            "read_thread": threadOf([rootItem("root1"), a, b])
        })

        compare(screen.itemId(screen.items[1]), "",
                "an item with no id yields no key rather than the string 'undefined'")
        compare(screen.resolveDepth(screen.items[1]), -1)
        compare(screen.resolveDepth(screen.items[2]), -1)
        screen.destroy()
    }
}
