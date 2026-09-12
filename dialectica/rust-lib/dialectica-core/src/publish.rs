//! The write path: building, signing and storing an op, and reading a thread.
//!
//! # Why this file exists
//!
//! Every primitive the MVP write path needs already existed —
//! [`crate::identity`] signs, [`crate::op`] encodes, [`crate::stoa`] identifies,
//! [`crate::log`] stores. What did not exist is the step that puts them in order
//! and refuses the arrangements that are wrong. That step is here.
//!
//! # This module decides authorisation; the store still decides nothing
//!
//! §3.3's "verification happens on read" is about **inbound** ops, and it stays
//! exactly as it was: [`crate::log::OpLog::append`] verifies nothing, and the
//! feed re-verifies everything it reads. Nothing below weakens that.
//!
//! What this module adds is a check on the **outbound** side, which is a
//! different question with a different answer. A peer publishing an op is not
//! receiving a claim it must evaluate — it is making one, with a key it holds,
//! about a Stoa it either reads or does not. Refusing to publish into a Stoa this
//! peer has not joined is not a filter on content; it is declining to sign
//! something the user could not have meant.
//!
//! The distinction matters because conflating them produces the defect §6.2
//! measured in the nearest kin project, "checks moderator authority only on the
//! send path and never on the read path". The rule here is: **both paths check,
//! and neither substitutes for the other.**
//!
//! # Storing is not publishing, and that seam is deliberate
//!
//! Every function here **stores an op locally and returns it**. None of them
//! sends anything: the delivery transport is a separate concern with a separate
//! owner, and the reliable channel does not exist at this layer.
//!
//! The shape is chosen so that adding it is additive. A publish returns the
//! [`crate::op::SignedOp`] it stored, whose `to_bytes()` is exactly what would go
//! on the wire — so a transport layer sends `to_bytes()` after the store
//! succeeded, and needs nothing from this module it is not already handed. The
//! ordering is the right one round: an op stored and not sent is one a peer can
//! re-gossip later, while an op sent and not stored is one this peer cannot serve
//! to anybody, including itself.
//!
//! **What is deliberately NOT here as a consequence:** any notion of delivery
//! success, any retry, any queue. A caller is told the op was stored, which is
//! the only thing this layer knows.
//!
//! # An op is signed with the ROOT key, which is one identity per user
//!
//! §5.2 gives a user one identity per Stoa and [`crate::keystore::Keystore::stoa_key`]
//! derives it. **The MVP does not call it** — see
//! [`crate::keystore::Keystore::root_key`] for the decision and what it costs.
//! Everything here takes a `&SecretKey` rather than reaching for a keystore, so
//! restoring §5.2 changes which key the *adapter* passes in and nothing in this
//! file.

use crate::identity::SecretKey;
use crate::log::{Appended, JoinedStoa, OpLog, OpLogError, Relation, StoaRegistry};
use crate::op::{Op, OpId, OpKind, SignedOp, VoteDirection};
use crate::stoa::{Genesis, GenesisError, Policy};

/// The longest body this peer will sign.
///
/// **Not the same bound as `op.rs`'s `MAX_FIELD_LEN`, and the difference is the
/// direction.** That one is a *decode* bound: it refuses a hostile length prefix
/// before allocating on it, and it is set by what SDS could have delivered. This
/// one is an *encode* bound, and it exists because a peer signing a 150 KiB post
/// produces a message the transport will refuse to carry — so the op would be
/// stored, unsendable, and indistinguishable from one that simply had not
/// propagated.
///
/// Set well below the field cap because a post is not an attachment: §4.6 puts
/// large content in Logos Storage behind a CID, which is what the
/// `attachments` field is for. 64 KiB is far more than any forum post and leaves
/// the rest of the SDS message budget for the envelope and the signature.
///
/// Refused rather than truncated. Truncating a signed document changes what the
/// author said, which is the one thing a signature exists to prevent.
pub const MAX_BODY_BYTES: usize = 64 * 1024;

