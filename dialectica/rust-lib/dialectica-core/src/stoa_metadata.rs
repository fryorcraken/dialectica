//! What is this Stoa called today? The read side of the `StoaMetadata` op.
//!
//! # Why this file exists
//!
//! [`OpKind::StoaMetadata`] carries a Stoa's current title and description, and
//! every peer stores the ones it receives. Until this module nothing read them,
//! so every title the module reported was the **founding** one from the genesis
//! record, however long ago that was chosen.
//!
//! The contract is the `stoa-metadata` spec's "Current metadata resolves by
//! last-write-wins, falling back to genesis"; the reasoning is in the `get-stoa`
//! change's `design.md`. What is repeated here is what a reader of this code
//! needs in order not to undo it.
//!
//! # The moderation resolver with a different subject
//!
//! [`crate::moderation::resolve`] answers "is this op hidden?" by reading the
//! ops that name a *target op*. This answers "what is this Stoa called?" by
//! reading the ops that name a *Stoa* — [`OpLog::iter_stoa`], because a metadata
//! op's [`Entry::target`] is `None` by design. Everything else is the same
//! machinery: the same order, the same three binding checks, run by the same
//! function, [`Moderators::authorises`].
//!
//! # Nothing here orders anything
//!
//! `iter_stoa` returns ops in [`crate::arrival::cmp_ops`] order, which is the
//! `op-ordering` capability's rule. The first binding entry is the leading one
//! and it decides. There is no re-sort, no tiebreak and — unlike the moderation
//! resolver — no degraded-order preference: the spec forbids this capability an
//! ordering of its own, and two titles have no fail-safe direction to prefer.
//! `design.md` decision 3 has the comparison.

use crate::identity::Address;
use crate::log::{Entry, OpLog, OpLogError};
use crate::moderation::Moderators;
use crate::op::{OpId, OpKind};
use crate::stoa::{Genesis, GenesisError};

/// What a genesis record fixes that metadata resolution needs: who may rename
/// the Stoa, and what to fall back to when nobody has.
///
/// # One record, so the two halves cannot come from two Stoas
///
/// [`resolve`] needs the moderator set and the founding title. Passed as two
/// arguments, a caller could pair one Stoa's moderators with another Stoa's
/// title, and the only symptom would be a wrong fallback. Built here from one
/// [`Genesis`], they cannot disagree — the same reason [`Moderators`] holds its
/// Stoa address rather than taking it beside each call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Founding {
    moderators: Moderators,
    title: String,
}

impl Founding {
    /// The moderator set and founding title of one genesis record.
    ///
    /// **The only constructor.** Fallible for exactly the reason
    /// [`Moderators::of`] is: a record whose title is over the genesis cap has
    /// no encoding, so no address, so it names no Stoa any op could name.
    ///
    /// **A record obtained from a peer must be verified against its address
    /// first** — [`crate::membership::Membership::verified`], which the wire
    /// reaches through `genesis_for`. This cannot check it: a substituted record
    /// computes its own address consistently.
    pub fn of(genesis: &Genesis) -> Result<Self, GenesisError> {
        Ok(Founding {
            moderators: Moderators::of(genesis)?,
            title: genesis.title.clone(),
        })
    }

    /// The Stoa this record founded.
    pub fn stoa(&self) -> &Address {
        self.moderators.stoa()
    }
}

