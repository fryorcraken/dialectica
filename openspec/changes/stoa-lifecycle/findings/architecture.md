# Architecture review — `stoa-membership`

Dimension: **architecture** only. Correctness, security and readability are other
instances' — where a finding below touches one of those, it is because the *shape*
is what produces it, and the shape is what is being reported.

Reviewed in `.claude/worktrees/rev-stoa-architecture` at `307bf13`. Baseline
measured before any conclusion below: `cargo test -p dialectica-core` — **550
passed, 0 failed**. Nothing in this worktree was mutated; the tree is clean (see
the closing note).

Standard applied: `CLAUDE.md`'s four — complexity in the data structure not the
logic; one function one job; do not let a function quietly acquire a second caller
with different needs; the core API is the deliverable — plus the forced core/UI
split.

---

- [x] **1. Which key the creator is, is decided in the one file no test can compile — for `dev-writer`**

`design.md`'s longest Decision ("Which key the creator is: the root identity, used
directly") settles a question that had already shipped wrong once: creation named
`derive_stoa_key(root, [0u8;32])` while the probe reported
`derive_stoa_key(root, stoa_address)`, so `Moderators::of(genesis).contains(posting_key)`
was false for a Stoa's own creator. The fix is correct. **The fix's shape is not:
the decision still lives at two independent call sites in `cfg(logos_scaffold)`
code, and nothing in `core` names it.**

Measured:

- `grep -n "identity_public_key\|identity_address" dialectica/rust-lib/dialectica-core/src/wire.rs` — **no output**. Neither method is mentioned anywhere in `core`'s wire surface.
- The two choices are made only at `dialectica/rust-lib/src/lib.rs:346` (`ks.identity_address()`, for the probe) and `dialectica/rust-lib/src/lib.rs:385` (`ks.identity_public_key()`, for the creator).
- `dialectica/rust-lib/build.rs:40` sets `logos_scaffold` only when `generated/provider_gen.rs` exists; `git ls-files dialectica/rust-lib/generated` returns **nothing**, so the file is never committed and `cargo test` never compiles either line. **The brief's claim is confirmed by measurement, not accepted on trust.**
- The team knows: `keystore.rs:3049-3050` says in a comment *"This is the pair the adapter wires up, checked here because `cfg(logos_scaffold)` is not built by tests."* That test
  (`the_creator_of_a_stoa_this_keystore_made_can_moderate_it`) pins a property of
  **two `Keystore` methods**. It does not and cannot pin that the adapter calls
  those two.

Failure scenario, concrete and reproducible: in
`dialectica/rust-lib/src/lib.rs:385`, change `ks.identity_public_key()` to
`ks.stoa_public_key(&ks.identity_address())`. It type-checks (`stoa_public_key`
returns the same `PublicKey`). `cargo test -p dialectica-core` stays at 550/550
because the line is not compiled. The **Build LGX** CI job *does* compile the
adapter (`.github/workflows/ci.yml:888`, `lgs basecamp build --variant all`), so a
*type* error would be caught — but a wrong *method choice* of the same type compiles
green. The result is the exact bug the Decision exists to prevent: every Stoa the
peer creates names a creator key it never signs with, fixed inside the address
preimage forever, and the only symptom is that moderation silently never binds —
which nothing in this change exercises, because there is no moderation call yet.

Why this is a shape finding and not a preference: **the closure bodies contain no
host type at all.** Both are three `core` calls over a `PathBuf`:
`core::keystore::default_path_in(&dir)` then `core::keystore::open_from_env(&path)`
then one accessor. `open_from_env` takes `&Path` and `default_path_in` takes `&Path`
(`keystore.rs:368`, `keystore.rs:381`). So "this crate cannot read the environment
or know the host's layout" — the stated reason for the closure — does not apply to
*which accessor is called*; it applies only to *which directory*. The directory is
already an ordinary parameter everywhere else (`membership_path_in(&dir)`,
`dir.join("ops.sqlite")`).

