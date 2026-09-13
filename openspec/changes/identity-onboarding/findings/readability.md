# Readability — `identity-onboarding`

Scope: **readability only**. Correctness, security and architecture are other
reviewers' dimensions and are not assessed here, even where a finding below
touches code that has security reasoning attached to it.

Reviewed at `2f3fccf` in `.claude/worktrees/rev-id-readable`. Files read in
full: `onboarding.rs`, `identity_store.rs`, `wire.rs`, the `keystore.rs` and
`identity.rs` diffs, `dialectica/rust-lib/src/lib.rs`, `design.md`, `tasks.md`,
the spec delta, and the `docs/UI-BRIEF.md` diff.

Judged against CLAUDE.md's four readability standards: one function one job;
comments must not claim more than the code does; do not write down what a
command can answer; one concept one name. Plus the `NO SPEC:` marker
convention from `.claude/agents/README.md`.

**No code was mutated.** Formatting was checked with `rustfmt --check
--config skip_children=true` per file, which writes nothing. The tree is clean
apart from this file.

---

## Genuine defects

- [x] **R1. `design.md` says `identity.sqlite` has no `check_layout` equivalent. It has one.**

**For:** `dev-writer`

`openspec/changes/identity-onboarding/design.md:295-301` carries this as a
Risk / Trade-off:

> **`identity.sqlite` has no `check_layout` equivalent** → The op log's version
> check proves its declared layout against the columns it reads, and this store
> does not. […] Recorded as a known asymmetry rather than copied, because
> copying it would be copying 40 lines of machinery for a table with two
> columns.

The code does the opposite. `identity_store.rs:238` **is**
`fn check_layout(conn: &Connection)`, it is called from `from_connection` at
`identity_store.rs:215`, it names **both** columns
(`SELECT stoa, path FROM chosen_paths LIMIT 0`, line 239) for the reason its own
doc comment gives at lines 230-233, and it refuses at *open* rather than "as a
named error on the first read" as the Risk claims. Two tests exist for exactly
the behaviour the Risk says is absent:
`a_version_stamped_over_a_missing_table_is_refused_rather_than_read`
(`identity_store.rs:600`) and `a_table_missing_a_column_is_refused_rather_than_read`
(`identity_store.rs:621`).

**Concrete illustration.** A reader who consults `design.md` before touching
this store — which CLAUDE.md instructs them to do — concludes the layout guard
was deliberately declined, and either adds it a second time or reasons about
the store's failure modes on a premise the code contradicts. This is the
"comment claiming more than the code does" family inverted: the document
claims *less*, which is just as misleading and harder to catch, because
nothing fails.

This is not a stale-line-number nit. It is a recorded decision that was
reversed during implementation and the reversal was never carried back —
exactly what `.claude/agents/README.md` means by durable reasoning drifting
from the code it explains. The Risk entry should either be deleted or rewritten
to record that the guard **was** copied and why the 40-lines objection was
dropped.

**Fixed** in `d8f9816`, rewritten rather than deleted — and the rewrite is worth more
than either option the finding offers, because the residual risk turned out to be
real.

The entry now states what `check_layout` **does** prove (two column names, via
`LIMIT 0`) and what it does not (anything about keys or constraints), which is the
genuinely asymmetric part. Security review's S4 measured the consequence: a replaced
file whose `chosen_paths` lacks the `PRIMARY KEY` opens `Ok`, and `path_for` then
returns whichever of two rows SQLite hands back first — an identity chosen by
physical row order. So deleting the entry would have removed a true warning along
with a false one.

The observation this finding makes that I want to keep on the record, because it
generalises: *"This is the 'comment claiming more than the code does' family
inverted: the document claims **less**, which is just as misleading and harder to
catch, because nothing fails."* That is the reason a stale `design.md` is worth a
reviewer's time at all.

S4 itself is **left open** — see `findings/security.md`. The constraint check is a
schema-verification design rather than a guard, and it does not belong bolted onto
this change.

- [x] **R2. `slate_json`'s doc comment claims a pinning its test does not perform**

**For:** `dev-writer` (comment), `tester` (the test, if the claim is kept)

`wire.rs:330-333`:

```rust
/// Pinned by a test against hardcoded key names, for the reason
/// `the_capability_json_is_pinned_to_the_exact_shape_the_plan_specifies` gives: a
/// view is written against these exact names and renaming one is a breaking change
/// no type checker would catch.
```

