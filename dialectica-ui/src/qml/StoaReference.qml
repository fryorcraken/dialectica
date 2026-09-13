pragma Singleton
import QtQml

// The shareable thing: what a share produces and what a paste field accepts.
//
// **These are ONE decision seen from each end, which is why they are one file.**
// A share producing something the paste field cannot read produces a string
// whose recipient can do nothing with it, and the failure surfaces on somebody
// else's machine as a refusal they cannot explain. Two implementations, one per
// screen, is exactly how the two ends drift apart.
//
// ---- what is carried, and why both halves --------------------------------
//
// An address is a one-way hash of the genesis record: enough to VERIFY a record
// somebody hands over, and not enough to RECONSTRUCT one. `join_stoa` takes both
// for that reason, so anything shared has to carry both. A share producing only
// an address produces something that looks like it should work and never can.
//
// ---- why JSON ------------------------------------------------------------
//
// It is the encoding both ends of the round trip already have — `JSON.parse`
// here, serde in the core — so neither end needs a parser written for this
// feature, and a parser written for this feature is one that can disagree with
// itself between the copy and the paste.
//
// It is also SELF-DESCRIBING, which decides the failure mode. A user who pastes
// half of a JSON object gets a parse failure that names the input as malformed.
// A positional format (`stoa:<addr>:<record>`) would accept a truncated second
// field as a short record, push it to the core, and come back as a verification
// mismatch — which is the OTHER of the three outcomes the join screen must keep
// apart, and the one that tells a user somebody handed them a wrong record.
QtObject {
    id: root

    // The display prefix the design bundle uses (`stoa:7f3a91c4…`). It is a
    // reading aid and NOT part of the wire: nothing in the core writes or
    // accepts one — a grep of `dialectica/` finds no such literal — and a
    // prefix that survived into `join_stoa` would be a hash verifying against
    // nothing, surfacing as a verification failure rather than as the malformed
    // paste it actually is.
    //
    // So: stripped on the way in, never added on the way out.
    readonly property string displayPrefix: "stoa:"

    function stripPrefix(value) {
        var s = String(value).trim()
        return s.indexOf(root.displayPrefix) === 0
            ? s.slice(root.displayPrefix.length)
            : s
    }

    // What a user copies. Bare hex on both halves, one line.
    //
    // The address is carried IN FULL. The 8-8-6 abbreviation is a recognition
    // aid for a reader looking at a screen; as a thing to be pasted it is lossy,
    // and the abbreviation exists precisely because a head and a tail can be
    // ground to match.
    //
    // Returns "" when no record is held. A caller MUST treat that as "offer no
    // share" rather than as a string to hand over — see StoaListScreen, where
    // the share affordance's absence is the honest rendering.
    function shareText(stoa, genesis) {
        if (!stoa || !genesis)
            return ""
        return JSON.stringify({
            stoa: root.stripPrefix(stoa),
            genesis: root.stripPrefix(genesis)
        })
    }

    // The inverse. Returns one of exactly two shapes, never a third:
    //   { ok: true,  stoa: "<hex>", genesis: "<hex>" }
    //   { ok: false, reason: "<what is wrong with what was pasted>" }
    //
    // **This checks only that the input CARRIES two halves, and never whether
    // they match.** Verification is a hash comparison and it belongs in the
    // core — a view that re-derived an address would be a second implementation
    // of the one check this entire design rests on, and two implementations of
    // a security check is one more than can be kept correct.
    function parse(text) {
        var raw = String(text === undefined || text === null ? "" : text).trim()
        if (raw === "")
            return { ok: false, reason: "Nothing was pasted." }

        var parsed
        try {
            parsed = JSON.parse(raw)
        } catch (e) {
            return {
                ok: false,
                reason: "What was pasted is not a Stoa reference. A Stoa reference "
                      + "carries an address AND the founding record it names; an "
                      + "address on its own cannot be joined, because the address "
                      + "is a hash of the record and cannot rebuild it."
            }
        }

        if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed))
            return { ok: false, reason: "What was pasted is not a Stoa reference." }

        // `typeof` on each half, not truthiness. A number, an object or an array
        // in either field is attacker-supplied input that would otherwise reach
        // `JSON.stringify` and be sent to the core as a shape it did not expect.
        var stoa = typeof parsed.stoa === "string" ? root.stripPrefix(parsed.stoa) : ""
        var genesis = typeof parsed.genesis === "string" ? root.stripPrefix(parsed.genesis) : ""

        if (stoa === "" && genesis === "")
            return { ok: false, reason: "What was pasted carries neither an address nor a founding record." }
        if (genesis === "")
            return {
                ok: false,
                reason: "What was pasted carries an address but no founding record. "
                      + "An address alone cannot be joined: it is a hash of the "
                      + "record, so it verifies one somebody hands over and cannot "
                      + "rebuild it. Ask whoever sent this for the whole reference."
            }
        if (stoa === "")
            return { ok: false, reason: "What was pasted carries a founding record but no address." }

        return { ok: true, stoa: stoa, genesis: genesis }
    }
}
