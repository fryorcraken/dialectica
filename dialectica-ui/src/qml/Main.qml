import QtQuick
import QtQuick.Layouts

// Dialectica — the view.
//
// Basecamp sandboxes this engine with a deny-all network access manager and no
// filesystem access outside the plugin directory (PLAN.md §2.1), so the view
// cannot fetch or read anything itself. Everything below goes through the core
// module, via the `Core` singleton, which is the only place a core call is
// written.
//
// This is Stage A of PLAN.md §9.1: onboarding, and a feed over one Stoa with
// the empty and unreadable states that must never look alike. There is no Stoa
// list, no thread view and no composer — each is a later change, and each needs
// core methods that do not exist yet.
Item {
    id: root

    // Which Stoa this view reads.
    //
    // **Nothing in core answers this yet**, and that is honest rather than
    // unfinished: §9.1 Stage D is where `joinStoa` and a list of held Stoas
    // arrive, and until then there is no source for an address but the one a
    // developer supplies. So it is empty by default and the feed says plainly
    // that it was given nothing, rather than inventing a Stoa to show.
    property string stoaAddress: ""
    property string stoaTitle: ""

    // The Stoa's genesis record, hex. See FeedScreen — it travels with the
    // request because nothing records joined Stoas yet, and it is safe to pass
    // because core verifies it hashes to the address.
    property string stoaGenesis: ""

    // ---- the launch branch ------------------------------------------------
    //
    // Which screen to show is **the module's answer**, never a flag this view
    // remembered. A remembered "this user has onboarded" outlives the thing it
    // remembers: a keystore that was deleted, moved or is unreadable leaves the
    // flag set and the user looking at a feed they cannot post in, with no path
    // back to the screen that would fix it. The module is the only party that
    // can see the store, so it is the only party that can answer.
    //
    //   "unknown"  not asked yet — reachable only before onCompleted has run
    //   "present"  there is an identity: the feed
    //   "absent"   there is none: onboarding
    //   "failed"   the call itself failed: neither branch, because which one is
    //              right is exactly what is not known
    property string identityState: "unknown"

    // The module's reason when there is no identity, held UNPARSED.
    //
    // Both absent cases reach onboarding — no identity stored, and an identity
    // that exists but could not be loaded — because in each the user has no
    // usable identity and the screen that offers one is where they must arrive.
    // Core distinguishes the two by this string alone; keeping it verbatim is
    // what leaves them distinguishable to a later screen.
    //
    // Deliberately NOT classified here. Any `indexOf(...)` over core's wording
    // would be a second copy of core's error taxonomy, maintained in the wrong
    // module, and would silently reclassify the day core rewords a message.
    property string identityReason: ""

    // What the module said about recovery, three-valued: `undefined` means it
    // did not say, and the screen then makes no claim either way.
    property var recoveryNeedsTheRecord: undefined

    // The failure text when the report itself failed.
    property string identityFailure: ""

    Component.onCompleted: root.askWhoAmI()

    // Asked on launch, and asked AGAIN after a keep reports success. The second
    // ask costs a round trip and buys the property that no screen state is ever
    // derived from an action having been invoked: even a transition this view
    // just watched happen is decided by the module, which is the only party
    // that knows whether anything was written.
    function askWhoAmI() {
        if (root.stoaAddress === "") {
            root.identityState = "failed"
            root.identityFailure = "No Stoa address was given to this view."
            return
        }

        var reply = Core.whoAmI(root.stoaAddress)

        if (!reply.ok) {
            // Neither branch. Showing a feed would claim an identity that was
            // never reported; showing onboarding would offer to replace one
            // that may well exist.
            root.identityState = "failed"
            root.identityFailure = reply.error
            return
        }

        if (reply.value.hasIdentity === true) {
            root.identityReason = ""
            root.recoveryNeedsTheRecord = reply.value.recoveryNeedsTheRecord
            root.identityFailure = ""
            root.identityState = "present"
            return
        }

        // `hasIdentity` absent, false, or any value that is not `true` — all of
        // them are "the module did not report an identity", and none of them
        // may open the feed. Failing towards onboarding is safe in a way the
        // reverse is not: onboarding's own keep is refused by core where an
        // identity already exists, so the worst case is a refusal the user can
        // read, rather than a forum they cannot post in.
        // A non-string reason is held as absent rather than stringified.
        // `String({...})` yields `[object Object]`, and this value exists to
        // keep the two absent cases distinguishable to a later screen — a
        // placeholder that is the same text for every unreadable reason
        // distinguishes nothing, while an empty string is honestly "the module
        // gave no reason this view could read".
        root.identityReason = typeof reply.value.reason === "string"
            ? reply.value.reason
            : ""
        root.recoveryNeedsTheRecord = reply.value.recoveryNeedsTheRecord
        root.identityFailure = ""
        root.identityState = "absent"
    }

    Rectangle {
        anchors.fill: parent
        color: Theme.desk
    }

    Flickable {
        anchors.fill: parent
        contentWidth: width
        contentHeight: content.implicitHeight + 2 * Theme.cardPaddingY
        clip: true

        ColumnLayout {
            id: content
            width: parent.width
            spacing: 0

            Item { Layout.preferredHeight: Theme.cardPaddingY }

            OnboardingScreen {
                id: onboarding
                visible: root.identityState === "absent"
                stoaAddress: root.stoaAddress
                recoveryNeedsTheRecord: root.recoveryNeedsTheRecord
                // A kept identity is not this screen's word for it. The module
                // is asked again, and the answer is what moves the branch.
                onIdentityKept: root.askWhoAmI()
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(Theme.cardWidth, root.width - 2 * Theme.cardPaddingX)
            }

            FeedScreen {
                id: frame
                visible: root.identityState === "present"
                stoaAddress: root.stoaAddress
                stoaTitle: root.stoaTitle
                stoaGenesis: root.stoaGenesis
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(Theme.cardWidth, root.width - 2 * Theme.cardPaddingX)
            }

            // The report itself failed. Neither branch is shown, because the
            // view does not know which one is right — and the module's own
            // message is what is shown, rather than a reworded one.
            ScreenFrame {
                visible: root.identityState === "failed"
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(Theme.cardWidth, root.width - 2 * Theme.cardPaddingX)

                Text {
                    text: "Whether you have an identity here could not be determined."
                    font: Theme.heading
                    color: Theme.accent
                    wrapMode: Text.WordWrap
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                }

                Text {
                    text: "Neither the forum nor a new identity is offered, because which of the two is right is exactly what is not known."
                    font: Theme.bodySmall
                    color: Theme.inkSoft
                    wrapMode: Text.WordWrap
                    lineHeight: 1.55
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                }

                Text {
                    text: root.identityFailure
                    font: Theme.address
                    color: Theme.ink
                    wrapMode: Text.WrapAnywhere
                    textFormat: Text.PlainText
                    Layout.fillWidth: true
                }

                FlatButton {
                    text: "Ask again"
                    kind: "primary"
                    onClicked: root.askWhoAmI()
                }
            }

            Item { Layout.preferredHeight: Theme.cardPaddingY }
        }
    }
}
