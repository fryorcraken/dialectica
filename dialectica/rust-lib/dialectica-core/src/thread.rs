//! One thread: a root post and the replies whose parent chains reach it.
//!
//! # The `thread` field inside a post is its author's claim, and this module is
//! where it stops being believed
//!
//! `authoring.rs` derives a reply's thread from its parent **on the publish
//! path** and says in as many words what it is leaving open:
//!
//! > an inbound op can name a thread its own parent does not belong to, and the
//! > log stores it, because the log decides nothing. This propagates the claim
//! > rather than auditing it, and that is the correct division: auditing a
//! > thread graph is a read-side question about which ops render where.
//!
//! **This is the read side, and this is the audit.** Nothing else performs it.
//!
//! The attack it closes needs no forgery at all, which is what makes it easy to
//! miss: a peer publishes an authentically signed post — its own key, its own
//! bytes, verifying perfectly — whose `thread` field names somebody else's
//! thread and whose `parent` names nothing in it. A reader placing posts by the
//! claimed field renders it inside that conversation, signed, verified, and with
//! nothing to mark it as out of place. `a_forged_thread_claim_cannot_inject_a_post_into_a_thread`
//! is that attack; it was written against a claim-trusting implementation and
//! watched fail before the walk below existed.
//!
//! So membership is **derived** here: follow `parent`, then its parent, until a
//! post with no parent is reached. That op id is the thread. The `thread` field
//! is never read by this module — [`thread_of`] does not mention it, which is
//! the strongest form of "never trusted" available.
//!
//! # The walk must terminate over ops chosen by an attacker
//!
//! Parent references arrive from peers, so a chain that could be made not to
//! terminate would turn one malformed op into a denial of service against the
//! peer that received it — and PHASE0-FINDINGS §3 measured that a module that
//! stops answering is a module process that has to be restarted.
//!
//! A post naming itself as its parent, and a cycle of any length, both reach an
//! op the walk has already visited. That is the terminating condition, and it is
//! a **visited set rather than a depth limit** because a limit would make a
//! legitimately deep thread stop being readable at a line nobody could derive.
//! The set is per walk: it answers "has this chain been here before", not "has
//! any chain".
//!
//! # A post whose chain cannot be completed is returned under NO thread
//!
//! Not dropped from the log, not guessed at, not placed by its claim. A peer
//! routinely holds a partial set (§3.3), so a reply whose parent has not arrived
//! is the ordinary case rather than a fault — and it becomes placeable the
//! moment the parent does arrive. Placing it by the claim is the one answer this
//! module exists to refuse.
//!
//! # No comparison is written here
//!
//! [`OpLog::iter_stoa`] already returns entries in
//! [`cmp_ops`](crate::arrival::cmp_ops) order, and this module filters and pages
//! that sequence. There is no `sort`, no `cmp` and no `max_by` — the same
//! discipline [`crate::revision`], [`crate::moderation`] and [`crate::feed`]
//! hold, and for the same reason: a second implementation of the ordering rule
//! could disagree with the first, and two orders that disagree produce no error
//! anywhere.
//!
//! **What that order guarantees is convergence, not recency.** Nothing supplies
//! a Lamport value today, so `cmp_ops` falls back to ascending op id — a hash,
//! carrying no temporal meaning. Two peers holding the same ops return the same
//! sequence; neither can say which reply was written first. Nothing in this
//! module should ever be reported to a caller as chronological.
//!
//! # The items are flat, and each names its parent
//!
//! A view that nests computes the depth from the parent chain it already holds.
//! Reporting a depth from here would be a second answer to a question the parent
//! field already settles, and the two could disagree on a partial set of ops.

use crate::identity::Address;
use crate::log::{Entry, OpLog, OpLogError};
use crate::moderation::{Moderation, Moderators};
use crate::op::{OpId, OpKind};
use crate::revision::current_version;
use crate::sanitise::{sanitise, Sanitised};
use std::collections::HashSet;

/// The largest page a caller may ask for.
///
/// Same value and same reasoning as [`crate::feed::MAX_PER_PAGE`]: `perPage`
/// arrives from the wire, every cross-module call is IPC (§2.4), and the reply is
/// serialised into one JSON string — so an unbounded page size is a caller asking
/// this peer to build an arbitrarily large string in memory.
///
/// **Its own constant rather than a re-export**, because the two caps answer
/// different questions: a feed row carries one post's body, and a thread item
/// carries one post's body *and* its parent, so a future decision to serve fewer
/// thread items than feed rows should not have to disentangle one number used
/// twice. They agree today, and `the_page_caps_agree_until_someone_decides_otherwise`
/// records that the agreement is observed rather than enforced.
pub const MAX_PER_PAGE: usize = 100;

/// The default page size when a caller names none.
pub const DEFAULT_PER_PAGE: usize = 20;

/// One post of a thread, resolved and sanitised, ready to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadItem {
    /// The thread this post belongs to: the op id of the root the parent chain
    /// reaches.
    ///
    /// **Derived, never read from the op.** See this module's documentation.
    /// Equal on every item of one page, including the root's, which is what makes
    /// it the value a caller passes back to read the next page.
    pub thread: String,
    /// This post's own op id — the one a reply names as its parent, and the one
    /// a moderation names as its target.
    ///
    /// Stable across revisions: a revision is a distinct op with its own id, and
    /// this is the post's.
    pub id: String,
    /// The op id of the version being rendered.
    ///
    /// Different from [`id`](Self::id) the moment the post has been edited. Two
    /// fields because they are two facts: a caller collapsing them would have to
    /// re-derive one.
    pub current_version: String,
    /// The post this replies to, or `None` for the root.
    ///
    /// **An op id and not a promise that the parent is among the items.** A
    /// parent may fall on an earlier page, or be hidden while the caller did not
    /// ask for hidden content. A caller must be able to render an item whose
    /// parent it does not hold in hand.
    pub parent: Option<String>,
    /// The author's per-Stoa address (§5.2), hex.
    ///
    /// **The unforgeable half of the pair.** A generated name and a generated
    /// mark are both cheaply re-rolled to resemble someone else's; only the
    /// address settles who published something.
    pub author: String,
    /// The public key that signed this op, hex.
    ///
    /// Carried **beside** the address and never instead of it, because the two
    /// are independent digests and a reader needs both: the generated display
    /// name is a function of the **key**, the mark is a function of the
    /// **address**, and an address is a one-way hash of a record rather than a
    /// transformation of the key. An item carrying only an address is one whose
    /// name nothing downstream can compute.
    ///
    /// **No name is sent from here.** A name is a pure function of this field, so
    /// sending one would put a second, derivable identifier on the wire beside the
    /// material it is derived from — where the two could disagree and a reader
    /// would have no way to tell which was wrong.
    pub author_key: String,
    /// The current version's body, sanitised — or `None` where it is **withheld**
    /// because this is a hidden root and the caller did not ask for hidden
    /// content.
    ///
    /// **An `Option` rather than an empty string with a flag beside it**, because
    /// a revision that cleared a body is a legitimate edit and reports
    /// `Some("")`. Withheld and empty are different facts, and this makes them
    /// different values rather than one value a caller has to disambiguate
    /// correctly at every call site.
    pub body: Option<Sanitised>,
    /// Logos Storage CIDs (§4.6), sanitised — or `None`, withheld by the same
    /// rule and for the same reason as [`body`](Self::body).
    pub attachments: Option<Vec<Sanitised>>,
    /// Whether the post has been revised (§5.7).
    ///
    /// Decided by **which op the current version is**, never by comparing
    /// content: an author may revise a post to text identical to the original,
    /// and a post never revised is not distinguishable from one revised back.
    pub is_revised: bool,
    /// What the ops this peer holds say about this post's moderation.
    ///
    /// **The resolver's own three-valued answer, carried whole.** Not a `bool`:
    /// a post nobody moderated and a post a moderator deliberately restored read
    /// identically under a flag and mean different things. Not a `bool` plus an
    /// id either — that would be a second representation of a decision
    /// [`crate::moderation`] already made, and the two could disagree.
    pub moderation: Moderation,
}

impl ThreadItem {
    /// Whether a moderation hid this post.
    ///
    /// A convenience over [`moderation`](Self::moderation) so a caller asking the
    /// common question does not have to spell the match — and deliberately not a
    /// stored field, because a stored one could disagree with the state it was
    /// derived from.
    pub fn is_hidden(&self) -> bool {
        self.moderation.is_hidden()
    }
}

/// One page of a thread.
///
/// The ecosystem's pagination shape — `{"items":[...],"page":N,"hasMore":bool}`
/// — with no total, for the reason [`crate::feed::FeedPage`] gives: a count of
/// what this peer holds is not a count of what exists, and the two are
/// indistinguishable once rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadPage {
    pub items: Vec<ThreadItem>,
    pub page: usize,
    /// Whether a further page exists **in this peer's copy**.
    pub has_more: bool,
}

/// Why a thread could not be read.
///
/// **Three refusals rather than one**, because three different mistakes call for
/// three different responses: wait for the op to propagate, correct a category
/// error, or read the thread this reply actually belongs to. A reader told only
/// "no such thread" for all three is sent looking in the wrong place twice out of
/// three times.
///
/// A store failure is deliberately **not** here: it is an [`OpLogError`] and
/// reaches the caller as one, because "the store could not be consulted" is not
/// an opinion about the thread. Collapsing the two would report a broken disk as
/// a thread that does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotAThread {
    /// No op in the log carries that id. The ordinary case on a partial set.
    NotHeld(OpId),
    /// The log holds that op and it is not a post — a vote, a moderation, a
    /// revision, a metadata op.
    NotAPost(OpId),
    /// The log holds that post and it has a parent, so it is a reply rather than
    /// a root. The thread it belongs to is a different id.
    IsAReply(OpId),
}

impl std::fmt::Display for NotAThread {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotAThread::NotHeld(id) => write!(
                f,
                "this peer holds no op under {}, so there is no thread to read; \
                 it may not have arrived yet",
                id.to_hex()
            ),
            NotAThread::NotAPost(id) => write!(
                f,
                "the op {} is not a post, so it opens no thread",
                id.to_hex()
            ),
            NotAThread::IsAReply(id) => write!(
                f,
                "the post {} is a reply rather than a thread's root; read the \
                 thread its parent chain reaches instead",
                id.to_hex()
            ),
        }
    }
}

/// Clamp a caller-supplied page size into something this peer will build.
///
/// Separate from [`read_thread`] because it is a guard and CLAUDE.md keeps a
/// guard as its own job. Zero clamps **up** to the default rather than down to an
/// empty page: a permanently empty thread is precisely the confusion the
/// held/not-held distinction exists to prevent.
pub fn clamp_per_page(requested: Option<usize>) -> usize {
    match requested {
        None | Some(0) => DEFAULT_PER_PAGE,
        Some(n) => n.min(MAX_PER_PAGE),
    }
}