/// A Stoa's current display metadata, and whether it came from an op.
///
/// # An enum, so the flag cannot disagree with the values
///
/// `is_genesis_fallback` is not a stored boolean beside a title and a
/// description. It is *which variant this is*, so a fallback carrying an op's
/// description, or an op's values flagged as a fallback, cannot be built. That
/// matters because the flag is the only thing a reader has for telling "this is
/// the current name" from "nobody here has seen a rename, and this may be years
/// stale" — `design.md` decision 12.
///
/// # The op's strings, not the op
///
/// [`CurrentMetadata::Declared`] holds the title and description the op
/// carried, extracted when the op was chosen. [`crate::moderation::Moderation`]
/// holds the deciding [`Entry`] instead, and pays for it with a `_` arm for
/// kinds its filter already excluded. Holding the strings makes a non-metadata
/// op deciding this unrepresentable rather than unreachable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrentMetadata {
    /// No binding metadata op is held for this Stoa. The values are the genesis
    /// record's: its title, and no description, because the record has none.
    ///
    /// **Not "nobody has renamed it."** A rename that has not reached this peer
    /// looks exactly like this. It says what this peer holds, and nothing more.
    Founding { title: String },
    /// The leading binding metadata op decided, and these are its values
    /// exactly as it carried them — **including an empty title**, which is a
    /// title and not an absence.
    Declared {
        title: String,
        description: String,
        /// The op that decided. Not on the wire; carried so that a test can
        /// name which op won rather than inferring it from the strings.
        decided_by: OpId,
    },
}

impl CurrentMetadata {
    /// The title to display.
    pub fn title(&self) -> &str {
        match self {
            CurrentMetadata::Founding { title } | CurrentMetadata::Declared { title, .. } => title,
        }
    }

    /// The description to display; empty on a fallback, since the genesis
    /// record carries none.
    pub fn description(&self) -> &str {
        match self {
            CurrentMetadata::Founding { .. } => "",
            CurrentMetadata::Declared { description, .. } => description,
        }
    }

    /// Whether no binding metadata op was held, so the values are the genesis
    /// record's.
    pub fn is_genesis_fallback(&self) -> bool {
        matches!(self, CurrentMetadata::Founding { .. })
    }
}

/// The title and description of an entry, if it is a metadata op that binds
/// under `moderators`.
///
/// **One function for the kind filter and the authority check**, so that the
/// resolver's filter is one call and "is the kind checked before the first
/// entry is taken?" has one place to look. Neither half may be skipped: an op of
/// another kind signed by the creator must not rename the Stoa, and a metadata
/// op signed by anyone else must not either.
fn binding_metadata(moderators: &Moderators, entry: &Entry) -> Option<CurrentMetadata> {
    // The kind FIRST, because it is free and `authorises` verifies a signature:
    // in a Stoa whose creator posts often, checking authority first would verify
    // every one of those posts on every resolution to learn nothing.
    let (title, description) = match &entry.op.op.kind {
        OpKind::StoaMetadata { title, description } => (title, description),
        // Every other kind: a post, a revision, a vote or a moderation says
        // nothing about what the Stoa is called, whoever signed it. Named rather
        // than `_` so that a new kind forces a decision here.
        OpKind::Post { .. }
        | OpKind::Revise { .. }
        | OpKind::Moderate { .. }
        | OpKind::Vote { .. } => return None,
    };
    if !moderators.authorises(entry) {
        return None;
    }
    // Cloned only here, for the one op that binds: `find_map` stops at it.
    Some(CurrentMetadata::Declared {
        title: title.clone(),
        description: description.clone(),
        decided_by: entry.id(),
    })
}

