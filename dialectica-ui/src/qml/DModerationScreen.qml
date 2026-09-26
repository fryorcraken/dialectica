import QtQuick
import QtQuick.Layouts

// Screen 07 — moderating. The confirmation, then what has already been
// moderated: authors on the left, single posts on the right, each row carrying
// its own UNMODERATE.
//
// ---- EVERY CONTROL ON THIS SCREEN IS INERT, AND THAT IS THE FIRST THING -----
//
// **No control here calls the core, and none can.** The `Dialectica` trait
// exposes `publish_post`, `publish_reply` and `publish_vote` and no
// moderation-publishing method at all, so there is nothing for these buttons to
// call. PLAN.md §9.2 ruling 3 records that; its screen half was reversed by the
// owner — the MVP ships this screen so it can be seen — and the publishing half
// was not.
//
// **An inert DESTRUCTIVE control is a hazard rather than a neutral
// placeholder**, which is why the screen carries a standing notice rather than
// relying on nothing visibly happening when a button is pressed. A user who
// presses "Mark as moderated" and is told nothing believes one of two things,
// and this software chooses which: that the post is now hidden for the Stoa's
// readers, or that the app is broken. The first is the dangerous one — a
// moderator who believes a post is hidden stops dealing with it — and on THIS
// screen a successful moderation and a control that does nothing look
// identical, because a real moderation would also change nothing here.
//
// That is the whole argument for the notice being rendered rather than being a
// comment, and `moderation-view`'s "The screen states that nothing it offers
// takes effect" is what stops it being deleted as clutter by someone who reads
// the buttons as merely unfinished.
//
// **The lists are FIXTURES.** Nothing enumerates what a Stoa has moderated:
// `moderation::resolve` answers whether one named target is hidden, which is a
// different question from "what is hidden here", and no listing exists on the
// trait. So these rows are held in the view, declared below and marked there,
// and the notice says they are examples rather than this peer's moderation
// state. PLAN.md §9.2's case 2 entries 5 and 6 carry both.
//
// ---- WHAT THIS SCREEN DELIBERATELY DOES NOT DO -----------------------------
//
// **It makes no core call of any kind** — not even a read. That is a contract
// and not an accident of what is bound today: `moderation-view`'s "No control
// on this screen makes a call, and no reply is rendered as one" exists so that
// the first person to wire one control finds a contract saying the others are
// inert on purpose, rather than a screen whose inertness is a property of its
// current bindings. There is no `Core.` anywhere in this file, and the
// reachability gate's singleton walk is one place that shows.
//
// **It renders no moderator badge and asserts no authority.** Whether this peer
// may moderate this Stoa is a question nothing answers — `getModerationCapability`
// is designed in PLAN.md §9.1 and does not exist — and a badge would be the one
// claim on this screen that a user could act on to their cost.
ScreenFrame {
    id: screen

    // The Stoa this screen is about, carried so the way out lands back on its
    // feed. Rendered nowhere: the screen's subject is a post, and a Stoa
    // address in the header would be decoration on a screen already dense with
    // addresses that mean something.
    property string stoaAddress: ""

    // The post the confirmation is about. Supplied by whoever routed here.
    property string subjectName: ""
    property string subjectAddress: ""
    property string subjectExcerpt: ""

    signal closed()

    // ---- the fixtures ---------------------------------------------------
    //
    // MOCK DATA, and marked as such at the site that declares it rather than in
    // a comment further up, because this is the line somebody copies.
    //
    // **These are not read from anything and must not become so quietly.** When
    // a core listing arrives, these properties are replaced by a read and the
    // notice below loses its second sentence; until then they are examples
    // chosen to exercise the layout — two authors, two posts, one long excerpt
    // that must elide.
    //
    // `readonly` so a caller cannot bind a core reply into them and leave the
    // notice claiming a fixture. That is the cheap half of "no reply is
    // rendered as one": the expensive half is that nothing here calls anything.
    readonly property var moderatedAuthors: [
        { name: "quiet obsidian rimrunner",
          address: "7e22d04390fa1c572b68b6e415d0873ac96f2b18e5407dc3916ba24f08ed5372",
          postCount: 9 },
        { name: "amber recursive cartographer",
          address: "05af913c6b2e84d17f0a35c9be41d27672c5db91a8e30f45c71b6d2803a63f4d8",
          postCount: 2 }
    ]

    readonly property var moderatedPosts: [
        { excerpt: "“Buy followers, cheap —”",
          authorAddress: "7e22d04390fa1c572b68b6e415d0873ac96f2b18e5407dc3916ba24f08ed5372",
          moderatorAddress: "8f2a41d97b05c3e19ac07f6d2841bb5390e7c4a21d68f03b57ce9a4412d0b7e6" },
        { excerpt: "“(duplicate of the thread above)”",
          authorAddress: "05af913c6b2e84d17f0a35c9be41d27672c5db91a8e30f45c71b6d2803a63f4d8",
          moderatorAddress: "8f2a41d97b05c3e19ac07f6d2841bb5390e7c4a21d68f03b57ce9a4412d0b7e6" }
    ]

    // ---- the way out ----------------------------------------------------
    //
    // Declared FIRST and outside everything conditional, so "always offered"
    // holds by construction rather than by a binding somebody has to keep
    // right. Nothing on this screen has a phase that could withdraw it, and
    // there is no phase in scope at this point in the file for a later edit to
    // reach for.
    RowLayout {
        Layout.fillWidth: true
        spacing: DTheme.itemGap

        FlatButton {
            objectName: "moderationBackButton"
            text: "Back"
            kind: "secondary"
            onClicked: screen.closed()
        }

        Item { Layout.fillWidth: true }
    }

    // ---- the standing notice --------------------------------------------
    //
    // Above the confirmation, not below it: a reader who acts on the first
    // control they see must have passed this. It is rendered as a note rather
    // than as an alarm — the absence is a scope decision, not a fault, and
    // nothing here is retryable.
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: notice.implicitHeight + 2 * DTheme.itemGap
        color: DTheme.paperDeep
        border.width: DTheme.hairline
        border.color: DTheme.rule2

        ColumnLayout {
            id: notice
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: DTheme.itemGap
            spacing: 4

            Text {
                objectName: "inertNoticeHeading"
                text: "NOTHING ON THIS SCREEN PUBLISHES ANYTHING"
                font: DTheme.label
                color: DTheme.accent
                textFormat: Text.PlainText
            }

            Text {
                objectName: "inertNoticeBody"
                text: "The core module offers no method for publishing a moderation, "
                    + "so every control here is inert: acting on one changes nothing, "
                    + "for you or for anyone else. The two lists below are examples "
                    + "held in the view, not a record of what this peer has moderated. "
                    + "This is what the build does today."
                font: DTheme.bodySmall
                color: DTheme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: DTheme.lineHeightBody
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }
        }
    }

    // ---- the confirmation ------------------------------------------------
    //
    // The 2px accent border is the design's, and it is the only 2px border on
    // any screen: this is the one block in the interface that asks for an
    // irreversible-looking decision.
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: confirm.implicitHeight + 2 * 18
        color: DTheme.field
        border.width: DTheme.border
        border.color: DTheme.accent

        ColumnLayout {
            id: confirm
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.top: parent.top
            anchors.margins: 18
            spacing: DTheme.itemGap

            Text {
                text: "Mark this post as moderated?"
                font: DTheme.display
                color: DTheme.ink
                textFormat: Text.PlainText
            }

            // The subject: who wrote it and what it said. Inset on `paper`
            // against the block's `field`, so the thing being acted on is
            // visually a quotation rather than part of the question.
            Rectangle {
                Layout.fillWidth: true
                implicitHeight: subject.implicitHeight + 20
                color: DTheme.paper
                border.width: DTheme.hairline
                border.color: DTheme.rule2

                RowLayout {
                    id: subject
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.margins: DTheme.itemGap
                    spacing: 12

                    Identicon {
                        address: screen.subjectAddress
                        size: DTheme.markInFeed
                    }

                    Text {
                        objectName: "subjectName"
                        text: screen.subjectName
                        font: DTheme.body
                        color: DTheme.ink
                        // Peer-supplied, carrying whatever characters its
                        // author typed. Never markup — this is a screen where
                        // a reader decides who somebody is.
                        textFormat: Text.PlainText
                    }

                    // The one abbreviation, owned by AddressLabel: head 8,
                    // middle 8, tail 6. No second elision is written here. A
                    // head-and-tail form is what vanity-address generators are
                    // built to defeat, and this screen is exactly where that
                    // matters.
                    AddressLabel { address: screen.subjectAddress }

                    Item { Layout.fillWidth: true }

                    Text {
                        objectName: "subjectExcerpt"
                        text: screen.subjectExcerpt
                        font: DTheme.note
                        color: DTheme.inkSoft
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        Layout.maximumWidth: parent.width / 2
                    }
                }
            }

            // **What moderating reaches, at its true extent.** This sentence is
            // the design's and it is also a requirement: a Stoa is a set of
            // peers exchanging signed ops, with no server holding the only copy
            // and no membership to revoke, so a moderation op is a published
            // judgement honest readers honour and nothing more. A screen
            // promising deletion promises what the architecture forbids, to the
            // one user whose decisions turn on the difference. The wording
            // survives a real publishing path unchanged.
            Text {
                objectName: "reachStatement"
                text: "Moderating means telling the users of the Stoa to hide this post. "
                    + "It does not delete a post, nor block a user from the network."
                font: DTheme.body
                color: DTheme.inkSoft
                wrapMode: Text.WordWrap
                lineHeight: DTheme.lineHeightBody
                textFormat: Text.PlainText
                Layout.fillWidth: true
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 16

                // INERT. `destructive` filled, then `destructive-outline`: two
                // filled reds side by side read as one decision offered twice,
                // which is why `FlatButton` carries the outline kind.
                FlatButton {
                    objectName: "markModeratedButton"
                    text: "Mark as moderated"
                    kind: "destructive"
                }

                FlatButton {
                    objectName: "moderateAuthorButton"
                    text: "Moderate all posts of this author"
                    kind: "destructive-outline"
                }

                // Cancel LEAVES, and it is the one control here that is not
                // inert. A cancel that did nothing would be the cruellest of
                // the three: it is the control a user reaches for once they
                // have decided against the action, and its failing silently
                // would strand them on a destructive screen.
                FlatButton {
                    objectName: "cancelButton"
                    text: "Cancel"
                    kind: "secondary"
                    onClicked: screen.closed()
                }
            }
        }
    }

    // ---- what is already moderated: two columns --------------------------
    //
    // TWO lists, never one. Moderating an author and moderating one of their
    // posts are different judgements with different scopes, and they are
    // independently reversible by design — a single control would make the
    // smaller reversal unavailable, so a moderator wanting one post back would
    // have to un-moderate the author. `moderation-view` contracts the
    // separation because a later screen merging them would look like a
    // simplification.
    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: DTheme.hairline
        color: DTheme.rule
    }

    RowLayout {
        Layout.fillWidth: true
        spacing: 26

        DModeratedList {
            objectName: "moderatedAuthorList"
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            Layout.alignment: Qt.AlignTop
            heading: "Moderated authors — every post omitted from the feed"
            rows: screen.moderatedAuthors

            rowDelegate: RowLayout {
                id: authorRow
                property var rowData: ({ name: "", address: "", postCount: 0 })
                spacing: 12

                // Muted: this mark stands for somebody already acted on.
                // `DTheme.markMutedAlpha` is a theme decision — how loud
                // "recessive" is on this paper — and not part of the mark's
                // determinism contract, which is why it is a token rather than
                // a literal here.
                Identicon {
                    address: authorRow.rowData.address
                    size: DTheme.markInFeed
                    muted: true
                }

                ColumnLayout {
                    spacing: 1
                    Layout.fillWidth: true

                    Text {
                        text: authorRow.rowData.name
                        font: DTheme.bodySmall
                        color: DTheme.inkMuted
                        // Peer-supplied. Never markup — this is a screen where
                        // a reader decides who somebody is.
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    AddressLabel { address: authorRow.rowData.address }
                }

                // A count of posts by an author in a FIXTURE row. **It is part
                // of the example, not a placeholder standing in for a call.**
                // Nothing enumerates moderated authors at all, so there is no
                // reply this could have come from and no position a real value
                // would fill — which is what distinguishes it from the Stoa
                // list's count placeholder, where a call could exist and does
                // not.
                Text {
                    text: authorRow.rowData.postCount + " posts"
                    font: DTheme.note
                    color: DTheme.inkMuted
                    textFormat: Text.PlainText
                }

                // INERT, and per-row: it names this author and acts on nothing
                // else. `secondary-micro` because a full-size button in a 19px
                // row out-weighs the row it acts on.
                FlatButton {
                    objectName: "unmoderateAuthorButton"
                    text: "Unmoderate"
                    kind: "secondary-micro"
                }
            }
        }

        // The vertical rule between the two columns. `fillHeight` works here:
        // the row is sized by its tallest child, which is a 150px list, so
        // there is slack for this to take. That is the case `ScreenFrame`'s
        // contract excludes — a card with no height — and this is not it.
        Rectangle {
            Layout.fillHeight: true
            Layout.preferredWidth: DTheme.hairline
            color: DTheme.rule
        }

        DModeratedList {
            objectName: "moderatedPostList"
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            Layout.alignment: Qt.AlignTop
            heading: "Moderated posts — visible here, omitted from the feed"
            rows: screen.moderatedPosts

            rowDelegate: RowLayout {
                id: postRow
                property var rowData: ({ excerpt: "", authorAddress: "",
                                         moderatorAddress: "" })
                spacing: 12

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Text {
                        text: postRow.rowData.excerpt
                        font: DTheme.note
                        color: DTheme.inkMuted
                        // Peer-supplied. Never markup.
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }

                    // BOTH addresses, and the second is the point: who
                    // moderated this is a fact a reader needs, because a Stoa's
                    // moderators are the people whose judgement they have
                    // chosen to honour. Rendered through `AddressLabel` each,
                    // so neither acquires a second abbreviation — a
                    // head-and-tail elision is what vanity-address generators
                    // are built to defeat.
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 6

                        AddressLabel { address: postRow.rowData.authorAddress }

                        Text {
                            text: "· moderated by"
                            font: DTheme.address
                            color: DTheme.inkFaint
                            textFormat: Text.PlainText
                        }

                        AddressLabel { address: postRow.rowData.moderatorAddress }

                        Item { Layout.fillWidth: true }
                    }
                }

                FlatButton {
                    objectName: "unmoderatePostButton"
                    text: "Unmoderate"
                    kind: "secondary-micro"
                }
            }
        }
    }
}
