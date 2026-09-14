import QtQuick
import QtQuick.Layouts

// Three lamps, always the same three and always in this order:
// delivery (can records leave and arrive), storage (can the store be read),
// zone (is this machine in a Stoa). Each is ok / degraded / failed.
//
// They replace every prose status line, and the reason is that a lamp cannot
// overclaim: a sentence saying "connected" asserts something the software would
// have to know, where a lamp asserts one of three states and puts the sentence
// in a tooltip where it is read on demand.
//
// The lamp colours are the only green and orange THE INTERFACE uses — see
// DTheme. Spending them anywhere else spends the signal reserved for "is this
// machine working" on something that is not that.
//
// The qualifier is load-bearing rather than pedantic: `DTheme`'s mark palette
// carries `markGreen` and `markSage`, and a reader who checks an unqualified
// claim finds it contradicted in the file cited to support it. The marks are
// not the interface palette (DTheme says so where they are declared) — an
// identicon ink is picked by an address, never to signal a state.
RowLayout {
    id: root

    property string deliveryState: "ok"
    property string storageState:  "ok"
    property string zoneState:     "ok"
    property string deliveryText: ""
    property string storageText:  ""
    property string zoneText:     ""

    spacing: 8

    // THE ONE PLACE A STATE STRING IS INTERPRETED, and it does not trust its
    // input.
    //
    // A lamp's state is a free string that will reach this component from a
    // screen, and ultimately from core. The obvious form — `s === "failed" ? …
    // : s === "degraded" ? … : ok` — treats EVERYTHING else as ok: "", "OK",
    // "okay", a typo, a value from a core version this UI does not know.
    //
    // Reading an unrecognised value as GREEN is the wrong direction to fail. A
    // lamp exists to say whether this machine is working; a value the UI cannot
    // interpret means it does not know, and green is a claim it cannot back.
    // SPEC.md's Tone section forbids exactly that — "never claiming more than
    // the software delivers" — so an unrecognised state degrades.
    //
    // NO SPEC: the bundle enumerates three states and is silent on a fourth.
    // Marked in tst_status_bar.qml.
    //
    // A function rather than an inline ternary in each lamp: a test can drive
    // it directly, and the fourth caller inherits the rule instead of
    // re-deriving it.
    function normalisedState(s) {
        if (s === "ok" || s === "degraded" || s === "failed")
            return s;
        return "degraded";
    }

    function lampColor(s) {
        var n = normalisedState(s);
        if (n === "failed")   return DTheme.statusFailed;
        if (n === "degraded") return DTheme.statusDegraded;
        return DTheme.statusOk;
    }

    component Lamp: Rectangle {
        id: lamp

        property string label: ""

        // `lampState`, NOT `state`. `QQuickItem` already has a `state`
        // property — the name of the active entry in `states` — and
        // redeclaring it is legal QML that SILENTLY SHADOWS the built-in.
        //
        // Measured on Qt 6.10.3, because the bundle's spelling is `state` and
        // the shadowing is invisible: a probe declaring `property string state`
        // on a Rectangle alongside `states: State { name: "degraded";
        // PropertyChanges { target: …; marker: 99 } }`, then assigning
        // `state: "degraded"`, instantiated fine, held "degraded" in the
        // property — and left `marker` at 0. The state machine never saw the
        // assignment. No error, no warning, no binding loop.
        //
        // So the bundle's name works today and disables states/transitions on
        // every lamp for whoever reaches for them next. One word avoids it.
        property string lampState: "ok"
        property string explanation: ""

        implicitWidth:  row.implicitWidth + 18
        implicitHeight: row.implicitHeight + 6
        color: "transparent"
        border.width: DTheme.hairline
        border.color: DTheme.rule2

        RowLayout {
            id: row
            anchors.centerIn: parent
            spacing: 6

            // `implicitWidth`/`implicitHeight`, not `width`/`height`. The dot
            // is a RowLayout child, and setting width on a layout-managed item
            // is undefined behaviour — qmllint's `Quick.layout-positioning`
            // says so and `check_qml_members.sh`'s `-W 0` makes it a failure.
            // The bundle's original sets `width`/`height` here; this is the one
            // place this component deliberately departs from it.
            Rectangle {
                implicitWidth: 7
                implicitHeight: 7
                radius: 3.5
                color: root.lampColor(lamp.lampState)
            }

            Text {
                text: lamp.label
                font: DTheme.label
                color: DTheme.inkSoft
                textFormat: Text.PlainText
            }
        }

        MouseArea {
            id: lampHover
            anchors.fill: parent
            hoverEnabled: true

            // `DTip`, not an attached tooltip binding — the attached form's
            // content item is a `Text` at `textFormat: StyledText`, and
            // `explanation` is the one free string this component takes. See
            // DTip.qml for the measurement.
            DTip {
                text: lamp.explanation
                visible: lampHover.containsMouse && lamp.explanation !== ""
            }
        }
    }

    // The order is fixed and is part of what the footer means: a reader learns
    // the position, so a lamp is identified by where it is before its label is
    // read. Do not make this a model — three lamps that could be reordered or
    // omitted would defeat that.
    Lamp { label: "DELIVERY"; lampState: root.deliveryState; explanation: root.deliveryText }
    Lamp { label: "STORAGE";  lampState: root.storageState;  explanation: root.storageText }
    Lamp { label: "ZONE";     lampState: root.zoneState;     explanation: root.zoneText }
}
