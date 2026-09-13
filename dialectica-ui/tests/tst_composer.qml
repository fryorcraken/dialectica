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

    // ---- the pre-submission warnings, asserted on the SCREEN -------------
    //
    // **The defect this file's own header warns about, found in this file.**
    //
    // Both warnings were pinned by the value feeding their binding —
    // `invisibleCount`, `overLimit` — and never by anything rendered. A reviewer
    // set each warning's `visible:` to `false` and the whole suite stayed green:
    // a user pasting bidirectional overrides gets no warning, a user over the cap
    // gets a greyed-out button and no explanation, and nothing fails.
    //
    // The spec is explicit that the obligation is on the rendering: "the view
    // **displays** a warning naming how many were found" (spec.md:211) and "the
    // view **reports** that it is too long" (spec.md:244). A count held in a
    // property displays nothing.
    //
    // Worth stating plainly rather than quietly repairing: documenting a defect
    // family in a header is not the same as being immune to it. This file's
    // header describes exactly this shape — assert the rendered property, not the
    // source — and the file then did it twice. The check that catches it is
    // cheap; the habit of reaching for the property is what needs interrupting.
    //
    // These read `renderedText`, which only concatenates a `Text` whose `visible`
    // is not false, so a hidden warning leaves nothing to find.

    function test_the_invisible_warning_is_on_the_screen_and_names_the_count() {
        var c = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' })

        c.draft = "safe‮text​more﻿end"
        compare(c.invisibleCount, 3, "one override, one ZWSP, one BOM")

        // **The half the value assertion above cannot reach.** The spec requires
        // the warning to NAME how many were found, so the count has to be in the
        // text rather than merely correct in a property.
        var shown = spec.renderedText(c)
        verify(shown.indexOf("invisible character") >= 0,
               "the warning must be displayed, not merely computed. Got: " + shown)
        verify(shown.indexOf("3 invisible character") >= 0,
               "and must name how many were found. Got: " + shown)

        compare(c.submittable, true, "the warning must NOT be a gate")
        c.destroy()
    }

    function test_a_clean_draft_displays_no_invisible_warning() {
        // The negative bound. Without it, a component that displayed the warning
        // unconditionally would satisfy the test above — and a warning on every
        // draft trains the reader to ignore the one that matters.
        var c = makeComposer({})
        c.draft = "An ordinary post, with punctuation — and a dash.\nAnd a newline."
        compare(c.invisibleCount, 0,
                "ordinary text and whitespace are content, not controls")
        verify(spec.renderedText(c).indexOf("invisible character") < 0,
               "a clean draft must display no warning at all")
        c.destroy()
    }

    function test_the_over_length_warning_is_on_the_screen() {
        // Same shape, the other warning. `overLimit` being true is what drives
        // the binding; it is not what the user reads.
        var c = makeComposer({}, { bodyByteLimit: 10 })
        c.draft = "far more than ten bytes of text"

        compare(c.overLimit, true, "precondition: the draft is over the cap")

        var shown = spec.renderedText(c)
        verify(shown.indexOf("longer than") >= 0,
               "the view must REPORT that the draft is too long, not merely "
               + "compute it. Got: " + shown)

        // And it says the draft was not truncated, which is the half the user
        // most needs: a greyed-out button with no text leaves them guessing
        // whether their words are still there.
        verify(shown.indexOf("Nothing has been removed") >= 0,
               "and must say nothing was removed from what they wrote. Got: " + shown)
        c.destroy()
    }

    function test_a_draft_within_the_limit_displays_no_over_length_warning() {
        // The negative bound for the over-length warning.
        var c = makeComposer({}, { bodyByteLimit: 100 })
        c.draft = "short"
        compare(c.overLimit, false, "precondition: within the cap")
        verify(spec.renderedText(c).indexOf("longer than") < 0,
               "a draft within the limit must display no over-length warning")
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

    // ---- what happens to the draft, per outcome -------------------------
    //
    // **A whole requirement that had no test in either direction.**
    //
    // spec.md:336-372 makes the three-way asymmetry the decision rather than an
    // inconsistency: cleared on a newly stored op, retained on a deduplicated one
    // and on every refusal. A reviewer deleted `clearDraft()` from the stored arm
    // (green), then added it to the `existing` arm — which the requirement's own
    // rationale forbids, "clearing would take away exactly what they need"
    // (green). Both halves of the asymmetry could be inverted with 124 tests
    // passing, and the failure mode is silent data loss.
    //
    // A table, because the requirement is one rule over three outcomes and three
    // near-identical functions would hide which one drifted. The reply shape and
    // the expected draft are the two things that vary; everything else is the
    // same submission.
    function draftFateCases() {
        return [
            {
                tag: "stored",
                reply: '{"opId":"aa","wasNew":true}',
                // Cleared. Nothing is lost — the text is published and readable —
                // and a draft left in the box is one the user can submit again,
                // where the second submission is a deduplicated no-op reported as
                // "already published": a confusing state reached through a
                // control that looked ready.
                expectDraft: ""
            },
            {
                tag: "existing",
                reply: '{"opId":"aa","wasNew":false}',
                // **Retained, and this is the half most likely to be "tidied"
                // into consistency with the arm above.** Nothing new was written,
                // so there is nothing to go and look at; and a user whose
                // intention was to publish something different needs the text in
                // front of them to edit.
                expectDraft: "what the user wrote"
            },
            {
                tag: "refused",
                reply: '{"error":"no such op is held by this peer"}',
                // Retained. Nothing was published, so the draft is the only copy.
                expectDraft: "what the user wrote"
            }
        ]
    }

    function test_the_drafts_fate_differs_across_the_three_outcomes() {
        var typed = "what the user wrote"
        var cases = spec.draftFateCases()

        // Collected first, asserted after, so the failure names every outcome
        // that drifted rather than only the first.
        var seen = []
        for (var i = 0; i < cases.length; i++) {
            var k = cases[i]
            var c = makeComposer({ "publish_post": k.reply })
            c.draft = typed
            c.submit()

            compare(c.draft, k.expectDraft,
                    "after a '" + k.tag + "' publish the composer must hold "
                    + JSON.stringify(k.expectDraft))
            seen.push(c.draft)
            c.destroy()
        }

        // **The asymmetry stated as a relation, not only as three values.** The
        // spec's scenario is "the draft's fate DIFFERS across the three
        // outcomes", and a component that cleared on all three — or on none —
        // satisfies a naive reading of each row while destroying the rule. This
        // is the assertion that fails for the reviewer's mutation B, where the
        // `existing` arm was made to clear like the stored one.
        verify(seen[0] !== seen[1],
               "a newly stored op and a deduplicated one must NOT leave the "
               + "composer in the same state — that asymmetry is the decision");
        compare(seen[1], seen[2],
               "a deduplicated publish and a refusal both retain the draft, "
               + "for the same reason applied to different facts")
    }

    function test_a_cleared_draft_is_cleared_rather_than_merely_shorter() {
        // The boundary the table above states but is worth pinning alone: after
        // a stored publish the composer holds NOTHING, and the submit affordance
        // goes with it. A composer holding "" that still offered a button would
        // publish an empty body on the next press.
        var c = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' })
        c.draft = "something worth publishing"
        c.submit()

        compare(c.draft, "", "the draft is cleared")
        compare(c.draft.length, 0)
        compare(c.submittable, false,
                "and an empty composer offers nothing to submit")
        c.destroy()
    }

    function test_a_retained_draft_is_submittable_again_unchanged() {
        // The other side: a retained draft is not merely present but usable, and
        // the retry sends the identical bytes. A draft retained in a composer
        // that will not submit again is a draft the user has to retype anyway.
        var typed = "identical on both attempts"
        var c = makeComposer({ "publish_post": '{"opId":"aa","wasNew":false}' })
        c.draft = typed
        c.submit()

        compare(c.draft, typed, "a deduplicated publish retains the draft")
        compare(c.submittable, true, "and it remains submittable")

        var firstBody = JSON.parse(spec.lastCall.args[0]).body
        c.submit()
        var secondBody = JSON.parse(spec.lastCall.args[0]).body
        compare(secondBody, firstBody, "and a resubmission sends the same body")
        compare(secondBody, typed)
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

    // **The second copy of the required denial, and the reason there are two.**
    //
    // QtTest spec files are separate QML documents with no shared scope, so a
    // helper in `tst_composer_claims.qml` is not reachable here. The sentence is
    // therefore written out twice, which is exactly the duplication that rots —
    // so `test_the_pinned_denial_is_spelled_the_same_way_in_both_files` below
    // fails the moment the two disagree, and names both spellings when it does.
    //
    // Do NOT update this to match a changed component. It is the spec's required
    // sentence; a change to it is a change to what the interface promises, and
    // the pin in the other file is the one that reports it.
    function deliveryDenial() {
        return "Whether any other peer has received it is not something this "
             + "software can tell you yet."
    }

    // The drift guard for that duplication.
    //
    // It cannot read the other spec file, so it does the next best thing and
    // pins this copy against the COMPONENT — which is legitimate here in a way
    // "asking the implementation what it wrote" is not, because the string is
    // independently pinned as a literal in `tst_composer_claims.qml`. Two
    // independent checks against one hardcoded sentence: if the component
    // changes, this fails; if only this file's copy changes, this fails; if only
    // the other file's copy changes, its own residue check fails.
    function test_the_pinned_denial_is_spelled_the_same_way_in_both_files() {
        var c = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' })
        c.draft = "something"
        c.submit()

        verify(spec.renderedText(c).indexOf(spec.deliveryDenial()) >= 0,
               "this file's copy of the required denial must be the sentence the "
               + "component actually renders — if this fails, the two test files' "
               + "copies have drifted or the component was reworded. Expected: "
               + JSON.stringify(spec.deliveryDenial())
               + " in: " + spec.renderedText(c))
        c.destroy()
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
        // **The denial is removed before the sweep runs, and the old comment
        // here described the bug rather than a design.** It used to say the
        // needles were "written against the affirmative forms" so that the
        // denial's own "has received it" would slip past — which is not a
        // property of the needles, it is an accident of spelling. Measured: the
        // denial reworded to "Whether it **was received by** any other peer..."
        // — semantically identical, still a denial — makes this test report a
        // delivery claim. A needle phrased as a bare participle cannot separate
        // a claim from its negation, and the required sentence IS a negation
        // built from that vocabulary.
        //
        // The sentence is pinned character-for-character by
        // `tst_composer_claims.qml::test_the_views_own_words_are_exactly_these_and_no_others`,
        // so removing it here loses nothing: a claim smuggled into it fails that
        // test first. It is spelled out rather than imported because QtTest spec
        // files do not share scope — and the two copies are kept honest by
        // `test_the_pinned_denial_is_spelled_the_same_way_in_both_files` below,
        // which fails if they drift.
        var swept = shown.split(spec.deliveryDenial().toLowerCase()).join("")

        var forbidden = ["was sent", "was delivered", "was received",
                         "has been sent", "has been delivered", "has received",
                         "received by", "delivered to",
                         "everyone can see", "others can see", "peers reached",
                         "published to the stoa"]
        for (var i = 0; i < forbidden.length; i++) {
            verify(swept.indexOf(forbidden[i]) < 0,
                   "a success must not claim '" + forbidden[i] + "', got: " + swept)
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

        // **The blaming-phrase grep that used to live here has been removed, and
        // where it went matters.**
        //
        // It stripped `outcomeDetail` from the rendering and searched the
        // remainder for seven phrases. A blaming sentence phrased differently —
        // "you should have checked the parent first", say — contains none of the
        // seven and passed. A list of forbidden phrasings cannot constrain a
        // sentence nobody on the list anticipated, which is the whole failure
        // mode: the test reported safety for wordings it had never considered.
        //
        // `tst_composer_claims.qml` replaces it with the inverse assertion —
        // the view's own sentences are pinned EXACTLY, and anything else in the
        // rendering is a failure. That constrains every reword rather than seven
        // of them. This test keeps the half that was always the stronger one.

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

    function test_a_post_given_a_parent_does_not_carry_it_anywhere() {
        // The comment on `parentOp` used to claim "a post must not have one" as
        // though the component enforced it. It did not: a stray parent was
        // accepted and silently dropped. `replyParent` makes the claim true —
        // a post's parent is not ignored, it is not reachable.
        var post = makeComposer({ "publish_post": '{"opId":"aa","wasNew":true}' },
                                { kind: "post", parentOp: "dd".repeat(32) })

        compare(post.parentOp, "dd".repeat(32), "the property still holds it")
        compare(post.replyParent, "",
                "but a post's effective parent is empty whatever it holds")

        post.draft = "top level"
        post.submit()

        compare(spec.lastCall.method, "publish_post")
        var sent = JSON.parse(spec.lastCall.args[0])
        compare(sent.parent, undefined,
                "no parent may reach core on a post — core refuses a post "
                + "naming one, so a stray parent would be a refusal the user "
                + "could not explain")
        post.destroy()
    }

    function test_a_reply_still_carries_its_parent() {
        // The negative control: `replyParent` must not have made every parent
        // unreachable, which would pass the assertion above for the wrong
        // reason and break replying entirely.
        var reply = makeComposer({ "publish_reply": '{"opId":"bb","wasNew":true}' },
                                 { kind: "reply", parentOp: "cc".repeat(32) })
        compare(reply.replyParent, "cc".repeat(32))

        reply.draft = "a reply"
        reply.submit()

        var sent = JSON.parse(spec.lastCall.args[0])
        compare(sent.parent, "cc".repeat(32), "a reply's parent must reach core")
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
