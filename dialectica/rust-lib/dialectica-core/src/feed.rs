//! The feed: a page of thread heads, resolved and sanitised, ready to render.
//!
//! # One ordering, and it is named for what it is
//!
//! §9.1 proposes `new` and `active` as the accepted orderings and then records,
//! at length, that **both are defined in terms of a Lamport timestamp that does
//! not reach us** — so both would today fall back to ascending op id, "a hash
//! [that] carries no recency whatever". §9.1 leaves the choice open between
//! shipping §7.2's two names with a degradation notice, and shipping one
//! ordering "named for what it actually is rather than for what §7.2 intends it
//! to become".
//!
//! **This takes the second option, and implements exactly one ordering.** The
//! name is `convergent`, because that is the property the order actually has:
//! every peer holding the same ops computes the same sequence. It is not `new`,
//! it is not `active`, and it is not `top`.
//!
//! There is no ordering parameter, no comparator to select, and no enum with
//! variants nothing implements. A second ordering is a change to this module
//! when core can answer a second question; until then an accepted-but-degraded
//! `order` argument would be a method telling a caller a falsehood, which is the
//! thing §9.1 is most explicit about not doing.
//!
//! # What "convergent" claims, and the much larger thing it does not
//!
//! It claims that **this ordering** is identical on every peer holding the same
//! ops. It says nothing about the feed as a whole being identical between two
//! people, and that distinction is not pedantry — it is a design property of the
//! system.
//!
//! Identity is per-Stoa and unlinkable (§5.2), and vouching is **per-reader and
//! never published** (§7.3). Two readers holding the identical op set will
//! legitimately compute different vote weights and, once a vote-ordered feed
//! exists, different orders. Neither is stale and neither is wrong. Nothing in
//! this file should ever be read as promising a globally consistent feed, and a
//! future reader who believes that will file ordinary divergence as a
//! synchronisation defect.
//!
//! # No comparison is written here
//!
//! [`OpLog::iter_stoa`] already returns entries in
//! [`cmp_ops`](crate::arrival::cmp_ops) order. This module filters and pages
//! that sequence and contains no `sort`, no `cmp` and no `max_by` — the same
//! discipline [`crate::revision`] and [`crate::moderation`] hold, and for the
//! same reason: a second implementation of the ordering rule could disagree with
//! the first, and two orders that disagree produce no error anywhere.
//!
//! # A feed is a list of THREAD HEADS
//!
//! §9.1: "A feed is a paginated list of thread heads, not of posts." A top-level
//! post is one whose `parent` is `None` — a reply is a `Post` with a parent set
//! (`op.rs` has no `Reply` kind), so the filter is on the parent field and not on
//! a kind.
//!
//! # What each row costs, and what is deliberately absent from it
//!
//! Each row resolves its own current version (§5.7) and its own moderation state
//! (§6). §9.1 records that `current_version` being per-post "is the shape that
//! decides whether the projection stores resolved current versions or resolves on
//! read" — nothing is optimised here speculatively, and the in-memory log makes
//! the per-row call cheap.
//!
//! **No reply count and no last reply.** §9.1's proposed row carries both, and
//! §7 names them as a gap: "There is no reply count and no most-recent-reply
//! [...] Both must also be of non-hidden replies, which makes them folds over
//! moderation-resolved state rather than over raw ops." §8 then asks whether the
//! count is worth its cost before Lamport values arrive, and leaves it open. A
//! fold over every reply's moderation state, per row, for a number sitting beside
//! an ordering that carries no recency, is not the smallest thing that works — so
//! it is not here. The view renders no reply count, which is honest, rather than
//! a wrong one, which would not be.
//!
//! **No vote score.** §9.1 is explicit that votes are not staged: "a vote button
//! would publish an op that changes nothing a reader can see". Nothing reads
//! `Vote` ops, so no row carries a score.

