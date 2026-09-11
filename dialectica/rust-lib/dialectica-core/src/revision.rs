//! Which version of a post is current.
//!
//! # Why this file exists
//!
//! PLAN.md §5.7: "**A post is never edited in place. An edit is a new version of
//! that post, published and signed by the same author.**" Three properties
//! follow, and until this module nothing implemented any of them:
//!
//! - **Authorship decides validity.** "A version signed by anyone other than the
//!   post's original author is invalid and dropped on read (§3.3)."
//! - **Lamport order decides currency.** "Among an author's own versions, the
//!   highest Lamport timestamp is current, ties broken by ascending message id."
//! - **History is kept.** "Superseded versions stay in the op log."
//!
//! [`log`](crate::log) holds the ops and orders them, [`op::SignedOp::verify`]
//! says whether one is authentic, and [`arrival::cmp_ops`](crate::arrival::cmp_ops)
//! says which of two the ordering rule places first. This is the one place that
//! needs all three at once.
//!
//! # The authorship check is this module's, and nowhere else's
//!
//! `op.rs` is explicit that it cannot make it, and pins the gap with a test:
//!
//! > **This answers authenticity only.** It does not ask [...] whether a
//! > revision's author owns the post it supersedes (§5.7) [...] Each of those
//! > needs state this type does not have.
//!
//! The state it lacks is the *target op*. A [`SignedOp`] holds an author field
//! and a signature over its own bytes, which together establish who sent it —
//! and say nothing whatever about who owns the post it names. Deciding that
//! needs the original post, which means it needs the log, which is why the check
//! lives here and not one layer down.
//!
//! Without it, any peer may rewrite any post: publishing an op takes no
//! permission, and a peer's signature over their own op is always valid. So
//! `op.rs`'s `a_revision_by_a_different_author_is_authentic_and_still_not_valid`
//! is a description of an open hole, and [`current_version`] is what closes it.
//!
//! # Verify first, then compare authors — the order is load-bearing
//!
//! An op's `author` field is a **claim**. [`SignedOp::verify`] is what turns it
//! into a fact, by re-deriving the address from the key that actually signed
//! (`identity::verify_authored_op`) rather than taking the field's word.
//!
//! Compare authors before verifying and the check compares one attacker-supplied
//! string against another: an attacker writes the victim's public key into
//! `author`, signs with their own key, and the comparison passes. The signature
//! is the only thing that distinguishes "Alice's key is written here" from
//! "Alice sent this", and the comparison is worthless until it has been made.
//!
//! Both checks are here, in one expression, in that order.
//!
//! # No ordering rule of its own
//!
//! [`OpLog::iter_target`] already returns entries in
//! [`cmp_ops`](crate::arrival::cmp_ops) order, so §5.7's "highest Lamport
//! timestamp is current, ties broken by ascending message id" is a `find` over
//! that sequence and **not a comparison written here**.
//!
//! That is a correctness property rather than a convenience. A second
//! implementation of the ordering rule could disagree with the first, and two
//! orders that disagree produce no error — each peer stays internally consistent
//! and renders the post differently from its neighbour. There is no `sort`, no
//! `max_by`, and no reference to `lamport()` in this module; the only ordering
//! input is the position the log already put the entry in.
//!
//! ## The first entry is the current one, which is NOT the same as the newest
//!
//! Worth stating plainly, because `iter_target`'s documentation once said
//! "most recent first" and pointed two resolvers at the same error.
//!
//! `cmp_ops` leads with the highest Lamport timestamp **only when the transport
//! supplied one**. Nothing supplies one today, so every arrival is
//! `Arrival::unordered()` and the comparison falls back to *ascending op id* —
//! and an op id is a hash of the op's own bytes, carrying no recency whatever.
//!
//! So under today's order, the current version is a **convergent arbitrary
//! choice, not a temporal one**. That is still a real and valuable property, and
//! it is the one §3.3 actually needs: two peers holding the same revisions agree
//! on which is current, even though neither can say which was written last. What
//! is bought is convergence; accuracy about time is not available at all until
//! the upstream fix lands.
//!
//! This module takes the first entry because that is **the position the ordering
//! rule defines as current**, never because the log promises recency. That is
//! also why it is correct under both regimes: when Lamport values start
//! arriving, the order under this `find` becomes temporal and this code does not
//! change.
//!
//! # Every version names the original post
//!
//! A `Revise` names the **post**, never another `Revise`. §5.7 says "an edit is a
//! new version of *that post*" — one named subject, not a chain — and the change's
//! `design.md` carries the argument for why the flat reading is the correct one
//! rather than merely a permitted one. The short form: a chain is not resolvable
//! over a partial set, and forks with no defined answer.
//!
//! Nothing here walks a chain, so there is no recursion, no depth limit, and no
//! cycle to detect: one `iter_target` call is the whole search, and an op naming
//! itself is simply an op whose `find` finds nothing of the right author.
//!
//! # A partial set is the normal case
//!
//! §3.3: two peers "routinely hold **different sets of ops**". A peer holding
//! fewer revisions than its neighbour resolves over the ones it has, which is
//! the right answer for that peer rather than a defect.
//!
//! What holds in **both** regimes is that the answer is a pure function of the
//! ops held: two peers with the same set agree, whatever sequence they arrived
//! in. What does **not** hold under today's degraded order is that the answer
//! only advances — a revision arriving late with a lower op id becomes current,
//! because op id carries no recency. Under a transport-supplied order it would
//! advance monotonically. Neither is a defect; they are the two regimes, and the
//! flat model (every version naming the original) is what keeps a late arrival a
//! *re-resolution* rather than a jump onto a different branch.

use crate::log::{Entry, OpLog};
use crate::op::{OpId, OpKind};

/// A post's current version, and the post it is a version of.
///
/// Both are carried because a caller needs both and deriving one from the other
/// is not possible in either direction. §5.7's "history is kept" exists so that
/// "the UI can show that a post was edited, and a moderator acting on a post is
/// acting on a version they can name" — showing *that* it was edited needs the
/// original's identity, and naming the version needs the version's. A type that
/// returned only the current entry would force every caller wanting the other to
/// re-read the log and re-derive it.
///
/// The two are the same entry when a post has no valid revision, which is the
/// ordinary case for most posts and is not a special outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentVersion<'a> {
    /// The post as first published. Always an [`OpKind::Post`].
    pub original: &'a Entry,
    /// The version to render: the valid revision the ordering rule places
    /// first, or `original` itself when there is none.
    ///
    /// "First" is not "newest" — see this module's documentation. Under the
    /// order production actually runs today it is a convergent arbitrary choice
    /// rather than a temporal one.
    pub current: &'a Entry,
}

