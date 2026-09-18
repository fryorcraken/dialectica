import QtQuick

// The defective half of the probe's demonstration pair: the sibling repo's real
// bug, reproduced here rather than cited from there.
//
// `radicle-ui`'s CommitView.qml declared a property named `data`, which is
// Item's DEFAULT property. Declaring a property of that name shadows it, so
// every declared child below is assigned to this new `var` property instead of
// becoming a scene-graph child. `children.length` reports 0 and nothing paints
// — the whole view rendered as a blank rectangle, with no warning, no undefined
// binding and no name collision for the existing gates to catch.
//
// THIS FILE AND RenderProbeHealthy.qml MUST DIFFER BY EXACTLY ONE LINE — the
// `property var data` below. See the comment in that file for why, and note
// that `tst_render_probe.qml` asserts the difference mechanically.
Item {
    property var data

    Rectangle {
        anchors.fill: parent
        color: "#1e3a5f"
    }
    Rectangle {
        x: 10
        y: 10
        width: 30
        height: 30
        color: "#f0c674"
    }
}