The precedent it cites pins the **whole serialised string** by equality —
`wire.rs:1580-1593` asserts `Capability::CanPost{…}.to_json()` equals
`r#"{"canPost":true,"identity":"abcd"}"#`. `Kept` (`wire.rs:2260-2276`) and
`Whoami` (`wire.rs:2511-2527`) do the same. `slate_json` does not: its test,
`the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
(`wire.rs:1696-1719`), only asserts each expected key **is present**:

```rust
for field in ["slate", "count", "candidates"] {
    assert!(v.get(field).is_some(), "the reply is missing {field}: {v}");
}
```

So the comment's "for the reason X gives" invites a reader to expect X's
strength, and the test name itself says "the exact shape" while checking a
subset. `tasks.md:174-181` already knows this and states it plainly:

> `the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against`
> checks each expected key is *present*, not that the key set is exactly
> those — so a `name` field added to a slate candidate would leave it green.

**The finding is that the honest account lives in `tasks.md`, which is deleted
before merge, while the overstated one lives in the code, which survives.** Per
`.claude/agents/README.md`, `findings/` and the tracker are scaffolding and the
reasoning has to move somewhere durable. Either the doc comment and the test
name drop the "exact shape" claim, or the test asserts the key set exactly.

**Measurement.** `slate_json` has one fewer guarantee than the three sibling
reply shapes, and the difference is invisible from the comment.

**Fixed** in `d8f9816` by taking the **second** of the two options offered — the test
now asserts the key set exactly — rather than the first, which was to weaken the
comment and the test name.

The finding is scrupulously even-handed between them, and the spec is what breaks the
tie. The scenario "A generated name and a mark are not settled by this capability"
requires that no candidate carry a display name or a visual mark; a presence-only
check cannot fail on that scenario at all. Dropping the "exact shape" claim would
have made the comment honest and left the contract unpinned, which is the worse of
the two honest states.

`the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against` now collects
each candidate's keys and `assert_eq!`s against a hardcoded sorted array, so an
**added** key fails as well as a removed one. Verified by re-applying the spec-test
reviewer's mutation — `"displayName": "Brave Otter"` on every candidate, which had
survived all 553 tests — and watching this test fail and nothing else.

The top-level key set is pinned the same way, which the finding did not ask for and
which costs nothing now that the shape of the assertion exists.

This also closes the half of `spec-test.md`'s finding 1 that was `tester`'s. **The
half that is `spec-writer`'s is not closed** — see that file.

**The generalisable point, which is the most valuable thing in this entry:** *"the
honest account lives in `tasks.md`, which is deleted before merge, while the
overstated one lives in the code, which survives."* That is the asymmetry that makes
an admission in a tracker worth less than a weaker test with an accurate comment.

- [x] **R3. `parse_index`'s doc comment names two callers; it has three, and the third is the one where its inline reasoning is wrong**

**For:** `dev-writer`

`wire.rs:913-933`. The doc comment:

```rust
/// Separated out because `page` and `perPage` are the same parsing job with the
/// same three failure modes, and a second copy would eventually disagree with
/// the first about whether `-1` is an error or a zero.
```

This change added a **third** caller — `keep_identity`'s `index` field at
`wire.rs:475`. Callers are `wire.rs:475` (`index`), `wire.rs:843` (`page`),
`wire.rs:847` (`perPage`).

The doc comment was not updated, and the inline comment at `wire.rs:922-925`
argues entirely in pagination terms:

```rust
// `as_u64` refuses a negative and a fractional number, which is
// exactly the set that should be refused: a page of -1 is not a
// page, and silently clamping it to 0 would serve the first page to
// a caller who asked for something impossible.
```

**Concrete illustration.** A reader arriving from `keep_identity` asks "why
does this refuse rather than coerce?" and is told about serving the wrong page
of a feed. The real answer for that caller is far stronger and is written down
elsewhere — `keep_identity` at `wire.rs:476-481` says a defaulted index "would
keep the first candidate for a caller who named none, which is storing an
identity nobody chose", and the spec calls that unrecoverable. The shared
helper's comment gives the mild reason for the mild caller and silently
withholds the severe one.

This is CLAUDE.md's named tell: *"Do not let a function quietly acquire a
second caller with different needs."* The needs differ in consequence, the
comment records only the first caller's.

Secondary, same site: the function is named `parse_index`, and `index` is now
also the name of a field it parses meaning something entirely different — a
slate position rather than a pagination offset. One concept, two names, and one
name, two concepts, in the same four lines.

**Fixed** in `d8f9816`. The doc comment names all three callers and the inline comment
now argues from the **severe** one, with the mild one kept beside it: a page of -1 is
not a page, and a defaulted index keeps the first candidate, which is storing an
identity nobody chose.

The finding's diagnosis of *why* this happened is the part I want recorded, because it
is CLAUDE.md's rule arriving from an angle the rule does not state: the shared helper
*"gives the mild reason for the mild caller and silently withholds the severe one"*.
The function was correct throughout — it always refused — so nothing failed; what was
wrong was that a reader could not learn from it which consequence the refusal is
protecting against. That is a comment defect with no code defect behind it, and those
are the ones that survive review.

While there I also fixed S6, which is about the same lines: `as usize` became
`usize::try_from`.

**The naming half is NOT fixed, and I am ticking this box anyway** — flagging that
rather than burying it. Renaming `parse_index` touches three handlers for no
behaviour change, and doing it inside a commit that also reshapes the session and
moves two derivations is the mixing CLAUDE.md forbids. A note now sits in the doc
comment saying the name came from pagination, that `index` means a slate position to
its newest caller, and that renaming is left to whoever next has a reason to touch
those handlers. If a reviewer thinks a tick requires the rename too, reopen it — the
box is ticked for the reasoning fix, which is what the finding's title and body are
about.

- [x] **R4. A field the spec never mentions is exposed in three replies with no `NO SPEC:` marker**

**For:** `dev-writer`

`path` appears in the slate reply (`wire.rs:348`), the keep reply
(`wire.rs:393`) and the whoami reply (`wire.rs:598`). Nothing in
`specs/identity-onboarding/spec.md` requires it: every occurrence of "path" in
that file (lines 55-403) is about derivation, recording, or the record's
readability — no requirement or scenario names a reply field.

`wire.rs:335-339` argues the decision well:

> **`path` is present**, and its presence is a decision rather than an
> oversight. It is not secret […] and a view that can show the user which path
> they are about to keep is a view that can render the recovery warning
> truthfully.

So the *reasoning* is recorded. What is missing is the **marker**, and
`.claude/agents/README.md` is specific about why it exists:

> **Mark unspecified behaviour in the code.** When the spec is silent and the
> dev chooses, the test carries `// NO SPEC: <what was chosen>`. Without a
> marker a reasonable default becomes permanent by accident.

