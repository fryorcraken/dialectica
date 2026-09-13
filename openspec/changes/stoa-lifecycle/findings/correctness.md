# Correctness findings — stoa-lifecycle

## Provenance, and why this file is a reconstruction

**These findings were relayed to the fixer through the runner's brief rather than
written to a file, because this piece's review ran before
`.claude/agents/README.md` gained the findings-file rule.** They are recorded here
after the fact so they survive the PR thread they arrived in.

That is the exact failure the rule exists to prevent, and it cost something here:
the measurements below are the reviewer's, restated by the fixer, and the
reviewer's own wording and its full evidence are **not recoverable** — the agent
is gone. Where a measurement is the reviewer's rather than one the fixer
re-derived, it says so. **Treat a reviewer measurement in this file as
second-hand**; the re-derivations are first-hand.

Entries 1–4 are the reviewer's. Entry 5 is the security note the same reviewer
raised, kept here because it is the evidence that decided the shape of the
creator-key fix; the independent security review of it is in `security.md`.

**Every `file:line` below is the position when the finding was made**, i.e. before
the fixes. The code has moved since. Lines outside this piece's own files
(`log/sqlite.rs:74`, `moderation.rs:141`/`:174`/`:191`) were re-verified against
the current tree and still hold; the `membership.rs` and `wire.rs` numbers are
historical and should be located by symbol, not by line.

---

- [x] **1. `MembershipStore::list` answers "you are in no Stoa" for a valid page 0**

**For:** `dev-writer`

`limit = per_page.saturating_add(1)` was then converted with `i64::try_from`, and
a failed conversion returned `items: []`, `has_more: false`. The guard therefore
tested the **incremented** limit, so the failure window is a `per_page` the
un-incremented limit would have served fine.

**Failure scenario — reviewer's measurement, on a store holding 3 Stoas:**

```
list(0, i64::MAX as usize)     => items=0  has_more=false   // WRONG
list(0, (i64::MAX as usize)-1) => items=3  has_more=false   // right
```

The wrong answer is **indistinguishable from an empty store** and it returns
`Ok`, so the obligation `list_stoas` states in its own `Err` arm — *"a storage
failure is the error shape and NEVER an empty listing"* — is broken one layer
down, silently.

Wire-unreachable today (`feed::clamp_per_page` caps at 100), but `list` is `pub`
on a `pub` module and the crate is the deliverable.

`membership.rs:459-486`. The doc comment at `:471-476` justified the conversion
for an unreachable **page** and was applied to an over-large **per_page**,
conflating the two.

**Outcome: FIXED** in `a08abdf`. `per_page` now saturates at `i64::MAX` (more rows
than exist is an answerable request); only `page` refuses (an offset past
`i64::MAX` would wrap negative, and SQLite treats a negative `OFFSET` as none at
all, serving the FIRST page to a caller who asked for an impossible one).
Fixer re-derived the measurement before fixing: the test
`membership.rs::a_per_page_at_the_conversion_boundary_still_lists_the_whole_store`
was written first and failed with `left: 0, right: 3` on the named assertion.
Reasoning recorded in `design.md` under the listing decision.

---

- [x] **2. `per_page == 0` gives a page both empty and not-the-last, so paging never terminates**

**For:** `dev-writer`

`limit = 0 + 1 = 1` fetched one row, `has_more = 1 > 0` was `true`, and
`items.truncate(0)` then emptied the page.

**Failure scenario — reviewer's measurement, on a store holding 3:**

```
list(0, 0) => items=0 has_more=true
list(5, 0) => items=0 has_more=true
```

The reviewer ran this module's own `every_stoa` test helper against
`per_page = 0`: **51 pages, 0 items, still `has_more`** — it escapes only via its
`assert!(page < 1000)` guard, which would have blamed the fixture rather than the
code. There is also an internal disagreement: on a store holding 1, `len()` is
`1` and `is_empty()` is `false`, while `list(0, 0)` reports empty-with-more.

`membership.rs:459` and `:507-508`.

`feed.rs` is **structurally immune** to the same bug (it slices with
`min(rows.len())` rather than looking one past), so this is specific to the
`LIMIT per_page+1` plus `truncate` shape and not a shared defect.

The reviewer noted that `has_more` and the `truncate` are two facts about one
boundary that can disagree, and asked for a reshape rather than a third guard,
citing CLAUDE.md.

