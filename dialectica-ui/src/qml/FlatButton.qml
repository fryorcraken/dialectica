import QtQuick

// No radius, no gradient, no shadow.
// destructive is reserved for publishing something irreversible.
//
// EACH KIND IS ONE ENTRY IN A TABLE, not a row of parallel ternary chains.
// Colour, border, text ink and padding were three separate chains keyed on the
// same string, so a kind was described in three places that had to agree and a
// new one meant editing each. The table makes a kind one object, so a missing
// field is visible where the kind is defined rather than surfacing as a chain
// that silently falls through to `secondary`.
//
// A MISSING FIELD IS NOT SELF-ANNOUNCING, which this comment used to imply and
// which is only half true — measured, and the half that fails is the dangerous
// one. A kind missing a COLOUR field raises `Unable to assign [undefined] to
// QColor`, which `run-qml-tests.sh`'s `check_bindings` turns into a failure. A
// kind missing `padX` or `padY` raises NOTHING: `implicitHeight` becomes `NaN`
// in silence, and a NaN-height button in a RowLayout is a control nobody can
// see that still accepts clicks — the exact failure the `secondary` fallback
// below was chosen to prevent. `NaN` is a valid `real`, so QML type-checks
// nothing; only a QColor assignment is checked.
//
// So the guarantee is an assertion rather than a type.
// `tst_flat_button.qml`'s `test_every_kind_declares_every_field` and
// `test_no_kind_renders_with_a_nan_dimension` sweep every key of this table,
// derived with `Object.keys` rather than restated, so a sixth entry is covered
// the moment it is added.
Rectangle {
    id: root

    property string text: ""
    property string kind: "secondary"
    signal clicked()

    // `fill` "" means no fill — the Rectangle stays transparent and the border
    // carries the shape. `stroke` "" means no border.
    //
    // PUBLIC ON PURPOSE: `tst_flat_button.qml` derives its sweep list from
    // `Object.keys(kinds)` rather than restating the five names, so a sixth
    // kind is covered the moment it is added here. Do not make this file-scoped
    // without moving that derivation — four sweeps would silently become passes
    // over zero kinds.
    //
    // Two known wrinkles, deferred rather than unnoticed. `readonly` on a `var`
    // freezes the REFERENCE, not the object, so a consumer can mutate an entry
    // (measured). And the literal is re-created per instance rather than
    // shared, so every button allocates six objects it never writes to. Neither
    // costs anything today — nothing in the tree mutates `kinds`, and the
    // allocation matters only in a long list. The fix is a singleton or a
    // file-scoped QtObject that is still reachable for the sweep above, which
    // is worth doing when the moderated-author list with a per-row UNMODERATE
    // exists to feel it.
    readonly property var kinds: ({
        "primary":             { fill: DTheme.ink,    stroke: "",           textInk: DTheme.paper,  font: DTheme.body,  padX: 36, padY: 16 },
        "secondary":           { fill: "",            stroke: DTheme.ink,   textInk: DTheme.ink,    font: DTheme.body,  padX: 32, padY: 14 },
        "destructive":         { fill: DTheme.accent, stroke: "",           textInk: DTheme.paper,  font: DTheme.body,  padX: 36, padY: 16 },
        // Beside a filled `destructive`, on the moderation confirmation. Two
        // filled reds side by side read as one decision offered twice; the
        // outline says "also destructive, and the second of the two".
        "destructive-outline": { fill: "",            stroke: DTheme.accent, textInk: DTheme.accent, font: DTheme.body,  padX: 32, padY: 14 },
        // The per-row UNMODERATE in a moderated list. A full-size secondary in
        // a 19px list row out-weighs the row it acts on, so this is `secondary`
        // at label type with the padding pulled in.
        "secondary-micro":     { fill: "",            stroke: DTheme.ink,   textInk: DTheme.ink,    font: DTheme.label, padX: 16, padY: 8 }
    })

    // An unrecognised kind renders as `secondary`. THIS IS A BEHAVIOUR CHANGE
    // and the reason to make it is what the previous form did instead, which
    // was traced rather than assumed: with three chains keyed on `kind`,
    // `filled` was `kind !== "secondary"`, so an unknown kind was `filled` —
    // no border (`filled ? 0 : hairline`), no fill (neither the primary nor
    // the destructive branch matched, so `"transparent"`), and paper-coloured
    // text. A typo'd kind rendered as PAPER TEXT ON NOTHING: an invisible
    // button that still accepted clicks.
    //
    // A control nobody can see is the worst of the available failures. Falling
    // back to `secondary` makes a typo look wrong rather than look absent.
    //
    // `hasOwnProperty` AND NOT `kinds[kind] !== undefined`, which is what this
    // was and which had a hole the shape of `Object.prototype`. A bracket
    // lookup walks the prototype chain, so `kind: "constructor"` resolves to
    // `Object.prototype.constructor` — a Function, not `undefined` — the guard
    // passed, `spec` became that Function, and every field read off it was
    // `undefined`. Measured on Qt 6.10.3: all seven inherited member names
    // (`constructor`, `toString`, `valueOf`, `hasOwnProperty`, `__proto__`,
    // `isPrototypeOf`, `propertyIsEnumerable`) rendered `#ffffff` fill with
    // black ink at NaN size — the invisible-clickable-control class this
    // fallback exists to close, reached through a different door.
    //
    // Asking whether the table has the key OF ITS OWN is the question that was
    // always meant; `!== undefined` was a proxy for it that is wrong for seven
    // strings. Latent rather than live — every `kind:` in the tree is a literal
    // today — and it becomes live the moment one is bound from a model field or
    // from core, which is exactly when nobody is looking.
    readonly property var spec: kinds.hasOwnProperty(kind) ? kinds[kind]
                                                           : kinds["secondary"]

    color:        spec.fill !== "" ? spec.fill : "transparent"
    border.width: spec.stroke !== "" ? DTheme.hairline : 0
    border.color: spec.stroke !== "" ? spec.stroke : DTheme.ink

    implicitWidth:  label.implicitWidth + spec.padX
    implicitHeight: label.implicitHeight + spec.padY

    Text {
        id: label
        anchors.centerIn: parent
        text: root.text
        font: root.spec.font
        color: root.spec.textInk
        textFormat: Text.PlainText
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
