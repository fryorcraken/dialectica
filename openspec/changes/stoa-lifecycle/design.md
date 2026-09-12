## Context

See `proposal.md` — Why. The constraints this design has to fit between are all
already in the tree, and three of them decide most of it:

- **`SqliteOpLog` refuses any store whose `PRAGMA user_version` is not exactly
  `LAYOUT_VERSION`**, in both directions, and `check_layout` additionally names
  every column of `ops` that a read touches. That refusal is deliberate and
  documented at length; `sqlite.rs`'s own comments state there is no migration
  path by design.
- **`create_schema` runs only when `user_version` reads `0`** — the never-stamped
  value. There is no `CREATE TABLE IF NOT EXISTS` anywhere in the crate, so a
  table added to that batch reaches fresh stores only and never an existing one.
- **`Moderators::of` takes a `&Genesis`, and it is the only constructor.** The
  `moderation-resolution` capability's "A Stoa's moderator set is derived from
  its genesis record" is what makes that the fail-closed shape. It is also why
  membership must retain the record: an address is a one-way hash, so a peer
  holding only addresses holds a list of Stoas it can store content for and
  cannot judge.

The core crate holds no opinion about where storage lives. `SqliteOpLog::open`
takes a path, `keystore::default_path_in` takes a directory, and the adapter in
`dialectica/rust-lib/src/lib.rs` is the one thing holding the host's
`instance_persistence_path`. This design follows that convention rather than
introducing a second one.

## Goals / Non-Goals

**Goals:**

- Membership as state beside the op log, readable and writable without the op
  log being consulted at all.
- Retain the complete genesis record per Stoa, in a form that still verifies
  against the address it is filed under after a restart.
- Three wire handlers — create, join, list — in `wire.rs`, each inside the panic
  guard and each answering the one failure shape.
- A signing key taken as an input, never minted.

**Non-Goals:**

- **No metadata resolution.** `stoa-metadata` owns current-versus-founding, and
  nothing resolves those ops. Every title this change reports is a founding
  title and says so on the wire.
- **No moderation call, no feed change, no compose.** Creation leaves the peer
  holding what a moderator set would later be derived from; it does not derive
  one.
- **No change to the op log**: not its schema, not its version, not its refusals.
- **No `getStoa`.** PLAN.md §9.1 names it, and it is the metadata-resolution
  call. Out of scope with metadata.

## Decisions

### Membership lives in its own SQLite file, not in a second table beside `ops`

**Chosen:** a separate store, `stoas.sqlite`, with its own
`PRAGMA user_version` and its own open path. `MembershipStore::open` never
touches the op log's file.

The spec requires that a store already holding ops stay readable, that opening
it not be refused for predating membership, and that a membership be
**recordable into such a store**. Put the table beside `ops` and each of the
three ways to get there is worse:

| Alternative | Why it loses |
|---|---|
| Add `memberships` to `create_schema` | That batch runs only at `user_version == 0`. An existing v1 store never gets the table; `check_layout` passes it (it names only `ops` columns), and the first membership read fails as `Storage("no such table: memberships")`. "Opening succeeds and ops are readable" holds; "a membership is recordable" does not. |
| Bump `LAYOUT_VERSION` to 2 | The spec forbids it in as many words, and `sqlite.rs` refuses an older layout on purpose — a v1 store would become permanently unopenable, discarding the user's ops to avoid a migration that is not needed. |
| `CREATE TABLE IF NOT EXISTS` on every open | Silently repairs a file that `check_layout` exists to refuse, and makes "what layout does version 1 mean" have two answers depending on when the file was opened. `sqlite.rs`'s documentation names a later "add a migration" refactor as the change its warning is addressed to; this would be that change, done by accident. |

