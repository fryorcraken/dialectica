# Design review — `identity-onboarding`

Scope: did the code take the decisions `design.md` records, and were the decisions
worth recording recorded? Not code quality, not test coverage — those are the
other reviewers'.

**The verdict up front.** The Decisions section is unusually good by this
project's standard: every headline entry names its alternative and what ruled it
out, the three pinned hex constants all carry their `openssl` provenance
*including* the validation-by-reproducing-v1 step, and the two claims I was asked
to verify against `log/sqlite.rs` and the derivation chain are both true about the
code (with one wrong line-number set, noted below as a nit).

What is wrong is a different shape: **three decisions the code took are not in
`design.md` at all, and two of them make a recorded decision false in the running
system.** Every one of them lives on the adapter path (`dialectica/rust-lib/src/lib.rs`),
which `cargo test` cannot reach — so the green suite is structurally unable to see
any of them. That is the pattern, and it is worth naming: the entries `design.md`
argues carefully are the ones inside `dialectica-core`, where tests hold the
fixtures fixed; the entries it is silent about are the ones the fixtures paper
over.

Ordered by severity.

---

- [x] **1. `keepIdentity` can never record a second Stoa, which makes a recorded decision false**

**For: `dev-writer`** (and `spec-writer` for the scenario, see below)

**The recorded decision.** `design.md:121-126`:

> **Two columns, keyed by Stoa.** `(stoa BLOB PRIMARY KEY, path INTEGER NOT NULL)`.
> The primary key is what makes "one chosen path per Stoa" hold by construction
> rather than by a guard at each write […] and it is what makes the spec's
> "Distinct choices for distinct Stoas are recorded separately" scenario a
> property of the schema.

`identity_store.rs:253-260` repeats it.

**What the code does.** `wire.rs:530-542` — every keep calls
`targets.keystore.create(...)` **before** `record_path`, and returns early on
`Err`:

```rust
if let Err(e) = targets
    .keystore
    .create(targets.keystore_path, targets.unlock)
{
    return Kept::Refused { reason: e.to_string() };
}
if let Err(e) = targets.paths.record_path(stoa, candidate.path) {
```

`Keystore::create` returns `KeystoreError::AlreadyExists` whenever the file
exists (`keystore.rs:803-805`). The keystore is **one file for the whole
install**, not one per Stoa (`keystore::default_path_in`, `lib.rs:455`).

**The scenario.** A user keeps an identity in Stoa A. They then join Stoa B and
call `keepIdentity` with B's address and a live slate:

1. `lib.rs:434` → `master_key(&dir)` → `open_from_env` succeeds (A's keystore is
   on disk), so the existing key is returned.
2. `wire.rs:530` → `create` → `AlreadyExists`.
3. Reply: `{"kept":false,"reason":"a keystore already exists at that path …"}`.