**Outcome: FIXED** in `a08abdf`. Two parts, because the reshape alone does not
cover zero: (a) an early return states once that a page of zero rows is the last
page — `has_more` means "paging further reaches a Stoa this page did not show",
and at `per_page == 0` every later page is also empty; (b) the remaining boundary
is a single `split_off`, so what is kept is the page and what comes off is the
evidence, and no arrangement of the two can contradict the other.
Test: `membership.rs::a_per_page_of_zero_terminates_rather_than_paging_forever`,
written first and failed on the `has_more` assertion.

The zero case is a **guard with an early return rather than arithmetic**, and that
is deliberate: arithmetic that also handles zero is arithmetic whose zero case
nobody can read, and the zero page is a different question from where a page
boundary falls. Carries a `NO SPEC:` marker — the spec does not say what a
`per_page` of zero lists, and the wire never produces one. Recorded in
`design.md`.

---

- [x] **3. An encode failure is reported as a decode failure**

**For:** `dev-writer`

`MembershipStore::join` called `canonical_bytes()` — the **encode** side — and
mapped its `GenesisError` to `MembershipError::UndecodableRecord`, whose `Display`
says *"the genesis record could not be read"*.

**Failure scenario:** a caller passing a 2000-byte title receives
`"the genesis record could not be read: title is 2000 bytes, the maximum is
1024"`. Nothing was read. This reaches `{"error":"..."}`, so the sentence is the
whole of what a user learns.

The variant's own doc distinguished itself from `RecordDoesNotMatchAddress` as
"material that does not decode", and the only way to reach it is material that
does not **encode**.

`membership.rs:141` (the variant), `:373-375` (the construction site).

Reachable only from a direct `MembershipStore::join` caller, which is the tested
API.

**Outcome: FIXED** in `a08abdf`. Renamed to `UnencodableRecord` and the message is
now *"that genesis record cannot be encoded, so it names no Stoa: {e}"*.

Fixer's addition to the finding, first-hand: `grep` for every construction site
found **exactly one**, on the encode side — so there was no decode-side sibling to
add, and the right fix was a rename rather than a second variant. The variant doc
now records why there is no decode sibling: a membership is joined from a `Genesis`
this crate already decoded (`wire.rs` owns that decode and reports it), and a row
read back that does not decode is `CorruptEntry`, which points at the file rather
than the caller.

Test: `membership.rs::an_encode_failure_does_not_report_itself_as_a_failure_to_read`,
asserting on the **rendered string** because that is the contract — asserting only
on the variant would have passed the wrong wording.

---

- [x] **4. `MembershipStore::is_empty` is unprotected**

**For:** `tester`

**Reviewer's measurement:** `cargo mutants` on `membership.rs` — 28 mutants, 18
caught, 9 unviable, **1 missed**: replacing `is_empty` with `Ok(true)` survives
all 544 tests. The only assertion on it was against a **fresh** store
(`membership.rs:719`), i.e. the `true` case; nothing asserted `false`.

Its docstring says a first-run view genuinely asks this, so a permanent `true`
renders every user as having joined nothing, with no gate seeing it.

`membership.rs:538-540`.

**Reviewer's note, which the fixer confirms matters:** `cargo mutants` generates
**no** mutant for `saturating_add`, `saturating_mul` or `try_from`, which is
exactly why entries 1 and 2 were invisible to it. **Do not treat a clean mutants
run as covering them.**

**Outcome: FIXED** in `a08abdf`. Test
`membership.rs::an_empty_store_and_a_populated_one_disagree_about_being_empty`
asserts both cases and that `is_empty` agrees with `len` — the drift the two exist
as a pair to avoid, and the shape the mutation produced.

Fixer re-ran the measurement first-hand after the fix: **26 mutants, 18 caught,
8 unviable, 0 missed** (down from 28 total because the reshaped `list` presents
fewer mutable expressions). The survivor is gone.

---

- [x] **5. `getCapabilities` reaches the creator identity through a caller-named address**

**For:** `dev-writer`

**Reviewer's measurement:**

```
creator ADDRESS         = 1584e675…b638999a
getCapabilities(00..00) = 1584e675…b638999a
```

`getCapabilities` took a caller-supplied hex string, decoded it with no
plausibility check, and handed it to `Keystore::stoa_address`
(`rust-lib/src/lib.rs:338`) — so `{"stoa":"<64 zeros>"}` returned **exactly** the
creator key's address, because `CREATOR_KEY_DOMAIN` was `[0u8; 32]`.