The separate file makes the requirement hold **by construction rather than by a
check**, which is CLAUDE.md's standing instruction applied to a file boundary: a
store predating membership has no membership file, `MembershipStore::open`
creates one, and there is no version of the op log's layout that has to mean two
things. Nothing in `SqliteOpLog` is edited by this change — which is also what
makes the spec's requirement testable as written, by opening an op store that
this change cannot have touched and reading its ops back afterwards.

**What actually holds the boundary between the two stores is `check_layout`
naming columns, not the two version numbers.** This was originally written the
other way round in three places and was wrong: `MEMBERSHIP_LAYOUT_VERSION` and
`log::sqlite::LAYOUT_VERSION` are **both `1`**, so handing
`MembershipStore::open` an op-log file gives `found == 1 == expected` and it
takes the already-stamped branch. The refusal comes entirely from `check_layout`
failing to prepare `SELECT stoa, genesis_bytes FROM stoas` against an `ops`
table. The two versions are independent in the sense that neither is read from
the other — a property the independence test asserts, since comparing two
constants that are both `1` would pass an implementation that read one from the
other — but independence is not distinguishing, and the version check separates
nothing today.

The cost is two files where one would do, and one more `user_version` to keep
straight. Both are cheap; the op log's own comments make the case that a version
number that has to mean two layouts is not.

**What two files foreclose, which the cost above does not state:** no
transaction can span the membership store and the op log, because they are two
SQLite connections. So a future "join and backfill" — record the membership and
import the ops a peer already holds for that Stoa, atomically — is not available
as one transaction. The recovery asymmetry is worth more than that atomicity
today (a half-done backfill loses ops the log will receive again; a half-done
join loses a Stoa the user must re-paste), but the trade is a trade and whoever
wants that backfill will find it foreclosed here rather than in their own change.

### The record is stored as its canonical bytes, and the address is derived on read

**Chosen:** one row per Stoa: `stoa` (the 32-byte address) as `PRIMARY KEY`, and
`genesis_bytes` — `Genesis::canonical_bytes()` verbatim.

Storing the record as decomposed columns (creator, policy, title) and
re-encoding on read would make the address depend on this module's re-encoding
agreeing with `stoa.rs`'s forever. The address **is** the hash of the canonical
encoding, so re-encoding is the one operation that must not be duplicated. Same
argument `sqlite.rs` makes for storing `op_bytes` verbatim, and the same test
shape holds it: the retained record is read back, its address recomputed, and
compared against the key it was filed under.

`stoa` is the primary key rather than a `UNIQUE` index over the bytes, because
the address is the identity. Repeated-join idempotence is then **structural**,
and the load-bearing half of that is the **primary key**: a second join of the
same Stoa cannot produce a second row, whatever the statement verb. So "MUST NOT
disturb what was already retained" is a property of the shape rather than a
branch that must be right at each call site. `INSERT OR IGNORE` is the verb that
says what is meant — a join is not a way to overwrite what a peer already holds.

**What the suite can and cannot see about the verb, measured, because
"structural" is doing real work for the shape and none for the verb.** Change
the statement to `INSERT OR REPLACE` and delete the single
`assert_eq!(…, Joined::AlreadyIn)` in
`a_repeated_join_is_idempotent_and_leaves_the_record_untouched`, and **544 tests
pass with zero failures** — the same as the baseline. That is not a gap in the
test: `join` verifies the record against the address *before* it inserts, so a
mismatched pair is unreachable and the only record REPLACE could ever write over
a row is the byte-identical one. The `Joined` value is the sole witness, the test
says so in its own comment, and nothing else can be.

**And the wire deliberately hides the distinction.** `create_stoa` and
`join_stoa` both discard `Joined` with `Ok(_)` (`wire.rs`), argued there: the
spec asks that a second join succeed and change nothing, and a reply rendering
"already joined" differently would render a distinction the user did not make.
The consequence, recorded because it is a cost nobody had written down: **nothing
observable at the module surface tells `OR IGNORE` from `OR REPLACE`**, so the
spec's non-destructiveness requirement is unfalsifiable from outside core and can
only be checked against `MembershipStore` directly. That is the right trade for
the reply shape; it means the store's own test is the only thing holding the
verb.