The `chosen_paths` table can therefore never hold more than one row on any path a
wire caller can take. The primary key is not "making an invariant hold by
construction" — it is guarding a table that structurally cannot receive a second
insert. The spec scenario the decision claims to discharge
(`specs/identity-onboarding/spec.md:188-191`, "Distinct choices for distinct Stoas
are recorded separately") is unreachable through the API.

**Why the suite does not see it.** Every multi-Stoa test writes the second row
with `store.record_path(...)` directly, bypassing `keep_identity`:
`wire.rs:2469-2471` (`distinct_stoas_report_distinct_identities`) and
`wire.rs:2889-2890`. There is no test that keeps two Stoas through the wire. I
checked all six `elsewhere` sites in `wire.rs`; none goes through `keep_identity`
twice.

**Why it matters, beyond the decision being false.** `identity-onboarding`'s own
spec makes identity per-Stoa and the whole capability exists so a fresh install
can post; a user who joins a second Stoa reaches a permanent dead end with a
reason string that names the wrong problem. And the second-keep refusal the spec
*does* want ("Keeping an identity does not replace an existing one",
`spec.md:252-268`) is currently doing double duty as an accidental
one-Stoa-per-install limit. Those are two different refusals sharing one
mechanism, which is exactly the collapse `KeystoreError`'s distinguishability
doctrine exists to prevent.

**What is missing from `design.md`.** Either a Decisions entry saying the keep
path is deliberately one-Stoa-per-install in this change and why (with the cost
named: the spec scenario is contract-only until a later change), or the
recognition that `create`-refuses-first cannot be both the second-keep guard and
the per-Stoa gate. Right now `design.md` claims the schema discharges a scenario
the code cannot reach, which is the more misleading of the two states.

**Fixed** in `63133c9` — the **second** of the two options, and not the first.

The entry offers them even-handedly, but they are not equal: documenting the limit
would have left a user who joins a second Stoa at a permanent dead end, with a reason
string naming a keystore they did not know they had, for the sake of an accurate
`design.md`. The sentence that settles it is this entry's own: *"Those are two
different refusals sharing one mechanism, which is exactly the collapse
`KeystoreError`'s distinguishability doctrine exists to prevent."* Once the collapse is
named, separating them is the fix and documenting it is not.

So the keystore is created only where no file exists, and the per-Stoa refusal comes
from `chosen_paths`' primary key — which makes the recorded decision true rather than
retracting it. `design.md` carries the whole story: the mechanism, why it read as
elegant, the install-scope-versus-Stoa-scope mismatch, and the two costs.

Two costs this uncovered, neither of which the finding could have predicted and both
now paid explicitly:

- **`encrypted` needed a second source.** Where the keystore already exists the keep
  writes nothing, so "the `Unlock` this keep used" is not the truth about the file. That
  branch reads it. An unreadable existing keystore refuses the keep rather than
  guessing.
- **The refusal needed its own error arm.** The primary key surfaces as SQLite's
  `UNIQUE constraint failed`, which `IdentityStoreError::Storage` renders as *"check the
  path and its containing directory"* — the wrong fix for the commonest refusal the
  store has. `ChoiceAlreadyRecorded` names the existing choice, matched on SQLite's
  error **code** rather than its message text. `each_keep_refusal_reason_is_pinned_to_its_own_situation`
  failed on exactly this when the refusal moved, which is what that test was for.

Tests: `keeping_an_identity_in_a_second_stoa_succeeds_and_reuses_the_master_key` goes
through the handler twice — the finding's key observation is that *"every multi-Stoa
test writes the second row with `store.record_path(...)` directly, bypassing
`keep_identity`"*, so a handler-level test was the missing thing. It also asserts the
written keystore re-derives **both** Stoas' addresses, because the old guard was
accidentally preventing a second keep from writing a new master key over the first.
`a_second_keep_for_one_stoa_is_still_refused_after_the_second_stoa_fix` sits beside it
so the fix cannot have traded a reachable second Stoa for a replaceable identity.

Measured: always calling `create` fails the first of those and nothing else in 563.

---

- [x] **2. The "one partial state" is unrecoverable through the API, and `whoAmI`'s reason tells the user to do the thing that cannot work**

**For: `dev-writer`**

**The recorded decision.** `design.md:190-199` and the Risks entry at
`design.md:271-276`:

> If the keystore write succeeds and the path record fails, the keystore exists
> with no recorded path. **That is the one partial state, and it is recoverable
> rather than silent** […] What is *not* offered is an automatic repair, because
> repairing means choosing a path on the user's behalf and the spec forbids
> coercing a selection.

"Recoverable rather than silent" and "no *automatic* repair" together read as: the
user can repair it by choosing again. **They cannot.** By finding 1's mechanism,
the next `keepIdentity` hits `create` → `AlreadyExists` and returns before
`record_path` is ever called. The state is unrecoverable through every wire method
this change adds; the only repair is deleting the keystore file by hand, which
throws away the master key.

**And the reason string actively misdirects.** `wire.rs:686-691`, the fourth row
of `design.md`'s own table:

```rust
reason: "a master key exists but no identity has been chosen for this Stoa; \
         generate a slate and keep one of its candidates"
```

`KeystoreError::Display` has a documented obligation to name the fix
(`design.md:233-236` leans on it explicitly). This message names a fix that is
guaranteed to fail. A user following it gets `AlreadyExists`, whose message is
about a keystore they did not know they had.

**Scenario.** Fresh install, `identity.sqlite`'s table dropped or the disk full at
the moment of `record_path`:

1. `keepIdentity` → keystore written, `record_path` fails → `{"kept":false,…}`.
2. `whoAmI` → `{"hasIdentity":false,"reason":"… generate a slate and keep one of
   its candidates"}`.
3. `generateIdentitySlate` → succeeds, returns five candidates.
4. `keepIdentity` → `{"kept":false,"reason":"a keystore already exists …"}`.
5. Goto 2, forever.

`wire.rs:2681` (`a_keep_whose_path_record_fails_reports_failure_and_names_no_identity`)
pins steps 1-2 and stops there; nothing pins 3-5, so the loop is invisible to the
suite.

**What `design.md` should say.** The honest statement, in place of "recoverable":
the partial state is a dead end in this change, the master key is not recoverable
from it without manual filesystem work, and the fourth row's reason string is
therefore currently misleading. Whether the fix is a repair path, a different
reason string, or moving the `create` call after `record_path` is `dev-writer`'s
call — but the *document describing what the code does* must not call this state
recoverable. This is the finding I was asked to check most closely, and the answer
is that `design.md`'s atomicity argument names the right mechanism (write order,
`create`'s refusal, `write_atomically`'s staging — all verified, see §6 below) and
then draws a conclusion about recoverability the mechanism does not support.

**Fixed** in `63133c9`, and by **none** of the three options this entry offers — which
is worth explaining, because the entry explicitly leaves the choice to `dev-writer`.

Not a repair path (it would mean choosing a path on the user's behalf, which the spec
forbids). Not a different reason string (the string names the right fix; what was wrong
was that the fix did not work). Not moving `create` after `record_path` (that produces
the state the write-order argument exists to prevent — a recorded path naming a master
key that does not exist).

**Fixing finding 1 closes this one.** A keep whose keystore already exists now reuses it
and proceeds to `record_path`, so the next choice does land. The state is recoverable in
the way the document claimed all along, and `whoAmI`'s reason is now a fix that works.

That the two findings share a mechanism is this entry's own discovery — *"By finding 1's
mechanism, the next `keepIdentity` hits `create` → `AlreadyExists` and returns before
`record_path` is ever called"* — and it is why the steps-3-to-5 loop was the more
convincing half of the evidence. A reader of finding 1 alone might have accepted a
documented one-Stoa-per-install limit; nobody would accept a documented permanent loop.

`design.md`'s entry now records the **correction rather than the conclusion**: that it
said "recoverable", that the choice could never be made, and that the mechanism it
named was right while the inference from it was not. The alternative was to quietly
rewrite the sentence into something true, which would have removed the one piece of
evidence that a careful-looking concession can be wrong.