The shape that makes it structural: two named `core` functions over a directory —
one for the creator key, one for the probe's identity — so the adapter reads
`|| core::creator_key_in(&dir)` and `|_| core::identity_in(&dir)`, and "the creator
and the poster are one key" becomes a property of two functions a test can call.
The closure parameter **stays** — it is genuinely earning its place, see Checked
and clean #2 — and the adapter keeps passing one; what moves into `core` is the
body, which is the part carrying the decision.

`dialectica/rust-lib/src/lib.rs:336-347` and `dialectica/rust-lib/src/lib.rs:375-389`

**Outcome: FIXED**, in exactly the shape this entry proposes. Same seam as
`findings/security.md` entry 3, found independently by both dimensions; answered
once in both files.

The observation that made it possible is this entry's, and it is the load-bearing
one: **the closure bodies contain no host type at all** — three `core` functions
over a `&Path`, so "this crate cannot read the environment or know the host's
layout" applies to *which directory* and not to *which accessor*. That is what I
checked first, and it holds.

Implemented slightly tighter than the two functions suggested. Rather than
`core::creator_key_in(&dir)` and `core::identity_in(&dir)` as two independent
functions — which would still be two derivation positions, just relocated somewhere
a test can reach — both are wrappers over one
`core::keystore::creator_and_poster_in(dir) -> (PublicKey, Address)`. Both halves
come out of one expression over one `identity_key()` root, so "the creator and the
poster are one key" is a property of one function rather than of two that agree.
The closure parameter stays, per this entry's own note and Checked-and-clean #2.

**The test that fails without it:**
`wire.rs::the_creator_a_creation_names_is_the_identity_the_probe_reports`, which
drives both wire handlers through those functions against a real on-disk keystore.
This entry's own named failure scenario — swapping in a `stoa_*` accessor — now
gives **550 passed, 1 failed**; before, the suite stayed at 550/550.

A Lint step was added for the residue a test still cannot reach (whether the
adapter *calls* those functions at all): `the adapter derives the creator and the
poster in one place`. Both its checks were verified to fire.

`design.md` records it under *"Where that decision lives:
`core::keystore::creator_and_poster_in`, because two agreeing call sites are not
one derivation"* — including what the grep cannot see, which is correctness.

---

- [x] **2. Three storage-path conventions in three different modules — for `dev-writer`**

`CLAUDE.md`: *"When you find yourself writing the fourth slightly-different copy of
a guard, that is the signal to reshape rather than to add a fourth test."* This
change adds the third copy of a *path convention*, and put it in a third place.

Measured, by grep:

| Store | File name decided in | Layer |
|---|---|---|
| the keystore | `keystore.rs:381` `default_path_in` → `identity.key` | the storage module that owns it |
| the op log | `dialectica/rust-lib/src/lib.rs:362` `dir.join("ops.sqlite")` — inline, in the adapter | the host adapter |
| memberships | `wire.rs:802` `membership_path_in` → `stoas.sqlite` | the **wire contract** module |

`grep -rn "ops.sqlite\|default_path_in\|stoas.sqlite" src/log/sqlite.rs src/log/mod.rs` returns
**nothing** — the op log module names no file at all.

`membership_path_in`'s own docstring says it follows `keystore::default_path_in`
("the naming convention belongs with the thing named"), and then does not: it lives
in `wire.rs`, the module whose stated job is *"the wire contract: the guard, and the
handler bodies behind it"* (`wire.rs:1`). A file name on disk is not wire contract.
`membership.rs` is the module that owns the file and is where the analogue it cites
would put it.

The specific future change this makes harder: adding a fourth store — the
vouching/weight state `docs/relevance-votes` is designing, or metadata resolution's
own projection. There is no convention to follow, so the author picks a fourth home,
and "where does dialectica keep its files" stops being answerable by one grep. The
cost is already visible: this change's author had a 50/50 choice and the docstring
records the wrong one.

`wire.rs:802`, against `keystore.rs:381` and `dialectica/rust-lib/src/lib.rs:362`

**Outcome: FIXED for this change's own copy; the third convention is recorded rather
than unified.**