**The write verifies before it inserts, and `MembershipStore::join` takes the
address and the record separately** so that the verification is inside the store
rather than in a caller that could forget it. A caller holding a `Genesis` can
always derive its address, so a store taking only the record could not express a
mismatch at all — and the whole point of the join shape is that a mismatch is
refusable.

### `joinStoa` takes `{stoa, genesis}`, departing from PLAN §9.1's `{address}`

**This is a deliberate departure from a PLAN signature, argued rather than made
in passing.** PLAN §9.1 specifies `joinStoa({address})` twice — in its item 5
("take an address, verify the genesis record hashes to it") and in its Stage D
API block. The implementation takes both the address and the record.

PLAN is wrong here, and its own sentence shows it: *"take an address, verify the
genesis record hashes to it"* does not say **which** record, because there is
none. A Stoa address is a one-way hash of its genesis record — sufficient to
**verify** a record somebody hands over, insufficient to **reconstruct** one. A
call given only an address has nothing to verify. It could only join a bare
address, and then the peer would hold a Stoa whose record it does not have, which
`moderation-resolution` requires before a reader may decide whether any
moderation of that Stoa's content binds. So the record has to arrive with the
address; there is nowhere else for it to come from.

Nothing is lost against what §9.1 wanted the call for. Its argument for the
signature was that a join reply must show what is being joined before it is
joined (§4.8), and the reply still carries the founding title for exactly that.
What §9.1's shape got wrong is the *input*, not the output.

**Why the address is still a parameter, given the record determines it.** This is
the half that reads as redundant and is not. The spec requires that a record
differing from the one the address names in **any** field is refused, *and that
the peer is not in the Stoa the supplied record would name either*. A `join`
taking only a record could not fail — it would join whatever it was handed, and a
substituted record names a creator of the attacker's choosing. The address is the
caller's claim about what it thinks it is joining, the record is the material, and
the refusal is the two disagreeing. That is `Genesis::matches`, which
`wire.rs::genesis_for` already uses for the feed path.

`docs/PLAN.md` §9.1 is corrected in this change rather than left to contradict
the code: its stale signature and its copy of the verification reasoning are
struck down to a line saying create, join and list exist, because the reasoning
lives here and two copies drift with no way for a reader to tell which is stale.
PLAN's copy was the stale one.

### Creation reuses `join`'s write path rather than having one of its own

`create_stoa` builds the record, computes the address, and records the
membership through the same `join` call a paste goes through. Two consequences
the spec asks for fall out:

- "Creating the same title twice yields one Stoa" is then the same
  `INSERT OR IGNORE` idempotence as a repeated join, not a second mechanism that
  has to agree with it.
- "Creating and then joining the same Stoa is one membership" is true because
  there is one write path, not because two were checked against each other.

The title bound is refused **before** the write, and that ordering is not
incidental: `Genesis::canonical_bytes()` is fallible for an over-cap title, and
building the record's bytes is what the store needs, so a failed encode returns
before any statement runs. The spec's "the refusal happens first, so a failed
creation leaves nothing behind" is therefore structural.

### The creator key is taken as a `PublicKey`, and the handler takes a closure

`create_stoa_from_request` takes `impl FnOnce() -> Result<PublicKey, KeystoreError>`
— the same shape `get_capabilities` takes for its identity lookup, and for the
same reason: this crate cannot read the environment or know the host's layout.
The adapter has the keystore path and supplies the closure.

**A `PublicKey`, not a `SecretKey` and not a `Keystore`.** A genesis record needs
the creator's public key and nothing else — no signature is made, because a
genesis record is not an op and carries none. Taking a secret would be taking
authority the operation does not use.