So `keystore.rs:284-291`'s justification for that constant — *"no record anyone
can construct hashes to this value; finding one would be a preimage attack"* — is
**true for a record and irrelevant here**. Nothing constrained the `stoa`
argument to be an address derived from any record. The reviewer's framing, which
is the durable lesson: **the gap is not a preimage, it is an unconstrained
argument.** And `the_creator_key_is_not_any_real_stoas_identity` establishes less
than its name claims.

Bounded at the time — `getCapabilities` is a local read-only probe and the caller
learns its own peer's creator address; `grep stoa_key(` found no production
signing path taking a caller-supplied address. **But the moment one lands, a
caller passing 64 zeros signs under the creator/moderator key.**

**Outcome: FIXED** in `4313cf6`, and the reviewer's requirement — *"it must not be
reachable by a caller naming an address"* — is met **by construction**:
`Keystore::identity_key` takes no address, so there is no argument left to
choose. `CREATOR_KEY_DOMAIN` and its now-wrong preimage justification are
**deleted, not retargeted**, as the reviewer asked. The three tests that rested on
the domain went with it.

Test: `keystore.rs::no_stoa_address_a_caller_can_name_reaches_the_identity_key`
asserts that for every context a caller could name — including the old all-zero
domain — the per-Stoa derivation stays a different key from the one a creation and
the probe use.

**This is verified independently in `security.md`** rather than resting on the
fixer's own account of its own fix, which is the right division and was asked for.

**Addendum, from the later review pass** — `security.md` entry 3 and
`architecture.md` entry 1 both re-derived this and confirmed the property holds, then
found that *nothing could see it*: the two closures naming the key sat in
`cfg(logos_scaffold)` code that `cargo test` never compiles, no CI job read, and
`cargo mutants` reported 6/6 unviable on. Reverting `lib.rs:346` to
`ks.stoa_address(_stoa)` restored the original bug with every gate green.

So this entry's fix is now held by something rather than only true:
`core::keystore::creator_and_poster_in` derives both halves in one expression, and
`wire.rs::the_creator_a_creation_names_is_the_identity_the_probe_reports` drives both
wire handlers through it — 550 pass, 1 fails under the revert, where before the whole
suite stayed green. A Lint step covers the part no test can reach.

Recorded here because a reader arriving at this entry should not conclude the matter
closed at `4313cf6`; it closed the defect, and a later pass closed the gate.

---

## Not a finding, but the reason entry 5 mattered more than it looked

The same fix closes a separate defect the correctness reviewer did not raise and
the design reviewer did: the creator key was not the key the peer posts with, so
`Moderators::of(genesis).contains(posting_key)` was **false for a Stoa's own
creator**. See `design-review.md` entry 1. Both defects had one cause — a
synthetic derivation domain that no document argued for — and one fix.

---

## Checked and clean — reviewer's, recorded so nobody re-derives them

Reported as verified and requiring no change. **Second-hand**; the fixer did not
re-derive these.

- **`SqliteOpLog` untouched** — verified against both `main` and the pre-fix
  branch. The design's load-bearing claim holds.
- **A refused join leaves no partial state**, held across a restart, asserted for
  both the claimed address and the record's own.
- **Creation reuses `join`'s single write path**, with no second path.
- **`Address::from_bytes` as `const` is behaviour-neutral.**
- **The derivation arithmetic is correct** and the pinned constant is the one the
  code uses.
- **Wire pagination is correct** — `as_u64` refuses negative and fractional values
  before any cast, so the negative-`OFFSET` bug cannot recur through JSON. (Note
  this is what confines entries 1 and 2 to the crate API.)
- **The `storage_dir` guard has no dropped call site** — all five storage-touching
  handlers call it, and `persistence_path` is read only inside it.
- **Idempotence: the single `assert_eq!` is adequate for `OR REPLACE`
  specifically**, because the **primary key** — not the statement verb — is what
  makes it structural, and `membership.rs`'s own comment says so accurately. The
  reviewer explicitly **disagreed** with a stronger framing put to it and declined
  to call the "structural" claim wrong. That disagreement is recorded because it
  changed what the fixer did: `design.md` now records where the single witness is
  and that the wire hides the distinction, and does **not** rewrite the structural
  claim. See `design-review.md` entry 3.
