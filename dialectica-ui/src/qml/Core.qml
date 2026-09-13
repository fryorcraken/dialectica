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
    // renders a broken store as an empty feed — which is the one confusion
    // UI-BRIEF obligation 5 exists to prevent.
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
    // its share affordance are one decision — see StoaReference.qml, which owns
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
}