/// The most attachments one op may carry.
///
/// A count bound rather than only a per-element one, for the reason `op.rs`
/// records about its own list decoding: bounding each element and not the count
/// leaves the product unbounded, and `MAX_FIELD_LEN` elements of
/// `MAX_FIELD_LEN` bytes is 22 GiB of perfectly in-bounds fields. On the encode
/// side the same arithmetic decides what the transport can carry.
pub const MAX_ATTACHMENTS: usize = 16;

/// The longest attachment reference this peer will sign.
///
/// A Logos Storage CID, which is a short content address and not a payload. A
/// generous multiple of any real CID, and small enough that
/// `MAX_ATTACHMENTS × MAX_ATTACHMENT_BYTES` stays a rounding error against
/// [`MAX_BODY_BYTES`].
pub const MAX_ATTACHMENT_BYTES: usize = 512;

/// Why an op could not be published.
///
/// Each variant names a **different** mistake, following `GenesisError`'s and
/// `OpLogError`'s reasoning: a caller told only "invalid" cannot tell a user
/// whether to shorten their post, join the Stoa, or report a bug.
#[derive(Debug, PartialEq, Eq)]
pub enum PublishError {
    /// The store could not be reached, or what it held did not decode.
    ///
    /// Carries [`OpLogError`] rather than flattening it to a string, because its
    /// variants are already distinguishable and re-stringifying would throw that
    /// away one layer from where a caller might want it.
    Storage(OpLogError),
    /// This peer has not joined the Stoa being posted to.
    ///
    /// **The outbound authorisation check.** Not a storage failure and not a bad
    /// request: the request is well-formed and names a real Stoa, and this peer
    /// simply has no genesis record for it — so it cannot know the Stoa's policy,
    /// cannot resolve its moderators, and would be signing into a forum it cannot
    /// read back.
    NotJoined,
    /// The genesis record could not be encoded, or does not describe this Stoa.
    Genesis(GenesisError),
    /// The body is over [`MAX_BODY_BYTES`].
    BodyTooLong { bytes: usize, max: usize },
    /// More than [`MAX_ATTACHMENTS`] attachments.
    TooManyAttachments { count: usize, max: usize },
    /// One attachment reference is over [`MAX_ATTACHMENT_BYTES`].
    AttachmentTooLong { bytes: usize, max: usize },
    /// The op a reply, vote or revision names is not in this peer's store.
    ///
    /// **Not an error about the target being invalid** — it may be perfectly
    /// valid and simply not have reached this peer (§3.3 makes a partial op set
    /// the normal case). It is refused anyway, because the alternative is signing
    /// a reply to something this peer cannot show the user, which is
    /// indistinguishable to them from a reply that went nowhere.
    TargetNotFound,
    /// The op named is in the store but is in a different Stoa.
    ///
    /// A cross-Stoa reply would be a thread spanning two forums, which the wire
    /// format can express and no reader resolves — `iter_stoa` restricts by the
    /// op's own `stoa` field, so the reply would be invisible in both.
    TargetInAnotherStoa,
    /// The op named cannot be replied to, voted on, or revised.
    ///
    /// A vote on a vote, a reply to a moderation. The wire format permits naming
    /// any op id; nothing resolves these, so signing one produces an op with no
    /// observable effect.
    TargetNotAPost,
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::Storage(e) => write!(f, "{e}"),
            PublishError::NotJoined => write!(
                f,
                "this peer has not joined that Stoa; join it before posting, so \
                 that its genesis record is available to read the result back"
            ),
            PublishError::Genesis(e) => write!(f, "genesis record: {e}"),
            PublishError::BodyTooLong { bytes, max } => write!(
                f,
                "the post body is {bytes} bytes, over the {max}-byte limit; \
                 shorten it, or attach the content by reference"
            ),
            PublishError::TooManyAttachments { count, max } => write!(
                f,
                "the post carries {count} attachments, over the limit of {max}"
            ),
            PublishError::AttachmentTooLong { bytes, max } => write!(
                f,
                "an attachment reference is {bytes} bytes, over the {max}-byte \
                 limit; an attachment is a content address, not the content"
            ),
            PublishError::TargetNotFound => write!(
                f,
                "the op being replied to or voted on is not in this peer's store; \
                 it may not have reached this peer yet"
            ),
            PublishError::TargetInAnotherStoa => write!(
                f,
                "the op being replied to or voted on belongs to a different Stoa"
            ),
            PublishError::TargetNotAPost => write!(
                f,
                "the op named is not a post, so it cannot be replied to or voted on"
            ),
        }
    }
}