`membership_path_in` moved to `membership.rs` — the module that owns the file, and the
one the analogue it cites would put it in. `wire.rs` re-exports it, so the adapter
still reaches `core::membership_path_in` and no caller changed. The entry's sharpest
line is the one that settled it: *"its own docstring says it follows
`keystore::default_path_in` … and then does not"*, and *"a file name on disk is not
wire contract"*. Both are right.

Two of three conventions now agree: the store that owns a file names it
(`keystore::default_path_in` → `identity.key`, `membership::membership_path_in` →
`stoas.sqlite`).

**The third is left as it is, deliberately.** `dir.join("ops.sqlite")` is inline in
the adapter and the op log module names no file at all — the entry's grep confirms it
— so unifying it means adding a function to `log/` and changing a call site in
`cfg(logos_scaffold)` code, for a module this change otherwise does not touch. That
is a change to the op log, not to Stoa lifecycle. What this change owes the next
author is the convention to follow, and it now exists in two places with the rule
stated: the naming belongs with the thing named. Recorded in `design.md` beside the
file decision.

So the entry's specific worry — *"adding a fourth store … there is no convention to
follow, so the author picks a fourth home"* — is answered: there is now a convention,
followed twice, with the one exception named.

---

- [x] **3. `list`'s two boundary guards return the same value twice, and the asymmetry the brief asks about is coherent but undocumented at one of the two sites — for `dev-writer`**

The brief asks whether the `per_page`-saturates / `page`-refuses split is one
boundary or two rules. **It is one boundary, and the reasoning is sound** — see
Checked and clean #1 for why I am not asking for it to be unified. The defect is
narrower and is a shape defect:

`MembershipStore::list` contains the identical literal twice —

```rust
return Ok(MembershipPage { items: Vec::new(), page, has_more: false })
```

— at `membership.rs:501-507` (the `per_page == 0` early return) and at
`membership.rs:549-558` (the `i64::try_from(offset)` refusal). Two spellings of
"this is an empty last page", in one function, thirty lines apart, reached for two
unrelated reasons.

Why that is a defect and not tidiness: this is the function whose two live bugs were
*exactly* a page and a `has_more` that disagreed because they were computed in two
places. The fix for the third fact (`split_off`, `membership.rs:581`) correctly made
one cut where there had been two. The two early returns are the same hazard left
standing: a future edit that adds a third unreachable-page case, or that changes what
an empty last page reports (`page` verbatim? `has_more` on an empty store?), has to
find and change both, and the compiler will not say so.

Failure scenario, reproducible: change only the first literal's `has_more` to `true`
and run the suite. `a_per_page_of_zero_terminates_rather_than_paging_forever`
(`membership.rs:1514`) fails — good. Now change only the *second* literal's and leave
the first: `a_page_index_too_large_to_offset_answers_empty_rather_than_the_first_page`
(`membership.rs:1456`) fails — also good, so both are covered today. The defect is not
a coverage gap, it is that the two facts are *independently* mutable when the function
means them to be one fact. One named constructor —
`MembershipPage::empty_last_page(page)` or equivalent — makes it one, and makes the
second call site read as the same answer rather than as a coincidence.

Second, smaller, at the same site: `membership.rs:521` computes
`let offset = page.saturating_mul(per_page);` and `membership.rs:549` shadows it with
`let offset = match i64::try_from(offset) {…}`. The shadow is between them separated
by the `prepare` call, so the `usize` `offset` is live across a fallible statement
preparation that does not use it. Reordering so the conversion sits beside its own
computation would put the whole boundary in one place — which is what the third
bullet of `design.md`'s `list` decision claims it already did.

`membership.rs:501-507`, `membership.rs:521`, `membership.rs:549-558`

**Outcome: FIXED, both halves, in the shape the entry names.**

**The duplicated literal.** `MembershipPage::empty_last_page(page)` is the named
constructor, and both early returns call it. The entry's argument for why this is a
defect and not tidiness is the one recorded in its doc-comment: this is the function
whose two live bugs were *exactly* a `page` and a `has_more` computed in two places
that disagreed, and the two returns left that hazard standing with the compiler
unable to say so.

