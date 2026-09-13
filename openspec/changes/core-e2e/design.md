# Design — `core-e2e`

## Context

`dialectica-core` had no integration target. Everything was an in-crate
`#[cfg(test)]` module, which can reach private items and mostly runs in memory —
so nothing checked the crate as an outside consumer sees it, and nothing checked
persistence against a file. This change adds one target and corrects the CI gate
that could not see it.

Written after the implementation and revised twice: once when review measured
three claims in the file as false, and once when a mutation found a test passing
for the wrong reason.

## Decisions

### Hardcode private caps rather than import them — and for the right reason

`FIELD_CAP` and `TITLE_CAP` are literals in the test file. The reason is simply
that **an integration test cannot see a private `const`** — `MAX_FIELD_LEN` and
`MAX_TITLE_BYTES` are private, and a test outside the crate has no route to them.

**Rejected reasoning, which the file originally gave:** that hardcoding is the
stronger assertion because nothing else pins these caps, so a drifted cap would
pass every test that only probes absurd values. That premise is false for this
crate. `op.rs::the_field_cap_is_pinned_to_a_known_answer` asserts
`MAX_FIELD_LEN == 150 * 1024` and `stoa.rs` asserts `MAX_TITLE_BYTES == 1024`,
both hardcoded, both for exactly that reason. A reader who believed the original
comment would think this file was the only guard and not know a drifted cap
already has two tests to argue with.

The conclusion survived; the argument did not. Worth recording because the
difference matters the next time someone asks whether to import a constant into
a test: the answer is about **reach**, not about strength.

### Reach the wire layer rather than renaming the file

The file is titled "end to end" and stopped at `feed::list_threads`, one layer
below `wire::list_threads_from_request` — the JSON in / JSON out boundary
CLAUDE.md calls the deliverable. Two options: extend the target, or rename the
file to say what it covers.

**Extending won, and the measurement is why.** `Err` and `{"error":...}` are two
different things, and the mapping between them is real behaviour, not plumbing.
Mutating `list_threads_from_request` to return an empty page instead of
`error_json` on a failed store open kills the new wire test **while the
`feed::list_threads`-level test one layer down stays green**. So §11.1
obligation 5 — "an empty feed is indistinguishable from a Stoa nobody has posted
in" — is an obligation about what a *reader* sees, and could not be discharged at
the layer the file stopped at. Renaming would have documented the gap; extending
closed it.

The rename was not wholly rejected: the header now carries an explicit "what this
does not cover" section naming the publish path, `list_stoas`, membership,
transport and the view. Those absences are legitimate — `list_stoas` does not
exist in the code at all — but "end to end" in a title reads as though they were
covered, so they are stated rather than left to be inferred.

### One rglob, not a list of source roots, for CI's declared count

The gate compares cargo's reported test count against `#[test]` attributes
counted from the source. That count came from a hand-maintained list of three
directories.

**Chosen:** one `rglob("*.rs")` over `dialectica/rust-lib` with
`if "target" in p.parts: continue`.

**Why the list lost.** Its failure mode is the one this project has been bitten by
— a list of N that an N+1th case escapes silently — and here the escape produces a
*misleading red*: `ran` grows, `declared` does not, and the job dies with a message
about cfg-gating, which is the one explanation that is not the problem. The list's
two stated advantages did not survive: walking `target/` is a one-line exclusion,
and "it would keep passing if the inner crate were moved out" is already covered by
the `declared == 0` guard plus the mismatch, because moving a crate out of the `-p`
set changes `ran` too.

**Measured, not assumed:** rglob 530, three roots 530, cargo `ran` 530 on the
merged tree — behaviour-preserving today and strictly broader tomorrow. There is
exactly one `target/` in the tree.

**The consequence, which is the real decision:** `examples/` is now in scope. The
old comment claimed a `#[test]` in an example would fail the gate "correctly,
because that test would never be run", and a review created
`dialectica-core/examples/probe.rs` with one `#[test]` and watched the gate stay
green — the example was in neither number, so the test was counted nowhere and run
nowhere. Under the rglob, the same probe makes `declared` exceed `ran` and the
mismatch fires. The claim is now true because the directory is walked.

### Parameterise the fixture's filename rather than let a test hand-roll the primitive

`TempDir::store()` and `reopen()` hardcoded `"ops.sqlite"`, so the one test needing
**two** stores in one directory — the over-cap pair, which needs an at-cap store
that stays readable beside an over-cap store that is poisoned — wrote its own
`SqliteOpLog::open` and `drop` inline. That is a second, divergent copy of the
restart primitive the fixture's own doc calls load-bearing.

**Chosen:** `store_at(name)` / `reopen_at(store, name)`, with `store()` / `reopen()`
as one-line wrappers over a `CONVENTIONAL_STORE` constant. Complexity in the data
shape, not a fourth branch; every existing call site is unchanged.

The cost of not doing it is recorded rather than hypothetical: `log/mod.rs` already
notes that leaving weaker copies in place "made them the template the next test
would be written against", and the publish path and a two-peer membership test will
both want two stores.