**Not pinned by a test, and saying so.** Steps 3-5 of the loop are now
survivable, but no test walks the whole cycle — `a_keep_whose_path_record_fails_reports_failure_and_names_no_identity`
still stops at step 2, as this entry notes. What *is* pinned is the mechanism that
closes it: `keeping_an_identity_in_a_second_stoa_succeeds_and_reuses_the_master_key`
proves a keep proceeds past an existing keystore. A test for the full recovery loop
would need a `record_path` that fails once and then succeeds, which no fixture here can
express. Flagged for `tester` rather than claimed.

---

- [x] **3. The master key a slate is derived from is minted fresh on every call, and `design.md` never mentions it**

**For: `dev-writer`**

**Where it lives.** Only as a doc comment on the adapter helper,
`dialectica/rust-lib/src/lib.rs:293-301`:

> **The fresh key is deterministic across neither calls nor slates**, which is a
> real consequence worth naming: two `generateIdentitySlate` calls on a fresh
> install offer candidates of two *different* master keys […] What makes it
> harmless is that the keep mints its own key too and writes THAT one, so the
> identity kept is always one of the candidates of the key that was stored.

That is a decision with a real alternative, a real cost, and a paragraph
justifying it — CLAUDE.md's own tell ("Anything a comment justifies at length —
if it needed a paragraph, it was a decision"). It is not in `design.md`. And the
justification is not sound as written.

**The scenario.** Fresh install, no keystore on disk:

1. `generateIdentitySlate` → `master_key` mints **K1** (in memory, unwritten),
   returns slate with nonce N and five `address`/`publicKey` pairs derived from
   K1. `self.live_slate = N`.
2. The user reads the five addresses, picks index 2.
3. `keepIdentity(stoa, N, 2)` → `master_key` mints **K2** (K1 was dropped at the
   end of call 1). `slate_from_nonce(stoa, N)` recomputes over K2.
4. `derive_path` depends only on `(nonce, index)` (`onboarding.rs:159-168`), so
   the **path matches**. But `candidate_key` is
   `derive_stoa_key_at_path(master_key, stoa, path)` (`onboarding.rs:319-321`), so
   the **address and public key do not**.
5. K2 is written, and the reply reports K2's address at path 2.

The user is shown five identities and keeps a sixth. `lib.rs`'s comment says the
kept identity "is always one of the candidates of the key that was stored" — true,
and beside the point: it is not one of the candidates the user was *shown*. The
address is the only unforgeable way to tell two candidates apart
(`spec.md:98-99`), so the value the choice was made on is precisely the value that
changed.

**Why the suite does not see it.** Every core test holds the master key fixed.
`wire.rs:2970` (`a_kept_identity_is_the_candidate_the_slate_offered_at_that_index`)
is the test written for exactly this property, and it passes
`|| Ok(a_master_key())` — the fixed `[7u8; 32]` keystore — to *both* calls
(`wire.rs:2986` and the `keep_through_the_wire` helper). Its comment says "The two
replies come from two calls, so agreeing is a property of the code rather than of
one value being copied", which is true of `core` and false of the adapter, and the
adapter is where the minting happens. This is this project's documented defect
family — two explanations, one answer — reappearing at the fixture boundary rather
than inside a fixture.

**What a Decisions entry needs.** What was chosen (mint per call, write only on
keep), the constraint (a slate must be available before a keystore exists, and
`generateIdentitySlate` must write nothing — `spec.md:398-412`), the alternatives
(hold the minted key in the module struct beside `live_slate`; derive the key
deterministically from the nonce; write the keystore at slate time and accept the
spec violation), what ruled each out, and **the cost, which is currently
unstated**: on a fresh install the addresses in a slate reply are not the
addresses a keep will produce. If the answer is "hold the minted key beside the
nonce", note that the module already holds `live_slate` as a second field
(`tasks.md` 5.1), so the shape exists.

**Fixed** in `63133c9`, and the decision is now recorded — but recording it was never
going to be enough, which is the one place I part company with this entry's framing.

It is filed as "a decision not recorded", and the remedy it specifies is a Decisions
entry naming the cost. But the cost, as the entry itself states it, is that *"the user
is shown five identities and keeps a sixth"* — and that is not a cost a design document
can make acceptable by naming it. It is the defect. Correctness and security review
filed the same mechanism as a high-severity bug; the honest reading is that this entry
found the bug and, by looking at it through the "was it recorded?" lens, filed it as a
documentation gap.

So both were done. The behaviour is fixed — `OnboardingSession` holds
`(keystore, live_slate)` in `core` and mints at most once — and `design.md` carries the
entry this asks for, including the alternatives and what ruled each out.

**Of the three alternatives listed, the first was taken**, for the reason this entry
supplies: *"the module already holds `live_slate` as a second field, so the shape
exists."* That is exactly right and it is why the fix is small. The other two were
rejected in `design.md`: deriving the key from the nonce would make the nonce key
material, when the type and its comments rest on it being public randomness; writing at
slate time breaks "Generating a slate SHALL NOT write to storage", which this same
review calls the best decision in the change.

**The fixture observation is the most valuable thing here** and shaped the test
directly: *"Its comment says 'The two replies come from two calls, so agreeing is a
property of the code rather than of one value being copied', which is true of `core` and
false of the adapter, and the adapter is where the minting happens."* A comment that is
true of the layer it is written in and false of the layer that matters. The regression
test supplies **no** key at all so that the property cannot be inherited from a fixture.