/// Resolve a Stoa's current display metadata from the ops this peer holds.
///
/// # The rule, in the order it applies
///
/// Walk [`OpLog::iter_stoa`], already in `cmp_ops` order; keep the first entry
/// that is a `StoaMetadata` op **and** passes [`Moderators::authorises`]; report
/// its values. Where none does, report the genesis values as a fallback.
///
/// **Ops that do not bind do not take part in the ordering at all.** A forged
/// or unauthorised op carrying a higher counter is skipped, not taken and then
/// rejected — a resolver that took the leading metadata op and then checked it
/// would fall back to the founding title whenever anyone published anything
/// that sorted first.
///
/// # Every check, every time
///
/// Nothing here is cached and nothing was decided at append time: the log
/// stores forgeries deliberately, and whether an op binds is decided on every
/// read. The scope check inside `authorises` is kept even though `iter_stoa`
/// has already narrowed to this Stoa — see `design.md` decision 4 for why, and
/// `the_resolver_does_not_trust_the_read_to_have_scoped_the_ops` for the test
/// that sees it.
///
/// # A read failure is not a fallback
///
/// `Err` means the store could not be consulted. A fallback would state that
/// the peer holds no binding op, which it cannot know — so a broken store would
/// be indistinguishable from a Stoa nobody has renamed. `design.md` decision 7.
pub fn resolve<L: OpLog>(log: &L, founding: &Founding) -> Result<CurrentMetadata, OpLogError> {
    // `find_map`, not `next()` then a check: the first entry that BINDS, which is
    // not the first entry. See "Ops that do not bind" above.
    let leading = log
        .iter_stoa(founding.stoa())?
        .into_iter()
        .find_map(|entry| binding_metadata(&founding.moderators, &entry));
    Ok(leading.unwrap_or_else(|| CurrentMetadata::Founding {
        title: founding.title.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arrival::Arrival;
    use crate::identity::{sign_op_bytes, PublicKey, SecretKey};
    use crate::log::{Appended, MemoryOpLog};
    use crate::op::{ModerationAction, Op, OpClock, SignedOp, VoteDirection, MAX_FIELD_LEN};
    use crate::stoa::Policy;

    fn a_key(seed: u8) -> SecretKey {
        SecretKey::from_bytes(&[seed; 32]).unwrap()
    }

    /// The creator of every fixture Stoa unless a test says otherwise, and so
    /// its sole moderator.
    fn creator() -> SecretKey {
        a_key(1)
    }

    /// A key that moderates nothing in these fixtures.
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

    fn agora() -> Genesis {
        a_genesis(&creator(), "Agora")
    }

    fn address_of(genesis: &Genesis) -> Address {
        genesis
            .address()
            .expect("every fixture title here is a short literal, far under the genesis cap")
    }

    fn founding_of(genesis: &Genesis) -> Founding {
        Founding::of(genesis)
            .expect("every fixture title here is a short literal, far under the genesis cap")
    }

    /// A metadata op signed by `signer`, naming `signer` as author, carrying
    /// `counter` (or none).
    ///
    /// The wall-clock is the same on every op this builds, so no ordering test
    /// can pass because two ops differed in a field the order must not read.
    fn a_rename(
        stoa: Address,
        signer: &SecretKey,
        counter: Option<u64>,
        title: &str,
        description: &str,
    ) -> SignedOp {
        Op {
            stoa,
            author: signer.public_key(),
            clock: counter.map(|counter| OpClock {
                counter,
                asserted_ms: 1_789_729_304_000,
            }),
            kind: OpKind::StoaMetadata {
                title: title.to_string(),
                description: description.to_string(),
            },
        }
        .sign(signer)
    }

    /// A metadata op CLAIMING `claimed_author` but signed by `actual_signer`.
    ///
    /// Asserts both halves of what makes it an authorship forgery rather than
    /// junk bytes: it fails `verify`, and the signature IS valid under the key
    /// that actually made it. Without the second, a caller would be showing "a
    /// bad signature is refused" rather than "a real signature by the wrong key
    /// is refused" — the same guard `moderation.rs`'s `a_forged_moderation`
    /// carries, for the same reason.
    fn a_forged_rename(
        stoa: Address,
        claimed_author: &PublicKey,
        actual_signer: &SecretKey,
        counter: Option<u64>,
        title: &str,
    ) -> SignedOp {
        let op = Op {
            stoa,
            author: claimed_author.clone(),
            clock: counter.map(|counter| OpClock {
                counter,
                asserted_ms: 1_789_729_304_000,
            }),
            kind: OpKind::StoaMetadata {
                title: title.to_string(),
                description: "forged".to_string(),
            },
        };
        let forged = SignedOp {
            signature: sign_op_bytes(actual_signer, &op.canonical_bytes()),
            op,
        };
        assert!(!forged.verify(), "the fixture must be an actual forgery");
        assert!(
            crate::identity::verify_authored_op(
                &actual_signer.public_key().to_bytes(),
                &forged.op.canonical_bytes(),
                &forged.signature.to_bytes()
            ),
            "the signature must be valid under the actual signer's key, or this is \
             junk rather than a forgery"
        );
        forged
    }

    fn a_log_of(ops: impl IntoIterator<Item = SignedOp>) -> MemoryOpLog {
        let mut log = MemoryOpLog::new();
        for op in ops {
            assert_eq!(
                log.append(op, Arrival::unordered()).unwrap(),
                Appended::Stored,
                "a fixture op must be distinct, or the test holds fewer ops than it says"
            );
        }
        log
    }

    fn resolved(log: &MemoryOpLog, genesis: &Genesis) -> CurrentMetadata {
        resolve(log, &founding_of(genesis)).expect("a MemoryOpLog cannot fail")
    }

    // ─── The fallback ──────────────────────────────────────────────────────

    #[test]
    fn a_stoa_with_no_metadata_op_resolves_to_its_founding_values() {
        let current = resolved(&MemoryOpLog::new(), &agora());
        assert_eq!(
            current,
            CurrentMetadata::Founding {
                title: "Agora".to_string()
            }
        );
        assert_eq!(current.title(), "Agora");
        assert_eq!(current.description(), "");
        assert!(current.is_genesis_fallback());
    }

    #[test]
    fn the_fallback_title_is_this_records_title() {
        // The fallback must come from the record asked about, not from any
        // fixed value — two titles, two answers. A resolver that returned a
        // constant would pass the test above for one of them.
        for title in ["Agora", "Lyceum", ""] {
            let genesis = a_genesis(&creator(), title);
            assert_eq!(resolved(&MemoryOpLog::new(), &genesis).title(), title);
        }
    }

    // ─── A binding op supplies the values ─────────────────────────────────

    #[test]
    fn the_creators_metadata_op_supplies_the_current_values() {
        let stoa = address_of(&agora());
        let rename = a_rename(
            stoa,
            &creator(),
            Some(1),
            "Stoa Poikile",
            "the painted porch",
        );
        let id = rename.op.id();
        let current = resolved(&a_log_of([rename]), &agora());

        assert_eq!(
            current,
            CurrentMetadata::Declared {
                title: "Stoa Poikile".to_string(),
                description: "the painted porch".to_string(),
                decided_by: id,
            }
        );
        assert!(!current.is_genesis_fallback());
    }

    #[test]
    fn a_higher_counter_supersedes_a_lower_one() {
        let stoa = address_of(&agora());
        let earlier = a_rename(stoa, &creator(), Some(2), "Earlier", "first");
        let later = a_rename(stoa, &creator(), Some(3), "Later", "second");
        let later_id = later.op.id();

        // Both append sequences, so the answer cannot be the last-appended op.
        for log in [
            a_log_of([earlier.clone(), later.clone()]),
            a_log_of([later.clone(), earlier.clone()]),
        ] {
            let current = resolved(&log, &agora());
            assert_eq!(current.title(), "Later");
            assert_eq!(current.description(), "second");
            assert!(matches!(
                current,
                CurrentMetadata::Declared { decided_by, .. } if decided_by == later_id
            ));
        }
    }

    #[test]
    fn a_later_rename_wins_whatever_the_two_op_ids_are() {
        // Run both ways round: the higher-counter op with the higher id, and
        // with the lower id. A resolver ordering by op id gets one wrong.
        let stoa = address_of(&agora());
        for later_has_higher_id in [true, false] {
            let (earlier, later) = (0u64..1000)
                .find_map(|n| {
                    let earlier = a_rename(stoa, &creator(), Some(n * 2 + 1), "Earlier", "");
                    let later = a_rename(stoa, &creator(), Some(n * 2 + 2), "Later", "");
                    let higher = later.op.id() > earlier.op.id();
                    (higher == later_has_higher_id).then_some((earlier, later))
                })
                .expect("a pair ranking either way exists within 1000 candidates");

            let current = resolved(&a_log_of([earlier, later]), &agora());
            assert_eq!(
                current.title(),
                "Later",
                "the higher counter must decide whether its op id is higher \
                 ({later_has_higher_id}) or lower"
            );
        }
    }

    #[test]
    fn an_op_carrying_a_counter_leads_one_carrying_none() {
        // Both id relations again: `cmp_ops` places every counter-carrying op
        // ahead of every counter-less one, whatever their ids.
        let stoa = address_of(&agora());
        for counted_has_higher_id in [true, false] {
            let (uncounted, counted) = (0u64..1000)
                .find_map(|n| {
                    let uncounted = a_rename(stoa, &creator(), None, "Uncounted", "");
                    let counted = a_rename(stoa, &creator(), Some(n), "Counted", "");
                    let higher = counted.op.id() > uncounted.op.id();
                    (higher == counted_has_higher_id).then_some((uncounted, counted))
                })
                .expect("a pair ranking either way exists within 1000 candidates");

            let current = resolved(&a_log_of([uncounted, counted]), &agora());
            assert_eq!(
                current.title(),
                "Counted",
                "the op carrying a counter must lead whether its id is higher \
                 ({counted_has_higher_id}) or lower"
            );
        }
    }

    // ─── Ops that do not bind ─────────────────────────────────────────────

    #[test]
    fn a_metadata_op_by_anyone_but_the_creator_does_not_bind() {
        let rename = a_rename(address_of(&agora()), &outsider(), Some(1), "Hijacked", "");
        // AUTHENTIC, or this would be testing the signature check rather than
        // the authority check.
        assert!(
            rename.verify(),
            "the fixture must be authentic with no authority"
        );

        let current = resolved(&a_log_of([rename]), &agora());
        assert_eq!(current.title(), "Agora");
        assert!(current.is_genesis_fallback());
    }

    #[test]
    fn a_metadata_op_forging_the_creators_authorship_does_not_bind() {
        // The other half: the named author IS the creator, so the authority
        // check alone would accept it. Only the signature check refuses it.
        let forged = a_forged_rename(
            address_of(&agora()),
            &creator().public_key(),
            &outsider(),
            Some(1),
            "Hijacked",
        );
        assert!(founding_of(&agora()).moderators.contains(&forged.op.author));

        let current = resolved(&a_log_of([forged]), &agora());
        assert_eq!(current.title(), "Agora");
        assert!(current.is_genesis_fallback());
    }

    #[test]
    fn an_unauthorised_higher_counter_op_does_not_displace_a_binding_one() {
        // The fixture that makes "skip and continue" and "take the leader, then
        // check it" disagree: the non-binding op sorts FIRST.
        let stoa = address_of(&agora());
        let genuine = a_rename(stoa, &creator(), Some(1), "Genuine", "kept");
        let unauthorised = a_rename(stoa, &outsider(), Some(99), "Hijacked", "");
        assert!(unauthorised.verify());

        let current = resolved(&a_log_of([genuine, unauthorised]), &agora());
        assert_eq!(current.title(), "Genuine");
        assert_eq!(current.description(), "kept");
    }

    #[test]
    fn a_forged_higher_counter_op_does_not_displace_a_binding_one() {
        let stoa = address_of(&agora());
        let genuine = a_rename(stoa, &creator(), Some(1), "Genuine", "kept");
        let forged = a_forged_rename(
            stoa,
            &creator().public_key(),
            &outsider(),
            Some(99),
            "Hijacked",
        );

        let current = resolved(&a_log_of([genuine, forged]), &agora());
        assert_eq!(current.title(), "Genuine");
    }

    #[test]
    fn a_creators_metadata_op_naming_another_stoa_does_not_rename_this_one() {
        // ONE creator, TWO Stoas — the fixture in which only the Stoa comparison
        // separates the ops. With two creators, the authority check alone would.
        let lyceum = a_genesis(&creator(), "Lyceum");
        assert_ne!(address_of(&agora()), address_of(&lyceum));
        let rename = a_rename(
            address_of(&lyceum),
            &creator(),
            Some(1),
            "Peripatos",
            "walks",
        );

        let log = a_log_of([rename]);
        let under_agora = resolved(&log, &agora());
        assert_eq!(under_agora.title(), "Agora");
        assert!(under_agora.is_genesis_fallback());

        let under_lyceum = resolved(&log, &lyceum);
        assert_eq!(under_lyceum.title(), "Peripatos");
        assert!(!under_lyceum.is_genesis_fallback());
    }

    /// A log whose `iter_stoa` ignores its argument and returns every op.
    ///
    /// A broken store, on purpose. Every correct `OpLog` narrows `iter_stoa` to
    /// the Stoa asked for, which makes the scope check inside
    /// `Moderators::authorises` refuse nothing the read did not already exclude
    /// — so no test through a correct log can see that check. This one can.
    struct UnscopedLog(MemoryOpLog);

    impl OpLog for UnscopedLog {
        fn append(&mut self, op: SignedOp, arrival: Arrival) -> Result<Appended, OpLogError> {
            self.0.append(op, arrival)
        }
        fn get(&self, id: &OpId) -> Result<Option<Entry>, OpLogError> {
            self.0.get(id)
        }
        fn iter(&self) -> Result<Vec<Entry>, OpLogError> {
            self.0.iter()
        }
        fn iter_stoa(&self, _stoa: &Address) -> Result<Vec<Entry>, OpLogError> {
            self.0.iter()
        }
        fn iter_target(&self, target: &OpId) -> Result<Vec<Entry>, OpLogError> {
            self.0.iter_target(target)
        }
        fn len(&self) -> Result<usize, OpLogError> {
            self.0.len()
        }
    }

    #[test]
    fn the_resolver_does_not_trust_the_read_to_have_scoped_the_ops() {
        // The spec: whether an op binds "MUST be decided each time a Stoa's
        // metadata is resolved", including that "the Stoa the op itself names is
        // the Stoa being resolved". Handed a read that did not narrow, the
        // resolver must still refuse the other Stoa's op. Deleting the scope
        // term from `authorises` turns this test red, and only this one.
        let lyceum = a_genesis(&creator(), "Lyceum");
        let elsewhere = a_rename(address_of(&lyceum), &creator(), Some(1), "Peripatos", "");
        let log = UnscopedLog(a_log_of([elsewhere]));

        let current = resolve(&log, &founding_of(&agora())).unwrap();
        assert_eq!(current.title(), "Agora");
        assert!(current.is_genesis_fallback());
    }

    #[test]
    fn a_stoa_whose_only_metadata_ops_fail_to_bind_resolves_exactly_as_one_with_none() {
        let stoa = address_of(&agora());
        let log = a_log_of([
            a_rename(stoa, &outsider(), Some(5), "Outsider", ""),
            a_forged_rename(
                stoa,
                &creator().public_key(),
                &outsider(),
                Some(6),
                "Forged",
            ),
            a_rename(
                address_of(&a_genesis(&creator(), "Lyceum")),
                &creator(),
                Some(7),
                "Elsewhere",
                "",
            ),
        ]);
        assert_eq!(
            resolved(&log, &agora()),
            resolved(&MemoryOpLog::new(), &agora())
        );
    }

    // ─── Only the metadata kind decides ───────────────────────────────────

    /// One op of every other kind, by the creator, in `stoa`, each carrying a
    /// counter HIGHER than anything a metadata op in these tests carries — so a
    /// resolver that took the leading entry of any kind would find one of these.
    fn the_creators_other_ops(stoa: Address) -> Vec<SignedOp> {
        let post = Op {
            stoa,
            author: creator().public_key(),
            clock: Some(OpClock {
                counter: 100,
                asserted_ms: 1_789_729_304_000,
            }),
            kind: OpKind::Post {
                thread: None,
                parent: None,
                body: "a post titled like a rename".to_string(),
                attachments: vec![],
            },
        }
        .sign(&creator());
        let target = post.op.id();
        let others = [
            OpKind::Revise {
                target,
                body: "a revision".to_string(),
                attachments: vec![],
            },
            OpKind::Moderate {
                target,
                action: ModerationAction::Hide,
            },
            OpKind::Vote {
                target,
                direction: VoteDirection::Up,
            },
        ];
        let mut ops = vec![post];
        for (i, kind) in others.into_iter().enumerate() {
            ops.push(
                Op {
                    stoa,
                    author: creator().public_key(),
                    clock: Some(OpClock {
                        counter: 101 + i as u64,
                        asserted_ms: 1_789_729_304_000,
                    }),
                    kind,
                }
                .sign(&creator()),
            );
        }
        ops
    }

    #[test]
    fn ops_of_other_kinds_do_not_change_current_metadata() {
        let current = resolved(
            &a_log_of(the_creators_other_ops(address_of(&agora()))),
            &agora(),
        );
        assert_eq!(current.title(), "Agora");
        assert!(current.is_genesis_fallback());
    }

    #[test]
    fn a_binding_rename_behind_higher_counter_ops_of_other_kinds_still_decides() {
        // The kind filter must run BEFORE the first entry is taken. The other
        // kinds all lead this rename in `cmp_ops` order, so a resolver that took
        // the leading entry and then checked its kind would fall back here.
        let stoa = address_of(&agora());
        let mut ops = the_creators_other_ops(stoa);
        ops.push(a_rename(
            stoa,
            &creator(),
            Some(1),
            "Renamed",
            "behind the rest",
        ));

        let current = resolved(&a_log_of(ops), &agora());
        assert_eq!(current.title(), "Renamed");
        assert!(!current.is_genesis_fallback());
    }

    // ─── Values are the op's, exactly ─────────────────────────────────────

    #[test]
    fn an_empty_title_in_a_binding_op_is_a_title_and_not_a_fallback() {
        let rename = a_rename(
            address_of(&agora()),
            &creator(),
            Some(1),
            "",
            "untitled now",
        );
        let current = resolved(&a_log_of([rename]), &agora());
        assert_eq!(current.title(), "");
        assert_eq!(current.description(), "untitled now");
        assert!(!current.is_genesis_fallback());
    }

    #[test]
    fn a_binding_op_carrying_the_founding_values_is_not_a_fallback() {
        // The case that makes the flag necessary: the op's values are exactly
        // what a fallback would report, and only the flag differs.
        let rename = a_rename(address_of(&agora()), &creator(), Some(1), "Agora", "");
        let current = resolved(&a_log_of([rename]), &agora());
        assert_eq!(current.title(), "Agora");
        assert_eq!(current.description(), "");
        assert!(!current.is_genesis_fallback());
    }

    // ─── Convergence ──────────────────────────────────────────────────────

    #[test]
    fn resolution_does_not_depend_on_the_sequence_ops_arrived_in() {
        let stoa = address_of(&agora());
        let ops = vec![
            a_rename(stoa, &creator(), Some(3), "Third", "c"),
            a_rename(stoa, &creator(), Some(1), "First", "a"),
            a_rename(stoa, &outsider(), Some(9), "Outsider", ""),
            a_forged_rename(
                stoa,
                &creator().public_key(),
                &outsider(),
                Some(8),
                "Forged",
            ),
            a_rename(stoa, &creator(), None, "Uncounted", ""),
            a_rename(stoa, &creator(), Some(2), "Second", "b"),
        ];
        let forwards = a_log_of(ops.clone());
        let backwards = a_log_of(ops.into_iter().rev());

        let a = resolved(&forwards, &agora());
        let b = resolved(&backwards, &agora());
        assert_eq!(a, b, "the decided op must be the same, not only the title");
        assert_eq!(a.title(), "Third");
    }

    // ─── Failure ──────────────────────────────────────────────────────────

    /// A log that cannot be read.
    struct UnreadableLog;

    impl OpLog for UnreadableLog {
        fn append(&mut self, _op: SignedOp, _arrival: Arrival) -> Result<Appended, OpLogError> {
            Err(OpLogError::Storage("disk on fire".into()))
        }
        fn get(&self, _id: &OpId) -> Result<Option<Entry>, OpLogError> {
            Err(OpLogError::Storage("disk on fire".into()))
        }
        fn iter(&self) -> Result<Vec<Entry>, OpLogError> {
            Err(OpLogError::Storage("disk on fire".into()))
        }
        fn iter_stoa(&self, _stoa: &Address) -> Result<Vec<Entry>, OpLogError> {
            Err(OpLogError::Storage("disk on fire".into()))
        }
        fn iter_target(&self, _target: &OpId) -> Result<Vec<Entry>, OpLogError> {
            Err(OpLogError::Storage("disk on fire".into()))
        }
        fn len(&self) -> Result<usize, OpLogError> {
            Err(OpLogError::Storage("disk on fire".into()))
        }
    }

    #[test]
    fn an_unreadable_store_is_an_error_not_a_fallback() {
        assert_eq!(
            resolve(&UnreadableLog, &founding_of(&agora())),
            Err(OpLogError::Storage("disk on fire".into()))
        );
    }

    #[test]
    fn a_founding_cannot_be_built_from_a_record_that_has_no_address() {
        // Reachable: `Genesis` is a plain struct, so an over-cap title need not
        // pass the decoder. A pair at the boundary, so a drifted cap is caught.
        // 1024 is `stoa.rs`'s `MAX_TITLE_BYTES`, pinned there by a hardcoded
        // assertion; it is private to that module.
        const GENESIS_TITLE_CAP: usize = 1024;
        assert!(Founding::of(&a_genesis(&creator(), &"x".repeat(GENESIS_TITLE_CAP))).is_ok());
        assert_eq!(
            Founding::of(&a_genesis(&creator(), &"x".repeat(GENESIS_TITLE_CAP + 1))),
            Err(GenesisError::TitleTooLong(GENESIS_TITLE_CAP + 1))
        );
    }

    // ─── Hostile input ────────────────────────────────────────────────────

    #[test]
    fn an_adversarial_log_resolves_without_panicking() {
        // Every op a peer holds arrived from a peer. A panic here aborts the
        // module process, so one hostile op would deny the receiving peer every
        // call. Mixed: forged, non-moderator, cross-Stoa, other kinds, the
        // maximum counter, no counter, and fields at the maximum length.
        let stoa = address_of(&agora());
        let longest = "\u{202e}".repeat(MAX_FIELD_LEN / "\u{202e}".len());
        let mut ops = vec![
            a_forged_rename(
                stoa,
                &creator().public_key(),
                &outsider(),
                Some(u64::MAX),
                "Forged",
            ),
            a_rename(stoa, &outsider(), Some(u64::MAX), &longest, &longest),
            a_rename(
                address_of(&a_genesis(&creator(), "Lyceum")),
                &creator(),
                Some(u64::MAX),
                "X",
                "",
            ),
            a_rename(stoa, &creator(), None, &longest, &longest),
            a_rename(stoa, &creator(), Some(0), "Zero", ""),
        ];
        ops.extend(the_creators_other_ops(stoa));
        let winner = a_rename(stoa, &creator(), Some(u64::MAX), &longest, &longest);
        ops.push(winner.clone());

        // Compared field by field rather than as one value: a failure printing
        // two 150 KiB strings of bidi overrides says nothing a reader can use.
        let current = resolved(&a_log_of(ops), &agora());
        assert!(
            matches!(&current, CurrentMetadata::Declared { decided_by, .. } if *decided_by == winner.op.id()),
            "the creator's op at the maximum counter must decide"
        );
        assert!(current.title() == longest, "the title must arrive whole");
        assert!(
            current.description() == longest,
            "the description must arrive whole"
        );
    }
}