### Follow the recorded 16-byte prefix rather than diverge to 31

The shared-prefix fixture originally agreed on 31 of 32 bytes.
`log::fixtures::SHARED_PREFIX_BYTES` is **16**, with an argued rationale that
explicitly rejects 31: comfortably past any plausible accidental truncation while
leaving the divergence unmistakable, where 31 "would test the same property and
read as a puzzle".

**Chosen: 16, with a sentence saying where the number comes from.** The duplication
is forced — `log::fixtures` is `pub(crate)` and an integration test cannot import
it — but duplicating the *value* while inventing a second, disagreeing *rationale*
leaves a reader with two precedents and no way to tell which is current.

**Verified the weaker-looking fixture is not weaker:** mutating `iter_stoa` to
compare `substr(stoa, 1, 8)` still leaks, killing the test with
`["right","left"]` vs `["left"]` — the same signature the 31-byte fixture produced.

### Assert the over-cap defect as it stands, and say that nothing tracks it

`an_over_cap_body_signs_and_appends_and_then_poisons_every_read_forever` pins a
live defect on `main` rather than the behaviour that would be correct, because a
test asserting the fix would fail on `main` and this is not the change that fixes
it. The assertion names the current outcome; the panic message names what should
replace it; the at-cap half means the fencepost is already covered when someone
does fix it.

**The gap, now stated in the file:** this test is the only record the defect
exists. Checked while writing — the repository has no issues at all
(`gh issue list --state all` empty) and `docs/PLAN.md` does not mention the
asymmetry. A test is a good place to reproduce a known bug and a bad place to be
the sole evidence of one, because deleting or rewriting the test loses the
knowledge with it. The file says so and says who should replace the paragraph.

### Split test names that join two claims with `and`

Two names asserted two things each, so neither half could fail on its own name.
Both are split, and the split immediately earned itself: under the
every-store-in-memory mutation, the new
`a_page_past_the_end_is_an_empty_page_rather_than_a_panic_or_a_wrapped_first_page`
**survived** — because "page 99 is empty" is also what a store holding nothing
answers. That is this project's own defect family, a fixture where two
explanations produce the same answer, and it was invisible while the claim was
bundled with the tiling claim that did fail. A fixture guard asserting page 0
holds three rows fixes it; the mutation now kills 21 of 24 rather than 20.

The counterpart is worth recording too: `an_empty_store_answers_every_read…`
**correctly** survives that mutation. An in-memory store genuinely answers every
read empty, which is the behaviour the test asserts, so there is no rival
explanation to exclude. A test that survives because the mutation does not change
what it claims is passing, not weak — and telling the two cases apart is the whole
reason to run the mutation rather than count the survivors.

### State the sectioning rule rather than reorganise the sections

Sections are the **boundary a test crosses**, not the capability it exercises,
because this crate's defects live in seams and organising by capability hides
seams. That was already the scheme; what was missing was a rule a later author
could apply, with four pieces in flight all adding to one file. Three sentences in
the header: a test belonging to two sections goes under the one it would fail at
first; a test crossing a boundary no section names gets a new section; and a
heading states the boundary and never a count. The file had shipped "at three
boundaries" over a section holding four.

### Date the mutation table, and say what a later author owes it

The table is eleven (now eighteen) rows of measured mutations, and nothing keeps it
true — no gate re-runs it, and most rows name implementation symbols a rename
silently invalidates. Rather than delete it or pretend it is current, it is grouped
by when each group was measured, with the commit named.

**What a later author owes it is stated explicitly, because the honest answer is
"almost nothing":** adding a test does not oblige anyone to re-run eleven
mutations — that would be a tax nobody would pay, and the table would rot anyway.
What is owed is the same discipline for the new test, and fixing or deleting a row
whose symbol you rename. A row pointing at a symbol that no longer exists is worse
than no row, because it reads as though somebody checked.

### `skip_specs` needs `schema:` beside it, or it is silently ignored

Worth writing down because the failure is misleading rather than absent. A
`.openspec.yaml` containing only `skip_specs: true` is **not valid change
metadata**, so `openspec validate --strict` does not honour the marker and then
fails for having no deltas — two errors that read as two problems when there is
one. The fix is a `schema:` key naming a known schema (`spec-driven` here, matching
`openspec/config.yaml`). With both keys the validator reports
`skip_specs is set ... zero deltas accepted` and passes.

This is the first test-only piece in the repo, so it is the first to need the
marker at all. The next one should not have to rediscover the second key.

## Recorded dead end

**A comment asserting coverage is the defect this file kept producing.** Three of
its claims were measured false by review — a "fully-specified row" comparison that
was three individual field reads, a fabricated ordinal citation of the project's
defect list, and a premise two other tests disprove — plus an uncited count of
"three bugs" with no record anywhere. All four read as evidence because they were
specific, and two of them were numbers.

The fix that generalises: **get a count, ordinal or duration from a command before
writing it**, and where a comment claims a test would catch something, make the
test structurally capable of it. The vote test's exhaustive destructure is the
worked example — it fails to *compile* when a field is added, which is a stronger
guarantee than any assertion and cannot be skipped.