---

- [x] **4. `getCapabilities` still reports the pathless identity, so two wire methods name two different addresses for one user and Stoa**

**For: `dev-writer`**

`design.md`'s Context opens with this method:

> The gap is that `keystore.rs` can store a root secret and nothing mints one, so
> `getCapabilities` is permanently `canPost:false`.

So `getCapabilities` is the motivation for the change. But `lib.rs:382` was not
touched:

```rust
core::keystore::open_from_env(&path).map(|ks| ks.stoa_address(stoa).to_hex())
```

`stoa_address` is the **two-input** derivation (`keystore.rs:712-713` →
`stoa_public_key` → `stoa_key` → `derive_stoa_key`). `whoAmI` uses
`stoa_public_key_at_path` (`wire.rs:700`). Under the salt bump — which
`design.md:58-75` argues for precisely so the two schemes cannot coincide — these
are guaranteed to differ for every path including 0
(`identity.rs:1177-1207` pins that).

**Scenario.** A user keeps path 52135370 in Stoa S. Then:

- `whoAmI` → `{"hasIdentity":true,"address":"<A_path>", …}`
- `getCapabilities` → `{"canPost":true,"identity":"<A_pathless>"}` where
  `A_pathless != A_path`.

A view rendering "you are posting as X" from the probe and "you are X" from
`whoAmI` shows two identities. Worse, `canPost:true` now asserts posting ability
for an address that has no recorded path and that nothing in the signing path will
ever use.

**Why this is a Decisions gap and not merely a bug.** There is a defensible
reading — `getCapabilities` belongs to the `posting-capability` spec, not this
one, and widening it is a separate change. That is a decision with an alternative
(update the lookup to consult the record, as `whoAmI` does) and a cost (two
methods disagree until then). `design.md` says none of it, and the omission is
conspicuous given that this method is the first sentence of the Context. Note the
new scheme's doc comment at `keystore.rs:718-727` says the path-taking trio is
"the same three hops as `Keystore::stoa_key` and its two callers" — which invites
the reader to assume the callers moved over. They did not.

**Fixed** in `63133c9`, by the alternative rather than by recording the gap — and here
the entry's own "defensible reading" is the thing I want to argue with, because it is
the reading that would have kept the defect.

The reading is that `getCapabilities` belongs to `posting-capability`, so widening it is
a separate change. That is true about *ownership* and wrong about *consequence*: the
merged `posting-capability` requirement says the reported identity SHALL be the one an op
published now would be attributed to, and after the salt bump it was not. Deferring
would not have been "two methods disagree until then" — it would have been shipping a
merged requirement violated, with the violation recorded as a deliberate cost. A spec
this change does not own is a stronger reason to fix it than a weaker one.