**Measured, and the result is better than "still covered".** The entry's own
verification was that mutating *either* literal fails a different test, so both were
covered and the defect was independent mutability. Now there is one thing to mutate:
setting `has_more: true` in `empty_last_page` fails **both**
`a_per_page_of_zero_terminates_rather_than_paging_forever` and
`a_page_index_too_large_to_offset_answers_empty_rather_than_the_first_page` — one
mutation, two failures, which is what "the function means them to be one fact" looks
like from the test side. The constructor's doc records which half is load-bearing
(`has_more: false`, or a caller paging to exhaustion never terminates) and why `page`
is reported verbatim rather than clamped.

**The second, smaller item.** The `usize` `offset` no longer shadows the `i64` one:
it is `row_offset`, and the `i64::try_from` moved up beside its own computation so
`row_offset` / `limit` / `offset` are three consecutive lines and the `prepare` call
no longer sits between the two bindings. That is also
`findings/readability.md` entry 8, answered by the same edit — and it makes
`design.md`'s `list` decision true, which the entry correctly noted claimed this had
already been done.

I did **not** unify the `per_page`-saturates / `page`-refuses split, per the entry's
own Checked-and-clean #1: it is one boundary with sound reasoning, and the entry
explicitly says it is not asking for that.

---

- [x] **4. `with_membership_store` hands `&mut` to a read-only handler, erasing which of the three writes — for `dev-writer`**

`list_stoas` takes `&crate::membership::MembershipStore` (`wire.rs:727`) — read-only,
deliberately and correctly. `with_membership_store`'s handler bound is
`impl FnOnce(&mut MembershipStore) -> String` (`wire.rs:779`), so the adapter's
`list_stoas` arm (`dialectica/rust-lib/src/lib.rs:407-409`) receives `&mut` and
reborrows it down. The type at the seam therefore says all three handlers write,
when one of them provably does not.

This is the brief's question about whether the generic is one job or a seam that
acquires a second caller with different needs — and the second caller with different
needs **has already arrived**: it is `list_stoas`, and the seam absorbed the
difference by widening rather than expressing it.

The specific future change this makes harder, and it is not hypothetical here: this
store and the op log set no `busy_timeout` and no WAL journal mode (`grep -rn
"busy_timeout\|journal_mode\|WAL" membership.rs log/sqlite.rs` → **no output**), and
the adapter's own comment at `dialectica/rust-lib/src/lib.rs:356-357` raises the case
it matters for — *"the host may hand the same path to another instance"*. The first
change anyone makes when two instances contend is to let readers proceed while a
writer holds the write lock. At that point the read path has to be distinguishable
from the write path at the seam, and it is not: every arm goes through one `&mut`
opener. Reshaping then means touching all three arms plus the opener, in
`cfg(logos_scaffold)` code no test compiles.

The reshape that costs nothing today: two functions, or one generic over a
`Fn(&MembershipStore)`/`FnMut(&mut MembershipStore)` distinction — so `list_stoas`'s
read-only-ness survives the seam it is reached through. `Connection` is not `Sync`, so
this buys no concurrency by itself; what it buys is that the type stops asserting
something false, and that the change above is local when it comes.

I have **no measurement** that this has caused a failure. It has not; it is a shape
finding, offered as one.

`wire.rs:776-787`, against `wire.rs:727`

**Outcome: FIXED** — two functions, which is the first of the two options the entry
offers. `with_membership_store` keeps `FnOnce(&mut MembershipStore)` for
`create_stoa` and `join_stoa`; `with_membership_store_read` takes
`FnOnce(&MembershipStore)` and is how `list_stoas` reaches its store, in the tests
and in the adapter.

The entry's framing is what made this worth doing now rather than when it bites, and
it is quoted in the code: the second caller with different needs **has already
arrived** — it is `list_stoas` — and the seam absorbed the difference by widening
rather than expressing it.

