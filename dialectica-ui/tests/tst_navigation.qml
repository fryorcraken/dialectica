import QtQuick
import QtTest
import "../src/qml"

// The navigator's transitions: which screen is shown, what travels across each
// move, and that every state a user can enter can be left.
//
// **These tests drive `Main.qml`, not a screen.** That is the point of the file:
// a component suite instantiates the component it tests and therefore supplies
// the reachability whose absence was the defect — 92 test functions passed over
// four screens no user could open. Driving the root is what makes "the user got
// there" a thing a test can observe at all.
//
// It still cannot discharge the reachability REQUIREMENT, and does not claim to.
// A type this file forgets to look for is indistinguishable from one nobody
// thought of; enumerating the registrations is `check_qml_reachable.py`'s, and
// that gate reads `qmldir` rather than asserting against a tree it built.
TestCase {
    id: spec
    name: "Navigation"

    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    // The house fake: answers each method from a map and RECORDS every call.
    //
    // The recording half is what lets a test assert a call was NOT made, which
    // several assertions below turn on — "returned without keeping anything" and
    // "made no keep call" are the same claim, and only the call log shows it.
    function bridgeFor(replies) {
        return {
            calls: [],
            callModule: function (module, method, args) {
                this.calls.push({ method: method, args: args })
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
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

    // Visibility checked on every ancestor, because QML's `visible` is not
    // inherited into the property: an element inside a hidden panel still
    // reports `visible: true` for itself, and an absence assertion that missed
    // that would pass on a screen showing the affordance.
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

    Component { id: mainComponent; Main {} }

    readonly property string stoaA:
        "aaaaaaaa11111111111111111111111111111111111111111111111111111111"
    readonly property string keyA:
        "k:0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"

    // A listing holding one Stoa, so the list screen has a row to open.
    readonly property string oneStoa:
        '{"items":[{"stoa":"' + stoaA + '","foundingTitle":"Nym Research"}],'
        + '"page":0,"hasMore":false}'

    // ---- the routing state reads BOTH probes ----------------------------

    // The load-bearing case. `who_am_i` says there IS an identity and
    // `get_capabilities` says posting is not possible — the two honestly
    // disagreeing, which `lib.rs:258` contracts they can. Routing on the posting
    // probe alone would collapse this into "there is nobody here" and offer key
    // CREATION, which for an existing identity is irreversible.
    function test_an_unusable_identity_is_not_offered_identity_creation() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":true,"publicKey":"' + spec.keyA
                        + '","recoveryNeedsTheRecord":false}',
            "get_capabilities": '{"canPost":false,"reason":"the keystore is readable by others"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")

        var feed = spec.namedAnywhere(view, "feed")[0]
        compare(feed.hasIdentity, true,
                "who_am_i reported an identity, so the view has one")
        compare(feed.capability.canPost, false,
                "and the posting probe says it cannot currently be used")

        // The chip is bound to the identity report, so it renders the
        // identity-present arm and offers no creation route.
        var chip = spec.namedAnywhere(view, "identityChip")[0]
        compare(chip.hasIdentity, true,
                "the chip must not offer key creation to a user who HAS a key — "
                + "keeping is refused where an identity exists, and replacing "
                + "one discards every identity derived from it")
        view.destroy()
    }

    function test_no_identity_at_all_does_offer_identity_creation() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"no keystore on this machine"}',
            "get_capabilities": '{"canPost":false,"reason":"no keystore on this machine"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")

        var chip = spec.namedAnywhere(view, "identityChip")[0]
        compare(chip.hasIdentity, false)
        view.destroy()
    }

    // Both probes, every render, neither answered from a retained value.
    function test_both_probes_are_re_read_on_every_render() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")

        var feed = spec.namedAnywhere(view, "feed")[0]
        var whoFirst = spec.callsTo(bridge, "who_am_i")
        var capFirst = spec.callsTo(bridge, "get_capabilities")
        verify(whoFirst > 0, "the identity report was asked at least once")
        verify(capFirst > 0, "the posting probe was asked at least once")

        feed.reload()

        compare(spec.callsTo(bridge, "who_am_i"), whoFirst + 1,
                "the identity report is asked again rather than answered from "
                + "the earlier render — a retained answer outlives the keystore "
                + "it stands for")
        compare(spec.callsTo(bridge, "get_capabilities"), capFirst + 1,
                "and so is the posting probe")
        view.destroy()
    }

    // A probe answering something that is not `true` must not read as an
    // identity. `"true"`, `1` and `null` are each truthy-or-falsy in a way that
    // does not match what they mean.
    function test_a_degenerate_identity_reply_claims_no_identity() {
        var shapes = ['{"hasIdentity":"true"}', '{"hasIdentity":1}',
                      '{"hasIdentity":null}', '{}']
        for (var i = 0; i < shapes.length; i++) {
            Core.bridge = spec.bridgeFor({
                "list_stoas": spec.oneStoa,
                "list_threads": '{"items":[],"page":0,"hasMore":false}',
                "who_am_i": shapes[i],
                "get_capabilities": '{"canPost":false,"reason":"none"}'
            })
            var view = mainComponent.createObject(null, {})
            view.open(spec.stoaA, "Nym Research", "")
            var feed = spec.namedAnywhere(view, "feed")[0]
            compare(feed.hasIdentity, false,
                    "reply " + shapes[i] + " must not claim an identity")
            view.destroy()
        }
    }

    // ---- acquiring an identity: the route to the Stoa list ----------------
    //
    // `view-navigation`, "Acquiring an identity is reached from the navigator".
    // In this release the identity is this machine's key, created on the Stoa
    // list; the per-Stoa onboarding screen these tests used to drive is not
    // reachable (issue #149, `machine-identity-scope`).

    // Driven through the CHIP's own signal, not `acquireIdentity()`, because the
    // requirement is about the affordance a user acts on: a navigator function
    // nothing wires to the chip would pass a test that called it directly.
    //
    // **The last assertion is a composition of two specs, and the fixture has
    // to answer both.** `view-navigation` requires only that the route renders
    // the list; what the list then draws is `stoa-navigation-view`'s, and there
    // the create-key action exists only in the no-key state, entered on
    // `get_master_key` answering `hasMasterKey:false` and on nothing else. So
    // this fixture states the master-key answer that agrees with its own
    // `who_am_i` — no keystore on this machine — and the claim is: a feed that
    // reports no identity, on a machine holding no key, lands the user where
    // the key is made.
    //
    // Without the `get_master_key` reply the fake answers that method with the
    // error shape, the list is in its could-not-be-read state, and no create-key
    // action exists — correctly, since a key may be held and unreadable. That is
    // how this test went red when it first met the key-state list: it was
    // written against a list that drew `createIdentityButton` unconditionally.
    function test_a_missing_identity_offers_the_route_to_the_stoa_list() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"no keystore on this machine"}',
            "get_capabilities": '{"canPost":false,"reason":"no keystore on this machine"}',
            "get_master_key": '{"hasMasterKey":false}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")
        compare(view.screenShown, "feed")

        var chip = spec.visibleNamed(view, "identityChip")
        compare(chip.length, 1, "the feed offers the identity chip")
        compare(chip[0].hasIdentity, false,
                "and it is in its no-identity arm, the one carrying the route")
        chip[0].createRequested()

        compare(view.screenShown, "list",
                "acting on the affordance renders the Stoa list, where this "
                + "machine's key is created")
        compare(spec.visibleNamed(view, "stoaList").length, 1,
                "and the list is what is on screen")
        compare(spec.visibleNamed(view, "createKeyButton").length, 1,
                "with the control that creates the key reachable on it, the "
                + "master-key query having reported no key held")
        view.destroy()
    }

    // "Following the route asks the module for nothing." The house fake
    // RECORDS, so a zero count below is "no call was made" and not "a call was
    // made and ignored" — and the fake answers all three methods, so a call
    // that WAS made would not have failed quietly on a missing reply.
    function test_following_the_route_creates_no_key_requests_no_slate_and_keeps_nothing() {
        var bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}',
            "create_identity": '{"publicKey":"' + spec.keyA
                               + '","encrypted":false,"wasNew":true}',
            "generate_identity_slate":
                '{"slate":"ab","count":1,"candidates":[{"index":0,"publicKey":"'
                + spec.keyA + '"}]}',
            "keep_identity": '{"kept":true,"publicKey":"' + spec.keyA
                             + '","path":0,"encrypted":false}',
            "get_master_key": '{"hasMasterKey":false}'
        })
        Core.bridge = bridge
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "")
        verify(bridge.calls.length > 0,
               "the feed did call the core, so the zeros below are this route "
               + "making no call rather than the fake being unreachable")

        var keyQueriesBefore = spec.callsTo(bridge, "get_master_key")
        spec.namedAnywhere(view, "identityChip")[0].createRequested()

        compare(view.screenShown, "list")

        // NO SPEC: `view-navigation` says following the route "MUST NOT itself
        // reach the module", and its scenario enumerates three calls — key
        // creation, slate, keep. `stoa-navigation-view` requires the list to
        // ask the read-only master-key query "each time it is shown after being
        // hidden", and the route is such a showing. So landing makes exactly one
        // `get_master_key` call. This pins it as the LIST's call rather than the
        // route's; whether the prose meant to forbid it too is reported to the
        // spec-writer, not decided here. Measured: an equality with
        // `keyQueriesBefore` fails, 2 against 1.
        compare(spec.callsTo(bridge, "get_master_key"), keyQueriesBefore + 1,
                "the list's showing asks the key state once, and mints nothing")
        compare(spec.callsTo(bridge, "create_identity"), 0,
                "the route created a key on the user's behalf")
        compare(spec.callsTo(bridge, "generate_identity_slate"), 0,
                "the route requested a slate")
        compare(spec.callsTo(bridge, "keep_identity"), 0,
                "the route kept a candidate")
        view.destroy()
    }

    // "The per-Stoa onboarding screen is instantiated nowhere the view's root
    // reaches." Asserted on the object TREE by type, not by `objectName`:
    // deleting the name from a still-mounted screen would satisfy a name search,
    // and the requirement is about instantiation. The registration half — the
    // `# UNINSTANTIATED:` record — is `check_qml_reachable.py`'s, which reads
    // `qmldir` rather than a tree it built.
    function test_the_per_stoa_onboarding_screen_is_instantiated_nowhere() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})

        function instancesOf(typeName) {
            var found = 0
            function walk(node) {
                if (!node)
                    return
                if (String(node).indexOf(typeName + "_QMLTYPE") === 0
                        || String(node).indexOf(typeName + "(") === 0)
                    found++
                var kids = node.children
                if (kids !== undefined)
                    for (var i = 0; i < kids.length; i++)
                        walk(kids[i])
            }
            walk(view)
            return found
        }

        // The walk can see a screen by type at all, or the zero below proves
        // nothing: the feed screen is mounted, so it must be found.
        verify(instancesOf("FeedScreen") > 0,
               "the type walk found no FeedScreen, so it cannot see screens")

        // Every state the navigator can enter, including the one the route
        // lands in, so a screen mounted only on some path is still caught.
        view.open(spec.stoaA, "Nym Research", "beef")
        view.openThread("cc" + "11".repeat(31))
        view.moderateIn(spec.stoaA, "Nym Research", "beef")
        view.acquireIdentity()
        compare(instancesOf("DOnboardingScreen"), 0,
                "the per-Stoa onboarding screen is mounted in 0.0.1")
        view.destroy()
    }

    // ---- the thread route -----------------------------------------------

    function test_a_feed_row_opens_its_thread_with_the_stoa_and_the_root_op() {
        // **A REVISED row: the two identifiers differ, and that is the point.**
        //
        // This fixture used to carry `currentVersion` alone, which is a row core
        // never sends — `wire.rs:6015-6027` pins the feed row's whole key set as
        // a SET (`attachments, author, body, currentVersion, isHidden,
        // isRevised, thread`), so a row is never short of `thread` and never
        // carries an `id`. With one identifier present the test could not tell
        // the two apart, and the route was built on the wrong one.
        //
        // Giving them DIFFERENT values is what makes the assertion below
        // discriminating: opening by `currentVersion` now fails it rather than
        // passing by coincidence. `feed.rs:130-140` is the contract — `thread`
        // is the root post's op id and "never changes across edits", where
        // `current_version` is "different from `thread` the moment the post has
        // been edited".
        var op = "cc" + "11".repeat(31)
        var editedVersion = "dd" + "22".repeat(31)
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[{"thread":"' + op
                + '","currentVersion":"' + editedVersion
                + '","author":"' + spec.keyA + '","body":{"text":"hi"},'
                + '"attachments":[],"isRevised":true,"isHidden":false}],'
                + '"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")

        var link = spec.visibleNamed(view, "readThreadLink")
        compare(link.length, 1, "the row offers a route into its thread")

        // **What the ROW resolved, not a value the test chose.** Raising
        // `feed.threadOpened(op)` with the op in hand would assert the
        // navigator's plumbing while proving nothing about which field the row
        // read — and the field is the whole question here. So the row's own
        // target is asserted, and it is that value which is then sent.
        compare(link[0].target, op,
                "the row opens its thread by the ROOT POST's id, not by the "
                + "version — these differ because this post was revised")

        var feed = spec.namedAnywhere(view, "feed")[0]
        feed.threadOpened(link[0].target)

        compare(view.screenShown, "thread")
        var args = String(spec.lastArgsTo(bridge, "read_thread"))
        verify(args.indexOf('"stoa":"' + spec.stoaA + '"') >= 0,
               "the thread is given the row's Stoa")
        verify(args.indexOf('"thread":"' + op + '"') >= 0,
               "and the ROOT POST's identifier, which does not move when the "
               + "post is edited")
        verify(args.indexOf(editedVersion) < 0,
               "and NOT the current version, which moves under an edit and "
               + "would leave the route pointing at a thread that stops "
               + "answering to it: " + args)
        verify(args.indexOf('"genesis":"beef"') >= 0,
               "and the founding record the view holds for that Stoa")
        view.destroy()
    }

    // Where the view holds no record, the thread is opened WITHOUT one rather
    // than with a fabricated or placeholder one. A fabricated record fails
    // verification in the core and surfaces as a refusal the user cannot act on.
    function test_no_record_is_invented_for_a_stoa_the_view_has_none_for() {
        var op = "cc" + "11".repeat(31)
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var bridge = Core.bridge
        var view = mainComponent.createObject(null, {})
        // Opened with NO genesis — the listing does not return one.
        view.open(spec.stoaA, "Nym Research", "")

        var feed = spec.namedAnywhere(view, "feed")[0]
        feed.threadOpened(op)

        var args = String(spec.lastArgsTo(bridge, "read_thread"))
        verify(args.indexOf('"genesis":""') >= 0,
               "the empty record travels as empty — the view neither invents "
               + "one nor substitutes a placeholder. What core then refuses is "
               + "rendered as the refusal it is: " + args)
        view.destroy()
    }

    function test_the_feed_is_reached_again_from_the_thread() {
        var op = "cc" + "11".repeat(31)
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")
        var feed = spec.namedAnywhere(view, "feed")[0]
        feed.threadOpened(op)
        compare(view.screenShown, "thread")

        var back = spec.visibleNamed(view, "threadBackButton")
        compare(back.length, 1, "the thread offers a way back to the feed")
        back[0].clicked()

        compare(view.screenShown, "feed")
        compare(feed.stoaAddress, spec.stoaA,
                "and it is the feed for the Stoa the thread was opened from, "
                + "without the view being restarted")
        view.destroy()
    }

    // The way out of a thread whose read FAILED. That is when a user most wants
    // out, so the control must not be withdrawn by the state it exists to leave.
    function test_a_thread_that_could_not_be_read_still_offers_the_way_back() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"error":"no root held for that thread"}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")
        spec.namedAnywhere(view, "feed")[0].threadOpened("cc" + "11".repeat(31))

        var thread = spec.namedAnywhere(view, "thread")[0]
        compare(thread.readState, "failed")
        compare(spec.visibleNamed(view, "threadBackButton").length, 1,
                "a failed read is exactly when the way out must be there")
        view.destroy()
    }

    // ---- the moderation route --------------------------------------------

    // The screen exists so the owner can SEE it, so a route to it is the whole
    // point rather than a detail. This drives the affordance a user acts on,
    // not `moderateIn` directly — the static gate proves the type is
    // instantiated, and only this shows that pressing something reaches it.
    function test_the_feed_offers_a_route_into_moderation() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")

        var link = spec.visibleNamed(view, "moderateLink")
        compare(link.length, 1, "the feed offers the route")
        spec.namedAnywhere(view, "feed")[0].moderationRequested()

        compare(view.screenShown, "moderation")
        compare(spec.visibleNamed(view, "moderation").length, 1)
        var screen = spec.namedAnywhere(view, "moderation")[0]
        compare(screen.stoaAddress, spec.stoaA,
                "and the Stoa travels, so the way back lands on its feed")
        view.destroy()
    }

    // **The assertion this file exists to make about this screen.** Every
    // control is inert, and the only way to show that from outside is to press
    // them and observe that the call log did not grow.
    //
    // A fake that answered every method identically could not tell "no call was
    // made" from "a call was made and ignored"; the house fake RECORDS, so the
    // count is what distinguishes them.
    function test_nothing_on_the_moderation_screen_calls_the_core() {
        var bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        Core.bridge = bridge
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")
        view.moderateIn(spec.stoaA, "Nym Research", "beef")

        var before = bridge.calls.length
        verify(before > 0, "the feed did call the core, so a zero below means "
               + "this screen made none rather than the fake being unreachable")

        var inert = ["markModeratedButton", "moderateAuthorButton",
                     "unmoderateAuthorButton", "unmoderatePostButton"]
        for (var i = 0; i < inert.length; i++) {
            var buttons = spec.visibleNamed(view, inert[i])
            verify(buttons.length > 0, inert[i] + " is on the screen")
            for (var j = 0; j < buttons.length; j++)
                buttons[j].clicked()
        }

        compare(bridge.calls.length, before,
                "pressing every inert control made no call")
        compare(view.screenShown, "moderation",
                "and none of them navigated anywhere either")
        view.destroy()
    }

    // The way out, and that acting on an inert control does not withdraw it.
    // Sharper here than on any other screen: every other control does nothing,
    // so this is the only one that answers a press at all.
    function test_the_moderation_screen_can_be_left_after_pressing_its_controls() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")
        view.moderateIn(spec.stoaA, "Nym Research", "beef")

        spec.visibleNamed(view, "markModeratedButton")[0].clicked()

        var back = spec.visibleNamed(view, "moderationBackButton")
        compare(back.length, 1, "the way out survives an inert press")
        back[0].clicked()

        compare(view.screenShown, "feed")
        compare(spec.namedAnywhere(view, "feed")[0].stoaAddress, spec.stoaA,
                "and it is the feed the screen was entered from")
        view.destroy()
    }

    // Cancel is the second way out, and it must work for the same reason: it is
    // the control a user reaches for once they have decided against acting, and
    // a cancel that did nothing would strand them on a destructive screen.
    function test_cancel_leaves_the_moderation_screen() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        view.open(spec.stoaA, "Nym Research", "beef")
        view.moderateIn(spec.stoaA, "Nym Research", "beef")

        spec.visibleNamed(view, "cancelButton")[0].clicked()
        compare(view.screenShown, "feed")
        view.destroy()
    }

    // ---- exactly one screen, from one source -----------------------------

    function test_exactly_one_screen_is_shown_in_every_reachable_state() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        var names = ["stoaList", "joinScreen", "feed", "thread", "moderation"]

        function shownCount() {
            var n = 0
            for (var i = 0; i < names.length; i++)
                n += spec.visibleNamed(view, names[i]).length
            return n
        }

        compare(shownCount(), 1, "the list alone, at startup")

        view.preview(spec.stoaA, "beef")
        compare(shownCount(), 1, "the join preview alone")

        view.open(spec.stoaA, "Nym Research", "beef")
        compare(shownCount(), 1, "the feed alone")

        view.openThread("cc" + "11".repeat(31))
        compare(shownCount(), 1, "the thread alone")

        view.moderateIn(spec.stoaA, "Nym Research", "beef")
        compare(shownCount(), 1, "the moderation screen alone")

        view.acquireIdentity()
        compare(shownCount(), 1, "the list alone, reached by the identity route")
        view.destroy()
    }

    // The screen-name list above is hand-written, which is this repo's
    // `hand-maintained sweep lists go stale silently` trap: a seventh screen
    // would be added to `Main.qml` with that walk still passing over six.
    //
    // This pins the list against the navigator's own `stateNames`, which is the
    // thing a new state must be added to for the navigator to work at all. It is
    // off by one on purpose and the test says why: `stateNames` holds the four
    // states that carry a payload, and the list screen is the fifth rendering —
    // the one shown when every state is null, so it has no entry to hold. It was
    // five and six until `machine-identity-scope` removed `onboarding`.
    function test_the_screen_walk_covers_every_state_the_navigator_has() {
        Core.bridge = spec.bridgeFor({ "list_stoas": spec.oneStoa })
        var view = mainComponent.createObject(null, {})
        compare(view.stateNames.length + 1, 5,
                "a state added to the navigator without a screen added to the "
                + "walk above leaves that screen unchecked")
        view.destroy()
    }

    // Each transition clears what the others own, so the state in which two are
    // set — which has no rendering, and which an ordered ternary would resolve
    // by accident of which test came first — cannot be constructed.
    function test_no_transition_leaves_two_states_set_at_once() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})

        // DERIVED from the navigator's own state list, not restated. The
        // restated version counted four states and a fifth was added to
        // `Main.qml` with this test still passing over four — it would have
        // reported "one state set" while `moderating` sat set beside it. A
        // sweep list written out by hand goes stale silently, and this is the
        // test whose whole subject is that no second state is set.
        function setCount() {
            var n = 0
            for (var i = 0; i < view.stateNames.length; i++)
                if (view[view.stateNames[i]] !== null)
                    n++
            return n
        }

        verify(view.stateNames.length >= 4)

        view.preview(spec.stoaA, "beef");            compare(setCount(), 1)
        view.open(spec.stoaA, "Nym Research", "beef"); compare(setCount(), 1)
        view.openThread("cc" + "11".repeat(31));     compare(setCount(), 1)
        view.closeThread();                          compare(setCount(), 1)
        view.acquireIdentity();                      compare(setCount(), 0)
        view.open(spec.stoaA, "Nym Research", "beef"); compare(setCount(), 1)
        view.moderateIn(spec.stoaA, "T", "");        compare(setCount(), 1)
        view.closeModeration();                      compare(setCount(), 1)
        view.closeFeed();                            compare(setCount(), 0)
        view.destroy()
    }

    // The reshape that made `enterOnly` the one transition primitive removed the
    // hand-written clears, each of which had to name every sibling state. Two of
    // them (`openThread`, and the identity route) had been written naming only
    // three of the four, so the invariant held by accident of which screens
    // could reach which rather than by anything in the code.
    //
    // The identity route is the one this drives. It is `acquireIdentity()` now,
    // landing on the list rather than on an onboarding state
    // (`machine-identity-scope`), and entering it from moderation must still
    // leave nothing behind — here that is EVERY state null, since the list is
    // the rendering with none set.
    function test_the_identity_route_from_moderation_clears_the_moderation_state() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})

        view.open(spec.stoaA, "Nym Research", "beef")
        view.moderateIn(spec.stoaA, "Nym Research", "beef")
        compare(view.moderating !== null, true, "moderation is up")

        view.acquireIdentity()
        compare(view.moderating, null,
                "the identity route leaves no moderation state behind it")
        compare(view.screenShown, "list")
        view.destroy()
    }

    // ---- shared chrome ---------------------------------------------------

    // The lamps accompany EVERY main-area screen. A status indicator absent from
    // a screen is one whose absence a user reads as "nothing to report".
    function test_the_status_bar_accompanies_every_main_area_screen() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": spec.oneStoa,
            "list_threads": '{"items":[],"page":0,"hasMore":false}',
            "read_thread": '{"items":[],"page":0,"hasMore":false}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})

        var states = [
            function () { view.closeFeed() },
            function () { view.preview(spec.stoaA, "beef") },
            function () { view.open(spec.stoaA, "Nym Research", "beef") },
            function () { view.openThread("cc" + "11".repeat(31)) },
            function () { view.moderateIn(spec.stoaA, "T", "") }
        ]
        for (var i = 0; i < states.length; i++) {
            states[i]()
            compare(spec.visibleNamed(view, "statusBar").length, 1,
                    "the lamps are rendered alongside " + view.screenShown)
        }
        view.destroy()
    }

    // Unbound chrome must not report health. Green is a claim the software
    // cannot back before the values have arrived, and every screen passes
    // through that state between appearing and core answering.
    function test_an_unbound_lamp_does_not_report_health() {
        Core.bridge = spec.bridgeFor({
            "list_stoas": '{"error":"the store could not be read"}',
            "who_am_i": '{"hasIdentity":false,"reason":"none"}',
            "get_capabilities": '{"canPost":false,"reason":"none"}'
        })
        var view = mainComponent.createObject(null, {})
        var bar = spec.namedAnywhere(view, "statusBar")[0]

        // NO SPEC: the spec requires the least-claiming appearance for an
        // unbound value and does not name which string that is. `DStatusBar`
        // already answers "degraded" for both an unset and an unrecognised
        // state, so the delivery lamp — deliberately bound to nothing, because
        // no synchronous call produces a delivery outcome — reads degraded.
        compare(bar.normalisedState(bar.deliveryState), "degraded",
                "the delivery lamp has no honest source and must not claim one")
        verify(bar.normalisedState(bar.storageState) !== "ok",
               "a store that could not be read is not reported as healthy")
        view.destroy()
    }
}
