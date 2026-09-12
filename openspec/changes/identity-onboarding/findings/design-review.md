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

## 1. `keepIdentity` can never record a second Stoa, which makes a recorded decision false

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

---

## 2. The "one partial state" is unrecoverable through the API, and `whoAmI`'s reason tells the user to do the thing that cannot work

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

---

## 3. The master key a slate is derived from is minted fresh on every call, and `design.md` never mentions it

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

---

## 4. `getCapabilities` still reports the pathless identity, so two wire methods name two different addresses for one user and Stoa

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

---

## 5. `design.md` says "nothing was written anywhere. Clean." — an empty `identity.sqlite` is written first

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

---

## 6. Verified as recorded — the three claims I was asked to check, and one line-number nit

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

---

## 7. The `NO SPEC:` refusals are durable reasoning living only in test comments

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

---

## 8. PLAN.md §5.2.1 was not shed, and it now contradicts the code in two places

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

---

## 9. `tasks.md` claims the code disproves

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

---

## 10. Two thin entries (suggestions only)

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
