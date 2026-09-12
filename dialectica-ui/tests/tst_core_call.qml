import QtQuick
import QtTest
import "../src/qml"

// The core call path: what `Core.call()` does with each shape of reply.
//
// The CI comment block names this as the first thing to add, and the reason is
// that this function is where the view's ONE error branch lives. Every screen
// depends on it normalising a reply into exactly one of two shapes, and until
// now nothing checked that it does.
//
// A fake bridge is what makes this testable at all: `logos` is injected by
// basecamp, so a harness cannot supply one. `Core.bridge` exists for this.
TestCase {
    id: spec
    name: "CoreCall"

    // Restored after every test, so one test's fake cannot leak into the next.
    // `Core` is a singleton — it is the same object for every test in this
    // file, which is exactly the shape that produces order-dependent passes if
    // nothing resets it.
    property var savedBridge: undefined

    function init() {
        spec.savedBridge = Core.bridge
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    // A bridge whose callModule returns `reply` verbatim, and which records
    // what it was asked for.
    function fakeBridge(reply) {
        return {
            lastModule: "",
            lastMethod: "",
            lastArgs: null,
            callModule: function (module, method, args) {
                this.lastModule = module
                this.lastMethod = method
                this.lastArgs = args
                return reply
            }
        }
    }

    // ---- a successful reply ---------------------------------------------

    function test_a_valid_reply_is_parsed_and_reported_as_ok() {
        Core.bridge = fakeBridge('{"items":[],"page":0,"hasMore":false}')
        var out = Core.call("list_threads", ["{}"])

        compare(out.ok, true, "a well-formed reply must be reported as ok")
        compare(out.error, undefined, "a success must carry no error — never both")
        verify(out.value !== undefined, "a success must carry the parsed value")
        compare(out.value.page, 0)
        compare(out.value.hasMore, false)
        compare(out.value.items.length, 0)
    }

    function test_the_call_reaches_the_bridge_with_the_module_and_method() {
        // Without this, a call that never left the view would be
        // indistinguishable from one that returned an empty result — the same
        // reasoning behind Phase 0's `ping` carrying a payload.
        var fake = fakeBridge('{"ok":true}')
        Core.bridge = fake
        Core.call("some_method", ["{\"a\":1}"])

        compare(fake.lastModule, "dialectica", "the module name must be passed")
        compare(fake.lastMethod, "some_method")
        compare(fake.lastArgs.length, 1)
        compare(fake.lastArgs[0], "{\"a\":1}", "the argument must arrive verbatim")
    }

    // ---- an error reply -------------------------------------------------

    function test_an_error_reply_is_reported_as_not_ok_and_carries_no_value() {
        // §2.5's one failure shape. A reply carrying both would let a caller
        // that checks `value` first render a failure as a result.
        Core.bridge = fakeBridge('{"error":"the store could not be read"}')
        var out = Core.call("list_threads", ["{}"])

        compare(out.ok, false)
        compare(out.error, "the store could not be read",
                "core's own message must reach the caller unchanged")
        compare(out.value, undefined, "a failure must never also carry a value")
    }

    function test_an_error_reply_is_not_confused_with_an_empty_success() {
        // THE distinction screen 07 exists for, at the layer that decides it.
        // These two replies must not produce the same normalised shape.
        Core.bridge = fakeBridge('{"error":"database is locked"}')
        var failed = Core.call("list_threads", ["{}"])

        Core.bridge = fakeBridge('{"items":[],"page":0,"hasMore":false}')
        var empty = Core.call("list_threads", ["{}"])

        compare(failed.ok, false)
        compare(empty.ok, true)
        verify(failed.ok !== empty.ok,
               "a broken store and an empty store must not normalise alike")
    }

    // ---- malformed replies ----------------------------------------------

    function test_a_non_json_reply_is_a_failure_rather_than_rendered_raw() {
        // The core always answers JSON, so anything else means the call did not
        // reach it. Rendering the bytes would put a transport error on screen
        // as though it were content.
        Core.bridge = fakeBridge("MODULE_NOT_LOADED")
        var out = Core.call("version", [])

        compare(out.ok, false)
        compare(out.value, undefined)
        verify(out.error.length > 0, "a failure must name itself")
    }

    function test_a_reply_that_is_json_but_not_an_object_is_a_failure() {
        // `JSON.parse("7")` succeeds and yields a number. A check that only
        // caught parse errors would hand a caller `value.items` on a number and
        // get `undefined` — which renders as an empty feed.
        var notObjects = ['7', '"a string"', 'null', 'true']
        for (var i = 0; i < notObjects.length; i++) {
            Core.bridge = fakeBridge(notObjects[i])
            var out = Core.call("version", [])
            compare(out.ok, false, "reply " + notObjects[i] + " must not be ok")
            compare(out.value, undefined)
        }
    }

    function test_a_throwing_bridge_is_a_failure_and_not_a_crash() {
        // A cross-module call can throw. An unhandled exception in a QML
        // function leaves the binding that called it in an undefined state,
        // which is a blank screen rather than a message.
        Core.bridge = {
            callModule: function () { throw new Error("IPC timed out") }
        }
        var out = Core.call("list_threads", ["{}"])

        compare(out.ok, false)
        compare(out.value, undefined)
        verify(out.error.indexOf("IPC timed out") >= 0,
               "the underlying reason must survive, got: " + out.error)
    }

    // ---- no bridge at all -----------------------------------------------

    // The guard's own message, asserted exactly rather than by "some error
    // occurred".
    //
    // **This pin is the whole strength of the next two tests, and it was added
    // after they failed to earn their keep.** With the guard deleted, `call()`
    // still returns a failure — the try/catch around `callModule` catches the
    // resulting TypeError and reports it — so an assertion of merely
    // `ok === false` passed with the guard gone. Two different explanations,
    // one answer: exactly this project's test-defect family.
    //
    // Asserting the guard's specific wording separates the two paths, because
    // the catch-all's message is the exception's text instead.
    readonly property string absentMessage:
        "The core module is not reachable from this view."

    function test_a_missing_bridge_is_reported_plainly() {
        // Named in the CI comment as one of the three things call() does. A
        // view running with no core must say so: silence here looks exactly
        // like a Stoa with nothing in it.
        Core.bridge = null
        var out = Core.call("version", [])

        compare(out.ok, false)
        compare(out.value, undefined)
        compare(out.error, spec.absentMessage,
                "the ABSENT-BRIDGE message must be the one reported, not a "
                + "TypeError the catch-all happened to convert")
    }

    function test_a_bridge_without_call_module_is_treated_as_absent() {
        // A present-but-wrong bridge object. Without the `callModule` half of
        // the guard this reaches the call, throws, and is reported by the
        // catch-all — a failure either way, which is why the message is what
        // this asserts on.
        Core.bridge = { somethingElse: true }
        var out = Core.call("version", [])

        compare(out.ok, false)
        compare(out.value, undefined)
        compare(out.error, spec.absentMessage,
                "a bridge lacking callModule must be diagnosed as absent "
                + "rather than reported as an exception")
    }

    // ---- the named wrappers --------------------------------------------

    function test_list_threads_serialises_its_request_and_names_the_method() {
        var fake = fakeBridge('{"items":[],"page":0,"hasMore":false}')
        Core.bridge = fake
        Core.listThreads({ stoa: "abcd", page: 2, includeHidden: true })

        compare(fake.lastMethod, "list_threads")
        var sent = JSON.parse(fake.lastArgs[0])
        compare(sent.stoa, "abcd")
        compare(sent.page, 2)
        compare(sent.includeHidden, true)
    }

    function test_get_capabilities_names_its_method_and_passes_the_stoa() {
        var fake = fakeBridge('{"canPost":false,"reason":"no keystore"}')
        Core.bridge = fake
        var out = Core.getCapabilities("beef")

        compare(fake.lastMethod, "get_capabilities")
        compare(JSON.parse(fake.lastArgs[0]).stoa, "beef")
        compare(out.ok, true)
        compare(out.value.canPost, false)
        compare(out.value.reason, "no keystore")
    }
}
