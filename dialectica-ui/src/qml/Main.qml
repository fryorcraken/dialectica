import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

// Dialectica — Phase 0 view.
//
// QML-only, with NO C++ backend, and that is the deliberate Phase 0 shape: it
// keeps the view provably thin. Basecamp sandboxes this engine with a deny-all
// network access manager and no filesystem access outside the plugin directory
// (PLAN.md §2.1), so a view cannot fetch or read anything itself even if
// someone later wanted it to. Everything below goes through the core module.
//
// The whole job here is to prove the path end to end: the view calls the Rust
// core, the core answers, and the answer renders. It is not the forum.
Item {
    id: root

    // Every core reply is JSON with exactly one failure shape, `{"error":...}`
    // (PLAN.md §2.5). So the view has ONE error branch, not one per call —
    // which is the property that convention buys and the reason to keep it.
    property string lastResult: ""
    property string lastError: ""

    readonly property color bg: "#12141a"
    readonly property color fg: "#e9ecf4"
    readonly property color muted: "#8b94a8"
    readonly property color bad: "#f0883e"

    Rectangle {
        anchors.fill: parent
        color: root.bg
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: 18

        RowLayout {
            spacing: 12

            Text {
                text: "Δ"
                color: root.fg
                font.pixelSize: 34
            }

            Text {
                text: "Dialectica"
                color: root.fg
                font.pixelSize: 26
            }
        }

        Text {
            text: "Phase 0 — proving the Rust module path."
            color: root.muted
            font.pixelSize: 13
        }

        RowLayout {
            spacing: 12
            Layout.fillWidth: true

            Button {
                text: "version"
                // No argument: the core method takes none.
                onClicked: root.call("version", [])
            }

            Button {
                text: "ping"
                // The payload proves the call carried data rather than merely
                // completing — a round trip that returns a constant would look
                // identical to one that never reached the core.
                onClicked: root.call("ping", [JSON.stringify({ payload: { from: "dialectica-ui" } })])
            }

            Button {
                text: "panic probe"
                // Phase 0 apparatus (PLAN.md §9): this makes the core panic on
                // purpose. A guarded module answers with the error shape and
                // KEEPS ANSWERING; an unguarded one poisons its instance mutex
                // and every later call dies too. Press it twice — the second
                // press is the half that tells you which.
                onClicked: root.call("panic_probe", [JSON.stringify({})])
            }

            Button {
                text: "delivery channelExists"
                // The Phase 0 bridge (PLAN.md §3.2). Expect an error unless a
                // delivery node is running — the point is WHICH error: a
                // delivery-side message means the bridge carried the call.
                onClicked: root.call("delivery_channel_exists",
                                     [JSON.stringify({ channelId: "dialectica-phase0" })])
            }
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 90
            color: "#1a1d27"
            radius: 8

            Text {
                anchors.fill: parent
                anchors.margins: 12
                text: root.lastError !== "" ? ("error: " + root.lastError)
                                            : (root.lastResult !== "" ? root.lastResult
                                                                      : "Press a button to call the core module.")
                color: root.lastError !== "" ? root.bad : root.fg
                font.pixelSize: 13
                wrapMode: Text.Wrap
                verticalAlignment: Text.AlignVCenter
            }
        }

        Item { Layout.fillHeight: true }
    }

    // One call site for every button, so the error handling exists once.
    function call(method, args) {
        root.lastResult = ""
        root.lastError = ""

        // The bridge is injected by the host. Absent means the view is running
        // somewhere that provides no core — worth saying plainly rather than
        // failing silently, which looks identical to a call that returned
        // nothing.
        if (typeof logos === "undefined" || !logos.callModule) {
            root.lastError = "logos bridge not available"
            return
        }

        var raw = String(logos.callModule("dialectica", method, args))

        // The core always answers JSON. Anything else means the call did not
        // reach it, so say so instead of rendering the raw bytes as if they
        // were a result.
        var reply
        try {
            reply = JSON.parse(raw)
        } catch (e) {
            root.lastError = "core returned non-JSON: " + raw
            return
        }

        if (reply.error !== undefined) {
            root.lastError = reply.error
            return
        }

        root.lastResult = JSON.stringify(reply)
    }
}
