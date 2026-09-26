import QtQuick
import QtTest
import "../src/qml"

// The read-only handles `Main.qml` exposes for the end-to-end suite
// (`tests/ui/*.yaml` asserts against them with `state:`).
//
// **What this file can see, and what it cannot.** It pins that each handle
// PROJECTS the screen state it names — so a handle bound to the wrong source, or
// to a constant, fails here in seconds rather than as a confusing red step after
// a Basecamp build. It cannot see whether sitometres can READ them through the
// host's inspector; only the e2e run can, and that is the run's job.
//
// Every fixture is input-dependent on purpose. A handle that returned a fixed
// value — `stoaCount` always 0, `listReadState` always "ok", `joinFailure`
// always non-empty — is the null implementation, and each case below is chosen
// so that it fails against one: a listing of TWO rows, a listing that FAILS,
// and a refusal whose exact text is compared.
TestCase {
    id: spec
    name: "E2eHandles"

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

    Component { id: mainComponent; Main {} }

    readonly property string stoaA:
        "aaaaaaaa11111111111111111111111111111111111111111111111111111111"
    readonly property string stoaB:
        "bbbbbbbb22222222222222222222222222222222222222222222222222222222"

    readonly property string twoStoas:
        '{"items":[{"stoa":"' + stoaA + '","foundingTitle":"Nym Research"},'
        + '{"stoa":"' + stoaB + '","foundingTitle":"Cobalt"}],'
        + '"page":0,"hasMore":false}'

    readonly property string noKey: '{"hasMasterKey":false}'

    // `replies` of `null` is how the "no bridge to the core" case is built:
    // `bridgeFor` always returns an object, so reaching an actually-missing
    // bridge needs this to bypass it rather than pass it an empty map.
    function makeMain(replies) {
        Core.bridge = replies === null ? null : spec.bridgeFor(replies)
        var main = createTemporaryObject(mainComponent, spec, { width: 1000, height: 800 })
        verify(main !== null, "Main.qml instantiated")
        return main
    }

    // A `Main` with NO parent, for the one test in this file that reads a
    // child screen's own `visible` rather than only `screenShown`.
    //
    // **`createTemporaryObject` parents to the TestCase, and that breaks a
    // `visible` read** — measured here, not assumed from `tst_stoa_screens.qml`'s
    // note about a DIFFERENT parenting shape: a screen created under `spec`
    // reads `visible === false` regardless of its own binding, because the
    // TestCase itself is never shown. Every other test in this file only reads
    // `screenShown`, a plain string untouched by that propagation, which is why
    // this is the one place it surfaces. A parentless `Main` behaves like the
    // real app (an always-shown top-level layout), so its children's `visible`
    // reads what their bindings actually say. Caller must `destroy()` it.
    function makeStandaloneMain(replies) {
        Core.bridge = replies === null ? null : spec.bridgeFor(replies)
        var main = mainComponent.createObject(null, { width: 1000, height: 800 })
        verify(main !== null, "Main.qml instantiated")
        return main
    }

    // Chrome `Main.qml` gives every screen, gated on nothing — so it is not one
    // of the screens `verifyOnlyTheListIsRendered` checks below. This is the one
    // hand-typed name left: `namedDescendants` cannot tell chrome from a screen
    // by structure alone (both are named Items), so a SECOND piece of chrome
    // that later gets an `objectName` has to be added here explicitly. Until it
    // is, the enumeration below still finds it and the per-screen loop checks
    // its `visible` too — failing loudly on the very screenShown-mismatch the
    // sibling assertion is built to catch, rather than silently skipping it.
    readonly property var chromeNames: ["statusBar"]

    // Every top-level named child `main` actually mounts, found by walking its
    // real children — the spec-test finding this replaces named exactly this
    // shape ("hand-maintained sweep lists go stale silently", CLAUDE.md) and
    // pointed at `tst_workflow_run_bodies.sh`'s glob as the alternative already
    // in this diff. A screen added to the main area is included the moment it
    // exists, because nothing here has to be told its name.
    //
    // **Stops at the first named item on each branch and does not look inside
    // it.** Every screen's own panels and buttons (`createKeyButton`,
    // `pasteFailureText`, `joinButton`, ...) are named too, several levels
    // deep, and their visibility is that screen's own business, gated on
    // reasons that have nothing to do with the navigator — collecting them
    // here would make this loop assert `pasteFailureText.visible === false`
    // unconditionally, which is exactly backwards from what the paste-failure
    // test needs. Only unnamed wrapper items (the background `Rectangle`, the
    // `Flickable`, the `ColumnLayout`, the padding `Item`s) are recursed into.
    function namedDescendants(item, out) {
        if (item.children === undefined)
            return out
        for (var i = 0; i < item.children.length; i++) {
            var child = item.children[i]
            if (child.objectName !== undefined && child.objectName !== "")
                out.push(child.objectName)
            else
                spec.namedDescendants(child, out)
        }
        return out
    }

    // Asserts that `list` alone is rendered: `screenShown` reads "list", AND the
    // element actually shown on screen is the list and nothing else. The two are
    // wired together by direct `visible: root.screenShown === "..."` bindings in
    // `Main.qml`, but checking only the string would miss a binding typo on one
    // of the OTHER screens — one that left it visible under a string it does not
    // actually match still passes a check that reads only `screenShown`.
    function verifyOnlyTheListIsRendered(main, label) {
        compare(main.screenShown, "list", label + ": the opening screen")
        var names = spec.namedDescendants(main, [])
        // A floor check on the enumeration itself: if `namedDescendants` ever
        // came back empty (a QML `children` reflection change, say), every
        // per-screen assertion below would be silently skipped and this
        // function would report a screen "rendered" that nothing looked at.
        verify(names.indexOf("stoaList") !== -1,
               label + ": the enumeration found the list itself (found: "
               + names.join(", ") + ")")
        for (var i = 0; i < names.length; i++) {
            var name = names[i]
            if (spec.chromeNames.indexOf(name) !== -1)
                continue
            var screen = findChild(main, name)
            verify(screen !== null, label + ": " + name + " is mounted")
            compare(screen.visible, name === "stoaList",
                    label + ": " + name + (name === "stoaList" ? " is rendered" : " is not rendered"))
        }
    }

    function test_the_listing_handles_follow_the_list_screen() {
        var main = spec.makeMain({ "list_stoas": spec.twoStoas, "get_master_key": spec.noKey })
        var list = findChild(main, "stoaList")
        verify(list !== null, "the list screen is found by its objectName")

        compare(main.listReadState, "ok")
        compare(main.listReadState, list.readState)
        compare(main.stoaCount, 2,
                "two rows in the listing is two, so a handle fixed at 0 fails here")
    }

    // The pair the list screen exists to keep apart: a failed read is not an
    // empty membership. `stoaCount` reads `visibleRows`, which is guarded on the
    // read state, so it is 0 here — and `listReadState` is what distinguishes
    // this 0 from the 0 of an empty listing.
    function test_a_failed_listing_is_not_read_as_ok() {
        var main = spec.makeMain({ "list_stoas": '{"error":"store unreadable"}',
                                   "get_master_key": spec.noKey })
        compare(main.listReadState, "failed")
        compare(main.stoaCount, 0)
    }

    // The case above cannot tell WHICH property `stoaCount` reads: on a first
    // read that fails, the guarded `visibleRows` and the unguarded
    // `lastListing` are both empty. A failed RELOAD is where they part —
    // `lastListing` keeps the previous good page on purpose (a failure must
    // not blank a good listing underneath a banner), and only the guard turns
    // it into 0 for a reader outside the screen. So this is the one fixture in
    // which a `stoaCount` bound past the guard reads 2 instead of 0.
    function test_a_failed_reload_does_not_count_the_listing_it_kept() {
        // The bridge reads `replies` at call time, so changing it between the
        // two reads is what makes the second one fail.
        var replies = { "list_stoas": spec.twoStoas, "get_master_key": spec.noKey }
        var main = spec.makeMain(replies)
        var list = findChild(main, "stoaList")
        compare(main.stoaCount, 2, "the first listing is read")

        replies["list_stoas"] = '{"error":"store unreadable"}'
        list.reload()

        compare(main.listReadState, "failed")
        // The precondition that makes this case discriminate. If the screen
        // ever blanks its kept listing on failure, this fails here rather than
        // leaving the assertion below unable to tell the two sources apart.
        compare(list.lastListing.length, 2,
                "the failed reload left the previous listing in place")
        compare(main.stoaCount, 0,
                "a listing the screen has said was not read is not counted")
    }

    // `view-navigation`: "The view opens on the Stoa list", scenario "What the
    // core answers does not change the opening screen". Two of its five listed
    // core answers are pinned elsewhere: a failed master-key query with an empty
    // listing in `tst_stoa_screens.qml`'s
    // `test_the_view_supplies_no_stoa_of_its_own_before_one_is_chosen`, and the
    // fifth is the same assertion with no core answer to change at all — a fresh
    // `Main` before any reply arrives, which every other test in this file
    // relies on already being "list" to make its own point. The three left are a
    // table here because they are one scenario applied to three different
    // answers, not three different behaviours: the same two assertions —
    // `screenShown` and each screen's own `visible` — repeated per case would be
    // the near-identical-functions shape CLAUDE.md asks to collapse.
    //
    // **What this can fail against.** `Main.qml`'s `screenShown` is computed
    // only from navigator state (`chosen`, `previewing`, `reading`,
    // `moderating`) that nothing here ever sets — so this table is pinning that
    // invariant, not merely restating it. A future change that let a failed read
    // or an unreachable core drive one of those properties (an auto-navigate to
    // an error screen, say) would turn this red without touching `screenShown`'s
    // own definition.
    function test_the_view_opens_on_the_list_whatever_core_answers() {
        var cases = [
            { label: "an empty listing with no key held",
              replies: { "list_stoas": '{"items":[],"page":0,"hasMore":false}',
                         "get_master_key": spec.noKey } },
            { label: "a failed listing",
              replies: { "list_stoas": '{"error":"store unreadable"}',
                         "get_master_key": spec.noKey } },
            { label: "no bridge to the core", replies: null }
        ]

        for (var i = 0; i < cases.length; i++) {
            var c = cases[i]
            var main = spec.makeStandaloneMain(c.replies)
            spec.verifyOnlyTheListIsRendered(main, c.label)
            main.destroy()
        }
    }

    // `view-navigation`: "A paste refused as not a Stoa reference leaves the
    // list rendered". Two cases, table-driven for the same reason as the test
    // above: one scenario, two shapes of refused input, the same three
    // assertions each time.
    function test_a_paste_refused_as_not_a_stoa_reference_leaves_the_list_rendered() {
        var cases = [
            { label: "plain text that is not a Stoa reference",
              pasted: "not a stoa reference" },
            // The typed half's own JSON, missing its record half — the
            // navigate-first-discover-the-failure-later shape the spec-test
            // finding named, closed for the JSON case as well as the parse case
            // `DStoaReference`'s own tests already pin.
            { label: "a JSON object carrying stoa but no genesis",
              pasted: JSON.stringify({ stoa: spec.stoaA }) }
        ]

        for (var i = 0; i < cases.length; i++) {
            var c = cases[i]
            // Standalone, not `makeMain`'s TestCase-parented one: this test's
            // sibling (`test_the_view_opens_on_the_list_whatever_core_answers`)
            // is where the parenting note above was measured, and
            // `verifyOnlyTheListIsRendered` reads exactly the `visible`
            // property that breaks under `makeMain`. Checking only
            // `screenShown`, as this test used to, would miss a stray binding
            // that rendered the join/preview screen ON TOP of the list after a
            // refused paste — the same gap the sibling test closed for the
            // opening screen.
            var main = spec.makeStandaloneMain({ "list_stoas": spec.twoStoas, "get_master_key": spec.noKey })
            var list = findChild(main, "stoaList")

            compare(main.pasteFailure, "", c.label + ": nothing pasted, nothing refused")
            list.pasted = c.pasted
            list.preview()
            verify(main.pasteFailure.length > 0, c.label + ": a malformed paste is refused")
            compare(main.pasteFailure, list.pasteFailure, c.label)
            spec.verifyOnlyTheListIsRendered(main, c.label + ": and it does not navigate")
            main.destroy()
        }
    }

    function test_the_join_handles_follow_the_join_screen() {
        var refusal = "the genesis record does not hash to this address"
        var main = spec.makeMain({
            "list_stoas": spec.twoStoas,
            "get_master_key": spec.noKey,
            "get_stoa": '{"error":"no Stoa is held at this address"}',
            "join_stoa": JSON.stringify({ error: refusal })
        })
        var join = findChild(main, "joinScreen")
        verify(join !== null, "the join screen is found by its objectName")

        main.preview(spec.stoaA, "00ff")
        compare(main.screenShown, "join")
        compare(main.joinState, "previewing")
        compare(main.joinFailure, "")

        join.join()
        compare(main.joinState, "failed")
        compare(main.joinFailure, refusal,
                "the refusal is the core's text, so a handle that is merely non-empty fails")
    }
}
