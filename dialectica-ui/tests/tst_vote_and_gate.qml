import QtQuick
import QtTest
import "../src/qml"

// Two things the feed screen must get right around publishing, and they share a
// harness because they share the probe:
//
//   - **The vote control displays no number**, and shows the viewer their own
//     vote back only for votes this view published.
//   - **The gate's copy makes no claim the software cannot support**, which is
//     specifically a test against the bundle's strings: two of them promise
//     delivery, and a future paste of copy.json would reintroduce them.
TestCase {
    id: spec
    name: "VoteAndGate"

    property var savedBridge: undefined
    property var calls: []

    function init() {
        spec.savedBridge = Core.bridge
        spec.calls = []
    }

    function cleanup() {
        Core.bridge = spec.savedBridge
    }

    function bridgeFor(replies) {
        return {
            callModule: function (module, method, args) {
                spec.calls.push({ method: method, args: args })
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    function makeScreen(replies) {
        Core.bridge = bridgeFor(replies)
        return feedComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            stoaTitle: "Agora"
        })
    }

    // Two rows, so "a vote on one post does not mark another" has a second post
    // to be wrong about.
    function twoRows() {
        return '{"items":['
            + '{"thread":"t1","currentVersion":"v1","author":"a1",'
            + '"body":{"text":"first","removed":0,"marked":0},'
            + '"attachments":[],"isRevised":false,"isHidden":false},'
            + '{"thread":"t2","currentVersion":"v2","author":"a2",'
            + '"body":{"text":"second","removed":0,"marked":0},'
            + '"attachments":[],"isRevised":false,"isHidden":false}'
            + '],"page":0,"hasMore":false}'
    }

    Component {
        id: feedComponent
        FeedScreen {}
    }

    Component {
        id: voteComponent
        VoteControl {}
    }

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

    // ---- the vote control shows no number -------------------------------

    function test_an_unmodified_control_displays_no_number() {
        // **This is the test that fails against the component as it was.**
        // `text: Math.max(0, score)` with `score` defaulting to 0 renders the
        // numeral "0" beside every post — a number, so it reads as a tally, and
        // the tally ZERO specifically: a claim that the post is known to have
        // received no votes. Core has never said that and it is false the moment
        // any peer votes.
        var v = voteComponent.createObject(null, {})

        compare(v.showScore, false,
                "the honest rendering must be the one you get by default")
        var shown = spec.renderedText(v)
        verify(!/[0-9]/.test(shown),
               "no numeral may appear on an unmodified control, got: " + JSON.stringify(shown))
        v.destroy()
    }

    function test_a_control_with_a_vote_still_displays_no_number() {
        // The absence must survive the viewer voting: a control that started
        // showing a number once the user pressed something would be reporting a
        // tally of one, which is a claim about everyone else's votes.
        var before = voteComponent.createObject(null, {})
        var after  = voteComponent.createObject(null, { vote: 1 })

        var a = spec.renderedText(before)
        var b = spec.renderedText(after)
        verify(!/[0-9]/.test(a))
        verify(!/[0-9]/.test(b))
        compare(a, b,
                "the two must be identical in what stands where a score would — "
                + "no number having appeared or changed")
        before.destroy(); after.destroy()
    }

    function test_the_score_property_survives_for_a_later_count_to_bind_to() {
        // `score` is unbound, not deleted: the later arrival of a count must be
        // one binding rather than a change to the component and every call site.
        // Setting it must still change nothing on screen while showScore is off.
        var v = voteComponent.createObject(null, { score: 12 })
        verify(!/[0-9]/.test(spec.renderedText(v)),
               "a score set without showScore must still render nothing")
        compare(v.score, 12, "but the property must hold the value")
        v.destroy()
    }

    // ---- the viewer's own vote ------------------------------------------

    function test_a_published_vote_is_reflected_on_that_posts_control_only() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": spec.twoRows(),
            "publish_vote": '{"opId":"votedop","wasNew":true}'
        })

        compare(screen.ownVotes["v1"], undefined, "nothing voted on yet")

        screen.voteOn("v1", 1)
        compare(screen.ownVotes["v1"], 1, "the direction published must come back")
        compare(screen.ownVotes["v2"], undefined,
                "a vote on one post must not mark another")

        screen.voteOn("v2", -1)
        compare(screen.ownVotes["v2"], -1)
        compare(screen.ownVotes["v1"], 1, "and must not disturb the first")
        screen.destroy()
    }

    function test_a_refused_vote_leaves_the_control_unchanged() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": spec.twoRows(),
            "publish_vote": '{"error":"the keystore is unreadable"}'
        })

        screen.voteOn("v1", 1)
        compare(screen.ownVotes["v1"], undefined,
                "no vote was recorded, so nothing may be shown back")
        screen.destroy()
    }

    function test_a_vote_reply_the_view_cannot_interpret_is_not_recorded() {
        // Same guard as the composer's: "success" must mean more than "no error
        // field", or a control shows a vote that was never published.
        var noId = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": spec.twoRows(),
            "publish_vote": '{"wasNew":true}'
        })
        noId.voteOn("v1", 1)
        compare(noId.ownVotes["v1"], undefined,
                "a reply with no op id must not record a vote")
        noId.destroy()

        var notJson = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": spec.twoRows(),
            "publish_vote": "not json at all"
        })
        notJson.voteOn("v1", 1)
        compare(notJson.ownVotes["v1"], undefined)
        notJson.destroy()
    }

    function test_the_direction_reaches_core_as_up_or_down() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": spec.twoRows(),
            "publish_vote": '{"opId":"x","wasNew":true}'
        })

        screen.voteOn("v1", 1)
        var up = JSON.parse(spec.calls[spec.calls.length - 1].args[0])
        compare(up.direction, "up")
        compare(up.target, "v1")

        screen.voteOn("v2", -1)
        var down = JSON.parse(spec.calls[spec.calls.length - 1].args[0])
        compare(down.direction, "down")
        screen.destroy()
    }

    function test_a_reload_forgets_the_viewers_votes() {
        // The cost of the design, asserted rather than absorbed: nothing reads
        // votes back, so a control that appeared to remember across a reload
        // would be the view inventing state core never reported.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": spec.twoRows(),
            "publish_vote": '{"opId":"x","wasNew":true}'
        })
        screen.voteOn("v1", 1)
        compare(screen.ownVotes["v1"], 1)

        var fresh = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": spec.twoRows(),
            "publish_vote": '{"opId":"x","wasNew":true}'
        })
        compare(fresh.ownVotes["v1"], undefined,
                "a newly opened view must show no vote it did not publish")
        screen.destroy(); fresh.destroy()
    }

    function test_no_ordering_is_offered_as_vote_based() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": spec.twoRows()
        })
        for (var i = 0; i < screen.orderings.length; i++) {
            var label = screen.orderings[i].label.toLowerCase()
            verify(label.indexOf("vote") < 0 && label.indexOf("top") < 0
                   && label.indexOf("rank") < 0 && label.indexOf("popular") < 0,
                   "no ordering may be labelled as vote-based while none "
                   + "consults votes, got: " + label)
        }
        screen.destroy()
    }

    // ---- the gate --------------------------------------------------------

    function test_a_closed_gate_renders_no_composer() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"No keystore found. Create one before posting."}',
            "list_threads": spec.twoRows()
        })
        compare(screen.capability.canPost, false)
        verify(!spec.hasVisibleComposer(screen),
               "a closed gate must render NO text input — not a disabled one, "
               + "not a read-only one")
        screen.destroy()
    }

    function test_an_open_gate_renders_a_composer() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"deadbeef"}',
            "list_threads": spec.twoRows()
        })
        verify(spec.hasVisibleComposer(screen),
               "an open gate must render a text input and a submit affordance")
        screen.destroy()
    }

    // Walks for a visible TextEdit. `visible` on a QML item is false when any
    // ancestor is hidden, so this answers "can the user type into anything".
    function hasVisibleComposer(item) {
        if (item === null || item === undefined)
            return false
        // A TextEdit has `selectByMouse`; ordinary Text does not. Checking a
        // property rather than a type name so this does not depend on the
        // component's internal structure.
        if (item.selectByMouse !== undefined && item.visible)
            return true
        var kids = item.children
        if (kids !== undefined) {
            for (var i = 0; i < kids.length; i++) {
                if (spec.hasVisibleComposer(kids[i]))
                    return true
            }
        }
        return false
    }

    function test_a_probe_with_neither_capability_nor_reason_closes_the_gate() {
        // Fail closed on a shape nobody designed for. `canPost !== true` covers
        // absent, "true", 1 and null without a branch per shape.
        var shapes = ['{}', '{"canPost":"true"}', '{"canPost":1}',
                      '{"canPost":null}', '{"identity":"aa"}']
        for (var i = 0; i < shapes.length; i++) {
            var screen = makeScreen({
                "get_capabilities": shapes[i],
                "list_threads": spec.twoRows()
            })
            compare(screen.capability.canPost, false,
                    "probe reply " + shapes[i] + " must close the gate")
            verify(!spec.hasVisibleComposer(screen),
                   "and must render no text input")
            screen.destroy()
        }
    }

    function test_two_different_reasons_both_reach_the_screen_unchanged() {
        // Character-for-character, and the two must be handled identically —
        // no branch having been taken on the text. A view that reworded, or
        // that matched on wording to choose what to show, fails this.
        var first = "No keystore found. Create one before posting."
        var second = "Your keystore is readable by other accounts on this machine (mode 0644). Restrict it to owner-only and replace the key."

        var a = makeScreen({
            "get_capabilities": JSON.stringify({ canPost: false, reason: first }),
            "list_threads": spec.twoRows()
        })
        var b = makeScreen({
            "get_capabilities": JSON.stringify({ canPost: false, reason: second }),
            "list_threads": spec.twoRows()
        })

        compare(a.capability.reason, first, "verbatim, not reworded or truncated")
        compare(b.capability.reason, second)
        verify(spec.renderedText(a).indexOf(first) >= 0,
               "the reason must reach the SCREEN, not just the property")
        verify(spec.renderedText(b).indexOf(second) >= 0)

        // The same behaviour for both: the rendering differs only by the reason.
        var strippedA = spec.renderedText(a).replace(first, "REASON")
        var strippedB = spec.renderedText(b).replace(second, "REASON")
        compare(strippedA, strippedB,
                "the view's behaviour must be the same for both reasons")
        a.destroy(); b.destroy()
    }

    function test_a_failed_probe_shows_the_error_and_closes_the_gate() {
        var screen = makeScreen({
            "get_capabilities": '{"error":"the keystore could not be read: permission denied"}',
            "list_threads": spec.twoRows()
        })
        compare(screen.capability.canPost, false)
        verify(spec.renderedText(screen).indexOf("permission denied") >= 0,
               "the failure must be shown rather than swallowed")
        screen.destroy()
    }

    function test_the_fix_affordance_is_offered_alongside_the_reason() {
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"No keystore found."}',
            "list_threads": spec.twoRows()
        })
        var shown = spec.renderedText(screen)
        verify(shown.indexOf("No keystore found.") >= 0, "the reason")
        // copy.json `compose.fix`, required verbatim.
        verify(shown.indexOf("Show me how to fix it") >= 0,
               "and a route to acting on it, got: " + shown)
        screen.destroy()
    }

    // ---- the copy audit, as a test --------------------------------------

    function test_the_closed_gate_promises_no_delivery() {
        // **The test written against the bundle.** copy.json's `compose.
        // noKeystore` and `compose.badPermissions` both end "the reply box comes
        // back when a reply would actually send" — a promise about DELIVERY. The
        // probe establishes whether a publish would be accepted and stored
        // locally; nothing in this system checks whether anything sends.
        //
        // A future paste of copy.json into the gate reintroduces the string, and
        // this test is what catches it.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"No keystore found. Create one before posting."}',
            "list_threads": spec.twoRows()
        })
        var shown = spec.renderedText(screen).toLowerCase()

        verify(shown.indexOf("would actually send") < 0,
               "the bundle's delivery promise must not be on screen")
        verify(shown.indexOf("comes back when") < 0,
               "nor the clause it sits in")
        verify(shown.indexOf("actually send") < 0,
               "nor any variant of it")
        screen.destroy()
    }

    function test_the_closed_gate_heading_names_what_is_actually_withheld() {
        // copy.json `compose.blockedTitle` is "You cannot reply in this Stoa
        // yet" — which names only replying, while the gate withholds posting and
        // voting too. A heading describing only replying is wrong about what is
        // blocked, and this test fails against that string.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"No keystore found."}',
            "list_threads": spec.twoRows()
        })
        var shown = spec.renderedText(screen)

        verify(shown.indexOf("You cannot reply in this Stoa yet.") < 0,
               "the bundle's heading names only replying and must not be used")

        // What must be true instead: the heading mentions posting, not replying
        // alone. Asserted as a relation rather than as a pinned string, so a
        // reword that keeps the meaning does not fail and a reword that loses it
        // does.
        var headingLine = ""
        var lines = shown.split("\n")
        for (var i = 0; i < lines.length; i++) {
            if (lines[i].indexOf("cannot") >= 0 && lines[i].indexOf("Stoa") >= 0) {
                headingLine = lines[i]
                break
            }
        }
        verify(headingLine !== "", "a heading naming the blockage must exist, got: " + shown)
        verify(headingLine.toLowerCase().indexOf("post") >= 0,
               "the heading must name posting, which the gate also withholds: " + headingLine)
        screen.destroy()
    }

    function test_the_apparatus_string_is_the_bundles_and_is_verbatim() {
        // `compose.apparatus` survived the audit that dropped the two above,
        // because it is a statement about this interface's own design and true
        // of it. Pinned exactly, since this one IS required verbatim.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":false,"reason":"No keystore found."}',
            "list_threads": spec.twoRows()
        })
        verify(spec.renderedText(screen).indexOf(
                   "There is no disabled composer here. A box you could type into "
                   + "and not send would lose what you wrote.") >= 0,
               "compose.apparatus must be present verbatim")
        screen.destroy()
    }

    // ---- no optimistic row ----------------------------------------------

    function test_a_publish_adds_no_row_the_view_composed() {
        // The feed must show what core reported and nothing else. A row the view
        // built would carry sanitiser counts and a revision flag it invented.
        var screen = makeScreen({
            "get_capabilities": '{"canPost":true,"identity":"aa"}',
            "list_threads": spec.twoRows(),
            "publish_post": '{"opId":"newop","wasNew":true}'
        })
        compare(screen.rows.length, 2)

        // Publishing through the screen's own composer, then re-reading against
        // a core that still reports the same two rows.
        screen.reload()
        compare(screen.rows.length, 2,
                "the rows must be exactly the ones the read returned")
        for (var i = 0; i < screen.rows.length; i++) {
            verify(screen.rows[i].thread === "t1" || screen.rows[i].thread === "t2",
                   "no row composed by the view may appear")
        }
        screen.destroy()
    }
}