use crate::identity::Address;
use crate::log::{OpLog, OpLogError};
use crate::moderation::{Moderation, Moderators};
use crate::op::OpKind;
use crate::revision::current_version;
use crate::sanitise::{sanitise, Sanitised};

/// The largest page a caller may ask for.
///
/// **A cap rather than a default**, and it exists because `perPage` arrives from
/// the wire. Every cross-module call is IPC (§2.4) and the reply is serialised
/// into one JSON string, so an unbounded `perPage` is a caller asking this peer
/// to build an arbitrarily large string in memory — reachable from the view, and
/// reachable from anything else that can call the module.
///
/// 100 is comfortably more than a screen and far short of a problem. It is
/// clamped rather than rejected: a caller asking for more is not attacking
/// anything, and refusing the whole page would be a worse answer than a smaller
/// one.
pub const MAX_PER_PAGE: usize = 100;

/// The default page size when a caller names none.
pub const DEFAULT_PER_PAGE: usize = 20;

/// One row of the feed: a thread head, ready to render.
///
/// Every string here has been through [`sanitise`], and the counts that came
/// back travel with it — see [`FeedRow::body`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedRow {
    /// The thread's id, which is the root post's op id (§4.1).
    ///
    /// **Never changes across edits**, which is what makes it the thing a reply
    /// names as its parent and the thing a view uses as a stable row key.
    pub thread: String,
    /// The op id of the version being rendered.
    ///
    /// Different from `thread` the moment the post has been edited. §9.1: "These
    /// are two fields because they are two facts", and a moderator acts on this
    /// one.
    pub current_version: String,
    /// The author's per-Stoa address (§5.2), hex.
    ///
    /// **An address and never a name.** There are no names in core — the
    /// generated name is a pure function of this address and is the view's to
    /// derive. Sending a name from here would put a second, forgeable identifier
    /// on the wire beside the real one.
    pub author: String,
    /// The post body, sanitised for display.
    pub body: Sanitised,
    /// Logos Storage CIDs (§4.6), sanitised.
    ///
    /// Sanitised because a CID is a peer-supplied string like any other and the
    /// view renders it. §4.6 makes an unresolvable CID a fetch outcome rather
    /// than a validation failure, so nothing here judges whether they resolve.
    pub attachments: Vec<Sanitised>,
    /// Whether the post has been revised (§5.7).
    pub is_revised: bool,
    /// Whether a moderator hid this thread's root.
    ///
    /// Only ever `true` in a `include_hidden` read — the default feed omits
    /// hidden threads entirely (§7.2 rule 4). Carried so that the show-hidden
    /// view can distinguish them, which §9.1 requires: "a reader who asked to see
    /// what was hidden is owed the knowledge of which ones those were."
    pub is_hidden: bool,
}

/// One page of the feed.
///
/// Mirrors the ecosystem's pagination shape — `{"items":[...],"page":N,
/// "hasMore":bool}` — with no total. §11.1's "never show a count of anything
/// global" is the reason there is no `total` field to render one from: a count
/// of what this peer holds is not a count of what exists, and the two are
/// indistinguishable once rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedPage {
    pub items: Vec<FeedRow>,
    pub page: usize,
    /// Whether a further page exists **in this peer's copy**.
    ///
    /// Computed by looking past the end of the slice rather than by comparing
    /// against a total, which is the same reason there is no total: the question
    /// "is there another page here" is answerable, and "how many are there
    /// anywhere" is not.
    pub has_more: bool,
}

/// Clamp a caller-supplied page size into something this peer will build.
///
/// Separate from [`list_threads`] because it is a guard and CLAUDE.md keeps a
/// guard as its own job — "so 'is it called everywhere?' stays a question with
/// an answer". There is one caller today; the point is that there is one place
/// to look.
///
/// Zero clamps **up** to the default rather than down to an empty page: a caller
/// asking for zero rows is a caller that has made a mistake, and answering with
/// a permanently empty feed would render as a Stoa with nothing in it — the one
/// confusion §11.1 obligation 5 exists to prevent.
pub fn clamp_per_page(requested: Option<usize>) -> usize {
    match requested {
        None | Some(0) => DEFAULT_PER_PAGE,
        Some(n) => n.min(MAX_PER_PAGE),
    }
}