/// The thread `id` belongs to, derived by following its parent chain.
///
/// **THE security property of this capability.** The op's own `thread` field is
/// not read, not compared and not mentioned — see this module's documentation
/// for the attack that makes reading it unsafe.
///
/// # What each outcome means
///
/// `Ok(Some(root))` — the chain reached a post with no parent, and that post's
/// op id is the thread. A root post's chain is one link long and reaches itself.
///
/// `Ok(None)` — the chain could not be completed, so this peer cannot establish
/// where the post belongs and **places it under no thread**. Four ways to get
/// here, and they are one outcome deliberately: the op is not held, the op is not
/// a post, the op does not verify, or the chain revisits an op it has already
/// been to. A caller can act on "I cannot place this" and has nothing to do
/// differently for the four.
///
/// `Err` — the store could not be consulted, which is not an opinion about the
/// post at all.
///
/// # Verification runs before a parent is followed, at every link
///
/// An op's `parent` is a field like any other: a claim until the signature is
/// checked. A walk verifying only its starting point would let a forged op with a
/// chosen parent act as a bridge between two genuine ones — placing a real post
/// in a thread its author never addressed, which is the same injection the
/// `thread` field would have allowed by a shorter route.
pub fn thread_of<L: OpLog>(log: &L, id: &OpId) -> Result<Option<OpId>, OpLogError> {
    // Per walk, not shared between walks. It answers "has THIS chain been here
    // before", which is the terminating condition; a shared set would answer a
    // different question and would wrongly abandon a second post whose chain
    // legitimately passes through the same ancestors.
    let mut visited: HashSet<OpId> = HashSet::new();
    let mut current = *id;

    loop {
        // A post naming itself, and a cycle of any length, both arrive here. This
        // is what makes the walk terminate over ops an attacker chose, and it is
        // bounded by the number of ops the log holds rather than by a depth
        // nobody could derive.
        if !visited.insert(current) {
            return Ok(None);
        }

        let Some(entry) = log.get(&current)? else {
            // A parent this peer does not hold. The ordinary partial-set case
            // (§3.3), and the post becomes placeable when the op arrives.
            return Ok(None);
        };

        // BEFORE any field of the op is read. Every field is a claim until the
        // signature is checked, and `parent` is the field this walk is about to
        // believe.
        if !entry.op.verify() {
            return Ok(None);
        }

        let OpKind::Post { parent, .. } = &entry.op.op.kind else {
            // A chain reaching a vote, a moderation, a revision or a metadata op
            // has reached something that is not part of any thread.
            return Ok(None);
        };

        match parent {
            // No parent: this is a root, and its id is the thread's.
            None => return Ok(Some(current)),
            Some(next) => current = *next,
        }
    }
}

/// One page of a thread: its root, then the replies whose chains reach it.
///
/// # What is filtered, and in what order
///
/// 1. The named op is resolved and refused three ways if it is not a root.
/// 2. Ops in this Stoa, already in [`cmp_ops`](crate::arrival::cmp_ops) order.
/// 3. Keep the ops that are **authentic** — the log holds junk deliberately
///    (§3.3) and the reader never trusts it.
/// 4. Keep the `Post`s whose **parent chain** reaches this root. Never the ones
///    whose `thread` field claims it.
/// 5. Resolve each one's current version and moderation state.
/// 6. Drop the hidden replies unless `include_hidden`. The hidden **root** is
///    kept and its body withheld.
/// 7. Page what is left, root first.
///
/// **Hiding is applied before paging**, which is the only order that gives stable
/// pages: filtering after slicing produces short pages with gaps, and a reader
/// paging forward silently skips items.
///
/// # A hidden root is returned and marked, never dropped
///
/// A feed omits a hidden thread from a list of many and the reader still has a
/// feed. A thread read that omitted its own subject would answer the caller with
/// nothing — and nothing is exactly what a thread this peer does not hold also
/// produces, so the two states would be indistinguishable. Returning the thread
/// marked as hidden reports the moderation as a moderation rather than as an
/// absence a reader would read as a lost post.
///
/// Its body and attachments are **withheld** — `None`, not empty — unless the
/// caller asked for hidden content. Withheld and cleared-by-the-author are
/// different facts and this keeps them different values.
///
/// # A read failure is never an empty thread
///
/// Every store failure comes back as `Err(OpLogError)` and none is flattened
/// into an empty page, for §11.1 obligation 5's reason: an empty listing is
/// indistinguishable from a subject nobody posted in, so swallowing this would
/// render a peer whose store is broken as a thread that is merely quiet.
pub fn read_thread<L: OpLog>(
    log: &L,
    moderators: &Moderators,
    stoa: &Address,
    root: &OpId,
    page: usize,
    per_page: usize,
    include_hidden: bool,
) -> Result<Result<ThreadPage, NotAThread>, OpLogError> {
    let Some(root_entry) = log.get(root)? else {
        return Ok(Err(NotAThread::NotHeld(*root)));
    };

    // Authenticity before the op's fields decide anything. A forged post is not
    // a thread this peer holds — the spec puts the refusal for one at the
    // not-held refusal, because an op that does not verify is not evidence that
    // anything was published.
    if !root_entry.op.verify() {
        return Ok(Err(NotAThread::NotHeld(*root)));
    }

    // The Stoa is checked here rather than left to the iteration below: a root in
    // another Stoa would otherwise produce an empty page, which is the answer
    // reserved for a thread with no replies.
    if &root_entry.op.op.stoa != stoa {
        return Ok(Err(NotAThread::NotHeld(*root)));
    }

    match &root_entry.op.op.kind {
        OpKind::Post { parent: None, .. } => {}
        // A post with a parent is a reply, and the thread it belongs to has a
        // different id. Distinguishable from the two refusals around it because
        // the response differs: this caller is one level too deep.
        OpKind::Post {
            parent: Some(_), ..
        } => return Ok(Err(NotAThread::IsAReply(*root))),
        _ => return Ok(Err(NotAThread::NotAPost(*root))),
    }

    let mut items = Vec::new();

    for entry in log.iter_stoa(stoa)? {
        // Authenticity first, before any field of the op is read — including the
        // `parent` the chain walk is about to follow.
        if !entry.op.verify() {
            continue;
        }
        if !matches!(entry.op.op.kind, OpKind::Post { .. }) {
            continue;
        }

        let id = entry.id();

        // THE membership rule. Not `entry.op.op.kind`'s `thread` field, which is
        // the author's claim; the chain this peer can actually verify.
        if thread_of(log, &id)? != Some(*root) {
            continue;
        }

        let is_root = id == *root;

        let Some(item) = resolve_item(log, moderators, &entry, root, is_root, include_hidden)?
        else {
            continue;
        };
        if is_root {
            // The root leads the sequence, wherever the log's order put it. This
            // is the one position this module fixes, and it is what makes the
            // root identifiable within a flat page without the caller comparing
            // op ids.
            items.insert(0, item);
        } else {
            items.push(item);
        }
    }

    // Page by slicing what survived. The `min` calls are what make a page past
    // the end an empty page rather than a panic on the slice, and
    // `saturating_mul` is what makes a caller-supplied `page` of `usize::MAX` an
    // empty page rather than a wrapped offset into the middle of the thread.
    let start = page.saturating_mul(per_page).min(items.len());
    let end = start.saturating_add(per_page).min(items.len());
    let has_more = end < items.len();

    Ok(Ok(ThreadPage {
        items: items[start..end].to_vec(),
        page,
        has_more,
    }))
}

