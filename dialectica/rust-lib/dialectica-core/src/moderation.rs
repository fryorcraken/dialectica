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
//! # Exactly one `Arrival` is read, and which one is the whole design
//!
//! This section said "nothing here reads an `Arrival`" and "no branch" until
//! architecture review caught it. Both became false with the `Hide`-wins
//! tie-break, and a stale paragraph here is worse than in most places: it is
//! what the author of a fourth resolver reads to learn the house discipline.
//!
//! What [`resolve`] actually does: filter [`OpLog::iter_target`] down to the
//! candidates that bind, then read **the leading candidate's**
//! [`Arrival`](crate::arrival::Arrival) — one `Arrival`, exactly once — and
//! branch on it. Where the transport ordered that op, its position is a real
//! last-write-wins answer and stands. Where it did not, the degraded order
//! carries no recency, and a `Hide` among the candidates decides instead. The
//! full statement is on [`resolve`]; the reasoning is in the change's
//! `design.md`.
//!
//! **Why the leader's, and not the read's.** The tempting alternative is to ask
//! whether the *sequence* was ordered — a property of what `iter_target`
//! returned. That is a different set of ops from the one whose leader decides,
//! because filtering happens in between: a read can contain unordered ops that
//! all fail authority, and the binding candidates left behind can be entirely
//! transport-ordered. Asking about the read would demote that case to the
//! degraded branch for no reason. The question this module needs is about the op
//! that is *about to decide*, so that is the op whose arrival it reads.
//!
//! **What survived from the old paragraph, because it is still true.** The
//! leading entry is taken because that is the position the ordering rule defines
//! as current — **not because it is the most recent**. `cmp_ops` leads with the
//! highest Lamport timestamp only where the transport supplied one; otherwise,
//! which is every op today, it falls back to *ascending op id*, an order
//! carrying no recency whatever. `log.rs`'s [`OpLog::iter_target`] and
//! [`cmp_ops`](crate::arrival::cmp_ops) both state this; it is not restated
//! here.
//!
//! The old paragraph then concluded that the same code is therefore correct
//! under both orders with no branch. That is the step that was wrong, and
//! `design.md` records why: it established *convergence* and treated convergence
//! as sufficient, never asking whether last-write-wins is meaningful at all when
//! there is no "last". It is not, and the branch above is the consequence.

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
///
/// # Where the transport ordered nothing, `Hide` wins the tie
///
/// Found by security review, and it is the one place this resolver departs from
/// "first entry wins".
///
/// A `Moderate` op is fully determined by `{stoa, author, target, action}` —
/// there is no nonce, no timestamp and no free byte. So for one Stoa, one
/// moderator and one target **exactly two ops can ever exist**, with two fixed
/// op ids. Every arrival today is unordered, so `cmp_ops` sorts by ascending op
/// id, and taking the first entry would mean *whichever id is lower wins
/// forever* — no matter who published first, no matter how often the other is
/// republished.
///
/// That is not last-write-wins degrading gracefully. It is a **pre-emptive
/// veto**: publish a bare `Unhide` naming an unmoderated target, discard the
/// key, and if that pair hashes the wrong way the target can never be hidden by
/// anyone. It is also grindable — the creator picks the Stoa title, the title
/// fixes the address, and the address is inside both ids.
///
/// So when **neither** candidate was ordered by the transport, a `Hide` beats an
/// `Unhide` regardless of op id. The asymmetry is deliberate and is the
/// fail-safe direction: an `Unhide` wrongly winning silently un-moderates
/// content with no recourse, where a `Hide` wrongly winning leaves something
/// hidden that a moderator can lift the moment real ordering arrives.
///
/// **Only candidates that already bind are eligible**, which is the point at
/// which this preference could silently undo the authority check above it. A
/// search over every op naming the target — rather than over the filtered
/// candidates — would let any peer publish a forged `Hide` and have every reader
/// report it, restoring §6.2's defect on the only path in use.
/// `the_hide_bias_searches_only_ops_that_already_bind` pins it, and is the one
/// fixture in this module combining unordered arrivals, a binding op and a
/// non-binding `Hide`.
///
/// **Confined to the degraded branch, keyed on the LEADING candidate.** Where
/// the transport ordered the leading op, §5.7's rule is real and last-write-wins
/// stands untouched — biasing there would make every hide permanent, which is a
/// worse bug than the one this closes.
/// `a_transport_ordered_unhide_still_reverses_a_hide` pins that.
///
/// The condition asks about `first` rather than about every candidate, and the
/// two differ on mixed arrivals — the normal state during a transport upgrade.
/// `first` being ordered means the leading position was won by a genuine
/// last-write-wins comparison, which is the entire reason not to second-guess
/// it; and since [`cmp_ops`](crate::arrival::cmp_ops) places every ordered op
/// ahead of every unordered one, an ordered leader means the ordered ops decided
/// among themselves. `the_ordered_branch_is_chosen_by_the_leading_op_not_by_all_of_them`
/// pins the distinction, which was previously a place the spec and the code
/// disagreed with no test able to tell.
///
/// **Convergence is preserved**, which is the property that would have made this
/// unacceptable. The bias is a pure function of the two ops' actions and their
/// recorded arrivals, so every peer holding the same ops computes the same
/// answer. It lives here rather than in [`cmp_ops`](crate::arrival::cmp_ops)
/// because it is moderation semantics: a general comparator has no business
/// knowing that one op kind's payload is safer to prefer.
pub fn resolve<'a, L: OpLog>(log: &'a L, moderators: &Moderators, target: &OpId) -> Moderation<'a> {
    let binding: Vec<&Entry> = log
        .iter_target(target)
        .into_iter()
        // The kind filter is the resolver's job, not the log's: `iter_target`
        // answers "what acts on this subject?" for every kind, so that a fifth
        // op kind does not widen the store's API. §5.7: a moderator's hide and
        // an author's edit "are about different things and do not contend".
        .filter(|e| matches!(e.op.op.kind, OpKind::Moderate { .. }) && moderators.authorises(e))
        .collect();

    let Some(first) = binding.first().copied() else {
        return Moderation::Unmoderated;
    };

    // Where the transport ordered the leading op, its position is a real
    // last-write-wins answer and nothing here second-guesses it.
    let deciding = if first.arrival.is_ordered_by_transport() {
        first
    } else {
        // Otherwise every candidate is in the degraded order, where position
        // carries no recency at all. Prefer a `Hide` if any binding one exists,
        // and fall back to the rule's first entry when none does.
        binding
            .iter()
            .copied()
            .find(|e| {
                matches!(
                    e.op.op.kind,
                    OpKind::Moderate {
                        action: ModerationAction::Hide,
                        ..
                    }
                )
            })
            .unwrap_or(first)
    };

    match deciding.op.op.kind {
        OpKind::Moderate {
            action: ModerationAction::Hide,
            ..
        } => Moderation::Hidden(deciding),
        _ => Moderation::Unhidden(deciding),
    }
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

    /// A Stoa whose hide/unhide pair hashes the OPPOSITE way to `agora()`'s.
    ///
    /// Returns `(genesis, hide, unhide, target)` with `hide.id() < unhide.id()`.
    ///
    /// Searched rather than hardcoded, for two reasons. Which of two SHA-256
    /// outputs is lower is not a fact a reader should take on trust, and a
    /// hardcoded guess that went stale would leave the test passing while
    /// exercising the wrong arrangement. And the search *is* the finding: the
    /// creator picks the title, the title fixes the Stoa address, and the
    /// address is inside both op ids — so a handful of titles is all it takes to
    /// choose which action wins the degraded order. Security review found one in
    /// four attempts; this loop is that, made repeatable.
    ///
    /// **The budget of 256 is chosen, not typed.** Each title is an independent
    /// ~50/50 trial, so exhausting it has probability ~2⁻²⁵⁶ — far below any
    /// rate at which a flaky test would be noticed, and below the collision
    /// probability of the hash itself. Exhaustion panics rather than skipping,
    /// so the impossible case is loud.
    fn a_stoa_where_the_hide_hashes_lower() -> (Genesis, SignedOp, SignedOp, OpId) {
        for n in 0..256u32 {
            let genesis = a_genesis(&creator(), &format!("Ground {n}"));
            let stoa = address_of(&genesis);
            let target = a_post(stoa, &a_key(2), "the subject").op.id();
            let hide = a_moderation(stoa, &creator(), target, ModerationAction::Hide);
            let unhide = a_moderation(stoa, &creator(), target, ModerationAction::Unhide);
            if hide.op.id() < unhide.op.id() {
                return (genesis, hide, unhide, target);
            }
        }
        panic!("no title in 256 attempts put the hide first; SHA-256 is not this biased");
    }

    /// A Stoa where an OUTSIDER's unhide of the target sorts below the
    /// moderator's hide.
    ///
    /// Returns `(genesis, hide, forged_unhide, target)`. Searched for the same
    /// reason as [`a_stoa_where_the_hide_hashes_lower`]: `agora()` happens to
    /// fall the other way, and asserting otherwise made the test that uses this
    /// fail on its first run rather than quietly stop exercising its own name.
    /// Same 256 budget, same ~2⁻²⁵⁶ exhaustion probability, same loud panic.
    fn a_stoa_where_a_forged_unhide_sorts_first() -> (Genesis, SignedOp, SignedOp, OpId) {
        for n in 0..256u32 {
            let genesis = a_genesis(&creator(), &format!("Contested {n}"));
            let stoa = address_of(&genesis);
            let target = a_post(stoa, &a_key(2), "the subject").op.id();
            let hide = a_moderation(stoa, &creator(), target, ModerationAction::Hide);
            let forged = a_moderation(stoa, &outsider(), target, ModerationAction::Unhide);
            if forged.op.id() < hide.op.id() {
                return (genesis, hide, forged, target);
            }
        }
        panic!("no title in 256 attempts put the forged unhide first");
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
    fn a_non_binding_op_leading_the_degraded_read_is_skipped() {
        // Named for what it proves, after blind spec-test review showed the
        // previous name was a false coverage claim — §6's "worse than a missing
        // test".
        //
        // It was `the_degraded_order_decides_when_the_transport_ordered_nothing`,
        // and it did not test that. Two `Unhide`s of one target need two distinct
        // authors, and in a one-moderator Stoa the second necessarily does not
        // bind — so after the authority filter the candidate vector holds exactly
        // ONE entry, `first` and `last` are the same, and the op-id fallback
        // decides nothing. Inverting the fallback left the whole suite green.
        //
        // What the fixture genuinely does exercise is still worth keeping: a
        // non-binding op sitting anywhere in the degraded read is skipped, and
        // the binding one is chosen and named.
        //
        // **NOT VERIFIABLE IN THIS CHANGE: the op-id fallback choosing between
        // two BINDING candidates in the degraded order.** That needs two ops that
        // both bind and differ, which needs two distinct moderators of one Stoa,
        // which needs the mutable moderator set §13 defers. The same blocker as
        // the two-moderator scenario in `spec.md`. When that set lands, this is
        // the fixture to add — do not assume the fallback is covered until then.
        let (mut log, target) = a_log_with_a_post();
        let binding = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Unhide,
        );
        let not_binding = a_moderation(
            address_of(&agora()),
            &outsider(),
            target,
            ModerationAction::Unhide,
        );
        let binding_id = binding.op.id();

        // Appended in the order that would be wrong if insertion order leaked.
        log.append(binding, Arrival::unordered());
        log.append(not_binding, Arrival::unordered());

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(!resolved.is_hidden());
        assert_eq!(
            resolved.deciding_op().map(|e| e.id()),
            Some(binding_id),
            "the binding unhide must decide, whatever the op-id order"
        );
    }

    #[test]
    fn an_unauthorised_op_does_not_displace_an_authorised_one_in_the_degraded_order() {
        // The degraded-order twin of `an_unauthorised_op_does_not_displace_an_
        // authorised_one`, which pins skip-and-continue only on the
        // transport-ordered branch — a branch production never reaches.
        //
        // Here both arrivals are unordered, so `cmp_ops` sorts by op id, and the
        // Stoa is SEARCHED for one where the forged unhide sorts first rather
        // than assumed. `agora()` is not such a Stoa — asserting it blindly made
        // this test fail on its first run, which is the fixture guard doing its
        // job. Without the search the genuine hide would lead anyway, and
        // take-then-validate would agree with skip-and-continue.
        let (stoa, hide, forged_unhide, target) = a_stoa_where_a_forged_unhide_sorts_first();
        assert!(
            forged_unhide.op.id() < hide.op.id(),
            "the search must have found the forgery sorting first"
        );
        let hide_id = hide.op.id();

        let mut log = MemoryOpLog::new();
        log.append(forged_unhide, Arrival::unordered());
        log.append(hide, Arrival::unordered());

        let resolved = resolve(&log, &moderators_of(&stoa), &target);
        assert!(
            resolved.is_hidden(),
            "a forgery leading the degraded order displaced a genuine hide"
        );
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(hide_id));
    }

    #[test]
    fn a_hide_is_not_defeated_by_the_unhide_hashing_lower() {
        // THE pre-emptive veto, found by security review.
        //
        // A `Moderate` op is fully determined by {stoa, author, target, action}
        // — no nonce, no timestamp, no free byte. So for one Stoa, one
        // moderator and one target there exist EXACTLY TWO ops, with two fixed
        // op ids. Every arrival today is `unordered()`, so `cmp_ops` sorts by
        // ascending op id and whichever id is lower would win *forever*:
        // regardless of publication order, regardless of republishing.
        //
        // That makes a bare `Unhide` a permanent veto. Publish one naming a
        // target nobody has moderated, then discard the key; if that target's
        // unhide id sorts below its hide id — about half of targets, and
        // grindable through the Stoa title, which the creator chooses and which
        // is inside both ids — the target can never be hidden by anyone, ever.
        // No op the moderator could publish wins, because only two exist and the
        // attacker took the lower one.
        //
        // This fixture is the shipped `agora()`, where the unhide genuinely does
        // hash lower — DETERMINED below, not assumed, so the test fails loudly
        // rather than silently stops exercising the veto if a fixture changes.
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
        assert!(
            unhide.op.id() < hide.op.id(),
            "this fixture must be one where the unhide sorts FIRST, or it does \
             not exercise the veto at all"
        );
        let hide_id = hide.op.id();

        // Both unordered: the only order production reaches today. The unhide is
        // appended FIRST, so insertion order cannot produce the expected answer
        // by accident either.
        log.append(unhide, Arrival::unordered());
        log.append(hide, Arrival::unordered());

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(
            resolved.is_hidden(),
            "an unhide that merely hashes lower defeated a hide permanently"
        );
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(hide_id));
    }

    #[test]
    fn the_hide_bias_applies_whichever_way_the_hashes_fall() {
        // The other half, and the one that stops the test above passing for a
        // resolver that simply always reports Hidden when it sees a Hide at all.
        //
        // Here the fixture is chosen so the HIDE hashes lower — so the untouched
        // op-id order already yields Hidden, and the bias changes nothing. Both
        // arrangements must give the same answer; that is what "the bias removes
        // the coin flip" means, as opposed to "the bias flips the coin".
        //
        // The Stoa is searched for rather than assumed, because which of two
        // hashes is lower is not something a reader should take on trust — and
        // because the search itself demonstrates the grindability the finding
        // rests on.
        let (stoa, hide, unhide, target) = a_stoa_where_the_hide_hashes_lower();
        assert!(
            hide.op.id() < unhide.op.id(),
            "the search must have found the opposite arrangement"
        );
        let hide_id = hide.op.id();

        let mut log = MemoryOpLog::new();
        log.append(hide, Arrival::unordered());
        log.append(unhide, Arrival::unordered());

        let resolved = resolve(&log, &moderators_of(&stoa), &target);
        assert!(resolved.is_hidden());
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(hide_id));
    }

    #[test]
    fn the_hide_bias_searches_only_ops_that_already_bind() {
        // THE test that was missing, found by blind spec-test review, and the
        // most dangerous gap in this module's suite.
        //
        // The `Hide`-wins tie-break searches for a `Hide` among the candidates.
        // If it searched `iter_target` — everything naming the target, validated
        // or not — instead of the already-filtered `binding` vector, then ANY
        // peer could publish a forged or unauthorised `Hide` of any target and
        // every conforming peer would report `Hidden`, naming the forgery as the
        // deciding op. That is §6.2's "any peer can forge a moderation"
        // reinstated verbatim, on the degraded path, which is the ONLY path
        // production runs today.
        //
        // The whole suite stayed green under that mutation, because the two
        // hostile-input tests are blind in different ways:
        //
        //   - `a_log_full_of_forgeries_...` holds NO binding op, so the fold
        //     returns `Unmoderated` before the bias is reached at all;
        //   - `one_genuine_hide_among_the_forgeries_still_binds` uses ordered
        //     arrivals throughout, so it takes the transport-ordered branch and
        //     never reaches the bias either.
        //
        // So this fixture is the combination neither had, and it is exactly the
        // degraded security surface: UNORDERED arrivals, at least one BINDING
        // op, and at least one NON-BINDING `Hide`. Each of the three is load
        // bearing; drop any one and the mutation survives again.
        let (mut log, target) = a_log_with_a_post();

        // Binds: the creator's unhide, in the right Stoa, correctly signed.
        let binding_unhide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Unhide,
        );
        let binding_id = binding_unhide.op.id();

        // Does NOT bind, and each fails a different one of the three checks, so
        // no single check carries this test on its own.
        let unauthorised_hide = a_moderation(
            address_of(&agora()),
            &outsider(),
            target,
            ModerationAction::Hide,
        );
        assert!(
            unauthorised_hide.verify(),
            "the fixture must be AUTHENTIC and merely unauthorised"
        );
        let forged_hide = a_forged_moderation(
            address_of(&agora()),
            &creator().public_key(),
            &outsider(),
            target,
            ModerationAction::Hide,
        );
        let lyceum = a_genesis(&creator(), "Lyceum");
        let cross_stoa_hide = a_moderation(
            address_of(&lyceum),
            &creator(),
            target,
            ModerationAction::Hide,
        );

        for op in [
            binding_unhide,
            unauthorised_hide,
            forged_hide,
            cross_stoa_hide,
        ] {
            log.append(op, Arrival::unordered());
        }

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(
            !resolved.is_hidden(),
            "a Hide that does not bind won the tie-break — any peer can now \
             forge a moderation on the degraded path"
        );
        assert_eq!(
            resolved.deciding_op().map(|e| e.id()),
            Some(binding_id),
            "the binding op must decide, and must be the one named"
        );
    }

    #[test]
    fn a_hide_that_binds_still_wins_over_hides_that_do_not() {
        // The complement, so the test above cannot be satisfied by a resolver
        // that simply never reports `Hidden` on the degraded path. Same shape,
        // plus one genuine `Hide` — which must win, and must be the op named
        // rather than any of the three impostors.
        let (mut log, target) = a_log_with_a_post();
        let lyceum = a_genesis(&creator(), "Lyceum");

        let binding_hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );
        let binding_id = binding_hide.op.id();

        for op in [
            binding_hide,
            a_moderation(
                address_of(&agora()),
                &outsider(),
                target,
                ModerationAction::Hide,
            ),
            a_forged_moderation(
                address_of(&agora()),
                &creator().public_key(),
                &outsider(),
                target,
                ModerationAction::Hide,
            ),
            a_moderation(
                address_of(&lyceum),
                &creator(),
                target,
                ModerationAction::Hide,
            ),
        ] {
            log.append(op, Arrival::unordered());
        }

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(resolved.is_hidden());
        assert_eq!(
            resolved.deciding_op().map(|e| e.id()),
            Some(binding_id),
            "an impostor Hide was named as the deciding op"
        );
    }

    #[test]
    fn the_ordered_branch_is_chosen_by_the_leading_op_not_by_all_of_them() {
        // NO SPEC — now specified; this pins the reading.
        //
        // The spec said "where no competing moderation was ordered by the
        // transport", a predicate over ALL candidates; the code asks only
        // whether the LEADING one was ordered. Blind review found the two
        // disagree on mixed arrivals — reachable today through
        // `Arrival::from_parts` and the normal state during a transport upgrade
        // — and that swapping the code to the spec's `any(...)` form broke no
        // test.
        //
        // The code is right: `first` being transport-ordered means the leading
        // position IS a genuine last-write-wins answer, which is the entire
        // rationale for not second-guessing it. A candidate further down being
        // unordered says nothing about the leading one, and `cmp_ops` already
        // places every ordered op ahead of every unordered one — so an ordered
        // leader means the ordered ops won on their own terms. The spec has
        // been tightened to say this.
        //
        // The fixture: an ordered `Unhide` leading, an unordered `Hide` behind
        // it. Under the code the ordered leader decides and the target is NOT
        // hidden; under the spec's old `any(...)` reading the mixed set would
        // take the degraded branch and the `Hide` would win.
        let (mut log, target) = a_log_with_a_post();
        let ordered_unhide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Unhide,
        );
        let unhide_id = ordered_unhide.op.id();
        let unordered_hide = a_moderation(
            address_of(&agora()),
            &creator(),
            target,
            ModerationAction::Hide,
        );

        log.append(ordered_unhide, Arrival::ordered(5, a_message_id(1)));
        log.append(unordered_hide, Arrival::unordered());

        // The fixture must really be mixed, or it exercises neither reading.
        let entries = log.iter_target(&target);
        assert!(
            entries.iter().any(|e| e.arrival.is_ordered_by_transport())
                && entries.iter().any(|e| !e.arrival.is_ordered_by_transport()),
            "the fixture must carry BOTH arrival kinds"
        );

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(
            !resolved.is_hidden(),
            "an ordered leading op must decide on its own terms, rather than \
             being demoted to the degraded branch by an unordered straggler"
        );
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(unhide_id));
    }

    #[test]
    fn a_transport_ordered_unhide_still_reverses_a_hide() {
        // The bias must be confined to the DEGRADED branch. Where the transport
        // did order the ops, §5.7's rule is real and last-write-wins must stand
        // — otherwise this "fix" would make every hide permanent the moment
        // ordering arrives, which is a worse bug than the one it closes.
        //
        // The unhide carries the higher Lamport value AND hashes lower, so a
        // resolver that applied the bias unconditionally would report Hidden.
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
        assert!(
            unhide.op.id() < hide.op.id(),
            "the fixture must be one the bias WOULD have caught"
        );
        let unhide_id = unhide.op.id();

        log.append(hide, Arrival::ordered(2, a_message_id(1)));
        log.append(unhide, Arrival::ordered(3, a_message_id(1)));

        let resolved = resolve(&log, &moderators_of(&agora()), &target);
        assert!(
            !resolved.is_hidden(),
            "a real Lamport order must still let an unhide reverse a hide"
        );
        assert_eq!(resolved.deciding_op().map(|e| e.id()), Some(unhide_id));
    }

    #[test]
    fn the_authority_predicate_consults_the_set_and_not_the_earlier_ops_author() {
        // Named for what the body supports, which is narrower than the rule it
        // motivates. It was called `a_moderator_may_reverse_a_moderation_they_
        // did_not_place`, and security review was right that the name overstated
        // it: with ONE moderator in the fixture, "any moderator may reverse any
        // moderation" and "only the placing moderator may" give identical
        // answers. §6's fixture trap, and a reader auditing whether the
        // any-moderator rule is tested would have found this and stopped looking.
        //
        // NO SPEC: any moderator may reverse any moderation, rather than only
        // the moderator who placed it. §6 makes an op valid when signed by "a
        // current moderator" and says nothing about ownership of an earlier op.
        // Chosen this way because the alternative — moderations owned by their
        // author — would make a compromised or departed moderator's hide
        // permanent, which §6.2's whole direction (raising the cost of one rogue
        // key) argues against.
        //
        // **That choice is UNTESTABLE today and is not tested here.** It needs
        // two distinct moderators, which needs a mutable moderator set, which
        // does not exist (§13 defers it). What IS testable, and is what this
        // asserts, is the structural precondition: `authorises` consults the
        // moderator set and nothing about the ops already in the log, so it
        // cannot be conditioning on who placed what. That is necessary for the
        // rule and not sufficient for it.
        //
        // The blast radius of the choice, which belongs with it: **one
        // compromised moderator key can `Unhide` every moderation in the Stoa**,
        // and with a single creator-moderator there is no recovery short of
        // forking. §6.2's threshold certificates are the intended answer — an
        // action takes N of M signatures, so one key is not enough — and until
        // they land this is the cost of the reversibility that §13 asked for.
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
        // `Unmoderated`. The spec makes the leading binding moderation decide
        // and says nothing about what it must follow. Chosen this way because
        // the alternative needs the resolver to know whether a `Hide` it may
        // never have received existed — §3.3's partial set makes that
        // undecidable, and a rule no peer can evaluate is not a rule.
        //
        // The marker above covers the REPRESENTATIONAL choice only, and stops
        // there deliberately.
        //
        // What a bare `Unhide` *does* was once under this marker too, and blind
        // review was right that it no longer belongs: it is **specified and
        // tested**, so leaving it here would tell a future reader "nobody
        // decided this" about something decided. A bare `Unhide` is a live op
        // competing in the order, and under the degraded order it would have
        // vetoed every future `Hide` of its target — closed by `resolve`'s
        // `Hide`-wins tie-break, required by the spec, and pinned by
        // `a_hide_is_not_defeated_by_the_unhide_hashing_lower`. Not a `NO SPEC:`
        // item; a cross-reference.
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