/// `std::error::Error`, so a caller can put this in a `Box<dyn Error>` and use `?`.
///
/// [`OpLogError`] already implements it and these did not, which is an asymmetry a
/// consumer meets immediately: the seeder example could `?` a storage failure and
/// not a publish failure, for no reason a caller could act on. An error type
/// crossing a public API is one somebody will want to compose.
///
/// `source` returns the underlying [`OpLogError`] for the one variant that wraps
/// another error, and `None` elsewhere. That is what makes the chain walkable
/// rather than the `Display` string being the only route to the cause.
impl std::error::Error for PublishError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PublishError::Storage(e) => Some(e),
            // `Genesis` deliberately does NOT forward: `GenesisError`'s `Display`
            // is already interpolated into this type's message, so reporting it as
            // a source too would print it twice in any consumer that walks the
            // chain. The variant carries it for matching, not for chaining.
            _ => None,
        }
    }
}

impl From<OpLogError> for PublishError {
    fn from(e: OpLogError) -> Self {
        PublishError::Storage(e)
    }
}

/// Create a Stoa: build its genesis record, remember it, return it.
///
/// # Creating a Stoa publishes no op, and that is `op.rs`'s decision not this
/// one's
///
/// §3.3 lists creating a Stoa among the things that happen, and `op.rs` records
/// why it is not an [`OpKind`]: "a Stoa is a genesis record its creator
/// publishes, and hashing that record is what creates it; there is nothing for a
/// *signed op* to add." The record is already bound to its creator — the creator
/// key is inside the address preimage — so a signature over it would authenticate
/// what the address already authenticates.
///
/// So this signs nothing, and takes the creator's **public** key rather than a
/// secret one. That is the honest signature: there is no signing step to hide.
///
/// # The title is not checked for uniqueness, because it cannot be
///
/// §4.8 and the UI brief's obligation 2b both say it: a title is decoration and
/// the address is the identity. Two Stoas may share a title, and one may be named
/// to impersonate another. Nothing here refuses that, because there is no
/// registry to refuse it against — that is what permissionless creation means.
pub fn create_stoa<R: StoaRegistry>(
    registry: &mut R,
    creator: &crate::identity::PublicKey,
    title: &str,
) -> Result<JoinedStoa, PublishError> {
    let genesis = Genesis {
        creator: creator.clone(),
        policy: Policy::Open,
        title: title.to_string(),
    };
    // Encoded here rather than left to the registry, so an over-long title is
    // refused as a `Genesis` error naming the bound rather than surfacing as a
    // storage-layer `CorruptEntry` about our own input.
    genesis
        .canonical_bytes()
        .map_err(PublishError::Genesis)?;

    registry.remember_stoa(&genesis, Relation::Created)?;
    Ok(JoinedStoa {
        genesis,
        relation: Relation::Created,
    })
}