/// One placed post, resolved into the item a caller renders — or `None` where it
/// is a hidden reply the caller did not ask for.
///
/// Separate from [`read_thread`] because resolving one post is a different job
/// from assembling a page, and because the hidden-root/hidden-reply asymmetry is
/// the piece most worth being able to read in one place.
fn resolve_item<L: OpLog>(
    log: &L,
    moderators: &Moderators,
    entry: &Entry,
    root: &OpId,
    is_root: bool,
    include_hidden: bool,
) -> Result<Option<ThreadItem>, OpLogError> {
    let id = entry.id();

    // `Ok(None)` cannot happen for an entry this reached — it is a `Post` the log
    // just handed us — but it is an answer rather than a panic, because a panic
    // aborts the module process (PHASE0-FINDINGS §3).
    let Some(version) = current_version(log, &id)? else {
        return Ok(None);
    };

    // Resolved on READ, every time, by this peer (§6.2). Never cached and never
    // decided at append time.
    let moderation = crate::moderation::resolve(log, moderators, &id)?;
    let hidden = moderation.is_hidden();

    // The asymmetry, in one expression. A hidden REPLY is omitted — moderation is
    // a filter rather than a penalty, and a visible placeholder is a penalty with
    // extra steps. A hidden ROOT is kept, because dropping it would make a
    // moderated thread indistinguishable from one this peer never received.
    if hidden && !is_root && !include_hidden {
        return Ok(None);
    }
    // Withheld only for the hidden root the caller did not ask to see. A hidden
    // reply that reached here was asked for, so it carries its content like any
    // other item.
    let withhold = hidden && !include_hidden;

    let parent = match &entry.op.op.kind {
        OpKind::Post { parent, .. } => parent.map(|p| p.to_hex()),
        // Unreachable: the caller filtered to posts. Answered rather than
        // panicked, and answered as "no parent" rather than guessed, because this
        // module runs on attacker-supplied content.
        _ => None,
    };

    Ok(Some(ThreadItem {
        thread: root.to_hex(),
        id: id.to_hex(),
        current_version: version.current.id().to_hex(),
        parent,
        // Both from the op verification has already bound to the key that signed.
        author: entry.op.op.author.address().to_hex(),
        author_key: hex::encode(entry.op.op.author.to_bytes()),
        body: if withhold {
            None
        } else {
            Some(sanitise(version.body()))
        },
        attachments: if withhold {
            None
        } else {
            Some(version.attachments().iter().map(|a| sanitise(a)).collect())
        },
        is_revised: version.is_revised(),
        moderation,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrival::{Arrival, MessageId};
    use crate::identity::{sign_op_bytes, PublicKey, SecretKey};
    use crate::log::MemoryOpLog;
    use crate::op::{ModerationAction, Op, SignedOp, VoteDirection};
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

    fn a_message_id(seed: u8) -> MessageId {
        MessageId::new(vec![seed; 32])
    }

    /// A thread-opening post: no parent, and therefore no thread field either.
    fn a_root(author_seed: u8, body: &str) -> SignedOp {
        a_post_in(a_stoa(), author_seed, None, None, body)
    }

    /// A reply whose `thread` field is the HONEST one: derived from its parent
    /// exactly as `authoring::reply` derives it.
    fn a_reply(author_seed: u8, parent: &SignedOp, body: &str) -> SignedOp {
        // The honest derivation: the parent's own thread when it has one, the
        // parent's id when it does not. Spelled here rather than called from
        // `authoring`, because these fixtures must be able to LIE about it and a
        // fixture that could only tell the truth cannot build the attack.
        let thread = match &parent.op.kind {
            OpKind::Post { thread, .. } => thread.unwrap_or(parent.op.id()),
            _ => parent.op.id(),
        };
        a_post_in(
            a_stoa(),
            author_seed,
            Some(thread),
            Some(parent.op.id()),
            body,
        )
    }

    /// A post with every field chosen, so a fixture can name a thread its parent
    /// does not belong to.
    fn a_post_in(
        stoa: Address,
        author_seed: u8,
        thread: Option<OpId>,
        parent: Option<OpId>,
        body: &str,
    ) -> SignedOp {
        let key = a_key(author_seed);
        Op {
            stoa,
            author: key.public_key(),
            kind: OpKind::Post {
                thread,
                parent,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(&key)
    }

    /// A post CLAIMING `claimed` as its author but signed by `signer`.
    ///
    /// Asserted to be a genuine forgery at construction, or a test using it
    /// proves nothing.
    fn a_forged_post(
        claimed: &PublicKey,
        signer: &SecretKey,
        thread: Option<OpId>,
        parent: Option<OpId>,
        body: &str,
    ) -> SignedOp {
        let op = Op {
            stoa: a_stoa(),
            author: claimed.clone(),
            kind: OpKind::Post {
                thread,
                parent,
                body: body.to_string(),
                attachments: vec![],
            },
        };
        let forged = SignedOp {
            signature: sign_op_bytes(signer, &op.canonical_bytes()),
            op,
        };
        assert!(!forged.verify(), "the fixture must be an actual forgery");
        forged
    }

    fn a_moderation(signer: &SecretKey, target: OpId, action: ModerationAction) -> SignedOp {
        Op {
            stoa: a_stoa(),
            author: signer.public_key(),
            kind: OpKind::Moderate { target, action },
        }
        .sign(signer)
    }

    fn a_revision(signer: &SecretKey, target: OpId, body: &str) -> SignedOp {
        Op {
            stoa: a_stoa(),
            author: signer.public_key(),
            kind: OpKind::Revise {
                target,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(signer)
    }

    fn a_vote(signer: &SecretKey, target: OpId) -> SignedOp {
        Op {
            stoa: a_stoa(),
            author: signer.public_key(),
            kind: OpKind::Vote {
                target,
                direction: VoteDirection::Up,
            },
        }
        .sign(signer)
    }

    fn a_metadata_op(signer: &SecretKey) -> SignedOp {
        Op {
            stoa: a_stoa(),
            author: signer.public_key(),
            kind: OpKind::StoaMetadata {
                title: "Agora, renamed".to_string(),
                description: "what the Stoa is called today".to_string(),
            },
        }
        .sign(signer)
    }

    /// Every op kind that is not a `Post`, each genuinely signed and in this
    /// Stoa, named so a failure says which kind broke.
    ///
    /// **Enumerated here so the enumeration is in one place**, which is the only
    /// honest way to test a rule the spec states over *kinds*: a kind added to
    /// `OpKind` and not added here is a gap, and the exhaustive match in
    /// [`every_op_kind_is_either_a_post_or_in_the_non_post_table`] is what makes
    /// that gap a compile error rather than a silently narrower sweep.
    fn every_non_post_kind(target: OpId) -> Vec<(&'static str, SignedOp)> {
        vec![
            ("a vote", a_vote(&a_key(4), target)),
            (
                "a moderation",
                a_moderation(&a_key(1), target, ModerationAction::Hide),
            ),
            ("a Stoa metadata op", a_metadata_op(&a_key(1))),
            ("a revision", a_revision(&a_key(2), target, "v2")),
        ]
    }

    fn a_log(ops: Vec<SignedOp>) -> MemoryOpLog {
        let mut log = MemoryOpLog::new();
        for op in ops {
            log.append(op, Arrival::unordered()).unwrap();
        }
        log
    }

    /// The whole thread, unpaged, for tests about content rather than paging.
    fn read(log: &MemoryOpLog, root: &SignedOp, include_hidden: bool) -> ThreadPage {
        read_thread(
            log,
            &moderators(),
            &a_stoa(),
            &root.op.id(),
            0,
            MAX_PER_PAGE,
            include_hidden,
        )
        .expect("the store must answer")
        .expect("the root must be readable")
    }

    fn ids_of(page: &ThreadPage) -> Vec<String> {
        page.items.iter().map(|i| i.id.clone()).collect()
    }

    // ─── THE security property: membership is derived, never claimed ──────

    #[test]
    fn a_forged_thread_claim_cannot_inject_a_post_into_a_thread() {
        // THE attack this capability exists to stop, and the test that was
        // written and watched fail against a claim-trusting implementation
        // before the chain walk existed.
        //
        // Nothing here is forged in the cryptographic sense. The intruder signs
        // its own op with its own key, so `verify()` passes — which is exactly
        // what makes this dangerous: every check the rest of the system makes
        // says yes. The op simply names somebody else's thread and has no parent
        // in it.
        //
        // A reader placing posts by the claimed `thread` field renders it inside
        // that conversation. `authoring.rs` deferred this audit to the read side
        // in as many words; this is the read side.
        let victim_root = a_root(2, "the thread under attack");
        let intruder = a_post_in(
            a_stoa(),
            9,
            // The claim: "I am in the victim's thread."
            Some(victim_root.op.id()),
            // The truth: no parent at all, so the chain reaches ITSELF as a root.
            None,
            "injected",
        );
        assert!(
            intruder.verify(),
            "the intruder's op must be AUTHENTIC — the attack needs no forgery, \
             which is what makes trusting the claim unsafe"
        );

        let log = a_log(vec![victim_root.clone(), intruder.clone()]);
        let page = read(&log, &victim_root, false);

        assert_eq!(
            ids_of(&page),
            vec![victim_root.op.id().to_hex()],
            "a post claiming a thread it has no parent in was placed in it"
        );
        // And the intruder IS in the log, so this passes because the read
        // refused it rather than because the fixture failed to store it.
        assert!(log.get(&intruder.op.id()).unwrap().is_some());
        // It is its own root, which is where the parent chain genuinely puts it.
        assert_eq!(
            thread_of(&log, &intruder.op.id()).unwrap(),
            Some(intruder.op.id())
        );
    }

    #[test]
    fn a_post_claiming_one_thread_is_placed_by_its_parent_in_another() {
        // The sharper half of the same rule: the post is a genuine reply to a
        // genuine post, and its `thread` field names a DIFFERENT thread that
        // also exists. A reader trusting the claim files it under the second; the
        // chain walk files it under the first.
        //
        // Both directions are asserted, because an implementation that returned
        // nothing anywhere would pass the negative half alone.
        let first = a_root(2, "the first thread");
        let second = a_root(3, "the second thread");
        let liar = a_post_in(
            a_stoa(),
            4,
            // Claims the second thread…
            Some(second.op.id()),
            // …while replying into the first.
            Some(first.op.id()),
            "filed under the wrong thread",
        );
        assert!(liar.verify());

        let log = a_log(vec![first.clone(), second.clone(), liar.clone()]);

        assert_eq!(
            ids_of(&read(&log, &first, false)),
            vec![first.op.id().to_hex(), liar.op.id().to_hex()],
            "the reply belongs to the thread its PARENT is in"
        );
        assert_eq!(
            ids_of(&read(&log, &second, false)),
            vec![second.op.id().to_hex()],
            "the claimed thread must not receive it"
        );
    }

    #[test]
    fn a_reply_to_a_reply_lands_in_the_root_thread_and_not_its_parents() {
        // The chain is followed all the way rather than one link. A reader that
        // stopped at the parent would file the deeper reply under a thread rooted
        // at its immediate parent — which is not a root at all.
        let root = a_root(2, "root");
        let mid = a_reply(3, &root, "mid");
        let deep = a_reply(4, &mid, "deep");
        let log = a_log(vec![root.clone(), mid.clone(), deep.clone()]);

        assert_eq!(
            thread_of(&log, &deep.op.id()).unwrap(),
            Some(root.op.id()),
            "the chain must be followed to the ROOT"
        );
        let page = read(&log, &root, false);
        assert_eq!(page.items.len(), 3);
        // And reading the mid reply's id is refused rather than served as a
        // thread of its own.
        assert_eq!(
            read_thread(
                &log,
                &moderators(),
                &a_stoa(),
                &mid.op.id(),
                0,
                MAX_PER_PAGE,
                false
            )
            .unwrap(),
            Err(NotAThread::IsAReply(mid.op.id()))
        );
    }

    #[test]
    fn a_post_whose_parent_is_absent_is_placed_under_no_thread() {
        // The partial-set case (§3.3), and the one place placing by the claim
        // would be most tempting: the claimed thread IS held, and the peer has
        // nothing else to go on. It still refuses, because "the author says so"
        // is not evidence.
        let root = a_root(2, "held");
        let absent_parent = a_root(3, "never received");
        let orphan = a_post_in(
            a_stoa(),
            4,
            Some(root.op.id()),
            Some(absent_parent.op.id()),
            "cannot be placed",
        );
        // The parent is deliberately NOT appended.
        let log = a_log(vec![root.clone(), orphan.clone()]);

        assert_eq!(thread_of(&log, &orphan.op.id()).unwrap(), None);
        let page = read(&log, &root, false);
        assert_eq!(ids_of(&page), vec![root.op.id().to_hex()]);
        // And no error was reported for the thread read — an unplaceable post is
        // an ordinary condition, not a fault.
        assert!(read_thread(
            &log,
            &moderators(),
            &a_stoa(),
            &root.op.id(),
            0,
            MAX_PER_PAGE,
            false
        )
        .unwrap()
        .is_ok());
    }

    #[test]
    fn a_post_becomes_placeable_when_its_missing_parent_arrives() {
        // Both halves in one test, because "it appears" proves nothing without
        // "it did not appear before" — an implementation that always returned it
        // would pass the second assertion alone.
        let root = a_root(2, "root");
        let mid = a_reply(3, &root, "mid");
        let deep = a_reply(4, &mid, "deep");

        let mut log = a_log(vec![root.clone(), deep.clone()]);
        assert_eq!(
            ids_of(&read(&log, &root, false)),
            vec![root.op.id().to_hex()],
            "the deep reply must not be placed while its parent is missing"
        );

        log.append(mid.clone(), Arrival::unordered()).unwrap();
        let after = read(&log, &root, false);
        assert_eq!(after.items.len(), 3);
        assert!(ids_of(&after).contains(&deep.op.id().to_hex()));
    }

    #[test]
    fn a_chain_reaching_any_op_that_is_not_a_post_places_nothing() {
        // A parent naming each non-post kind in turn. Each is a real op in the
        // log, so the walk genuinely reaches it and refuses it for its KIND
        // rather than for its absence — and the rule the spec states over kinds
        // is exercised over all of them rather than over the one that happened
        // to get written first.
        let root = a_root(2, "root");
        for (name, non_post) in every_non_post_kind(root.op.id()) {
            let child = a_post_in(
                a_stoa(),
                7,
                // Claims the genuine thread, so an implementation placing posts
                // by the claim would return it — this is not merely a chain that
                // reaches nowhere, it is one with a tempting wrong answer.
                Some(root.op.id()),
                Some(non_post.op.id()),
                "replying to something that is not a post",
            );
            let log = a_log(vec![root.clone(), non_post.clone(), child.clone()]);

            assert!(
                log.get(&non_post.op.id()).unwrap().is_some(),
                "{name} must be held, or this passes for absence rather than kind"
            );
            assert_eq!(
                thread_of(&log, &child.op.id()).unwrap(),
                None,
                "a chain reaching {name} must place nothing"
            );
            assert_eq!(
                ids_of(&read(&log, &root, false)),
                vec![root.op.id().to_hex()],
                "a post beneath {name} must not appear in the thread it claims"
            );
        }
    }

    // ─── The walk terminates over ops an attacker chose ───────────────────

    #[test]
    fn a_post_naming_itself_as_its_parent_terminates_and_is_not_a_root() {
        // The one-op cycle. A walk without a visited set spins here forever, and
        // a module that stops answering is a module process that has to be
        // restarted (PHASE0-FINDINGS §3).
        //
        // This half covers the DANGLING case, which `MemoryOpLog` can hold: a
        // parent naming an op id nothing carries. The genuinely self-naming and
        // cyclic cases need a store whose keys disagree with its contents, which
        // is `a_cycle_among_parents_terminates_and_places_nothing` below with the
        // `CyclicLog` fake — see that test for why a real log cannot mint one.
        let root = a_root(2, "root");
        let nowhere = OpId::from_hex(&"ab".repeat(32)).unwrap();
        let dangling = a_post_in(
            a_stoa(),
            3,
            Some(root.op.id()),
            Some(nowhere),
            "parent is nothing at all",
        );
        let log = a_log(vec![root.clone(), dangling.clone()]);

        assert_eq!(thread_of(&log, &dangling.op.id()).unwrap(), None);
        assert_eq!(
            ids_of(&read(&log, &root, false)),
            vec![root.op.id().to_hex()]
        );
        // Reading the dangling post's own id is refused as a reply rather than
        // served as a root, because it HAS a parent whatever that parent is.
        assert_eq!(
            read_thread(
                &log,
                &moderators(),
                &a_stoa(),
                &dangling.op.id(),
                0,
                MAX_PER_PAGE,
                false
            )
            .unwrap(),
            Err(NotAThread::IsAReply(dangling.op.id()))
        );
    }

    /// A log that answers `get` from a table of `(id, entry)` pairs **whose ids
    /// need not be the entries' own**.
    ///
    /// # Why a fake is the honest way to build a cycle
    ///
    /// An op id is the hash of bytes that include the `parent` field, so a post
    /// naming itself is a preimage naming its own hash, and a two-op cycle needs
    /// each id computed from bytes that already carry the other. **Neither is
    /// mintable against SHA-256**, and `MemoryOpLog` keys on `op.id()`, so no
    /// fixture over a real log can produce one.
    ///
    /// That is a reason to build the fixture differently, not a reason to skip
    /// the test. A peer's store is a **file**: `SqliteOpLog` reads `op_bytes`
    /// from a row keyed by an `op_id` column, and a corrupted, hand-edited or
    /// hostile file can hold a row whose key disagrees with its bytes. The walk
    /// consumes whatever `OpLog::get` returns, so this fake is exactly that store
    /// — and without the visited set the tests below do not fail, they **hang**,
    /// which is the failure mode worth pinning.
    struct CyclicLog {
        entries: Vec<(OpId, Entry)>,
    }

    impl CyclicLog {
        /// A post whose stored parent is `parent`, filed in the log under `at`.
        ///
        /// The op is genuinely signed and genuinely verifies — the lie is the
        /// store's key, not the op — so the walk reaches the `parent` field
        /// rather than stopping at the signature check.
        fn entry_at(at: OpId, parent: Option<OpId>, seed: u8) -> (OpId, Entry) {
            let op = a_post_in(a_stoa(), seed, None, parent, "cyclic");
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

    impl OpLog for CyclicLog {
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
        fn iter_stoa(&self, _stoa: &Address) -> Result<Vec<Entry>, OpLogError> {
            self.iter()
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
    fn a_post_naming_itself_as_its_parent_terminates_and_places_nothing() {
        // The one-op cycle, over a store whose key disagrees with its contents —
        // see `CyclicLog` for why that is the only way to build one and why a
        // real store can nonetheless hold it.
        //
        // Without the visited set this does not fail, it hangs.
        let me = an_id(0x11);
        let log = CyclicLog {
            entries: vec![CyclicLog::entry_at(me, Some(me), 3)],
        };

        assert_eq!(
            thread_of(&log, &me).unwrap(),
            None,
            "a post that is its own parent reaches no root"
        );
        // And the fixture really is self-naming — asserted rather than assumed,
        // or this passes for absence rather than for the cycle.
        match &log.get(&me).unwrap().unwrap().op.op.kind {
            OpKind::Post { parent, .. } => assert_eq!(*parent, Some(me)),
            other => panic!("expected a post, got {other:?}"),
        }
    }

    #[test]
    fn a_cycle_among_parents_terminates_and_places_nothing() {
        // Cycles of length two, three and four, so a walk that happened to
        // terminate for one length is caught. Each op's chain revisits an op
        // already on it and therefore reaches no root.
        for length in 2..=4u8 {
            let ids: Vec<OpId> = (0..length).map(|i| an_id(0x20 + i)).collect();
            let entries = (0..length as usize)
                .map(|i| {
                    // Each names the next, and the last names the first: a closed
                    // ring with no root anywhere in it.
                    let next = ids[(i + 1) % ids.len()];
                    CyclicLog::entry_at(ids[i], Some(next), 3 + i as u8)
                })
                .collect();
            let log = CyclicLog { entries };

            for id in &ids {
                assert_eq!(
                    thread_of(&log, id).unwrap(),
                    None,
                    "a cycle of {length} must reach no root"
                );
            }
        }
    }

    #[test]
    fn a_chain_that_runs_into_a_cycle_terminates_and_places_nothing() {
        // The shape that catches a walk guarding only its STARTING op: the post
        // being placed is not itself in the cycle — it is a perfectly ordinary
        // reply whose ancestors loop. A walk remembering only where it began
        // never revisits that, and spins in the ring forever.
        let ring_a = an_id(0x31);
        let ring_b = an_id(0x32);
        let entrant = an_id(0x33);
        let log = CyclicLog {
            entries: vec![
                CyclicLog::entry_at(ring_a, Some(ring_b), 3),
                CyclicLog::entry_at(ring_b, Some(ring_a), 4),
                CyclicLog::entry_at(entrant, Some(ring_a), 5),
            ],
        };

        assert_eq!(thread_of(&log, &entrant).unwrap(), None);
    }

    #[test]
    fn a_long_legitimate_chain_still_reaches_its_root() {
        // The positive half, without which every termination test above passes
        // for a walk that returns `None` unconditionally.
        let root = a_root(2, "root");
        let a = a_reply(3, &root, "a");
        let b = a_reply(4, &a, "b");
        let c = a_reply(5, &b, "c");
        let log = a_log(vec![root.clone(), a.clone(), b.clone(), c.clone()]);

        for op in [&root, &a, &b, &c] {
            assert_eq!(
                thread_of(&log, &op.op.id()).unwrap(),
                Some(root.op.id()),
                "every link of a legitimate chain reaches the same root"
            );
        }
        assert_eq!(read(&log, &root, false).items.len(), 4);
    }

    // ─── Only authentic posts in the named Stoa are returned ──────────────

    #[test]
    fn a_forged_reply_is_not_returned_and_the_genuine_ones_are() {
        // The log stores forgeries deliberately (§3.3). A read that did not
        // verify would render a post attributed to whoever the attacker named.
        let root = a_root(2, "root");
        let genuine = a_reply(3, &root, "genuine");
        let forged = a_forged_post(
            &a_key(3).public_key(),
            &a_key(9),
            Some(root.op.id()),
            Some(root.op.id()),
            "I did not write this",
        );
        let log = a_log(vec![root.clone(), genuine.clone(), forged.clone()]);

        let page = read(&log, &root, false);
        assert_eq!(
            ids_of(&page),
            vec![root.op.id().to_hex(), genuine.op.id().to_hex()]
        );
        // The forgery IS in the log, so this fails if verification is removed
        // rather than passing because nothing was stored.
        assert!(log.get(&forged.op.id()).unwrap().is_some());
    }

    #[test]
    fn a_forged_op_in_the_chain_cannot_place_a_genuine_post() {
        // Verification runs at EVERY link, not only on the post being placed.
        // Here the intermediate op is the forgery: a genuine post replies to it,
        // and the forgery names the root. A walk verifying only its starting
        // point would follow the forged op's `parent` and place the genuine post
        // in a thread its author never addressed.
        let root = a_root(2, "root");
        let bridge = a_forged_post(
            &a_key(3).public_key(),
            &a_key(9),
            Some(root.op.id()),
            Some(root.op.id()),
            "a forged bridge",
        );
        let genuine_child = a_post_in(
            a_stoa(),
            4,
            Some(root.op.id()),
            Some(bridge.op.id()),
            "genuine, and unplaceable",
        );
        assert!(genuine_child.verify(), "the CHILD must be genuine");

        let log = a_log(vec![root.clone(), bridge.clone(), genuine_child.clone()]);
        assert!(
            log.get(&bridge.op.id()).unwrap().is_some(),
            "the forged bridge must be in the log, or the walk stops for absence"
        );

        assert_eq!(
            thread_of(&log, &genuine_child.op.id()).unwrap(),
            None,
            "a forged op must not be able to place a genuine one"
        );
        assert_eq!(
            ids_of(&read(&log, &root, false)),
            vec![root.op.id().to_hex()]
        );
    }

    #[test]
    fn a_forged_root_is_not_readable_as_a_thread() {
        let forged = a_forged_post(&a_key(2).public_key(), &a_key(9), None, None, "not mine");
        let log = a_log(vec![forged.clone()]);
        assert_eq!(
            read_thread(
                &log,
                &moderators(),
                &a_stoa(),
                &forged.op.id(),
                0,
                MAX_PER_PAGE,
                false
            )
            .unwrap(),
            Err(NotAThread::NotHeld(forged.op.id())),
            "an op that does not verify is not a thread this peer holds"
        );
    }

    #[test]
    fn another_stoas_posts_are_never_returned() {
        let elsewhere = Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Somewhere else".to_string(),
        }
        .address()
        .unwrap();
        let root = a_root(2, "here");
        // A post in the OTHER Stoa naming this thread's root as its parent —
        // which is the shape a Stoa filter written as "chain reaches the root"
        // alone would admit.
        let foreign = a_post_in(
            elsewhere,
            3,
            Some(root.op.id()),
            Some(root.op.id()),
            "not here",
        );
        let log = a_log(vec![root.clone(), foreign.clone()]);

        assert_eq!(
            ids_of(&read(&log, &root, false)),
            vec![root.op.id().to_hex()]
        );
        assert!(log.get(&foreign.op.id()).unwrap().is_some());
    }

    #[test]
    fn a_chain_through_another_stoa_still_places_only_this_stoas_posts() {
        // NO SPEC: the spec requires that only ops in the named Stoa are
        // RETURNED, and says nothing about an op in another Stoa appearing as an
        // intermediate LINK of a chain whose two ends are in this one. This
        // accepts the bridge — the chain is followed by parent alone, and the
        // Stoa filter is applied to what is returned rather than to what is
        // walked.
        //
        // What the spec does require is unaffected and is asserted here: the
        // foreign op is not among the items, and the in-Stoa reply beneath it is
        // placed under the in-Stoa root its chain genuinely reaches. A reader
        // gets no post from the second Stoa either way.
        //
        // The alternative — refusing to cross a Stoa boundary mid-chain — is a
        // defensible reading and would drop the deeper reply instead. It is
        // reported rather than chosen silently, because the two answers differ
        // and nothing in the spec picks between them.
        let elsewhere = Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Elsewhere".to_string(),
        }
        .address()
        .unwrap();

        let root = a_root(2, "root in this Stoa");
        let bridge = a_post_in(
            elsewhere,
            3,
            Some(root.op.id()),
            Some(root.op.id()),
            "in another Stoa",
        );
        let beneath = a_post_in(
            a_stoa(),
            4,
            Some(root.op.id()),
            Some(bridge.op.id()),
            "in this Stoa, under a foreign parent",
        );
        let log = a_log(vec![root.clone(), bridge.clone(), beneath.clone()]);

        let page = read(&log, &root, false);
        let ids = ids_of(&page);
        assert!(
            !ids.contains(&bridge.op.id().to_hex()),
            "the foreign op must never be an ITEM"
        );
        assert!(
            ids.contains(&beneath.op.id().to_hex()),
            "the in-Stoa reply is placed by the chain its parent leads along"
        );
        assert_eq!(page.items.len(), 2);
    }

    #[test]
    fn a_thread_read_naming_the_wrong_stoa_is_refused() {
        let root = a_root(2, "here");
        let log = a_log(vec![root.clone()]);
        let elsewhere = Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Somewhere else".to_string(),
        }
        .address()
        .unwrap();

        assert_eq!(
            read_thread(
                &log,
                &moderators(),
                &elsewhere,
                &root.op.id(),
                0,
                MAX_PER_PAGE,
                false
            )
            .unwrap(),
            Err(NotAThread::NotHeld(root.op.id())),
            "a root in another Stoa is not held IN THIS ONE"
        );
    }

    // ─── The author is an address AND a key ───────────────────────────────

    #[test]
    fn every_item_carries_both_an_address_and_the_key_that_signed() {
        // Two independent digests: the generated name comes from the KEY and the
        // mark from the ADDRESS, and an address is a one-way hash so the key
        // cannot be recovered from it. An item carrying only an address is one
        // whose name nothing downstream can compute.
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "reply");
        let log = a_log(vec![root.clone(), reply.clone()]);
        let page = read(&log, &root, false);

        let root_item = &page.items[0];
        let reply_item = &page.items[1];

        assert_eq!(root_item.author, a_key(2).public_key().address().to_hex());
        assert_eq!(
            root_item.author_key,
            hex::encode(a_key(2).public_key().to_bytes())
        );
        assert_eq!(reply_item.author, a_key(3).public_key().address().to_hex());
        assert_eq!(
            reply_item.author_key,
            hex::encode(a_key(3).public_key().to_bytes())
        );

        // The two fields are DIFFERENT values, and the address is the one the
        // identity rules derive from that key — so a caller can check the pairing
        // rather than trust it.
        assert_ne!(root_item.author, root_item.author_key);
        assert_eq!(
            root_item.author,
            PublicKey::from_bytes(&hex::decode(&root_item.author_key).unwrap())
                .unwrap()
                .address()
                .to_hex()
        );
        // Two authors differ in BOTH fields.
        assert_ne!(root_item.author, reply_item.author);
        assert_ne!(root_item.author_key, reply_item.author_key);
    }

    // ─── The current version, and whether it was revised ──────────────────

    #[test]
    fn a_revised_reply_renders_the_revisions_body_and_is_marked_revised() {
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "the original words");
        let revision = a_revision(&a_key(3), reply.op.id(), "the replacement words");
        let log = a_log(vec![root.clone(), reply.clone(), revision.clone()]);

        let page = read(&log, &root, false);
        let item = page
            .items
            .iter()
            .find(|i| i.id == reply.op.id().to_hex())
            .expect("the reply must be returned under its own op id");
        assert_eq!(item.body.as_ref().unwrap().text, "the replacement words");
        assert!(item.is_revised);
        assert_eq!(item.current_version, revision.op.id().to_hex());
        // The post's own id does NOT move, which is what a reply names and what a
        // view keys a row on.
        assert_eq!(item.id, reply.op.id().to_hex());
        assert_ne!(item.id, item.current_version);
    }

    #[test]
    fn an_unrevised_post_reports_the_two_ids_as_equal_and_is_not_revised() {
        // The other half of the pair: without it a bug setting `is_revised`
        // unconditionally passes every revision test.
        let root = a_root(2, "never touched");
        let log = a_log(vec![root.clone()]);
        let item = &read(&log, &root, false).items[0];
        assert!(!item.is_revised);
        assert_eq!(item.id, item.current_version);
    }

    #[test]
    fn a_revision_that_clears_a_body_renders_as_empty_and_not_as_withheld() {
        // The distinction `body: Option<Sanitised>` exists to keep: an author
        // clearing their post reports `Some("")`, and only a withheld hidden root
        // reports `None`.
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "said too much");
        let cleared = a_revision(&a_key(3), reply.op.id(), "");
        let log = a_log(vec![root.clone(), reply.clone(), cleared]);

        let page = read(&log, &root, false);
        let item = page
            .items
            .iter()
            .find(|i| i.id == reply.op.id().to_hex())
            .unwrap();
        assert_eq!(
            item.body.as_ref().map(|b| b.text.as_str()),
            Some(""),
            "a cleared body is present and empty, never absent"
        );
        assert!(item.is_revised);
    }

    #[test]
    fn a_revision_by_someone_other_than_the_author_changes_nothing() {
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "mine");
        let intruder = a_revision(&a_key(9), reply.op.id(), "not yours to edit");
        assert!(intruder.verify(), "the intruder's op is AUTHENTIC");
        let log = a_log(vec![root.clone(), reply.clone(), intruder]);

        let item = read(&log, &root, false)
            .items
            .into_iter()
            .find(|i| i.id == reply.op.id().to_hex())
            .unwrap();
        assert_eq!(item.body.unwrap().text, "mine");
        assert!(!item.is_revised);
    }

    #[test]
    fn the_root_reports_no_parent_and_every_other_item_reports_one() {
        let root = a_root(2, "root");
        let mid = a_reply(3, &root, "mid");
        let deep = a_reply(4, &mid, "deep");
        let log = a_log(vec![root.clone(), mid.clone(), deep.clone()]);

        let page = read(&log, &root, false);
        assert_eq!(page.items[0].id, root.op.id().to_hex());
        assert_eq!(page.items[0].parent, None, "the root reports no parent");
        for item in page.items.iter().skip(1) {
            assert!(item.parent.is_some(), "every reply must name its parent");
        }
        // And each names the post it actually replies to, not the root.
        let deep_item = page
            .items
            .iter()
            .find(|i| i.id == deep.op.id().to_hex())
            .unwrap();
        assert_eq!(deep_item.parent, Some(mid.op.id().to_hex()));
        assert_ne!(deep_item.parent, Some(root.op.id().to_hex()));
    }

    #[test]
    fn every_item_reports_the_thread_asked_for() {
        let root = a_root(2, "root");
        let mid = a_reply(3, &root, "mid");
        let deep = a_reply(4, &mid, "deep");
        let log = a_log(vec![root.clone(), mid, deep]);
        for item in read(&log, &root, false).items {
            assert_eq!(item.thread, root.op.id().to_hex());
        }
    }

    #[test]
    fn revising_the_root_does_not_move_the_threads_identifier() {
        let root = a_root(2, "v1");
        let revision = a_revision(&a_key(2), root.op.id(), "v2");
        let reply = a_reply(3, &root, "a reply");
        let log = a_log(vec![root.clone(), revision.clone(), reply.clone()]);

        // Still readable by the ORIGINAL id.
        let page = read(&log, &root, false);
        assert_eq!(page.items[0].thread, root.op.id().to_hex());
        assert_eq!(page.items[0].current_version, revision.op.id().to_hex());
        assert_ne!(page.items[0].thread, page.items[0].current_version);
        // The reply is still placed, which it would not be if the revision had
        // displaced the root.
        assert_eq!(page.items.len(), 2);
    }

    #[test]
    fn reading_by_a_revisions_op_id_is_not_reading_the_thread() {
        // SPEC CONFLICT, resolved toward the requirement rather than the
        // scenario, and flagged because the two genuinely disagree.
        //
        // "Reading by a revision's op id is not reading the thread" says the
        // refusal "is the one for a thread this peer does not hold". The
        // refusals requirement says a read is refused "when the op it holds
        // under that id is not a post", that this refusal SHALL be
        // distinguishable from the not-held one, and that the message SHALL say
        // "the op named is not a post" — and a revision IS an op held that is not
        // a post, so the two rules name different answers for one input.
        //
        // The requirement wins over the scenario, for the reason the requirement
        // itself gives: three different mistakes call for three different
        // responses, and a caller who named a revision has made the category
        // error the not-a-post message exists to name. Answering "this peer holds
        // no op under that id" would be false — the peer holds it — and would
        // send the caller to wait for propagation of something already arrived.
        //
        // Both halves of the scenario's OBSERVABLE claim still hold: the thread
        // is not returned, and the reply is a refusal. Only which refusal
        // differs. Reported to the spec-writer.
        let root = a_root(2, "v1");
        let revision = a_revision(&a_key(2), root.op.id(), "v2");
        let log = a_log(vec![root.clone(), revision.clone()]);

        assert_eq!(
            read_thread(
                &log,
                &moderators(),
                &a_stoa(),
                &revision.op.id(),
                0,
                MAX_PER_PAGE,
                false
            )
            .unwrap(),
            Err(NotAThread::NotAPost(revision.op.id()))
        );
    }

    // ─── Moderation: three-valued, and naming its deciding op ─────────────

    #[test]
    fn an_unmoderated_post_names_no_deciding_op() {
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "reply");
        let log = a_log(vec![root.clone(), reply]);
        for item in read(&log, &root, false).items {
            assert_eq!(item.moderation, Moderation::Unmoderated);
            assert!(item.moderation.deciding_op().is_none());
            assert!(!item.is_hidden());
        }
    }

    #[test]
    fn a_hidden_post_names_the_op_that_hid_it() {
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "reply");
        let hide = a_moderation(&a_key(1), reply.op.id(), ModerationAction::Hide);
        let log = a_log(vec![root.clone(), reply.clone(), hide.clone()]);

        let item = read(&log, &root, true)
            .items
            .into_iter()
            .find(|i| i.id == reply.op.id().to_hex())
            .expect("a hidden reply must be returned when asked for");
        assert!(item.is_hidden());
        assert_eq!(
            item.moderation.deciding_op().map(|e| e.id()),
            Some(hide.op.id())
        );
    }

    #[test]
    fn a_restored_post_is_distinguishable_from_one_nobody_moderated() {
        // The distinction a boolean loses: an untouched post and a vindicated one
        // read identically under a flag and mean different things.
        //
        // The unhide must WIN, which under the degraded order it does not do by
        // hashing lower — `moderation.rs` biases toward `Hide` where nothing is
        // transport-ordered. So the arrivals are ordered, which is the branch
        // where last-write-wins is real.
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "reply");
        let hide = a_moderation(&a_key(1), reply.op.id(), ModerationAction::Hide);
        let unhide = a_moderation(&a_key(1), reply.op.id(), ModerationAction::Unhide);

        let mut log = MemoryOpLog::new();
        log.append(root.clone(), Arrival::ordered(1, a_message_id(1)))
            .unwrap();
        log.append(reply.clone(), Arrival::ordered(2, a_message_id(1)))
            .unwrap();
        log.append(hide, Arrival::ordered(3, a_message_id(1)))
            .unwrap();
        log.append(unhide.clone(), Arrival::ordered(4, a_message_id(1)))
            .unwrap();

        let item = read(&log, &root, false)
            .items
            .into_iter()
            .find(|i| i.id == reply.op.id().to_hex())
            .expect("an unhidden reply is returned in the DEFAULT view");
        assert!(!item.is_hidden());
        assert_ne!(
            item.moderation,
            Moderation::Unmoderated,
            "a restored post must not read as one nobody touched"
        );
        assert_eq!(
            item.moderation.deciding_op().map(|e| e.id()),
            Some(unhide.op.id())
        );
    }

    #[test]
    fn a_forged_moderation_decides_nothing_and_is_named_by_nothing() {
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "reply");
        // Authentic, and by somebody who is not a moderator.
        let no_authority = a_moderation(&a_key(9), reply.op.id(), ModerationAction::Hide);
        assert!(
            no_authority.verify(),
            "the op is AUTHENTIC; it is the AUTHORITY that fails"
        );
        let log = a_log(vec![root.clone(), reply.clone(), no_authority]);

        let item = read(&log, &root, false)
            .items
            .into_iter()
            .find(|i| i.id == reply.op.id().to_hex())
            .expect("a non-moderator must not be able to hide a reply");
        assert_eq!(item.moderation, Moderation::Unmoderated);
        assert!(item.body.is_some());
    }

    // ─── A hidden root is returned and marked; a hidden reply is omitted ──

    #[test]
    fn a_hidden_root_is_returned_marked_with_its_body_withheld() {
        let root = a_root(2, "the moderated opening post");
        let reply = a_reply(3, &root, "a reply that is not hidden");
        let hide = a_moderation(&a_key(1), root.op.id(), ModerationAction::Hide);
        let log = a_log(vec![root.clone(), reply.clone(), hide.clone()]);

        let page = read(&log, &root, false);
        let root_item = &page.items[0];
        assert_eq!(root_item.id, root.op.id().to_hex());
        assert!(root_item.is_hidden(), "the root must be MARKED hidden");
        assert_eq!(
            root_item.body, None,
            "the hidden root's body is WITHHELD, not emptied"
        );
        assert_eq!(root_item.attachments, None);
        assert_eq!(
            root_item.moderation.deciding_op().map(|e| e.id()),
            Some(hide.op.id())
        );

        // Hiding the root does not hide its replies.
        assert_eq!(page.items.len(), 2);
        let reply_item = &page.items[1];
        assert!(!reply_item.is_hidden());
        assert_eq!(
            reply_item.body.as_ref().unwrap().text,
            "a reply that is not hidden"
        );
    }

    #[test]
    fn the_hidden_roots_body_comes_back_when_hidden_content_is_asked_for() {
        let root = a_root(2, "the moderated opening post");
        let hide = a_moderation(&a_key(1), root.op.id(), ModerationAction::Hide);
        let log = a_log(vec![root.clone(), hide]);

        let item = read(&log, &root, true).items.into_iter().next().unwrap();
        assert_eq!(
            item.body.as_ref().unwrap().text,
            "the moderated opening post"
        );
        assert!(item.is_hidden(), "and it is STILL marked hidden");
    }

    #[test]
    fn a_hidden_thread_is_distinguishable_from_one_the_peer_does_not_hold() {
        // THE confusion this whole shape exists to prevent. A read that dropped a
        // hidden root would answer with nothing, which is exactly what an unheld
        // thread also produces.
        let root = a_root(2, "hidden");
        let hide = a_moderation(&a_key(1), root.op.id(), ModerationAction::Hide);
        let log = a_log(vec![root.clone(), hide]);
        let never_seen = a_root(3, "never received").op.id();

        let hidden = read_thread(
            &log,
            &moderators(),
            &a_stoa(),
            &root.op.id(),
            0,
            MAX_PER_PAGE,
            false,
        )
        .unwrap();
        let absent = read_thread(
            &log,
            &moderators(),
            &a_stoa(),
            &never_seen,
            0,
            MAX_PER_PAGE,
            false,
        )
        .unwrap();

        let page = hidden.expect("a hidden thread is still readable");
        assert_eq!(page.items.len(), 1);
        assert!(page.items[0].is_hidden());
        assert_eq!(absent, Err(NotAThread::NotHeld(never_seen)));
    }

    #[test]
    fn a_moderation_by_a_peer_with_no_authority_does_not_hide_a_root() {
        let root = a_root(2, "still here");
        let impostor = a_moderation(&a_key(9), root.op.id(), ModerationAction::Hide);
        assert!(impostor.verify());
        let log = a_log(vec![root.clone(), impostor]);

        let item = read(&log, &root, false).items.into_iter().next().unwrap();
        assert!(!item.is_hidden());
        assert_eq!(item.body.unwrap().text, "still here");
    }

    #[test]
    fn a_hidden_reply_is_omitted_by_default_and_returned_marked_on_request() {
        let root = a_root(2, "root");
        let visible = a_reply(3, &root, "visible");
        let to_hide = a_reply(4, &root, "hidden");
        let hide = a_moderation(&a_key(1), to_hide.op.id(), ModerationAction::Hide);
        let log = a_log(vec![
            root.clone(),
            visible.clone(),
            to_hide.clone(),
            hide.clone(),
        ]);

        let default_view = read(&log, &root, false);
        assert!(!ids_of(&default_view).contains(&to_hide.op.id().to_hex()));
        assert!(ids_of(&default_view).contains(&visible.op.id().to_hex()));

        let wide = read(&log, &root, true);
        let hidden_item = wide
            .items
            .iter()
            .find(|i| i.id == to_hide.op.id().to_hex())
            .expect("a reader who asked to see hidden content is owed which");
        assert!(hidden_item.is_hidden());
        assert_eq!(hidden_item.body.as_ref().unwrap().text, "hidden");
        let visible_item = wide
            .items
            .iter()
            .find(|i| i.id == visible.op.id().to_hex())
            .unwrap();
        assert!(!visible_item.is_hidden());
    }

    #[test]
    fn a_reply_to_a_hidden_reply_is_still_returned_and_still_names_it() {
        // One hide binds one op. Removing descendants would let one moderation
        // reach ops no moderator acted on.
        let root = a_root(2, "root");
        let hidden = a_reply(3, &root, "hidden");
        let beneath = a_reply(4, &hidden, "beneath");
        let hide = a_moderation(&a_key(1), hidden.op.id(), ModerationAction::Hide);
        let log = a_log(vec![root.clone(), hidden.clone(), beneath.clone(), hide]);

        let page = read(&log, &root, false);
        let ids = ids_of(&page);
        assert!(!ids.contains(&hidden.op.id().to_hex()));
        let child = page
            .items
            .iter()
            .find(|i| i.id == beneath.op.id().to_hex())
            .expect("a reply beneath a hidden one must survive");
        assert_eq!(
            child.parent,
            Some(hidden.op.id().to_hex()),
            "it still reports the hidden post as its parent"
        );
    }

    #[test]
    fn a_forged_hide_removes_nothing() {
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "reply");
        // Claims the moderator as author, signed by somebody else.
        let moderator = a_key(1);
        let op = Op {
            stoa: a_stoa(),
            author: moderator.public_key(),
            kind: OpKind::Moderate {
                target: reply.op.id(),
                action: ModerationAction::Hide,
            },
        };
        let forged = SignedOp {
            signature: sign_op_bytes(&a_key(9), &op.canonical_bytes()),
            op,
        };
        assert!(!forged.verify(), "the fixture must be an actual forgery");
        let log = a_log(vec![root.clone(), reply.clone(), forged]);

        let item = read(&log, &root, false)
            .items
            .into_iter()
            .find(|i| i.id == reply.op.id().to_hex())
            .expect("a forged hide must remove nothing");
        assert!(!item.is_hidden());
    }

    // ─── Held, unheld, and the three refusals ─────────────────────────────

    #[test]
    fn a_thread_with_no_replies_is_served_and_not_refused() {
        let root = a_root(2, "alone");
        let log = a_log(vec![root.clone()]);
        let page = read(&log, &root, false);
        assert_eq!(ids_of(&page), vec![root.op.id().to_hex()]);
        assert!(!page.has_more);
    }

    #[test]
    fn the_three_refusals_are_three_different_messages() {
        // Three different mistakes call for three different responses: wait for
        // the op to propagate, correct a category error, or read the thread this
        // reply actually belongs to.
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "a reply");
        let voter = a_key(4);
        let vote = Op {
            stoa: a_stoa(),
            author: voter.public_key(),
            kind: OpKind::Vote {
                target: root.op.id(),
                direction: VoteDirection::Up,
            },
        }
        .sign(&voter);
        let never_seen = a_root(5, "never received").op.id();
        let log = a_log(vec![root, reply.clone(), vote.clone()]);

        let refuse = |id: &OpId| {
            read_thread(&log, &moderators(), &a_stoa(), id, 0, MAX_PER_PAGE, false)
                .unwrap()
                .expect_err("must be refused")
        };
        let not_held = refuse(&never_seen);
        let not_a_post = refuse(&vote.op.id());
        let is_a_reply = refuse(&reply.op.id());

        assert_eq!(not_held, NotAThread::NotHeld(never_seen));
        assert_eq!(not_a_post, NotAThread::NotAPost(vote.op.id()));
        assert_eq!(is_a_reply, NotAThread::IsAReply(reply.op.id()));

        // And the MESSAGES differ, not merely the variants — a view renders the
        // string, and three variants sharing one wording tells a caller nothing.
        let messages = [
            not_held.to_string(),
            not_a_post.to_string(),
            is_a_reply.to_string(),
        ];
        for (i, a) in messages.iter().enumerate() {
            for b in messages.iter().skip(i + 1) {
                assert_ne!(a, b, "two refusals share a message");
            }
        }
    }

    #[test]
    fn every_op_kind_is_either_a_post_or_in_the_non_post_table() {
        // The guard on [`every_non_post_kind`]'s hand-written list. A hand
        // maintained sweep list goes stale silently: a sixth kind enters
        // `OpKind` and the sweep below keeps passing over five.
        //
        // This match is exhaustive and has no wildcard arm, so adding a variant
        // to `OpKind` stops this file compiling until somebody decides whether
        // the new kind is a post or belongs in the table. That is the only
        // mechanism available — nothing can enumerate an enum's variants at
        // runtime — and it is why the arms are spelled out rather than
        // `_ => {}`.
        let root = a_root(2, "root");
        let table = every_non_post_kind(root.op.id());
        let mut seen_kinds = 0;
        for (_, op) in &table {
            match &op.op.kind {
                // Never in the table: a post is the kind that opens a thread.
                OpKind::Post { .. } => {
                    panic!("a Post must not be in the non-post table")
                }
                OpKind::Revise { .. }
                | OpKind::Moderate { .. }
                | OpKind::Vote { .. }
                | OpKind::StoaMetadata { .. } => seen_kinds += 1,
            }
        }
        assert_eq!(
            seen_kinds, 4,
            "every non-post kind must be represented exactly once"
        );
        // And they are four DISTINCT kinds rather than four copies of one — a
        // table of four votes would satisfy the count above.
        let discriminants: HashSet<std::mem::Discriminant<OpKind>> = table
            .iter()
            .map(|(_, op)| std::mem::discriminant(&op.op.kind))
            .collect();
        assert_eq!(discriminants.len(), 4, "four kinds, not four of one kind");
    }

    #[test]
    fn every_non_post_kind_takes_the_same_refusal() {
        // The rule is stated over KINDS rather than enumerated, so it is tested
        // over kinds: a vote, a moderation op, a Stoa metadata op and a revision
        // are each an op this peer holds that is not a post, and the outcome must
        // not vary between them. A kind this table does not name would be left
        // with an answer of its own, which is what
        // `every_op_kind_is_either_a_post_or_in_the_non_post_table` prevents.
        let root = a_root(2, "root");
        let mut ops = vec![root.clone()];
        let table = every_non_post_kind(root.op.id());
        ops.extend(table.iter().map(|(_, op)| op.clone()));
        let log = a_log(ops);

        for (name, op) in &table {
            let id = op.op.id();
            // Held and in this Stoa, or the refusal below would be about an
            // absence rather than about a kind.
            assert!(
                log.get(&id).unwrap().is_some(),
                "{name} must be HELD, or this passes for absence rather than kind"
            );
            assert_eq!(
                log.get(&id).unwrap().unwrap().op.op.stoa,
                a_stoa(),
                "{name} must be in the named Stoa"
            );
            assert!(op.verify(), "{name} must be authentic");

            assert_eq!(
                read_thread(&log, &moderators(), &a_stoa(), &id, 0, MAX_PER_PAGE, false).unwrap(),
                Err(NotAThread::NotAPost(id)),
                "{name} must take the not-a-post refusal"
            );
        }
    }

    #[test]
    fn no_held_non_post_is_ever_reported_as_unheld() {
        // The specific falsehood, and the one a revision invites: a revision is
        // bound up with a post, so "this is a version of a post, not a post" is
        // a distinction a reader can talk themselves out of — and the answer
        // they talk themselves into is the not-held one, which the log disproves.
        //
        // **The expectation is the message the implementation would have
        // produced had it been wrong**, built for the SAME op id. So this
        // cannot pass by both messages being reworded together, and it cannot
        // pass because two different ids made two different strings. The only
        // way to satisfy it is to not report a held op as unheld.
        let root = a_root(2, "root");
        let table = every_non_post_kind(root.op.id());
        let mut ops = vec![root.clone()];
        ops.extend(table.iter().map(|(_, op)| op.clone()));
        let log = a_log(ops);

        for (name, op) in &table {
            let id = op.op.id();
            let refusal = read_thread(&log, &moderators(), &a_stoa(), &id, 0, MAX_PER_PAGE, false)
                .unwrap()
                .expect_err("a non-post must be refused");
            let the_false_answer = NotAThread::NotHeld(id).to_string();

            assert_ne!(
                refusal.to_string(),
                the_false_answer,
                "{name} is held, so reporting it as not held is false"
            );
            assert_ne!(
                refusal,
                NotAThread::NotHeld(id),
                "{name} must not take the not-held variant either"
            );
        }

        // And the not-held refusal is genuinely reachable in this same log for an
        // id the peer really does not hold — without which the assertions above
        // would be satisfied by an implementation that never says "not held" at
        // all, which is a different bug rather than this one fixed.
        let never_seen = a_root(7, "never received").op.id();
        assert!(log.get(&never_seen).unwrap().is_none());
        assert_eq!(
            read_thread(
                &log,
                &moderators(),
                &a_stoa(),
                &never_seen,
                0,
                MAX_PER_PAGE,
                false
            )
            .unwrap(),
            Err(NotAThread::NotHeld(never_seen))
        );
    }

    #[test]
    fn the_three_refusals_each_name_the_next_action_they_imply() {
        // Three different strings is not the property. The property is that a
        // caller acts differently on each, and the message has to carry which:
        //
        //   not held        -> the op may still arrive; wait, and say so.
        //   held, not a post -> the identifier names the wrong kind of thing;
        //                       waiting never helps.
        //   held, a reply    -> one level too deep; there is a right answer
        //                       nearby, reachable by another call.
        //
        // Asserted as a RELATION between the three rather than as three pinned
        // literals, because a pinned literal fails on a reword and passes on
        // misinformation, which is the wrong way round. Each phrase below is
        // required of exactly one refusal and forbidden of the other two, so
        // moving a phrase from one message to another fails here even though the
        // three strings stay distinct.
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "a reply");
        let vote = a_vote(&a_key(4), root.op.id());
        let never_seen = a_root(5, "never received").op.id();
        let log = a_log(vec![root, reply.clone(), vote.clone()]);

        let refuse = |id: &OpId| {
            read_thread(&log, &moderators(), &a_stoa(), id, 0, MAX_PER_PAGE, false)
                .unwrap()
                .expect_err("must be refused")
                .to_string()
        };
        let not_held = refuse(&never_seen);
        let not_a_post = refuse(&vote.op.id());
        let is_a_reply = refuse(&reply.op.id());

        // Waiting is the answer to exactly ONE of the three, and telling a
        // caller to wait for an op that has already arrived, or that can never
        // be a thread, is the expensive mistake this triple exists to prevent.
        assert!(
            not_held.contains("may not have arrived yet"),
            "the not-held refusal must say the op may still arrive, got {not_held}"
        );
        for (name, other) in [("not-a-post", &not_a_post), ("is-a-reply", &is_a_reply)] {
            assert!(
                !other.contains("arriv"),
                "the {name} refusal must not send a caller waiting for an op the \
                 peer already holds, got {other}"
            );
            assert!(
                !other.contains("holds no op"),
                "the {name} refusal must not claim the peer holds nothing under \
                 an id it does hold, got {other}"
            );
        }

        // The category error names the category, and nothing else does.
        assert!(
            not_a_post.contains("is not a post"),
            "the not-a-post refusal must name the category error, got {not_a_post}"
        );
        assert!(!not_held.contains("is not a post"));
        assert!(!is_a_reply.contains("is not a post"));

        // And the one a caller can act on by making another call says which call.
        assert!(
            is_a_reply.contains("read the") && is_a_reply.contains("thread"),
            "the is-a-reply refusal must point at the thread to read instead, got \
             {is_a_reply}"
        );
        assert!(!not_held.contains("read the"));
        assert!(!not_a_post.contains("read the"));
    }

    #[test]
    fn a_store_failure_is_an_error_and_never_an_empty_thread() {
        // An empty thread and a broken store mean opposite things and look
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

        let err = read_thread(
            &BrokenLog,
            &moderators(),
            &a_stoa(),
            &a_root(2, "x").op.id(),
            0,
            MAX_PER_PAGE,
            false,
        )
        .expect_err("a store failure must not be flattened into a page");
        assert!(
            err.to_string().contains("the disk is on fire"),
            "the underlying reason must survive to the caller, got {err}"
        );
        // And the chain walk answers the same way rather than silently placing
        // nothing, which would report a broken disk as an unplaceable post.
        assert!(thread_of(&BrokenLog, &a_root(2, "x").op.id()).is_err());
    }

    // ─── Order, and what it does and does not claim ───────────────────────

    /// A thread whose root does **not** lead the log's own order.
    ///
    /// Returns `(root, replies)` with at least one reply's op id sorting below
    /// the root's, so that `read_thread`'s `items.insert(0, …)` is genuinely the
    /// thing under test.
    ///
    /// **Searched rather than hoped for.** The branch-on-what-happened form —
    /// "if the log leads with the root this still passes, it just tests less" —
    /// reports the case rather than guaranteeing it, and a fixture edit that made
    /// the hashes fall the other way would leave the test green while exercising
    /// nothing. Which of several SHA-256 outputs sorts first is not a fact to
    /// take on trust, so the arrangement is found and then asserted.
    ///
    /// The budget of 256 mirrors `moderation.rs`'s two searches and is chosen for
    /// the same reason: each body is an independent trial and every one of four
    /// replies would have to sort above the root, so exhausting it is far below
    /// any rate at which a flaky test would be noticed. Exhaustion panics rather
    /// than skipping, so the impossible case is loud.
    fn a_thread_whose_root_does_not_sort_first() -> (SignedOp, Vec<SignedOp>) {
        for n in 0..256u32 {
            let root = a_root(2, &format!("root {n}"));
            let replies: Vec<SignedOp> = (0..4)
                .map(|i| a_reply(3 + i, &root, &format!("reply {i} of {n}")))
                .collect();
            if replies.iter().any(|r| r.op.id() < root.op.id()) {
                return (root, replies);
            }
        }
        panic!("no body in 256 attempts put a reply's op id below the root's; SHA-256 is not this biased");
    }

    #[test]
    fn the_root_is_the_first_item_whatever_the_logs_order_puts_first() {
        // The one position this module fixes. The fixture is SEARCHED so that the
        // log's own order does not lead with the root — see
        // `a_thread_whose_root_does_not_sort_first` for why branching on which
        // way the hashes fell is not good enough.
        let (root, replies) = a_thread_whose_root_does_not_sort_first();
        let mut ops = replies.clone();
        ops.push(root.clone());
        let log = a_log(ops);

        let log_order: Vec<String> = log
            .iter_stoa(&a_stoa())
            .unwrap()
            .iter()
            .map(|e| e.id().to_hex())
            .collect();
        assert_ne!(
            log_order[0],
            root.op.id().to_hex(),
            "the search must have found a log whose order does NOT lead with the \
             root, or the insertion is not being exercised"
        );

        let page = read(&log, &root, false);
        assert_eq!(
            page.items[0].id,
            root.op.id().to_hex(),
            "the root leads the page even though it does not lead the log"
        );
        assert_eq!(page.items.len(), 5);

        // And the replies keep the log's relative order — this module sorts
        // nothing.
        let replies_from_log: Vec<String> = log_order
            .iter()
            .filter(|id| **id != root.op.id().to_hex())
            .cloned()
            .collect();
        let replies_from_page: Vec<String> = ids_of(&page).into_iter().skip(1).collect();
        assert_eq!(replies_from_page, replies_from_log);
    }

    #[test]
    fn two_peers_holding_the_same_ops_return_the_same_sequence() {
        // The one property the order actually claims. Two logs, opposite
        // insertion sequences, identical output.
        let root = a_root(2, "root");
        let ops: Vec<SignedOp> = std::iter::once(root.clone())
            .chain((0..4).map(|i| a_reply(3 + i, &root, &format!("reply {i}"))))
            .collect();

        let forwards = a_log(ops.clone());
        let mut backwards = MemoryOpLog::new();
        for op in ops.iter().rev() {
            backwards.append(op.clone(), Arrival::unordered()).unwrap();
        }

        let a = ids_of(&read(&forwards, &root, false));
        let b = ids_of(&read(&backwards, &root, false));
        assert_eq!(a, b);
        assert_eq!(a.len(), 5, "the fixture must exercise all five");
    }

    #[test]
    fn a_deep_chain_comes_back_flat() {
        let root = a_root(2, "root");
        let one = a_reply(3, &root, "one");
        let two = a_reply(4, &one, "two");
        let three = a_reply(5, &two, "three");
        let log = a_log(vec![root.clone(), one.clone(), two.clone(), three.clone()]);

        let page = read(&log, &root, false);
        assert_eq!(page.items.len(), 4, "all four in ONE flat sequence");
        let parents: Vec<Option<String>> = page.items.iter().map(|i| i.parent.clone()).collect();
        assert!(parents.contains(&Some(one.op.id().to_hex())));
        assert!(parents.contains(&Some(two.op.id().to_hex())));
    }

    // ─── Paging ───────────────────────────────────────────────────────────

    #[test]
    fn pages_partition_the_thread_with_no_gap_and_no_repeat() {
        let root = a_root(2, "root");
        let mut ops = vec![root.clone()];
        ops.extend((0..4).map(|i| a_reply(3 + i, &root, &format!("reply {i}"))));
        let log = a_log(ops);

        let whole = ids_of(&read(&log, &root, false));
        assert_eq!(whole.len(), 5);

        let mut seen = Vec::new();
        for page in 0..3 {
            let p = read_thread(
                &log,
                &moderators(),
                &a_stoa(),
                &root.op.id(),
                page,
                2,
                false,
            )
            .unwrap()
            .unwrap();
            assert_eq!(p.page, page);
            seen.extend(ids_of(&p));
        }
        assert_eq!(seen, whole, "pages must partition the thread in order");
    }

    #[test]
    fn the_root_occupies_a_slot_and_is_not_repeated_on_a_later_page() {
        let root = a_root(2, "root");
        let mut ops = vec![root.clone()];
        ops.extend((0..3).map(|i| a_reply(3 + i, &root, &format!("reply {i}"))));
        let log = a_log(ops);

        let first = read_thread(&log, &moderators(), &a_stoa(), &root.op.id(), 0, 2, false)
            .unwrap()
            .unwrap();
        assert_eq!(
            first.items.len(),
            2,
            "the root occupies one of the first page's two slots"
        );
        assert_eq!(first.items[0].id, root.op.id().to_hex());

        let second = read_thread(&log, &moderators(), &a_stoa(), &root.op.id(), 1, 2, false)
            .unwrap()
            .unwrap();
        assert!(
            !ids_of(&second).contains(&root.op.id().to_hex()),
            "the root must not be repeated on a later page"
        );
    }

    #[test]
    fn has_more_is_true_exactly_while_a_further_page_exists() {
        // At the boundary: 4 items at a page size of 2 means page 1 is the LAST
        // page and must report false.
        let root = a_root(2, "root");
        let mut ops = vec![root.clone()];
        ops.extend((0..3).map(|i| a_reply(3 + i, &root, &format!("reply {i}"))));
        let log = a_log(ops);

        let first = read_thread(&log, &moderators(), &a_stoa(), &root.op.id(), 0, 2, false)
            .unwrap()
            .unwrap();
        assert!(first.has_more);
        let last = read_thread(&log, &moderators(), &a_stoa(), &root.op.id(), 1, 2, false)
            .unwrap()
            .unwrap();
        assert_eq!(last.items.len(), 2);
        assert!(
            !last.has_more,
            "the final FULL page must not claim another follows"
        );
    }

    #[test]
    fn hidden_replies_are_excluded_before_the_page_is_cut() {
        // The bug this pins: filtering after slicing yields short pages with
        // gaps, and a reader paging forward silently skips items.
        //
        // The two replies the ORDER puts first are hidden, so a
        // filter-after-paging implementation returns a first page holding only
        // the root.
        let root = a_root(2, "root");
        let replies: Vec<SignedOp> = (0..4)
            .map(|i| a_reply(3 + i, &root, &format!("reply {i}")))
            .collect();
        let mut ops = vec![root.clone()];
        ops.extend(replies.clone());
        let unhidden_log = a_log(ops.clone());
        // Whichever two the convergent order puts first among the REPLIES.
        let reply_order: Vec<String> = ids_of(&read(&unhidden_log, &root, false))
            .into_iter()
            .skip(1)
            .collect();

        let moderator = a_key(1);
        for hex in reply_order.iter().take(2) {
            ops.push(a_moderation(
                &moderator,
                OpId::from_hex(hex).unwrap(),
                ModerationAction::Hide,
            ));
        }
        let log = a_log(ops);

        let page = read_thread(&log, &moderators(), &a_stoa(), &root.op.id(), 0, 3, false)
            .unwrap()
            .unwrap();
        assert_eq!(
            page.items.len(),
            3,
            "the first page must be FULL of visible items, not the survivors of \
             the first three"
        );
        assert!(!page.has_more, "three visible items fill one page of three");
        assert_eq!(
            ids_of(&page),
            std::iter::once(root.op.id().to_hex())
                .chain(reply_order[2..].iter().cloned())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn a_page_past_the_end_is_empty_rather_than_an_error() {
        let root = a_root(2, "root");
        let log = a_log(vec![root.clone()]);
        let p = read_thread(&log, &moderators(), &a_stoa(), &root.op.id(), 99, 20, false)
            .unwrap()
            .unwrap();
        assert!(p.items.is_empty());
        assert!(!p.has_more);
        assert_eq!(p.page, 99);
    }

    #[test]
    fn an_enormous_page_index_does_not_overflow() {
        // `page * per_page` on a caller-supplied `page` is the multiplication
        // that overflows. In release builds it wraps silently, which would serve
        // a page from the middle of the thread to a caller who asked for one past
        // the end.
        let root = a_root(2, "root");
        let reply = a_reply(3, &root, "reply");
        let log = a_log(vec![root.clone(), reply]);
        let p = read_thread(
            &log,
            &moderators(),
            &a_stoa(),
            &root.op.id(),
            usize::MAX,
            20,
            false,
        )
        .unwrap()
        .unwrap();
        assert!(p.items.is_empty());
        assert!(!p.has_more);
        assert_eq!(p.page, usize::MAX);
    }

    #[test]
    fn per_page_is_clamped_at_both_ends() {
        assert_eq!(clamp_per_page(None), DEFAULT_PER_PAGE);
        assert_eq!(clamp_per_page(Some(0)), DEFAULT_PER_PAGE);
        assert_eq!(clamp_per_page(Some(1)), 1);
        // At the boundary rather than past it.
        assert_eq!(clamp_per_page(Some(MAX_PER_PAGE)), MAX_PER_PAGE);
        assert_eq!(clamp_per_page(Some(MAX_PER_PAGE + 1)), MAX_PER_PAGE);
        assert_eq!(clamp_per_page(Some(usize::MAX)), MAX_PER_PAGE);
    }

    #[test]
    fn the_page_caps_agree_until_someone_decides_otherwise() {
        // Hardcoded rather than compared against the feed's constant, which would
        // pass whichever way both drifted. The two are separate numbers that
        // happen to agree; this records the agreement as observed rather than
        // enforced, so a deliberate divergence is a visible edit here.
        assert_eq!(MAX_PER_PAGE, 100);
        assert_eq!(DEFAULT_PER_PAGE, 20);
        assert_eq!(crate::feed::MAX_PER_PAGE, MAX_PER_PAGE);
        assert_eq!(crate::feed::DEFAULT_PER_PAGE, DEFAULT_PER_PAGE);
    }

    // ─── Sanitising ───────────────────────────────────────────────────────

    #[test]
    fn a_hostile_body_is_sanitised_and_the_stored_op_is_not() {
        let hostile = "p\u{0430}ypal\u{202E}gnp.js";
        let root = a_root(2, "root");
        let reply = a_post_in(a_stoa(), 3, Some(root.op.id()), Some(root.op.id()), hostile);
        let log = a_log(vec![root.clone(), reply.clone()]);

        let item = read(&log, &root, false)
            .items
            .into_iter()
            .find(|i| i.id == reply.op.id().to_hex())
            .unwrap();
        let body = item.body.unwrap();
        assert_eq!(body.removed, 1, "the bidi override must be removed");
        assert_eq!(body.marked, 1, "the Cyrillic letter must be marked");
        assert!(!body.text.contains('\u{202E}'));
        assert!(
            body.text.contains('\u{0430}'),
            "marking must not correct — the character is still there"
        );

        // And the LOG still holds the author's bytes, unchanged and verifying.
        let stored = log.get(&reply.op.id()).unwrap().unwrap();
        match &stored.op.op.kind {
            OpKind::Post { body, .. } => assert_eq!(body, hostile),
            other => panic!("expected a post, got {other:?}"),
        }
        assert!(stored.op.verify());
    }

    #[test]
    fn attachments_are_sanitised_too() {
        let key = a_key(2);
        let root = Op {
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
        let log = a_log(vec![root.clone()]);

        let item = read(&log, &root, false).items.into_iter().next().unwrap();
        let attachments = item.attachments.unwrap();
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].removed, 1);
        assert!(!attachments[0].text.contains('\u{202E}'));
    }

    #[test]
    fn an_ordinary_body_reports_nothing_removed_and_nothing_marked() {
        let root = a_root(2, "ordinary text");
        let log = a_log(vec![root.clone()]);
        let body = read(&log, &root, false).items[0].body.clone().unwrap();
        assert_eq!(body.text, "ordinary text");
        assert_eq!(body.removed, 0);
        assert_eq!(body.marked, 0);
    }

    // ─── An adversarial log, read whole ───────────────────────────────────

    #[test]
    fn a_genuine_thread_is_unaffected_by_adversarial_chains_elsewhere() {
        // Everything at once: a forgery, a claim with no parent in the thread, a
        // reply whose parent is absent, a chain through a vote, and a post from
        // another Stoa. Exactly the posts whose chains reach this root come back.
        let root = a_root(2, "root");
        let genuine_one = a_reply(3, &root, "genuine one");
        let genuine_two = a_reply(4, &genuine_one, "genuine two");

        let liar = a_post_in(a_stoa(), 5, Some(root.op.id()), None, "claims, no parent");
        let orphan = a_post_in(
            a_stoa(),
            6,
            Some(root.op.id()),
            Some(a_root(7, "absent").op.id()),
            "parent absent",
        );
        let forged = a_forged_post(
            &a_key(3).public_key(),
            &a_key(9),
            Some(root.op.id()),
            Some(root.op.id()),
            "forged",
        );
        let voter = a_key(8);
        let vote = Op {
            stoa: a_stoa(),
            author: voter.public_key(),
            kind: OpKind::Vote {
                target: root.op.id(),
                direction: VoteDirection::Down,
            },
        }
        .sign(&voter);
        let through_a_vote = a_post_in(
            a_stoa(),
            10,
            Some(root.op.id()),
            Some(vote.op.id()),
            "chained through a vote",
        );
        let elsewhere = Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Elsewhere".to_string(),
        }
        .address()
        .unwrap();
        let foreign = a_post_in(
            elsewhere,
            11,
            Some(root.op.id()),
            Some(root.op.id()),
            "another Stoa",
        );

        let log = a_log(vec![
            root.clone(),
            genuine_one.clone(),
            genuine_two.clone(),
            liar.clone(),
            orphan,
            forged,
            vote,
            through_a_vote,
            foreign,
        ]);

        let page = read(&log, &root, false);
        let mut got = ids_of(&page);
        got.sort();
        let mut want = vec![
            root.op.id().to_hex(),
            genuine_one.op.id().to_hex(),
            genuine_two.op.id().to_hex(),
        ];
        want.sort();
        assert_eq!(got, want, "exactly the posts whose chains reach the root");

        // The liar is its own root, which is a thread of one — so the read is
        // refusing it here rather than losing it everywhere.
        let its_own = read(&log, &liar, false);
        assert_eq!(ids_of(&its_own), vec![liar.op.id().to_hex()]);
    }
}