**The failure mode is a `KeystoreError`, not a bool.** "Creation MUST fail when
the peer has no usable signing key, and MUST NOT proceed by generating a key" is
satisfied by there being no path from this handler to `Keystore::generate()`.
Whether a key is usable and why not is `posting-capability`'s probe: this
handler surfaces the error's own message and adds no reason vocabulary of its
own, exactly as `capability_for` does.

### Which key the creator is: the root identity, used directly

**Chosen:** `Keystore::identity_key` — the root secret used as an Ed25519 seed,
with no derivation step. The genesis record's `creator` is its public half, and
the capability probe reports that same key's address. **One derivation position,
one key.**

**The bug this replaced, because it is the reason this entry exists.** The first
implementation used `creator_public_key()` = `derive_stoa_key(root,
CREATOR_KEY_DOMAIN)` where `CREATOR_KEY_DOMAIN` was `[0u8; 32]`, while the probe
reported `derive_stoa_key(root, stoa_address)`. Those are different keys.
`Moderators::of(genesis)` names `genesis.creator` as the sole moderator, so
`contains(posting_key)` was **false for a Stoa's own creator** — every Stoa a
peer created was a Stoa nobody could moderate, under a key that peer would never
sign an op with.

That could not be deferred to when moderation lands. The genesis record is
immutable once published and the address is its hash, so the wrong creator is
fixed inside the address forever. Deferring meant shipping records that are wrong
permanently.

**The circularity that produced the synthetic domain is real**, and any fix has
to respect it: the address is `SHA-256(canonical_bytes)` over a record whose
`creator` field is the key in question, so there is no per-Stoa key derivable
before the record exists. A synthetic derivation context was a reasonable answer
to that. It was not compatible with one-identity-per-user, and it was a fourth
derivation position no document argued for.

**How this reconciles with PLAN §5.2 — it does not depart from it; the old code
did.** §5.2's MVP subsection is an owner decision, recorded not argued: *for the
MVP a user has one identity across every Stoa*, and — quoting it, because this is
the sentence the fix implements — *"one identity per user means **not calling**
[`derive_stoa_key`] and signing with the root key directly."* §9.2 lists per-Stoa
identity (§5.2) as **out of the MVP** and says `derive_stoa_key` "is built and
simply is not called". The root key is knowable before any record exists, so
under that rule the circularity **dissolves rather than needing to be worked
around**. There was never a case for a third derivation position; the code had
one because it was written against §5.2's destination and not its MVP.

`derive_stoa_key` and `Keystore::stoa_key` stay built and tested. The live
`identity` spec requires the primitive and its properties (same root and Stoa
yield the same key; two Stoas yield different keys; no public derivation), and
those are properties of the function, which the tests exercise directly. Nothing
in `identity` or `posting-capability` requires a *handler* to call it —
`posting-capability` asks only that the reported identity be "the one an op
published now would be attributed to, derived from the key that would actually
sign it", which the one key satisfies exactly.

**What this costs, and what it does not.** Cross-Stoa unlinkability is
**suspended, not withdrawn** — §5.2's own words. One key signs in every Stoa, so
anyone observing two Stoas can link the same participant across them: the public
key is the join. That cost is §9.2's, accepted knowingly there, and it is
strictly *smaller* than the cost of the code this replaces, which paid the same
linkability for creator keys **and** got an unmoderatable Stoa for it. Restoring
per-Stoa identity is switching the call back on plus the create-or-select flows
§5.2 says are the deferred part.

**Alternatives, each with what ruled it out.** The first three were recorded only
in a code comment on the now-deleted `creator_public_key`, which is why they are
here: the archive is where someone greps.