/// One page of thread heads in a Stoa, in convergent order.
///
/// # What is filtered, and in what order
///
/// 1. Ops in this Stoa, already in [`cmp_ops`](crate::arrival::cmp_ops) order.
/// 2. Keep the ops that are **authentic** — [`crate::op::SignedOp::verify`].
///    §3.3 puts verification on read and the log "may hold junk; the reader never
///    trusts it". Without this a peer could publish a post attributed to anyone.
/// 3. Keep the `Post`s whose `parent` is `None`: the thread heads.
/// 4. Resolve each one's current version and moderation state.
/// 5. Drop the hidden ones unless `include_hidden`.
/// 6. Page what is left.
///
/// **Hiding is applied after resolution and before paging**, which is the only
/// order that gives stable pages: filtering after paging would produce short
/// pages with gaps, and a reader clicking "next" would skip rows.
///
/// # A read failure is never an empty feed
///
/// Every failure comes back as `Err`, and none is flattened into an empty list.
/// §11.1 obligation 5 and [`OpLogError`]'s own documentation both say why: "an
/// empty feed is indistinguishable from a Stoa nobody has posted in, so
/// swallowing this would render a forum whose store is broken as a forum that is
/// merely quiet."
pub fn list_threads<L: OpLog>(
    log: &L,
    moderators: &Moderators,
    stoa: &Address,
    page: usize,
    per_page: usize,
    include_hidden: bool,
) -> Result<FeedPage, OpLogError> {
    let mut rows = Vec::new();

    for entry in log.iter_stoa(stoa)? {
        // Authenticity first. The log stores forgeries deliberately (§3.3), so
        // this is the check that stops one rendering as a post by the author it
        // names. It runs before anything else reads the op's fields, because
        // every field is a claim until the signature is checked.
        if !entry.op.verify() {
            continue;
        }

        // Thread heads only: a `Post` with no parent. A reply is a `Post` with
        // `parent` set, so this is a field test rather than a kind test.
        match &entry.op.op.kind {
            OpKind::Post { parent: None, .. } => {}
            _ => continue,
        }

        let id = entry.id();

        // Resolve the version to render. `Ok(None)` cannot happen for an entry
        // this loop reached — it is a `Post` the log just handed us — but it is
        // an answer rather than a panic, because a panic aborts the module
        // process (PHASE0-FINDINGS §3).
        let Some(version) = current_version(log, &id)? else {
            continue;
        };

        // Moderation is resolved on READ, every time, by this peer (§6.2). It is
        // never cached and never decided at append time.
        let moderation = crate::moderation::resolve(log, moderators, &id)?;
        let is_hidden = matches!(moderation, Moderation::Hidden(_));
        if is_hidden && !include_hidden {
            continue;
        }

        rows.push(FeedRow {
            thread: id.to_hex(),
            current_version: version.current.id().to_hex(),
            author: entry.op.op.author.address().to_hex(),
            body: sanitise(version.body()),
            attachments: version.attachments().iter().map(|a| sanitise(a)).collect(),
            is_revised: version.is_revised(),
            is_hidden,
        });
    }

    // Page by slicing what survived. `saturating_mul`/`checked` arithmetic is
    // not needed because `per_page` is clamped and `page` is bounded below by
    // its type — but a page past the end must answer with an empty page rather
    // than panicking on the slice, which is what the `min` calls arrange.
    let start = page.saturating_mul(per_page).min(rows.len());
    let end = start.saturating_add(per_page).min(rows.len());
    let has_more = end < rows.len();

    Ok(FeedPage {
        items: rows[start..end].to_vec(),
        page,
        has_more,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrival::Arrival;
    use crate::identity::SecretKey;
    use crate::log::{Entry, MemoryOpLog};
    use crate::op::{ModerationAction, Op, OpId, SignedOp};
    use crate::stoa::{Genesis, Policy};

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    /// The genesis record, and therefore the moderator set: key 1 is creator.
    fn a_genesis() -> Genesis {
        Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        }
    }

    fn a_stoa() -> Address {
        a_genesis().address().unwrap()
    }

    fn moderators() -> Moderators {
        Moderators::of(&a_genesis()).unwrap()
    }

    /// A thread head by `author_seed`, signed by that same key.
    ///
    /// Signed by the key whose public half is in the `author` field, because an
    /// op signed by anyone else is a forgery the reader drops — which is a
    /// property under test elsewhere and must not be accidentally on here.
    fn a_thread(author_seed: u8, body: &str) -> SignedOp {
        let key = a_key(author_seed);
        Op {
            stoa: a_stoa(),
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(&key)
    }

    fn a_reply(author_seed: u8, parent: OpId, body: &str) -> SignedOp {
        let key = a_key(author_seed);
        Op {
            stoa: a_stoa(),
            author: key.public_key(),
            kind: OpKind::Post {
                thread: Some(parent),
                parent: Some(parent),
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(&key)
    }

    fn a_log(ops: Vec<SignedOp>) -> MemoryOpLog {
        let mut log = MemoryOpLog::new();
        for op in ops {
            log.append(op, Arrival::unordered()).unwrap();
        }
        log
    }

    /// The whole feed, unpaged, for tests about content rather than paging.
    fn all_of(log: &MemoryOpLog, include_hidden: bool) -> Vec<FeedRow> {
        list_threads(
            log,
            &moderators(),
            &a_stoa(),
            0,
            MAX_PER_PAGE,
            include_hidden,
        )
        .unwrap()
        .items
    }

    #[test]
    fn a_thread_head_is_returned_and_a_reply_is_not() {
        // §9.1: "A feed is a paginated list of thread heads, not of posts." A
        // feed including replies would render every reply as a top-level item.
        let head = a_thread(2, "the head");
        let reply = a_reply(3, head.op.id(), "a reply");
        let log = a_log(vec![head.clone(), reply.clone()]);

        let rows = all_of(&log, false);
        assert_eq!(rows.len(), 1, "only the head is a feed row");
        assert_eq!(rows[0].thread, head.op.id().to_hex());
        // And the reply really is in the log, so this is not passing because
        // the fixture failed to store it.
        assert_eq!(log.len().unwrap(), 2);
    }

    #[test]
    fn a_forged_post_never_reaches_the_feed() {
        // THE read-path check. The log stores forgeries deliberately (§3.3), so
        // a feed that did not verify would render a post attributed to whoever
        // the attacker named.
        let victim = a_key(2);
        let attacker = a_key(9);
        let op = Op {
            stoa: a_stoa(),
            author: victim.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "I did not write this".to_string(),
                attachments: vec![],
            },
        };
        let forged = SignedOp {
            signature: crate::identity::sign_op_bytes(&attacker, &op.canonical_bytes()),
            op,
        };
        assert!(!forged.verify(), "the fixture must actually be a forgery");

        let genuine = a_thread(2, "genuine");
        let log = a_log(vec![forged, genuine.clone()]);

        let rows = all_of(&log, false);
        assert_eq!(rows.len(), 1, "the forgery must not render");
        assert_eq!(rows[0].thread, genuine.op.id().to_hex());
        // The forgery IS in the log — so this test would fail if verification
        // were removed, rather than passing because the op was never stored.
        assert_eq!(log.len().unwrap(), 2);
    }

    #[test]
    fn a_hidden_thread_is_omitted_by_default_and_returned_on_request() {
        // §7.2 rule 4: hidden threads are omitted, and the parameter that
        // includes them is explicit. Both directions, because a filter that
        // always omitted and one that always included each pass half of this.
        let head = a_thread(2, "moderated");
        let other = a_thread(3, "untouched");
        let creator = a_key(1);
        let hide = Op {
            stoa: a_stoa(),
            author: creator.public_key(),
            kind: OpKind::Moderate {
                target: head.op.id(),
                action: ModerationAction::Hide,
            },
        }
        .sign(&creator);
        let log = a_log(vec![head.clone(), other.clone(), hide]);

        let default_view: Vec<String> = all_of(&log, false).iter().map(|r| r.thread.clone()).collect();
        assert_eq!(default_view, vec![other.op.id().to_hex()]);

        let hidden_view = all_of(&log, true);
        assert_eq!(hidden_view.len(), 2);
        let hidden_row = hidden_view
            .iter()
            .find(|r| r.thread == head.op.id().to_hex())
            .expect("the hidden thread must appear in the show-hidden view");
        assert!(
            hidden_row.is_hidden,
            "a reader who asked to see what was hidden is owed knowing which"
        );
        let visible_row = hidden_view
            .iter()
            .find(|r| r.thread == other.op.id().to_hex())
            .unwrap();
        assert!(!visible_row.is_hidden);
    }

    #[test]
    fn a_forged_hide_does_not_remove_anything_from_the_feed() {
        // Authority, not merely authenticity. Key 9 signs a genuine op that is
        // genuinely not a moderator's, and the post must stay.
        let head = a_thread(2, "still here");
        let impostor = a_key(9);
        let hide = Op {
            stoa: a_stoa(),
            author: impostor.public_key(),
            kind: OpKind::Moderate {
                target: head.op.id(),
                action: ModerationAction::Hide,
            },
        }
        .sign(&impostor);
        assert!(hide.verify(), "the op is authentic; it is the AUTHORITY that fails");

        let log = a_log(vec![head.clone(), hide]);
        let rows = all_of(&log, false);
        assert_eq!(rows.len(), 1, "a non-moderator must not be able to hide a post");
        assert!(!rows[0].is_hidden);
    }

    #[test]
    fn a_row_renders_the_current_version_and_says_it_was_edited() {
        // §5.7 through the feed: the body is the revision's, the thread id is
        // still the original's, and `is_revised` is set. A feed rendering the
        // original body would show text the author replaced.
        let head = a_thread(2, "the original words");
        let author = a_key(2);
        let revision = Op {
            stoa: a_stoa(),
            author: author.public_key(),
            kind: OpKind::Revise {
                target: head.op.id(),
                body: "the replacement words".to_string(),
                attachments: vec![],
            },
        }
        .sign(&author);
        let log = a_log(vec![head.clone(), revision.clone()]);

        let rows = all_of(&log, false);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].body.text, "the replacement words");
        assert!(rows[0].is_revised);
        assert_eq!(
            rows[0].thread,
            head.op.id().to_hex(),
            "the thread id is the ORIGINAL's and never moves"
        );
        assert_eq!(
            rows[0].current_version,
            revision.op.id().to_hex(),
            "the current version is the revision's id — the two are different facts"
        );
        assert_ne!(rows[0].thread, rows[0].current_version);
    }

    #[test]
    fn an_unrevised_row_reports_the_two_ids_as_equal_and_is_not_edited() {
        // The other half of the pair above. Without it, a bug setting
        // `is_revised` unconditionally would pass every revision test.
        let head = a_thread(2, "never touched");
        let log = a_log(vec![head.clone()]);
        let rows = all_of(&log, false);
        assert!(!rows[0].is_revised);
        assert_eq!(rows[0].thread, rows[0].current_version);
    }

    #[test]
    fn a_body_is_sanitised_before_it_leaves_core() {
        // The obligation, at the boundary it actually has to hold at. The op
        // must keep the bytes exactly (op.rs pins that separately); what the
        // feed hands out must not.
        let hostile = "p\u{0430}ypal\u{202E}gnp.js";
        let head = a_thread(2, hostile);
        let log = a_log(vec![head.clone()]);

        let rows = all_of(&log, false);
        assert_eq!(rows[0].body.removed, 1, "the bidi override must be removed");
        assert_eq!(rows[0].body.marked, 1, "the Cyrillic letter must be marked");
        assert!(!rows[0].body.text.contains('\u{202E}'));
        assert!(
            rows[0].body.text.contains('\u{0430}'),
            "marking must not correct"
        );

        // And the LOG still holds the original bytes, unchanged. Sanitisation
        // is a rendering, never a rewrite of the record.
        let stored = log.get(&head.op.id()).unwrap().unwrap();
        match &stored.op.op.kind {
            OpKind::Post { body, .. } => assert_eq!(body, hostile),
            other => panic!("expected a post, got {other:?}"),
        }
    }

    #[test]
    fn attachments_are_sanitised_too() {
        // A CID is a peer-supplied string the view renders, so it goes through
        // the same path. A sanitiser applied to bodies alone leaves the
        // obligation half-met.
        let key = a_key(2);
        let head = Op {
            stoa: a_stoa(),
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "see the attachment".to_string(),
                attachments: vec!["cid\u{202E}txt.exe".to_string()],
            },
        }
        .sign(&key);
        let log = a_log(vec![head]);

        let rows = all_of(&log, false);
        assert_eq!(rows[0].attachments.len(), 1);
        assert_eq!(rows[0].attachments[0].removed, 1);
        assert!(!rows[0].attachments[0].text.contains('\u{202E}'));
    }

    #[test]
    fn only_this_stoas_threads_are_returned() {
        // A feed leaking another Stoa's posts would break the per-Stoa identity
        // model as well as the rendering.
        let elsewhere = Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Somewhere else".to_string(),
        }
        .address()
        .unwrap();
        let key = a_key(2);
        let foreign = Op {
            stoa: elsewhere,
            author: key.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "not here".to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);
        let mine = a_thread(2, "here");
        let log = a_log(vec![foreign, mine.clone()]);

        let rows = all_of(&log, false);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].thread, mine.op.id().to_hex());
    }

    #[test]
    fn the_author_is_an_address_and_matches_the_key_that_signed() {
        // §11.1 obligation 6: the address is the identity. It must be the
        // address of the key that actually signed, not of the field's claim —
        // though verification has already made those the same thing by here.
        let head = a_thread(4, "mine");
        let log = a_log(vec![head]);
        let rows = all_of(&log, false);
        assert_eq!(rows[0].author, a_key(4).public_key().address().to_hex());
        assert_ne!(rows[0].author, a_key(2).public_key().address().to_hex());
    }

    // ─── Ordering ─────────────────────────────────────────────────────────

    #[test]
    fn two_peers_holding_the_same_ops_render_the_same_order() {
        // The one property "convergent" actually names. Two logs, opposite
        // insertion sequences, identical output. A feed ordering by insertion
        // would return reversed lists here.
        let ops = vec![
            a_thread(2, "alpha"),
            a_thread(3, "beta"),
            a_thread(4, "gamma"),
            a_thread(5, "delta"),
        ];
        let forwards = a_log(ops.clone());
        let mut backwards = MemoryOpLog::new();
        for op in ops.iter().rev() {
            backwards.append(op.clone(), Arrival::unordered()).unwrap();
        }

        let a: Vec<String> = all_of(&forwards, false).iter().map(|r| r.thread.clone()).collect();
        let b: Vec<String> = all_of(&backwards, false).iter().map(|r| r.thread.clone()).collect();
        assert_eq!(a, b);
        assert_eq!(a.len(), 4, "the fixture must exercise all four");
    }

    #[test]
    fn the_feed_order_is_the_logs_order_and_is_not_re_sorted_here() {
        // This module writes no comparison, so the sequence must be exactly the
        // log's, filtered. A `sort` added here would diverge from `cmp_ops`
        // silently — both orders are internally consistent and nothing errors.
        let ops = vec![
            a_thread(2, "alpha"),
            a_thread(3, "beta"),
            a_thread(4, "gamma"),
        ];
        let log = a_log(ops);

        let from_log: Vec<String> = log
            .iter_stoa(&a_stoa())
            .unwrap()
            .iter()
            .map(|e| e.id().to_hex())
            .collect();
        let from_feed: Vec<String> = all_of(&log, false).iter().map(|r| r.thread.clone()).collect();
        assert_eq!(from_feed, from_log);
    }

    // ─── Paging ───────────────────────────────────────────────────────────

    #[test]
    fn pages_partition_the_feed_with_no_gap_and_no_repeat() {
        // The property paging must have and the one an off-by-one breaks. Three
        // pages of two over five rows: the concatenation must equal the whole
        // feed exactly.
        let ops: Vec<SignedOp> = (0..5).map(|i| a_thread(2 + i, &format!("post {i}"))).collect();
        let log = a_log(ops);
        let whole: Vec<String> = all_of(&log, false).iter().map(|r| r.thread.clone()).collect();
        assert_eq!(whole.len(), 5);

        let mut seen = Vec::new();
        for page in 0..3 {
            let p = list_threads(&log, &moderators(), &a_stoa(), page, 2, false).unwrap();
            assert_eq!(p.page, page);
            seen.extend(p.items.iter().map(|r| r.thread.clone()));
        }
        assert_eq!(seen, whole, "pages must partition the feed in order");
    }

    #[test]
    fn has_more_is_true_exactly_while_a_further_page_exists() {
        // At the boundary, not past it: with 4 rows and a page size of 2, page
        // 1 is the LAST page and must report false. A test using 10 rows would
        // pass against an implementation that always said true.
        let ops: Vec<SignedOp> = (0..4).map(|i| a_thread(2 + i, &format!("post {i}"))).collect();
        let log = a_log(ops);

        let first = list_threads(&log, &moderators(), &a_stoa(), 0, 2, false).unwrap();
        assert_eq!(first.items.len(), 2);
        assert!(first.has_more, "page 0 of 2 has a page after it");

        let last = list_threads(&log, &moderators(), &a_stoa(), 1, 2, false).unwrap();
        assert_eq!(last.items.len(), 2);
        assert!(
            !last.has_more,
            "the final FULL page must not claim another follows"
        );
    }

    #[test]
    fn an_exactly_full_single_page_does_not_claim_more() {
        // The other boundary: rows == per_page exactly. An implementation
        // computing `has_more` as `start + per_page <= len` gets this wrong.
        let ops: Vec<SignedOp> = (0..3).map(|i| a_thread(2 + i, &format!("post {i}"))).collect();
        let log = a_log(ops);
        let p = list_threads(&log, &moderators(), &a_stoa(), 0, 3, false).unwrap();
        assert_eq!(p.items.len(), 3);
        assert!(!p.has_more);
    }

    #[test]
    fn a_page_past_the_end_is_empty_rather_than_a_panic() {
        // Reachable from the wire: `page` arrives from a caller. A slice past
        // the end would panic, and a panic aborts the module process.
        let log = a_log(vec![a_thread(2, "only one")]);
        let p = list_threads(&log, &moderators(), &a_stoa(), 99, 20, false).unwrap();
        assert!(p.items.is_empty());
        assert!(!p.has_more);
        assert_eq!(p.page, 99);
    }

    #[test]
    fn an_enormous_page_number_does_not_overflow() {
        // `page * per_page` on a caller-supplied `page` is the multiplication
        // that overflows. In release builds that wraps silently, which would
        // produce a page from the middle of the feed for a caller who asked for
        // one past the end.
        let log = a_log(vec![a_thread(2, "only one")]);
        let p = list_threads(&log, &moderators(), &a_stoa(), usize::MAX, 20, false).unwrap();
        assert!(p.items.is_empty());
        assert!(!p.has_more);
    }

    #[test]
    fn hidden_threads_are_dropped_before_paging_not_after() {
        // The bug this pins: filtering after slicing yields short pages with
        // gaps, and a reader clicking "next" silently skips rows. Two of four
        // threads are hidden, so one full page of two must come back — not a
        // page of two containing the survivors of the first two.
        let heads: Vec<SignedOp> = (0..4).map(|i| a_thread(2 + i, &format!("post {i}"))).collect();
        let creator = a_key(1);
        let log_all = a_log(heads.clone());
        // Hide whichever two the convergent order puts FIRST, so that a
        // filter-after-paging implementation would return an empty first page.
        let order: Vec<String> = all_of(&log_all, false).iter().map(|r| r.thread.clone()).collect();
        let mut ops = heads.clone();
        for hex in order.iter().take(2) {
            ops.push(
                Op {
                    stoa: a_stoa(),
                    author: creator.public_key(),
                    kind: OpKind::Moderate {
                        target: OpId::from_hex(hex).unwrap(),
                        action: ModerationAction::Hide,
                    },
                }
                .sign(&creator),
            );
        }
        let log = a_log(ops);

        let p = list_threads(&log, &moderators(), &a_stoa(), 0, 2, false).unwrap();
        assert_eq!(
            p.items.len(),
            2,
            "the first page must be full of VISIBLE rows, not the survivors of the first two"
        );
        assert!(!p.has_more, "two visible rows fill exactly one page of two");
        assert_eq!(
            p.items.iter().map(|r| r.thread.clone()).collect::<Vec<_>>(),
            order[2..].to_vec()
        );
    }

    // ─── The empty and failed states, which must never look alike ─────────

    #[test]
    fn an_empty_stoa_is_an_empty_page_and_not_an_error() {
        // A Stoa this peer holds nothing for is an ANSWER. §3.3 makes a partial
        // set the normal case.
        let log = MemoryOpLog::new();
        let p = list_threads(&log, &moderators(), &a_stoa(), 0, 20, false).unwrap();
        assert!(p.items.is_empty());
        assert!(!p.has_more);
    }

    #[test]
    fn a_store_failure_is_an_error_and_never_an_empty_feed() {
        // §11.1 obligation 5, at the layer where the confusion would be
        // introduced. A log that fails every read must produce `Err`, because
        // an empty feed and a broken store mean opposite things and look
        // identical once rendered.
        struct BrokenLog;
        impl OpLog for BrokenLog {
            fn append(
                &mut self,
                _op: SignedOp,
                _arrival: Arrival,
            ) -> Result<crate::log::Appended, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn get(&self, _id: &OpId) -> Result<Option<Entry>, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn iter(&self) -> Result<Vec<Entry>, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn iter_stoa(&self, _stoa: &Address) -> Result<Vec<Entry>, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn iter_target(&self, _target: &OpId) -> Result<Vec<Entry>, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
            fn len(&self) -> Result<usize, OpLogError> {
                Err(OpLogError::Storage("the disk is on fire".into()))
            }
        }

        let err = list_threads(&BrokenLog, &moderators(), &a_stoa(), 0, 20, false)
            .expect_err("a store failure must not be flattened into an empty feed");
        assert!(
            err.to_string().contains("the disk is on fire"),
            "the underlying reason must survive to the caller, got {err}"
        );
    }

    // ─── The page-size guard ──────────────────────────────────────────────

    #[test]
    fn per_page_is_clamped_at_both_ends() {
        assert_eq!(clamp_per_page(None), DEFAULT_PER_PAGE);
        assert_eq!(clamp_per_page(Some(0)), DEFAULT_PER_PAGE);
        assert_eq!(clamp_per_page(Some(1)), 1);
        // At the boundary rather than past it: MAX is allowed, MAX+1 is not.
        assert_eq!(clamp_per_page(Some(MAX_PER_PAGE)), MAX_PER_PAGE);
        assert_eq!(clamp_per_page(Some(MAX_PER_PAGE + 1)), MAX_PER_PAGE);
        assert_eq!(clamp_per_page(Some(usize::MAX)), MAX_PER_PAGE);
    }
}
