.pragma library

// Container-relative geometry measurement for QML layout specs.
//
// ## Why these are functions and not assertions on properties
//
// Dialectica's first basecamp launch rendered as text drawn on top of text, and
// the whole property-based suite was green throughout. It could not have been
// otherwise: every one of those tests asks what a property HOLDS, and none asks
// where anything is DRAWN. Radicle hit the same class first and recorded the
// lesson these helpers implement:
//
//   > Measure against the container's bounds, not against `item.width` — an
//   > overflowing layout child keeps its full width and its `visible` stays
//   > true, so every property-based check passes while the user sees nothing.
//
// So the only number that says what a user can actually see is the INTERSECTION
// with the container. An item pushed outside its parent keeps a sensible
// `width`, a plausible `x` and `visible === true`; its intersection is what goes
// to zero.
//
// ## Why a shared library rather than a copy per spec
//
// These live in a `.pragma library` .js so every screen's layout spec imports
// the same implementation. The alternative — copying them into each new spec —
// is how a subtly weakened variant gets introduced: the copy that drops the
// strict-inequality tolerance in `verticallyOverlap`, or compares against
// `item.height` after all, still passes its own suite while no longer measuring
// the thing these exist to measure. One implementation, tested once, in
// tst_geometry.qml.

/// How much of `item` falls inside `container`, horizontally.
function visibleWidthIn(item, container) {
    if (!item || !item.visible)
        return 0
    var left = item.mapToItem(container, 0, 0).x
    var right = left + item.width
    return Math.max(0, Math.min(right, container.width) - Math.max(left, 0))
}

/// The same in the vertical axis.
///
/// This is the one that catches a collapsed container: children stacked inside a
/// zero-height parent have no vertical intersection with it, while every
/// horizontal assertion stays green.
function visibleHeightIn(item, container) {
    if (!item || !item.visible)
        return 0
    var top = item.mapToItem(container, 0, 0).y
    var bottom = top + item.height
    return Math.max(0, Math.min(bottom, container.height) - Math.max(top, 0))
}

/// Do two items overlap in the vertical axis, within `container`?
///
/// Text drawn on top of text is the user-visible symptom, and it is a rectangle
/// intersection rather than a property. Two panels that both report
/// `visible: true` at `y: 0` is exactly what the first launch photographed.
function verticallyOverlap(a, b, container) {
    if (!a || !b || !a.visible || !b.visible)
        return false
    if (a.height <= 0 || b.height <= 0)
        return false
    var aTop = a.mapToItem(container, 0, 0).y
    var bTop = b.mapToItem(container, 0, 0).y
    var aBottom = aTop + a.height
    var bBottom = bTop + b.height
    // A shared edge is abutment, not overlap, so the comparison is strict — with
    // a half-pixel tolerance, because a layout's own rounding routinely leaves
    // adjacent items sharing a fractional boundary. Without the tolerance every
    // stacked pair in a ColumnLayout reads as overlapping and the helper reports
    // a defect on a correct layout.
    return aTop < bBottom - 0.5 && bTop < aBottom - 0.5
}

/// Every descendant of `node` whose `objectName` is exactly `name`.
///
/// Returns ALL of them rather than the first, because "drawn exactly once" is
/// one of the things a layout spec asserts, and a finder that stops at the first
/// hit structurally cannot see a duplicate.
function findAllByName(node, name) {
    var found = []
    if (!node)
        return found
    if (node.objectName === name)
        found.push(node)
    for (var i = 0; i < node.children.length; i++)
        found = found.concat(findAllByName(node.children[i], name))
    return found
}

function findByName(node, name) {
    var all = findAllByName(node, name)
    return all.length > 0 ? all[0] : null
}