The doc states what the split does and does not buy, using the entry's own
measurement: `rusqlite::Connection` is not `Sync` and both halves still open per call,
so **no concurrency is gained**. What is gained is that the type stops asserting
something false, and that the change the entry forecasts — letting readers proceed
while a writer holds the write lock, once `busy_timeout`/WAL matter because the host
hands the same path to another instance — becomes a change to one function instead of
to all three adapter arms plus the opener, in `cfg(logos_scaffold)` code no test
compiles.

`findings/readability.md` entry 2 is the same defect seen as a false comment; one
change answers both, and the doc there no longer claims all three handlers take
`&mut`. 550 tests pass, as they must for a reshape that changes no behaviour.

---

- [x] **5. `with_membership_store` takes a `method` name that is already inside every handler it can be given — for `dev-writer`**

`with_membership_store(method, path, handler)` calls `guarded(method, …)`
(`wire.rs:781`). Each of the three handlers it is given *also* calls `guarded` with
its own hardcoded name — `guarded("create_stoa", …)` at `wire.rs:611`,
`guarded("join_stoa", …)` at `wire.rs:681`, `guarded("list_stoas", …)` at
`wire.rs:728`. So the guard is nested, and the outer one's `method` is a second,
caller-supplied copy of a name the inner one already knows.

Two consequences, both reproducible:

- **The outer `method` is reachable only by a panic in `MembershipStore::open`**, because the inner guard catches everything else and returns a `String`. So the parameter names a frame that, for these three callers, can only ever report an open failure — and it is the *handler's* name that gets reported for it.
- **It can disagree with the handler and nothing notices.** Call `with_membership_store("list_stoas", &path, |store| create_stoa(req, key, store))`. It compiles, runs, and a panic in `open` is reported as `panic in list_stoas`. `wire.rs:3107` and `wire.rs:3116` in the existing tests pass the matching name by hand each time; nothing checks that they match, and the adapter has three more hand-written pairs (`dialectica/rust-lib/src/lib.rs:375`, `:397`, `:407`).

This is `CLAUDE.md`'s *"do not let a function quietly acquire a second caller with
different needs"* read the other way: a parameter that exists so a caller can
re-state what the callee already knows is a parameter that will eventually be
re-stated wrongly. The shapes that remove it: drop the inner guard from the three
handlers (the outer one covers the same body, and `with_membership_store`'s docstring
already argues the guard must wrap the open too), or drop the outer `method` and let
the inner guard own the name, accepting that a panic in `open` is reported under a
generic label. Either is one fact in one place.

**Not a correctness finding** — the nesting is harmless, the guard fires either way,
and I am not reporting a double-catch as a defect. It is the parameter that is the
defect.

`wire.rs:776-787`, `wire.rs:611`, `wire.rs:681`, `wire.rs:728`

**Outcome: FIXED**, taking the **second** of the two shapes the entry offers: the
outer `method` is gone and the inner guards keep the names. Both halves of the
parameter's defect go with it — there is nothing left for a caller to re-state, so
nothing left to re-state wrongly, and the five hand-written pairs (two in tests, three
in the adapter) are five fewer things to keep in step.

**Why not the other option** — dropping the inner guards so the outer one covers the
body. That would have moved the method name *out* of the report for every panic, not
just for a panic in `open`: the handlers' own `guarded("create_stoa", …)` is what makes
`{"error":"panic in create_stoa: …"}` name the method a view called, and
`guard_names_the_method_that_panicked` pins that. Keeping the inner guards and dropping
the outer parameter loses nothing a caller can observe.

The outer guard **stays**, with a generic label. The entry's own observation is what
makes that honest rather than a fudge: the outer frame is reachable *only* by a panic
in `MembershipStore::open`, because the inner guard catches everything else and returns
a `String`. So `"opening the Stoa membership store"` is accurate for everything it can
ever catch, and the doc says exactly that.

I also kept the guard wrapping the open, which the entry notes the docstring already
argued for and which is not in dispute: a panic while opening a store aborts the module
process like any other.

---

- [x] **6. `membership.rs` cites the wire's `clamp_per_page` to justify its own unreachability — for `dev-writer`**

`membership.rs:498-500`, inside the storage module:

