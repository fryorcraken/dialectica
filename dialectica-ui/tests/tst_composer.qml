import QtQuick
import QtTest
import "../src/qml"

// The composer's state machine, driven through a fake bridge — so what is under
// test is the real component's behaviour rather than a re-implementation of it.
//
// The property nearly every test here is really asserting: **a publish that did
// not demonstrably happen must not be reported as one, and a draft must survive
// anything that is not a store.** Both are about what the user is told and what
// they still have; neither can be checked by looking at the happy path.
TestCase {
    id: spec
    name: "Composer"

    property var savedBridge: undefined
    property var lastCall: ({})

    function init() {
        spec.savedBridge = Core.bridge
        spec.lastCall = {}
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    // Records the method and the raw argument string, so a test can assert what
    // was actually sent rather than only what came back. The byte-for-byte
    // requirement is unprovable without this.
    function bridgeFor(replies) {
        return {
            callModule: function (module, method, args) {
                spec.lastCall = { method: method, args: args }
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    // A bridge that is not a bridge: `callModule` absent. This is the
    // "core unreachable" case, and it must reach the same refused state as a
    // core that answered with an error.
    function deadBridge() {
        return { }
    }

    function makeComposer(replies, props) {
        Core.bridge = bridgeFor(replies)
        var p = props === undefined ? {} : props
        if (p.stoaAddress === undefined)
            p.stoaAddress = "ab".repeat(32)
        return composerComponent.createObject(null, p)
    }

    Component {
        id: composerComponent
        Composer {}
    }

    // ---- the UTF-8 byte count ------------------------------------------
    //
    // The unit is the thing a composer gets wrong, so it is asserted directly
    // rather than only through the over-limit behaviour built on it.

    function test_the_length_is_utf8_bytes_and_not_utf16_code_units() {
        var c = makeComposer({})

        // ASCII: the two agree, which is exactly why a character-counting
        // composer looks correct in every test written in English.
        c.draft = "hello"
        compare(c.draftBytes, 5)
        compare(c.draft.length, 5, "the degenerate case where both answers match")

        // Two bytes. `.length` says 1.
        c.draft = "é"
        compare(c.draftBytes, 2, "U+00E9 is two bytes in UTF-8")
        verify(c.draftBytes !== c.draft.length,
               "this test is worthless if the two counts agree here")

        // Three bytes. `.length` still says 1.
        c.draft = "中"
        compare(c.draftBytes, 3, "a CJK character is three bytes")

        // Four bytes, and TWO UTF-16 code units — the case where a composer
        // counting `.length` under-reports by half rather than by a third.
        c.draft = "🏛"   // U+1F3DB CLASSICAL BUILDING
        compare(c.draftBytes, 4, "an astral character is four bytes")
        compare(c.draft.length, 2, "and two code units, which is the trap")

        c.destroy()
    }

    function test_an_unpaired_surrogate_is_counted_rather_than_throwing() {
        // The reason `encodeURIComponent` was rejected: it throws URIError on a
        // lone surrogate, which a paste or an IME can leave in the field
        // mid-edit. A length function that throws leaves the submit affordance
        // in whatever state it held when the exception unwound.
        var c = makeComposer({})

        c.draft = "a\ud83cb"       // a high surrogate with no low one
        compare(c.draftBytes, 5, "1 + 3 for the malformed surrogate + 1")

        c.draft = "a\udfdbb"       // a lone LOW surrogate
        compare(c.draftBytes, 5)

        c.destroy()
    }

    // ---- the limit ------------------------------------------------------

    function test_an_over_length_draft_blocks_submission_before_any_call() {
        var c = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' },
                             { bodyByteLimit: 10 })

        c.draft = "12345678901"     // 11 bytes
        compare(c.overLimit, true)
        compare(c.submittable, false, "the submit affordance must be unavailable")

        // And no call is made even if submit() is reached some other way.
        c.submit()
        compare(spec.lastCall.method, undefined,
                "an over-length draft must not reach the bridge at all")
        compare(c.outcome, "", "and must not produce an outcome")
        c.destroy()
    }

    function test_the_limit_is_bytes_so_a_short_multibyte_draft_is_refused() {
        // THE test that a character-counting implementation fails. Ten
        // characters, well under a ten-character limit; thirty bytes, well over
        // a ten-byte one.
        var c = makeComposer({}, { bodyByteLimit: 10 })

        c.draft = "中".repeat(10)
        compare(c.draft.length, 10, "ten characters — a character count would pass this")
        compare(c.draftBytes, 30)
        compare(c.overLimit, true, "but thirty bytes, which is over the limit")
        c.destroy()
    }

    function test_an_over_length_draft_is_never_truncated() {
        var c = makeComposer({}, { bodyByteLimit: 10 })
        var typed = "the whole of what was written, which is well over ten bytes"

        c.draft = typed
        compare(c.draft, typed,
                "the draft the composer holds must still be the full text")
        c.destroy()
    }

    function test_a_draft_at_the_limit_is_submittable() {
        // The boundary AT the boundary, not far from it: a test using a draft
        // of one byte would pass against an implementation with the comparison
        // the wrong way round.
        var c = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' },
                             { bodyByteLimit: 10 })

        c.draft = "1234567890"      // exactly 10
        compare(c.draftBytes, 10)
        compare(c.overLimit, false, "at the limit is within the limit")
        compare(c.submittable, true)
        c.destroy()
    }

    // ---- the body reaches core unchanged --------------------------------

    function test_the_body_sent_is_byte_for_byte_the_draft() {
        // Not "a body was sent" — the body that was sent, parsed back out of
        // the argument the bridge actually received. A composer that trimmed,
        // normalised or stripped would pass a weaker assertion.
        var c = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' })

        var typed = "  ‮ leading and trailing space ​ and a 🏛  \n"
        c.draft = typed
        c.submit()

        compare(spec.lastCall.method, "publish_post")
        var sent = JSON.parse(spec.lastCall.args[0])
        compare(sent.body, typed,
                "the body must reach core exactly as typed — an op is signed over its bytes")
        c.destroy()
    }

    // ---- the invisible-character warning --------------------------------

    function test_invisible_characters_are_counted_and_do_not_block_submission() {
        var c = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' })

        c.draft = "safe‮text​more﻿end"
        compare(c.invisibleCount, 3, "one override, one ZWSP, one BOM")
        compare(c.submittable, true, "the warning must NOT be a gate")
        c.destroy()
    }

    function test_a_clean_draft_carries_no_warning() {
        var c = makeComposer({})
        c.draft = "An ordinary post, with punctuation — and a dash.\nAnd a newline."
        compare(c.invisibleCount, 0,
                "ordinary text and whitespace are content, not controls")
        c.destroy()
    }

    function test_the_invisible_set_matches_the_ranges_core_removes() {
        // Each is one character from a range `sanitise::is_invisible` removes.
        // A view whose list drifted from core's would warn about the wrong
        // things, and the drift would be invisible to both.
        var c = makeComposer({})
        var invisibles = ["‪", "‮", "⁦", "⁩", "​",
                          "‍", "‎", "‏", "⁠", "⁤",
                          "؜", "﻿", "￹", "￻"]
        for (var i = 0; i < invisibles.length; i++) {
            c.draft = "a" + invisibles[i] + "b"
            compare(c.invisibleCount, 1,
                    "U+" + invisibles[i].charCodeAt(0).toString(16) + " was not counted")
        }

        // And the boundaries: one code point outside each range must NOT count.
        // Asserting that 'A' is not invisible would prove nothing about where a
        // range ends.
        var visibles = [" ", " ", " ", "⁥", "⁪",
                        " ", "‐", "﻾", "؛", "؝",
                        "￸", "￼"]
        for (var j = 0; j < visibles.length; j++) {
            c.draft = "a" + visibles[j] + "b"
            compare(c.invisibleCount, 0,
                    "U+" + visibles[j].charCodeAt(0).toString(16)
                    + " is outside every removed range and was counted anyway")
        }
        c.destroy()
    }

    // ---- the three outcomes ---------------------------------------------

    function test_a_new_op_is_the_stored_outcome() {
        var c = makeComposer({ "publish_post": '{"opId":"deadbeef","wasNew":true}' })
        c.draft = "something"
        c.submit()
        compare(c.outcome, "stored")
        c.destroy()
    }

    function test_a_deduplicated_op_is_its_own_outcome_and_not_an_error() {
        var c = makeComposer({ "publish_post": '{"opId":"deadbeef","wasNew":false}' })
        c.draft = "something"
        c.submit()
        compare(c.outcome, "existing",
                "wasNew:false is neither a failure nor a fresh success")
        verify(c.outcome !== "refused",
               "nothing failed, so this must not be the refused state")
        c.destroy()
    }

    // Create AND submit before the next composer is made.
    //
    // `Core` is a singleton, so `makeComposer` reassigns the one `Core.bridge`
    // every composer shares. Building three composers and then submitting all
    // three would run every submission against the LAST bridge — and the first
    // draft of these two tests did exactly that, reporting the three outcomes as
    // indistinguishable when they are not. The failure was real and the cause
    // was the harness, which is worth keeping as a comment: this shape is easy
    // to reintroduce and its symptom looks like a defect in the component.
    function submitAgainst(reply) {
        var c = makeComposer({ "publish_post": reply })
        c.draft = "identical text"
        c.submit()
        return c
    }

    function test_the_three_outcomes_are_mutually_distinguishable() {
        // The property stated directly rather than inferred: the SAME
        // submission against three cores must reach three different states.
        var fresh = submitAgainst('{"opId":"aa","wasNew":true}')
        var dedup = submitAgainst('{"opId":"aa","wasNew":false}')
        var bad   = submitAgainst('{"error":"the keystore is unreadable"}')

        compare(fresh.outcome, "stored")
        compare(dedup.outcome, "existing")
        compare(bad.outcome, "refused")

        verify(fresh.outcome !== dedup.outcome, "stored vs existing")
        verify(fresh.outcome !== bad.outcome,   "stored vs refused")
        verify(dedup.outcome !== bad.outcome,   "existing vs refused")

        fresh.destroy(); dedup.destroy(); bad.destroy()
    }

    // ---- the message a user actually reads ------------------------------
    //
    // The three outcomes being distinct STATES does not make their messages
    // distinct. These read the rendered text, because the requirement is about
    // what is displayed.

    function renderedText(item, acc) {
        // Every Text in the tree, joined. `text` on a non-Text is not a string,
        // so the typeof guard is what keeps this from concatenating objects.
        var out = acc === undefined ? "" : acc
        if (item === null || item === undefined)
            return out
        if (typeof item.text === "string" && item.visible !== false)
            out += item.text + "\n"
        var kids = item.children
        if (kids !== undefined) {
            for (var i = 0; i < kids.length; i++)
                out = spec.renderedText(kids[i], out)
        }
        return out
    }

    function test_no_two_outcomes_display_the_same_message() {
        var fresh = submitAgainst('{"opId":"aa","wasNew":true}')
        var dedup = submitAgainst('{"opId":"aa","wasNew":false}')
        var bad   = submitAgainst('{"error":"the keystore is unreadable"}')

        var a = spec.renderedText(fresh)
        var b = spec.renderedText(dedup)
        var d = spec.renderedText(bad)

        verify(a !== b, "a stored op and an existing one must not read alike")
        verify(a !== d, "a stored op and a refusal must not read alike")
        verify(b !== d, "an existing op and a refusal must not read alike")

        fresh.destroy(); dedup.destroy(); bad.destroy()
    }

    function test_a_success_names_local_storage_and_claims_no_delivery() {
        var c = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' })
        c.draft = "something"
        c.submit()

        var shown = spec.renderedText(c).toLowerCase()

        // The claim that must be there.
        verify(shown.indexOf("this machine") >= 0,
               "a success must say the content was saved on THIS MACHINE, got: " + shown)

        // The claims that must not. Each is a word for an outcome no part of
        // this system has checked: the reply carries no delivery outcome, and
        // delivery's result arrives asynchronously after the call returns.
        //
        // "delivered" and "received" are matched as whole claims about the post.
        // The qualifier sentence legitimately contains "received it" inside
        // "whether any other peer HAS received it is not something this software
        // can tell you" — a denial, not a claim — so the assertions below are
        // written against the affirmative forms.
        var forbidden = ["was sent", "was delivered", "was received",
                         "has been sent", "has been delivered",
                         "everyone can see", "others can see", "peers reached",
                         "published to the stoa"]
        for (var i = 0; i < forbidden.length; i++) {
            verify(shown.indexOf(forbidden[i]) < 0,
                   "a success must not claim '" + forbidden[i] + "', got: " + shown)
        }
        c.destroy()
    }

    // ---- a reply the view cannot interpret is a refusal ------------------

    function test_an_unreachable_core_is_a_refusal_and_keeps_the_draft() {
        Core.bridge = deadBridge()
        var c = composerComponent.createObject(null, { stoaAddress: "ab".repeat(32) })

        c.draft = "what was written"
        c.submit()

        compare(c.outcome, "refused")
        compare(c.draft, "what was written", "the draft must survive")
        c.destroy()
    }

    function test_a_reply_that_is_not_json_is_a_refusal() {
        var c = makeComposer({ "publish_post": "<html>gateway timeout</html>" })
        c.draft = "something"
        c.submit()
        compare(c.outcome, "refused")
        verify(spec.renderedText(c).indexOf("saved on this machine") < 0,
               "no success message on a reply the view could not parse")
        c.destroy()
    }

    function test_a_success_shape_with_no_op_id_is_a_refusal() {
        // The guard that makes "success" mean more than "no error field". A
        // composer testing only `reply.ok` reports this as a publish that
        // happened, and the user goes looking for a post that was never written.
        var c = makeComposer({ "publish_post": '{"wasNew":true}' })
        c.draft = "something"
        c.submit()
        compare(c.outcome, "refused",
                "an object carrying no op id and no error must be refused")
        compare(c.draft, "something")
        c.destroy()
    }

    function test_a_success_shape_with_no_was_new_is_a_refusal() {
        // `wasNew` absent and `wasNew:false` are different facts. Guessing one
        // of the two successes would tell the user either that a post was saved
        // or that it already existed, with nothing behind either claim.
        var c = makeComposer({ "publish_post": '{"opId":"aa"}' })
        c.draft = "something"
        c.submit()
        compare(c.outcome, "refused")
        c.destroy()
    }

    function test_cores_error_shape_is_a_refusal_carrying_its_message() {
        var c = makeComposer({
            "publish_post": '{"error":"the keystore at /x/key is readable by other accounts"}'
        })
        c.draft = "something"
        c.submit()

        compare(c.outcome, "refused")
        verify(c.outcomeDetail.indexOf("readable by other accounts") >= 0,
               "core's message must reach the view, got: " + c.outcomeDetail)
        verify(spec.renderedText(c).indexOf("readable by other accounts") >= 0,
               "and must reach the screen")
        c.destroy()
    }

    // ---- a refusal keeps the draft AND the retry ------------------------

    function test_a_refusal_keeps_the_draft_and_a_retry_sends_the_same_body() {
        // The two halves of the requirement in one test, because the second is
        // what makes the first mean something: a draft retained in a composer
        // that will not submit again is a draft the user has to retype anyway.
        var c = makeComposer({
            "publish_post": '{"error":"no such op is held by this peer"}'
        })

        var typed = "a reply to something that has not arrived yet"
        c.draft = typed
        c.submit()

        compare(c.outcome, "refused")
        compare(c.draft, typed, "the draft must be unchanged")
        compare(c.submittable, true, "and a retry must remain available")

        var firstBody = JSON.parse(spec.lastCall.args[0]).body
        c.submit()
        var secondBody = JSON.parse(spec.lastCall.args[0]).body
        compare(secondBody, firstBody,
                "a retry must send the identical body")
        compare(secondBody, typed)
        c.destroy()
    }

    function test_no_reply_refusal_blames_the_user_or_claims_permanence() {
        // Both reply refusals — "parent not held" and "target is not a post" —
        // reach the view as differing prose in ONE error shape with no
        // discriminant, so the view cannot tell them apart and must not try.
        // What it supplies alongside core's message therefore has to be safe
        // under either reading.
        var notHeld = makeComposer({
            "publish_reply": '{"error":"no such op is held by this peer"}'
        }, { kind: "reply", parentOp: "cc".repeat(32) })
        var notAPost = makeComposer({
            "publish_reply": '{"error":"the op held for that id is a Vote, not a Post"}'
        }, { kind: "reply", parentOp: "cc".repeat(32) })

        notHeld.draft = "a reply"; notHeld.submit()
        notAPost.draft = "a reply"; notAPost.submit()

        // Both keep the draft and a retry — the reading that is safe either way.
        compare(notHeld.draft, "a reply")
        compare(notAPost.draft, "a reply")
        compare(notHeld.submittable, true)
        compare(notAPost.submittable, true,
                "a retry must be offered even here: the view has no way to "
                + "establish that retrying cannot help")

        // And the view's own words blame nobody and promise nothing permanent.
        // Core's message is excluded from this check by construction — it is
        // asserted separately — because these are claims the VIEW supplies.
        var blaming = ["you did", "your mistake", "invalid", "permanently",
                       "cannot be retried", "will never", "you cannot fix"]
        var mine = [notHeld, notAPost]
        for (var i = 0; i < mine.length; i++) {
            var shown = spec.renderedText(mine[i]).toLowerCase()
            // Core's own text is in there too; strip it so this asserts only
            // what the view wrote.
            shown = shown.replace(mine[i].outcomeDetail.toLowerCase(), "")
            for (var j = 0; j < blaming.length; j++) {
                verify(shown.indexOf(blaming[j]) < 0,
                       "the view must not say '" + blaming[j] + "', got: " + shown)
            }
        }

        // The two are presented the SAME way — no branch was taken on the
        // message's wording. The only difference between the two renderings
        // must be core's own message.
        var a = spec.renderedText(notHeld).replace(notHeld.outcomeDetail, "")
        var b = spec.renderedText(notAPost).replace(notAPost.outcomeDetail, "")
        compare(a, b, "the view must not branch on core's message text")

        notHeld.destroy(); notAPost.destroy()
    }

    // ---- the call shapes ------------------------------------------------

    function test_a_post_and_a_reply_call_different_methods_with_the_right_fields() {
        var post = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' })
        post.draft = "top level"
        post.submit()
        compare(spec.lastCall.method, "publish_post")
        var postArgs = JSON.parse(spec.lastCall.args[0])
        compare(postArgs.body, "top level")
        compare(postArgs.parent, undefined,
                "a post must not name a parent")
        post.destroy()

        var reply = makeComposer({ "publish_reply": '{"opId":"bb","wasNew":true}' },
                                 { kind: "reply", parentOp: "cc".repeat(32) })
        reply.draft = "a reply"
        reply.submit()
        compare(spec.lastCall.method, "publish_reply")
        var replyArgs = JSON.parse(spec.lastCall.args[0])
        compare(replyArgs.parent, "cc".repeat(32))
        compare(replyArgs.body, "a reply")
        compare(replyArgs.thread, undefined,
                "core derives the thread from the parent and REFUSES a request "
                + "naming one — sending a thread would be refused every time")
        reply.destroy()
    }

    function test_the_published_signal_fires_on_a_success_and_not_on_a_refusal() {
        // The signal is what drives the feed's re-read, so a refusal firing it
        // would re-read for nothing and a success not firing it would leave the
        // feed stale behind a success message.
        var ok = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' })
        var okSpy = signalSpy.createObject(null, { target: ok, signalName: "published" })
        ok.draft = "x"; ok.submit()
        compare(okSpy.count, 1)
        okSpy.destroy(); ok.destroy()

        var bad = makeComposer({ "publish_post": '{"error":"nope"}' })
        var badSpy = signalSpy.createObject(null, { target: bad, signalName: "published" })
        bad.draft = "x"; bad.submit()
        compare(badSpy.count, 0, "a refusal must not trigger a re-read")
        badSpy.destroy(); bad.destroy()
    }

    Component {
        id: signalSpy
        SignalSpy {}
    }
}
