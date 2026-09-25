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

    function makeMain(replies) {
        Core.bridge = spec.bridgeFor(replies)
        var main = createTemporaryObject(mainComponent, spec, { width: 1000, height: 800 })
        verify(main !== null, "Main.qml instantiated")
        return main
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

    function test_the_paste_failure_is_the_list_screens_own() {
        var main = spec.makeMain({ "list_stoas": spec.twoStoas, "get_master_key": spec.noKey })
        var list = findChild(main, "stoaList")

        compare(main.pasteFailure, "", "nothing pasted, nothing refused")
        list.pasted = "not a stoa reference"
        list.preview()
        verify(main.pasteFailure.length > 0, "a malformed paste is refused")
        compare(main.pasteFailure, list.pasteFailure)
        compare(main.screenShown, "list", "and it does not navigate")
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
