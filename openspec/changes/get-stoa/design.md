## Context

See `proposal.md` for why. The pieces this change joins already exist:

- `op.rs` defines `OpKind::StoaMetadata { title, description }`; every peer
  stores such ops, and before this change nothing read them.
- `moderation.rs` holds the only code that decides authority:
  `Moderators::of(&Genesis)` (the creator, as the sole moderator, plus the
  Stoa's address) and `Moderators::authorises`, which runs authenticity,
  authority and scope together.
- `OpLog::iter_stoa` returns one Stoa's ops in `cmp_ops` order — the order
  `op-ordering` defines and the one every resolver must take as given.
- `wire::genesis_for` decodes a request's `genesis` field and verifies it
  against `stoa`, returning the verified pair. `listThreads` and `readThread`
  already take their genesis record this way.

## Goals / Non-Goals

**Goals:**

- A resolver that is a pure function of the ops held and the genesis record,
  so two peers holding the same ops reach the same answer.
- A reply in which "fell back" and "resolved" cannot be confused, by the view
  or by the code producing it.
- `getStoa` reads nothing but the op log, so it answers for a Stoa the peer
  has not joined and has no route by which to record a membership.

**Non-Goals:**

- Publishing a metadata op (#125, milestone 0.0.2).
- Any view calling `getStoa`. Wiring the join preview is #143 (milestone
  0.0.1). No QML changes here.
- A mutable moderator set. The resolver inherits `Moderators`' set of one.

## Decisions

### 1. The resolver is the moderation resolver with a different subject, and reads by Stoa

The shape was fixed before this change, in `log/mod.rs`'s doc on
`iter_target` and in the retired plan (`docs/PLAN.md` §9.1's section 7, "What
the resolvers do not provide", as it stood at `d5ce992^`, lines 3247–3252):
*"it is 'the moderation resolver with a
different subject', reading by `Address` rather than by target, and reusing the
authority check unchanged. `iter_target` cannot serve it, because a metadata
op's `Entry::target()` is `None` by design."* Issue #98 restates it.

So `stoa_metadata::resolve` reads `OpLog::iter_stoa(stoa)`, keeps the entries
that are `StoaMetadata` **and** bind, and takes the first. A new module beside
`moderation.rs` rather than a function inside it: moderation decides what is
rendered, this decides what a Stoa is called, and a reader looking for either
should find it by name.

The kind is checked before authority. Both must pass, so the order changes no
answer; it is there because `authorises` verifies a signature, and in a Stoa
whose creator posts often, checking authority first would verify every one of
those posts on every resolution.

**Considered: widening `Entry::target` to return an address for a metadata
op.** Rejected for the reason `log/mod.rs` gives: the subject of `iter_target`
is an `OpId`, and `Entry::target`'s exhaustive match exists so that a kind that
should name a target cannot silently not. Making it return "a target or an
address" widens the store's API for one caller.

### 2. The authority check is `Moderators::authorises`, reused, not re-spelled

`authorises` was private to `moderation.rs`; it is now `pub(crate)`. That is
the whole of the change to that file. Its own doc anticipated this: *"The next
resolvers … call this rather than re-spelling the conjunction and dropping a
term."* The spec's three binding conditions are exactly its three checks, and
`stoa-metadata`'s requirement says the check is the one
`moderation-resolution` makes, not a new one.

**What breaks without each term**, measured by deleting it and running
`stoa_metadata`'s tests:

- the authority term (`contains(author)`):
  `a_metadata_op_by_anyone_but_the_creator_does_not_bind`,
  `an_unauthorised_higher_counter_op_does_not_displace_a_binding_one`,
  `a_stoa_whose_only_metadata_ops_fail_to_bind_resolves_exactly_as_one_with_none`
  and `resolution_does_not_depend_on_the_sequence_ops_arrived_in`;
- the authenticity term (`verify()`):
  `a_metadata_op_forging_the_creators_authorship_does_not_bind`,
  `a_forged_higher_counter_op_does_not_displace_a_binding_one`, and the same
  last two;
- the scope term (`entry.op.op.stoa == self.stoa`):
  `the_resolver_does_not_trust_the_read_to_have_scoped_the_ops`, and only that
  one — see decision 4.

And the filter-then-take shape itself: replacing `find_map` with "take the
first entry, then check it" turns red
`a_binding_rename_behind_higher_counter_ops_of_other_kinds_still_decides`, both
`…_does_not_displace_a_binding_one` tests, and
`resolution_does_not_depend_on_the_sequence_ops_arrived_in`.

### 3. The first binding entry decides, and there is no degraded-order preference

`iter_stoa` is already in `cmp_ops` order: higher counter first, equal counters
by ascending op id, counter-less ops after every op carrying one. The spec
forbids this capability an ordering of its own, so the resolver takes the first
binding entry and does not re-sort.

**Considered: copying `moderation::resolve`'s degraded-branch preference.**
That resolver prefers a `Hide` among counter-less candidates, because among
those the order is ascending op id and "lowest id wins forever" became a
pre-emptive veto an attacker could grind. It does not transfer, for two
reasons. The spec rules it out ("MUST NOT define an ordering or a tiebreak of
its own"). And the preference existed because one action was the fail-safe
direction; two titles have no safe direction to prefer. The veto it closed is
also not reachable here — only the creator's ops bind, so a creator grinding op
ids would be vetoing their own renames.

### 4. The scope check is made twice, once by the read and once by `authorises`

`iter_stoa` returns only ops naming the Stoa asked for, so for every real
`OpLog` the scope term in `authorises` refuses nothing the read did not already
exclude. It is kept, not skipped, because the spec requires the resolver itself
to decide binding "each time a Stoa's metadata is resolved", and a check
delegated to the storage layer's filter is a check that moves when the storage
layer does.

That makes the term invisible to every test that goes through a correct log, so
`the_resolver_does_not_trust_the_read_to_have_scoped_the_ops` uses a log whose
`iter_stoa` returns every op it holds. Deleting the scope term turns it red and
nothing else.

### 5. The resolution is an enum whose fallback case cannot carry op values

```rust
pub enum CurrentMetadata {
    Fallback { title: String },
    Declared { title: String, description: String, decided_by: OpId },
}
```

`is_genesis_fallback()` is `matches!(self, Fallback { .. })`; `description()` is
`""` for `Fallback`. So:

- **the flag cannot disagree with the values.** A struct of
  `{title, description, is_genesis_fallback: bool}` admits a fallback carrying
  an op's description, and a resolved value flagged as a fallback. Both are
  unrepresentable here.
- **an empty title is a title.** There is no `Option<String>` whose `None` a
  later edit could conflate with `Some("")`, which is the spec's "an empty title
  in a binding op … MUST NOT trigger the fallback".
- **no unreachable arm.** `Declared` holds the op's strings, extracted when the
  op is chosen, rather than the `Entry` as `moderation::Moderation` does. Holding
  the entry would force every reader to match on its kind again, with a `_` arm
  for kinds the filter already excluded — the exact shape `moderation::resolve`
  documents as how a new variant defaults silently.

`decided_by` is carried so a test can name which op decided, not because the
wire reports it — the spec does not ask for it, and adding it to the reply
would widen the contract.

**Considered: `isGenesisFallback` inferred at the wire by comparing the title
to the genesis title.** Rejected, and the spec forbids a caller needing to: a
binding op may carry the founding title.

### 6. The resolver takes a `Founding`, so the moderator set and the fallback title come from one record

`resolve` needs two things from the genesis record: the moderator set, and the
founding title to fall back to. Passed separately, a caller could pair one
Stoa's moderators with another Stoa's title and nothing would notice — the
fallback would simply be wrong. `Founding::of(&Genesis)` builds both from one
record and is the only constructor, following `Moderators`, which holds its
Stoa address for the same reason ("a caller passing it separately is the
fourth-slightly-different-guard shape").

It is fallible for the one reason `Moderators::of` is: a record over the
genesis title cap has no encoding and so names no Stoa. At the wire this is
unreachable — `genesis_for` has already decoded the record, and the decoder
enforces the same cap — and it is reported as the error shape rather than
unwrapped.

### 7. A store that cannot be consulted is `Err`, never a fallback

`resolve` returns `Result<CurrentMetadata, OpLogError>`. The spec's reason is
the whole argument: a fallback states the peer holds no binding op, which an
unreadable store cannot know. It is a `Result` rather than a third variant for
the reason `moderation::resolve` gives — a variant is something an existing
`match` can absorb with a default, a `Result` is something a caller cannot
ignore without a compile error.

**What breaks without it:** in the handler, answering a resolver `Err` with a
fallback reply turns `an_unreadable_store_is_the_error_shape_and_never_a_fallback`
red (measured). In the resolver, the same softening fails
`an_unreadable_store_is_an_error_not_a_fallback`, which asserts the `Err` itself.

### 8. The request carries the genesis record: `{stoa, genesis}`, not the issue's `{stoa}`

Issue #98 sketched `getStoa({stoa})`. The spec-writer proposed `{stoa, genesis}`
and the runner kept it; the owner has not objected, and has not explicitly
approved it either. The reasoning, which is the spec's:

- **An address cannot be turned back into a record.** It is a one-way hash of
  the record, and the moderator set — and so which metadata ops bind — comes
  from the record. With the address alone the resolver has no basis for an
  authority check.
- **It is the pattern the other Stoa reads use.** `listThreads` and
  `readThread` take `{stoa, genesis}` and verify one against the other through
  `genesis_for`; `getStoa` calls the same function, so a record the membership
  calls report is accepted here as it is.

**Considered: taking `{stoa}` and reading the record from the membership
store.** Rejected: it answers only for a Stoa the peer is in, and the preview
before a join — the consumer this call exists for (#143) — is by definition for
a Stoa the peer is not in. It would also put a membership store within reach of
a call the spec forbids to record membership (decision 9).

### 9. `getStoa` cannot change state because it holds nothing it could change it with

The handler is `get_stoa(request, store)`, where `store` opens an op log and
nothing else. The resolver takes `&L`, and `OpLog::append` needs `&mut self`, so
the resolver cannot append. There is no membership-store parameter, so no join
can be recorded. The spec's "MUST NOT record membership, append or publish"
holds because the capability to do any of it is absent, not because a branch
declines to — which is also why the task list labels those scenarios
satisfied-by-construction rather than pointing at a test: a test asserting the
listing is unchanged after a call passes for the null implementation too.

### 10. The record is verified before the store is opened

The handler parses, reads `stoa`, decodes and verifies `genesis`, and only then
calls the store opener. A malformed request never touches the disk, and a
record that fails to match its address is refused with the same message
`listThreads` and `joinStoa` give — it is the same function.

**What breaks without it:** opening the store ahead of `genesis_for` turns
`a_request_that_fails_verification_never_opens_the_store` red (measured), and
nothing else — every refusal is the same message in either order, so only an
opener that records whether it ran can see the difference.

### 11. The reply carries no founding title of its own

The owner removed `foundingTitle` from the reply: *"I don't understand why we
have a founding title, I would remove that."* The proposal carries the
reasoning: the caller already holds the founding title, in the genesis record it
supplied, and the membership calls report it under a name of its own. Where the
resolution falls back, `title` is the founding title and `isGenesisFallback`
says so.

That also retires a claim in `wire.rs`: `FOUNDING_TITLE`'s doc said that when
resolution landed it would add `title` and `isGenesisFallback` *beside*
`foundingTitle`. It does not — it is a separate call with a separate shape, and
the membership replies are unchanged.

### 12. `isGenesisFallback` is the field worth defending

From the retired plan (`docs/PLAN.md` §9.1's section 6, "The core API this
requires", at `d5ce992^`, line 3162), the paragraph under its `getStoa` sketch:
*"a reader
prefers the latest valid metadata op and falls back to the genesis values, and
those are different epistemic states. A peer that has not yet received a Stoa's
metadata op is showing a founding title that may be years stale, and it should
be able to say so rather than presenting it as current."* Issue #98: *"it
distinguishes 'this is the Stoa's current name' from 'nobody has published a
metadata op yet and this could be years stale.'"*

That is why it is carried on every successful reply, why it is derived from the
enum in decision 5 rather than computed at the wire, and why a store failure
must not produce it (decision 7).

### 13. `policy` comes from the genesis record through the name `stoa-membership` already uses

`wire::policy_name`, shared with `createStoa` and `joinStoa`, so the two calls
cannot spell one policy differently. A metadata op has no policy field, so the
resolution never touches it.

### 14. The method joins the envelope sweep, and the trait is what makes that compulsory

Declaring `get_stoa` on `DialecticaModule` is enough to turn
`the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares` red
until it is added to `every_request_taking_method` with a served fixture
(measured). The sweep's wrapper serves from a log holding a rename, so the
envelope tests exercise the resolver's `Declared` path rather than only the
fallback, and `isGenesisFallback` is added to the result fields
`a_non_object_refusal_carries_no_result_field` checks.

The adapter's own line is compiled only by `nix build .#lgx` (the `cfg(logos_scaffold)`
gate). It is a `storage_dir()` plus one `core::get_stoa` call with an op-log
opener, the shape `list_threads` has beside it, and it opens no membership
store.

### 15. #98 and #125 land separately

Issue #125's comment: *"Whoever picks either up should check whether they're
better landed as one piece."* Checked, and they are not. The read side is
complete and testable on its own: the resolver reads what the log holds, not who
put it there, and its tests stage metadata ops directly. The publish side needs
a spec delta of its own, a moderator-only authoring path and a UI affordance,
and is milestone 0.0.2; bundling it would pull 0.0.2 work into 0.0.1. Until it
lands, `getStoa` on this build falls back unless a metadata op arrives from
elsewhere.

### 16. The reply reports the parsed `Address`, so the module sets the letter case, not the request

`identity`'s display-form requirement now states the rule. Parsing accepts hex
letters in either case, and every address the module reports is the lowercase
display form. That rule describes what every Stoa-taking call already did.
This change adds no code for it. `getStoa` inherits the rule, because its reply
names the address that was asked for.

**How the code holds it:**

- `Address` is `[u8; 32]`. `Address::from_hex` decodes through `hex::decode`,
  which reads `a`–`f` and `A`–`F` alike, into those bytes. `to_hex` is
  `hex::encode`, which writes lowercase. The case a request used is never
  stored, so no reply can report it back.
- `stoa_metadata_json` renders `stoa` as `stoa.to_hex()` from the parsed
  `Address`, never from the request's string. `parse_stoa` is the only
  production code that reads a request's `stoa` field.

**Why accepting both cases is safe:** `from_hex` is strict so that two different
Stoas cannot collide in what a reader sees (its own doc says so). Letter case
has no bearing on that. An uppercase hex digit names the same four bits as its
lowercase form, so two spellings that differ only in case decode to the same
bytes and name the same Stoa, and no spelling names two different Stoas. The
length and alphabet checks, which are what close off a collision, are
unchanged. Accepting both cases also keeps an address working after something
between copy and paste changed its case.

**Considered: refusing uppercase (a parser that accepts only lowercase).**
Rejected. It adds no collision resistance, for the reason above. It would also
refuse an address that names a real Stoa, and it would change the behaviour of
every Stoa-taking call, not only this one, which is too wide a change for a read
call.

**Considered: echoing the request's string in the reply.** Rejected. The reply
would then carry a spelling the module reports nowhere else. A caller comparing
reported addresses as strings, such as `getStoa`'s `stoa` against a
`listStoas` row, would see one Stoa under two names.

**What breaks without it:** `an_address_asked_for_in_uppercase_is_answered_in_lowercase`
sends an uppercase address and checks both halves at the wire: the call
succeeds, and the reply's `stoa` is lowercase. A lowercase-only `from_hex`
would fail its first check, and a reply built from the request's string would
fail its second. This comes from reading the code, not from running the
mutation.

## Risks / Trade-offs

- **The join preview is out of step with its spec until #143 lands** →
  `proposal.md`'s Impact section states which case obliges what; #143 is filed
  in the same milestone.
- **The adapter opens the op log per call, which creates an empty store file on
  a profile that has none** → `listThreads` and `readThread` already behave the
  same way (`SqliteOpLog::open` on `ops.sqlite`). `stoa-metadata`'s no-state
  requirement settles whether this counts: initialising an empty op store is
  not a change of state, because it holds no ops and no call answers
  differently for it than for no store. A peer with no store yet is answered
  with a fallback, not refused. The trade-off is a disk write on a read call,
  accepted so `getStoa` matches the other reads. It becomes a real change of
  state only if an empty store ever answers differently from no store, and that
  is when to revisit it.
- **A metadata op carrying counter `u64::MAX` can never be outranked by a
  higher counter** → only the creator's key binds, so this is a creator (or
  whoever holds that key) freezing their own Stoa's title; `op-ordering`'s
  bound on how far a received counter advances a peer's own clock is what keeps
  it from costing any other Stoa anything. The resolver does not second-guess a
  counter, because the spec forbids it an ordering of its own. A mutable
  moderator set would make this reachable by a second key and is when it needs
  revisiting.
- **`iter_stoa` reads every op of the Stoa to find one metadata op** → the same
  cost `OpLog::clock` already pays per publish. An index by kind is a storage
  change for when a Stoa's op count makes it matter.
- **The adapter's dispatch line is compiled only by `nix build .#lgx`** → it is
  one `core::get_stoa` call with a store opener, the shape of `list_threads`
  beside it; the handler's behaviour is tested in `dialectica-core`.