/// Join a Stoa by address, verifying the record hashes to it.
///
/// # THE security property, and it is not weakened here
///
/// §4.8: an address is the hash of the genesis record, so a wrong or tampered
/// record fails to match. PLAN.md §9.1 is explicit that "`joinStoa` therefore
/// verifies rather than trusts, and a mismatch is an error, never a join of
/// something-close-enough".
///
/// [`Genesis::matches`] is that check, and it consults nothing — no registry, no
/// peer, no third party. A caller cannot use this to install themselves as a
/// Stoa's moderator: changing the creator changes the record, which changes the
/// address, which no longer matches the one they were given.
///
/// **The record is a parameter because there is nowhere to fetch it from.** A
/// peer handed only an address cannot recover the record — the address is a
/// one-way hash. So the record arrives with the address, from whoever shared the
/// Stoa, and this function's job is to refuse the pairs that do not agree.
pub fn join_stoa<R: StoaRegistry>(
    registry: &mut R,
    address: &crate::identity::Address,
    genesis: &Genesis,
) -> Result<JoinedStoa, PublishError> {
    // Re-derive and compare. `matches` is `is_ok_and`, so a record too long to
    // encode matches nothing rather than erroring — which is the correct answer
    // here too: it has no address, so it cannot be the one this address names.
    if !genesis.matches(address) {
        return Err(PublishError::Genesis(GenesisError::TrailingBytes));
    }
    // Idempotent, and the relation of an existing row is preserved: a peer that
    // CREATED this Stoa and later pastes its own address stays "created". That is
    // `remember_stoa`'s first-wins rule doing the work, and it is the one thing
    // the rule is actually load-bearing for here.
    registry.remember_stoa(genesis, Relation::Joined)?;
    // Read back rather than returning what was passed in, so the reply reports
    // the relation the STORE holds rather than the one this call proposed.
    registry
        .get_stoa(address)?
        .ok_or(PublishError::Storage(OpLogError::CorruptEntry(
            "a Stoa just remembered could not be read back".to_string(),
        )))
}

/// The genesis record for a Stoa this peer reads, or [`PublishError::NotJoined`].
///
/// **The outbound authorisation gate, as one function.** CLAUDE.md: "A guard is a
/// job. Keep it separate, so 'is it called everywhere?' stays a question with an
/// answer." Every publishing function below opens with this call, and that is
/// checkable by reading five call sites rather than by auditing five inlined
/// lookups.
///
/// It returns the record rather than a bool, which is what makes it awkward to
/// skip: a caller needs the record anyway — to know the Stoa's address is real —
/// so the natural way to get it is through the check.
fn joined_stoa<R: StoaRegistry>(
    registry: &R,
    stoa: &crate::identity::Address,
) -> Result<JoinedStoa, PublishError> {
    registry.get_stoa(stoa)?.ok_or(PublishError::NotJoined)
}

/// Check a body and an attachment list against the encode-side bounds.
///
/// Its own function for the reason [`joined_stoa`] is: three call sites share it,
/// and the fourth would otherwise spell it slightly differently. Returns `()` —
/// there is nothing to hand back, only a refusal to make.
fn check_content(body: &str, attachments: &[String]) -> Result<(), PublishError> {
    if body.len() > MAX_BODY_BYTES {
        return Err(PublishError::BodyTooLong {
            bytes: body.len(),
            max: MAX_BODY_BYTES,
        });
    }
    if attachments.len() > MAX_ATTACHMENTS {
        return Err(PublishError::TooManyAttachments {
            count: attachments.len(),
            max: MAX_ATTACHMENTS,
        });
    }
    for a in attachments {
        if a.len() > MAX_ATTACHMENT_BYTES {
            return Err(PublishError::AttachmentTooLong {
                bytes: a.len(),
                max: MAX_ATTACHMENT_BYTES,
            });
        }
    }
    Ok(())
}

/// Sign an op and store it, returning what was stored.
///
/// The single place an op is signed and appended, so "is every published op
/// stored?" and "is every stored op signed with the caller's key?" are each one
/// question with one answer.
///
/// # The arrival is `unordered`, and that is a recording of fact
///
/// [`crate::arrival::Arrival::unordered`] is what a locally-created op honestly
/// carries: no transport has seen it, so no transport has ordered it. Inventing a
/// Lamport value here is the one thing `arrival.rs` spends a section forbidding —
/// a locally-minted clock cannot agree with SDS's, and two peers would order the
/// same ops differently with no error anywhere.
///
/// The consequence is worth stating plainly: an op this peer wrote sorts in the
/// degraded branch, by ascending op id, alongside every other op today. That is
/// not a defect of this function; it is the same gap the read path already has.
fn sign_and_store<L: OpLog>(
    log: &mut L,
    op: Op,
    key: &SecretKey,
) -> Result<SignedOp, PublishError> {
    let signed = op.sign(key);
    log.append(signed.clone(), crate::arrival::Arrival::unordered())?;
    Ok(signed)
}

