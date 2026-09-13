import QtQuick
import QtTest
import "../src/qml"

// What the composer is allowed to SAY, and what it must never say.
//
// The sibling suites pin the composer's state machine (`tst_composer.qml`) and
// the feed's gate and vote control (`tst_vote_and_gate.qml`). This one pins the
// half those two cannot: **the meaning of the three messages a user actually
// reads.**
//
// ## Why a separate file, and what went wrong on the sibling piece
//
// `thread-read` asserted three refusal messages were three DIFFERENT strings. A
// tester then reworded one to actively misinform — it told the reader to wait
// for something that had already arrived — and **both distinguishability tests
// stayed green**, because three misinforming strings are still three distinct
// strings. Distinctness is a property of the string SET; a claim is a property
// of ONE string, and no amount of the former constrains the latter.
//
// So every test here asserts what a particular message must and must not imply,
// with the phrases hardcoded in this file rather than read back off the
// component. A test that took its expectation from `PublishOutcome`'s own
// property would be asking the implementation what it wrote and agreeing.
//
// ## What these tests structurally cannot see
//
// QtTest drives properties and signals, never pixels. So: nothing here checks
// that a refusal is legible, that the three outcomes are visually distinct, that
// the accent colour reads as a warning, or that any of this fits on a screen.
// A message can satisfy every assertion below and be rendered in 4pt grey on
// grey. That is unverified by anything in this repo.
TestCase {
    id: spec
    name: "ComposerClaims"

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

    Component { id: composerComponent; Composer {} }
    Component { id: feedComponent;     FeedScreen {} }
    Component { id: outcomeComponent;  PublishOutcome {} }

    // **Create AND submit before the next composer is made.**
    //
    // `Core` is a singleton, so assigning `Core.bridge` reassigns the ONE bridge
    // every composer shares. Building three composers and then submitting all
    // three runs every submission against the LAST bridge — which reports three
    // outcomes as indistinguishable when they are not, and the symptom looks
    // exactly like a defect in the component. `tst_composer.qml` hit this in its
    // first draft; the shape is easy to reintroduce, so it is spelled out here
    // too rather than cross-referenced.
    function submitAgainst(reply, props) {
        Core.bridge = bridgeFor({ "publish_post": reply, "publish_reply": reply })
        var p = props === undefined ? {} : props
        if (p.stoaAddress === undefined)
            p.stoaAddress = "ab".repeat(32)
        var c = composerComponent.createObject(null, p)
        c.draft = "identical text"
        c.submit()
        return c
    }

    // ---- reading the rendered tree --------------------------------------

    // Every visible Text in the tree, joined. Deliberately reads `text` — the
    // property the user's eye lands on — and is paired everywhere it matters
    // with `everyTextFormatIsPlain()` below, because `text` alone is blind to
    // formatting: a `StyledText` element renders markup that never appears in
    // its source string. That exact gap let a `textFormat` mutation survive on
    // the `ui-stoa-list` piece.
    function renderedText(item, acc) {
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

    // Every element in the tree that both HOLDS text and declares a format.
    // Returns the list of offenders, so a failure names which one drifted.
    function nonPlainTextElements(item, acc) {
        var out = acc === undefined ? [] : acc
        if (item === null || item === undefined)
            return out
        if (typeof item.text === "string" && item.textFormat !== undefined) {
            // Text.PlainText and TextEdit.PlainText are both 0. Anything else —
            // RichText (1), AutoText (2), StyledText (4), MarkdownText (8) —
            // renders its source as markup.
            if (item.textFormat !== 0)
                out.push(JSON.stringify(item.text) + " has textFormat " + item.textFormat)
        }
        var kids = item.children
        if (kids !== undefined) {
            for (var i = 0; i < kids.length; i++)
                out = spec.nonPlainTextElements(kids[i], out)
        }
        return out
    }

    // ---- the claims table ------------------------------------------------
    //
    // **This is the table the sibling piece was missing.** One row per outcome,
    // naming what that outcome's rendering MUST imply and what it MUST NOT.
    //
    // The phrases are hardcoded here on purpose. They are not read off the
    // component, not derived from it, and must not be "updated to match" a
    // reworded message: a reword that loses one of these meanings is exactly the
    // failure this table exists to report. If a reword is genuinely right, the
    // spec requirement it answers to is what changes first.
    //
    // `mustSayOneOf` is a disjunction so that an honest reword survives; the
    // meaning is pinned, not the sentence. `mustNotSay` is a conjunction: each
    // is a claim no part of this system has established, and none of them has an
    // honest phrasing.
    function claimCases() {
        return [
            {
                tag: "stored",
                reply: '{"opId":"deadbeef","wasNew":true}',
                // It happened, and it happened HERE. The spec requires the
                // message to state the content was saved on this machine.
                mustSayOneOf: [["saved on this machine", "stored on this machine",
                                "in this machine's log"]],
                // Every one of these is a delivery claim, and the reply core
                // sends carries no delivery outcome at all. "will be sent" is in
                // the list because a future-tense promise is the reassuring
                // reword most likely to be added later: nothing schedules a
                // send, so it is a claim about the future with nothing behind it.
                mustNotSay: ["was sent", "has been sent", "will be sent",
                             "is being sent", "sending",
                             "was delivered", "has been delivered", "delivered to",
                             "was received", "has been received",
                             "was propagated", "propagated to",
                             "published to the stoa", "everyone can", "others can see",
                             "other peers can see", "peers reached", "peers received",
                             "now visible to", "visible to everyone",
                             "your post is below", "is now below", "appears below",
                             "scroll", "highlighted below"]
            },
            {
                tag: "existing",
                reply: '{"opId":"deadbeef","wasNew":false}',
                // **The hardest of the three, and the reason this table exists.**
                //
                // `wasNew:false` must read as neither a fresh success nor a
                // failure. Nothing failed, so a refusal is wrong. Nothing new was
                // written, so reporting a fresh success leaves the user watching
                // for a post that will never appear.
                //
                // Two disjunctions, both required: it must say the content is
                // ALREADY there, and it must say nothing NEW happened. A message
                // saying only the first ("this was already published") without
                // the second passes a distinctness test and still leaves a reader
                // who thinks "published — good, it went out just now".
                mustSayOneOf: [["already published", "already in this machine",
                                "already exists"],
                               ["nothing new was written", "nothing new was stored",
                                "nothing was written"]],
                // It must not read as a fresh store, and it must not read as a
                // failure. "failed", "could not", "error" and "refused" are here
                // because the spec explicitly forbids reporting this as an error
                // state, and "just now"/"was saved" because reporting a fresh
                // success is the other half of the same requirement.
                mustNotSay: ["was saved on this machine", "just now",
                             "failed", "could not", "was not published",
                             "error", "refused", "rejected", "try again",
                             "was sent", "was delivered"]
            },
            {
                tag: "refused",
                reply: '{"error":"no such op is held by this peer"}',
                // A refusal owes the user three things: that nothing was
                // published, that their text survived, and that a retry exists.
                mustSayOneOf: [["was not published", "nothing was published"],
                               ["still here", "still in the box", "what you wrote is"],
                               ["try again", "you can retry"]],
                // **The blame-and-permanence list, and the point of putting it in
                // a table.** The view CANNOT tell a parent-not-yet-arrived
                // refusal from a target-is-not-a-post one — one error shape, no
                // discriminant. The common case is nobody's fault and is expected
                // to succeed on a retry, so the view's own words must not blame
                // the user or claim permanence.
                mustNotSay: ["you did", "your mistake", "your fault",
                             "invalid", "permanent", "permanently",
                             "cannot be retried", "will never", "you cannot fix",
                             "not allowed", "you are not", "incorrect",
                             "gave up", "abandoned",
                             // and it must not claim a success it did not have
                             "was saved on this machine", "was published successfully"]
            }
        ]
    }

    // **The test the `thread-read` reword would have failed.**
    //
    // The sibling piece's distinctness tests stayed green against a message that
    // actively misinformed, because distinctness cannot see meaning. This walks
    // the table above and asserts each outcome's rendering against what it must
    // and must not imply — one assertion per claim, so a rendering breaking three
    // rules reports three names rather than the first.
    function test_each_outcome_implies_what_it_must_and_denies_what_it_must_not() {
        var cases = spec.claimCases()
        for (var i = 0; i < cases.length; i++) {
            var k = cases[i]
            var c = spec.submitAgainst(k.reply)
            var shown = spec.renderedText(c).toLowerCase()

            for (var m = 0; m < k.mustSayOneOf.length; m++) {
                var alts = k.mustSayOneOf[m]
                var found = false
                for (var a = 0; a < alts.length; a++) {
                    if (shown.indexOf(alts[a]) >= 0) {
                        found = true
                        break
                    }
                }
                verify(found,
                       "the '" + k.tag + "' outcome must say one of "
                       + JSON.stringify(alts) + ", got: " + shown)
            }

            for (var n = 0; n < k.mustNotSay.length; n++) {
                verify(shown.indexOf(k.mustNotSay[n]) < 0,
                       "the '" + k.tag + "' outcome must not say '"
                       + k.mustNotSay[n] + "', got: " + shown)
            }
            c.destroy()
        }
    }

    // The deduplicated outcome, asserted as the RELATION the spec states rather
    // than as three strings being unequal.
    //
    // A user reaching this state has to end up in neither of the other two
    // beliefs. Distinctness of the state string is already covered in
    // `tst_composer.qml`; what is covered here is that the *sentence a reader
    // reads* does not carry the other two outcomes' load-bearing claims — which
    // is a different property, and the one a reword can silently break.
    function test_a_deduplicated_publish_reads_as_neither_a_fresh_success_nor_a_failure() {
        var fresh = spec.submitAgainst('{"opId":"aa","wasNew":true}')
        var dedup = spec.submitAgainst('{"opId":"aa","wasNew":false}')
        var bad   = spec.submitAgainst('{"error":"the keystore is unreadable"}')

        var freshText = spec.renderedText(fresh).toLowerCase()
        var dedupText = spec.renderedText(dedup).toLowerCase()
        var badText   = spec.renderedText(bad).toLowerCase()

        // The sentence each of the other two leans on, taken from each of THEM
        // rather than hardcoded — so this half stays true through an honest
        // reword of either — and then asserted absent from the middle one.
        verify(freshText.indexOf("saved on this machine") >= 0,
               "precondition: a fresh store says so, got: " + freshText)
        verify(dedupText.indexOf("saved on this machine") < 0,
               "a deduplicated publish must not borrow the fresh-store claim: "
               + dedupText)

        verify(badText.indexOf("was not published") >= 0,
               "precondition: a refusal says nothing was published, got: " + badText)
        verify(dedupText.indexOf("was not published") < 0,
               "a deduplicated publish must not read as a failure: " + dedupText)

        // And the outcome it reached is not the refused one — the spec states
        // this separately from the message, because a view could render the
        // right words in the wrong state (an error border round a success).
        compare(dedup.outcome, "existing")
        verify(dedup.outcome !== bad.outcome,
               "an already-published op is not an error state")

        fresh.destroy(); dedup.destroy(); bad.destroy()
    }

    // ---- the delivery claim, across every state the composer can be in ----
    //
    // The spec forbids claiming delivery after a publish. The gate's copy is
    // forbidden the same claim BEFORE one. This checks the whole composer tree
    // in all four of its states with one list, rather than only the success
    // message — a reassuring sentence added to the over-limit warning or the
    // invisibles warning would satisfy every other test in this suite.
    //
    // **On how strong this is, plainly: it is an absence assertion and those are
    // the weakest kind.** It catches a forbidden phrase from a fixed list. A
    // sentence claiming delivery in words nobody listed — "it's on its way", "in
    // flight", "the network has it" — passes. The list is written against the
    // phrasings a reassuring reword actually reaches for, and it is a filter, not
    // a proof. The real defect underneath (a maximal legal post is refused by
    // every receiving peer, silently) cannot be seen by any test in this repo,
    // because nothing here has a second peer.
    function deliveryClaims() {
        return ["was sent", "has been sent", "will be sent", "is being sent",
                "sent to", "sending",
                // The affordance LABEL, not only the messages. "Publish", not
                // "Send": the word is the claim, and a button reading "Send the
                // post" promises a delivery outcome before anything is even
                // submitted. Found by mutation — an earlier draft of this list
                // had every past-tense form and missed the imperative, which is
                // the phrasing a button actually uses.
                "send the ", "send this ", "send it",
                "was delivered", "has been delivered", "will be delivered",
                "delivered to", "delivery",
                "was received", "has been received", "received by",
                "propagated", "broadcast", "transmitted",
                "would actually send", "actually send", "comes back when",
                "peers reached", "peers received", "reached the network",
                "on the network", "in flight", "on its way",
                "everyone can see", "others can see", "other peers can see",
                "visible to everyone", "now visible to"]
    }

    function test_no_composer_state_claims_delivery() {
        var claims = spec.deliveryClaims()

        // Every state the composer can render: the four outcomes, and the two
        // pre-submission warnings. Built as a table so a fifth state added later
        // is one row rather than a fifth near-identical test function.
        var states = [
            { tag: "untouched",  build: function () { return spec.submitAgainstNothing("") } },
            { tag: "typing",     build: function () { return spec.submitAgainstNothing("an ordinary draft") } },
            { tag: "over-limit", build: function () { return spec.overLimitComposer() } },
            { tag: "invisibles", build: function () { return spec.submitAgainstNothing("safe‮text") } },
            { tag: "stored",     build: function () { return spec.submitAgainst('{"opId":"aa","wasNew":true}') } },
            { tag: "existing",   build: function () { return spec.submitAgainst('{"opId":"aa","wasNew":false}') } },
            { tag: "refused",    build: function () { return spec.submitAgainst('{"error":"the keystore is unreadable"}') } }
        ]

        for (var i = 0; i < states.length; i++) {
            var c = states[i].build()
            var shown = spec.renderedText(c).toLowerCase()
            for (var j = 0; j < claims.length; j++) {
                verify(shown.indexOf(claims[j]) < 0,
                       "the '" + states[i].tag + "' state must not claim '"
                       + claims[j] + "', got: " + shown)
            }
            c.destroy()
        }
    }

    function submitAgainstNothing(draft) {
        Core.bridge = spec.bridgeFor({})
        var c = composerComponent.createObject(null, { stoaAddress: "ab".repeat(32) })
        c.draft = draft
        return c
    }

    function overLimitComposer() {
        Core.bridge = spec.bridgeFor({})
        var c = composerComponent.createObject(null,
            { stoaAddress: "ab".repeat(32), bodyByteLimit: 4 })
        c.draft = "well over four bytes"
        return c
    }

    // The same list against the closed gate, whose copy bundle contains the
    // delivery promise verbatim: `compose.noKeystore` and `compose.badPermissions`
    // both end "the reply box comes back when a reply would actually send". A
    // future paste of copy.json reintroduces it, and the gate's OWN guidance text
    // is the other place a reassuring sentence would land.
    //
    // **Nothing in this file asserts that APPARATUS text is rendered, and that
    // is deliberate.** The right-hand `APPARATUS` column — `ON PUBLISHING`, `ON
    // THE ARROWS`, `ON THE MISSING BOX` and the rest — is annotation from the
    // design bundle explaining the design to a reader. It was shipped into the
    // real QML by mistake and is being removed from the screens. A test asserting
    // one of those notes is present would then fail for the right reason and read
    // as a regression, so every assertion here is an ABSENCE sweep: it is
    // indifferent to whether the apparatus is on screen or gone.
    //
    // The cost of that, stated rather than absorbed: the sweep can only say the
    // interface does not claim delivery, never that it positively DENIES delivery
    // knowledge. The denial currently lives only in an apparatus note, so once
    // the column goes, no test in this repo checks that the honest disclaimer
    // survives anywhere. That is a gap for whoever owns the removal.
    function test_no_gate_state_claims_delivery() {
        var claims = spec.deliveryClaims()

        Core.bridge = spec.bridgeFor({
            "get_capabilities": '{"canPost":false,"reason":"No keystore found. Create one before posting."}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        var shut = feedComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", stoaTitle: "Agora"
        })
        // With the guidance REVEALED: the fix text is hidden until pressed, and
        // a delivery promise hiding behind a disclosure is still on screen for
        // the reader who presses it. A test reading only the collapsed state
        // would pass with the promise sitting one click away.
        shut.showFix = true

        var shownShut = spec.renderedText(shut).toLowerCase()
        verify(shownShut.indexOf("show me how to fix it") < 0
               || shownShut.indexOf("gate is checked again") >= 0,
               "precondition: the guidance must actually be revealed, got: " + shownShut)
        for (var i = 0; i < claims.length; i++) {
            verify(shownShut.indexOf(claims[i]) < 0,
                   "the closed gate must not claim '" + claims[i] + "', got: " + shownShut)
        }
        shut.destroy()

        // And the OPEN gate.
        Core.bridge = spec.bridgeFor({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": '{"items":[],"page":0,"hasMore":false}'
        })
        var open = feedComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", stoaTitle: "Agora"
        })
        var shownOpen = spec.renderedText(open).toLowerCase()

        for (var j = 0; j < claims.length; j++) {
            // The right-hand APPARATUS column currently renders a note ending
            // "nothing in this interface will tell you a post was delivered" —
            // a DENIAL, not a claim, but it contains the word this sweep looks
            // for, so that one clause is removed rather than the check being
            // weakened for every state.
            //
            // **Written as a removal that no-ops when the clause is absent**,
            // deliberately. The apparatus column is annotation from the design
            // bundle that was shipped into the QML by mistake and is being taken
            // out of the screens; a `replace` of a string that is not there
            // changes nothing, so this sweep keeps working either way. Nothing
            // in this file asserts apparatus text is PRESENT — see the note on
            // `test_no_gate_state_claims_delivery` above.
            var stripped = shownOpen.replace(
                "nothing in this interface will tell you a post was delivered.", "")
            verify(stripped.indexOf(claims[j]) < 0,
                   "the open gate must not claim '" + claims[j] + "', got: " + stripped)
        }
        open.destroy()
    }

    // ---- formatting, not just content -----------------------------------
    //
    // **The `ui-stoa-list` defect, applied here before it can be introduced.**
    // On that piece a `textFormat: Text.StyledText` mutation SURVIVED, because
    // the test asserted `Text.text` — the source string, which the format does
    // not change — while the title rendered as live markup.
    //
    // Every assertion in this file reads `text`, so every one of them is blind
    // to that mutation. This is the test that is not.
    //
    // It matters here specifically because a refusal renders CORE'S message
    // verbatim, and a refusal message can carry text a peer influenced. Under
    // StyledText or AutoText — which SNIFFS its input and switches to rich text
    // when the string looks like markup — that message renders as markup.
    function test_every_text_the_composer_renders_is_plain_text() {
        var cases = spec.claimCases()
        for (var i = 0; i < cases.length; i++) {
            var c = spec.submitAgainst(cases[i].reply)
            var bad = spec.nonPlainTextElements(c)
            compare(bad.length, 0,
                    "every Text in the '" + cases[i].tag
                    + "' rendering must be PlainText, offenders: " + JSON.stringify(bad))
            c.destroy()
        }
    }

    // A refusal carrying markup renders as the characters, not as the markup.
    // The assertion is on the format, because `text` is identical either way —
    // asserting `c.outcomeDetail === markup` would pass under StyledText and is
    // precisely the self-consistency check that let the sibling defect through.
    function test_a_refusal_message_containing_markup_is_not_rendered_as_markup() {
        var markup = "<b>no such op</b> is held by this <i>peer</i>"
        var c = spec.submitAgainst(JSON.stringify({ error: markup }))

        // The source reaches the view intact...
        compare(c.outcomeDetail, markup, "core's message must arrive verbatim")
        verify(spec.renderedText(c).indexOf(markup) >= 0,
               "and must reach the screen verbatim")

        // ...and the element holding it renders it as characters. This is the
        // half that fails under a format mutation; the two above do not.
        var bad = spec.nonPlainTextElements(c)
        compare(bad.length, 0,
                "core's message must render as characters, not markup: "
                + JSON.stringify(bad))
        c.destroy()
    }

    // ---- the view's words do not depend on core's --------------------------
    //
    // **Replaces the blaming-phrase grep as the primary guard.**
    //
    // The old test stripped `outcomeDetail` from the rendering and grepped the
    // remainder for seven blaming phrases — so a blaming sentence phrased
    // differently passed it. Its stronger half was the identity check, and this
    // builds on that instead: the view's own contribution to a refusal is pinned
    // to an exact set of sentences, and those sentences are hardcoded here.
    //
    // That converts "does it contain any of seven bad phrases" into "is it
    // exactly this". Any reword — blaming, permanence-claiming, or merely
    // careless — fails, and fixing the failure means a person reading the new
    // sentence against the requirement rather than a grep missing it.
    //
    // The instruction that goes with a pinned string: **do not update these to
    // match a changed component.** They are the requirement's text, and a change
    // to them is a change to what the interface promises.
    //
    // `PublishOutcome` is driven directly rather than through a composer, so the
    // residue check sees only the outcome block. Driven through a composer it
    // would also sweep up the byte counter and the submit label, and the only way
    // to exclude those would be to list them — at which point the check is a
    // phrase list again, which is the thing being replaced.
    //
    // One table, three rows, rather than three near-identical functions: the
    // failure names which outcome drifted, and a fourth outcome is one row.
    // **The one place the required denial's text is written down**, because two
    // consumers need the identical string and they pull in opposite directions:
    // the pin below asserts it is PRESENT, and the delivery sweep must exclude
    // it from the corpus it searches for delivery CLAIMS.
    //
    // The reason the sweep has to exclude it is the interesting half. The
    // required sentence is a statement *about* reception, so it contains the
    // vocabulary of reception — and needles phrased as bare participles
    // ("received by", "was received") cannot separate a claim from its
    // negation. The current wording escapes the list only by the accident of
    // spelling "has received" where the list spells "has been received", which
    // means an equivalent reword would be reported as claiming delivery.
    //
    // Excluding the pinned denial is right rather than a loosening: the sweep's
    // job is to catch a sentence nobody pinned, and this sentence is pinned
    // character-for-character by `test_the_views_own_words_are_exactly_these_and_no_others`.
    // A claim hiding inside it is impossible without that test failing first.
    function deliveryDenial() {
        return "Whether any other peer has received it is not something this "
             + "software can tell you yet."
    }

    function pinnedSentences() {
        return [
            {
                tag: "stored",
                props: { outcome: "stored", detail: "", subject: "post" },
                sentences: [
                    "Your post was saved on this machine.",
                    "It is in this machine's log.",
                    spec.deliveryDenial()
                ]
            },
            {
                tag: "existing",
                props: { outcome: "existing", detail: "", subject: "post" },
                sentences: [
                    "This post was already published.",
                    "The identical content is already in this machine's log, under "
                        + "the same op id. Nothing new was written.",
                    // **Added when the spec promoted the denial from a
                    // prohibition to a positive SHALL.** `wasNew: false` is a
                    // success — nothing failed and the component routes it to a
                    // non-refusal outcome — and the requirement opens "When a
                    // publish succeeds", so it covers this row too.
                    //
                    // This is NOT the forbidden "update the pin to match a
                    // changed component". The order was the other way round: the
                    // spec moved first, and the pin was stale against it.
                    spec.deliveryDenial()
                ]
            },
            {
                tag: "refused",
                props: { outcome: "refused",
                         detail: "no such op is held by this peer",
                         subject: "post" },
                sentences: [
                    "Your post was not published.",
                    "Nothing was published and what you wrote is still here. "
                        + "You can try again."
                ]
            }
        ]
    }

    function test_the_views_own_words_are_exactly_these_and_no_others() {
        var rows = spec.pinnedSentences()
        for (var i = 0; i < rows.length; i++) {
            var r = rows[i]
            var c = outcomeComponent.createObject(null, r.props)
            var shown = spec.renderedText(c)

            // Core's message removed, leaving only what the VIEW supplied.
            var mine = r.props.detail === "" ? shown : shown.replace(r.props.detail, "")

            // Hardcoded. Not read from the component, not derived from it.
            var residue = mine
            for (var j = 0; j < r.sentences.length; j++) {
                verify(mine.indexOf(r.sentences[j]) >= 0,
                       "the '" + r.tag + "' outcome must supply the sentence "
                       + JSON.stringify(r.sentences[j]) + ", got: " + mine)
                residue = residue.replace(r.sentences[j], "")
            }

            // And nothing else of the view's own. Everything left after those
            // sentences and core's message are removed must be whitespace — so a
            // sentence added later fails here rather than needing to trip a
            // phrase list that does not know about it.
            residue = residue.replace(/\s/g, "")
            compare(residue, "",
                    "the '" + r.tag + "' outcome must supply no sentence beyond "
                    + "those pinned above; if one was added deliberately, the "
                    + "requirement it answers to is what changes first. Residue: "
                    + JSON.stringify(residue))
            c.destroy()
        }
    }

    // The empty outcome renders nothing at all. Without this, a component that
    // showed its refusal wording before anything was submitted would pass every
    // other test in this file: they all submit first.
    function test_an_unsubmitted_composer_displays_no_outcome_at_all() {
        var c = outcomeComponent.createObject(null,
            { outcome: "", detail: "", subject: "post" })
        compare(spec.renderedText(c).replace(/\s/g, ""), "",
                "nothing submitted yet must render no outcome text")
        c.destroy()
    }

    // The two reply refusals the view cannot tell apart, presented identically.
    // This is the sibling of the test above and it is kept because it pins a
    // different property: not WHAT the view says, but that it says the SAME
    // thing regardless of which prose core sent — i.e. no branch on wording.
    //
    // The two messages are chosen to be maximally different in the ways a
    // branch would key on: different length, different words, one naming an op
    // type and one not. Two near-identical messages would let a branch that
    // matched on a shared substring survive.
    function test_the_two_reply_refusals_are_rendered_identically_but_for_cores_text() {
        var notHeld = spec.submitAgainst(
            '{"error":"no such op is held by this peer"}',
            { kind: "reply", parentOp: "cc".repeat(32) })
        var notAPost = spec.submitAgainst(
            JSON.stringify({ error: "the op held for that id is a Vote, not a Post, "
                                  + "and a reply may only name a post as its parent" }),
            { kind: "reply", parentOp: "cc".repeat(32) })

        verify(notHeld.outcomeDetail !== notAPost.outcomeDetail,
               "precondition: the two refusals must differ, or this proves nothing")

        var a = spec.renderedText(notHeld).replace(notHeld.outcomeDetail, "CORE")
        var b = spec.renderedText(notAPost).replace(notAPost.outcomeDetail, "CORE")
        compare(a, b, "the view must not branch on core's message text")

        // Both keep the draft and a retry — the reading safe under either, since
        // the view has no discriminant to tell which refusal it received.
        compare(notHeld.draft, "identical text")
        compare(notAPost.draft, "identical text")
        compare(notHeld.submittable, true)
        compare(notAPost.submittable, true)

        notHeld.destroy(); notAPost.destroy()
    }

    // A reply and a post refused by the identical core message differ ONLY in
    // the view's own word for the affordance. Nothing else about a refusal may
    // depend on which was submitted — the spec states every rule about refusals
    // once, for both.
    function test_a_post_and_a_reply_refusal_differ_only_in_the_subject_word() {
        var message = '{"error":"the keystore is unreadable"}'
        var post  = spec.submitAgainst(message)
        var reply = spec.submitAgainst(message, { kind: "reply", parentOp: "cc".repeat(32) })

        var a = spec.renderedText(post).replace(/post/g, "SUBJECT")
        var b = spec.renderedText(reply).replace(/reply/g, "SUBJECT")
        compare(a, b,
                "a post and a reply refusal must be presented the same way")

        // The precondition that makes the substitution meaningful: the two
        // renderings genuinely differ before it. Without this the compare above
        // would pass against a component that ignored `kind` entirely.
        verify(spec.renderedText(post) !== spec.renderedText(reply),
               "precondition: the two must differ before the substitution, or "
               + "this test passes for the wrong reason")

        post.destroy(); reply.destroy()
    }
}