So `posting_identity` in `core` consults the record and derives at the recorded path,
and `design.md` carries the decision including the two consequences taken on purpose
(`getCapabilities` now depends on `IdentityStore`; the lookup's error type widened).

**The doc-comment observation was the sharpest pointer in this entry** and is fixed:
*"the path-taking trio is 'the same three hops as `Keystore::stoa_key` and its two
callers' — which invites the reader to assume the callers moved over. They did not."*
They have now. As a side effect the pathless trio has no production caller at all, which
is recorded in `design.md`'s Risks rather than resolved by deletion.

Tests: `the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa` and
`the_probe_and_whoami_give_one_reason_when_no_choice_is_recorded_for_this_stoa`. The
second covers the state this entry does not mention and which the old code got worst:
`canPost: true` for an address with no recorded path, asserting posting ability for an
identity nothing would ever sign with.

---

- [x] **5. `design.md` says "nothing was written anywhere. Clean." — an empty `identity.sqlite` is written first**

**For: `dev-writer`** (small, but it is the document describing the wrong thing)

`design.md:195`:

> - If the keystore write fails, nothing was written anywhere. Clean.

In the adapter, `IdentityStore::open` runs **before** the keystore write
(`lib.rs:438-443`, then `core::keep_identity` at `:450`), and `open` creates the
file and stamps the schema on a fresh path (`identity_store.rs:181-184` →
`from_connection` → `create_schema`, `:200-219`). So a keep that fails its
keystore write leaves a zero-row `identity.sqlite` with `user_version = 1` behind.

Harmless — `path_for` returns `None`, `whoAmI` takes the fourth row correctly, and
the suite's `a_keep_whose_keystore_write_fails_records_no_path` (`wire.rs:2121`)
asserts the right thing. But "nothing was written anywhere" is not what the code
does, and the precise claim ("no *path* was recorded anywhere, and the file that
was created carries no row") is both true and just as strong. A concession that
overstates its own cleanliness is the shape that got caught on another piece this
week.

**Fixed** in `63133c9`, using the replacement wording this entry proposes almost
verbatim, because it is right that the precise claim is *"both true and just as
strong"*. `design.md` now says no path was recorded anywhere and the file the adapter
opened on the way in carries no row, and names what is actually left behind: a zero-row
`identity.sqlite` with `user_version = 1`.

The finding is explicitly the smallest in the file and I want to record why it was worth
acting on rather than noting. Its last sentence is the reason: *"A concession that
overstates its own cleanliness is the shape that got caught on another piece this
week."* A concession is the part of a document a reader trusts most, because it is where
the author is arguing against themselves — so an overstatement there is worth more than
an overstatement in a claim. Two of the four things this reviewer found wrong in
`design.md` were inside concessions (this one and finding 2's "recoverable"), which is
not a coincidence.

The entry's own caveat — harmless, and
`a_keep_whose_keystore_write_fails_records_no_path` asserts the right thing — is
confirmed, so **no test changed** and none needed to. This is a documentation fix and is
ticked as one.

---

- [x] **6. Verified as recorded — the three claims I was asked to check, and one line-number nit**

**For: nobody. Recorded so the next reviewer does not redo it.**

- **The derivation chain is on the live path, and it is not dead code.**
  `keystore.rs:728-741` gives the `stoa_key_at_path` / `stoa_public_key_at_path` /
  `stoa_address_at_path` trio over `identity::derive_stoa_key_at_path`
  (`identity.rs:424-425`), and `wire.rs:700` (`whoami_for`) plus
  `onboarding.rs:320` (`candidate_key`) are live callers. The pre-existing
  `derive_stoa_key` is untouched and its pinned value `b62b6b59…` still asserts
  (`identity.rs:725-731`). **The nit:** `design.md:9-14` cites
  `keystore.rs:651 → :645 → :640 → :641`; the functions are actually at
  `:701 → :706 → :712` (and `lib.rs:251` is now `:382`). The chain is real, the
  line numbers are from an earlier revision. Cite by symbol name instead — line
  numbers in a `design.md` rot within the same change.
- **The `log/sqlite.rs` argument for a separate file is accurate.** `LAYOUT_VERSION`
  in `PRAGMA user_version` (`log/sqlite.rs:74`), `check_layout` proving the
  declared columns (`:298-330`, with `:274-292`'s "THE VERSION IS A CLAIM"
  comment), and `create_schema` stating there is no migration path by design
  (`:405-411`, `:591`) all say what `design.md:16-21` says they say. The split is
  argued on a constraint the existing code states about itself, as claimed.
- **All three pinned constants carry their provenance, including the v1
  validation.** `identity.rs:733-762` records the `openssl kdf` invocation and
  that it "was FIRST validated by reproducing the version-1 value above exactly";
  the third pin at `0x0100_0000` is at `identity.rs:1267-1304` with the same
  validation restated and the reason given (paths 0 and 1 differ in the last byte
  alone). `onboarding.rs:413-440` does the same for the two `derive_path` values
  via `openssl dgst`, with both digests' top bits noted as set so a no-op mask
  cannot hide. This is the part of the change in the best shape.
- **No passphrase constant exists.** `grep` for a source-level passphrase finds
  nothing; `lib.rs:449` calls `core::keystore::protection_from_env()` and
  `design.md:248-256` records that the spec's refusal is honoured "by there being
  no constant to find". True.
- **The slate-as-nonce decision is implemented as described.** `onboarding.rs:243-294`
  is the reproducing walk, `wire.rs:509-513` is the single nonce comparison that
  collapses superseded and never-existed into one path, and
  `OnboardingError::NonceIsNotTheLiveSlate` (`onboarding.rs:337-343`) documents
  the collapse. Both `SLATE_SIZE = 5` (PLAN.md §5.2.1, cited at
  `onboarding.rs:52-54` and pinned at `onboarding.rs:439`) and the 2^31 mask
  (`onboarding.rs:137-151` and `design.md:158-163`) are recorded with their
  reasons, so the two items my brief flagged as possibly-missing are in fact
  present.
- **The `NO SPEC:` markers are two, not three**, and both are in test comments
  (`identity_store.rs:700`, `:746`). The third marker my brief expected is at
  `wire.rs:1348` and is about `Fn` vs `FnOnce`, unrelated. See finding 7.

**Actioned — the line-number nit is fixed; the verifications needed nothing.** Ticked
because the one item addressed to anyone has been dealt with, and recorded here because
this entry saved real time.

The nit: `design.md`'s `keystore.rs:651/:645/:640/:641` and `lib.rs:251` are replaced by
the symbol chain, which is what this entry recommends (*"line numbers in a `design.md`
rot within the same change"*). Architecture's A9 found the same thing independently; the
`proposal.md:217-220` copy is untouched because the proposal is `spec-writer`'s.

The four verified claims I did **not** re-derive, on the strength of this entry saying
they were walked rather than inferred — the `log/sqlite.rs` no-migration argument, the
three pinned constants with their `openssl` provenance including the validate-against-v1
step, the absent passphrase constant, and the slate-as-nonce implementation. The
spec-test reviewer independently reproduced the pinned constant with `openssl kdf`, which
is two agents agreeing from different directions, so I spent my time on the four defects
instead. Recording that as a deliberate choice rather than an omission.

One item here **became false during the fix pass** and is worth flagging rather than
leaving for someone to trip over: the marker count is now **three**, not two. `path`'s
appearance in three replies gained a `NO SPEC:` marker on
`the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against`, per readability's
R4. So a later reader grepping `NO SPEC` and comparing against this entry will find a
discrepancy that is a fix rather than a drift.

---

- [x] **7. The `NO SPEC:` refusals are durable reasoning living only in test comments**

**For: `dev-writer`**

Two refusals the spec does not require, chosen by the dev, justified only in a
`#[cfg(test)]` comment:

- `identity_store.rs:700` — a stored path outside `u32` is refused, not clamped.
- `identity_store.rs:746` — a malformed stored Stoa key is refused.

The reasoning is good and it is *not* only in the test: the error variants carry
it too (`IdentityStoreError::PathOutOfRange` at `:83-89` — "Refused rather than
clamped: coercing would name an identity the user never chose, which is the one
outcome the spec calls unrecoverable" — and `StoaNotAnAddress` at `:90-91`, plus
`all_paths`'s comment at `:391-393`). So this is the mildest of the findings.

But `.claude/agents/README.md` is explicit that `findings/` is deleted before
merge and durable reasoning moves to `design.md` first, and a `NO SPEC:` marker is
by definition a choice the contract does not cover — the exact thing Decisions
exists to hold. Both refusals turn on the same principle (coercing disk content
names an identity nobody chose), and stating it once in Decisions is stronger than
stating it twice in doc comments, because a later reader reaching for
`INSERT OR REPLACE` or a `try_into().unwrap()` reads the archive, not the variant
docs. `tasks.md` 2.2 records that both refusals were mutation-verified, so the
evidence exists; it just does not live where the flow says it should.

**Fixed** in `d8f9816`: `design.md` has a Decisions entry, "Disk content is refused,
never coerced — one principle, three applications", stating the principle once with a
table of where it applies.

**Three, not two, and that is the fix improving on the finding rather than just
following it.** The entry's own argument is that *"Both refusals turn on the same
principle … and stating it once in Decisions is stronger than stating it twice in doc
comments."* Acting on that surfaced a third application arriving in the same fix pass:
`ChoiceAlreadyRecorded` (from finding 1) is the same principle — a replaced choice
strands every op the previous identity signed — and it would otherwise have been a
fourth doc comment restating it. Which is exactly what the finding predicts happens.

The table is the useful form: a stored path outside the writable range, a stored Stoa key
that is not 32 bytes, and a second choice for a Stoa that has one. Each hands the user
something that *works* and is not theirs, with no error anywhere.

**The `NO SPEC:` markers stay in the tests**, deliberately, and this is where I read the
finding slightly differently from how it is written. It treats the markers and the
reasoning as the same thing moving to one place. They are two claims addressed to two
readers: the marker says *the spec is silent here*, which is `spec-writer`'s to act on
and is what a reviewer greps for; the Decisions entry says *and this is why we chose
refusal*, which is the archive's. Moving the reasoning does not make the silence go
away. Both now exist.

This is fairly called the mildest finding in the file, and the entry says so. It was
worth acting on because the prediction embedded in it came true inside the same pass.

---

- [x] **8. PLAN.md §5.2.1 was not shed, and it now contradicts the code in two places**

**For: `dev-writer`**

`git diff origin/main -- docs/PLAN.md` is **empty**. `docs/UI-BRIEF.md` got its
two new obligations (7 and 8, `+51` lines — that half of the brief obligation was
met properly). PLAN.md got nothing, and §5.2.1 is the section this change
implements.

Read PLAN.md from `origin/main` per the flow's own rule. Two paragraphs are now
wrong rather than merely unshed:

**`docs/PLAN.md:1386-1395`** — *"Regeneration discards keys, and the user cannot
see it. Each refresh mints five keypairs and keeps at most one […] Whether the
keystore writes on every refresh or only on selection is an implementation
question with no user-visible consequence, and is deliberately not decided
here."*

Both halves are now false. A refresh mints **no keypairs** — it derives five paths
over one master key, and `proposal.md:63-74` records that the five-independent-roots
shape was considered and rejected. And the write question **was decided**: nothing
writes on refresh, structurally, because `generate_identity_slate` has no store
parameter to write to (`wire.rs:297-301`, and `design.md`'s Goals lean on exactly
that). A reader hitting this paragraph designs against a model the code abandoned,
which is the §4.3 failure `.claude/agents/README.md:278` says has already happened
here once.

**`docs/PLAN.md:1369-1384`** — *"The picker still discards and redraws a
duplicate […] a presentation rule with no protocol consequence whatsoever."*
Duplicate handling is now in **core**, not the picker, and it is an index walk
rather than a redraw (`onboarding.rs:243-294`). `onboarding.rs:225-242` and
`design.md:171-179` both argue *against* the redraw this paragraph assigns to the
UI, on the ground that a fresh nonce would destroy reproducibility. So PLAN.md
tells a UI author to implement the thing the core deliberately does not do.

Per `.claude/agents/README.md:41-52`, the behaviour should be struck through in
PLAN.md with a one-line summary that the thing exists (as §5.6 already does for
the keystore — `docs/PLAN.md:2090-2094` is the model), and the reasoning should
move into `design.md` under Decisions and be **removed** from PLAN.md. Note that
some of §5.2.1's reasoning has *already* been duplicated rather than moved:
`design.md`/`onboarding.rs` now argue the one-master-key model at length while
PLAN.md still argues the five-keypairs model. That is the two-copies-drift failure
the rule exists to prevent, and here the wrong copy is the one a reader reaches
first.

Also worth checking while in there: `docs/PLAN.md:3954-3962`'s open question
*"Should every root identity be derived?"* says "only the per-Stoa key is derived
from it (§5.2, `derive_stoa_key`)". Still true of the root, but the sentence now
describes a scheme with two variants and names only one.

**Fixed.** All three places, in the prescribed shape: struck through, with a one-line
summary that the thing exists and a pointer to the spec, and the reasoning removed
rather than duplicated.

- **The regeneration paragraph.** Both halves were false and both are struck. A refresh
  mints no keypairs — the five candidates are paths over one master key — and the write
  question was decided, structurally, by the slate handler having no store parameter.
  One sentence is kept because it is the reason the shape is safe rather than a
  description of it: *an identity becomes real when it signs, and nothing signs during
  onboarding.*
- **The duplicate-redraw paragraph.** Struck, with the correction that duplicate
  handling is in **core** and is an index walk rather than a redraw, because a fresh
  nonce would destroy reproducibility. The distinction this section draws against the
  cross-identity collision is kept — it is still true and is the reason the two cases
  must not be conflated.
- **The `#### Grinding` analysis is left alone**, per this entry's own note that whether
  reasoning survives in PLAN.md is a judgement call. It analyses an attack rather than
  describing built behaviour, and nothing in this change answers it.
- **The open question** now says two schemes exist, names both, and says which one a
  kept identity uses. It also lost a clause that had become false — "§9.2's MVP does not
  call even that" — which I would not have looked at if this entry had not sent me
  there.

**The observation that made this worth doing properly** is the one about duplication
rather than staleness: *"some of §5.2.1's reasoning has already been duplicated rather
than moved: `design.md`/`onboarding.rs` now argue the one-master-key model at length
while PLAN.md still argues the five-keypairs model. That is the two-copies-drift failure
the rule exists to prevent, and here the wrong copy is the one a reader reaches first."*
Two copies is worse than one stale copy, because a reader who finds the plan first has no
signal that a better one exists. That is why the reasoning is deleted rather than
annotated.

---

- [x] **9. `tasks.md` claims the code disproves**

**For: `dev-writer`**

- **6.1 says "546 passed, 0 failed".** I measured **553 passed, 0 failed** with
  `cargo test --manifest-path …/dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`
  at `2f3fccf`. Two commits landed after the tick (`b88ca79`, `2f3fccf`), which is
  the honest explanation — but a hardcoded count in a tracker is exactly what
  CLAUDE.md's "do not write down anything a command can answer" forbids. Name the
  command, not the number.
- **3.1 says `SLATE_SIZE = 5` … "verified by watching both the pinned constants
  test and `every_derived_path_is_below_two_to_the_thirty_one` FAIL with the mask
  removed."** Accurate as far as it goes, and I confirmed both pinned digests have
  the top bit set (`onboarding.rs:427-433`), so the claim that a no-op mask cannot
  hide holds.
- **3.5 (the `Zeroizing` stack local) is correctly recorded as deliberately
  untested**, with the keystore precedent cited. Not a gap; flagged here only so
  the next reviewer does not re-raise it.

I did not find other `tasks.md` claims the code contradicts. 4.3's account of the
write-order gap ("Reversing the two writes left the entire suite green — measured,
not assumed") is corroborated by `wire.rs:2121-2176` existing and asserting the
right thing, and 2.2's two mutation results are consistent with
`record_path`'s `INSERT`-without-`OR REPLACE` (`identity_store.rs:314-332`) and
`path_from_row`'s refusal.

**Fixed** — 6.1 no longer carries a count. Readability's R7 filed the same thing from the
rule's side; this entry supplied the measurement (553 against the recorded 546), and the
two together are what made the answer obvious.

Replaced rather than corrected, which is the point. Updating 546 to 563 would have been
wrong within the hour: this fix pass changed the number three times. 6.1 now names the
command and says the tests this change adds are the ones listed against the tasks above
— tied to a diff someone can check rather than to a total nobody can. This entry's
framing is exact: *"Two commits landed after the tick, which is the honest explanation —
but a hardcoded count in a tracker is exactly what CLAUDE.md's 'do not write down
anything a command can answer' forbids."*

6.2's `cargo fmt --check` tick was the same class of problem one step further on — true,
and measuring nothing, because that gate cannot reach `dialectica-core`. It now says so
and records the per-file `rustfmt --check` that was run instead.

Two items here needed **no** action and the entry is right that they do not:

- **3.1's mask verification** holds. Both pinned digests have the top bit set, so a no-op
  mask cannot hide, which this entry independently confirmed.
- **3.5 is correctly recorded as deliberately untested.** A stack local after its
  function returns is unobservable from a test. Flagged here *"only so the next reviewer
  does not re-raise it"*, and it worked — I did not re-raise it, and S5's
  residual-`seed` finding one layer down is a different claim which is recorded in
  `design.md` rather than tested.

The verification of 4.3 and 2.2 saved re-deriving both; noted as time this entry bought.

---

- [x] **10. Two thin entries (suggestions only)**

**For: `dev-writer`**

- **"Recovery-needs-the-record is a static fact this build states"**
  (`design.md:258-267`) names what was chosen (a `bool` field on `whoAmI`), the
  constraint, and one alternative (a separate method) with what ruled it out.
  **What it costs is not stated.** The cost is real: a field hardcoded `true`
  (`wire.rs:707`) cannot fail when backup lands and someone forgets to flip it —
  no test can distinguish "correctly true" from "stuck true", because there is no
  reachable state where it is false. Worth one sentence, because it forecloses
  detecting its own staleness.
- **"The protection report is derived from what was written, not re-read"**
  (`design.md:238-246`) has a genuinely good alternative-and-rejection (re-reading
  would report the protection of whatever is at the path *now*). But
  `tasks.md` 4.4 says the report was "verified […] against the FILE via
  `Keystore::is_encrypted` rather than only against the argument" — i.e. the test
  does the re-read the decision rejects. That is fine and even clever, but the
  entry should say so: the decision is about what the *reply* is derived from, and
  the test's independent re-read is what keeps the derived value honest. As
  written, entry and test look like they disagree.

**Fixed**, both, and the second turned out to be load-bearing rather than a polish
suggestion.

**The recovery-needs-the-record entry** now states the cost in the terms this finding
supplies, which are sharper than anything I would have written: the field is hardcoded
`true`, *"no test can distinguish 'correctly true' from 'stuck true', because there is no
reachable state where it is false"*, so it **forecloses detecting its own staleness**.
That is a specific failure mode rather than a general caveat — the change that implements
backup will not be reminded by anything — and the entry now says what carries it instead
(UI-BRIEF obligation 7, which is a person, not a gate).

**The protection-report entry** now draws the distinction this finding asks for: the
decision is about what the *reply* derives from, and the test's independent re-read is
what keeps that derived value honest, because asserting the reply against the same
`Unlock` it was computed from would be the reply agreeing with itself.

Writing that paragraph is what made the second-keep fix's `encrypted` problem visible.
Having just articulated *why* "the value this code used" is the authority, I noticed that
a keep whose keystore already exists **uses no value** — it writes nothing — so that
justification does not reach it, and reporting the `Unlock` there would have been a
straightforward lie in a field a view renders as a padlock. That branch now reads the
file. The entry records both sources and why they differ.

So a finding filed as "suggestions only" prevented a defect in a different fix. Worth
recording, because the temptation with a thin-entry finding is to reword the document and
move on.

---

## Summary

| # | For | Kind |
|---|---|---|
| 1 | `dev-writer` | **Code contradicts a recorded decision** — primary key cannot discharge the per-Stoa scenario; `keepIdentity` is one-Stoa-per-install |
| 2 | `dev-writer` | **Code contradicts a recorded decision** — the "one partial state" is unrecoverable, and `whoAmI`'s reason names a fix that cannot work |
| 3 | `dev-writer` | **Decision not recorded** — per-call master-key minting; slate addresses are not the addresses a keep produces |
| 4 | `dev-writer` | **Decision not recorded** — `getCapabilities` left on the pathless scheme; two methods report two addresses |
| 5 | `dev-writer` | **Document describes the wrong thing** — "nothing was written anywhere" vs. an empty `identity.sqlite` |
| 6 | — | Verified as recorded; one stale line-number citation to fix |
| 7 | `dev-writer` | **Decision not recorded** — the two `NO SPEC:` refusals belong in Decisions |
| 8 | `dev-writer` | **PLAN.md not shed** — §5.2.1 contradicts the code in two places; reasoning duplicated |
| 9 | `dev-writer` | `tasks.md` 6.1's hardcoded test count is stale (546 recorded, 553 measured) |
| 10 | `dev-writer` | Two thin entries — missing cost, and an entry that reads as disagreeing with its test |

Findings 1-4 all sit on the adapter path, which `cargo test` cannot reach
(`tasks.md` 5.2 says so explicitly and gives it as the reason the bodies are kept
thin). The bodies are thin; the *decisions* in them are not. That is the
structural observation worth carrying out of this review: `design.md` documents
`dialectica-core` thoroughly and `dialectica/rust-lib/src/lib.rs` not at all, and
the untested half is exactly the half where an undocumented choice cannot be
caught by anything else.

---

### Outcomes — `dev-writer`

All ten actioned; each entry above carries its own detail. **Every one is ticked**, so
this file blocks nothing.

| # | Outcome |
|---|---|
| 1 | **Fixed** `63133c9` — refusal moved to the primary key; a second Stoa is keepable and two tests constrain each other |
| 2 | **Fixed** `63133c9` — closed by fixing 1; the state is now recoverable as the document always claimed |
| 3 | **Fixed** `63133c9` — behaviour *and* the Decisions entry; filed as a documentation gap, but the cost it names is the defect |
| 4 | **Fixed** `63133c9` — `posting_identity` in `core`; the "defensible reading" that would have deferred it is argued against in the entry |
| 5 | **Fixed** `63133c9` — the wording this entry proposes, near-verbatim |
| 6 | **Actioned** — line numbers replaced by symbol chains; four verified claims not re-derived, deliberately |
| 7 | **Fixed** `d8f9816` — one Decisions entry, and acting on it surfaced a **third** application of the principle |
| 8 | **Fixed** — all three places shed in §5.6's shape; the grinding analysis deliberately kept |
| 9 | **Fixed** — 6.1 names the command; 6.2 now says what `cargo fmt --check` cannot see |
| 10 | **Fixed** — and the protection-report half prevented a defect in the finding-1 fix |

**The closing paragraph was the most useful thing in this review**, and it is worth
saying why rather than just agreeing. Its claim is structural: the four serious findings
are on the adapter path *because* that is the path no test reaches, so an undocumented
choice there cannot be caught by anything else. That predicts where to look, and it was
right — correctness, security and architecture each found the same defect from their own
direction, all four in `lib.rs`.

The fix follows the prediction rather than the individual findings: every decision named
in 1-4 **moved into `core`**, which is why they are fixed rather than documented.
`design.md` still documents `dialectica-core` more thoroughly than the adapter, and that
is now much less dangerous, because the adapter holds three "where is the file" helpers
and no decisions. What remains structurally true and unfixed: the adapter is still
`#[cfg(logos_scaffold)]`, and CI's test-count gate is still blind to a whole `impl` being
compiled out (architecture A2 notes this). Nothing here changes that.