```
// NO SPEC: the spec does not say what a `per_page` of zero lists, and the
// wire never produces one (`clamp_per_page` turns 0 into the default). This
// is the answer that cannot hang a caller.
```

`MembershipStore` is `pub` on a `pub mod` and knows nothing about `wire`. This comment
makes the storage layer's correctness argument depend on a caller one layer up — the
inverted direction. `design.md` already states the honest version: *"`list` is
correct for arguments the wire cannot produce, and that is a deliberate cost rather
than defensiveness … it is reachable through the crate, which is the deliverable."*
The code comment says the opposite thing (it is fine because the wire clamps), which
is the reading that invites someone to delete the guard when `clamp_per_page` changes.

The specific future change this makes harder: raising `MAX_PER_PAGE` or changing
`clamp_per_page(Some(0))` to pass `0` through (`feed.rs:175-178`) is a change in
`feed.rs` that silently alters what this comment claims about `membership.rs`. The
reshape is a comment edit, not a code edit: say the guard is unconditional because the
function is `pub`, and drop the wire citation. Same for the duplicate at
`membership.rs:1530-1533`.

Related, same direction, `wire.rs:753-765`: `with_membership_store`'s docstring says
*"hand it to one of the three handlers above"* and *"generic over the handler rather
than written three times"*. A `pub` generic function with three named permitted
callers is either not generic or not restricted to three; as written a reader cannot
tell which constraint is real. No measurement — a documentation-shape finding.

`membership.rs:498-500`, `membership.rs:1530-1533`, `wire.rs:759`

**Outcome: FIXED**, all three sites, and it was a comment edit rather than a code edit
exactly as the entry says.

Both `membership.rs` copies now argue from the function being `pub`: *"the guard is
UNCONDITIONAL because this function is `pub` on a `pub mod`, and that is the whole
reason."* The wire citation is dropped, and each says why — a storage module whose
correctness argument rests on a caller one layer up invites someone to delete the guard
when that caller changes. The specific edit the entry names is spelled out at the first
site: raising `MAX_PER_PAGE` or letting `clamp_per_page(Some(0))` pass zero through is a
change in `feed.rs`, and it must not be able to make this function wrong.

The entry is also right that `design.md` already had the honest version, so this was a
comment contradicting the design document it should have been quoting.

**The `wire.rs:759` half — the "three named permitted callers" docstring — is fixed as
a side effect and worth noting as such.** That whole paragraph was rewritten for
`findings/architecture.md` entries 4 and 5 (the seam split and the dropped `method`
parameter), so the sentence the entry objects to — a `pub` generic function with three
named permitted callers, where *"a reader cannot tell which constraint is real"* — no
longer exists. The replacement says which handler reaches which of the two functions
and why there are two, which is a constraint a reader can check against the
signatures.

---

- [ ] **7. What the two-file seam forecloses that `design.md` does not record: the two stores can disagree and nothing can detect it — for `spec-writer`**

