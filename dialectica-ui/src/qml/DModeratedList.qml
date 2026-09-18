import QtQuick
import QtQuick.Layouts

// One of screen 07's two moderated lists: a label, a bounded block of rows, and
// a hairline under every row but the last.
//
// **Extracted because there are two of them and they differ only in what a row
// contains.** Moderated authors and moderated posts share the heading
// treatment, the 150px bound, the clipping and the separator rule; only the row
// body differs. Written out twice, a change to the separator logic would have to
// be made twice and would be wrong in one of them — which is how the two lists
// come to disagree about something neither is about.
//
// ---- A Repeater, NOT a ListView, AND THAT IS A CORRECTNESS FIX -------------
//
// A `ListView` instantiates only the delegates it currently needs, and inside a
// layout that has not yet been given its height it needs one. **Measured on
// Qt 6.10.3**: with a two-row fixture and `Layout.preferredHeight: 150`, exactly
// ONE row existed — `tst_moderation_screen.qml`'s
// `test_the_two_lists_are_separate_with_their_own_controls` failed with actual 1,
// expected 2.
//
// That is not only a test problem. It is this repo's silent-failure house style:
// a row present in the model and absent from the screen, with nothing logged. A
// `Repeater` instantiates every row, which is right for a list this short.
//
// **The trade, stated because it is real:** a `Repeater` has no virtualisation,
// so a long list builds every row. These are fixtures of two. When a core
// listing arrives and the list can be long, this becomes a `ListView` again —
// and whoever makes that change must give it a height that exists before the
// delegates are asked for, or re-create the defect above.
//
// **The separator drops on the LAST row**, unlike screen 08's Stoa list which
// rules under its last row too. The reference draws the moderation lists that
// way: a bounded, possibly-clipped column ending in a rule reads as though it
// had been cut off mid-list, where the Stoa list is the whole of what there is.
ColumnLayout {
    id: list

    property string heading: ""
    property var rows: []

    // The row body, as a Component. A delegate rather than a fixed shape
    // because the row body is the only thing the two lists do not share, and
    // pushing a `kind: "author" | "post"` branch in here would put both row
    // layouts in one file under a conditional — one component, two jobs, and a
    // branch to get right at each of the six elements that differ.
    //
    // **The row's data reaches the delegate through a `rowData` property the
    // delegate declares on its own root**, not by reaching up through `parent`.
    // The first version put it on the Loader, which made the delegate's own
    // children write `parent.parent.rowData` — a chain whose correct length
    // depends on how deeply that child happens to be nested, so moving an
    // element one level in silently reads the wrong object with nothing
    // failing. A property on the delegate's root is resolved by QML's scope
    // chain and is the same expression at every depth inside it.
    property Component rowDelegate: null

    spacing: 8

    Text {
        text: list.heading
        font: DTheme.label
        color: DTheme.inkMuted
        textFormat: Text.PlainText
    }

    // A fixed height rather than a fill: `ScreenFrame`'s contract says a card
    // sized from its content has no slack, so `Layout.fillHeight` would measure
    // ZERO here and the whole list would vanish with every gate green. 150 is
    // the reference's `max-height`.
    Item {
        objectName: "listBody"
        Layout.fillWidth: true
        Layout.preferredHeight: 150
        clip: true

        ColumnLayout {
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            spacing: 0

            Repeater {
                model: list.rows

                delegate: ColumnLayout {
                    id: block
                    required property var modelData
                    required property int index
                    Layout.fillWidth: true
                    spacing: 0

                    Loader {
                        Layout.fillWidth: true
                        Layout.topMargin: DTheme.itemGap
                        Layout.bottomMargin: DTheme.itemGap
                        sourceComponent: list.rowDelegate
                        // Assigns the delegate's own `rowData` property after
                        // it is instantiated.
                        //
                        // **`rowData` is NOT `required`, and cannot be**, which
                        // is worth stating because the opposite is the obvious
                        // guess and this comment used to make it. A `Loader`
                        // builds its component first and emits `onLoaded`
                        // second, so there is no point at which a required
                        // property could be supplied: marking it `required`
                        // makes every row fail to instantiate. Measured on
                        // Qt 6.10.3 — `Required property rowData was not
                        // initialized` per row, and
                        // `test_the_two_lists_are_separate_with_their_own_controls`
                        // saw 0 rows where it expects 2.
                        //
                        // So each delegate declares `rowData` with a default of
                        // the shape it reads, and an unbound delegate renders
                        // blank rather than erroring. That is the trade this
                        // pattern makes, not an oversight: `Loader` buys the
                        // two lists a shared body, and gives up the
                        // construction-time enforcement `required property var
                        // modelData` has two lines above.
                        onLoaded: item.rowData = block.modelData
                    }

                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: DTheme.hairline
                        color: DTheme.rule
                        visible: block.index < list.rows.length - 1
                    }
                }
            }
        }
    }
}
