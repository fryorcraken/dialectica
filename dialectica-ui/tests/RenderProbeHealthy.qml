import QtQuick

// The healthy half of the probe's demonstration pair.
//
// THIS FILE AND RenderProbeShadowed.qml MUST DIFFER BY EXACTLY ONE LINE — the
// `property var data` declaration in the other. That is not tidiness: the spec
// requires the two subjects to differ ONLY in whether their declared children
// reach the scene graph, because any second difference leaves two explanations
// for the difference in verdict and establishes neither.
//
// `tst_render_probe.qml` asserts the one-line difference mechanically, so a
// well-meaning edit to one file that is not mirrored here fails the suite
// rather than quietly weakening the demonstration.
//
// Plain Rectangles rather than a real screen, deliberately: the pair tests the
// PROBE, not the view. A screen carrying the shadowing property would be a
// modified copy of production code, which is itself a second difference.
Item {
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