impl CurrentVersion<'_> {
    /// The body a reader should render.
    ///
    /// Written once here rather than at each call site, because pulling the body
    /// out of `current` means matching on its kind — and `current` is a `Post`
    /// or a `Revise` depending on whether the post was ever edited, which is
    /// exactly the branch a caller should not have to get right. A caller that
    /// spelled it itself would eventually spell it as "match Post, else empty"
    /// and silently render every edited post blank.
    pub fn body(&self) -> &str {
        // An EMPTY body is a legitimate edit — a user clearing their post — and
        // never "the revision said nothing". No fallback to `original`; see
        // `an_empty_revision_body_is_an_edit_and_not_an_absence`, and
        // `attachments` below for the same rule with sharper consequences.
        match &self.current.op.op.kind {
            OpKind::Post { body, .. } => body,
            OpKind::Revise { body, .. } => body,
            // Unreachable by construction: `current` is either `original`, which
            // `current_version` establishes is a `Post`, or an entry the `find`
            // matched as a `Revise`. Answered rather than panicked because this
            // module runs on attacker-supplied content and a panic aborts the
            // module process (PHASE0-FINDINGS §3) — an empty body is a rendering,
            // a crash is a denial of service.
            _ => "",
        }
    }

    /// The attachments a reader should resolve (§4.6).
    ///
    /// Same reasoning as [`body`](Self::body): one match, in one place.
    pub fn attachments(&self) -> &[String] {
        // An EMPTY list is "this version has no attachments", never "this
        // version said nothing about them". There is no fallback to `original`
        // here and there must not be: §4.6 makes these Logos Storage CIDs, so a
        // resolver that treated empty as unset would keep fetching an image the
        // author deleted. `a_revision_clearing_the_attachments_removes_them`
        // pins it; the same reasoning applies to `body` above.
        match &self.current.op.op.kind {
            OpKind::Post { attachments, .. } => attachments,
            OpKind::Revise { attachments, .. } => attachments,
            _ => &[],
        }
    }

    /// Whether the post has been revised since it was published.
    ///
    /// §5.7: "The UI can show that a post was edited." Compared by op id rather
    /// than by pointer, so it says what it means — two entries are the same
    /// version when they are the same op, which is the identity the whole log is
    /// keyed on.
    pub fn is_revised(&self) -> bool {
        self.current.id() != self.original.id()
    }
}

/// The version of `post` a reader should render, or absence.
///
/// §5.7's rule, applied to one post over the ops this peer happens to hold.
///
/// # What each outcome means
///
/// `None` is returned when the log holds no op with this id, **and** when it
/// holds one that is not an [`OpKind::Post`]. The two are one outcome
/// deliberately: only a post has versions, and a caller holding a vote's op id
/// has made a category mistake for which there is no rendering either way. Two
/// outcomes is what a caller can act on — there is something to render, or there
/// is not.
///
/// `Some` is returned otherwise, and it is never an error: a peer holding fewer
/// revisions than its neighbour resolves over the ones it has, and a peer
/// holding none resolves to the original. §3.3 makes different op sets the
/// normal case, so "not enough information" is not an outcome any peer could
/// distinguish from "this is all there is".
///
/// # What is dropped, and in what order
///
/// A candidate revision must be all four of: a [`OpKind::Revise`], naming this
/// post, **authentic**, and **by this post's author**. The log's `iter_target`
/// supplies the second; the two checks here supply the rest, in that order —
/// see this module's documentation for why verifying first is load-bearing
/// rather than tidy.
pub fn current_version<'a, L: OpLog>(log: &'a L, post: &OpId) -> Option<CurrentVersion<'a>> {
    let original = log.get(post)?;

    // Only a post has versions. A `Revise`, a `Vote` or a `Moderate` reaching
    // this point means the caller named something that has no body to replace,
    // and resolving it as though it did would let a revision naming a vote
    // substitute content into a reader's view of it.
    if !matches!(original.op.op.kind, OpKind::Post { .. }) {
        return None;
    }

    let current = log
        .iter_target(post)
        .into_iter()
        // `find`, not `max_by` or a sort: `iter_target` is already in `cmp_ops`
        // order, so the first match IS the one the ordering rule places first.
        // No ordering rule is written here.
        //
        // NOT "the most recent". `cmp_ops` leads with the highest Lamport
        // timestamp only on the transport-ordered branch, which production never
        // reaches today; the degraded branch is ascending op id and carries no
        // recency. See this module's documentation.
        .find(|entry| is_valid_revision(entry, original))
        .unwrap_or(original);

    Some(CurrentVersion { original, current })
}

