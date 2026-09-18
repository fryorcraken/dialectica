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

    // A bridge that answers the way BASECAMP answers, and which records what it
    // was asked for.
    //
    // **The double encoding is the point, and this fake got it wrong for the
    // whole life of the suite.** Core returns a JSON string;
    // `LogosQmlBridge::callModule` then "serialize[s] QVariant result to JSON"
    // (logos-basecamp `docs/project.md`), and serialising a QString that already
    // holds JSON wraps the entire reply in a JSON string literal. So what
    // reaches QML is the reply ESCAPED INSIDE QUOTES, not the reply.
    //
    // Measured in a real launch: `list_stoas` arrived as the 45-character
    //   "{\"hasMore\":false,\"items\":[],\"page\":0}"
    // whose `typeof` after ONE `JSON.parse` is `string`.
    //
    // A fake that returned the reply verbatim modelled a host that does not
    // exist, so every test here passed against a bridge that could never have
    // worked — 22 spec files green over a view where every core call failed.
    // `JSON.stringify(reply)` is the whole correction: it is exactly the extra
    // encoding layer the host applies.
    function fakeBridge(reply) {
        return {
            lastModule: "",
            lastMethod: "",
            lastArgs: null,
            callModule: function (module, method, args) {
                this.lastModule = module
                this.lastMethod = method
                this.lastArgs = args
                return JSON.stringify(reply)
            }
        }
    }

    // The host WITHOUT the extra encoding layer — the reply verbatim.
    //
    // This exists because the unwrap must not be a bet on one host's quirk. If
    // basecamp stops double-encoding, `call()` has to keep working, and a suite
    // that only ever sees the wrapped form could not tell a correct unwrap from
    // one that happens to match today's host. Both shapes are pinned.
    function fakeBridgeVerbatim(reply) {
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

    // ---- the double encoding itself --------------------------------------

    function test_a_double_encoded_reply_is_unwrapped_to_the_object() {
        // The regression this file exists to pin. Written against the measured
        // bytes from a real launch rather than a guess at them: with the unwrap
        // removed, `JSON.parse` yields a string and `call()` reports "not a
        // reply" — which is exactly what every screen showed.
        Core.bridge = fakeBridge('{"hasMore":false,"items":[],"page":0}')
        var out = Core.call("list_stoas", ["{}"])

        compare(out.ok, true, "a double-encoded success must be unwrapped, not "
                + "reported as 'not a reply'")
        compare(out.value.hasMore, false)
        compare(out.value.page, 0)
        compare(out.value.items.length, 0)
    }

    function test_a_double_encoded_error_reaches_the_caller_as_an_error() {
        // The measured `create_stoa` reply. Without the unwrap this is reported
        // as a malformed-reply failure, so the user is told the module answered
        // nonsense rather than being told WHY the Stoa was not created — the
        // error shape exists precisely so that reason survives.
        Core.bridge = fakeBridge('{"error":"no keystore found; create one before posting"}')
        var out = Core.call("create_stoa", ["{}"])

        compare(out.ok, false)
        compare(out.error, "no keystore found; create one before posting",
                "core's own reason must reach the caller, not a generic "
                + "malformed-reply message")
    }

    function test_a_reply_that_is_not_double_encoded_still_works() {
        // The unwrap is conditional on there being a layer to unwrap, so a host
        // that hands back the reply verbatim must keep working. Without this the
        // suite could not distinguish a correct unwrap from one hardcoded to
        // today's host.
        Core.bridge = fakeBridgeVerbatim('{"items":[],"page":0,"hasMore":false}')
        var out = Core.call("list_stoas", ["{}"])

        compare(out.ok, true, "a single-encoded reply must still parse")
        compare(out.value.page, 0)
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
        //
        // **The unwrap must not turn any of these into a success.** `'7'`
        // double-encodes to `"7"`, which parses to the STRING "7", and the
        // conditional re-parse then yields the number 7 — still not an object,
        // so still a failure. That is the case most likely to be got wrong by a
        // second parse written without this test watching.
        var notObjects = ['7', 'null', 'true']
        for (var i = 0; i < notObjects.length; i++) {
            Core.bridge = fakeBridge(notObjects[i])
            var out = Core.call("version", [])
            compare(out.ok, false, "reply " + notObjects[i] + " must not be ok")
            compare(out.value, undefined)
        }
    }

    function test_a_string_that_is_not_json_underneath_is_a_failure() {
        // A bare string reply, once unwrapped, is not JSON at all. The unwrap
        // must report that rather than swallowing it — a string that fails the
        // second parse is a transport answer, not content.
        //
        // Split out from the loop above because it exercises the OTHER branch:
        // there the re-parse succeeds and the guard rejects the value; here the
        // re-parse itself throws.
        Core.bridge = fakeBridge("a string")
        var out = Core.call("version", [])

        compare(out.ok, false)
        compare(out.value, undefined)
        verify(out.error.length > 0, "a failure must name itself")
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

    // ---- the shared probe normalisation ----------------------------------
    //
    // `capabilityFrom` and `identityFrom` were byte-for-byte copies on
    // `FeedScreen.qml` and `DThreadScreen.qml`, with NOTHING able to observe them
    // drifting apart — each copy internally consistent, so every gate stayed
    // green while a fix to one left the other carrying the defect. These pin the
    // rule on `Core`, where both screens now read it from.
    //
    // The degenerate SHAPES (`"true"`, `1`, `null`) are driven as rendering
    // fixtures in `tst_gate_affordance.qml` and `tst_vote_and_gate.qml`; what is
    // pinned here is the normalisation itself, and that both screens get the same
    // answer from it.

    function test_a_capability_probe_is_normalised_to_both_fields_always_present() {
        // Input-dependent on purpose: a normaliser that returned a fixed object
        // would pass an assertion that only checked the fields exist.
        var open = Core.capabilityFrom({ ok: true, value: { canPost: true, reason: "ignored" } })
        compare(open.canPost, true)
        // An open gate carries no reason, even when the probe supplied one.
        compare(open.reason, "")

        var shut = Core.capabilityFrom({ ok: true, value: { canPost: false, reason: "no keystore found" } })
        compare(shut.canPost, false)
        compare(shut.reason, "no keystore found")

        var broken = Core.capabilityFrom({ ok: false, error: "the core module is not reachable" })
        compare(broken.canPost, false)
        compare(broken.reason, "the core module is not reachable")
    }

    function test_only_a_literal_true_opens_the_capability_gate() {
        // The fail-closed rule. Each of these is truthy-or-falsy in a way that
        // does not match what it means, and each must reach the SAME state as a
        // probe reporting "not possible".
        var looser = ["true", 1, null, undefined, {}, "yes"]
        for (var i = 0; i < looser.length; i++) {
            var out = Core.capabilityFrom({ ok: true, value: { canPost: looser[i] } })
            verify(!out.canPost, "canPost was opened by a non-true value at index " + i)
            compare(typeof out.reason, "string")
        }
    }

    function test_an_identity_probe_is_normalised_to_all_three_fields() {
        var present = Core.identityFrom({
            ok: true, value: { hasIdentity: true, publicKey: "ab12cd", reason: "ignored" }
        })
        compare(present.hasIdentity, true)
        compare(present.publicKey, "ab12cd")
        // A filled chip carries no reason: it would describe a state the reader
        // is not in.
        compare(present.reason, "")

        var absent = Core.identityFrom({
            ok: true, value: { hasIdentity: false, reason: "keystore permissions are too open" }
        })
        compare(absent.hasIdentity, false)
        compare(absent.publicKey, "")
        compare(absent.reason, "keystore permissions are too open")

        var failed = Core.identityFrom({ ok: false, error: "no keystore found" })
        compare(failed.hasIdentity, false)
        compare(failed.publicKey, "")
        compare(failed.reason, "no keystore found")
    }

    function test_only_a_literal_true_claims_an_identity() {
        var looser = ["true", 1, null, undefined, {}, "yes"]
        for (var i = 0; i < looser.length; i++) {
            var out = Core.identityFrom({
                ok: true, value: { hasIdentity: looser[i], publicKey: "ab12cd" }
            })
            verify(!out.hasIdentity, "hasIdentity was claimed by a non-true value at index " + i)
            // No key is named for an identity that was not established — naming
            // one is the defect the `=== true` rule exists to avert.
            compare(out.publicKey, "")
        }
    }

    // The property that the duplication could not hold: both screens answer
    // identically because both delegate to the same rule.
    //
    // **This fails if either screen re-acquires a private copy that differs.** A
    // copy that is still identical passes, which is honest — the defect is
    // divergence, and this is what observes it.
    function test_both_screens_normalise_a_probe_the_same_way() {
        var feed = feedScreen.createObject(spec)
        var thread = threadScreen.createObject(spec)

        // Distinguishing inputs: a closed gate carrying a reason, and a refused
        // probe. A shared implementation gives one answer for both screens.
        var probes = [
            { ok: true, value: { canPost: false, reason: "no keystore found" } },
            { ok: true, value: { canPost: true, reason: "ignored" } },
            { ok: false, error: "the core module is not reachable" }
        ]
        for (var i = 0; i < probes.length; i++) {
            var f = feed.capabilityFrom(probes[i])
            var t = thread.capabilityFrom(probes[i])
            compare(t.canPost, f.canPost, "capability.canPost diverged at index " + i)
            compare(t.reason, f.reason, "capability.reason diverged at index " + i)
            compare(f.reason, Core.capabilityFrom(probes[i]).reason,
                    "the feed's answer left Core's rule at index " + i)
        }

        var idProbes = [
            { ok: true, value: { hasIdentity: true, publicKey: "ab12cd" } },
            { ok: true, value: { hasIdentity: false, reason: "permissions are too open" } },
            { ok: false, error: "no keystore found" }
        ]
        for (var j = 0; j < idProbes.length; j++) {
            var fi = feed.identityFrom(idProbes[j])
            var ti = thread.identityFrom(idProbes[j])
            compare(ti.hasIdentity, fi.hasIdentity, "identity.hasIdentity diverged at index " + j)
            compare(ti.publicKey, fi.publicKey, "identity.publicKey diverged at index " + j)
            compare(ti.reason, fi.reason, "identity.reason diverged at index " + j)
            compare(fi.reason, Core.identityFrom(idProbes[j]).reason,
                    "the feed's answer left Core's rule at index " + j)
        }

        feed.destroy()
        thread.destroy()
    }

    Component {
        id: feedScreen
        FeedScreen {}
    }

    Component {
        id: threadScreen
        DThreadScreen {}
    }
}
