import QtQuick
import QtTest
import "../src/qml"

// **Can the user type something in, and can they send it?**
//
// The spec's gate requirement is not about a component's type name. It is about
// two affordances existing together or not at all: a box that accepts text, and
// a control that submits it. A box the user can type into and not submit loses
// what they wrote, which is why "not a disabled one, not a read-only one, and
// not one that accepts text and refuses submission" is written into the
// requirement rather than left to taste.
//
// ## Why this file exists alongside `tst_vote_and_gate.qml`
//
// That suite's `hasVisibleComposer()` identifies a text input by the presence of
// a `selectByMouse` property. Structural on purpose — it avoids depending on the
// component's internals — but it answers a proxy question: **"is there something
// here with a property TextEdit happens to have"**, not "can the user type". Any
// future component exposing `selectByMouse` — a read-only selectable transcript,
// a code block a reader can copy from — makes a closed gate look open, and the
// test would report the gate as working while the box was gone.
//
// So this file probes the affordance by USING it: it writes text into every
// candidate and keeps only the ones that took the text and are editable. A
// read-only element fails that; a plain `Text` fails that; an element rendered
// invisible by a hidden ancestor fails that.
//
// The two approaches are kept side by side deliberately. The structural one is
// cheap and fails fast; this one cannot be fooled by a property name. They
// disagreeing would itself be worth knowing.
//
// ## What this cannot see
//
// Nothing here drives a mouse or a key event. "The user can type into it" is
// established by a programmatic write plus `readOnly`/`enabled`/`visible`, which
// is a proxy for a keystroke reaching the element and not the thing itself. A
// box covered by an opaque sibling, or positioned off the card, passes every
// assertion in this file. QtTest here has no pixels.
TestCase {
    id: spec
    name: "GateAffordance"

    property var savedBridge: undefined

    function init() { spec.savedBridge = Core.bridge }
    function cleanup() { Core.bridge = spec.savedBridge }

    function bridgeFor(replies) {
        return {
            callModule: function (module, method, args) {
                if (replies[method] === undefined)
                    return '{"error":"no fake reply for ' + method + '"}'
                return replies[method]
            }
        }
    }

    Component { id: feedComponent; FeedScreen {} }

    function twoRows() {
        return '{"items":['
            + '{"thread":"t1","currentVersion":"v1","author":"a1",'
            + '"body":{"text":"first","removed":0,"marked":0},'
            + '"attachments":[],"isRevised":false,"isHidden":false}'
            + '],"page":0,"hasMore":false}'
    }

    function makeScreen(probeReply) {
        Core.bridge = spec.bridgeFor({
            "get_capabilities": probeReply,
            "list_threads": spec.twoRows(),
            "publish_post": '{"opId":"aa","wasNew":true}'
        })
        return feedComponent.createObject(null, {
            stoaAddress: "ab".repeat(32),
            stoaGenesis: "00ff",
            stoaTitle: "Agora"
        })
    }

    // ---- the behavioural probe -------------------------------------------

    // Every element in the tree the user could put text into, found by TRYING.
    //
    // The test is: it is visible, it is not read-only, it is enabled, and a
    // write to `.text` sticks. The write is undone before returning, so a caller
    // can probe a tree and then assert on it.
    //
    // `readOnly !== undefined` is what distinguishes a text INPUT from a `Text`:
    // a Text has `text` but no `readOnly`. That is still a property check, but it
    // is the property that decides the requirement — a component exposing
    // `readOnly` and accepting a write IS a box the user can type into,
    // whatever it is called.
    function editableTextTargets(item, acc) {
        var out = acc === undefined ? [] : acc
        if (item === null || item === undefined)
            return out

        if (typeof item.text === "string"
                && item.readOnly !== undefined
                && item.visible === true
                && item.readOnly === false
                && item.enabled !== false) {
            var before = item.text
            var probe = "✓probe✓"
            item.text = probe
            var took = (item.text === probe)
            item.text = before
            if (took)
                out.push(item)
        }

        var kids = item.children
        if (kids !== undefined) {
            for (var i = 0; i < kids.length; i++)
                out = spec.editableTextTargets(kids[i], out)
        }
        return out
    }

    // Every visible control that would submit.
    //
    // Found by its label, hardcoded here rather than read off the component —
    // the two labels the composer actually offers. And restricted to the
    // PRESSABLE element: a `FlatButton` is a Rectangle carrying `text` plus an
    // inner `Text` bound to the same string, so a label match alone counts every
    // button twice. `clicked !== undefined` is what picks the one a user presses;
    // the inner label has no such signal.
    //
    // The label strings are the affordance's promise, and they are checked as
    // strings for a reason beyond identification: "Publish", not "Send". The word
    // is the claim, and a button reading "Send the post" would claim a delivery
    // outcome nothing in this system checks — so a reword to it fails here.
    function submitAffordances(item, acc) {
        var out = acc === undefined ? [] : acc
        if (item === null || item === undefined)
            return out
        if (typeof item.text === "string" && item.visible === true
                && item.clicked !== undefined
                && (item.text === "Publish the post"
                    || item.text === "Publish the reply"))
            out.push(item)
        var kids = item.children
        if (kids !== undefined) {
            for (var i = 0; i < kids.length; i++)
                out = spec.submitAffordances(kids[i], out)
        }
        return out
    }

    // Every text INPUT in the tree, editable or not.
    //
    // `editableTextTargets` above answers "can the user type", and that alone is
    // not the requirement: the spec forbids "not a disabled one, not a read-only
    // one" as well, because a box the user can see and not use reads as a broken
    // feature and invites them to try. So this counts the inputs regardless of
    // their editability, identified by `readOnly` — the property a text input has
    // and a `Text` does not.
    //
    // A read-only box slips past the editable probe by construction (it is not
    // editable), which is precisely why both exist.
    function textInputsOfAnyKind(item, acc) {
        var out = acc === undefined ? [] : acc
        if (item === null || item === undefined)
            return out
        if (typeof item.text === "string" && item.readOnly !== undefined
                && item.visible === true)
            out.push(item)
        var kids = item.children
        if (kids !== undefined) {
            for (var i = 0; i < kids.length; i++)
                out = spec.textInputsOfAnyKind(kids[i], out)
        }
        return out
    }

    // ---- the gate --------------------------------------------------------

    // **A closed gate offers nothing to type into — established by trying to
    // type into everything.**
    //
    // This is the test the `selectByMouse` probe approximates. Against a
    // component that rendered a read-only or disabled box, the structural probe
    // and this one disagree, and this one is right.
    function test_a_closed_gate_offers_nothing_the_user_can_type_into() {
        var shapes = [
            '{"canPost":false,"reason":"No keystore found. Create one before posting."}',
            '{"error":"the keystore could not be read: permission denied"}',
            '{}',
            '{"canPost":"true"}',
            '{"canPost":1}',
            '{"canPost":null}',
            '{"identity":"aa"}',
            'not json at all'
        ]
        for (var i = 0; i < shapes.length; i++) {
            var screen = spec.makeScreen(shapes[i])

            compare(screen.capability.canPost, false,
                    "probe reply " + shapes[i] + " must close the gate")

            var boxes = spec.editableTextTargets(screen)
            compare(boxes.length, 0,
                    "probe reply " + shapes[i] + " must leave NOTHING the user "
                    + "can type into — not a disabled box, not a read-only one. "
                    + "Found " + boxes.length)

            var buttons = spec.submitAffordances(screen)
            compare(buttons.length, 0,
                    "and no submit affordance, found " + buttons.length)

            // **And no text input at all — not even a read-only one.**
            //
            // The spec spells this out: "not a disabled one, not a read-only
            // one, and not one that accepts text and refuses submission". The
            // editable probe above cannot see a read-only box, because a
            // read-only box is by definition not editable; without this
            // assertion a closed gate could render a greyed-out preview field
            // and every other test on this screen would stay green.
            var inputs = spec.textInputsOfAnyKind(screen)
            compare(inputs.length, 0,
                    "probe reply " + shapes[i] + " must render NO text input of "
                    + "any kind, read-only and disabled included. Found "
                    + inputs.length)

            screen.destroy()
        }
    }

    // The positive half, and it is what makes the negative half mean something:
    // a probe that found nothing typeable anywhere would pass the test above
    // against a screen with no composer at all, in any state.
    function test_an_open_gate_offers_exactly_one_box_that_takes_text() {
        var screen = spec.makeScreen('{"canPost":true,"identity":"deadbeef"}')

        var boxes = spec.editableTextTargets(screen)
        compare(boxes.length, 1,
                "an open gate must offer exactly one editable box, found "
                + boxes.length)

        // And that box is the ONLY text input on the screen — so the closed
        // gate's "found zero inputs of any kind" assertion is a real count over
        // a probe that can find one when there is one, rather than a probe that
        // finds nothing anywhere.
        compare(spec.textInputsOfAnyKind(screen).length, 1,
                "and it must be the only text input on the screen")

        // And it genuinely holds what is written to it — the draft the composer
        // publishes IS this field's text, so a box that discarded a write would
        // be a box that loses what the user typed.
        var typed = "what the user wrote"
        boxes[0].text = typed
        compare(boxes[0].text, typed)

        screen.destroy()
    }

    // The submit affordance appears only once there is something to submit, and
    // the two halves are tested together because a box with no way to send is
    // exactly the state the requirement forbids.
    function test_an_open_gate_offers_a_submit_affordance_once_a_draft_exists() {
        var screen = spec.makeScreen('{"canPost":true,"identity":"deadbeef"}')

        // Empty draft: nothing to publish, so no button. Asserted rather than
        // assumed, because it is the precondition for the assertion below
        // meaning "the button appeared" rather than "the button was always there".
        compare(spec.submitAffordances(screen).length, 0,
                "an empty draft offers nothing to submit")

        var boxes = spec.editableTextTargets(screen)
        compare(boxes.length, 1)
        boxes[0].text = "something worth publishing"

        var buttons = spec.submitAffordances(screen)
        compare(buttons.length, 1,
                "a draft must bring a submit affordance with it")

        // **And pressing it publishes what is in the box.**
        //
        // Every other assertion in this file establishes that two things are on
        // screen. This one establishes they are CONNECTED — which is the whole
        // requirement: "not one that accepts text and refuses submission". A box
        // and a button that are not wired to each other satisfy every count
        // above and lose exactly what the requirement exists to protect.
        //
        // The body is read back out of the argument the bridge received rather
        // than off the composer, so what is asserted is what left the view.
        var sentBody = ""
        Core.bridge = {
            callModule: function (module, method, args) {
                if (method === "publish_post") {
                    sentBody = JSON.parse(args[0]).body
                    return '{"opId":"aa","wasNew":true}'
                }
                if (method === "get_capabilities")
                    return '{"canPost":true,"identity":"deadbeef"}'
                return spec.twoRows()
            }
        }
        buttons[0].clicked()
        compare(sentBody, "something worth publishing",
                "pressing the affordance must publish the text in the box")

        screen.destroy()
    }

    // **The gate and the affordance move together, on the SAME screen.**
    //
    // Every test above builds a fresh screen per probe answer, so none of them
    // can see a gate that opened once and never shut. This re-probes an existing
    // screen — which is what `reload()` does on every render — and asserts the
    // box arrives and leaves with the answer.
    //
    // The spec's "SHALL NOT decide this from a probe answer obtained before the
    // current render" is the requirement, and a cached answer is exactly what
    // this catches: a composer left on screen after the key went away is a box
    // whose submissions will all be refused.
    function test_the_box_arrives_and_leaves_with_the_probe_answer() {
        var replies = {
            "get_capabilities": '{"canPost":true,"identity":"deadbeef"}',
            "list_threads": spec.twoRows(),
            "publish_post": '{"opId":"aa","wasNew":true}'
        }
        Core.bridge = spec.bridgeFor(replies)
        var screen = feedComponent.createObject(null, {
            stoaAddress: "ab".repeat(32), stoaGenesis: "00ff", stoaTitle: "Agora"
        })

        compare(spec.editableTextTargets(screen).length, 1,
                "open to begin with")

        // The key goes away between renders. Same screen, same object — only
        // what the probe answers has changed.
        replies["get_capabilities"] =
            '{"canPost":false,"reason":"The keystore is no longer readable."}'
        screen.reload()

        compare(screen.capability.canPost, false)
        compare(spec.editableTextTargets(screen).length, 0,
                "a gate that shut must take the box with it — a composer left "
                + "on screen after the key went away is a box whose submissions "
                + "will all be refused")
        compare(spec.submitAffordances(screen).length, 0,
                "and the submit affordance with it")

        // And back again, so this is not passing because the box was destroyed
        // by the first reload and never rebuilt.
        replies["get_capabilities"] = '{"canPost":true,"identity":"deadbeef"}'
        screen.reload()
        compare(spec.editableTextTargets(screen).length, 1,
                "and it must come back when the probe does")

        screen.destroy()
    }

    // The closed gate's reason reaches the screen and the box does not — the two
    // asserted together, because a screen showing neither is also wrong and each
    // assertion alone permits it.
    function test_a_closed_gate_shows_the_reason_where_the_box_would_have_been() {
        var reason = "Your keystore is readable by other accounts on this machine "
                   + "(mode 0644). Restrict it to owner-only and replace the key."
        var screen = spec.makeScreen(JSON.stringify({ canPost: false, reason: reason }))

        compare(spec.editableTextTargets(screen).length, 0, "no box")
        verify(spec.renderedText(screen).indexOf(reason) >= 0,
               "and core's reason, character for character, in its place")

        screen.destroy()
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
}
