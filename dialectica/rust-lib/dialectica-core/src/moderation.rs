//! Is this target hidden? The read-path answer, and the authority check that
//! makes it mean anything.
//!
//! # Why this file exists
//!
//! PLAN.md §6 states the rule: "Moderation ops are valid only when signed by a
//! current moderator, and every peer verifies independently — so a hide binds
//! for everyone running honest code." §5.7 gives the ordering: "Among
//! *moderation* ops on the same target, last-write-wins by Lamport order, valid
//! only if the signer was a moderator at that time (§6)."
//!
//! Every piece existed before this module and none of them joined:
//! [`op::SignedOp::verify`] answers *authenticity* and says in its own
//! documentation that it does not answer authority; [`log::OpLog`] stores
//! forgeries deliberately and decides nothing; [`stoa::Genesis`] names the
//! creator. **This is the first code in the project that can decide authority**,
//! because it is the first that holds both a genesis record and the log.
//!
//! # The check is on READ, every time, by every peer
//!
//! §6.2 measured what the alternative costs in the nearest kin project, which
//! "checks moderator authority only on the send path and never on the read path,
//! so any peer can forge a moderation" — or forge the removal of one — in
//! shipped code (Appendix A). That is the defect this module exists to not have.
//!
//! So: nothing here caches an answer, nothing here is decided at append time,
//! and no peer's say-so substitutes for the check. [`resolve`] is a pure
//! function of the ops the peer holds and the Stoa's genesis record, so two
//! peers holding the same ops reach the same answer with nothing local in it.
//!
//! # Three checks, and the third is the one a reader will not expect
//!
//! [`Moderators::authorises`] is the only route to a binding op, and it runs all
//! three:
//!
//! 1. **Authenticity.** [`SignedOp::verify`] — is this really from the key it
//!    names?
//! 2. **Authority.** Is that key a moderator of this Stoa? The check `op.rs`
//!    explicitly declines to make.
//! 3. **Scope.** Is the op's own `stoa` the Stoa this moderator set governs?
//!
//! The third looks redundant and is not. `op.rs` puts the Stoa address inside
//! the signed preimage, so an op *lifted* from one Stoa's channel onto another's
//! fails check 1 — `an_op_replayed_into_another_stoa_does_not_verify` pins it.
//! But the preimage stops an attacker rewriting the field; it does not stop the
//! author choosing it. A moderator of Stoa A can honestly sign a `Moderate`
//! naming Stoa B: valid signature, real moderator key, and not a moderation of
//! anything, because that key moderates A. Checks 1 and 2 both pass it. Only
//! check 3 refuses it.
//!
//! What the signed field *does* buy is that check 3 is worth making: an op that
//! passed check 1 has a Stoa its author chose and nobody since has altered. The
//! two compose; neither substitutes for the other.
//!
//! # Failing closed is arranged by the type, not by a branch
//!
//! A genesis record is **not an op** — `op.rs`: "Creating a Stoa is not here [...]
//! there is nothing for a *signed op* to add" — so it is not in the log, and a
//! resolver that went looking for one would be looking where it provably is not.
//!
//! [`Moderators::of`] is therefore the only constructor, and it takes a
//! [`Genesis`]. A caller with no genesis record cannot build the argument, so the
//! call does not compile. That is failing closed at build time rather than as a
//! runtime value something has to decide what to do with. The change's
//! `design.md` argues why closed is the right direction, and what would reverse
//! it.
//!
//! # Correct under both orders, with no branch
//!
//! Nothing here reads an [`Arrival`](crate::arrival::Arrival). The fold walks
//! [`OpLog::iter_target`], which is already in [`cmp_ops`](crate::arrival::cmp_ops)
//! order, and takes the first entry that binds — because that is the position
//! the ordering rule defines as current.
//!
//! **Not because the first entry is the most recent.** `cmp_ops` leads with the
//! highest Lamport timestamp only where the transport supplied one; otherwise —
//! which is every op today — it falls back to *ascending op id*, an order
//! carrying no recency whatever. `log.rs`'s [`OpLog::iter_target`] and
//! [`cmp_ops`](crate::arrival::cmp_ops) both state this; it is not restated
//! here. What matters to this fold is that the rule defines a first position and
//! every peer computes the same one, which holds under both branches — so the
//! same code is correct before and after the upstream fix.

use crate::identity::{Address, PublicKey};
use crate::log::{Entry, OpLog};
use crate::op::{ModerationAction, OpId, OpKind};
use crate::stoa::{Genesis, GenesisError};

/// Who may moderate a Stoa, and which Stoa they may moderate.
///
/// # Why the Stoa address is in here
///
/// Check 3 above needs it, and a caller passing it separately is the
/// fourth-slightly-different-guard shape CLAUDE.md warns about: a set of keys
/// with no Stoa attached cannot state the check at all, so every call site would
/// have to remember to. Holding the two together makes the comparison the type's
/// to make.
///
/// # A set of one, deliberately
///
/// §6: "**The creator is the sole moderator initially.** A mutable moderator set
/// is later work." §13 names concurrent moderator-set edits as "the one genuine
/// merge question in the design" and records that it "does not arise while the
/// creator is the sole moderator" — so this type has no `add`, no `remove`, and
/// one constructor. The merge question stays unreachable rather than unanswered.
///
/// `design.md` records what has to change when the mutable set lands, and the
/// sharpest part is worth repeating here: §5.7 requires the set **as of the
/// candidate op's Lamport position**, not as of now. The two are
/// indistinguishable today only because the set is constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moderators {
    /// The Stoa these keys moderate. Compared against each candidate op's own
    /// Stoa, which is what check 3 is.
    stoa: Address,
    /// The creator, from the genesis record. §6's sole moderator.
    creator: PublicKey,
}