| Alternative | Ruled out by |
|---|---|
| **A synthetic derivation domain** (`derive_stoa_key(root, [0u8; 32])`) — what shipped first | Produces a creator key the peer never signs with, so the creator cannot moderate. Also a fourth derivation position, and **reachable by a caller**: see the security note below. |
| **Derive from the title** | Two Stoas with the same title share a creator key, and the address becomes a function of the title alone — so a third party could compute a peer's address for any title. Worse than linkability. |
| **A fresh random key per creation** | The root secret is the only thing backed up, so a key minted outside it is a moderator key that vanishes with the device. A Stoa whose sole moderator's key is unrecoverable can never be moderated again, and the address cannot be un-minted. |
| **Take the key from the caller** | The spec forbids it in as many words, and for the right reason: a call accepting a creator is a call that can be asked to create a Stoa moderated by somebody else. |
| **Per-Stoa, resolving the circularity with a two-pass derivation** (derive from a provisional address, re-derive, re-hash) | Does not converge — each re-derivation changes the record and therefore the address. And it would be building §5.2's destination inside the change that §9.2 scopes it out of. |

**The security note, which is the other half of why the synthetic domain is
deleted rather than retargeted.** Its stated justification was that no genesis
record hashes to all-zero, so no real Stoa address could collide with the domain
— *"finding one would be a preimage attack on the hash"*. That is true about
records and **irrelevant about arguments**. `getCapabilities` takes a
caller-supplied hex string, decodes it, and hands it to the derivation with no
plausibility check, so `{"stoa":"00…00"}` returned **exactly** the creator's
address — measured, not reasoned. Harmless while the probe is a local read-only
lookup that tells a caller about its own peer, and a live hazard the moment any
signing path takes a caller-supplied address: a caller passing 64 zeros would
sign under the moderator key.

The fix closes this **by construction**: `identity_key` takes no address, so
there is no argument left to choose.
`no_stoa_address_a_caller_can_name_reaches_the_identity_key` pins the
consequence. Any future synthetic domain must be
unreachable through a caller-named address, and must be tested for it — the
lesson is that "no record hashes here" is the wrong invariant when nothing
requires the value to have come from a record.

**`CREATOR_KEY_DOMAIN` and its preimage argument are therefore deleted, not
moved.** Recorded here so the deletion is not re-proposed: the constant was
`[0u8; 32]`, it was address-determining (changing it silently re-mints the
creator identity of every Stoa a user has made, because each peer stays
internally consistent and nothing errors), and it was pinned by a hardcoded
assertion for exactly that reason. `identity_key` inherits the
address-determining property and the pinning obligation —
`the_identity_key_is_pinned_to_a_known_answer` freezes the public key of the
`[7; 32]` root — but not the preimage argument, which has no purpose once there
is no synthetic context.

### Where that decision lives: `core::keystore::creator_and_poster_in`, because two agreeing call sites are not one derivation

Found by two reviewers independently — `findings/security.md` entry 3 and
`findings/architecture.md` entry 1 — and it is about the *shape* of the fix above
rather than about which key it chose.

**The problem, measured rather than argued.** The fix named
`identity_public_key()` at `dialectica/rust-lib/src/lib.rs:385` and
`identity_address()` at `:346`. Those are two independent call sites that have to
agree, and they sit in the one file no gate reads:

- `cargo test` does not compile it. `build.rs` sets `logos_scaffold` only when
  `generated/provider_gen.rs` exists, and `git ls-files dialectica/rust-lib/generated`
  is empty — the builder writes that file and committing a copy would recreate the
  contract drift `codegen.rust.trait` exists to prevent.
- `cargo mutants` cannot see the accessors either: run over `keystore.rs` filtered
  to the six identity/stoa accessors, **all 6 mutants came back unviable** (no
  `Default` for the key types).
- The Rust job's clippy and fmt never reach the file, and no Lint-job check read it.
- **Build LGX** does compile the adapter, so a *type* error would be caught — but
  `stoa_address` and `identity_address` return the same type, so the wrong *method*
  compiles green.

So changing `:346` back to `ks.stoa_address(_stoa)` restored the bug this
Decision exists to prevent, **with every gate green**. A correctness property no
gate can see is one revert away from being wrong again.

