pragma Singleton
// QtQml, not QtQuick: the only type here is `QtObject`, which QtQml provides.
// Importing QtQuick pulled in the whole graphical stack for nothing and the
// linter said so ("Unused import"). This file has no visual element at all,
// which is the point of it.
import QtQml

// The one place the view talks to the core module.
//
// Basecamp sandboxes this engine with a deny-all network access manager and no
// filesystem access outside the plugin directory (PLAN.md §2.1), so there is no
// second route to data and this file is not one option among several — it is
// the whole of the view's access to the world.
//
// Every core reply is JSON with exactly one failure shape, `{"error":...}`
// (PLAN.md §2.5), so the error branch exists ONCE, here, rather than once per
// call site. That is the property the convention buys and the reason to keep
// every call going through `call()`.
QtObject {
    id: root

    readonly property string moduleName: "dialectica"

    // The host's bridge, as a property rather than a direct read of the global.
    //
    // **This exists so the call path can be tested, and that is the whole
    // reason.** `logos` is injected into the QML context by basecamp, so a test
    // harness has no way to provide one — a function reading the global
    // directly is a function whose four branches (valid reply, error reply,
    // malformed JSON, absent bridge) cannot be exercised anywhere.
    //
    // The default keeps production behaviour identical: `typeof` rather than a
    // bare reference because an undeclared identifier THROWS on evaluation, and
    // a property initialiser that throws leaves the singleton unusable — which
    // would turn "no bridge" from a diagnosable message into a dead view.
    property var bridge: (typeof logos !== "undefined") ? logos : null

    // A reply, normalised into exactly one of two shapes for the caller:
    //   { ok: true,  value: <parsed JSON> }
    //   { ok: false, error: "<message>" }
    //
    // Never both, and never a third shape. A caller that has to distinguish
    // "failed" from "succeeded with nothing" is the caller that eventually
    // renders a broken store as an empty feed — the one confusion this shape
    // exists to prevent, because the two look identical to a reader and mean
    // opposite things.
    //
    // **This separates a failure from an answer. It does not tell you the
    // operation succeeded** — for three methods the answer itself can be no.
    // See the note at the `ok: true` return below, which is the half of this
    // contract a caller is most likely to miss.
    function call(method, args) {
        // The bridge is injected by the host. Absent means the view is running
        // somewhere that provides no core, which is worth saying plainly: a
        // silent failure here looks exactly like a Stoa with nothing in it.
        if (!root.bridge || !root.bridge.callModule)
            return { ok: false, error: "The core module is not reachable from this view." }

        var raw
        try {
            raw = String(root.bridge.callModule(root.moduleName, method, args))
        } catch (e) {
            return { ok: false, error: "The call to the core module failed: " + e }
        }

        var reply
        try {
            reply = JSON.parse(raw)
        } catch (e) {
            // The core always answers JSON. Anything else means the call did
            // not reach it, so say so rather than rendering the raw bytes as
            // though they were a result.
            return { ok: false, error: "The core module returned something that is not JSON." }
        }

        if (reply === null || typeof reply !== "object")
            return { ok: false, error: "The core module returned something that is not a reply." }

        if (reply.error !== undefined)
            return { ok: false, error: String(reply.error) }

        // **`ok: true` means THE MODULE ANSWERED. It does not mean the thing
        // you asked for happened.**
        //
        // Read that before using `value`. Three core methods answer a refusal
        // as a wire SUCCESS, so `ok` is true and the answer is no:
        //
        //   keep_identity     {"kept":false,"reason":…}
        //   who_am_i          {"hasIdentity":false,"reason":…}
        //   get_capabilities  {"canPost":false,"reason":…}
        //
        // For those three, the negative field is the answer and `reason` says
        // why. A caller that stops at `ok` reports an identity that was never
        // stored, or opens a composer for a user who cannot post — and it does
        // so silently, because nothing failed.
        //
        // This is not a defect in the normalisation. `{"error":…}` is the wire's
        // one FAILURE shape and that is what `ok:false` reports; a refusal is a
        // different thing from a failure and the contract is right to keep them
        // apart. What the caller owes is the second branch: `ok` first, then the
        // method's own answer field.
        //
        // The warning lives HERE rather than only at the call sites that
        // already get it right, because this is the line a new wrapper's author
        // reads.
        return { ok: true, value: reply }
    }

    // ---- the methods this view actually uses ----------------------------
    //
    // Named wrappers rather than call sites spelling the method string, so that
    // a renamed core method is one edit and a typo is not a silent empty feed.

    // The genesis record travels with the request because nothing in core
    // records which Stoas this peer has joined yet — that is Stage D's
    // `joinStoa`, which does not exist. It is safe to pass it from here
    // because an address IS the hash of the record: core re-derives the address
    // and refuses a record that does not match, so a wrong or tampered record
    // is an error rather than a Stoa with a moderator of the caller's choosing.
    function listThreads(request) {
        return root.call("list_threads", [JSON.stringify(request)])
    }

    function getCapabilities(stoa) {
        return root.call("get_capabilities", [JSON.stringify({ stoa: stoa })])
    }

    // Create a Stoa. A title and nothing else, because there is no creator
    // argument and there cannot be: the creator key is fixed inside the address
    // preimage forever, so a call accepting one would mint a Stoa nobody can
    // moderate at an address nobody can withdraw. The key comes from this peer's
    // keystore, and creation fails — with the keystore's own reason — when there
    // is no usable one.
    function createStoa(title) {
        return root.call("create_stoa", [JSON.stringify({ title: title })])
    }

    // Join a Stoa somebody else created.
    //
    // BOTH halves, and that is a property of the address rather than an
    // awkwardness of this call: an address is a one-way hash of the record,
    // enough to verify a record handed over and not enough to reconstruct one.
    // A bare address is not joinable, which is why the view's paste field and
    // its share affordance are one decision — see DStoaReference.qml, which owns
    // both `parse` and `shareText` in one file so the two ends cannot drift.
    function joinStoa(stoa, genesis) {
        return root.call("join_stoa", [JSON.stringify({ stoa: stoa, genesis: genesis })])
    }

    // One page of the Stoas this peer is in.
    //
    // `perPage` is the caller's rather than defaulted here. This wrapper's job
    // is to name the method once and shape the request; how many rows a screen
    // shows is that screen's decision, and a default buried here is a number
    // two screens would silently share.
    function listStoas(page, perPage) {
        return root.call("list_stoas", [JSON.stringify({ page: page, perPage: perPage })])
    }

    // ---- publishing -----------------------------------------------------
    //
    // Three wrappers rather than three call sites spelling the method string,
    // for the same reason as the two above. Each returns `call()`'s two-shape
    // reply and interprets nothing: what counts as a success for a publish is
    // richer than "no error" — `wasNew` distinguishes a fresh op from a
    // deduplicated one — and that judgement belongs at the one place that
    // renders it, not repeated in three wrappers.
    //
    // **The body is passed through untouched.** An op is signed over its bytes,
    // so anything done to a draft here would publish, under the user's
    // signature, something the user did not write. `JSON.stringify` escapes for
    // transport and core parses that back to the same string; nothing else on
    // this path reads the body.

    function publishPost(stoa, body) {
        return root.call("publish_post", [JSON.stringify({ stoa: stoa, body: body })])
    }

    // No `thread` argument, deliberately: core derives the thread from the
    // parent and REFUSES a request that names one, which is what makes a reply
    // filed under the wrong thread unrepresentable rather than checked here.
    function publishReply(stoa, parent, body) {
        return root.call("publish_reply",
                         [JSON.stringify({ stoa: stoa, parent: parent, body: body })])
    }

    // `direction` is "up" or "down". It is NOT mapped from a number here —
    // core refuses an unrecognised direction naming what was supplied, and a
    // view translating -1/+1 into strings would be a second place the mapping
    // could be got wrong.
    function publishVote(stoa, target, direction) {
        return root.call("publish_vote",
                         [JSON.stringify({ stoa: stoa, target: target, direction: direction })])
    }

    // ---- onboarding ------------------------------------------------------
    //
    // **None of these three takes an identity**, and the omission is the
    // contract's rather than an oversight: the identity follows from the Stoa
    // and the selection, so a request naming one would be asking the module to
    // act as somebody it is not. Core refuses such a request; there is nothing
    // here that could send one.
    //
    // Two of the three have TWO success shapes — `{"kept":true,…}` /
    // `{"kept":false,"reason":…}` and the same for `hasIdentity`. `call()`
    // normalises both to `ok:true`, because both ARE successes at the wire
    // level: the module answered the question it was asked. Telling a refusal
    // from a success is the caller's job and is done once, in
    // DOnboardingScreen, where the three outcomes become three phases.

    // `{"stoa":hex}` -> `{"slate":hex,"count":N,"candidates":[…]}`.
    // No count parameter: a caller-supplied count is a number deciding how much
    // key derivation the module performs, so the module fixes it and reports it.
    function generateIdentitySlate(stoa) {
        return root.call("generate_identity_slate", [JSON.stringify({ stoa: stoa })])
    }

    // `{"stoa":hex,"slate":hex,"index":N}` -> kept, or refused with a reason.
    //
    // `slate` is the identifier the offering reply carried, and sending it is
    // what lets core refuse a selection made against a superseded set rather
    // than satisfying it with the current set's candidate at that index — which
    // would store an identity the user never saw.
    function keepIdentity(stoa, slate, index) {
        return root.call("keep_identity",
                         [JSON.stringify({ stoa: stoa, slate: slate, index: index })])
    }

    // `{"stoa":hex}` -> the identity in use, or that there is none with a reason.
    //
    // A DIFFERENT question from `getCapabilities`, and the two can honestly
    // disagree: a stored identity whose keystore permissions are too open is a
    // real identity that cannot currently be used. This is the one that can
    // tell an absent identity from an unusable one.
    //
    // **It is also the only reply carrying `recoveryNeedsTheRecord`** — the
    // keep reply does not have the field (see `Whoami::Identity` in
    // `wire.rs`). That matters because the view may not claim a saved master
    // key is a complete backup while the record of which candidate was kept
    // lives only on this machine, and this call is the sole route by which
    // that fact can reach a screen: the view has no filesystem access, so a
    // screen that did not ask would be guessing about a secret on the
    // user's disk.
    function whoAmI(stoa) {
        return root.call("who_am_i", [JSON.stringify({ stoa: stoa })])
    }
}