impl Moderators {
    /// The moderator set a genesis record establishes.
    ///
    /// **The only constructor**, and that is the fail-closed decision made
    /// structural: a caller who cannot produce a genesis record cannot produce a
    /// moderator set, and so cannot ask this module for an answer it has no basis
    /// to give.
    ///
    /// **Fallible, because a record that cannot be encoded names no Stoa.**
    /// [`Genesis::address`] refuses a title over the genesis cap, and a record
    /// with no address cannot be the one any address names —
    /// [`Genesis::matches`] takes the same position from the other side. A
    /// `Genesis` is a plain struct a caller may build directly, so an over-cap
    /// title is reachable without going through the decoder; `unwrap`ping here
    /// would turn that into a panic, and everything this module sees arrived
    /// from a peer.
    ///
    /// Returning the underlying [`GenesisError`] rather than an [`Option`] keeps
    /// the reason, which is what lets a caller distinguish "that title is too
    /// long" from "that is not a Stoa I can identify".
    ///
    /// **A record obtained from a peer must be checked first.** `stoa.rs` supplies
    /// [`Genesis::matches`] for exactly that, and §4.8 is why: a Stoa address in a
    /// post is attacker-supplied content, and a substituted record names a
    /// creator of the attacker's choosing. This function cannot make that check
    /// itself — it has no address to check against beyond the one the record
    /// computes, which a substituted record also computes consistently.
    pub fn of(genesis: &Genesis) -> Result<Self, GenesisError> {
        Ok(Moderators {
            stoa: genesis.address()?,
            creator: genesis.creator.clone(),
        })
    }

    /// The Stoa this set governs.
    pub fn stoa(&self) -> &Address {
        &self.stoa
    }

    /// Whether a key may moderate this Stoa.
    ///
    /// Authority alone — it says nothing about any particular op. Separate from
    /// [`Moderators::authorises`] because "is this person a moderator?" is a
    /// question a UI asks (to offer a hide button) and not only one a resolver
    /// asks, and because passing an op to ask about a key would be a function
    /// with two jobs.
    pub fn contains(&self, key: &PublicKey) -> bool {
        &self.creator == key
    }

    /// Whether this entry is a moderation that binds.
    ///
    /// **All three checks, in one place, so there is one place to forget them
    /// rather than one per resolver.** CLAUDE.md: "A guard is a job. Keep it
    /// separate, so 'is it called everywhere?' stays a question with an answer."
    /// The next resolvers — §6.1's author-scoped suppression, §6.2's threshold
    /// certificate — call this rather than re-spelling the conjunction and
    /// dropping a term.
    ///
    /// Note what is NOT checked: the op's kind. A caller wanting moderations has
    /// already narrowed to them, and this is asked of things that are already
    /// candidates.
    fn authorises(&self, entry: &Entry) -> bool {
        // Scope first, because it is the cheapest and the most likely to
        // exclude: an unrestricted read over a peer holding several Stoas is
        // mostly other Stoas. Order is a performance matter only — all three
        // must pass, so no ordering of them can change the answer.
        entry.op.op.stoa == self.stoa
            && self.contains(&entry.op.op.author)
            // Last, because signature verification is the expensive one, and
            // because an op that fails either check above never needed it.
            && entry.op.verify()
    }
}

/// What the ops a peer holds say about one target.
///
/// # Why this is not a `bool`
///
/// Three reasons, in increasing order of weight.
///
/// §5.7 already argues the first for the other resolver: "a moderator acting on a
/// post is acting on a version they can name". A moderator *reversing* a hide is
/// in the same position — they are acting on a specific judgement, possibly
/// someone else's — and a bare boolean cannot name it.
///
/// Second, [`Moderation::Unmoderated`] and [`Moderation::Unhidden`] are different
/// facts that a bool collapses. A target nobody moderated and a target a moderator
/// deliberately restored are the same `false`, and the difference cannot be
/// recovered afterwards. It is also what lets a hostile-input test distinguish
/// "the forgery was ignored" from "the forgery was applied as an unhide" — under
/// a bool both read as `false`.
///
/// Third, a bool invites a caller to store it, and §6.2's whole finding is about a
/// check that stopped being run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Moderation<'a> {
    /// No binding moderation of this target reached this peer.
    ///
    /// Not distinguishable from "nobody moderated it", and that is correct rather
    /// than a limitation: §3.3's different-op-sets case is the normal case, so a
    /// hide that has not propagated yet and a hide that was never published look
    /// the same from here. Convergence closes the gap.
    Unmoderated,
    /// A moderator hid it, and this is the op that did.
    Hidden(&'a Entry),
    /// A moderator hid it and a moderator lifted that, or a moderator published a
    /// bare unhide. This is the op that decided it.
    Unhidden(&'a Entry),
}

impl<'a> Moderation<'a> {
    /// Whether a conforming peer stops rendering the target.
    ///
    /// §6.1 is careful about the ceiling and so is this name: moderation "can only
    /// change what conforming peers *render*", and cannot unpublish anything.
    pub fn is_hidden(&self) -> bool {
        matches!(self, Moderation::Hidden(_))
    }

    /// The op that decided this, if one did.
    pub fn deciding_op(&self) -> Option<&'a Entry> {
        match self {
            Moderation::Unmoderated => None,
            Moderation::Hidden(e) | Moderation::Unhidden(e) => Some(e),
        }
    }
}

