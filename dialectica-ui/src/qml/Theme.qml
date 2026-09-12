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

    // ---- mark inks: SIX, and they are NOT the interface palette ----------
    // The identicon's palette has a different job from the interface's, so it
    // is scoped separately. See docs/IDENTICON.md for the derivation.
    //
    // THESE ARE FROZEN WIRE-VISIBLE CONSTANTS, NOT THEME TOKENS. Every peer
    // must render the same address identically, so editing a value here changes
    // what every identity looks like and makes two peers on different app
    // versions disagree about the same person. That is the precise failure the
    // mark's determinism contract exists to prevent. Add a new ink if the mark
    // ever needs one; do not retune an existing one.
    //
    // Hand-picked, not sliced off a hue wheel — adjacent steps on a wheel land
    // inside a just-noticeable difference and manufacture parameter states
    // nobody can tell apart.
    //
    // The ordering is a LIGHTNESS LADDER (L 0.219 -> 0.811), and that shape is
    // forced rather than aesthetic. Under deuteranopia and protanopia the
    // red/green axis collapses, so two inks separated only by hue become one
    // colour; what dichromats retain is lightness and the blue/yellow axis.
    // Spacing the ladder is therefore what makes the palette work for the ~8%
    // of men with a colour vision deficiency, and it is why the set is six
    // rather than eight: a usable lightness range on this paper does not hold
    // eight rungs at the floor below.
    //
    // LIGHTNESS DOES THE SEPARATING, CHROMA DOES THE DAMAGE. These are
    // different axes, and holding L while cutting C is what lets the palette be
    // both accessible and tonally right. An earlier version put a C 0.169 rung
    // at L 0.72 and it read as a highlighter against this paper; desaturating it
    // to C 0.049 at the same lightness cost nothing measurable, because after
    // dichromat collapse the surviving separation is almost entirely lightness.
    // Do not "fix" a tonal complaint by darkening a rung — that pulls pairs back
    // under the floor and re-creates the defect the ladder exists to prevent.
    //
    // Measured on THREE constraints, not one. An ink-to-ink matrix cannot see
    // fluorescence: the C 0.169 rung was far from every other ink and still
    // wrong on the page.
    //
    //   pair separation  >= 0.10   worst 0.151 normal, 0.108 simulated
    //   contrast v paper >= 0.20   worst 0.205 (markSky)
    //   chroma at high L <  0.09   worst 0.094 (markRust, at L 0.40)
    //
    // The three together are a tight squeeze and that is worth knowing before
    // retuning: paper contrast caps ink lightness near 0.74, so the usable band
    // is about 0.17-0.74, and six inks need five gaps of ~0.10 inside it. There
    // is very little slack. Adding a seventh ink is not a free change.
    //
    // The instrument is tmp/render/tst_palette.qml, which self-tests against
    // #808080 -> L 0.5998 before reporting, because an earlier hand-computed
    // version of this table shipped a pair at 0.006 while claiming 0.100.
    readonly property color markInk:     "#100f0c"   // L 0.17
    readonly property color markIndigo:  "#2b3062"   // L 0.33
    readonly property color markRust:    "#71321f"   // L 0.40
    readonly property color markGreen:   "#42744f"   // L 0.51
    readonly property color markLavender: "#7c80a0"  // L 0.61
    readonly property color markSage:    "#aeab84"   // L 0.73

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
