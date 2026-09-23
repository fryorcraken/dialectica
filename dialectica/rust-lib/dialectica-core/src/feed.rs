//! The feed: a page of thread heads, resolved and sanitised, ready to render.
//!
//! # One ordering, and it is named for what it is
//!
//! §9.1 proposes `new` and `active` as the accepted orderings, and leaves the
//! choice open between shipping §7.2's two names with a degradation notice, and
//! shipping one ordering "named for what it actually is rather than for what
//! §7.2 intends it to become".
//!
//! **This takes the second option, and implements exactly one ordering.** The
//! name is `convergent`, because that is the property the order actually has:
//! every peer holding the same ops computes the same sequence. It is not `new`,
//! it is not `active`, and it is not `top`.
//!
//! # The reason for that name survived the arrival of a Lamport counter, and the
//! premise it was first argued from did not
//!
//! This header used to argue the name from "no Lamport timestamp reaches us, so
//! any ordering falls back to ascending op id, a hash carrying no recency". A
//! counter now reaches us — inside the signed op preimage rather than from the
//! transport — and [`cmp_ops`](crate::arrival::cmp_ops) leads with it, so this
//! feed **is** counter-ordered. The old premise is withdrawn.
//!
//! **The name is still `convergent`, on a different and narrower argument.** A
//! Lamport counter is **causal, not temporal**: it says its author had seen
//! something at N, never *when*. Two ops at counters five apart were not written
//! five of anything apart, and an author who has seen nothing publishes at one
//! however long they waited. So `new` remains a claim this ordering cannot
//! support, and `convergent` remains what it can: every peer holding the same
//! ops computes the same sequence, which is now true *because* the counter is in
//! the preimage rather than despite there being nothing to order on.
//!
//! The op's wall-clock is **not** the answer either, and must not be read as
//! one. It is display-only, reachable only as formatted text
//! (see [`crate::asserted_time`]), and ordering on a value its author chooses
//! freely is the censorship vector `op-ordering` refuses.
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
//! # A row reports its thread's visible replies: how many, and which is latest
//!
//! **This header used to argue for leaving both out, and that argument is
//! withdrawn.** It held that a fold over every reply's moderation state, for a
//! number no caller had asked for, was not the smallest thing that works. Issue
//! #100 is the caller: a row that cannot say whether anyone has answered, and a
//! later `active` ordering that has nothing to order on until the latest reply
//! exists. The `feed-read` capability is the contract, and the `reply-count`
//! change's `design.md` records the withdrawal.
//!
//! What survives from the old argument is the part that was never about cost:
//! both values are **folds over moderation-resolved state**, never over raw ops.
//! A count including hidden replies looks right and is wrong, and so does a latest
//! reply that names one.
//!
//! Three rules decide what the fold sees, and none of them is written here:
//!
//! - **Which thread a reply is in** is [`thread_of`]'s answer — the parent chain,
//!   never the reply's own `thread` field, which is its author's claim. A count
//!   read from that field would let any peer inflate any thread's count, or
//!   become its latest reply, by naming it.
//! - **Whether a reply is hidden** is [`crate::moderation::resolve`]'s answer, and
//!   the include-hidden flag does not reach the fold at all. The flag decides
//!   which rows appear; it does not widen what a row counts.
//! - **Which reply is latest** is the first one the fold meets in
//!   [`OpLog::iter_stoa`]'s order — so the ordering rule picks it, and this module
//!   still writes no comparison.
//!
//! **The count is of what this peer holds**, for the reason [`FeedPage`] carries
//! no total: a count of what one machine holds is not a count of what exists.
//! Nothing here may call it the thread's total, and a view rendering it bare would
//! be making that claim for it.
//!
//! **No vote score.** §9.1 is explicit that votes are not staged: "a vote button
//! would publish an op that changes nothing a reader can see". Nothing reads
//! `Vote` ops, so no row carries a score.

use crate::identity::Address;
use crate::log::{Entry, OpLog, OpLogError};
use crate::moderation::{Moderation, Moderators};
use crate::op::{OpId, OpKind};
use crate::revision::current_version;
use crate::sanitise::{sanitise, Sanitised};
use crate::thread::thread_of;
use std::collections::HashMap;
use std::num::NonZeroUsize;

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
    /// The author's per-Stoa **public key** (§5.2), hex.
    ///
    /// **The key is the identity.** No display name rides beside it: the
    /// `generated-names` capability requires that a name never travels, on any
    /// reply, because a derived value beside the material it derives from is two
    /// values that must agree and could disagree — and a name on the wire is one
    /// a relay could strip or forge.
    ///
    /// **This field carried an author address until issue #80**, and that was the
    /// gap the comment here used to record as owed: a name and a mark are pure
    /// functions of the **public key**, and a caller holding only an address
    /// cannot arrive at either, an address being a hash from which no key is
    /// recoverable. Carrying the key closes it — the row now supplies the
    /// derivation's input, so a caller can render both channels.
    ///
    /// What a caller may *not* conclude from a name it renders is contracted by
    /// `generated-names`, under *"A name is never unique, never an identifier,
    /// and never numbered"*, which binds whatever renders this row.
    ///
    /// **No capability owns this reply's shape, and that is stated rather than
    /// hidden.** `generated-names` forbids a name on a feed row; nothing
    /// specifies what a feed row *does* carry. So the change from an address's
    /// hex to the key's was made with the code as its only authority, because
    /// leaving it would have the row carry an identifier whose derivation no
    /// longer exists. Writing that requirement is a capability's worth of work
    /// and is deliberately not bundled into a deletion.
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
    /// The thread's visible replies, or `None` where it has none.
    ///
    /// **One `Option` rather than a count and an optional id side by side**, so
    /// the two cannot disagree: a row with replies has a latest one, and a row
    /// without has neither. Read it through [`reply_count`](Self::reply_count) and
    /// [`latest_reply`](Self::latest_reply), which are the two facts the wire
    /// reports.
    pub replies: Option<Replies>,
}

impl FeedRow {
    /// How many of the thread's replies this peer holds that are not hidden.
    ///
    /// Zero where there are none, never absent.
    pub fn reply_count(&self) -> usize {
        self.replies.as_ref().map_or(0, |r| r.count.get())
    }