/// Whether `candidate` is a version of `original` that a reader may trust.
///
/// Separate from [`current_version`] because it is a distinct job — "is this a
/// valid version?" is not "which version is current?" — and because separating
/// it is what makes the three conditions enumerable at one glance rather than
/// buried in a fold.
///
/// All three must hold, and the last two are in this order for the reason this
/// module's documentation gives: an unverified `author` field is a claim, and
/// comparing a claim against a fact passes for whoever wrote the claim.
fn is_valid_revision(candidate: &Entry, original: &Entry) -> bool {
    // It must be a revision. `iter_target` returns every kind naming this post —
    // a moderation, a vote — because "what acts on this subject?" is one
    // question; narrowing it is the reader's job.
    matches!(candidate.op.op.kind, OpKind::Revise { .. })
        // It must be authentic. §3.3: the store holds junk and the reader never
        // trusts it, so this is not redundant with anything the log did — the
        // log deliberately verifies nothing on append.
        && candidate.op.verify()
        // And it must be the post's author's. THE §5.7 rule: "A version signed by
        // anyone other than the post's original author is invalid and dropped on
        // read." Without this line any peer can rewrite any post.
        && candidate.op.op.author == original.op.op.author
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrival::{Arrival, MessageId};
    use crate::identity::{sign_op_bytes, Address, PublicKey, SecretKey};
    use crate::log::MemoryOpLog;
    use crate::op::{ModerationAction, Op, SignedOp, VoteDirection};
    use crate::stoa::{Genesis, Policy};

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    /// The post's author throughout. Every fixture that is *supposed* to
    /// succeed signs with this key.
    fn author() -> SecretKey {
        a_key(2)
    }

    /// Someone who is not the post's author. Every forgery fixture uses this.
    fn stranger() -> SecretKey {
        a_key(9)
    }

    fn a_stoa() -> Address {
        Genesis {
            creator: a_key(1).public_key(),
            policy: Policy::Open,
            title: "Agora".to_string(),
        }
        .address()
        .expect("a short fixture title is always under the genesis title cap")
    }

    fn a_message_id(seed: u8) -> MessageId {
        MessageId::new(vec![seed; 32])
    }

    /// A post by [`author`].
    fn a_post(body: &str) -> SignedOp {
        Op {
            stoa: a_stoa(),
            author: author().public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(&author())
    }

    /// A revision of `target`, signed by `key` and claiming `key` as its author.
    ///
    /// Authentic by construction: the author field and the signing key agree, so
    /// `verify()` passes. What it is NOT is authorised — that is what the
    /// resolver decides, and passing a key other than the post author's is how
    /// the stranger cases are built.
    fn a_revision_by(key: &SecretKey, target: OpId, body: &str) -> SignedOp {
        Op {
            stoa: a_stoa(),
            author: key.public_key(),
            kind: OpKind::Revise {
                target,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(key)
    }

    /// A revision of `target`, signed by [`author`].
    fn a_revision(target: OpId, body: &str) -> SignedOp {
        a_revision_by(&author(), target, body)
    }

    /// A revision claiming to be from `claimed` but signed by `signer`.
    ///
    /// THE forgery this module exists to stop: the author field names the
    /// victim, so an authorship check performed before verification passes.
    /// Asserted to be a genuine forgery at every use, or the test proves nothing.
    fn a_forged_revision(
        claimed: &PublicKey,
        signer: &SecretKey,
        target: OpId,
        body: &str,
    ) -> SignedOp {
        let op = Op {
            stoa: a_stoa(),
            author: claimed.clone(),
            kind: OpKind::Revise {
                target,
                body: body.to_string(),
                attachments: vec![],
            },
        };
        SignedOp {
            signature: sign_op_bytes(signer, &op.canonical_bytes()),
            op,
        }
    }

    /// Two revisions of `target` whose op ids are known to differ, lower first.
    ///
    /// Determined rather than assumed, following `arrival.rs` and `log.rs`: the
    /// ids are hashes, and hardcoding the wrong guess would make a
    /// degraded-order test pass for the wrong reason.
    fn two_revisions_by_ascending_id(target: OpId) -> (SignedOp, SignedOp) {
        let (one, two) = (a_revision(target, "alpha"), a_revision(target, "beta"));
        assert_ne!(
            one.op.id(),
            two.op.id(),
            "the fixture needs two distinct ops"
        );
        if one.op.id() < two.op.id() {
            (one, two)
        } else {
            (two, one)
        }
    }

    // ─── The original stands when nothing supersedes it ───────────────────

    #[test]
    fn a_post_with_no_revisions_is_its_own_current_version() {
        let post = a_post("as published");
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post.clone(), Arrival::unordered());

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.original.id(), id);
        assert_eq!(resolved.current.id(), id);
        assert_eq!(resolved.body(), "as published");
        assert!(!resolved.is_revised());
    }

    #[test]
    fn a_post_the_log_does_not_hold_resolves_to_absence() {
        // §3.3: absence is a defined answer, not an error. A peer may simply not
        // have received the post yet.
        let log = MemoryOpLog::new();
        assert!(current_version(&log, &a_post("never received").op.id()).is_none());
    }

    #[test]
    fn a_revision_whose_target_is_absent_does_not_become_a_post() {
        // The dangling-reference case, which is ordinary: a revision can arrive
        // before the post it revises. It must not be resolved as the current
        // version of a post this peer does not hold — there would be no original
        // to have checked its authorship against.
        let post = a_post("never received");
        let target = post.op.id();
        let revision = a_revision(target, "revises something we lack");

        let mut log = MemoryOpLog::new();
        log.append(revision.clone(), Arrival::unordered());

        assert!(
            current_version(&log, &target).is_none(),
            "a revision must not stand in for the post it revises"
        );
        // And the revision is genuinely in the log, so this is not passing
        // because nothing was stored.
        assert!(log.get(&revision.op.id()).is_some());
    }

    // ─── Authorship decides validity ──────────────────────────────────────

    #[test]
    fn a_revision_by_a_stranger_is_dropped() {
        // THE security property of the whole revision design. §5.7: "A version
        // signed by anyone other than the post's original author is invalid and
        // dropped on read." Without it, any peer rewrites any post.
        let post = a_post("mine");
        let id = post.op.id();
        let intruder = a_revision_by(&stranger(), id, "not yours to edit");

        // The intruder's op is AUTHENTIC — it really is from them. That is
        // exactly what makes this test worth having: `verify()` alone accepts it,
        // and only the authorship comparison rejects it.
        assert!(
            intruder.verify(),
            "the fixture must be authentic, not forged"
        );
        assert_ne!(intruder.op.author, post.op.author);

        let mut log = MemoryOpLog::new();
        log.append(post.clone(), Arrival::ordered(1, a_message_id(1)));
        log.append(intruder, Arrival::ordered(2, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "mine");
        assert_eq!(resolved.current.id(), id);
        assert!(!resolved.is_revised());
    }

    #[test]
    fn a_strangers_revision_loses_to_an_older_one_by_the_author() {
        // The fixture that makes the two rules DISAGREE. Recency says the
        // stranger's version wins; authorship says the author's does. A resolver
        // that dropped the authorship check would return "hijacked" — so this
        // test can fail for the reason it names.
        let post = a_post("v1");
        let id = post.op.id();
        let mine = a_revision(id, "v2 by me");
        let theirs = a_revision_by(&stranger(), id, "hijacked");

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(mine.clone(), Arrival::ordered(2, a_message_id(1)));
        // The stranger's is strictly MORE recent, so it heads `iter_target`.
        log.append(theirs.clone(), Arrival::ordered(3, a_message_id(1)));

        // Confirm the fixture: the stranger's op really does lead the order, so
        // the resolver had to reject it rather than never having seen it.
        let ordered: Vec<OpId> = log.iter_target(&id).iter().map(|e| e.id()).collect();
        assert_eq!(
            ordered,
            vec![theirs.op.id(), mine.op.id()],
            "the fixture must put the stranger's revision first"
        );

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "v2 by me");
        assert_eq!(resolved.current.id(), mine.op.id());
    }

    #[test]
    fn a_strangers_revision_is_dropped_when_it_is_the_only_one() {
        let post = a_post("mine");
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::unordered());
        log.append(
            a_revision_by(&stranger(), id, "hijacked"),
            Arrival::ordered(9, a_message_id(1)),
        );

        assert_eq!(current_version(&log, &id).unwrap().body(), "mine");
    }

    #[test]
    fn every_key_but_the_authors_is_rejected() {
        // Not one stranger but several, so a resolver that happened to reject
        // this particular key — or compared only some bytes of it — is caught.
        let post = a_post("mine");
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(0, a_message_id(0)));

        for seed in [3u8, 4, 5, 9, 200, 255] {
            let key = a_key(seed);
            assert_ne!(key.public_key(), author().public_key());
            log.append(
                a_revision_by(&key, id, "hijacked"),
                Arrival::ordered(seed as u64 + 10, a_message_id(seed)),
            );
        }

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(
            resolved.body(),
            "mine",
            "a stranger's revision was accepted"
        );
        assert!(!resolved.is_revised());
    }

    // ─── Verification happens on read ─────────────────────────────────────

    #[test]
    fn a_forged_revision_is_dropped() {
        // The attack the check ORDER exists to stop: the author field names the
        // victim, so an authorship comparison made before verification passes.
        // Only `verify()` — which re-derives the address from the key that
        // actually signed — catches it.
        let post = a_post("mine");
        let id = post.op.id();
        let forged = a_forged_revision(&post.op.author, &stranger(), id, "forged");

        // The fixture must genuinely be a forgery, or this test proves nothing.
        assert!(!forged.verify(), "the fixture must be an actual forgery");
        // And it must pass an authorship check made on the claim alone — which
        // is precisely why verification has to come first.
        assert_eq!(forged.op.author, post.op.author);

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(forged, Arrival::ordered(2, a_message_id(1)));

        assert_eq!(current_version(&log, &id).unwrap().body(), "mine");
    }

    #[test]
    fn a_forgery_does_not_displace_a_genuine_older_revision() {
        // Two rules made to DISAGREE again: recency says the forgery wins,
        // verification says the genuine older version does.
        let post = a_post("v1");
        let id = post.op.id();
        let genuine = a_revision(id, "v2 by me");
        let forged = a_forged_revision(&post.op.author, &stranger(), id, "forged v3");
        assert!(!forged.verify());

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(genuine.clone(), Arrival::ordered(2, a_message_id(1)));
        log.append(forged.clone(), Arrival::ordered(3, a_message_id(1)));

        // The forgery really does lead the order.
        let ordered: Vec<OpId> = log.iter_target(&id).iter().map(|e| e.id()).collect();
        assert_eq!(ordered, vec![forged.op.id(), genuine.op.id()]);

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "v2 by me");
        assert_eq!(resolved.current.id(), genuine.op.id());
    }

    #[test]
    fn a_revision_with_a_garbage_signature_is_dropped() {
        // Not a valid signature over anything: the bytes are simply wrong. A
        // resolver that only compared authors would accept it.
        let post = a_post("mine");
        let id = post.op.id();
        let mut junk = a_revision(id, "garbage-signed");
        junk.signature = crate::identity::Signature::from_bytes(&[7u8; 64]).unwrap();
        assert!(!junk.verify());
        assert_eq!(junk.op.author, post.op.author, "the author still matches");

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(junk, Arrival::ordered(2, a_message_id(1)));

        assert_eq!(current_version(&log, &id).unwrap().body(), "mine");
    }

    #[test]
    fn a_revision_lifted_into_another_stoa_is_dropped() {
        // The Stoa is inside the signed bytes, so a revision replayed from
        // another Stoa's channel fails verification. Pinned here because the
        // resolver never looks at the Stoa itself — it relies entirely on
        // `verify()` to catch this, and that reliance should be tested.
        let post = a_post("mine");
        let id = post.op.id();
        let genuine = a_revision(id, "v2");
        let lifted = SignedOp {
            op: Op {
                stoa: crate::identity::stoa_address(b"a different stoa"),
                ..genuine.op.clone()
            },
            signature: genuine.signature.clone(),
        };
        assert!(!lifted.verify());

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(lifted, Arrival::ordered(2, a_message_id(1)));

        assert_eq!(current_version(&log, &id).unwrap().body(), "mine");
    }

    // ─── Lamport order decides currency ───────────────────────────────────

    #[test]
    fn the_highest_lamport_revision_is_current() {
        // §5.7: "the highest Lamport timestamp is current". Appended in an order
        // that is neither the answer nor its reverse, so a resolver returning
        // insertion order fails either way.
        let post = a_post("v1");
        let id = post.op.id();
        let v2 = a_revision(id, "v2");
        let v3 = a_revision(id, "v3");
        let v4 = a_revision(id, "v4");

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(v3.clone(), Arrival::ordered(3, a_message_id(1)));
        log.append(v2, Arrival::ordered(2, a_message_id(1)));
        log.append(v4.clone(), Arrival::ordered(4, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "v4");
        assert_eq!(resolved.current.id(), v4.op.id());
        assert!(resolved.is_revised());
        // The original is still reported, and is still the original.
        assert_eq!(resolved.original.id(), id);
        assert_eq!(resolved.original.op.op.kind, a_post("v1").op.kind);
        // And the superseded version is untouched in the log (§5.7: history is
        // kept), nameable by its own id.
        assert!(log.get(&v3.op.id()).is_some());
    }

    #[test]
    fn the_lamport_timestamp_decides_currency_against_op_id_order() {
        // The fixture trap this repo has paid for: a test where op-id order
        // happens to agree with Lamport order passes for a resolver that
        // consults no metadata at all.
        //
        // Here the LOWER op id carries the HIGHER Lamport value, so the two
        // rules DISAGREE and only the Lamport rule gives the asserted answer.
        let post = a_post("v1");
        let id = post.op.id();
        let (low_id, high_id) = two_revisions_by_ascending_id(id);

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(low_id.clone(), Arrival::ordered(9, a_message_id(1)));
        log.append(high_id.clone(), Arrival::ordered(8, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(
            resolved.current.id(),
            low_id.op.id(),
            "the higher Lamport value must win, against op-id order"
        );
    }

    #[test]
    fn a_lamport_tie_is_broken_by_ascending_message_id_against_op_id_order() {
        // §5.7's tiebreak, with the same disagreement construction: equal
        // Lamport values, and the op with the LOWER op id carries the HIGHER
        // message id. A resolver falling back to op id returns the other one.
        let post = a_post("v1");
        let id = post.op.id();
        let (low_id, high_id) = two_revisions_by_ascending_id(id);

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(low_id.clone(), Arrival::ordered(7, a_message_id(9)));
        log.append(high_id.clone(), Arrival::ordered(7, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(
            resolved.current.id(),
            high_id.op.id(),
            "the lower message id must win, against op-id order"
        );
    }

    #[test]
    fn under_the_degraded_order_the_lower_op_id_is_current() {
        // The ONLY order production uses today: nothing supplies a Lamport
        // value, so every arrival is `unordered()` and `cmp_ops` falls back to
        // ascending op id. Appended in the reverse of the answer, so insertion
        // order fails.
        //
        // Asserted against the op ids themselves rather than a hardcoded body:
        // which of two bodies hashes lower is not something to guess.
        let post = a_post("v1");
        let id = post.op.id();
        let (low_id, high_id) = two_revisions_by_ascending_id(id);

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::unordered());
        log.append(high_id.clone(), Arrival::unordered());
        log.append(low_id.clone(), Arrival::unordered());

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.current.id(), low_id.op.id());
    }

    #[test]
    fn an_ordered_revision_beats_an_unordered_one_whatever_its_op_id() {
        // The transition state the upstream fix produces: a peer's older ops are
        // unordered and new ones carry Lamport values. Lamport 0 is the sharp
        // case, and the ORDERED revision is given the HIGHER op id so a resolver
        // falling back to op id would pick the other one.
        let post = a_post("v1");
        let id = post.op.id();
        let (unordered, ordered_zero) = two_revisions_by_ascending_id(id);

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::unordered());
        log.append(unordered.clone(), Arrival::unordered());
        log.append(ordered_zero.clone(), Arrival::ordered(0, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(
            resolved.current.id(),
            ordered_zero.op.id(),
            "even Lamport 0 beats a revision the transport did not order"
        );
    }

    #[test]
    fn two_peers_holding_the_same_ops_resolve_to_the_same_version() {
        // §3.3's convergence property at the resolver. Two peers received the
        // same ops in opposite sequences; both must render the same version.
        let post = a_post("v1");
        let id = post.op.id();
        let ops = [
            (post, Arrival::ordered(1, a_message_id(1))),
            (a_revision(id, "v2"), Arrival::ordered(2, a_message_id(1))),
            (a_revision(id, "v3"), Arrival::ordered(2, a_message_id(0))),
            (a_revision(id, "v4"), Arrival::unordered()),
            (
                a_revision_by(&stranger(), id, "hijacked"),
                Arrival::ordered(99, a_message_id(1)),
            ),
        ];

        let mut forwards = MemoryOpLog::new();
        for (op, arrival) in ops.iter() {
            forwards.append(op.clone(), arrival.clone());
        }
        let mut backwards = MemoryOpLog::new();
        for (op, arrival) in ops.iter().rev() {
            backwards.append(op.clone(), arrival.clone());
        }
        assert_eq!(forwards.len(), 5, "the fixture must exercise all five");

        let a = current_version(&forwards, &id).unwrap();
        let b = current_version(&backwards, &id).unwrap();
        assert_eq!(a.current.id(), b.current.id());
        assert_eq!(a.body(), b.body());
        // And the stranger's — which leads both logs' order — is not the answer.
        assert_ne!(a.body(), "hijacked");
    }

    // ─── A partial set is the normal case ─────────────────────────────────

    #[test]
    fn under_a_transport_order_a_peer_missing_the_newest_revision_resolves_to_an_older_one() {
        // §3.3: two peers "routinely hold different sets of ops". The peer
        // without v3 resolving to v2 is the CORRECT answer for that peer, not a
        // degraded one — and neither peer reports anything unusual.
        //
        // TRANSPORT-ORDERED, and the name says so, because the observable
        // claim here is regime-specific: that the peer holding MORE revisions
        // resolves to a LATER one. That is only true where the transport
        // supplied an order. Under the degraded order the extra revision may
        // sort below the one both peers hold, and the two peers then resolve
        // IDENTICALLY — holding more becomes unobservable. The counterpart
        // below asserts what survives that.
        let post = a_post("v1");
        let id = post.op.id();
        let v2 = a_revision(id, "v2");
        let v3 = a_revision(id, "v3");

        let mut complete = MemoryOpLog::new();
        complete.append(post.clone(), Arrival::ordered(1, a_message_id(1)));
        complete.append(v2.clone(), Arrival::ordered(2, a_message_id(1)));
        complete.append(v3.clone(), Arrival::ordered(3, a_message_id(1)));

        let mut behind = MemoryOpLog::new();
        behind.append(post, Arrival::ordered(1, a_message_id(1)));
        behind.append(v2.clone(), Arrival::ordered(2, a_message_id(1)));

        assert_eq!(current_version(&complete, &id).unwrap().body(), "v3");
        assert_eq!(current_version(&behind, &id).unwrap().body(), "v2");
        // Both are revised, and neither is an error.
        assert!(current_version(&behind, &id).unwrap().is_revised());
    }

    #[test]
    fn under_the_degraded_order_a_peer_holding_more_may_resolve_identically() {
        // The counterpart, and the regime production ACTUALLY RUNS.
        //
        // The fixture is built with `two_revisions_by_ascending_id` precisely so
        // the hashes are CONTROLLED rather than lucky: the peer that is behind
        // holds the LOWER op id, which the degraded order places first. So the
        // peer holding an extra revision resolves to the same version as the
        // peer without it — "holding more" is simply not observable here.
        //
        // That is not a defect, and pinning it is the point: the partial-set
        // guarantee is NOT "more ops means a later version". It is the weaker,
        // regime-neutral pair of claims asserted below — every peer gets a
        // defined answer with no error, and the answer is a function of the set
        // held rather than of arrival sequence. A reader who took the
        // transport-ordered test above as the general rule would be relying on
        // something the degraded order does not provide.
        let post = a_post("v1");
        let id = post.op.id();
        let (low_id, high_id) = two_revisions_by_ascending_id(id);

        let mut complete = MemoryOpLog::new();
        complete.append(post.clone(), Arrival::unordered());
        complete.append(low_id.clone(), Arrival::unordered());
        complete.append(high_id.clone(), Arrival::unordered());

        let mut behind = MemoryOpLog::new();
        behind.append(post, Arrival::unordered());
        behind.append(low_id.clone(), Arrival::unordered());

        let ahead = current_version(&complete, &id).unwrap();
        let back = current_version(&behind, &id).unwrap();

        // Both answer, neither errors — the regime-neutral half.
        assert!(ahead.is_revised());
        assert!(back.is_revised());
        // And here the extra revision changes nothing, because the op the peer
        // that is behind already holds sorts first.
        assert_eq!(ahead.current.id(), low_id.op.id());
        assert_eq!(back.current.id(), low_id.op.id());
        assert_eq!(
            ahead.current.id(),
            back.current.id(),
            "under the degraded order, holding more need not change the answer"
        );

        // The answer is a function of the SET held, not of arrival sequence:
        // the same two ops appended the other way round agree with `complete`.
        let mut reversed = MemoryOpLog::new();
        reversed.append(high_id.clone(), Arrival::unordered());
        reversed.append(low_id, Arrival::unordered());
        reversed.append(a_post("v1"), Arrival::unordered());
        assert_eq!(
            current_version(&reversed, &id).unwrap().current.id(),
            ahead.current.id()
        );
    }

    #[test]
    fn under_a_transport_order_the_answer_only_moves_forward() {
        // Monotonicity holds on the TRANSPORT-ORDERED branch, which this
        // fixture uses and which production does not reach today. Adding ops
        // advances the current version and a late-arriving OLDER one does not
        // retract it. Hardcoded expected sequence of bodies as each op lands.
        //
        // Its degraded-order counterpart is the next test, and the two
        // deliberately give OPPOSITE answers — monotonicity is a property of the
        // regime, not of this resolver.
        let post = a_post("v1");
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        assert_eq!(current_version(&log, &id).unwrap().body(), "v1");

        log.append(a_revision(id, "v2"), Arrival::ordered(2, a_message_id(1)));
        assert_eq!(current_version(&log, &id).unwrap().body(), "v2");

        log.append(a_revision(id, "v3"), Arrival::ordered(3, a_message_id(1)));
        assert_eq!(current_version(&log, &id).unwrap().body(), "v3");

        // An OLDER revision arriving late does not move the answer backwards.
        log.append(
            a_revision(id, "late v0"),
            Arrival::ordered(0, a_message_id(1)),
        );
        assert_eq!(current_version(&log, &id).unwrap().body(), "v3");
    }

    #[test]
    fn under_the_degraded_order_a_late_arrival_can_change_the_answer() {
        // The counterpart, and the regime production ACTUALLY RUNS. Ascending
        // op id carries no recency, so a revision arriving later with a lower op
        // id becomes current — the answer is re-resolved, not advanced.
        //
        // Pinning this is the point: monotonicity is a property of the
        // transport-ordered regime, and a reader who assumed it held everywhere
        // would be reasoning about a guarantee the degraded order does not make.
        // What DOES hold in both regimes is convergence, which
        // `two_peers_holding_the_same_ops_resolve_to_the_same_version` pins.
        let post = a_post("v1");
        let id = post.op.id();
        let (low_id, high_id) = two_revisions_by_ascending_id(id);

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::unordered());
        // The HIGHER op id arrives first and is current while it is alone.
        log.append(high_id.clone(), Arrival::unordered());
        assert_eq!(
            current_version(&log, &id).unwrap().current.id(),
            high_id.op.id()
        );

        // The LOWER op id arrives later and takes over, because the degraded
        // order is ascending op id and knows nothing of arrival sequence.
        log.append(low_id.clone(), Arrival::unordered());
        assert_eq!(
            current_version(&log, &id).unwrap().current.id(),
            low_id.op.id(),
            "the degraded order re-resolves rather than advancing"
        );
    }

    // ─── Only a post has versions ─────────────────────────────────────────

    #[test]
    fn an_op_that_is_not_a_post_has_no_current_version() {
        // A vote, a moderation and a revision each resolve to absence — and
        // each is given a revision naming it, so a resolver that skipped the
        // kind check would return that revision as their "current version".
        let key = author();
        let stoa = a_stoa();
        let not_posts = [
            OpKind::Vote {
                target: a_post("subject").op.id(),
                direction: VoteDirection::Up,
            },
            OpKind::Moderate {
                target: a_post("subject").op.id(),
                action: ModerationAction::Hide,
            },
            OpKind::Revise {
                target: a_post("subject").op.id(),
                body: "a revision".to_string(),
                attachments: vec![],
            },
        ];

        for kind in not_posts {
            let op = Op {
                stoa,
                author: key.public_key(),
                kind,
            }
            .sign(&key);
            let id = op.op.id();

            let mut log = MemoryOpLog::new();
            log.append(op, Arrival::ordered(1, a_message_id(1)));
            // A perfectly valid revision naming it, by its own author.
            let tempting = a_revision(id, "should not be reachable");
            log.append(tempting.clone(), Arrival::ordered(2, a_message_id(1)));
            // The revision really does name it, so the kind check is what
            // rejects this rather than an empty `iter_target`.
            assert_eq!(log.iter_target(&id).len(), 1);

            assert!(
                current_version(&log, &id).is_none(),
                "only a post has versions"
            );
        }
    }

    #[test]
    fn a_reply_is_a_post_and_has_versions() {
        // The other side of the kind check: a reply is a `Post` with its parent
        // set (§4.1, and `op.rs`'s "there is no `Reply` kind"), so it must
        // resolve like any other post. A kind check written as "a top-level
        // post" would make every reply unrevisable.
        let parent = a_post("the parent").op.id();
        let reply = Op {
            stoa: a_stoa(),
            author: author().public_key(),
            kind: OpKind::Post {
                thread: Some(parent),
                parent: Some(parent),
                body: "a reply".to_string(),
                attachments: vec![],
            },
        }
        .sign(&author());
        let id = reply.op.id();

        let mut log = MemoryOpLog::new();
        log.append(reply, Arrival::ordered(1, a_message_id(1)));
        log.append(
            a_revision(id, "an edited reply"),
            Arrival::ordered(2, a_message_id(1)),
        );

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "an edited reply");
        assert!(resolved.is_revised());
    }

    // ─── A version of a version is not a version of the post ──────────────

    #[test]
    fn a_revision_of_a_revision_is_not_a_version_of_the_post() {
        // Specified, not a default: the `post-revision` requirement "Every
        // version names the original post, and a version of a version is not
        // one". PLAN.md §5.7 left it open — it says "an edit is a new version of
        // that post" without saying whether a `Revise` may name another — and
        // this change closed it. `design.md` carries the argument: a chain is
        // not resolvable over a partial set (a peer missing an intermediate link
        // cannot reach versions it already holds, and cannot observe that it is
        // pinned), and a chain forks, which `cmp_ops` has no notion of.
        //
        // The fixture makes the two readings DISAGREE: the chained op is the
        // most recent of all, so a resolver that followed chains would return
        // "v3 via chain", and only the flat reading returns "v2".
        let post = a_post("v1");
        let id = post.op.id();
        let v2 = a_revision(id, "v2");
        let chained = a_revision(v2.op.id(), "v3 via chain");

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(v2.clone(), Arrival::ordered(2, a_message_id(1)));
        log.append(chained.clone(), Arrival::ordered(3, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "v2");
        assert_eq!(resolved.current.id(), v2.op.id());

        // The chained op is in the log and names v2 — so it was dropped by the
        // flat rule, not by never having been stored.
        assert!(log.get(&chained.op.id()).is_some());
        assert_eq!(log.iter_target(&v2.op.id()).len(), 1);
        // And resolving the revision itself is absence: it is not a post.
        assert!(current_version(&log, &v2.op.id()).is_none());
    }

    #[test]
    fn versions_naming_the_original_all_compete_directly() {
        // The flat model's positive form: four versions, all naming the post,
        // ordered against one another rather than through any chain.
        let post = a_post("v1");
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        for (n, lamport) in [("v2", 2u64), ("v3", 3), ("v4", 4), ("v5", 5)] {
            log.append(
                a_revision(id, n),
                Arrival::ordered(lamport, a_message_id(1)),
            );
        }
        assert_eq!(log.iter_target(&id).len(), 4, "all four name the post");
        assert_eq!(current_version(&log, &id).unwrap().body(), "v5");
    }

    // ─── History is kept ──────────────────────────────────────────────────

    #[test]
    fn resolving_removes_nothing_and_alters_nothing() {
        // §5.7: "Superseded versions stay in the op log." Asserted directly: the
        // log's contents before and after resolving are identical, byte for
        // byte, signatures intact.
        let post = a_post("v1");
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post.clone(), Arrival::ordered(1, a_message_id(1)));
        log.append(a_revision(id, "v2"), Arrival::ordered(2, a_message_id(1)));
        log.append(a_revision(id, "v3"), Arrival::ordered(3, a_message_id(1)));
        log.append(
            a_revision_by(&stranger(), id, "dropped"),
            Arrival::ordered(4, a_message_id(1)),
        );

        let before: Vec<Vec<u8>> = log.iter().iter().map(|e| e.op.to_bytes()).collect();
        let count = log.len();

        let _ = current_version(&log, &id).unwrap();

        let after: Vec<Vec<u8>> = log.iter().iter().map(|e| e.op.to_bytes()).collect();
        assert_eq!(before, after, "resolving altered the log");
        assert_eq!(log.len(), count);
        assert_eq!(count, 4, "including the dropped stranger's revision");
        // Even the version that was dropped on read is still there and still
        // verifiable as the authentic op it is.
        assert!(log.iter().iter().all(|e| e.op.verify()));
    }

    #[test]
    fn a_superseded_version_is_nameable_by_its_own_op_id() {
        // §5.7: "a moderator acting on a post is acting on a version they can
        // name". So each version must remain individually addressable.
        let post = a_post("v1");
        let id = post.op.id();
        let v2 = a_revision(id, "v2");
        let v3 = a_revision(id, "v3");

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(v2.clone(), Arrival::ordered(2, a_message_id(1)));
        log.append(v3.clone(), Arrival::ordered(3, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.current.id(), v3.op.id());
        assert_eq!(log.get(&v2.op.id()).unwrap().op, v2);
        assert_eq!(log.get(&id).unwrap().id(), id);
    }

    // ─── What the resolved version reports ────────────────────────────────

    #[test]
    fn the_attachments_come_from_the_current_version() {
        // §4.6: attachments are a field of the op. An edit that changes them
        // must be reflected, or a reader renders the old version's images beside
        // the new version's text.
        let post = Op {
            stoa: a_stoa(),
            author: author().public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "v1".to_string(),
                attachments: vec!["old-cid".to_string()],
            },
        }
        .sign(&author());
        let id = post.op.id();
        let revised = Op {
            stoa: a_stoa(),
            author: author().public_key(),
            kind: OpKind::Revise {
                target: id,
                body: "v2".to_string(),
                attachments: vec!["new-cid".to_string(), "another".to_string()],
            },
        }
        .sign(&author());

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(revised, Arrival::ordered(2, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.attachments(), ["new-cid", "another"]);
        assert_eq!(resolved.body(), "v2");
    }

    #[test]
    fn an_unrevised_post_reports_its_own_body_and_attachments() {
        // The other branch of `body()`/`attachments()`: `current` is the `Post`
        // itself. A match written for `Revise` alone would render every
        // unedited post blank.
        let post = Op {
            stoa: a_stoa(),
            author: author().public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "only ever this".to_string(),
                attachments: vec!["cid".to_string()],
            },
        }
        .sign(&author());
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::unordered());

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "only ever this");
        assert_eq!(resolved.attachments(), ["cid"]);
        assert!(!resolved.is_revised());
    }

    #[test]
    fn an_empty_revision_body_is_an_edit_and_not_an_absence() {
        // Empty is a legitimate edit — a user clearing their post — and must not
        // read as "no revision". A resolver treating an empty body as absent
        // would silently restore text the author deleted.
        let post = a_post("said too much");
        let id = post.op.id();
        let cleared = a_revision(id, "");

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(cleared.clone(), Arrival::ordered(2, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "");
        assert!(resolved.is_revised());
        assert_eq!(resolved.current.id(), cleared.op.id());
    }

    #[test]
    fn a_revision_clearing_the_attachments_removes_them() {
        // The attachment counterpart of the test above, and the sharper of the
        // two. §4.6 makes attachments Logos Storage CIDs, so a resolver that
        // treated an EMPTY list as "the revision said nothing about
        // attachments" and fell back to the original's would keep fetching and
        // rendering an image the author deleted.
        //
        // That is the "defensive" mistake — `if current.is_empty() { original }`
        // looks like robustness and is data resurrection. Every other fixture
        // here varies attachments non-empty to non-empty, so this is the only
        // test that takes the empty branch at all.
        let post = Op {
            stoa: a_stoa(),
            author: author().public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "with a picture".to_string(),
                attachments: vec!["old-cid".to_string()],
            },
        }
        .sign(&author());
        let id = post.op.id();
        let stripped = Op {
            stoa: a_stoa(),
            author: author().public_key(),
            kind: OpKind::Revise {
                target: id,
                body: "picture removed".to_string(),
                attachments: vec![],
            },
        }
        .sign(&author());

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(stripped.clone(), Arrival::ordered(2, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(
            resolved.attachments(),
            [] as [String; 0],
            "a deleted attachment must not be resurrected from the original"
        );
        // And the revision really is current, so the empty list came from the
        // revision rather than from the resolver having failed to find it.
        assert_eq!(resolved.body(), "picture removed");
        assert_eq!(resolved.current.id(), stripped.op.id());
        assert!(resolved.is_revised());
    }

    // ─── Other op kinds naming the post do not interfere ──────────────────

    #[test]
    fn a_moderation_or_a_vote_on_the_post_is_not_a_version_of_it() {
        // `iter_target` returns every kind naming the post, so the resolver must
        // narrow. §5.7 is explicit that a hide and an edit "are about different
        // things and do not contend": a moderation must neither become the
        // current version nor displace one.
        //
        // Both are given the HIGHEST Lamport values, so they head `iter_target`
        // and a resolver without the kind check would return one of them.
        let post = a_post("v1");
        let id = post.op.id();
        let v2 = a_revision(id, "v2");
        let key = author();
        let hide = Op {
            stoa: a_stoa(),
            author: key.public_key(),
            kind: OpKind::Moderate {
                target: id,
                action: ModerationAction::Hide,
            },
        }
        .sign(&key);
        let vote = Op {
            stoa: a_stoa(),
            author: key.public_key(),
            kind: OpKind::Vote {
                target: id,
                direction: VoteDirection::Up,
            },
        }
        .sign(&key);

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(v2.clone(), Arrival::ordered(2, a_message_id(1)));
        log.append(hide.clone(), Arrival::ordered(8, a_message_id(1)));
        log.append(vote.clone(), Arrival::ordered(9, a_message_id(1)));

        // The fixture must genuinely put them first.
        let ordered: Vec<OpId> = log.iter_target(&id).iter().map(|e| e.id()).collect();
        assert_eq!(ordered, vec![vote.op.id(), hide.op.id(), v2.op.id()]);

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.current.id(), v2.op.id());
        assert_eq!(resolved.body(), "v2");
    }

    #[test]
    fn the_kind_check_holds_at_the_top_of_the_order() {
        // Split out of `resolving_against_a_log_of_junk_never_panics`, which
        // was carrying this coverage in a final assertion its name does not
        // describe — so someone trimming that test to "just check it doesn't
        // panic" would have halved the guard on the `Revise` kind check with a
        // green suite.
        //
        // A moderation and a vote by the post's OWN author, at the very top of
        // the order: everything except the kind check says they should win.
        let post = a_post("the subject");
        let id = post.op.id();
        let v2 = a_revision(id, "v2");
        let key = author();

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::unordered());
        log.append(v2.clone(), Arrival::ordered(1, a_message_id(9)));
        for kind in [
            OpKind::Moderate {
                target: id,
                action: ModerationAction::Hide,
            },
            OpKind::Vote {
                target: id,
                direction: VoteDirection::Up,
            },
        ] {
            log.append(
                Op {
                    stoa: a_stoa(),
                    author: key.public_key(),
                    kind,
                }
                .sign(&key),
                Arrival::ordered(u64::MAX, MessageId::new(vec![0x00])),
            );
        }

        // The fixture must genuinely place them ahead of the revision.
        let ordered: Vec<OpId> = log.iter_target(&id).iter().map(|e| e.id()).collect();
        assert_eq!(ordered.len(), 3);
        assert_eq!(
            ordered[2],
            v2.op.id(),
            "the revision must be LAST, so only the kind check can reject the other two"
        );

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.current.id(), v2.op.id());
        assert_eq!(resolved.body(), "v2");
    }

    #[test]
    fn a_reply_to_the_post_is_not_a_version_of_it() {
        // A reply names its parent as a reply relationship, not as a subject
        // acted upon — `Entry::target` returns `None` for a `Post`. Pinned from
        // this side too: a reply must never be mistaken for an edit.
        let post = a_post("the parent");
        let id = post.op.id();
        let reply = Op {
            stoa: a_stoa(),
            author: author().public_key(),
            kind: OpKind::Post {
                thread: Some(id),
                parent: Some(id),
                body: "a reply, not an edit".to_string(),
                attachments: vec![],
            },
        }
        .sign(&author());

        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(reply, Arrival::ordered(9, a_message_id(1)));

        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "the parent");
        assert!(!resolved.is_revised());
    }

    #[test]
    fn a_revision_naming_a_different_post_does_not_reach_this_one() {
        // Two posts by one author, each revised. Each must resolve to its own
        // version — a resolver that ignored the target would see both.
        let first = a_post("first v1");
        let second = a_post("second v1");
        let (first_id, second_id) = (first.op.id(), second.op.id());
        assert_ne!(first_id, second_id);

        let mut log = MemoryOpLog::new();
        log.append(first, Arrival::ordered(1, a_message_id(1)));
        log.append(second, Arrival::ordered(1, a_message_id(2)));
        log.append(
            a_revision(first_id, "first v2"),
            Arrival::ordered(5, a_message_id(1)),
        );
        log.append(
            a_revision(second_id, "second v2"),
            Arrival::ordered(9, a_message_id(1)),
        );

        assert_eq!(current_version(&log, &first_id).unwrap().body(), "first v2");
        assert_eq!(
            current_version(&log, &second_id).unwrap().body(),
            "second v2"
        );
    }

    // ─── Hostile input ────────────────────────────────────────────────────

    #[test]
    fn resolving_against_a_log_of_junk_never_panics() {
        // PHASE0-FINDINGS §3: a panic ABORTS the module process, so every op
        // here — all of which arrived from a peer — is a denial-of-service lever
        // if it can reach one. This log holds forgeries, strangers' revisions,
        // dangling targets, self-naming ops and every op kind at once.
        let post = a_post("the subject");
        let id = post.op.id();
        let absent = a_post("never received").op.id();
        let key = author();
        let stoa = a_stoa();

        let mut log = MemoryOpLog::new();
        log.append(post.clone(), Arrival::unordered());
        log.append(
            a_forged_revision(&post.op.author, &stranger(), id, "forged"),
            Arrival::ordered(u64::MAX, a_message_id(0)),
        );
        log.append(
            a_revision_by(&stranger(), id, "stranger"),
            Arrival::ordered(0, MessageId::new(vec![])),
        );
        log.append(
            a_revision(absent, "dangling"),
            Arrival::from_parts(Some(1), None),
        );
        log.append(
            a_revision(id, &"x".repeat(64 * 1024)),
            Arrival::from_parts(None, Some(a_message_id(1))),
        );
        log.append(
            a_revision(id, "\u{0}\u{feff}🏛"),
            Arrival::ordered(u64::MAX, MessageId::new(vec![0xFF; 1024])),
        );
        for kind in [
            OpKind::Moderate {
                target: id,
                action: ModerationAction::Unhide,
            },
            OpKind::Vote {
                target: id,
                direction: VoteDirection::Down,
            },
        ] {
            // Given the TOP of the order deliberately, and by the post's own
            // author: everything except the kind check says they should win, so
            // the final assertion below discriminates on the kind check alone.
            log.append(
                Op {
                    stoa,
                    author: key.public_key(),
                    kind,
                }
                .sign(&key),
                Arrival::ordered(u64::MAX, MessageId::new(vec![0x00])),
            );
        }

        // Every op id in the log, plus ids naming nothing at all.
        let mut ids: Vec<OpId> = log.iter().iter().map(|e| e.id()).collect();
        ids.push(absent);
        ids.push(OpId::from_hex(&"00".repeat(32)).unwrap());
        ids.push(OpId::from_hex(&"ff".repeat(32)).unwrap());
        for probe in ids {
            if let Some(resolved) = current_version(&log, &probe) {
                let _ = resolved.body();
                let _ = resolved.attachments();
                let _ = resolved.is_revised();
            }
        }

        // And the post still resolves to something sane despite all of it: the
        // only valid revisions are the author's own two, and the ordering rule
        // places the Lamport-u64::MAX one first.
        let resolved = current_version(&log, &id).unwrap();
        assert_eq!(resolved.body(), "\u{0}\u{feff}🏛");
    }

    #[test]
    fn an_op_naming_itself_terminates() {
        // A `Revise` cannot name itself — its target is inside the bytes its id
        // hashes — so this is built from a `Vote`, which can be given its own
        // id as a target only by construction. The property under test is that
        // nothing here recurses: one `iter_target` call is the whole search.
        let post = a_post("the subject");
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::unordered());

        // A revision naming the post, then a second revision naming THAT — the
        // nearest thing to a cycle the format permits, plus a self-targeting
        // vote.
        let v2 = a_revision(id, "v2");
        log.append(v2.clone(), Arrival::unordered());
        log.append(a_revision(v2.op.id(), "v3"), Arrival::unordered());
        let key = author();
        let self_vote = {
            let vote = Op {
                stoa: a_stoa(),
                author: key.public_key(),
                kind: OpKind::Vote {
                    target: id,
                    direction: VoteDirection::Up,
                },
            };
            let self_id = vote.id();
            Op {
                stoa: a_stoa(),
                author: key.public_key(),
                kind: OpKind::Vote {
                    target: self_id,
                    direction: VoteDirection::Up,
                },
            }
            .sign(&key)
        };
        log.append(self_vote.clone(), Arrival::unordered());

        assert_eq!(current_version(&log, &id).unwrap().current.id(), v2.op.id());
        assert!(current_version(&log, &v2.op.id()).is_none());
        assert!(current_version(&log, &self_vote.op.id()).is_none());
    }

    // ─── Written against the trait, not the implementation ────────────────

    #[test]
    fn the_resolver_is_generic_over_the_log_trait() {
        // NO SPEC: the spec requires a resolver over the log's read API; it does
        // not require that the resolver be generic over the `OpLog` trait rather
        // than written against `MemoryOpLog`. Chosen: generic, because §3.3's
        // SQLite implementation is the reason the trait exists — a resolver
        // naming the concrete type would have to be rewritten rather than
        // relinked.
        //
        // Pinned by calling it through a generic function, which does not
        // compile if `current_version` names a concrete log type.
        fn resolve_through_the_trait<L: OpLog>(log: &L, post: &OpId) -> Option<String> {
            current_version(log, post).map(|v| v.body().to_string())
        }

        let post = a_post("v1");
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        log.append(a_revision(id, "v2"), Arrival::ordered(2, a_message_id(1)));

        assert_eq!(resolve_through_the_trait(&log, &id), Some("v2".to_string()));
    }
}