/// Resolve one target's moderation state from the ops this peer holds.
///
/// # The whole rule, in the order it applies
///
/// Walk [`OpLog::iter_target`], which is already in
/// [`cmp_ops`](crate::arrival::cmp_ops) order; keep the first entry that is a
/// [`OpKind::Moderate`] **and** passes [`Moderators::authorises`]; report its
/// action. That is §5.7's "last-write-wins by Lamport order, valid only if the
/// signer was a moderator at that time".
///
/// The first entry is taken because that is the position the ordering rule
/// defines as current — **not because it is the most recent**, which
/// [`OpLog::iter_target`] is careful to say it cannot currently promise.
///
/// **Ops that do not bind do not participate in the ordering at all.** A forgery
/// ahead of a genuine hide does not displace it — it is skipped, and the fold
/// continues. A resolver that took the leading *moderation* and then checked it
/// would report `Unmoderated` for a hidden post whenever an attacker published
/// anything that sorted first, which is a censorship-resistance failure reached
/// through a validation check.
///
/// # Defined over a partial set
///
/// Every outcome is an answer, and none is an error. A target the log holds no ops
/// about is [`Moderation::Unmoderated`]; so is a target whose only moderations are
/// forged. A target op the peer never received is irrelevant — the moderation ops
/// naming it are what decide, and `iter_target` returns those whether or not the
/// subject arrived.
///
/// The converse holds and is also correct: a peer missing the newest `Unhide`
/// reports `Hidden`. §3.3's different-op-sets case is the normal one.
pub fn resolve<'a, L: OpLog>(log: &'a L, moderators: &Moderators, target: &OpId) -> Moderation<'a> {
    log.iter_target(target)
        .into_iter()
        .find_map(|entry| match &entry.op.op.kind {
            // The kind filter is the resolver's job, not the log's: `iter_target`
            // answers "what acts on this subject?" for every kind, so that a
            // fifth op kind does not widen the store's API. §5.7: a moderator's
            // hide and an author's edit "are about different things and do not
            // contend".
            OpKind::Moderate { action, .. } if moderators.authorises(entry) => match action {
                ModerationAction::Hide => Some(Moderation::Hidden(entry)),
                ModerationAction::Unhide => Some(Moderation::Unhidden(entry)),
            },
            _ => None,
        })
        .unwrap_or(Moderation::Unmoderated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrival::{Arrival, MessageId};
    use crate::identity::{sign_op_bytes, SecretKey};
    use crate::log::MemoryOpLog;
    use crate::op::{Op, SignedOp, VoteDirection};
    use crate::stoa::Policy;

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    /// The creator of the Stoa under test, and so its sole moderator (§6).
    fn creator() -> SecretKey {
        a_key(1)
    }

    /// Someone who is emphatically not a moderator anywhere in these fixtures.
    fn outsider() -> SecretKey {
        a_key(9)
    }

    fn a_genesis(creator: &SecretKey, title: &str) -> Genesis {
        Genesis {
            creator: creator.public_key(),
            policy: Policy::Open,
            title: title.to_string(),
        }
    }

    /// The Stoa every fixture below uses unless it says otherwise.
    fn agora() -> Genesis {
        a_genesis(&creator(), "Agora")
    }

    /// A fixture Stoa's address.
    ///
    /// [`Genesis::address`] is fallible because a title over the genesis cap has
    /// no encoding and so no address. Every title in this module is a short
    /// literal, so the arm cannot fire — and the `expect` is written to say why
    /// rather than to silence a `Result`. **A future fixture with a long title
    /// should fail loudly here** rather than quietly derive an address for a
    /// record other than the one it named.
    fn address_of(genesis: &Genesis) -> Address {
        genesis
            .address()
            .expect("every fixture title here is a short literal, far under the genesis cap")
    }

    /// The moderator set of a fixture Stoa.
    ///
    /// Same reasoning as [`address_of`]: `Moderators::of` is fallible for
    /// exactly the one condition these fixtures cannot produce.
    fn moderators_of(genesis: &Genesis) -> Moderators {
        Moderators::of(genesis)
            .expect("every fixture title here is a short literal, far under the genesis cap")
    }

    fn a_message_id(seed: u8) -> MessageId {
        MessageId::new(vec![seed; 32])
    }

    /// A post in `stoa`, by `author`.
    fn a_post(stoa: Address, author: &SecretKey, body: &str) -> SignedOp {
        Op {
            stoa,
            author: author.public_key(),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: body.to_string(),
                attachments: vec![],
            },
        }
        .sign(author)
    }

    /// A moderation op, signed by `signer` and claiming `signer` as its author.
    fn a_moderation(
        stoa: Address,
        signer: &SecretKey,
        target: OpId,
        action: ModerationAction,
    ) -> SignedOp {
        Op {
            stoa,
            author: signer.public_key(),
            kind: OpKind::Moderate { target, action },
        }
        .sign(signer)
    }

    /// A moderation op CLAIMING `claimed_author` but signed by `actual_signer`.
    ///
    /// The forgery `op.rs`'s `an_op_signed_by_someone_else_is_rejected` describes,
    /// assembled here so the resolver can be shown one. Every use asserts the
    /// fixture really does fail `verify`, or the test would prove nothing.
    fn a_forged_moderation(
        stoa: Address,
        claimed_author: &PublicKey,
        actual_signer: &SecretKey,
        target: OpId,
        action: ModerationAction,
    ) -> SignedOp {
        let op = Op {
            stoa,
            author: claimed_author.clone(),
            kind: OpKind::Moderate { target, action },
        };
        let forged = SignedOp {
            signature: sign_op_bytes(actual_signer, &op.canonical_bytes()),
            op,
        };
        assert!(!forged.verify(), "the fixture must be an actual forgery");
        forged
    }

    /// A log holding a post and whatever else, with the post's id.
    ///
    /// The post is the target every test moderates.
    fn a_log_with_a_post() -> (MemoryOpLog, OpId) {
        let post = a_post(address_of(&agora()), &a_key(2), "the subject");
        let id = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));
        (log, id)
    }

    // ─── THE security property: authority is decided on read ──────────────

    #[test]
    fn a_moderation_by_a_non_moderator_does_not_hide_anything() {
        // THE test this module exists to pass. §6.2, on the nearest kin project:
        // it "checks moderator authority only on the send path and never on the
        // read path, so any peer can forge a moderation".
        //
        // The op here is AUTHENTIC — `op.rs`'s
        // `verification_answers_authenticity_and_not_authority` pins that a
        // moderation by a non-moderator verifies, and that is correct. So a
        // resolver that ran only `verify()` would hide this target. Only the
        // authority check refuses it.
        let (mut log, target) = a_log_with_a_post();
        let hide = a_moderation(
            address_of(&agora()),
            &outsider(),
            target,
            ModerationAction::Hide,
        );
        // The fixture must be authentic, or this test passes for the wrong
        // reason — it would then be checking the signature check, not the
        // authority check.
        assert!(
            hide.verify(),
            "the fixture must be an AUTHENTIC op with no authority"
        );
        log.append(hide, Arrival::ordered(2, a_message_id(1)));

        let moderators = moderators_of(&agora());
        assert_eq!(resolve(&log, &moderators, &target), Moderation::Unmoderated);
        assert!(!resolve(&log, &moderators, &target).is_hidden());
    }

    #[test]
    fn a_moderation_forging_a_moderators_authorship_does_not_hide_anything() {
        // The other half of the forgery surface: rather than signing as
        // themselves, the attacker claims the MODERATOR as author. The authority
        // check now passes — the named author really is a moderator — and only
        // the signature check refuses it.
        //
        // Paired with the test above, this is what shows both checks are
        // load-bearing: each fixture defeats exactly one of them.
        let (mut log, target) = a_log_with_a_post();
        let forged = a_forged_moderation(
            address_of(&agora()),
            &creator().public_key(),
            &outsider(),
            target,
            ModerationAction::Hide,
        );
        let moderators = moderators_of(&agora());
        // The authority check alone would accept this: the claimed author IS the
        // moderator.
        assert!(moderators.contains(&forged.op.author));
        log.append(forged, Arrival::ordered(2, a_message_id(1)));

        assert_eq!(resolve(&log, &moderators, &target), Moderation::Unmoderated);
    }

    #[test]
    fn a_moderation_by_the_creator_hides_the_target() {
        // The positive case, without which every test above passes for a
        // resolver that hides nothing at all.
        let (mut log, target) = a_log_with_a_post();
        let hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let hide_id = hide.op.id();
        log.append(hide, Arrival::ordered(2, a_message_id(1)));

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(resolved.is_hidden());
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(hide_id));
    }

    #[test]
    fn an_unauthorised_op_does_not_displace_an_authorised_one() {
        // The fixture that makes two candidate rules DISAGREE.
        //
        // A resolver that took the most recent MODERATION and then checked it —
        // rather than the most recent moderation that PASSES the check — would
        // find the outsider's unhide first, reject it, and report Unmoderated.
        // The genuine hide would be silently dropped. That is a
        // censorship-resistance failure reached through a validation check, and
        // it is invisible to any test where the newest op happens to be valid.
        //
        // Lamport values put the forgery strictly LAST-written, so the two rules
        // demand opposite answers: "skip and continue" gives Hidden, "take newest
        // then validate" gives Unmoderated.
        let (mut log, target) = a_log_with_a_post();
        let hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let hide_id = hide.op.id();
        let bogus_unhide = a_moderation(
            address_of(&agora()),
            &outsider(),
            target,
            ModerationAction::Unhide,
        );
        log.append(hide, Arrival::ordered(2, a_message_id(1)));
        log.append(bogus_unhide, Arrival::ordered(3, a_message_id(1)));

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(resolved.is_hidden(), "a forged unhide lifted a real hide");
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(hide_id));
    }

    #[test]
    fn the_answer_depends_on_nothing_but_the_ops_and_the_moderator_set() {
        // §6: "every peer verifies independently". Two logs holding the same ops
        // in OPPOSITE append sequences must reach the same answer and name the
        // same op — if anything peer-local reached the decision, these diverge.
        let post = a_post(address_of(&agora()), &a_key(2), "the subject");
        let target = post.op.id();
        let ops = [
            (post, Arrival::ordered(1, a_message_id(1))),
            (
                a_moderation(
                    address_of(&agora()),
                    &creator(),
                    target,
                    ModerationAction::Hide,
                ),
                Arrival::ordered(2, a_message_id(1)),
            ),
            (
                a_moderation(
                    address_of(&agora()),
                    &outsider(),
                    target,
                    ModerationAction::Unhide,
                ),
                Arrival::ordered(3, a_message_id(1)),
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

        let moderators = moderators_of(&agora());
        let a = resolve(&forwards, &moderators, &target);
        let b = resolve(&backwards, &moderators, &target);
        assert!(a.is_hidden() && b.is_hidden());
        assert_eq!(
            a.deciding_op().map(|e| e.id()),
            b.deciding_op().map(|e| e.id())
        );
    }

    // ─── The moderator set comes from genesis, and is a set of one ────────

    #[test]
    fn the_creator_is_a_moderator_and_nobody_else_is() {
        // §6: "The creator is the sole moderator initially."
        let moderators = moderators_of(&agora());
        assert!(moderators.contains(&creator().public_key()));
        for other in [a_key(2), a_key(3), outsider()] {
            assert!(
                !moderators.contains(&other.public_key()),
                "only the creator moderates"
            );
        }
    }

    #[test]
    fn the_moderator_set_names_the_stoa_the_record_addresses() {
        // The address in `Moderators` must be the genesis record's own, or check
        // 3 compares against the wrong Stoa and either refuses everything or
        // admits another Stoa's ops.
        let genesis = agora();
        assert_eq!(moderators_of(&genesis).stoa(), &address_of(&genesis));
    }

    /// The genesis title cap, as `stoa.rs` pins it.
    ///
    /// Duplicated as a literal because `MAX_TITLE_BYTES` is private to `stoa`.
    /// That is safe only because `stoa.rs`'s own
    /// `the_wire_constants_are_pinned_to_known_answers` asserts it equals 1024
    /// with a hardcoded value, so a change to the cap fails there loudly rather
    /// than drifting silently past here.
    const GENESIS_TITLE_CAP: usize = 1024;

    #[test]
    fn a_moderator_set_cannot_be_built_from_a_record_that_has_no_address() {
        // `Moderators::of` became fallible when the genesis title gained a cap.
        // The condition is reachable: `Genesis` is a plain struct a caller may
        // build directly, so an over-cap title never passes through the decoder
        // that would have refused it. `unwrap`ping inside the library would turn
        // peer-supplied input into a panic, which aborts the module process.
        //
        // Tested as a PAIR at the boundary, not with a value far past it: a cap
        // that silently drifted upward would still refuse a wildly oversized
        // title and leave a one-sided test green.
        let at_cap = a_genesis(&creator(), &"x".repeat(GENESIS_TITLE_CAP));
        assert!(
            Moderators::of(&at_cap).is_ok(),
            "a title of exactly the cap must still name a Stoa"
        );

        let over_cap = a_genesis(&creator(), &"x".repeat(GENESIS_TITLE_CAP + 1));
        assert_eq!(
            Moderators::of(&over_cap),
            Err(GenesisError::TitleTooLong(GENESIS_TITLE_CAP + 1)),
            "one byte over the cap must fail, and say which failure it was"
        );
    }

    #[test]
    fn a_different_creator_yields_a_different_moderator_set() {
        // The creator field must actually reach the set. A `Moderators::of` that
        // ignored it would pass the two tests above if it happened to hold the
        // right key by other means.
        let theirs = a_genesis(&outsider(), "Agora");
        let mine = agora();
        assert!(!moderators_of(&theirs).contains(&creator().public_key()));
        assert!(moderators_of(&theirs).contains(&outsider().public_key()));
        assert!(!moderators_of(&mine).contains(&outsider().public_key()));
    }

    // ─── Check 3: a moderation binds only in its own Stoa ─────────────────

    #[test]
    fn a_moderators_op_naming_another_stoa_does_not_bind() {
        // The check that looks redundant and is not.
        //
        // `op.rs` puts the Stoa inside the signed preimage, and
        // `an_op_replayed_into_another_stoa_does_not_verify` shows an op LIFTED
        // from one Stoa to another fails verification. That stops an attacker
        // rewriting the field. It does not stop the author CHOOSING it: this op
        // is signed by Agora's creator, names Lyceum, and verifies — so checks 1
        // and 2 both pass, and only the Stoa comparison refuses it.
        let lyceum = a_genesis(&a_key(5), "Lyceum");
        assert_ne!(address_of(&agora()), address_of(&lyceum));

        let (mut log, target) = a_log_with_a_post();
        let elsewhere = a_moderation(
            address_of(&lyceum),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        // It really is authentic, and its author really is Agora's moderator.
        assert!(elsewhere.verify());
        let moderators = moderators_of(&agora());
        assert!(moderators.contains(&elsewhere.op.author));
        log.append(elsewhere, Arrival::ordered(2, a_message_id(1)));

        assert_eq!(resolve(&log, &moderators, &target), Moderation::Unmoderated);
    }

    #[test]
    fn each_stoas_moderator_binds_only_in_their_own_stoa() {
        // ONE creator, TWO Stoas. That is the fixture that makes the Stoa
        // comparison the only rule that can separate these ops — with two
        // different creators the authority check alone would exclude the other
        // Stoa's op, and this test would pass with the Stoa comparison deleted.
        // (Measured: it did, until the mutation run caught it.)
        //
        // A person creating two Stoas is ordinary, and it is exactly the case
        // where a resolver that trusted the moderator key alone goes wrong.
        let lyceum = a_genesis(&creator(), "Lyceum");
        assert_ne!(address_of(&agora()), address_of(&lyceum));
        assert_eq!(agora().creator, lyceum.creator, "one creator, two Stoas");

        let (mut log, target) = a_log_with_a_post();
        let agora_hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let agora_hide_id = agora_hide.op.id();
        let lyceum_unhide = a_moderation(
            address_of(&lyceum),
            &creator(),
            target,
            ModerationAction::Unhide,
        );
        let lyceum_unhide_id = lyceum_unhide.op.id();
        // Lyceum's op is the more recent, so a resolver that ignored the Stoa
        // would report Unhidden under BOTH sets.
        log.append(agora_hide, Arrival::ordered(2, a_message_id(1)));
        log.append(lyceum_unhide, Arrival::ordered(3, a_message_id(1)));

        let under_agora = resolve(&log, &moderators_of(&agora()), &target);
        assert!(under_agora.is_hidden());
        assert_eq!(
            under_agora.deciding_op().map(|e| e.id()),
            Some(agora_hide_id)
        );

        let under_lyceum = resolve(&log, &moderators_of(&lyceum), &target);
        assert_eq!(
            under_lyceum,
            Moderation::Unhidden(log.get(&lyceum_unhide_id).unwrap())
        );
    }

    // ─── Last-write-wins, and a hide is reversible ────────────────────────

    #[test]
    fn a_later_unhide_reverses_an_earlier_hide() {
        // §5.7: "last-write-wins by Lamport order". §13 asked whether a hide is
        // reversible and `op.rs` answered by naming `Unhide`; this is the
        // behaviour that makes the answer real.
        let (mut log, target) = a_log_with_a_post();
        let hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let unhide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Unhide,
        );
        let unhide_id = unhide.op.id();
        log.append(hide, Arrival::ordered(2, a_message_id(1)));
        log.append(unhide, Arrival::ordered(3, a_message_id(1)));

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(!resolved.is_hidden());
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(unhide_id));
        // And it is distinguishable from a target nobody moderated.
        assert_ne!(resolved, Moderation::Unmoderated);
    }

    #[test]
    fn a_later_hide_reverses_an_earlier_unhide() {
        // The other direction, which a resolver special-casing `Unhide` as
        // "terminal" or treating `Hide` as sticky would fail. Same ops, opposite
        // Lamport values.
        let (mut log, target) = a_log_with_a_post();
        let unhide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Unhide,
        );
        let hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let hide_id = hide.op.id();
        log.append(unhide, Arrival::ordered(2, a_message_id(1)));
        log.append(hide, Arrival::ordered(3, a_message_id(1)));

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(resolved.is_hidden());
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(hide_id));
    }

    #[test]
    fn the_order_is_by_lamport_and_not_by_op_id() {
        // The fixture trap, applied to this resolver. Both orders are live —
        // `cmp_ops` uses Lamport values where it has them and falls back to
        // ascending op id where it does not — so a test whose two ops agree under
        // both proves neither.
        //
        // `cmp_ops` reads the LOWEST op id first when nothing is ordered, and the
        // HIGHEST Lamport value first when things are. So the two rules disagree
        // only when the op with the HIGHER op id carries the HIGHER Lamport
        // value — which is how this fixture is built. Assigning them the other
        // way round would make both rules name the same op, and the test would
        // pass for a resolver that consulted no metadata at all.
        let (mut log, target) = a_log_with_a_post();
        let one = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let two = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Unhide,
        );
        // Determined, not assumed: which of two hashes is lower is not something
        // a reader should take on trust.
        let (low, high) = if one.op.id() < two.op.id() {
            (one, two)
        } else {
            (two, one)
        };
        let high_action = match &high.op.kind {
            OpKind::Moderate { action, .. } => *action,
            _ => unreachable!(),
        };
        let high_id = high.op.id();

        // The HIGH op id gets the HIGH Lamport value, so it decides under §5.7
        // and would NOT decide under the op-id fallback.
        log.append(low, Arrival::ordered(2, a_message_id(1)));
        log.append(high, Arrival::ordered(9, a_message_id(1)));

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert_eq!(
            resolved.deciding_op().map(|e| e.id()),
            Some(high_id),
            "the higher Lamport value must decide, against op-id order"
        );
        assert_eq!(resolved.is_hidden(), high_action == ModerationAction::Hide);
    }

    #[test]
    fn the_degraded_order_decides_when_the_transport_ordered_nothing() {
        // The order production ACTUALLY RUNS today: no Lamport value reaches us,
        // so `cmp_ops` falls back to ascending op id. A resolver correct only
        // under Lamport values would be untested for every op a peer currently
        // holds.
        //
        // `cmp_ops` sorts unordered ops by `a_id.cmp(b_id)` — ASCENDING — and
        // `Ordering::Less` means "orders first", so the LOWEST op id reads first
        // and is therefore what "most recent" degrades to. That is arbitrary with
        // respect to time and `arrival.rs` says so: the property bought is
        // convergence, not accuracy.
        //
        // Expected answer derived from the op ids alone, and the ops are appended
        // HIGHER-id first so insertion order cannot produce it by accident.
        let (mut log, target) = a_log_with_a_post();
        let one = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let two = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Unhide,
        );
        let (low, high) = if one.op.id() < two.op.id() {
            (one, two)
        } else {
            (two, one)
        };
        let low_id = low.op.id();
        let low_action = match &low.op.kind {
            OpKind::Moderate { action, .. } => *action,
            _ => unreachable!(),
        };

        log.append(high, Arrival::unordered());
        log.append(low, Arrival::unordered());

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert_eq!(
            resolved.deciding_op().map(|e| e.id()),
            Some(low_id),
            "under the degraded order the lower op id reads first"
        );
        assert_eq!(resolved.is_hidden(), low_action == ModerationAction::Hide);
    }

    #[test]
    fn a_moderator_may_reverse_a_moderation_they_did_not_place() {
        // NO SPEC: any moderator may reverse any moderation, rather than only
        // the moderator who placed it. §6 makes an op valid when signed by "a
        // current moderator" and says nothing about ownership of an earlier op.
        // Chosen this way because the alternative — moderations owned by their
        // author — would make a compromised or departed moderator's hide
        // permanent, which §6.2's whole direction (raising the cost of one rogue
        // key) argues against.
        //
        // Two moderators need a mutable moderator set, which does not exist. So
        // this is tested at the level that DOES exist: the check is membership,
        // not identity with the earlier op's author, and that is asserted
        // directly. Writing a two-moderator scenario would mean asserting against
        // behaviour that cannot be built today, which `.claude/agents/README.md`
        // rules out.
        let (mut log, target) = a_log_with_a_post();
        let hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        log.append(hide.clone(), Arrival::ordered(2, a_message_id(1)));

        // The authority predicate consults the moderator set and nothing about
        // the ops already present — so it cannot be conditioning on who placed
        // what.
        let moderators = moderators_of(&agora());
        let entry = log.get(&hide.op.id()).unwrap();
        assert!(moderators.authorises(entry));
        assert!(moderators.contains(&creator().public_key()));
    }

    // ─── Only moderation ops moderate ─────────────────────────────────────

    #[test]
    fn a_revision_does_not_clear_a_hide() {
        // §5.7: a moderator's hide and an author's edit "are about different
        // things and do not contend: an edit does not clear a hide". The revision
        // is the MOST RECENT op naming the target, so a resolver that took the
        // newest op and asked what it did would report Unmoderated.
        let post = a_post(address_of(&agora()), &a_key(2), "the subject");
        let target = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));

        let hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let hide_id = hide.op.id();
        let author = a_key(2);
        let revision = Op {
            stoa: address_of(&agora()),
            author: author.public_key(),
            kind: OpKind::Revise {
                target,
                body: "edited after being hidden".to_string(),
                attachments: vec![],
            },
        }
        .sign(&author);
        log.append(hide, Arrival::ordered(2, a_message_id(1)));
        log.append(revision, Arrival::ordered(3, a_message_id(1)));

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(resolved.is_hidden(), "an edit cleared a hide");
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(hide_id));
    }

    #[test]
    fn a_revision_by_the_moderator_themselves_still_does_not_moderate() {
        // The sharp version of the kind filter: a `Revise` by the CREATOR passes
        // authenticity, authority and scope. Only the kind check refuses it. A
        // resolver whose predicate was "authorised op naming this target" — with
        // the kind match dropped or widened — would treat this as a moderation.
        let post = a_post(address_of(&agora()), &creator(), "the subject");
        let target = post.op.id();
        let mut log = MemoryOpLog::new();
        log.append(post, Arrival::ordered(1, a_message_id(1)));

        let revision = Op {
            stoa: address_of(&agora()),
            author: creator().public_key(),
            kind: OpKind::Revise {
                target,
                body: "the creator edits their own post".to_string(),
                attachments: vec![],
            },
        }
        .sign(&creator());
        let revision_id = revision.op.id();
        log.append(revision, Arrival::ordered(2, a_message_id(1)));

        let moderators = moderators_of(&agora());
        // Every check but the kind check passes on this very entry — asserted
        // directly, so the test cannot pass because the fixture was accidentally
        // unauthorised for some other reason.
        assert!(
            moderators.authorises(log.get(&revision_id).unwrap()),
            "the fixture must clear authenticity, authority and scope"
        );
        assert_eq!(resolve(&log, &moderators, &target), Moderation::Unmoderated);
    }

    #[test]
    fn a_vote_by_a_moderator_does_not_moderate() {
        // The other kind that names a target and that a moderator plausibly
        // publishes. A `Vote{direction: Down}` by the creator is not a hide.
        let (mut log, target) = a_log_with_a_post();
        let vote = Op {
            stoa: address_of(&agora()),
            author: creator().public_key(),
            kind: OpKind::Vote {
                target,
                direction: VoteDirection::Down,
            },
        }
        .sign(&creator());
        assert!(vote.verify());
        log.append(vote, Arrival::ordered(2, a_message_id(1)));

        assert_eq!(
            resolve(&log, &moderators_of(&agora()), &target),
            Moderation::Unmoderated
        );
    }

    // ─── A partial set is the normal case ─────────────────────────────────

    #[test]
    fn a_target_no_op_names_is_not_hidden() {
        let (log, _) = a_log_with_a_post();
        let never_moderated = a_post(address_of(&agora()), &a_key(2), "untouched").op.id();
        assert_eq!(
            resolve(&log, &moderators_of(&agora()), &never_moderated),
            Moderation::Unmoderated
        );
    }

    #[test]
    fn an_empty_log_answers_and_does_not_error() {
        let log = MemoryOpLog::new();
        let target = a_post(address_of(&agora()), &a_key(2), "absent").op.id();
        assert_eq!(
            resolve(&log, &moderators_of(&agora()), &target),
            Moderation::Unmoderated
        );
    }

    #[test]
    fn a_hide_binds_even_when_the_target_op_never_arrived() {
        // The dangling-reference case, which is ordinary: §4.7's backfill delivers
        // history out of order by design, so a moderation may arrive before its
        // subject. A resolver that required the target op would leave a hidden
        // post rendered for as long as the ordering happened to be unlucky.
        let missing_target = a_post(address_of(&agora()), &a_key(2), "never received")
            .op
            .id();
        let hide = a_moderation(
            address_of(&agora()),
            &creator(),
            missing_target,
            ModerationAction::Hide,
        );
        let mut log = MemoryOpLog::new();
        log.append(hide, Arrival::ordered(2, a_message_id(1)));

        assert_eq!(
            log.get(&missing_target),
            None,
            "the target really is absent"
        );
        assert!(resolve(&log, &moderators_of(&agora()), &missing_target).is_hidden());
    }

    #[test]
    fn a_peer_missing_the_newest_unhide_still_reports_hidden() {
        // §3.3: two peers routinely hold different sets of ops. The peer without
        // the unhide is not in error — it holds a valid hide and nothing later.
        // Asserted as a PAIR, so that "the answer differs" is what is pinned
        // rather than either half alone.
        let post = a_post(address_of(&agora()), &a_key(2), "the subject");
        let target = post.op.id();
        let hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let unhide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Unhide,
        );

        let mut behind = MemoryOpLog::new();
        behind.append(post.clone(), Arrival::ordered(1, a_message_id(1)));
        behind.append(hide.clone(), Arrival::ordered(2, a_message_id(1)));

        let mut current = MemoryOpLog::new();
        current.append(post, Arrival::ordered(1, a_message_id(1)));
        current.append(hide, Arrival::ordered(2, a_message_id(1)));
        current.append(unhide, Arrival::ordered(3, a_message_id(1)));

        let moderators = moderators_of(&agora());
        assert!(resolve(&behind, &moderators, &target).is_hidden());
        assert!(!resolve(&current, &moderators, &target).is_hidden());
    }

    // ─── The result names the op ──────────────────────────────────────────

    #[test]
    fn the_deciding_op_carries_its_author_and_action() {
        // §5.7's reasoning for the other resolver — "a moderator acting on a post
        // is acting on a version they can name" — applied here: a moderator
        // reversing a hide needs to name the judgement they are reversing.
        let (mut log, target) = a_log_with_a_post();
        let hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let hide_id = hide.op.id();
        log.append(hide, Arrival::ordered(2, a_message_id(1)));

        let entry = resolve(&log, &moderators_of(&agora()), &target)
            .deciding_op()
            .expect("a hidden target names its op");
        assert_eq!(entry.id(), hide_id);
        assert_eq!(entry.op.op.author, creator().public_key());
        assert_eq!(
            entry.op.op.kind,
            OpKind::Moderate {
                target,
                action: ModerationAction::Hide
            }
        );
    }

    #[test]
    fn an_unmoderated_target_names_no_op() {
        let (log, _) = a_log_with_a_post();
        let target = a_post(address_of(&agora()), &a_key(2), "untouched").op.id();
        assert_eq!(
            resolve(&log, &moderators_of(&agora()), &target).deciding_op(),
            None
        );
    }

    #[test]
    fn a_restored_target_is_distinguishable_from_one_nobody_moderated() {
        // Both are "not hidden", and they are different facts. A bool return
        // would collapse them irrecoverably.
        //
        // NO SPEC: an `Unhide` with no prior `Hide` is binding and reported as
        // `Unhidden`, rather than refused as meaningless or reported as
        // `Unmoderated`. The spec makes the most recent binding moderation
        // decide and says nothing about what it must follow. Chosen this way
        // because the alternative needs the resolver to know whether a `Hide` it
        // may never have received existed — §3.3's partial set makes that
        // undecidable, and a rule no peer can evaluate is not a rule.
        let (mut log, target) = a_log_with_a_post();
        let unhide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Unhide,
        );
        log.append(unhide, Arrival::ordered(2, a_message_id(1)));
        let untouched = a_post(address_of(&agora()), &a_key(2), "untouched").op.id();

        let moderators = moderators_of(&agora());
        let restored = resolve(&log, &moderators, &target);
        let never = resolve(&log, &moderators, &untouched);
        assert!(!restored.is_hidden() && !never.is_hidden());
        assert_ne!(restored, never, "a bool would have collapsed these");
        assert!(restored.deciding_op().is_some());
        assert!(never.deciding_op().is_none());
    }

    // ─── Hostile input ────────────────────────────────────────────────────

    #[test]
    fn a_log_full_of_forgeries_resolves_without_a_panic_and_hides_nothing() {
        // PHASE0-FINDINGS §3: a panic ABORTS the module process, so a hostile op
        // would become a denial of service. Everything here arrived from a peer.
        //
        // Every op names the target and every one fails exactly one check, so a
        // resolver missing any single check would hide something. The assertion is
        // Unmoderated, not merely "no panic".
        let lyceum = a_genesis(&a_key(5), "Lyceum");
        let (mut log, target) = a_log_with_a_post();

        let junk: Vec<SignedOp> = vec![
            // Authentic, no authority.
            a_moderation(
                address_of(&agora()),
                &outsider(),
                target,
                ModerationAction::Hide,
            ),
            // Claims the moderator, signed by someone else.
            a_forged_moderation(
                address_of(&agora()),
                &creator().public_key(),
                &outsider(),
                target,
                ModerationAction::Hide,
            ),
            // Claims the moderator, signed by the moderator of a DIFFERENT Stoa.
            a_forged_moderation(
                address_of(&agora()),
                &creator().public_key(),
                &a_key(5),
                target,
                ModerationAction::Hide,
            ),
            // A real moderator, wrong Stoa.
            a_moderation(
                address_of(&lyceum),
                &creator(),
                target,
                ModerationAction::Hide,
            ),
            // Another Stoa's moderator, that Stoa — nothing to do with Agora.
            a_moderation(
                address_of(&lyceum),
                &a_key(5),
                target,
                ModerationAction::Hide,
            ),
            // A vote by the moderator: right author, right Stoa, wrong kind.
            Op {
                stoa: address_of(&agora()),
                author: creator().public_key(),
                kind: OpKind::Vote {
                    target,
                    direction: VoteDirection::Down,
                },
            }
            .sign(&creator()),
            // A revision by the moderator: same, other kind.
            Op {
                stoa: address_of(&agora()),
                author: creator().public_key(),
                kind: OpKind::Revise {
                    target,
                    body: String::new(),
                    attachments: vec![],
                },
            }
            .sign(&creator()),
        ];

        let arrivals = [
            Arrival::unordered(),
            Arrival::ordered(0, MessageId::new(vec![])),
            Arrival::ordered(u64::MAX, MessageId::new(vec![0xFF; 1024])),
            Arrival::from_parts(None, Some(MessageId::new(vec![1]))),
            Arrival::from_parts(Some(u64::MAX), None),
        ];
        for (n, op) in junk.into_iter().enumerate() {
            log.append(op, arrivals[n % arrivals.len()].clone());
        }

        let moderators = moderators_of(&agora());
        assert_eq!(
            resolve(&log, &moderators, &target),
            Moderation::Unmoderated,
            "a forgery bound"
        );
        // And resolving other things over the same hostile log does not panic.
        let all_zero = OpId::from_hex(&"00".repeat(32)).unwrap();
        let all_ones = OpId::from_hex(&"ff".repeat(32)).unwrap();
        let _ = resolve(&log, &moderators, &all_zero);
        let _ = resolve(&log, &moderators, &all_ones);
        let _ = resolve(&log, &moderators_of(&lyceum), &target);
    }

    #[test]
    fn one_genuine_hide_among_the_forgeries_still_binds() {
        // The complement of the test above, and the reason that one is not
        // vacuous: the same hostile log, plus one valid hide, must report Hidden
        // and must name THAT op. Without this, a resolver that returned
        // Unmoderated unconditionally would pass every hostile-input assertion.
        //
        // The genuine hide is given the LOWEST Lamport value of the moderation
        // ops, so it is the LEAST recent — a resolver that stopped at the newest
        // moderation and validated it would miss it.
        let lyceum = a_genesis(&a_key(5), "Lyceum");
        let (mut log, target) = a_log_with_a_post();

        let genuine = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let genuine_id = genuine.op.id();
        log.append(genuine, Arrival::ordered(2, a_message_id(1)));

        for (n, op) in [
            a_moderation(
                address_of(&agora()),
                &outsider(),
                target,
                ModerationAction::Unhide,
            ),
            a_forged_moderation(
                address_of(&agora()),
                &creator().public_key(),
                &outsider(),
                target,
                ModerationAction::Unhide,
            ),
            a_moderation(
                address_of(&lyceum),
                &creator(),
                target,
                ModerationAction::Unhide,
            ),
        ]
        .into_iter()
        .enumerate()
        {
            log.append(op, Arrival::ordered(10 + n as u64, a_message_id(1)));
        }

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(resolved.is_hidden());
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(genuine_id));
    }
}
