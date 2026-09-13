import QtQuick

// The view's one route to the clipboard.
//
// `AddressLabel.copyRequested()` has existed since the feed screen and nothing
// connected it — there was no clipboard anywhere in the view. This is that
// receiver, and it is a component rather than a call written at each site
// because WHAT gets copied differs from what is displayed: a row shows the 8-8-6
// abbreviation and copies the full share string, and that divergence is the kind
// of decision that must be made in one place or it is made three different ways.
//
// **A hidden TextEdit, not `Qt.labs.platform`'s Clipboard.** Basecamp sandboxes
// this engine (PLAN.md §2.1) and the view has no C++ of its own, so the only
// clipboard available is the one QtQuick itself ships. `TextEdit` is in QtQuick
// proper — already imported everywhere here — where `Qt.labs.platform` is a
// separate QML module that CI does not install and the flake does not pull in.
//
// It is off-screen and cannot be focused: `visible: false` keeps it out of the
// layout and out of the tab order, and zero size means it contributes nothing to
// an implicit height even if a layout ever did see it.
Item {
    id: root

    // What `copy()` was last asked to put on the clipboard.
    //
    // **This exists to be asserted on, and that is worth saying plainly.** Under
    // `QT_QPA_PLATFORM=offscreen` — which the test runner sets — there is no
    // system clipboard, so `TextEdit.copy()` writes nowhere a test can read. A
    // test can therefore check what this component was ASKED to copy and cannot
    // check what landed on the clipboard. Recording the argument is the half
    // that is verifiable; the other half is named in tst_stoa_screens.qml rather
    // than asserted around.
    property string lastCopied: ""

    visible: false
    width: 0
    height: 0

    function copy(text) {
        root.lastCopied = text
        sink.text = text
        sink.selectAll()
        sink.copy()
        sink.deselect()
    }

    TextEdit {
        id: sink
        visible: false
        width: 0
        height: 0
        // Never rich text. This holds a share string today, but the element is
        // one binding away from holding peer-supplied text, and QML's AutoText
        // default SNIFFS its input — so the format is pinned rather than left to
        // what the current content happens to look like. CI greps for this.
        textFormat: TextEdit.PlainText
    }
}
