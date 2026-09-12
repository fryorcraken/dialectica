pragma Singleton
import QtQuick

QtObject {
    // ---- surfaces -------------------------------------------------------
    readonly property color desk:      "#d9d2c2"   // behind the cards
    readonly property color paper:     "#efe9dc"   // card
    readonly property color paperDeep: "#e7dfcd"   // apparatus column
    readonly property color field:     "#f7f3ea"   // inset panels, inputs

    // ---- interface inks: three, and only three --------------------------
    // Three for the INTERFACE. The mark has its own eight-ink palette below,
    // and the two are deliberately separate scopes.
    readonly property color ink:       "#26231d"
    readonly property color inkSoft:   "#3a362e"
    readonly property color inkMuted:  "#6f685a"
    readonly property color inkFaint:  "#8c8577"
    readonly property color accent:    "#a33a2b"   // red: caveats, destructive, apparatus rules
    readonly property color accent2:   "#4a6b74"   // teal: second fill ink, secondary marks

    // ---- mark inks: eight, and they are NOT the interface palette --------
    // The identicon's palette has a different job from the interface's, so it
    // is scoped separately: iterating `accent` must not silently change what
    // every identity looks like. See docs/IDENTICON.md for the derivation.
    //
    // Hand-picked, not sliced off a hue wheel — adjacent steps on a wheel land
    // inside a just-noticeable difference and manufacture parameter states
    // nobody can tell apart. Separated in LIGHTNESS and CHROMA as well as hue,
    // which is how a categorical palette exceeds what hue rotation alone can
    // distinguish.
    //
    // Minimum pairwise OKLab distance 0.080 (markMoss/markTeal). Every pair was
    // re-checked under simulated deuteranopia and protanopia, where the
    // red/green axis collapses: markMoss is DARK and markOchre is LIGHT
    // precisely so those pairs separate on lightness, the channel dichromats
    // retain, rather than on hue. Two further candidates were cut for measuring
    // under the floor — a clay at 0.075 against markRust and an olive at 0.062
    // against markMoss — which is why there are eight and not ten.
    //
    // markInk and markRust duplicate `ink` and `accent` by value today. That is
    // deliberate duplication, not an oversight: the two palettes are free to
    // diverge.
    readonly property color markInk:    "#26231d"
    readonly property color markIndigo: "#37407e"
    readonly property color markMoss:   "#2f5233"
    readonly property color markPlum:   "#8a4479"
    readonly property color markRust:   "#a33a2b"
    readonly property color markTeal:   "#1f7a7a"
    readonly property color markStone:  "#8f8d84"
    readonly property color markOchre:  "#c98a2e"

    // ---- rules ----------------------------------------------------------
    readonly property color rule:      "#d8d0bf"
    readonly property color rule2:     "#c3bbaa"
    readonly property int   hairline:  1
    readonly property int   border:    2

    // ---- type -----------------------------------------------------------
    readonly property string serif: "Spectral"
    readonly property string mono:  "IBM Plex Mono"

    readonly property font display:    Qt.font({ family: serif, pixelSize: 27 })
    readonly property font heading:    Qt.font({ family: serif, pixelSize: 25 })
    readonly property font postBody:   Qt.font({ family: serif, pixelSize: 20 })
    readonly property font body:       Qt.font({ family: serif, pixelSize: 15 })
    readonly property font bodySmall:  Qt.font({ family: serif, pixelSize: 14 })
    readonly property font note:       Qt.font({ family: serif, pixelSize: 14, italic: true })
    readonly property font address:    Qt.font({ family: mono,  pixelSize: 11, letterSpacing: 0.2 })
    readonly property font addressBig: Qt.font({ family: mono,  pixelSize: 15, letterSpacing: 0.6 })
    readonly property font label:      Qt.font({ family: mono,  pixelSize: 9,  letterSpacing: 1.6,
                                                 capitalization: Font.AllUppercase })

    readonly property real lineHeightBody: 1.6

    // ---- metrics --------------------------------------------------------
    readonly property int cardWidth:      1000
    readonly property int apparatusWidth: 244
    readonly property int cardPaddingX:   34
    readonly property int cardPaddingY:   28
    readonly property int blockGap:       20
    readonly property int itemGap:        10
    readonly property int markInFeed:     19
    readonly property int markInList:     40
    readonly property int markMinWeave:   12   // below this: flat fill, contour only
    readonly property int markMinDraw:    10   // below this: no mark, print the address

    // ---- address abbreviation -------------------------------------------
    readonly property int headChars:   8
    readonly property int middleChars: 8
    readonly property int tailChars:   6
}