    /// The op id of the visible reply the ordering rule places first, or `None`
    /// where the thread has no visible reply.
    pub fn latest_reply(&self) -> Option<&str> {
        self.replies.as_ref().map(|r| r.latest.as_str())
    }
}

/// What a row reports about its thread's visible replies, where it has any.
///
/// **The count cannot be zero**, which is what keeps a latest reply from sitting
/// beside a count saying there is nothing to be latest. A thread with no visible
/// reply has no `Replies` at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replies {
    /// How many replies are counted. Over this peer's copy, never a total.
    pub count: NonZeroUsize,
    /// The reply post's own op id, hex — not its current version's. **Latest by
    /// the ordering rule**, which is causal: it says which reply leads the
    /// forum's order, never which was written most recently in time.
    pub latest: String,
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
/// 6. Attach each head's visible replies — see [`visible_replies_by_thread`].
/// 7. Page what is left.
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
    let entries = log.iter_stoa(stoa)?;
    // Folded ONCE over the Stoa rather than per row, and before the heads are
    // filtered: `include_hidden` is deliberately not an argument, because it
    // decides which rows appear and never what a row counts.
    let mut replies = visible_replies_by_thread(log, moderators, &entries)?;
    let mut rows = Vec::new();

    for entry in &entries {
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
            author: entry.op.op.author.to_hex(),
            body: sanitise(version.body()),
            attachments: version.attachments().iter().map(|a| sanitise(a)).collect(),
            is_revised: version.is_revised(),
            is_hidden,
            // `remove` rather than `get(..).cloned()`: each thread has one row,
            // so the tally is handed over rather than copied.
            replies: replies.remove(&id),
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

/// Every thread's visible replies, keyed by the root each reply's parent chain
/// reaches.
///
/// # What is counted
///
/// A reply is counted under a thread when it verifies, is a `Post` with a
/// parent, its chain reaches that thread's root through [`thread_of`], and
/// [`crate::moderation::resolve`] does not report it hidden. That is exactly
/// what a thread read with hidden content excluded returns beside its root, and
/// it is the same two functions deciding it, so the feed and the thread screen
/// cannot disagree about which replies exist.
///
/// Nothing else is read. Not the reply's `thread` field, not its parent's
/// moderation state — a reply beneath a hidden reply is still counted — and not
/// any op that is not a post: a revision, a vote or a moderation naming a reply
/// is skipped by its kind before anything else looks at it.
///
/// # The latest reply is the FIRST one met
///
/// `entries` arrive in [`cmp_ops`](crate::arrival::cmp_ops) order, so the first
/// visible reply met for a thread is the one the ordering rule places first.
/// `or_insert_with` keeps it and every later reply only adds to the count. No
/// value is compared here, for the reason this module's header gives.
///
/// Its place is the reply POST's own, because only posts are counted: a revision
/// is a different op, skipped by its kind, and cannot lift an old reply to the
/// front. The id recorded is the post's for the same reason.
///
/// # Over the whole Stoa, not the requested page
///
/// Every reply in `entries` is placed and resolved, whether or not its thread
/// ends up on the page asked for. The head loop in [`list_threads`] already
/// resolves every head in the Stoa on every call, so this keeps the read in the
/// same cost class rather than adding a second shape of work. It also means a
/// store failure met on any reply fails the read, including one under a thread
/// that is not on this page.
///
/// # Terminates, and cannot abort, over whatever the log holds
///
/// The only loop is over `entries`, and [`thread_of`]'s walk terminates on a
/// visited set over any parent references a peer chose. The count saturates
/// rather than overflowing, though no log reaches `usize::MAX` ops.
fn visible_replies_by_thread<L: OpLog>(
    log: &L,
    moderators: &Moderators,
    entries: &[Entry],
) -> Result<HashMap<OpId, Replies>, OpLogError> {
    let mut by_thread: HashMap<OpId, Replies> = HashMap::new();

    for entry in entries {
        // Authenticity before any field is read, as in the head loop.
        if !entry.op.verify() {
            continue;
        }
        // Replies only: the root is never counted, and neither is any op that
        // is not a post, whatever post it names.
        if !matches!(
            entry.op.op.kind,
            OpKind::Post {
                parent: Some(_),
                ..
            }
        ) {
            continue;
        }

        let id = entry.id();

        // THE membership rule, owned by `thread-read`. A chain that cannot be
        // completed places the reply under no thread, so it is counted nowhere.
        let Some(root) = thread_of(log, &id)? else {
            continue;
        };

        if crate::moderation::resolve(log, moderators, &id)?.is_hidden() {
            continue;
        }

        by_thread
            .entry(root)
            .and_modify(|replies| replies.count = replies.count.saturating_add(1))
            .or_insert_with(|| Replies {
                count: NonZeroUsize::MIN,
                latest: id.to_hex(),
            });
    }

    Ok(by_thread)
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
            clock: None,
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
            clock: None,
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
            clock: None,
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
        // And an AUTHORSHIP forgery rather than junk bytes: valid under the
        // attacker's own key. `!verify()` is a refusal guard that fabricated
        // bytes satisfy identically, so without this the test would show "a bad
        // signature does not render" rather than "a forged author does not
        // render" — the property it names, and the one the signature check is now
        // solely responsible for since issue #80 deleted the address guard.
        assert!(
            crate::identity::verify_authored_op(
                &attacker.public_key().to_bytes(),
                &forged.op.canonical_bytes(),
                &forged.signature.to_bytes()
            ),
            "the attacker's signature must be valid under the attacker's own key, \
             or this fixture is a junk signature rather than a forgery"
        );

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
            clock: None,
            kind: OpKind::Moderate {
                target: head.op.id(),
                action: ModerationAction::Hide,
            },
        }
        .sign(&creator);
        let log = a_log(vec![head.clone(), other.clone(), hide]);

        let default_view: Vec<String> = all_of(&log, false)
            .iter()
            .map(|r| r.thread.clone())
            .collect();
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
            clock: None,
            kind: OpKind::Moderate {
                target: head.op.id(),
                action: ModerationAction::Hide,
            },
        }
        .sign(&impostor);
        assert!(
            hide.verify(),
            "the op is authentic; it is the AUTHORITY that fails"
        );

        let log = a_log(vec![head.clone(), hide]);
        let rows = all_of(&log, false);
        assert_eq!(
            rows.len(),
            1,
            "a non-moderator must not be able to hide a post"
        );
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
            clock: None,
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
            clock: None,
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
            clock: None,
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
    fn a_row_carries_the_public_key_and_no_derived_display_name() {
        // "No reply SHALL carry a display name: not a feed row, not a thread
        // item, not an onboarding slate candidate."
        //
        // **Pinned POSITIVELY rather than left as the current shape.** An
        // earlier pass put a `displayName` on this row; the owner reversed it,
        // and an absence nobody asserts is an absence a later pass restores
        // with every test still green. The same shape caught a re-exclusion in
        // `names.rs` — `the_lists_carry_no_exclusion_of_any_kind` pins words as
        // PRESENT for exactly this reason.
        //
        // **The JSON side is pinned separately and more strongly**, by
        // `the_feed_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
        // in `wire.rs`, which asserts the row's whole key set — so a restored
        // `displayName` fails there on an ADDED key. This half pins the struct,
        // which is where such a field would be added first.
        //
        // Both are wanted: a field on `FeedRow` that never reaches the wire
        // would not violate the requirement but is the step before one that
        // does, and a reader of `feed.rs` should not have to open `wire.rs` to
        // learn that the absence is deliberate.
        // NO SPEC: that the `author` field carries the signing key's hex is this
        // change's choice. No capability owns the feed reply's shape —
        // `generated-names` forbids a name on a feed row and nothing states what
        // a row does carry — so the move from an author address's hex to the
        // key's was made with the code as its only authority, because leaving it
        // would have the row name an author in a form whose derivation no longer
        // exists. `design.md` §5 carries it.
        let head = a_thread(4, "mine");
        let log = a_log(vec![head]);
        let rows = all_of(&log, false);

        assert_eq!(rows[0].author, a_key(4).public_key().to_hex());

        // The struct's fields, enumerated by destructuring rather than by a
        // list someone keeps up to date: adding a field to `FeedRow` makes this
        // stop compiling, which is a louder failure than an assertion and one
        // that cannot go stale.
        //
        // `replies` joined the set with issue #100, and it carries a count and a
        // reply's op id — neither of them a name.
        let FeedRow {
            thread: _,
            current_version: _,
            author: _,
            body: _,
            attachments: _,
            is_revised: _,
            is_hidden: _,
            replies: _,
        } = &rows[0];
    }

    #[test]
    fn two_keys_that_derive_one_name_stay_two_rows_with_two_keys() {
        // The collision case, with two REAL colliding keys rather than a stubbed
        // derivation — the pair was found by searching the shipped scheme (see
        // `names::tests_support::COLLIDING_SEED_A`).
        //
        // **What the feed owes here is narrower than it was**, now that no row
        // carries a name: two identities whose keys happen to derive one name
        // must still be two rows with two keys. The feed must not collapse,
        // deduplicate or otherwise merge them — the keys are what tells them
        // apart, and they are the only thing on the row that could.
        //
        // The pair is kept rather than replaced by two arbitrary keys because it
        // exercises the case where everything a reader *sees rendered* is
        // identical. Two unrelated keys would not.
        let ka = SecretKey::from_bytes(&crate::names::tests_support::COLLIDING_SEED_A).unwrap();
        let kb = SecretKey::from_bytes(&crate::names::tests_support::COLLIDING_SEED_B).unwrap();

        // The fixture must actually be a colliding pair, or this test is about
        // two ordinary keys and its name is a lie.
        assert_eq!(
            crate::names::display_name(&ka.public_key()).render(),
            crate::names::display_name(&kb.public_key()).render(),
            "the fixture's two keys must still derive the same name"
        );

        let post = |key: &SecretKey, body: &str| {
            Op {
                stoa: a_stoa(),
                author: key.public_key(),
                clock: None,
                kind: OpKind::Post {
                    thread: None,
                    parent: None,
                    body: body.to_string(),
                    attachments: vec![],
                },
            }
            .sign(key)
        };

        let log = a_log(vec![post(&ka, "from a"), post(&kb, "from b")]);
        let rows = all_of(&log, false);
        assert_eq!(rows.len(), 2, "the fixture must produce two rows");

        assert_ne!(
            rows[0].author, rows[1].author,
            "the two rows must carry different author keys"
        );
        assert_eq!(
            rows[0].author,
            ka.public_key().to_hex(),
            "each row's key must be its own signer's"
        );
        assert_eq!(rows[1].author, kb.public_key().to_hex());
    }

    #[test]
    fn two_posts_by_one_author_carry_one_key() {
        let first = a_thread(4, "one");
        let key = a_key(4);
        let second = Op {
            stoa: a_stoa(),
            author: key.public_key(),
            clock: None,
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "two".to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);
        let log = a_log(vec![first, second]);

        let rows = all_of(&log, false);
        assert_eq!(rows.len(), 2, "the fixture must produce two rows");
        assert_eq!(rows[0].author, rows[1].author);
    }

    #[test]
    fn a_name_in_a_posts_body_reaches_no_author_field() {
        // "A name does not travel as content." An op has no name field to strip
        // — `OpKind::Post` carries `body`, `attachments`, `thread` and `parent`
        // and nothing else — so a name-shaped string in a body is body text and
        // is never treated as the author's name.
        //
        // Satisfied by construction rather than by a filter: there is no field
        // for a name to arrive in, and now no field for one to leave in either.
        // This asserts the consequence anyway, because "by construction" is a
        // claim about a data shape that a future field could silently undo.
        //
        // The body is a REAL derivable name (`pensive aporia of lampsakos`,
        // each word verified by index in `names.rs`), so the test exercises a
        // string the scheme could actually produce rather than one it could not.
        let head = a_thread(4, "pensive aporia of lampsakos");
        let log = a_log(vec![head]);
        let rows = all_of(&log, false);

        assert_eq!(rows[0].author, a_key(4).public_key().to_hex());
        assert_ne!(
            rows[0].author, "pensive aporia of lampsakos",
            "the body's text must not become the author field"
        );
    }

    #[test]
    fn the_author_is_the_public_key_that_signed() {
        // §11.1 obligation 6: the public key is the identity. It must be the key
        // that actually signed, not the field's claim — though verification has
        // already made those the same thing by here.
        let head = a_thread(4, "mine");
        let log = a_log(vec![head]);
        let rows = all_of(&log, false);
        assert_eq!(rows[0].author, a_key(4).public_key().to_hex());
        assert_ne!(rows[0].author, a_key(2).public_key().to_hex());
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

        let a: Vec<String> = all_of(&forwards, false)
            .iter()
            .map(|r| r.thread.clone())
            .collect();
        let b: Vec<String> = all_of(&backwards, false)
            .iter()
            .map(|r| r.thread.clone())
            .collect();
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
        let from_feed: Vec<String> = all_of(&log, false)
            .iter()
            .map(|r| r.thread.clone())
            .collect();
        assert_eq!(from_feed, from_log);
    }

    // ─── Paging ───────────────────────────────────────────────────────────

    #[test]
    fn pages_partition_the_feed_with_no_gap_and_no_repeat() {
        // The property paging must have and the one an off-by-one breaks. Three
        // pages of two over five rows: the concatenation must equal the whole
        // feed exactly.
        let ops: Vec<SignedOp> = (0..5)
            .map(|i| a_thread(2 + i, &format!("post {i}")))
            .collect();
        let log = a_log(ops);
        let whole: Vec<String> = all_of(&log, false)
            .iter()
            .map(|r| r.thread.clone())
            .collect();
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
        let ops: Vec<SignedOp> = (0..4)
            .map(|i| a_thread(2 + i, &format!("post {i}")))
            .collect();
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
        let ops: Vec<SignedOp> = (0..3)
            .map(|i| a_thread(2 + i, &format!("post {i}")))
            .collect();
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
        let heads: Vec<SignedOp> = (0..4)
            .map(|i| a_thread(2 + i, &format!("post {i}")))
            .collect();
        let creator = a_key(1);
        let log_all = a_log(heads.clone());
        // Hide whichever two the convergent order puts FIRST, so that a
        // filter-after-paging implementation would return an empty first page.
        let order: Vec<String> = all_of(&log_all, false)
            .iter()
            .map(|r| r.thread.clone())
            .collect();
        let mut ops = heads.clone();
        for hex in order.iter().take(2) {
            ops.push(
                Op {
                    stoa: a_stoa(),
                    author: creator.public_key(),
                    clock: None,
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

    // ─── The reply count and the latest reply ─────────────────────────────
    //
    // Every fixture here that asserts a subtraction also asserts the log
    // WITHOUT it, so a fold that ignored moderation, forgery or membership
    // could not pass by producing the same number for a different reason.

    /// A post carrying a counter, so a fixture can say which reply the
    /// ordering rule places first rather than leaving it to op-id hashes.
    fn a_post_at(author_seed: u8, parent: Option<OpId>, counter: u64, body: &str) -> SignedOp {
        let key = a_key(author_seed);
        Op {
            stoa: a_stoa(),
            author: key.public_key(),
            clock: Some(crate::op::OpClock {
                counter,
                asserted_ms: 1_789_729_304_000,
            }),
            kind: OpKind::Post {
                thread: parent,
                parent,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(&key)
    }

    fn a_moderation_at(target: OpId, action: ModerationAction, counter: u64) -> SignedOp {
        let creator = a_key(1);
        Op {
            stoa: a_stoa(),
            author: creator.public_key(),
            clock: Some(crate::op::OpClock {
                counter,
                asserted_ms: 1_789_729_304_000,
            }),
            kind: OpKind::Moderate { target, action },
        }
        .sign(&creator)
    }

    fn row_for<'a>(rows: &'a [FeedRow], root: &SignedOp) -> &'a FeedRow {
        rows.iter()
            .find(|r| r.thread == root.op.id().to_hex())
            .expect("the thread must have a row")
    }

    #[test]
    fn a_thread_with_no_replies_reports_zero_and_no_latest() {
        let head = a_thread(2, "unanswered");
        let rows = all_of(&a_log(vec![head.clone()]), false);
        let row = row_for(&rows, &head);
        assert_eq!(row.reply_count(), 0);
        assert_eq!(row.latest_reply(), None);
        assert_eq!(row.replies, None);
    }

    #[test]
    fn replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest() {
        let root = a_post_at(2, None, 1, "root");
        let mid = a_post_at(3, Some(root.op.id()), 2, "mid");
        let deep = a_post_at(4, Some(mid.op.id()), 3, "deep");
        let rows = all_of(&a_log(vec![root.clone(), mid, deep.clone()]), false);
        let row = row_for(&rows, &root);
        assert_eq!(row.reply_count(), 2);
        assert_eq!(row.latest_reply(), Some(deep.op.id().to_hex().as_str()));
    }

    #[test]
    fn the_latest_reply_is_the_one_the_ordering_rule_places_first() {
        // Appended higher-counter FIRST and in the opposite sequence on a
        // second log, so neither insertion order nor a "last seen wins" fold
        // lands on the right answer by accident.
        let root = a_post_at(2, None, 1, "root");
        let lower = a_post_at(3, Some(root.op.id()), 2, "lower");
        let higher = a_post_at(4, Some(root.op.id()), 7, "higher");
        let forwards = a_log(vec![root.clone(), higher.clone(), lower.clone()]);
        let backwards = a_log(vec![lower, higher.clone(), root.clone()]);
        for log in [&forwards, &backwards] {
            let rows = all_of(log, false);
            assert_eq!(
                row_for(&rows, &root).latest_reply(),
                Some(higher.op.id().to_hex().as_str())
            );
        }
    }

    #[test]
    fn a_hidden_reply_is_neither_counted_nor_latest() {
        // The test issue #100 asks for by name: the case that looks right by
        // accident. The hidden reply is the one that would otherwise be latest,
        // so a fold ignoring moderation gets BOTH fields wrong.
        let root = a_post_at(2, None, 1, "root");
        let visible = a_post_at(3, Some(root.op.id()), 2, "visible");
        let to_hide = a_post_at(4, Some(root.op.id()), 3, "to hide");
        let hide = a_moderation_at(to_hide.op.id(), ModerationAction::Hide, 4);

        let unmoderated = a_log(vec![root.clone(), visible.clone(), to_hide.clone()]);
        let before = all_of(&unmoderated, false);
        assert_eq!(row_for(&before, &root).reply_count(), 2);
        assert_eq!(
            row_for(&before, &root).latest_reply(),
            Some(to_hide.op.id().to_hex().as_str()),
            "without the hide, the reply to be hidden is latest — or this fixture \
             cannot tell a field that ignores moderation from one that applies it"
        );

        let moderated = a_log(vec![root.clone(), visible.clone(), to_hide, hide]);
        let after = all_of(&moderated, false);
        assert_eq!(row_for(&after, &root).reply_count(), 1);
        assert_eq!(
            row_for(&after, &root).latest_reply(),
            Some(visible.op.id().to_hex().as_str())
        );
    }

    #[test]
    fn a_thread_whose_only_reply_is_hidden_reports_zero_and_no_latest() {
        let root = a_post_at(2, None, 1, "root");
        let only = a_post_at(3, Some(root.op.id()), 2, "only");
        let hide = a_moderation_at(only.op.id(), ModerationAction::Hide, 3);
        let rows = all_of(&a_log(vec![root.clone(), only, hide]), false);
        let row = row_for(&rows, &root);
        assert_eq!(row.reply_count(), 0);
        assert_eq!(row.latest_reply(), None);
    }

    #[test]
    fn a_reply_beneath_a_hidden_reply_is_counted() {
        let root = a_post_at(2, None, 1, "root");
        let hidden = a_post_at(3, Some(root.op.id()), 2, "hidden");
        let beneath = a_post_at(4, Some(hidden.op.id()), 3, "beneath");
        let hide = a_moderation_at(hidden.op.id(), ModerationAction::Hide, 4);
        let rows = all_of(
            &a_log(vec![root.clone(), hidden, beneath.clone(), hide]),
            false,
        );
        let row = row_for(&rows, &root);
        assert_eq!(row.reply_count(), 1);
        assert_eq!(row.latest_reply(), Some(beneath.op.id().to_hex().as_str()));
    }

    #[test]
    fn a_restored_reply_is_counted_again() {
        // Counters on both moderations, so last-write-wins is real: without
        // them the resolver's fail-closed bias keeps the hide, which is the
        // degraded case and not this one.
        let root = a_post_at(2, None, 1, "root");
        let reply = a_post_at(3, Some(root.op.id()), 2, "reply");
        let hide = a_moderation_at(reply.op.id(), ModerationAction::Hide, 3);
        let unhide = a_moderation_at(reply.op.id(), ModerationAction::Unhide, 4);

        let hidden_only = all_of(
            &a_log(vec![root.clone(), reply.clone(), hide.clone()]),
            false,
        );
        assert_eq!(row_for(&hidden_only, &root).reply_count(), 0);

        let restored = all_of(&a_log(vec![root.clone(), reply, hide, unhide]), false);
        assert_eq!(row_for(&restored, &root).reply_count(), 1);
    }

    #[test]
    fn a_non_moderators_hide_removes_nothing_from_the_count() {
        let root = a_post_at(2, None, 1, "root");
        let reply = a_post_at(3, Some(root.op.id()), 2, "reply");
        let impostor = a_key(9);
        let hide = Op {
            stoa: a_stoa(),
            author: impostor.public_key(),
            clock: None,
            kind: OpKind::Moderate {
                target: reply.op.id(),
                action: ModerationAction::Hide,
            },
        }
        .sign(&impostor);
        assert!(hide.verify(), "authentic; it is the AUTHORITY that fails");
        let rows = all_of(&a_log(vec![root.clone(), reply, hide]), false);
        assert_eq!(row_for(&rows, &root).reply_count(), 1);
    }

    #[test]
    fn a_forged_reply_is_neither_counted_nor_latest() {
        let root = a_post_at(2, None, 1, "root");
        let genuine = a_post_at(3, Some(root.op.id()), 2, "genuine");
        // Claims key 4, signed by key 9, and carries the highest counter in
        // the thread — so a fold that skipped verification would report it as
        // latest as well as counting it.
        let op = Op {
            stoa: a_stoa(),
            author: a_key(4).public_key(),
            clock: Some(crate::op::OpClock {
                counter: 50,
                asserted_ms: 1_789_729_304_000,
            }),
            kind: OpKind::Post {
                thread: Some(root.op.id()),
                parent: Some(root.op.id()),
                body: "forged".to_string(),
                attachments: vec![],
            },
        };
        let forged = SignedOp {
            signature: crate::identity::sign_op_bytes(&a_key(9), &op.canonical_bytes()),
            op,
        };
        assert!(!forged.verify(), "the fixture must be a forgery");

        let log = a_log(vec![root.clone(), genuine.clone(), forged.clone()]);
        assert!(
            log.get(&forged.op.id()).unwrap().is_some(),
            "the forgery is stored, so its absence from the count is the reader's doing"
        );
        let rows = all_of(&log, false);
        let row = row_for(&rows, &root);
        assert_eq!(row.reply_count(), 1);
        assert_eq!(row.latest_reply(), Some(genuine.op.id().to_hex().as_str()));
    }

    #[test]
    fn a_post_is_counted_by_its_parent_chain_and_never_by_its_thread_field() {
        // Both directions: counted under the thread its parent is in, and not
        // under the thread it names. A fold returning zero everywhere passes
        // the second half alone.
        let first = a_post_at(2, None, 1, "first");
        let second = a_post_at(3, None, 2, "second");
        let key = a_key(4);
        let liar = Op {
            stoa: a_stoa(),
            author: key.public_key(),
            clock: Some(crate::op::OpClock {
                counter: 9,
                asserted_ms: 1_789_729_304_000,
            }),
            kind: OpKind::Post {
                thread: Some(second.op.id()),
                parent: Some(first.op.id()),
                body: "names the second, replies into the first".to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);
        let rows = all_of(
            &a_log(vec![first.clone(), second.clone(), liar.clone()]),
            false,
        );
        assert_eq!(row_for(&rows, &first).reply_count(), 1);
        assert_eq!(
            row_for(&rows, &first).latest_reply(),
            Some(liar.op.id().to_hex().as_str())
        );
        assert_eq!(row_for(&rows, &second).reply_count(), 0);
        assert_eq!(row_for(&rows, &second).latest_reply(), None);
    }

    #[test]
    fn a_post_whose_parent_is_not_held_is_counted_once_the_parent_arrives() {
        let root = a_post_at(2, None, 1, "root");
        let mid = a_post_at(3, Some(root.op.id()), 2, "mid");
        let deep = a_post_at(4, Some(mid.op.id()), 3, "deep");
        let mut log = a_log(vec![root.clone(), deep]);
        assert_eq!(row_for(&all_of(&log, false), &root).reply_count(), 0);
        log.append(mid, Arrival::unordered()).unwrap();
        assert_eq!(row_for(&all_of(&log, false), &root).reply_count(), 2);
    }

    #[test]
    fn a_revision_does_not_move_a_reply_or_change_the_id_reported() {
        let root = a_post_at(2, None, 1, "root");
        let earlier = a_post_at(3, Some(root.op.id()), 2, "earlier");
        let later = a_post_at(4, Some(root.op.id()), 3, "later");
        let revise = |target: &SignedOp, seed: u8, counter: u64| {
            let key = a_key(seed);
            Op {
                stoa: a_stoa(),
                author: key.public_key(),
                clock: Some(crate::op::OpClock {
                    counter,
                    asserted_ms: 1_789_729_304_000,
                }),
                kind: OpKind::Revise {
                    target: target.op.id(),
                    body: "revised".to_string(),
                    attachments: vec![],
                },
            }
            .sign(&key)
        };
        // The EARLIER reply is revised at a counter above everything, so a fold
        // taking a reply's position from its current version would move it to
        // the front.
        let lifted = revise(&earlier, 3, 20);
        let rows = all_of(
            &a_log(vec![root.clone(), earlier.clone(), later.clone(), lifted]),
            false,
        );
        assert_eq!(
            row_for(&rows, &root).reply_count(),
            2,
            "a revision is not a reply"
        );
        assert_eq!(
            row_for(&rows, &root).latest_reply(),
            Some(later.op.id().to_hex().as_str())
        );

        // And the LATEST reply revised: the id reported is the post's own.
        let own = revise(&later, 4, 21);
        let rows = all_of(
            &a_log(vec![root.clone(), earlier, later.clone(), own.clone()]),
            false,
        );
        assert_eq!(
            row_for(&rows, &root).latest_reply(),
            Some(later.op.id().to_hex().as_str())
        );
        assert_ne!(
            row_for(&rows, &root).latest_reply(),
            Some(own.op.id().to_hex().as_str())
        );
    }

    #[test]
    fn the_include_hidden_flag_changes_no_rows_reply_fields() {
        let root = a_post_at(2, None, 1, "root");
        let visible = a_post_at(3, Some(root.op.id()), 2, "visible");
        let hidden = a_post_at(4, Some(root.op.id()), 3, "hidden");
        let hide = a_moderation_at(hidden.op.id(), ModerationAction::Hide, 4);
        let log = a_log(vec![root.clone(), visible.clone(), hidden, hide]);

        let excluded = all_of(&log, false);
        let included = all_of(&log, true);
        assert_eq!(
            row_for(&excluded, &root).replies,
            row_for(&included, &root).replies
        );
        assert_eq!(row_for(&included, &root).reply_count(), 1);
        assert_eq!(
            row_for(&included, &root).latest_reply(),
            Some(visible.op.id().to_hex().as_str())
        );
    }

    #[test]
    fn a_hidden_threads_row_counts_its_visible_replies() {
        let root = a_post_at(2, None, 1, "root");
        let a = a_post_at(3, Some(root.op.id()), 2, "a");
        let b = a_post_at(4, Some(root.op.id()), 3, "b");
        let hide = a_moderation_at(root.op.id(), ModerationAction::Hide, 4);
        let rows = all_of(&a_log(vec![root.clone(), a, b.clone(), hide]), true);
        let row = row_for(&rows, &root);
        assert!(row.is_hidden);
        assert_eq!(row.reply_count(), 2);
        assert_eq!(row.latest_reply(), Some(b.op.id().to_hex().as_str()));
    }

    #[test]
    fn a_new_reply_does_not_move_its_threads_row() {
        let first = a_post_at(2, None, 5, "first");
        let second = a_post_at(3, None, 4, "second");
        let before_log = a_log(vec![first.clone(), second.clone()]);
        let before: Vec<String> = all_of(&before_log, false)
            .iter()
            .map(|r| r.thread.clone())
            .collect();
        assert_eq!(
            before,
            vec![first.op.id().to_hex(), second.op.id().to_hex()]
        );

        let reply = a_post_at(4, Some(second.op.id()), 9, "a reply above both roots");
        let after_log = a_log(vec![first, second.clone(), reply]);
        let after = all_of(&after_log, false);
        assert_eq!(
            after.iter().map(|r| r.thread.clone()).collect::<Vec<_>>(),
            before,
            "the reply must not move its thread, nor appear as a row"
        );
        assert_eq!(row_for(&after, &second).reply_count(), 1);
    }

    /// A log that answers `get`/`iter`/`iter_stoa` from a table of `(id, entry)`
    /// pairs **whose ids need not be the entries' own** — the same device
    /// `thread.rs`'s `CyclicLog` uses, and for the same reason: an op id is the
    /// hash of bytes that include `parent`, so a two-op cycle needs each id
    /// computed from bytes that already carry the other, which is not mintable
    /// against SHA-256. `MemoryOpLog` keys on `op.id()`, so no fixture over a
    /// real log can produce a cycle; a store whose key disagrees with its bytes
    /// is what a corrupted or hand-edited file can hold, and the fold consumes
    /// whatever `OpLog::get`/`iter_stoa` return.
    struct WrongKeyLog {
        entries: Vec<(OpId, Entry)>,
    }

    impl WrongKeyLog {
        fn filed_at(at: OpId, op: SignedOp) -> (OpId, Entry) {
            assert!(op.verify(), "the op itself must be authentic");
            (
                at,
                Entry {
                    op,
                    arrival: Arrival::unordered(),
                },
            )
        }
    }

    impl OpLog for WrongKeyLog {
        fn append(
            &mut self,
            _op: SignedOp,
            _arrival: Arrival,
        ) -> Result<crate::log::Appended, OpLogError> {
            unreachable!("this fake is read-only")
        }
        fn get(&self, id: &OpId) -> Result<Option<Entry>, OpLogError> {
            Ok(self
                .entries
                .iter()
                .find(|(at, _)| at == id)
                .map(|(_, e)| e.clone()))
        }
        fn iter(&self) -> Result<Vec<Entry>, OpLogError> {
            Ok(self.entries.iter().map(|(_, e)| e.clone()).collect())
        }
        fn iter_stoa(&self, stoa: &Address) -> Result<Vec<Entry>, OpLogError> {
            Ok(self
                .entries
                .iter()
                .map(|(_, e)| e.clone())
                .filter(|e| e.op.op.stoa == *stoa)
                .collect())
        }
        fn iter_target(&self, _target: &OpId) -> Result<Vec<Entry>, OpLogError> {
            Ok(vec![])
        }
        fn len(&self) -> Result<usize, OpLogError> {
            Ok(self.entries.len())
        }
    }

    fn an_id(seed: u8) -> OpId {
        OpId::from_hex(&format!("{seed:02x}").repeat(32)).unwrap()
    }

    #[test]
    fn a_post_in_a_parent_cycle_is_counted_under_no_thread() {
        // A closed ring of three replies, each naming the next as parent and
        // each carrying a `thread` field naming the genuine root — so a fold
        // reading `thread` instead of walking the chain would count all three,
        // and a walk with no visited set would spin forever. `thread_of`'s own
        // termination over a cycle is pinned in `thread.rs`
        // (`a_cycle_among_parents_terminates_and_places_nothing`); this pins
        // that the feed's count and latest reply see the same "placed nowhere"
        // answer for every op in the ring, rather than looping or crediting any
        // of them to the root.
        let root = a_post_at(2, None, 1, "root");
        let ring_ids: Vec<OpId> = (0..3).map(an_id).collect();
        let ring: Vec<(OpId, Entry)> = (0..ring_ids.len())
            .map(|i| {
                let next = ring_ids[(i + 1) % ring_ids.len()];
                let key = a_key(3 + i as u8);
                let op = Op {
                    stoa: a_stoa(),
                    author: key.public_key(),
                    clock: Some(crate::op::OpClock {
                        counter: 9 + i as u64,
                        asserted_ms: 1_789_729_304_000,
                    }),
                    kind: OpKind::Post {
                        thread: Some(root.op.id()),
                        parent: Some(next),
                        body: "cyclic".to_string(),
                        attachments: vec![],
                    },
                }
                .sign(&key);
                WrongKeyLog::filed_at(ring_ids[i], op)
            })
            .collect();

        let mut entries = ring;
        entries.push(WrongKeyLog::filed_at(root.op.id(), root.clone()));
        let log = WrongKeyLog { entries };

        let rows = list_threads(&log, &moderators(), &a_stoa(), 0, MAX_PER_PAGE, false)
            .unwrap()
            .items;
        let row = row_for(&rows, &root);
        assert_eq!(
            row.reply_count(),
            0,
            "no op in a parent cycle is placed under any thread"
        );
        assert_eq!(row.latest_reply(), None);
    }

    #[test]
    fn a_reply_carrying_a_far_future_asserted_time_is_not_thereby_latest() {
        // The reply with the LOWER counter asserts a time far in the future,
        // and the reply with the higher counter asserts an earlier time. If the
        // fold read the wall-clock instead of relying on arrival order, the
        // future-dated reply would win.
        let root = a_post_at(2, None, 1, "root");
        let key = a_key(3);
        let future_but_lower = Op {
            stoa: a_stoa(),
            author: key.public_key(),
            clock: Some(crate::op::OpClock {
                counter: 2,
                asserted_ms: 9_999_999_999_000,
            }),
            kind: OpKind::Post {
                thread: Some(root.op.id()),
                parent: Some(root.op.id()),
                body: "claims the future".to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);
        let earlier_but_higher = a_post_at(4, Some(root.op.id()), 7, "earlier assertion");
        let rows = all_of(
            &a_log(vec![
                root.clone(),
                earlier_but_higher.clone(),
                future_but_lower,
            ]),
            false,
        );
        assert_eq!(
            row_for(&rows, &root).latest_reply(),
            Some(earlier_but_higher.op.id().to_hex().as_str()),
            "the higher counter wins regardless of which reply claims the later time"
        );
    }

    #[test]
    fn a_second_stoas_reply_naming_the_first_stoas_root_as_parent_is_not_counted() {
        // A validly signed post whose OWN `stoa` field is a second Stoa, naming
        // a thread root of the FIRST Stoa as its parent. `visible_replies_by_thread`
        // is folded over `iter_stoa(stoa)`'s entries, so this op is never in the
        // set it walks — the same boundary `only_this_stoas_threads_are_returned`
        // pins for heads, pinned here for a reply specifically, since a reply
        // reaches `thread_of` rather than the head filter.
        let root = a_post_at(2, None, 1, "root");
        let elsewhere = Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Somewhere else".to_string(),
        }
        .address()
        .unwrap();
        let key = a_key(5);
        let foreign_reply = Op {
            stoa: elsewhere,
            author: key.public_key(),
            clock: Some(crate::op::OpClock {
                counter: 9,
                asserted_ms: 1_789_729_304_000,
            }),
            kind: OpKind::Post {
                thread: Some(root.op.id()),
                parent: Some(root.op.id()),
                body: "from elsewhere, naming this root as parent".to_string(),
                attachments: vec![],
            },
        }
        .sign(&key);
        let rows = all_of(&a_log(vec![root.clone(), foreign_reply]), false);
        let row = row_for(&rows, &root);
        assert_eq!(row.reply_count(), 0);
        assert_eq!(row.latest_reply(), None);
    }

    #[test]
    fn two_peers_holding_different_copies_report_different_counts_without_error() {
        // One peer holds all three of a thread's replies; another holds only
        // two of them — the ordinary partial-set case (§3.3), not an attack or
        // a defect. Both reads must succeed, and each reports what it holds.
        let root = a_post_at(2, None, 1, "root");
        let a = a_post_at(3, Some(root.op.id()), 2, "a");
        let b = a_post_at(4, Some(root.op.id()), 3, "b");
        let c = a_post_at(5, Some(root.op.id()), 4, "c");

        let full_peer = a_log(vec![root.clone(), a.clone(), b.clone(), c.clone()]);
        let partial_peer = a_log(vec![root.clone(), a, b]);

        let full = list_threads(&full_peer, &moderators(), &a_stoa(), 0, MAX_PER_PAGE, false)
            .expect("the first peer's read is not an error");
        let partial = list_threads(
            &partial_peer,
            &moderators(),
            &a_stoa(),
            0,
            MAX_PER_PAGE,
            false,
        )
        .expect("the second peer's read is not an error");

        assert_eq!(row_for(&full.items, &root).reply_count(), 3);
        assert_eq!(row_for(&partial.items, &root).reply_count(), 2);
    }

    #[test]
    fn the_count_agrees_with_the_thread_read() {
        // One fixture carrying every case that subtracts, read both ways.
        let root = a_post_at(2, None, 1, "root");
        let visible = a_post_at(3, Some(root.op.id()), 2, "visible");
        let hidden = a_post_at(4, Some(root.op.id()), 3, "hidden");
        let beneath = a_post_at(5, Some(hidden.op.id()), 4, "beneath the hidden");
        let hide = a_moderation_at(hidden.op.id(), ModerationAction::Hide, 5);
        // Names the thread in its `thread` field and has no parent in it.
        let claimant = Op {
            stoa: a_stoa(),
            author: a_key(6).public_key(),
            clock: Some(crate::op::OpClock {
                counter: 6,
                asserted_ms: 1_789_729_304_000,
            }),
            kind: OpKind::Post {
                thread: Some(root.op.id()),
                parent: None,
                body: "claims the thread and has no parent in it".to_string(),
                attachments: vec![],
            },
        }
        .sign(&a_key(6));
        let op = Op {
            stoa: a_stoa(),
            author: a_key(7).public_key(),
            clock: None,
            kind: OpKind::Post {
                thread: Some(root.op.id()),
                parent: Some(root.op.id()),
                body: "forged".to_string(),
                attachments: vec![],
            },
        };
        let forged = SignedOp {
            signature: crate::identity::sign_op_bytes(&a_key(9), &op.canonical_bytes()),
            op,
        };
        let log = a_log(vec![
            root.clone(),
            visible,
            hidden,
            beneath,
            hide,
            claimant,
            forged,
        ]);

        let thread = crate::thread::read_thread(
            &log,
            &moderators(),
            &a_stoa(),
            &root.op.id(),
            crate::thread::ReadOptions {
                page: 0,
                per_page: crate::thread::MAX_PER_PAGE,
                include_hidden: false,
                now_ms: 1_789_729_304_000,
            },
        )
        .unwrap()
        .unwrap();
        assert!(!thread.has_more, "the whole thread must fit one page here");
        let reply_ids: Vec<String> = thread
            .items
            .iter()
            .filter(|i| i.id != root.op.id().to_hex())
            .map(|i| i.id.clone())
            .collect();

        let rows = all_of(&log, false);
        let row = row_for(&rows, &root);
        assert_eq!(row.reply_count(), reply_ids.len());
        assert_eq!(row.reply_count(), 2, "visible and beneath-the-hidden");
        assert!(reply_ids.contains(&row.latest_reply().unwrap().to_string()));
    }

    /// A log that fails every read keyed by one op id, and answers the rest.
    ///
    /// Keyed rather than total, so a failure can be placed on ONE reply: a log
    /// failing everything fails at `iter_stoa` and never reaches the fold.
    struct FailingOn {
        log: MemoryOpLog,
        id: OpId,
    }

    impl OpLog for FailingOn {
        fn append(
            &mut self,
            op: SignedOp,
            arrival: Arrival,
        ) -> Result<crate::log::Appended, OpLogError> {
            self.log.append(op, arrival)
        }
        fn get(&self, id: &OpId) -> Result<Option<Entry>, OpLogError> {
            if *id == self.id {
                return Err(OpLogError::Storage("this reply's row is unreadable".into()));
            }
            self.log.get(id)
        }
        fn iter(&self) -> Result<Vec<Entry>, OpLogError> {
            self.log.iter()
        }
        fn iter_stoa(&self, stoa: &Address) -> Result<Vec<Entry>, OpLogError> {
            self.log.iter_stoa(stoa)
        }
        fn iter_target(&self, target: &OpId) -> Result<Vec<Entry>, OpLogError> {
            if *target == self.id {
                return Err(OpLogError::Storage("this reply's row is unreadable".into()));
            }
            self.log.iter_target(target)
        }
        fn len(&self) -> Result<usize, OpLogError> {
            self.log.len()
        }
    }

    #[test]
    fn a_store_failure_while_counting_is_an_error_and_not_a_zero() {
        let root = a_post_at(2, None, 1, "root");
        let reply = a_post_at(3, Some(root.op.id()), 2, "reply");

        // The same ops with no failure: the row is there. So the error below is
        // the one met while counting, not one met finding the head.
        let healthy = FailingOn {
            log: a_log(vec![root.clone(), reply.clone()]),
            id: OpId::from_hex(&"ab".repeat(32)).unwrap(),
        };
        let page = list_threads(&healthy, &moderators(), &a_stoa(), 0, 20, false).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].reply_count(), 1);

        let failing = FailingOn {
            log: a_log(vec![root, reply.clone()]),
            id: reply.op.id(),
        };
        let err = list_threads(&failing, &moderators(), &a_stoa(), 0, 20, false)
            .expect_err("a failure while counting must not become a count of zero");
        assert!(
            err.to_string().contains("this reply's row is unreadable"),
            "got {err}"
        );
    }

    // NO SPEC: `feed-read` requires a store failure met while computing a row's
    // reply fields to fail the read, and says nothing about a failure met on a
    // reply whose thread is NOT on the requested page. The fold runs over the
    // whole Stoa, so this fails the page too; `design.md` records why.
    #[test]
    fn a_store_failure_on_a_reply_off_the_page_fails_the_page() {
        let on_page = a_post_at(2, None, 9, "first row");
        let off_page = a_post_at(3, None, 8, "second row");
        let reply = a_post_at(4, Some(off_page.op.id()), 10, "reply to the second");
        let failing = FailingOn {
            log: a_log(vec![on_page.clone(), off_page, reply.clone()]),
            id: reply.op.id(),
        };
        let err = list_threads(&failing, &moderators(), &a_stoa(), 0, 1, false)
            .expect_err("the page holding only the first row still fails");
        assert!(err.to_string().contains("unreadable"), "got {err}");
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