/// Publish a top-level post.
///
/// `thread` is `None` and `parent` is `None`. `op.rs` explains the first: a
/// thread-opening post is its own thread's root, and it cannot name its own id at
/// signing time because that id is the hash of the bytes being signed. The feed
/// reads a thread head as "a `Post` whose parent is `None`", so the two fields
/// agree with what the reader looks for.
pub fn create_post<S: OpLog + StoaRegistry>(
    store: &mut S,
    key: &SecretKey,
    stoa: &crate::identity::Address,
    body: &str,
    attachments: &[String],
) -> Result<SignedOp, PublishError> {
    joined_stoa(store, stoa)?;
    check_content(body, attachments)?;

    let op = Op {
        stoa: *stoa,
        author: key.public_key(),
        kind: OpKind::Post {
            thread: None,
            parent: None,
            body: body.to_string(),
            attachments: attachments.to_vec(),
        },
    };
    sign_and_store(store, op, key)
}

/// Publish a reply.
///
/// # A reply is a `Post` with a parent, and there is no `Reply` kind
///
/// §4.1 puts `threadId` and `parentPostId` in the payload, and `op.rs` declines a
/// separate kind so that "is this a reply?" stays one question. This function is
/// therefore [`create_post`] plus two resolved fields and three checks.
///
/// # The thread is DERIVED, never taken from the caller
///
/// A caller supplies the parent; the thread is whatever the parent's thread is,
/// or the parent itself when the parent is a thread root. Taking it as a
/// parameter would let a caller file a reply under a thread it does not belong to
/// — an op that is validly signed, stores fine, and renders in the wrong thread
/// on every peer. Deriving it makes that unrepresentable rather than checked.
pub fn create_reply<S: OpLog + StoaRegistry>(
    store: &mut S,
    key: &SecretKey,
    stoa: &crate::identity::Address,
    parent: &OpId,
    body: &str,
    attachments: &[String],
) -> Result<SignedOp, PublishError> {
    joined_stoa(store, stoa)?;
    check_content(body, attachments)?;

    let parent_entry = postable_target(store, stoa, parent)?;
    // The parent's thread if it has one, else the parent itself — which is the
    // thread-root case, where the root's own id IS the thread id.
    let thread = match &parent_entry.op.op.kind {
        OpKind::Post { thread, .. } => thread.unwrap_or(*parent),
        // Unreachable: `postable_target` already refused every non-`Post` kind.
        // Written as a returned error rather than an `unreachable!` because a
        // panic here aborts the module process (PHASE0-FINDINGS §3), and an
        // "unreachable" that is reached is exactly when that matters.
        _ => return Err(PublishError::TargetNotAPost),
    };

    let op = Op {
        stoa: *stoa,
        author: key.public_key(),
        kind: OpKind::Post {
            thread: Some(thread),
            parent: Some(*parent),
            body: body.to_string(),
            attachments: attachments.to_vec(),
        },
    };
    sign_and_store(store, op, key)
}

/// Publish a vote.
///
/// # Nothing reads this yet, and the method exists anyway
///
/// PLAN.md §9.1 lists `vote` among the methods **deliberately not proposed**, on
/// the reasoning that "a method publishing an op with no observable effect is a
/// method that will be called and then explained away". That objection is real
/// and is not resolved by this function existing — §7.2 rule 2 ships no score, so
/// a vote published today changes nothing any reader can see.
///
/// It is here because the MVP scope asks for it, and because §13's own answer to
/// "are votes an op at all" points the other way: "v1 either collects vote ops
/// nothing reads or has no votes. Collecting them early is cheap and makes the
/// history available when scoring lands; deciding by accident is not."
///
/// **The obligation that follows is the view's, and it is not optional:** a vote
/// control must not imply an effect it does not have. The UI brief's vote section
/// is where that lands. A peer storing vote ops and a UI showing a score are two
/// different claims, and only the first is true today.
pub fn create_vote<S: OpLog + StoaRegistry>(
    store: &mut S,
    key: &SecretKey,
    stoa: &crate::identity::Address,
    target: &OpId,
    direction: VoteDirection,
) -> Result<SignedOp, PublishError> {
    joined_stoa(store, stoa)?;
    // Same gate as a reply: a vote on an op this peer does not hold, or holds in
    // another Stoa, or that is not a post, is a vote nothing will ever resolve.
    postable_target(store, stoa, target)?;

    let op = Op {
        stoa: *stoa,
        author: key.public_key(),
        kind: OpKind::Vote {
            target: *target,
            direction,
        },
    };
    sign_and_store(store, op, key)
}