**Concrete illustration.** A later reviewer grepping `NO SPEC` to enumerate
this change's unspecified choices gets two hits, both in `identity_store.rs`,
and concludes the wire replies carry nothing unspecified. The `path` field then
becomes a contract by silence — and it is a *widening* of the core API, which
CLAUDE.md says is "a decision to make on purpose rather than a side effect of
needing one more field."

**Measurement.** `grep -rn "NO SPEC" dialectica/rust-lib/` returns 2 markers
for this change (`identity_store.rs:700`, `:746`). Both of those are good — see
Clean, below. This is the third unspecified choice and it has none.

**Fixed** in `d8f9816`: a `NO SPEC:` marker on
`the_slate_json_is_pinned_to_the_exact_shape_a_view_is_written_against`, which is the
test that now pins the key set `path` appears in. It names what was chosen, why (not
secret, and a view that can show the path can render the recovery warning
truthfully), that no requirement or scenario asks for it, and that it appears in the
keep and whoami replies too.

The finding is right that the *reasoning* was already recorded on `slate_json` and
that what was missing was the marker, and right about why that matters: *"a later
reviewer grepping `NO SPEC` to enumerate this change's unspecified choices gets two
hits, both in `identity_store.rs`, and concludes the wire replies carry nothing
unspecified."* A marker is addressed to a different reader than a doc comment — the
one enumerating rather than the one reading in place — and good prose in the wrong
place does not reach them.

Placed on the test rather than beside `slate_json`, per the convention in
`.claude/agents/README.md` ("the test carries `// NO SPEC:`"), and it lands on the one
test a change to the reply's shape has to pass.

**For `spec-writer`:** this is now a visible unspecified choice rather than a hidden
one, and it is a *widening of the core API* — which CLAUDE.md says should be
deliberate rather than a side effect. The contract should either require `path` in
these replies or say it is not part of them. Not something `dev-writer` can settle.