**Chosen:** move the pairing into `core` as a single derivation —
`keystore::creator_and_poster_in(dir) -> (PublicKey, Address)`, with
`creator_key_in` and `poster_address_in` as the two thin wrappers the adapter
calls. Both halves now come out of one expression over one `identity_key()` root,
so there is no argument to pass differently and no second accessor to reach for:
the invariant holds **by construction** rather than by two comments agreeing.

**Why it could move at all, which is the load-bearing observation.** Both closure
bodies contained *no host type*. Each was `core::keystore::default_path_in(&dir)`
then `core::keystore::open_from_env(&path)` then one accessor — three `core`
functions over a `&Path`. The stated reason for the closure ("`core` cannot read
the environment or know the host's layout") applies to **which directory**, not to
**which accessor**, and the directory is already an ordinary argument everywhere
else. The closure parameter stays; only the body moved.

**What now pins it.** `wire.rs`'s
`the_creator_a_creation_names_is_the_identity_the_probe_reports` drives *both wire
handlers* through the two `core` functions, against a real on-disk keystore, and
asserts the probe's reported identity is the address of the key the creation
recorded as creator. Pointing the derivation at `stoa_address` fails that test and
**only** that test — verified by running the mutation: 550 passed, 1 failed.

That is strictly stronger than what stood before. `keystore.rs`'s
`the_creator_of_a_stoa_this_keystore_made_can_moderate_it` asserts
`identity_address() == identity_public_key().address()`, a property of two
`Keystore` methods that is true whatever the module wires up; its comment claimed
to be "the pair the adapter wires up, checked here because `cfg(logos_scaffold)`
is not built by tests", which it structurally could not be. The comment is
corrected rather than the test deleted — the test is fine, its description was not.

**And a CI grep, for the half no test can reach.** The test proves the derivation
is *correct*; it cannot prove the adapter still *calls* it. A new Lint step
(`the adapter derives the creator and the poster in one place`) asserts both
wrapper names appear in the adapter and that it names no `Keystore` accessor
itself. Both halves were verified to fire: removing `poster_address_in` trips the
first, and calling `ks.stoa_address(...)` while leaving the name in place trips the
second. Stated as what it cannot see: it reads text, so it says nothing about
correctness — that half is the test's.

**Alternative considered: leave it, and rely on review.** Ruled out by the
evidence in the finding — this exact divergence already shipped once and was
caught by a human reading code, not by a gate. `.claude/agents/README.md`'s rule
applies: *"say what a gate cannot see rather than reporting it as passed."* The
answer here is that the gap was closable, so it was closed.

### The reply names its title as founding

`{"stoa":"<hex>","foundingTitle":"…","policy":"open"}` for create and join;
`{"items":[{"stoa":…,"foundingTitle":…}],"page":N,"hasMore":bool}` for list.

The field is **named** `foundingTitle` rather than carried as `title` beside a
boolean flag. The spec requires the reply make it distinguishable from a
resolved current title; a `{"title":…,"isFounding":true}` shape puts a view one
forgotten branch away from rendering a founding title as current, and `title`
would then have to mean two things depending on a sibling field. A name that
cannot be misread costs nothing. When metadata resolution lands it adds `title`
and `isGenesisFallback` beside this field rather than redefining it — which is
PLAN.md §9.1's own shape for `getStoa`, and this change does not pre-empt it.

`policy` is included on the create and join replies because the spec requires
the posting policy be **answerable** from what was retained, and the join reply
is where a view shows what is being joined. It is deliberately **not** on the
list items: the spec fixes list items as carrying the address and the founding
title, and widening the paginated envelope's item shape is a decision for
whoever needs it.

**The policy's wire NAME is a choice the spec does not make**, and it is a
lasting one: `policy_name` returns the lowercase literal `"open"`, which a view
branches on. The spec requires the policy be answerable and says nothing about
its spelling, so `"open"` versus `"Open"` versus a numeric discriminant was
this change's to pick — and once a view compares against the string, changing it
is a breaking change to the module surface. Recorded as unspecified behaviour
below. The function is deliberately **exhaustive with no wildcard arm**, which is
a separate decision and not a NO SPEC: a new `Policy` variant must force a
decision here rather than defaulting to a name describing a different policy.
`Policy::from_byte` refuses an unknown discriminant rather than treating it as
`Open`, and a wildcard here would undo that one layer up — telling a view a
token-gated Stoa is world-postable.

### Listing pages over an ordered read, and the order is the address

`ORDER BY stoa ASC` — the primary key, so it is an index walk. The spec requires
every Stoa be reachable by paging and none appear twice, which is a property of
the order being **total and stable**; the address is 32 bytes of hash and
unique by primary key, so it is both. Insertion order was the alternative and is
worse for the reason `log/mod.rs` gives about its own reads: it is per-peer, and
a reader that could reach it would be one refactor away from presenting it as
meaningful. Nothing in the spec asks for a join order, and a `joined_at`
timestamp would be a local wall-clock reading — the thing `arrival.rs` refuses.

`page`/`perPage` reuse `wire.rs::parse_index` and `feed::clamp_per_page`
unchanged, so the envelope's arguments behave identically to the feed's rather
than acquiring a second interpretation of a negative page.

**`MembershipStore::list` is correct for arguments the wire cannot produce, and
that is a deliberate cost rather than defensiveness.** `clamp_per_page` caps
`per_page` at 100 and turns `0` into the default, so the two boundary cases below
are unreachable through the module surface. They are reachable through the crate,
which is the deliverable: `list` is `pub` on a `pub` module, and the second caller
is the one that gets bitten. Both were live bugs when review measured them, and
both are invisible to `cargo mutants`, which generates no mutant for
`saturating_add`, `saturating_mul` or `try_from`.

- **An over-large `per_page` saturates; only `page` refuses.** SQLite parameters
  are `i64`, so both cross that boundary, and the right answer differs: "more rows
  than exist" is answerable, so the limit saturates at `i64::MAX` and serves
  everything. An unreachable **page** is not answerable — a `usize` offset past
  `i64::MAX` cast rather than converted becomes negative, and SQLite treats a
  negative `OFFSET` as none at all, serving the FIRST page to a caller who asked
  for an impossible one. Empty is the honest reply there. The bug was converting
  `per_page + 1` and giving up on failure: the guard tested the *incremented*
  limit, so `list(0, i64::MAX as usize)` on a store holding three Stoas answered
  "you are in no Stoa" while `list(0, (i64::MAX as usize) - 1)` answered all three
  — indistinguishable from an empty store, and returned `Ok`.

- **A page of zero rows is the last page.** `has_more` means "paging further
  reaches a Stoa this page did not show"; at `per_page == 0` every later page is
  also empty, so it is `false` however many Stoas the store holds. This is a guard
  and an early return rather than arithmetic, because arithmetic that also handles
  zero is arithmetic whose zero case nobody can read — the zero page is a
  different question from where a page boundary falls. Previously `LIMIT 0+1`
  fetched a row, `has_more` was `1 > 0`, and the truncate then emptied the page, so
  every page was both empty and not-the-last and paging to exhaustion never
  terminated.

- **The remaining boundary is one operation, not two facts.** `split_off` cuts the
  look-ahead read once: what is kept is the page, what comes off is the evidence of
  a further one. Computing `has_more` from what was fetched and the page from a
  separate `truncate` is how the two came to disagree, and CLAUDE.md's rule applies
  — prefer a shape that cannot express the mistake over a third guard that checks
  for it. `feed.rs` is structurally immune to the same bug for a related reason (it
  slices with `min(rows.len())` rather than looking one past), so this was never a
  shared defect; it was specific to the `LIMIT per_page+1` plus `truncate` shape.

### An op for an unjoined Stoa creates no membership, and that needs no code

The requirement is satisfied by the absence of a call. There is no path from
`OpLog::append` to `MembershipStore`, the two types share no state, and nothing
in the receive path this change does not own could reach membership without a
new call site. **So the tests for this requirement are written against the
absence**: append ops through the real log, then read the real membership store,
and assert the listing is untouched. That is the honest shape — a test asserting
a function was not called would be asserting on an implementation, where this
asserts on the observable state the spec constrains.

## Risks / Trade-offs

- **Two stores, two `user_version`s** → `MEMBERSHIP_LAYOUT_VERSION` is pinned by
  a hardcoded assertion, following `identity.rs`'s wire constants and
  `sqlite.rs`'s own layout version, because `cargo mutants` cannot see a wrong
  `const`. Neither refusal reads the other's constant, which is what the
  independence test asserts — and **not** what separates the two stores, since
  both values are `1`. `check_layout` naming columns does that; see the file
  decision above.

- **A membership store refuses an unknown layout, and the op log is then still
  readable** → this is the asymmetry the separate FILE buys and it is worth
  naming: a peer holding a membership store from a future build cannot list its
  Stoas and can still read every op it holds. That is the recoverable direction
  — the alternative, one file, makes an unknown membership layout cost the user
  their ops. This survives both layout versions being `1`, which is why it is a
  distinct claim from the one above rather than a restatement of it.

- **A row whose `genesis_bytes` no longer decode** → reported as
  `MembershipError::CorruptEntry`, never skipped. Skipping would make a
  corrupted row indistinguishable from a Stoa the user never joined, which is
  `OpLogError`'s "an empty feed is not an unreadable store" argument applied to a
  listing.

- **`stoas.sqlite` is a new file in the host's persistence directory** → no
  migration and nothing to clean up: a peer that never creates or joins a Stoa
  never gets the file, and a peer that does gets it created on first write.

- **The creator's key is the root identity, and one key signs in every Stoa** →
  the cost is cross-Stoa linkability: anyone observing two Stoas can link the
  same participant across them, because the public key is the join. This is
  §9.2's accepted scope decision and not this change's to make — see the creator
  key decision above, which also records why the alternative it replaced was
  worse on the same axis.

- **Two files mean no transaction can span membership and ops** → a future "join
  and backfill" is foreclosed as one atomic operation. Stated with the file
  decision above; repeated here because it is a trade-off and not only a
  rationale.

## Unspecified behaviour, marked in the code

Each of these is a `// NO SPEC:` marker on a test or at the choice, and each is a
question for the spec-writer rather than a decision this change should own:

1. **What `policy` a created Stoa declares.** `Policy::Open` is the only variant
   and the spec does not say creation may choose. Creation accepts no policy
   parameter and always declares `Open`.
2. **What a posting policy is CALLED on the wire.** `policy_name` returns the
   lowercase literal `"open"`. The spec requires the policy be answerable and
   fixes no spelling, so this change chose one — and a view that branches on the
   string makes changing it a breaking change to the module surface. This is the
   entry that was missing: there are five NO SPEC subjects in the code and this
   was the one design.md did not carry.
3. **Whether an unknown field in a request is refused.** Ignored, matching
   `list_threads`'s treatment of an offered `order`.
4. **What an empty listing's `hasMore` is.** `false`.
5. **What a `per_page` of zero lists.** An empty page with nothing after it.
   Unreachable through the wire (`clamp_per_page` turns `0` into the default), so
   this is a decision about the crate's own API; it is the answer that cannot hang
   a caller paging to exhaustion.

**Which key the creator is used to be on this list and is not any more.** It is
settled by PLAN §5.2's MVP subsection, which the spec-writer should read rather
than re-decide: the spec's "the key the caller would sign an op with" and the
one-identity rule together name exactly one key. See the decision above.