/// The entry an op may reply to or vote on, or the reason it may not.
///
/// Three refusals in one place, because they are one question — "is this a thing
/// I can address?" — and because a caller that asked them separately would
/// eventually ask two of the three.
///
/// **It verifies the target's signature.** The store holds junk by design (§3.3),
/// so a target that does not verify is one the feed will never render — replying
/// to it produces a reply under an invisible parent. This is the read-path rule
/// applied to a read that the write path performs.
fn postable_target<L: OpLog>(
    log: &L,
    stoa: &crate::identity::Address,
    target: &OpId,
) -> Result<crate::log::Entry, PublishError> {
    let entry = log.get(target)?.ok_or(PublishError::TargetNotFound)?;

    // Authenticity before any field is read, for `feed.rs`'s reason: every field
    // is a claim until the signature is checked, and `stoa` below is a field.
    if !entry.op.verify() {
        return Err(PublishError::TargetNotFound);
    }
    if &entry.op.op.stoa != stoa {
        return Err(PublishError::TargetInAnotherStoa);
    }
    if !matches!(entry.op.op.kind, OpKind::Post { .. }) {
        return Err(PublishError::TargetNotAPost);
    }
    Ok(entry)
}

// ─── Reading a thread ─────────────────────────────────────────────────────

/// One post in a thread, resolved and ready to render.
///
/// Mirrors [`crate::feed::FeedRow`] rather than inventing a second shape, with
/// the two fields a thread needs and a feed does not: `parent`, because a thread
/// is a tree, and the moderation state, because §9.1's `getThread` carries it
/// where the feed only filters on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadPost {
    /// This post's op id — the thing a reply names as its parent.
    pub post: String,
    /// The version being rendered, which differs from `post` once revised.
    pub current_version: String,
    /// The parent post's id, absent for the thread root.
    ///
    /// `Option` and not an empty string: "this is the root" and "this replies to
    /// a post whose id is empty" are different facts, and an empty string can
    /// only express one of them badly.
    pub parent: Option<String>,
    pub author: String,
    pub body: crate::sanitise::Sanitised,
    pub attachments: Vec<crate::sanitise::Sanitised>,
    pub is_revised: bool,
    /// Whether a moderator hid this post, and which op decided it.
    ///
    /// Carried rather than filtered on, because §9.1 requires it: "a reader who
    /// asked to see what was hidden is owed the knowledge of which ones those
    /// were."
    pub is_hidden: bool,
}

/// One page of a thread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadPage {
    pub items: Vec<ThreadPost>,
    pub page: usize,
    pub has_more: bool,
}