- [x] **R5. `who_am_i` says "three ways that can fail" and then names a fourth**

**For:** `dev-writer`

`wire.rs:618-622`:

```rust
/// This handler therefore reports the identity where one is recorded and the
/// keystore opens, and a **distinguishable reason** in each of the three ways that
/// can fail. The fourth state — a master key with no recorded path for this Stoa —
/// is the one the two-store split creates, and its reason names the record so it
/// does not read as "you are nobody".
```

"three ways that can fail" followed immediately by "the fourth state" reads as
an off-by-one a reader has to stop and resolve. Counting the arms in
`whoami_for` (`wire.rs:660-698`) there are **four** failure routes, not three
plus one: `master()` returning `Err`, `paths()` returning `Err`,
`store.path_for` returning `Ok(None)`, and `store.path_for` returning `Err`.

`design.md:221-226`'s table has four rows and does not have this problem. The
code comment should either say four, or say "three keystore states and a fourth
the split creates" — whichever it means.

**Fixed** in `63133c9`: it says **four**, and enumerates them, because the finding's
count is the right one. Reading `whoami_for`'s arms confirms it — `master()` errors,
`paths()` errors, `path_for` returns `Ok(None)`, `path_for` errors — four failure
routes, not three plus a special case.

The alternative wording the finding offers ("three keystore states and a fourth the
split creates") would have been defensible and is worse, because two of the four are
*record* failures rather than keystore ones, so "three keystore states" is not true
either. Enumerating removes the question instead of rephrasing it.

The comment also now says the no-choice reason is the same string `getCapabilities`
gives, which is true as of the S2 fix (`NO_CHOICE_FOR_THIS_STOA`) and was not true
before it.

- [x] **R6. `OnboardingDir` credits a shape it did not copy**

**For:** `dev-writer`

`wire.rs:1615-1619`:

```rust
/// Same shape as `keystore.rs`'s and `log/sqlite.rs`'s, for the reason they
/// record: one need, in tests, is not worth a `tempfile` dependency.
```

It is `log/sqlite.rs`'s shape and **not** `keystore.rs`'s. Compare:

- `wire.rs:1623-1629` — `std::process::id()` in the name, a pre-emptive
  `remove_dir_all` before `create_dir_all`.
- `log/sqlite.rs:867-875` — identical: `std::process::id()`, pre-emptive
  `remove_dir_all`, same "The process id and the test's own name…" comment.
- `keystore.rs:1439-1445` — **8 bytes from `getrandom::fill`**, hex-encoded, and
  **no** pre-emptive removal.

Those are two different collision strategies with different properties: the
process-id form is reused within one run and so needs the pre-emptive wipe;
the random form does not. A reader told they are "the same shape" and then
asked to change one of them has been pointed at the wrong precedent.

`identity_store.rs:818-822` gets this right — it says "Copied in shape from
`log/sqlite.rs`'s `TempDir`" and names only that one. The accurate comment and
the inaccurate one are in the same change.

Note also that `identity_store.rs:823-839` and `wire.rs:1620-1645` are now the
*third* and *fourth* near-copies of this helper in the crate, differing only in
the filename prefix and the accessor method. Flagged as an observation rather
than a demand: CLAUDE.md warns against speculative refactoring, and "one need,
in tests" was the argument for hand-rolling it the first time. But the fourth
copy is the point at which "a fourth slightly-different guard" starts to apply,
and whoever touches this next should decide deliberately rather than adding a
fifth.

**Fixed** in `d8f9816`, and the correction is more specific than "name one file"
because the finding's evidence is specific: the comment now says it copies
`log/sqlite.rs`'s shape, and explains that the two strategies differ —
`std::process::id()` in the name needs the pre-emptive `remove_dir_all` because a name
is reused within one run, while `keystore.rs`'s 8 random bytes from `getrandom` does
not. So a reader asked to change one is told which precedent actually applies and
why they are not interchangeable.

The detail worth keeping: *"The accurate comment and the inaccurate one are in the same
change"* — `identity_store.rs`'s equivalent credits only `log/sqlite.rs` and is
correct. One author, one sitting, two comments, one wrong.

**The fourth-copy observation is recorded in the comment, not acted on**, which is
what the finding asks for. It now states that this is the fourth near-copy, names all
four, and says whoever needs a fifth should decide deliberately instead of adding it.
Unifying four test fixtures is a refactor with no change in front of it, which is the
speculative half CLAUDE.md warns against — and the finding says so itself.

- [x] **R7. `tasks.md` records a test count, which CLAUDE.md names as the canonical thing not to write down**

**For:** `dev-writer`

`tasks.md:148`:

> - [x] 6.1 Full suite: **546 passed, 0 failed** (475 before this change), with
>   no warnings from either of this repo's crates.

CLAUDE.md's "Keeping this file true" table lists *"how many tests pass"* against
*"Say to run: the suite, or the latest CI run"*, and the surrounding rule is
explicit: **"When you do record present state, make it self-invalidating."**
`546` and `475` cannot fail loudly; the next commit that adds a test makes them
quietly wrong, and a reader cannot tell whether they were true when written.

The self-invalidating form is available and costs nothing: "the suite passes,
and this change adds the tests listed above" ties the claim to a diff someone
can check, rather than to a total nobody can.

Same file, `tasks.md:151-153`:

> - [x] 6.2 `cargo fmt --check` exits 0

This is true and measures nothing, which the project already knows — see R8 and
`.claude/agents/README.md`'s "A green gate can be structurally blind". The tick
should say what the gate could not see, per that document's own instruction:
*"Say what a gate cannot see rather than reporting it as passed."*

**Fixed**, both halves, in the same commit as R8.

6.1 now names the command instead of a total, and says the tests this change adds are
the ones listed against the tasks above — the self-invalidating form the finding
prescribes, tied to a diff someone can check. It also records *why* the number was
replaced rather than merely corrected: design review measured 553 against the recorded
546, and the honest explanation is that two commits landed after the tick. A count
cannot fail loudly; that is the whole objection, and correcting 546 to 563 would have
reproduced it within the hour, since this fix pass changed the number again.

6.2 now says what `cargo fmt --check` **cannot see** — it does not follow path
dependencies, so it never reaches `dialectica-core`, where every file this change adds
lives — and records the per-file `rustfmt --check` that was run instead, with which
files' hunks are this change's and which are pre-existing. The clippy half of that box
was always real and is unchanged.

- [x] **R8. The new files are not rustfmt-clean, and the gate that would have said so cannot see them**

**For:** `dev-writer`

`cargo fmt --check` does not follow path dependencies, so it never reaches
`dialectica-core` — recorded in the user's memory as `dialectica-ci-fmt-gap.md`
and in `.claude/agents/README.md`. Checked per file instead, with
`rustfmt --check --edition 2021 --config skip_children=true`:

| file | result |
|---|---|
| `onboarding.rs` | 2 hunks |
| `identity_store.rs` | 5 hunks |
| `wire.rs` | 13 hunks |

All of it is in code this change added, so this is not pre-existing drift. A
sample, `identity_store.rs:236` — the whole `check_layout` body reindents,
including the comment inside the method chain:

```
-        conn.query_row("SELECT stoa, path FROM chosen_paths LIMIT 0", [], |_| Ok(()))
-            // `LIMIT 0` returns no row, so `QueryReturnedNoRows` is the SUCCESS
+        conn.query_row(
+            "SELECT stoa, path FROM chosen_paths LIMIT 0",
+            [],
+            |_| Ok(()),
+        )
+        // `LIMIT 0` returns no row, so `QueryReturnedNoRows` is the SUCCESS
```

and `wire.rs:623` / `wire.rs:651`, where both `who_am_i` and `whoami_for`
declare the same over-long `paths:` bound that rustfmt would break across four
lines.

**Why this is a readability finding and not housekeeping.** The next person to
run a formatter over this crate — or the change that closes the CI gap — gets a
diff touching three files in ways unrelated to whatever they were doing, and
the real change hides inside it. That is the same objection CLAUDE.md makes to
mixing a refactor with a behaviour change, arriving by a different route.

**Fixed**, and the finding's closing argument is what determined *how*, so it is worth
saying rather than just reporting clean.

`onboarding.rs` and `identity_store.rs` are **new files in this change** — neither
exists on `origin/main`, which I checked rather than assumed — so every hunk in them is
this change's and they were simply formatted, in the commits that own them.

`wire.rs` is not new, and this is where the finding's argument bites. Eight of its
hunks reproduce against `origin/main`'s copy of the file, so they are not this
change's. Formatting the whole file inside a behaviour commit would have swept them in
and produced exactly the unreviewable diff this entry describes; leaving my own hunks
unformatted would have repeated the defect. So the file was formatted in **its own
commit, ahead of the behaviour changes** (`90f698c`), with the suite re-run at the
unchanged count to show it changes nothing. That is CLAUDE.md's "make the change easy,
then make the easy change", with the counter-pressure respected: the file the change in
front of me touches, not a sweep of the crate.

**Two disclosures, because a clean `rustfmt --check` would otherwise overstate this.**

- `keystore.rs` has many pre-existing hunks. I formatted it, saw that it had swept
  them all in, and **reverted** — reapplying my four content edits by hand. Its
  pre-existing hunks are untouched and it is *not* format-clean.
- `identity.rs` picked up about five pre-existing hunks, because they sit inside test
  bodies this change edits (the `SecretKey::generate` call sites). Separating them
  would have meant reverting and reapplying those edits for five whitespace hunks. I
  judged that not worth a second stash cycle and am flagging it rather than letting it
  pass as clean work.

**A correction to the brief I was given, for the record:** it told me 20 hunks across
the three touched files were pre-existing and to reformat none of them. That is wrong
in the direction that matters — two of the three files do not exist on `main`, so
their hunks are all this change's, which is what R8 says. The only genuinely
pre-existing hunks in the touched set are `wire.rs`'s eight, plus `keystore.rs`'s and
`identity.rs`'s (which the brief did not mention). Trusting the brief over the measurement
would have left both new files unformatted.

---

## Stylistic preferences — not defects

- [x] **S1. `Slate::from_nonce`'s loop keeps a `paths` vector that tracks `candidates` exactly**

`onboarding.rs:248-285`. `paths` and `candidates` are pushed in lockstep —
every `paths.push` at line 259 is followed by a `candidates.push` at 279 unless
derivation errors and returns — so `paths.contains(&path)` at line 256 could be
`candidates.iter().any(|c| c.path == path)` and the second vector would go
away.

Preference, not a defect: the separate vector makes the *distinctness check*
textually independent of the candidate construction, and the doc comment at
lines 237-242 argues specifically that "Distinctness of *paths* is what is
checked". Keeping the checked thing in its own variable supports that reading.
Recorded so nobody 'simplifies' it without noticing it was deliberate.

**No change, ticked as actioned rather than fixed.** The reviewer states this is a
preference and not a defect, argues *against* making the change, and says the entry
exists so nobody simplifies it without noticing. Agreed on all three, and the code is
untouched.

Ticked rather than left open because there is nothing here that blocks a merge: an
empty box would tell the runner a decision is outstanding when the reviewer and I
agree the current shape is right. The entry has done its job by existing — the next
reader who reaches for `candidates.iter().any(...)` will find the argument for not
doing it.

- [x] **S2. `whoami_for` / `who_am_i` / `Whoami` — three spellings of one concept**

`wire.rs:563` (`Whoami`), `:623` (`who_am_i`), `:651` (`whoami_for`), and the
wire method is `whoAmI`. Each spelling is idiomatic for its own position (Rust
type, Rust fn, JSON camelCase) and the `Capability` / `capability_for` /
`get_capabilities` trio beside it has the same structure, so this is
consistent with the file it lives in. Noted only because the brief asks about
one-concept-one-name; I do not think it should change.

**No change, ticked as actioned.** The reviewer's own conclusion is that it should not
change, and the reason given is the right one: the three spellings are one per
position, and the `Capability` trio beside it establishes the pattern — so renaming
any of them would make this handler the inconsistent one.

Ticked for the reason S1 is: an empty box would signal an outstanding decision where
there is none. Worth noting that the S2 fix in `findings/security.md` added a fourth
member to the same family (`posting_identity`, which answers the probe's half of the
question), and it follows the same convention.

---

## What was clean

- **Both `NO SPEC:` markers explain *why*, not merely *what*.**
  `identity_store.rs:700-707` does not just say "refuse"; it names clamping as
  the rejected alternative and says why it is the dangerous one — *"a clamped
  path derives a perfectly valid key, so the user would be handed a working
  identity that is not the one they chose, with no error anywhere"*.
  `identity_store.rs:746-749` does the same for a malformed stored Stoa and
  cross-references the path argument rather than restating it. A later reader
  can decide whether the default was right, which is the test the brief sets.
  Both also say *why the spec is silent* ("nothing this build writes can
  produce one"), which is more than the convention asks for.

- **Test names say what would break, near-uniformly.** Read every new test name
  across the five touched files — 16 in `onboarding.rs`, 16 in
  `identity_store.rs`, the onboarding block in `wire.rs`, and the additions to
  `identity.rs` and `keystore.rs`. They name behaviour, not mechanism:
  `a_keep_whose_keystore_write_fails_records_no_path`,
  `a_selection_outside_the_set_is_refused_rather_than_coerced`,
  `a_second_choice_for_one_stoa_is_refused_and_changes_nothing`,
  `the_whole_path_reaches_derivation_and_not_only_its_low_byte`,
  `a_failed_keep_leaves_the_record_no_fuller_than_it_found_it`. The
  `X_rather_than_Y` construction in particular names the wrong behaviour it
  excludes, which is stronger than naming the right one. I found no name that
  describes only the mechanism.

- **Comments that record a correction rather than only a conclusion.** Several
  places write down a prediction that was wrong, which is the most useful thing
  a comment can do and the hardest to remember to do:
  `wire.rs:36-42` (mutex poisoning predicted, abort observed),
  `wire.rs:184-196` (`FnOnce`/`Fn`, "turned out to be half right", verified by
  reverting), `onboarding.rs:668-677` (the `[7;32]`/`[7;32]` fixture defect that
  made the no-secret test fail on first run), `tasks.md:22-27` (the
  non-collision test does *not* fail when the salt is reverted, and the code
  comment says so). `identity.rs`'s new pinned-constant comments state that the
  OpenSSL invocation was *first validated against the existing version-1 value*
  — that is the difference between a trustworthy vector and a plausible one,
  and saying so is what lets a reader trust it.

- **`docs/UI-BRIEF.md` was updated in the same change**, per CLAUDE.md's rule,
  and obligations 7 and 8 are written for someone who cannot read the code:
  they say what must not be claimed ("must not say 'save this and you can
  always get back in'", "do not render a padlock […] on the strength of this
  flag being unset") rather than only what is true. The note that the recovery
  flag "is currently always set […] when they are, the flag stops being set and
  the copy should follow it" is self-invalidating in exactly the form CLAUDE.md
  asks for.

- **Vocabulary is consistent where it matters, and the one bridge is
  explicit.** The spec says "master key" throughout; `keystore.rs` says "root
  secret" / `root`, inherited. `onboarding.rs:211` names the seam outright —
  *"The master key is a `&[u8; 32]` — the keystore's root"* — so a reader
  meeting both words knows they are one thing. `Stoa`, `slate`, `candidate`,
  `keep`, `path` are each used for one thing only across code, spec, design and
  the UI brief. `nonce` vs the wire field `slate` is the one place two words
  name one value, and `onboarding.rs:80-89` explains why the public name is
  `slate`.

- **The two split-out decision functions earn their split in the comment.**
  `capability_for`, `keep_selection` (`wire.rs:488-493`) and `whoami_for`
  (`wire.rs:641-650`) each say the split is so the mapping can be tested
  without also asserting on serialisation. That is a reason a reader cannot
  derive from the code, which is the test CLAUDE.md sets for a comment earning
  its place.

- **The adapter bodies stay thin and say why.** `dialectica/rust-lib/src/lib.rs`
  — `storage_dir` and `master_key` are extracted with the duplication count
  stated ("previously duplicated in two handlers and heading for five"), and
  `master_key`'s doc comment names a real consequence most authors would have
  left implicit: two slate calls on a fresh install offer candidates of two
  *different* master keys, the `live_slate` check does not catch it, and what
  makes it harmless is that the keep mints and writes its own. That is a
  reader's "but wait —" answered before they ask it.

---

## Not assessed

Correctness of the derivation, the write ordering, panic reachability, the
`MAX_PATH_WALK` bound, whether `path_from_row` is applied at every read site,
and whether the recorded decisions match the code beyond the one documentation
contradiction in R1 — all other reviewers' dimensions.

## Tree state

No files other than this one were created or modified. `rustfmt --check` was
used, which writes nothing; no `cargo mutants` run and no hand-broken property.
`git status` in the review worktree shows only this findings file.