`design.md` records one foreclosure — no transaction spans membership and ops, so a
future atomic "join and backfill" is unavailable — and argues the recovery asymmetry
is worth more. **I agree the seam is in the right place, for the reason `design.md`
gives** (see Checked and clean #3). The finding is that the *recorded* cost is the
smaller half.

The unrecorded half: the membership store retains a genesis record naming a creator
`PublicKey`, and the **keystore is a third file that the membership store has no
relationship with at all**. `create_stoa` reads `identity.key`, mints a record naming
that key's public half, and writes it to `stoas.sqlite`. Nothing ever re-checks the
two against each other.

Failure scenario, concrete: a peer creates a Stoa; the user later restores
`identity.key` from a different backup, or re-runs onboarding, so the root secret
changes. `list_stoas` still lists the Stoa and still reports its `foundingTitle`
truthfully. `MembershipStore::get` still verifies (the record still hashes to its
address — `decode_row` at `membership.rs:640` checks record-against-address, which is
unaffected). But `Moderators::of(genesis).contains(identity_public_key())` is now
false: the peer holds a Stoa it created and cannot moderate, and **every check in this
change passes**. There is no call that can report it, because no call in this
capability compares the retained creator against the current identity.

Why this belongs to `spec-writer` and not `dev-writer`: nothing in the
`stoa-membership` spec asks for it, and adding a check would be widening the surface
without a requirement — which is precisely what `CLAUDE.md` says not to do. What is
missing is the *contract question*: is "a Stoa this peer created under a key it no
longer holds" a state the module must be able to report, or a state the spec
deliberately says nothing about? Either answer is fine; the silence is not, because
the state is reachable, is invisible, and the symptom appears only when moderation
lands — which is the same "invisible until a moderation op arrived" argument the spec
already makes at its own retention requirement (`spec.md:165`).

`design.md`'s "What two files foreclose" paragraph; `membership.rs:640`;
`dialectica/rust-lib/src/lib.rs:384-385`

**Outcome: OPEN — this is `spec-writer`'s, and the box stays unticked.** Noted by
`dev-writer` so it is not read as forgotten, and because the entry's reasoning for the
routing is right and I am not going to work around it: nothing in the
`stoa-membership` spec asks for this check, and adding one would widen the surface
without a requirement.

**I verified the failure scenario rather than taking it on trust, and it holds.**
`decode_row` checks record-against-address and is unaffected by a changed root
secret, so `MembershipStore::get` still verifies and `list_stoas` still answers
truthfully — while `Moderators::of(genesis).contains(identity_public_key())` becomes
false. Nothing in this capability compares the retained creator against the current
identity, so there is no call that could report it.

**What I did do, which is short of the contract answer the entry asks for.** The state
is now recorded in `design.md` under Risks / Trade-offs as a reachable, undetectable
one, with the scenario and the reason no check exists. That is a note, not a
requirement — the question *"is a Stoa created under a key the peer no longer holds a
state the module must be able to report?"* is still unanswered, and answering it is
the spec's job.

One thing worth adding for whoever picks it up: this is **adjacent to but distinct
from** `findings/security.md` entry 3 and this file's own entry 1. Those were about two
derivations of the *current* identity disagreeing with each other, which is now
impossible by construction (`keystore::creator_and_poster_in`). This one is about the
current identity disagreeing with a *record written in the past*, which no amount of
one-derivation-position fixes — the record is immutable and the key can change under
it. The reshape does not touch this finding, and it should not be assumed to.

---

## Checked and clean

These are settled. Re-opening one costs an afternoon and reaches the same place.

1. **The `per_page`-saturates / `page`-refuses asymmetry is one coherent boundary, not two rules.** I tried to find a single rule and there is not one, because the two questions differ in whether they are *answerable*. "More rows than exist" has an answer (all of them), so saturating the limit at `i64::MAX` serves it. "A page past `i64::MAX` rows in" has no answer, and the only shapes available are empty (honest) or clamped-to-the-last-page (which silently answers a different question) — and a bare `as i64` cast is worse still, because SQLite reads a negative `OFFSET` as none and serves page 0 to a caller who asked for page `usize::MAX`, pinned by `a_page_index_too_large_to_offset_answers_empty_rather_than_the_first_page` (`membership.rs:1456`). Unifying the two would mean making one of them lie. Finding #3 is about the duplicated *literal*, not about this.

2. **The closure-passing pattern is the right boundary, and should stay.** I checked whether it pushes decisions where they cannot be tested, and for the *closure parameter* the answer is no — the tests exploit it heavily and could not exist without it: `wire.rs:1187` injects a synthetic `KeystoreError` into the probe without a keystore file, and `wire.rs:2118` does the same for `create_stoa` ("no usable signing key" would otherwise need a broken file on disk). Removing the closure in favour of a path would make the spec's "no key means no Stoa" scenario reachable only through filesystem fixtures. What is wrong is the closure *body*'s location, which is finding #1 and a strictly smaller change.

3. **Two SQLite files is the right seam.** `design.md`'s table of the three alternatives holds up against the code: `log/sqlite.rs`'s `create_schema` runs only at `user_version == 0` and its `check_layout` names only `ops` columns, so a `memberships` table added to that batch would reach fresh stores and pass the layout check on existing ones, failing at the first read. The recovery asymmetry argument is the decisive one and is correctly stated: an unknown membership layout costs the user their Stoa list; one file would cost them their ops. Nothing in this change edits `SqliteOpLog`, which I verified by grep — so the spec's "opening an existing op store still works" is structural rather than tested-into-place.

4. **Creation reusing `join`'s write path is right, and the ordering claim holds.** `create_stoa` (`wire.rs:642`) computes the address via `genesis.address()` before touching the store, and `join` (`membership.rs:399`) encodes before its first statement. So "a failed creation leaves nothing behind" is a consequence of the call order and not a rule to remember, exactly as claimed. One write path also means the spec's "creating the same title twice" and "creating then joining" are the same `INSERT OR IGNORE` rather than two mechanisms that must agree.

5. **The `foundingTitle` / `FOUNDING_TITLE` constant, and `policy` being absent from list items.** Both are deliberate narrowings of the deliverable API rather than omissions, both are argued in `design.md`, and the argument is the right one: a `{"title":…,"isFounding":true}` shape makes `title` mean two things depending on a sibling, and widening the paginated item shape is a decision for whoever needs it. `policy_name`'s exhaustive match with no wildcard arm (`wire.rs:555`) is the same position applied one layer up from `Policy::from_byte`, and is correct — a wildcard would tell a view a token-gated Stoa is world-postable.

6. **Nothing in `core` knows about the host.** Checked directly: `membership.rs` imports only `crate::*`, `rusqlite`, `std::fmt`, `std::path::Path`. `MembershipStore::open` takes a `&Path`, matching `SqliteOpLog::open` and `keystore::default_path_in`. The one host fact — `instance_persistence_path` — is held in exactly one field in the adapter and set in exactly one place (`dialectica/rust-lib/src/lib.rs:419`). Finding #2 is about *where in core* a file name lives, not about core knowing the host.

### Dead ends, written down so nobody spends the afternoon again

- **A trait-driven sweep to make the adapter's wiring testable is not available here, and not for the reason it first looks.** The obstacle is not the `cfg`: it is that there is nothing to abstract *over*. The adapter's four handler bodies contain exactly one thing `core` cannot express — `modules().delivery_module` (`dialectica/rust-lib/src/lib.rs:320`), which calls `lp_*` symbols undefined in a test binary. Every other line in every other arm is a `core` call over a `PathBuf`. So a trait would have one method, one implementation, and one `cfg`'d test double, to cover a single call this capability does not touch. Finding #1's reshape — move the closure *bodies* into named `core` functions — reaches the same place with no trait and no double, because the bodies were never host-coupled in the first place.

- **`cargo mutants` is the wrong instrument for the two shapes this review is about, and I did not run it.** The brief for a different dimension asks for it; here it would measure nothing new. `design.md` already records the measurement that matters (`INSERT OR REPLACE` plus deleting one `assert_eq!` leaves 544 green, because the pre-insert verification makes a mismatched pair unreachable), and it records that no mutant is generated for `saturating_add`, `saturating_mul` or `try_from` — which is every arithmetic site in finding #3. For finding #1 it is structurally blind twice over: the line lives behind a `cfg` `cargo test` does not set, so it is not in the mutation set at all. I did not re-run it, so the tree was never mutated.

- **Two-instance concurrency is a pre-existing posture, not this change's regression.** I checked for `busy_timeout`, `journal_mode`/WAL and `foreign_keys` in both `membership.rs` and `log/sqlite.rs` and found none in either. So the membership store matches the op log exactly, and reporting it here would be reporting the op log's decision against this change. It is named in finding #4 only as the future change that makes the `&mut` seam expensive — not as a defect this piece introduced.

---

**Tree state:** nothing in this worktree was modified. No mutation experiment was
run (see the second dead end for why), so there is nothing to restore.
`cargo test -p dialectica-core` measured 550 passing before and the tree is
unchanged since. `git status` clean apart from this findings file.