/// A thread: its root post and the replies to it, resolved.
///
/// # Why this is not `iter_target`
///
/// PLAN.md §9.1 names this as a gap and says exactly why: "`iter_stoa` returns
/// every op in a Stoa; `iter_target` returns the ops acting on one op. Neither
/// answers 'the posts whose `thread` is T', which is what a thread view is."
///
/// A reply does not *act on* its parent — `Entry::target` returns `None` for a
/// `Post` precisely so that a target read does not conflate replies with
/// moderations of the same post. So a thread read cannot be a target read, and
/// this filters [`OpLog::iter_stoa`] on the `thread` field instead.
///
/// # The root is included, and is found by id rather than by its thread field
///
/// A thread-opening post carries `thread: None` (it cannot name its own id at
/// signing time), so filtering on `thread == Some(T)` finds every reply and never
/// the root. The root is fetched separately and placed first — which is also the
/// only position for it that does not depend on the ordering rule, since
/// `cmp_ops`'s degraded branch is by op id and would scatter it.
///
/// # Ordering, and what it does not claim
///
/// Replies come back in [`OpLog::iter_stoa`] order, which is
/// [`crate::arrival::cmp_ops`] — today the degraded branch, **ascending op id**,
/// which carries no recency whatever. This is not chronological, and a view
/// presenting it as "oldest first" would be asserting something the data does not
/// support. No comparison is written here, for `feed.rs`'s reason: a second
/// expression of the order could disagree with the first.
pub fn read_thread<L: OpLog>(
    log: &L,
    moderators: &crate::moderation::Moderators,
    stoa: &crate::identity::Address,
    thread: &OpId,
    page: usize,
    per_page: usize,
    include_hidden: bool,
) -> Result<ThreadPage, OpLogError> {
    let mut rows = Vec::new();

    // The root first, by id. Its absence is an empty thread rather than an error:
    // §3.3 makes a peer holding a partial op set the normal case, so a thread
    // whose root has not arrived is a thread this peer cannot show yet.
    if let Some(root) = log.get(thread)? {
        if root.op.verify() && &root.op.op.stoa == stoa {
            if let Some(row) = resolve_post(log, moderators, &root, include_hidden)? {
                rows.push(row);
            }
        }
    }

    for entry in log.iter_stoa(stoa)? {
        // Authenticity before any field is read — `feed.rs`'s rule, and `thread`
        // below is a field.
        if !entry.op.verify() {
            continue;
        }
        // Replies to this thread: a `Post` whose `thread` names it. The root is
        // excluded here by construction rather than by an id comparison, because
        // the root's `thread` is `None` and never `Some(its own id)`.
        match &entry.op.op.kind {
            OpKind::Post {
                thread: Some(t), ..
            } if t == thread => {}
            _ => continue,
        }
        if let Some(row) = resolve_post(log, moderators, &entry, include_hidden)? {
            rows.push(row);
        }
    }

    let start = page.saturating_mul(per_page).min(rows.len());
    let end = start.saturating_add(per_page).min(rows.len());
    let has_more = end < rows.len();

    Ok(ThreadPage {
        items: rows[start..end].to_vec(),
        page,
        has_more,
    })
}

/// Resolve one post into a row, or drop it.
///
/// Shared by the root and the replies so that a root and a reply cannot be
/// resolved by two slightly different pieces of code — which is how a moderation
/// check ends up applying to replies and not to the post they hang under.
///
/// `Ok(None)` means "filtered out", which today is only the hidden-and-not-asked
/// case. It is not an error: §3.3's partial-set case means a resolver finding
/// nothing found nothing.
fn resolve_post<L: OpLog>(
    log: &L,
    moderators: &crate::moderation::Moderators,
    entry: &crate::log::Entry,
    include_hidden: bool,
) -> Result<Option<ThreadPost>, OpLogError> {
    let id = entry.id();

    let Some(version) = crate::revision::current_version(log, &id)? else {
        return Ok(None);
    };

    // Resolved on READ, every time, from the genesis record (§6.2). Never cached
    // and never decided at append.
    let moderation = crate::moderation::resolve(log, moderators, &id)?;
    let is_hidden = moderation.is_hidden();
    if is_hidden && !include_hidden {
        return Ok(None);
    }

    let parent = match &entry.op.op.kind {
        OpKind::Post { parent, .. } => parent.map(|p| p.to_hex()),
        // Only a `Post` reaches here — both callers filter on the kind — but a
        // returned `None` beats a panic for PHASE0-FINDINGS §3's reason.
        _ => return Ok(None),
    };

    Ok(Some(ThreadPost {
        post: id.to_hex(),
        current_version: version.current.id().to_hex(),
        parent,
        author: entry.op.op.author.address().to_hex(),
        body: crate::sanitise::sanitise(version.body()),
        attachments: version
            .attachments()
            .iter()
            .map(|a| crate::sanitise::sanitise(a))
            .collect(),
        is_revised: version.is_revised(),
        is_hidden,
    }))
}

/// Whether an append stored something new.
///
/// A tiny helper, and it exists because `Appended` is the op log's vocabulary and
/// a publish reply is not the place to leak it. A caller wants "was this new?",
/// which is a bool.
pub fn was_stored(appended: Appended) -> bool {
    matches!(appended, Appended::Stored)
}
