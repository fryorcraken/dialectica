//! The read path, end to end, against real files on a real disk.
//!
//! # What this target covers, and what it does not
//!
//! **Covers:** the read path from the JSON request a view sends to
//! [`dialectica_core::wire::list_threads_from_request`] down to the bytes in a
//! SQLite file, plus the keystore file that mints the signing identity. Both ends
//! are real files, and the layers between them are crossed rather than stubbed.
//!
//! **Does NOT cover, and these are absences to know about rather than gaps to
//! infer:**
//!
//! - **Revising.** No test here publishes a revision. The rest of the publish
//!   path is now crossed — `authoring::post`, `reply` and `vote` have their own
//!   section at the end of this file, added under rule 2 when the seeder piece
//!   needed the write path covered by something that runs. Every test ABOVE that
//!   section is still a read over hand-appended fixtures, which is why the
//!   sections are ordered as they are.
//! - **`list_stoas` and membership.** Neither exists yet: `grep -rn
//!   "list_stoas\|listStoas"` over `dialectica/` **and** `docs/` finds nothing in
//!   `dialectica/` but these comment lines, and in `docs/` only the `listStoas()`
//!   line of PLAN.md's JSON contract. There is nothing here to cover because
//!   there is nothing there to call. Both roots matter: run it over `dialectica/`
//!   alone and it returns only itself, which confirms nothing.
//! - **The capability probe, and this one is a GAP rather than an absence.**
//!   `wire::get_capabilities` and `wire::capability_for` are public in the same
//!   module as the `list_threads_from_request` this file reaches, and
//!   `openspec/specs/posting-capability/spec.md:53` requires six reasons be
//!   distinguishable, three of which are properties of a FILE: "the keystore's
//!   permissions are too open", "the keystore's directory is writable by others",
//!   and "the keystore is unreadable or malformed". A file property is precisely
//!   what an in-memory per-change suite cannot pin and what this target exists
//!   for, and the fixture cost here is near zero — this file already writes
//!   keystores to disk and already sets `0o700`.
//!
//!   So this line is an admission, not a boundary. It is the same shape as the
//!   gap that put `list_threads_from_request` in scope: public on `main`, with
//!   this file stopping one layer below it and saying nothing. Naming it is the
//!   minimum; closing it needs whoever owns `posting-capability` to say which of
//!   the six states a test may construct, since three of them are reached by
//!   making a file hostile rather than by calling anything.
//! - **Transport.** Nothing here sends or receives an op over the network. An
//!   arriving op is modelled as an `append` with an `Arrival`, which is what the
//!   store sees, and not as anything a peer did.
//! - **The QML view.** Out of scope by construction: this is a Rust integration
//!   test of the core crate, and the view forwards JSON.
//!
//! # How the sections are organised, and where a new test goes
//!
//! **Sections are the BOUNDARY a test crosses**, not the capability it exercises
//! — because this crate's defects live in seams and organising by capability
//! hides seams. Three rules, so that four pieces in flight adding to one file
//! answer "where does this go?" the same way:
//!
//! 1. A test belonging to two sections goes under the boundary it would fail at
//!    **first**. A forged hide is both authority and refusal; it fails at the
//!    authority check, so it lives under moderation.
//! 2. A test crossing a boundary no section names **gets a new section**. The
//!    write path was the one this rule anticipated, and it has arrived: the final
//!    section crosses `authoring::*`. A revision publish is the next one.
//! 3. A section's heading states the boundary, never a count of what is under it.
//!    A count goes stale the first time somebody adds a test and is a claim
//!    nothing checks — this file shipped "at three boundaries" over four.
//!
//! # Why this target exists, and what the per-change suites structurally cannot do
//!
//! Every other test in this crate is an in-crate `#[cfg(test)]` module, which
//! means two things this file deliberately does not inherit:
//!
//! 1. **They can reach private items.** A unit test constructs a `Keystore` from
//!    a fixed seed, pokes a `SortKey`, calls a `pub(crate)` decoder, or borrows a
//!    fixture from `log::fixtures`. That makes them good tests of mechanism and
//!    no evidence at all that the *public API* is usable: a crate whose public
//!    surface was missing a method entirely would pass every one of them. This
//!    file imports `dialectica_core` as an outside consumer does and touches
//!    nothing private. Where a test here needs a value the crate keeps private
//!    — the field cap, the title cap — the value is **hardcoded, because an
//!    integration test cannot see a private `const` at all**. That is the whole
//!    reason, and it is enough of one.
//!
//!    It is specifically NOT that nothing else pins these caps. Two unit tests
//!    already hardcode both — `op.rs`'s
//!    `the_field_cap_is_pinned_to_a_known_answer` asserts
//!    `MAX_FIELD_LEN == 150 * 1024`, and `stoa.rs` asserts
//!    `MAX_TITLE_BYTES == 1024` — so a drifted cap has two other tests to argue
//!    with before it reaches this one. What this file adds is an outside
//!    consumer's view of the same boundary, reached only through public API.
//!
//! 2. **They mostly run in memory.** `SqliteOpLog::in_memory()` exercises the
//!    real SQL, which is most of the value, and by construction cannot survive
//!    being dropped. "The store is rebuildable by replay" is a claim about a
//!    *file*, so every store here is a file and every restart is a dropped
//!    connection and a reopened path — never a flag.
//!
//! A third reason is specific to this crate's defect history. Its expensive
//! defects have been **cross-layer** — the seam between encode and decode,
//! between the wire shape and the store, between memory and disk — and a
//! per-change suite structurally cannot see one, because each layer is correct
//! in isolation and the defect lives between them. The over-cap body this file
//! documents below is the worked example: `canonical_bytes` and `decode` are
//! each defensible alone and disagree with each other. So the tests below prefer
//! crossing a boundary to going deep on one.
//!
//! No count is given for how many such defects there have been, because none is
//! recorded anywhere countable — a review measured that `grep -rn "cross-layer"`
//! over `openspec/` and `docs/` returns nothing but this file. The argument is
//! about the SHAPE of the defect, which does not need a tally to hold.
//!
//! # The defect family these fixtures are shaped against
//!
//! Every test defect found in this repo so far has been **a fixture where two
//! explanations produce the same answer**. An empty store and a refused write
//! both list nothing; a root parent and a copied thread id give the same thread;
//! `"ab"` and `"abc"` differ whether or not a length prefix is written. So each
//! fixture below is built so that only one explanation survives, and the comment
//! on the test says which rival explanation it excludes.
//!
//! The instance this file has to work hardest to avoid is asserting that
//! something "came back" using a value that came from the same call that produced
//! it. A read that returns what the write just handed us is the test agreeing
//! with itself. Where an expected value can be derived independently — an address
//! is a hash of a record, an author address is a hash of a key — it is derived
//! that way; where it cannot, it is hardcoded.
//!
//! # Mutations run against this file, and what each killed
//!
//! Every row was applied to the implementation, the suite run, and the mutation
//! reverted. A test nobody has watched fail is a test nobody knows works.
//!
//! Each row carries the failure PREDICTED before the run and the failure
//! OBSERVED after it. The notes below the table are the rows where the two
//! disagreed, and most of those disagreements changed this file — a prediction
//! that misses is the most useful row in the table, because it is the one that
//! found something. No count is quoted here: the notes are numbered where they
//! stand, and a tally in this paragraph would go stale the next time one is
//! added, which it has.
//!
//! **EACH ROW WAS MEASURED ONCE, at the commit its group heading names, and
//! nothing keeps
//! this table true.** There is no gate that re-runs these and no test that fails
//! when a row goes stale — and most rows name implementation symbols
//! (`feed::list_threads`, `moderation::resolve`, `Moderators::contains`,
//! `SqliteOpLog::append`, `SqliteOpLog::open`, `from_connection`) that a rename
//! silently invalidates. Read the table as dated history, not as a current claim.
//!
//! **What a later author owes it:** if you add a test here, you owe this table
//! nothing — a row is a record of one experiment, not a coverage claim, and
//! re-running the whole table to add one test would be a tax nobody would pay.
//! What you owe is the same discipline for YOUR test: mutate the thing it
//! claims to cover, predict the failure, watch it, and add a row saying so. If
//! you RENAME or MOVE something the table names, fix the row in that commit or
//! delete it; a row pointing at a symbol that no longer exists is worse than no
//! row, because it reads as though somebody checked.
//!
//! ## Measured at commit `9bb2bc1`, when this file was added
//!
//! | Mutation | Predicted | Observed |
//! |---|---|---|
//! | `feed::list_threads` drops its `entry.op.verify()` check | forgery test alone | that test alone, `["genuine","forged"]` vs `["genuine"]` — as predicted |
//! | `feed::list_threads` keeps `Post`s with any `parent` | reply test alone | that test alone, `["root","reply"]` vs `["root"]` — as predicted |
//! | `moderation::resolve` checks authority AFTER taking the leading `Moderate` | forged-hide test alone | that test alone, `Unmoderated` vs `Hidden(..)` — as predicted |
//! | `Moderators::contains` returns `true` unconditionally | two tests | **ONE test, and at a fixture guard** — see note 1 |
//! | `SqliteOpLog::append` uses `INSERT OR REPLACE` | re-arrival test, on the Lamport value | that test, **but at the `Appended` assertion** — see note 2 |
//! | `iter_stoa`'s `WHERE stoa = ?` compares a 2-byte prefix | shared-prefix test alone | that test alone, `["right","left"]` leaked into the `left` read — as predicted |
//! | `from_connection` discards `check_layout`'s error | mislabelled-store test alone | that test alone, opened `Ok` — as predicted |
//! | `found != LAYOUT_VERSION` loosened to `found >` | foreign-version test alone | that test alone, degraded to `LayoutDoesNotMatchItsVersion{version:1,why:"no such table: ops"}` — as predicted, including the mechanism |
//! | `sanitise` stops removing invisibles | sanitise test alone | that test alone, `"hello\u{200b}world"` vs `"helloworld"` — as predicted |
//! | the paging slice loses one row per page | paging test alone | that test alone, first page 2 rows vs 3 — as predicted |
//! | `SqliteOpLog::open` ignores its path and opens `:memory:` | ~15 of 20 | **18 of 20** — see note 3; re-run at 24 tests in the group below |
//!
//! ## Measured at commit `1342aa9`, the readability and architecture findings
//!
//! The suite was 20 tests when the group above was run and is 24 now, so the
//! `:memory:` row was re-run rather than left to read as though it still
//! described the whole suite.
//!
//! | Mutation | Predicted | Observed |
//! |---|---|---|
//! | `pub score: i64` added to `FeedRow`, set to `7` | the vote test, as a COMPILE error | `E0027: pattern does not mention field \`score\`` — as predicted. This is the mutation a review ran against the previous form of that test and watched **pass**; the destructure is what changed it |
//! | `list_threads_from_request` returns an empty page instead of `error_json` on a failed store open | the JSON error-shape test alone | that test alone, `{"hasMore":false,"items":[],"page":0}` where an `error` was required — and the `feed::list_threads`-level test one layer down **stayed green**, which is why that layer could not cover this seam |
//! | the wire row's `author` is emitted as `""` | the JSON happy-path test alone | that test alone, `""` vs the independently derived address — as predicted |
//! | the paging slice loses one row per page | the TILING test, and not the past-the-end test | exactly that split, first page 2 vs 3 — which is what splitting the old `…and…` name bought |
//! | `iter_stoa` compares `substr(stoa, 1, 8)` | shared-prefix test alone | that test alone, `["right","left"]` vs `["left"]` — so the 16-byte fixture is as strong as the 31-byte one it replaced |
//! | `SqliteOpLog::open` refuses a path that does not exist | the created-file test **at its own assertion**, plus fixture-guard deaths | 18 of 24: the split test died on `a missing store is created, not refused`, the other 17 at the `dir.store()` helper — see note 4 |
//! | `SqliteOpLog::open` ignores its path and opens `:memory:` (re-run) | more than 18, since two wire tests reach a file | **20 of 24**, then **21 of 24** — see note 4 |
//!
//! ## Measured at commit `f007bcd`, the correctness and security findings
//!
//! The suite is 25 tests now: `two_temp_dirs_with_the_same_tag_get_different_unguessable_names`
//! was added with the fixture change the security findings asked for.
//!
//! | Mutation | Predicted | Observed |
//! |---|---|---|
//! | `Moderators::contains` returns `true` unconditionally (re-run, BEFORE the guard moved) | 2 of 24, the hide test at its fixture guard | exactly that — `22 passed; 2 failed`, the hide test at `the fixture's outsider must not be a moderator`. Note 1's remedy had been applied to only one of its two halves |
//! | the same mutation, AFTER the guard moved below the assertions | still 2, but the hide test on the RESOLUTION | `Ok(Hidden(Entry{..action: Hide..}))` vs `Ok(Unmoderated)` — the substance of the requirement, as predicted. See note 5 |
//! | `moderation::resolve` checks authority AFTER taking the leading `Moderate` (re-run, to test whether the OTHER guard has the same defect) | the forged-hide test at its `resolve` assertion, NOT at its `iter_target` guard | exactly that, 1 of 24, `Unmoderated` vs `Hidden(..)` at the named claim. **The two guards are not the same case**, so this was one line and not a pattern |
//! | `TempDir::new`'s `set_permissions(0o700)` replaced by `0o777` | the two keystore tests, at `Keystore::open` and NOT at `Keystore::create` | exactly that: `DirectoryWritableByOthers { mode: 511 }` at the two `open`/reopen call sites. `0o777` rather than deleting the call, because a deleted call inherits whatever the temp directory's default mode is — which could already be private and would make the probe pass for the wrong reason |
//!
//! Two more at the same commit `f007bcd`, mutating the new fixture test itself,
//! since a test pinning a fixture property is the easiest kind to write
//! unfalsifiably:
//!
//! | Mutation | Predicted | Observed |
//! |---|---|---|
//! | `TempDir::new` reverted to `dialectica-e2e-<pid>-<tag>` | `assert_ne!` on the two paths | that assertion, both `"/tmp/dialectica-e2e-2431642-x"` — as predicted |
//! | the random suffix narrowed from 8 bytes to 2 | the LENGTH assertion, since two 2-byte names still differ | that assertion, `21` vs `33` — as predicted, so the two halves discriminate independently |
//!
//! ## Measured at commit `5323b57`, the spec-test findings
//!
//! One mutation, applied twice to separate its halves. The suite is 26 tests now:
//! `the_json_envelope_reports_the_page_that_was_asked_for_and_whether_more_follows`
//! was added because a review proved the envelope had **no coverage at this
//! seam** — see note 6, which is the most useful row this table has.
//!
//! | Mutation | Predicted | Observed |
//! |---|---|---|
//! | the feed reply emits `"page": 0, "hasMore": false` as literals | the envelope test at its `page` assertion on page 1, `0` vs `1` | that test alone, 1 of 26, but at the **`hasMore` assertion on page 0** — `Bool(false)` vs `Bool(true)`. See note 6 |
//! | `"page": 0` alone, `has_more` restored | the `page` assertion on page 1, `0` vs `1` | exactly that, `Number(0)` vs `Number(1)` — so the two halves discriminate independently and neither rides on the other |
//!
//! ## Measured on `piece/seed-store`, when the publish-path section was added
//!
//! Three mutations, for the two tests in the final section. Each was applied to
//! the implementation, the target run, and the mutation reverted; `git status`
//! was checked clean before the commit.
//!
//! | Mutation | Predicted | Observed |
//! |---|---|---|
//! | `authoring::publish` signs and attributes EVERY op with one fixed key — authorship collapsed | both new tests: the set assertion, and the seeding test's feed-author check | exactly that, 2 of 28. The set came back one address against two; the seeding test died at claim 2. **The other 26 survived**, which is what shows the publish path had no attribution coverage in this target before |
//! | `authoring::thread_of` returns the op's own id instead of the parent's `thread` — the `thread: parent` bug | the seeding test alone, at the **`nested`** row and not the `reply` row | exactly that, 1 of 28, `nested`'s thread `Some(reply)` vs `Some(root)`. The `reply` row passed first, because at two levels the parent's id and the parent's thread are the same value — which is why the fixture has three |
//! | `wire.rs`'s `posting_identity` reports `stoa_public_key` instead of `stoa_address_at_path` — **the three-derivations gap closed at the probe** | the seeding test alone, at the `assert_ne!`, both operands equal | exactly that, 1 of 28. This is the same mutation a review ran against the example's copy of this assertion and watched **exit 0** — the copy in `examples/` is compiled by CI and run by nothing, which is the whole reason this section exists |
//!
//! **Note 6 — the mutation this file could not kill, and why the fixture was the
//! reason.** A review replaced `"page": page.page, "hasMore": page.has_more` in
//! `wire::feed_page_json` with the literals `0` and `false`, and **all 25 tests
//! here passed.** Repo-wide only one test died, and it uses a `MemoryOpLog` —
//! so the JSON-against-a-real-file seam this section exists to own had no
//! envelope coverage at all.
//!
//! The cause was not a missing assertion. `a_request_naming_a_stoa_on_disk…`
//! asserts both fields; it reads **page 0 of a one-post store**, where the
//! correct answer *is* `0` and `false`. Two explanations, one answer — this
//! file's own stated defect family, at the outermost boundary the crate has.
//! A fixture, not an assertion, is what was wrong.
//!
//! So the new test uses five posts at `perPage: 2`: three pages, a non-zero
//! index, `hasMore` true on one read and false on another. No single constant
//! satisfies both ends. The prediction then missed on WHICH assertion fires
//! first — page 0 is read before page 1 and its `hasMore` is genuinely `true`,
//! so the `hasMore` half fires an iteration earlier than the `page` half. That
//! miss is why the mutation was re-run with `page` alone: a test killed only by
//! the `hasMore` literal would leave `page` unproven, and the second row is what
//! shows it is not.
//!
//! **Note 1 — a mismatch that was a defect in this file.** The `contains`
//! mutation was predicted to kill two tests. It killed one, and it killed it at
//! `assert!(!moderators.contains(..))` — a FIXTURE GUARD, which aborts its test
//! before the behaviour under test runs. So the suite went red for the right
//! reason by accident: nothing had asserted what `contains` answers as a
//! behaviour, and the unauthorised-hide resolution was never reached.
//! `the_moderator_set_of_a_genesis_record_is_exactly_its_creator` was added in
//! response, asserting both directions of the predicate directly. Re-running the
//! mutation then killed two tests, one of them on the behavioural claim.
//!
//! **That remedy was half of one, and note 5 is the other half.** Adding a test
//! beside the weak one does not strengthen the weak one: the guard was still
//! sitting in front of the assertions it blocked, so `a_hide_by_a_non_moderator…`
//! went on dying before it observed anything. A review measured exactly that and
//! the fix is recorded in note 5.
//!
//! **Note 2 — a mismatch that reordered a test.** The `INSERT OR REPLACE`
//! mutation killed the right test at the wrong assertion: `Appended::Stored` vs
//! `AlreadyPresent` fired first and the test died before checking the recorded
//! Lamport value, which is the substance of the requirement. The two `Appended`
//! values are now captured and asserted AFTER the metadata, so the mutation fails
//! on `Some(9)` vs `Some(1)` — the claim the test is named for. A test that
//! reports the shallowest of its failures hides the rest.
//!
//! **Note 3 — the load-bearing mutation.** Making every store in-memory killed
//! **18 of 20** at the time, more than the ~15 predicted. The two survivors were
//! the only two tests that deliberately touch no store at all
//! (`the_moderator_set_of_a_genesis_record_is_exactly_its_creator` and
//! `an_over_cap_genesis_title_is_refused_before_it_can_name_a_stoa`), which is the
//! correct outcome for pure-value tests. This is what proves the persistence
//! claims here are about a file rather than about process memory: a suite where
//! this mutation killed little would be a suite whose "survives a restart" tests
//! were restarting nothing.
//!
//! **Note 4 — the re-run found a test passing for the wrong reason.** Re-running
//! note 3's mutation at 24 tests killed **20**, and the four survivors were the
//! two pure-value tests above plus the two halves of the newly split paging pair.
//! One of those two was a genuine defect and one was correct, and telling them
//! apart is the point of running the mutation at all:
//!
//! - `a_page_past_the_end_is_an_empty_page…` **should not have survived.** "Page
//!   99 is empty" is also what a store holding nothing answers, so the test
//!   passed against a fixture that had persisted no rows — this project's own
//!   defect family, a fixture where two explanations give the same answer. A
//!   fixture guard asserting page 0 holds three rows was added, and the re-run
//!   then killed **21 of 24**.
//! - `an_empty_store_answers_every_read…` **correctly survives**, and is not the
//!   same case. An in-memory store genuinely does answer every read empty, which
//!   is the behaviour that test asserts; there is no rival explanation for it to
//!   exclude. A test that survives because the mutation does not change what it
//!   claims is a passing test, not a weak one.
//!
//! **Note 5 — a fixture guard belongs AFTER the assertions it guards, and this
//! file learned that twice.** Note 2 moved the `Appended` values below the
//! metadata check for this reason; the identically-shaped guard in
//! `a_hide_by_a_non_moderator_leaves_the_thread_visible` was left in place, and a
//! review measured the consequence: under the `contains` mutation the test died at
//! the guard and observed neither the resolution nor the feed, so the two
//! assertions carrying the requirement had never been watched fail. Moved below
//! them, the same mutation fails on `Hidden(..)` vs `Unmoderated`.
//!
//! **The generalisation was checked before it was drawn, and it does not hold.**
//! The one other guard of the same surface shape — the `iter_target` ordering
//! check in `a_forged_hide_does_not_displace…` — is correctly placed, because the
//! mutation that test is aimed at changes `resolve` and cannot change the store's
//! ordering: the guard passes under it and the substantive assertion fires
//! (measured, above). A guard in front of its assertions is a defect only when a
//! plausible defect makes the guard fail *first*; where the guard fires only under
//! a different layer's defect, it is the more useful message. So "move every
//! guard to the end" is the wrong rule. The right one is: **ask which mutation
//! would trip this guard, and whether that is the same mutation the test is
//! aimed at.**
//!
//! # A defect this file found, and did not fix
//!
//! `an_over_cap_body_signs_and_appends_and_then_poisons_every_read_forever`
//! documents a live cross-layer defect on `main`: `Op::canonical_bytes` is
//! infallible and writes any length, while `Op::decode` refuses a field over
//! 153,600 bytes. So a 153,601-byte body signs, appends `Ok(Stored)`, and from
//! that moment every `iter`/`iter_stoa` on the store fails `CorruptEntry` —
//! permanently, across a restart, with no public API able to remove the row.
//!
//! **The test asserts the defect as it stands rather than the behaviour that
//! would be correct**, because a test asserting the fix would fail on `main` and
//! this file is not the change that fixes it. It is written so that fixing the
//! encoder flips it loudly: the assertion names the current outcome and the
//! comment names what should replace it.
//!
//! **THIS TEST IS CURRENTLY THE ONLY RECORD THAT THE DEFECT EXISTS**, which is a
//! bad place for it to be: a test is a good place to reproduce a known bug and a
//! bad place to be the sole evidence of one, because deleting or rewriting the
//! test loses the knowledge with it. Checked while writing this: the repository
//! has no issues at all (`gh issue list --state all` is empty), and `docs/PLAN.md`
//! does not mention the asymmetry. **Whoever files it should replace this
//! paragraph with the pointer.** Until then the search terms are
//! `Op::canonical_bytes` and `MAX_FIELD_LEN`.

use dialectica_core::arrival::{Arrival, MessageId};
use dialectica_core::authoring;
use dialectica_core::feed::{self, FeedPage, FeedRow};
use dialectica_core::identity::{Address, PublicKey, SecretKey};
use dialectica_core::identity_store::IdentityStore;
use dialectica_core::keystore::{Keystore, Unlock};
use dialectica_core::log::sqlite::LAYOUT_VERSION;
use dialectica_core::log::{Appended, Entry, OpLog, OpLogError, SqliteOpLog};
use dialectica_core::membership::{Membership, MembershipStore};
use dialectica_core::moderation::{self, Moderation, Moderators};
use dialectica_core::op::{ModerationAction, Op, OpId, OpKind, SignedOp, VoteDirection};
use dialectica_core::revision::current_version;
use dialectica_core::stoa::{Genesis, GenesisError, Policy};
use dialectica_core::wire;
use std::path::{Path, PathBuf};

// ─── Caps this crate keeps private, hardcoded here ────────────────────────

/// `op.rs`'s per-field decode cap, from §4.4's 150 KiB SDS message limit.
///
/// **Hardcoded because an integration test cannot see a private `const` at all.**
/// That is the whole reason, and it is enough of one — the answer is about
/// **reach**, not about strength.
///
/// It is specifically NOT that this file is the only guard against a drifted cap.
/// `op.rs::the_field_cap_is_pinned_to_a_known_answer` asserts
/// `MAX_FIELD_LEN == 150 * 1024` and `stoa.rs` asserts `MAX_TITLE_BYTES == 1024`,
/// both hardcoded, both for exactly that purpose, so a drifted cap has two other
/// tests to argue with before it reaches this one.
///
/// **Do not update this to match the code.** If they disagree, one of them is a
/// bug and this file is the half that is not allowed to blink.
const FIELD_CAP: usize = 153_600;

/// `stoa.rs`'s genesis title cap. Hardcoded for the reason `FIELD_CAP` is.
const TITLE_CAP: usize = 1024;

// ─── Fixtures ─────────────────────────────────────────────────────────────

/// A temporary directory that removes itself, named per test.
///
/// **The name carries a per-test tag AND 8 bytes of randomness**, and the two do
/// different jobs. The tag makes a directory identifiable while a test is
/// running — under `--nocapture` or a debugger, you can tell which test owns
/// which path. The randomness is what makes the name unguessable, and that is a
/// security property rather than a tidiness one: two of the tests below write an
/// **unencrypted Ed25519 root secret** into this directory. With a name any
/// local user could compute — the old form was
/// `dialectica-e2e-<pid>-<literal from the source>`, and a pid is readable from
/// `/proc` — an attacker could race the gap between the `remove_dir_all` and the
/// `create_dir_all` below and capture that secret, or make the test panic by
/// pre-placing a directory. Randomising removes the race rather than narrowing
/// it, and it also means the `remove_dir_all` cannot delete a caller-owned tree
/// that happened to collide (a recycled pid, an aborted run's leftovers).
///
/// This is the shape `keystore.rs`'s own in-crate `TempDir` already uses, for
/// the same reason. `getrandom` and `hex` are ordinary dependencies of this
/// crate, so no dev-dependency is added — matching that fixture's recorded
/// refusal to take `tempfile`.
///
/// **A failing test does NOT leave its directory behind**, and do not write that
/// it does: [`Drop::drop`] below calls `remove_dir_all` unconditionally, and a
/// panicking `#[test]` unwinds by default (no `panic = "abort"` profile is set),
/// so the cleanup runs on the failure path too. Only a `SIGKILL` leaves one.
///
/// Mode `0o700` is not tidiness, but the call it is load-bearing for is
/// **`Keystore::open`, not `Keystore::create`.** `create` checks only
/// `path.exists()` and then writes; the directory-mode check lives in
/// `read_checked`, which `open` and `is_encrypted` reach and `create` does not.
/// So removing the `set_permissions` call below does not break a `create` — it
/// breaks the *reopen* in both
/// `a_keystore_on_disk_signs_a_post_that_a_reopened_store_still_attributes_to_it`
/// and `the_same_keystore_posts_under_different_addresses_in_two_stoas`, which is
/// where to look when one of them starts refusing a keystore.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let mut suffix = [0u8; 8];
        getrandom::fill(&mut suffix).expect("the fixture's directory name needs randomness");
        let mut path = std::env::temp_dir();
        path.push(format!("dialectica-e2e-{name}-{}", hex::encode(suffix)));
        std::fs::create_dir_all(&path).expect("a temporary directory is creatable");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
                .expect("the temporary directory's mode is settable");
        }
        TempDir(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// The filename `store()` and `reopen()` use when a test needs only one.
    const CONVENTIONAL_STORE: &'static str = "ops.sqlite";

    /// A store at the conventional path in this directory.
    fn store(&self) -> SqliteOpLog {
        self.store_at(Self::CONVENTIONAL_STORE)
    }

    /// Drop a store and reopen the same path — **this is what "a restart" means
    /// here.** Not a flag, not a method on the store: the connection goes away
    /// and the file is opened again from scratch, which is the only thing that
    /// distinguishes a claim about a file from a claim about process memory.
    fn reopen(&self, store: SqliteOpLog) -> SqliteOpLog {
        self.reopen_at(store, Self::CONVENTIONAL_STORE)
    }

    /// A store at a NAMED path in this directory.
    ///
    /// **The filename is a parameter so that a test needing two stores in one
    /// directory does not have to hand-roll the primitive.** It used to be
    /// hardcoded, and the one test that needed two — the over-cap pair, which
    /// wants an at-cap store and an over-cap store side by side — therefore
    /// wrote its own `SqliteOpLog::open` and `drop` inline. That is a second,
    /// divergent copy of the restart primitive the doc above calls load-bearing,
    /// and this crate has already paid for leaving a weaker copy in place: it
    /// becomes the template the next test is written against. The publish path
    /// and a two-peer membership test will both want two stores.
    fn store_at(&self, name: &str) -> SqliteOpLog {
        SqliteOpLog::open(&self.file(name)).expect("a fresh store opens")
    }

    /// Restart a NAMED store: the same dropped-and-reopened primitive as
    /// [`Self::reopen`], for a directory holding more than one.
    fn reopen_at(&self, store: SqliteOpLog, name: &str) -> SqliteOpLog {
        drop(store);
        SqliteOpLog::open(&self.file(name)).expect("an existing store reopens")
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The fixture's own unpredictability, pinned — **this is a test ABOUT the
/// fixture**, which is why it sits here rather than under a boundary section.
///
/// Two tests below write an unencrypted root secret into a `TempDir`, so the
/// directory name being unguessable is a security property of this file and not
/// a detail of it. Nothing else in the suite would notice if the randomness
/// stopped arriving: every test would still pass against a fully predictable
/// name, which is exactly how the predictable form survived review in the first
/// place. So the property gets an assertion of its own.
///
/// **Two `TempDir`s built with the SAME tag** — the rival explanation this
/// excludes is that the paths differ because the tags differ, which is what a
/// two-different-tags fixture would prove instead, and which holds with no
/// randomness at all.
///
/// The expected length is derived by hand from the format rather than from the
/// code: `dialectica-e2e-` (15) + the tag + `-` (1) + 8 bytes as hex (16). With
/// the tag `"x"` that is 15 + 1 + 1 + 16 = **33**. A `getrandom::fill` that
/// silently wrote nothing would keep the length and lose the difference; a
/// dropped `hex::encode` would keep the difference and lose the length.
#[test]
fn two_temp_dirs_with_the_same_tag_get_different_unguessable_names() {
    let a = TempDir::new("x");
    let b = TempDir::new("x");

    assert_ne!(
        a.0, b.0,
        "the same tag must not produce the same path, or the name is predictable"
    );

    for dir in [&a, &b] {
        let name = dir
            .0
            .file_name()
            .expect("a temp directory has a final component")
            .to_str()
            .expect("the name is ASCII by construction");
        assert_eq!(
            name.len(),
            33,
            "the name must carry the tag AND 16 hex characters of randomness: {name}"
        );
        assert!(
            name.starts_with("dialectica-e2e-x-"),
            "the tag must still be readable in the name, for identifying a running test: {name}"
        );
        let random_half = &name["dialectica-e2e-x-".len()..];
        assert!(
            random_half.chars().all(|c| c.is_ascii_hexdigit()),
            "the suffix must be hex, not a pid or a counter: {random_half}"
        );
        assert_ne!(
            random_half, "0000000000000000",
            "an all-zero suffix is what a no-op `getrandom::fill` leaves behind"
        );
    }
}

/// A deterministic signing identity.
///
/// `from_bytes` over a fixed seed rather than `generate()`, so a failure is
/// reproducible and so two identities in one test are distinguishable by name
/// rather than by luck. Every 32-byte string is a valid Ed25519 seed, so this is
/// infallible in substance and the `expect` is a formality.
fn a_key(seed: u8) -> SecretKey {
    SecretKey::from_bytes(&[seed; 32]).expect("every 32-byte string is a valid Ed25519 seed")
}

/// A Stoa's genesis record, with `creator` as its sole moderator.
fn a_genesis(creator: &PublicKey, title: &str) -> Genesis {
    Genesis {
        creator: creator.clone(),
        policy: Policy::Open,
        title: title.to_string(),
    }
}

/// A thread head: a `Post` with no parent.
fn a_post(stoa: &Address, author: &SecretKey, body: &str) -> SignedOp {
    Op {
        stoa: *stoa,
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

/// A reply: a `Post` naming a parent.
fn a_reply(stoa: &Address, author: &SecretKey, parent: &OpId, body: &str) -> SignedOp {
    Op {
        stoa: *stoa,
        author: author.public_key(),
        kind: OpKind::Post {
            thread: Some(*parent),
            parent: Some(*parent),
            body: body.to_string(),
            attachments: vec![],
        },
    }
    .sign(author)
}

fn a_moderation(
    stoa: &Address,
    author: &SecretKey,
    target: &OpId,
    action: ModerationAction,
) -> SignedOp {
    Op {
        stoa: *stoa,
        author: author.public_key(),
        kind: OpKind::Moderate {
            target: *target,
            action,
        },
    }
    .sign(author)
}

/// A `Post` signed by one key and **attributed to another** — a forgery.
///
/// Built by signing with `signer` after setting `author` to somebody else's key,
/// which is exactly what a hostile peer does. The log stores it deliberately
/// (§3.3), so this is the fixture that proves the reader is the thing refusing
/// it.
fn a_forged_post(stoa: &Address, claimed: &PublicKey, signer: &SecretKey, body: &str) -> SignedOp {
    Op {
        stoa: *stoa,
        author: claimed.clone(),
        kind: OpKind::Post {
            thread: None,
            parent: None,
            body: body.to_string(),
            attachments: vec![],
        },
    }
    .sign(signer)
}

/// The bodies a feed page renders, in order. The shape most assertions compare.
fn bodies(page: &FeedPage) -> Vec<&str> {
    page.items.iter().map(|r| r.body.text.as_str()).collect()
}

// ─── The whole chain: a keystore file, a store file, and a restart ─────────

#[test]
fn a_keystore_on_disk_signs_a_post_that_a_reopened_store_still_attributes_to_it() {
    // THE END-TO-END CASE, and the one that crosses the most boundaries: a
    // keystore file on disk mints the identity, the derived key signs a post, the
    // post goes into a SQLite file, both connections are dropped, and both files
    // are reopened from their paths before anything is asserted.
    //
    // The rival explanation this fixture excludes: that the author address came
    // out of the row we just wrote. It does not — the expected address is
    // re-derived from the keystore REOPENED FROM DISK, so the assertion compares
    // two independent paths to the same value. A store that stashed the address
    // as a column and handed it back would pass a weaker version of this test and
    // fail this one, because the keystore half would still have to agree.
    let dir = TempDir::new("keystore-to-feed");
    let key_path = dir.file("identity.key");

    // Mint and write the keystore. Nothing is read back from this handle.
    let minted = Keystore::generate().expect("the test host has randomness");
    minted
        .create(&key_path, &Unlock::Unencrypted)
        .expect("a keystore is creatable in a 0700 directory");

    // The Stoa's genesis record. Its creator is a separate identity, so that
    // "the author" and "the moderator" are not the same key by accident — the
    // commonest way a moderation test passes for the wrong reason.
    let founder = a_key(1);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    // The address is the hash of the record, so re-deriving it independently is a
    // real check rather than a restatement: `Genesis::address` could have returned
    // any 32 bytes and this would catch it.
    assert_eq!(
        stoa,
        dialectica_core::identity::stoa_address(
            &genesis.canonical_bytes().expect("a short title encodes")
        ),
        "a Stoa's address must be the hash of its genesis record"
    );

    // Sign with the key derived for THIS Stoa, from the keystore we just wrote.
    let author_key = minted.stoa_key(&stoa);
    let post = a_post(&stoa, &author_key, "First");
    let post_id = post.op.id();

    let mut store = dir.store();
    assert_eq!(
        store.append(post, Arrival::unordered()),
        Ok(Appended::Stored),
        "a fresh op is newly stored"
    );

    // THE RESTART. Both handles go away; both files are reopened from their paths.
    let store = dir.reopen(store);
    drop(minted);
    let reopened_keystore =
        Keystore::open(&key_path, &Unlock::Unencrypted).expect("the keystore reopens unencrypted");

    // The expected author address, derived from the keystore that came back off
    // disk — NOT read from the feed row being asserted on.
    let expected_author = reopened_keystore.stoa_address(&stoa).to_hex();

    let moderators = Moderators::of(&genesis).expect("a genesis record yields its moderator set");
    let page = feed::list_threads(&store, &moderators, &stoa, 0, 20, false)
        .expect("a feed is readable from a reopened store");

    assert_eq!(
        bodies(&page),
        vec!["First"],
        "the post survives the restart"
    );
    assert_eq!(
        page.items[0].author, expected_author,
        "the surviving post is attributed to the address the reopened keystore derives"
    );
    // The thread id is the root post's op id, computed before the store ever saw
    // it. A store that re-derived an id from what it stored would differ here.
    assert_eq!(page.items[0].thread, post_id.to_hex());
    // Unrevised, so the rendered version IS the root. Asserting equality rather
    // than `is_some` is what would catch a resolver that returned a different op.
    assert_eq!(page.items[0].current_version, post_id.to_hex());
    assert!(!page.items[0].is_revised);
    assert!(!page.items[0].is_hidden);
}

#[test]
fn the_same_keystore_posts_under_different_addresses_in_two_stoas() {
    // §5.2's per-Stoa unlinkability, proved through the store rather than at the
    // derivation function. The rival explanation excluded: that the two feeds
    // differ because two different keystores wrote them. There is ONE keystore
    // here, reopened from one file, and it is asked for two Stoas' keys.
    //
    // Asserting the two addresses merely differ would be weak — two hashes differ
    // by default. So this also pins that each feed reports the address that
    // keystore derives FOR THAT STOA, which a derivation ignoring its Stoa
    // argument would fail.
    let dir = TempDir::new("per-stoa-address");
    let key_path = dir.file("identity.key");
    Keystore::generate()
        .expect("the test host has randomness")
        .create(&key_path, &Unlock::Unencrypted)
        .expect("a keystore is creatable");
    let ks = Keystore::open(&key_path, &Unlock::Unencrypted).expect("the keystore opens");

    let founder = a_key(1);
    let first = a_genesis(&founder.public_key(), "Agora");
    let second = a_genesis(&founder.public_key(), "Lyceum");
    let a = first.address().expect("a short title encodes");
    let b = second.address().expect("a short title encodes");
    assert_ne!(
        a, b,
        "two titles must give two Stoas, or this proves nothing"
    );

    let mut store = dir.store();
    store
        .append(
            a_post(&a, &ks.stoa_key(&a), "in Agora"),
            Arrival::unordered(),
        )
        .expect("a post is storable");
    store
        .append(
            a_post(&b, &ks.stoa_key(&b), "in Lyceum"),
            Arrival::unordered(),
        )
        .expect("a post is storable");
    let store = dir.reopen(store);

    let page_a = feed::list_threads(
        &store,
        &Moderators::of(&first).expect("moderators"),
        &a,
        0,
        20,
        false,
    )
    .expect("a feed is readable");
    let page_b = feed::list_threads(
        &store,
        &Moderators::of(&second).expect("moderators"),
        &b,
        0,
        20,
        false,
    )
    .expect("a feed is readable");

    assert_eq!(bodies(&page_a), vec!["in Agora"]);
    assert_eq!(bodies(&page_b), vec!["in Lyceum"]);

    // Each feed reports the address this keystore derives for that Stoa — the
    // independent derivation, not the row.
    assert_eq!(page_a.items[0].author, ks.stoa_address(&a).to_hex());
    assert_eq!(page_b.items[0].author, ks.stoa_address(&b).to_hex());
    assert_ne!(
        page_a.items[0].author, page_b.items[0].author,
        "one identity must present two addresses across two Stoas (§5.2)"
    );
}

// ─── Empty versus unreadable, at each boundary a read can fail at ──────────
//
// This is the project's defect family at its sharpest: an empty store and a
// broken one both produce an empty listing, so a test that only checks "the feed
// is empty" cannot tell them apart. Each pair below asserts the two outcomes are
// DIFFERENT KINDS of answer, not merely different values.

#[test]
fn a_missing_store_file_is_created_rather_than_refused() {
    // Split out from the empty-read test below, so that each name states ONE
    // falsifiable thing: creation is a claim about `open`, and the empty answers
    // are a claim about every read method. Joined, a failure named a test whose
    // other half was still true and the reader had to open the body to find out
    // which. `feed.rs` already splits the equivalent pair.
    let dir = TempDir::new("missing-file");
    let path = dir.file("ops.sqlite");
    assert!(!path.exists(), "the fixture must start with no file");

    let store = SqliteOpLog::open(&path).expect("a missing store is created, not refused");
    assert!(path.exists(), "opening a fresh path writes the file");
    // Created EMPTY rather than created-and-populated, which is the only other
    // thing "created" could mean.
    assert_eq!(store.len(), Ok(0));
}

#[test]
fn an_empty_store_answers_every_read_with_an_empty_answer_and_not_a_failure() {
    // The rival explanation excluded: that the reads returned empty because they
    // failed. They are asserted as `Ok`, and the error cases are the tests below.
    let dir = TempDir::new("empty-store");
    let store = dir.store();

    let founder = a_key(1);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    assert_eq!(store.len(), Ok(0));
    assert_eq!(store.is_empty(), Ok(true));
    assert_eq!(store.iter(), Ok(vec![]));
    assert_eq!(store.iter_stoa(&stoa), Ok(vec![]));
    // A target read over an op id nothing names. `from_hex` is the only public
    // route to an `OpId`, so the value is a literal rather than a hash of
    // anything the store produced.
    let absent = OpId::from_hex(&"7a".repeat(32)).expect("64 hex characters is an op id");
    assert_eq!(store.iter_target(&absent), Ok(vec![]));
    assert_eq!(store.get(&absent), Ok(None));

    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("an empty store yields an empty feed, not an error");
    assert_eq!(page.items, vec![]);
    assert!(!page.has_more, "there is no further page of nothing");
    assert_eq!(page.page, 0);
}

#[test]
fn a_store_written_by_another_layout_version_is_refused_naming_both_numbers() {
    // The empty-versus-unreadable pair's second half, at the OPEN boundary: this
    // store is not empty and not readable, and it says so as a DIFFERENT variant
    // from the mislabelled case below. A build that accepted any version would
    // open it and report an empty feed — indistinguishable from the test above.
    //
    // A NEGATIVE version, deliberately, rather than `LAYOUT_VERSION + 1`: a
    // negative number is guaranteed to be a version nothing will ever legitimately
    // stamp, so this fixture stays valid as `LAYOUT_VERSION` grows. A test phrased
    // as `+ 1` becomes a test of the NEXT version the moment the cap moves.
    let dir = TempDir::new("foreign-version");
    let path = dir.file("foreign.sqlite");
    let foreign_version: i32 = -7;

    let conn = rusqlite_stamp(&path, foreign_version, None);
    drop(conn);

    match SqliteOpLog::open(&path) {
        Err(OpLogError::UnknownLayoutVersion { found, expected }) => {
            assert_eq!(
                found, foreign_version,
                "the refusal names the version found"
            );
            // `expected` is the build's own number, so comparing it to
            // LAYOUT_VERSION is the one place reading the constant is right: the
            // claim is that the error reports the build's version, not that the
            // version is any particular value.
            assert_eq!(
                expected, LAYOUT_VERSION,
                "the refusal names the version this build understands"
            );
        }
        other => panic!("a foreign layout version must be refused as unknown, got {other:?}"),
    }
}

#[test]
fn a_store_stamping_our_layout_without_our_tables_is_refused_as_mislabelled() {
    // The third boundary, and the one the discarded suite could NOT reach from a
    // test target — it recorded that building such a file needed the private
    // schema DDL. It does not: the file only has to carry our version number and
    // NOT our table, and `rusqlite` is a dependency of this crate so a test can
    // write one directly.
    //
    // The distinction being pinned is the whole point: a store claiming our
    // layout and not having it is `LayoutDoesNotMatchItsVersion`, NOT
    // `UnknownLayoutVersion` (that is a store with somebody else's honest number)
    // and NOT `Storage` (that would blame the disk, which is fine). Three
    // explanations, three variants — and a build without the layout check would
    // open this Ok and fail later as `Storage("no such table: ops")`.
    let dir = TempDir::new("mislabelled");
    let path = dir.file("mislabelled.sqlite");

    // Our version, stamped over a table that is not ours.
    let conn = rusqlite_stamp(
        &path,
        LAYOUT_VERSION,
        Some("CREATE TABLE something_else (x INTEGER);"),
    );
    drop(conn);

    match SqliteOpLog::open(&path) {
        Err(OpLogError::LayoutDoesNotMatchItsVersion { version, why }) => {
            assert_eq!(
                version, LAYOUT_VERSION,
                "the refusal names the version the file claimed"
            );
            assert!(
                !why.is_empty(),
                "the refusal must say what was wrong, not merely that something was"
            );
        }
        other => panic!("a store claiming our layout without our tables must be refused as mislabelled, got {other:?}"),
    }
}

#[test]
fn a_store_that_is_not_a_database_is_a_storage_failure_and_not_an_empty_feed() {
    // The fourth way a read can fail, and the one most likely to be swallowed:
    // the path exists and holds bytes that are not SQLite at all. This must not
    // come back as an empty feed: an empty result and a failed one mean opposite
    // things, so they have to differ in KIND rather than in message.
    //
    // Paired with the empty-store test above: same API calls, same shape of
    // answer wanted, and the two must differ in KIND. That pairing is what makes
    // this more than an error-message test.
    let dir = TempDir::new("not-a-database");
    let path = dir.file("garbage.sqlite");
    // A SQLite file begins "SQLite format 3\0". These bytes deliberately do not,
    // and are long enough that the header check is what refuses them rather than
    // the file being too short to have a header at all.
    std::fs::write(
        &path,
        b"this is not a database, it is a text file\n".repeat(8),
    )
    .expect("a file is writable");

    let founder = a_key(1);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    // The failure may surface at open or at the first read depending on when
    // SQLite reads the header; either is correct, and what matters is that it is
    // a failure rather than an empty answer. So the assertion covers both routes
    // and refuses the one outcome that would be wrong.
    let outcome = SqliteOpLog::open(&path).and_then(|store| {
        feed::list_threads(
            &store,
            &Moderators::of(&genesis).expect("moderators"),
            &stoa,
            0,
            20,
            false,
        )
    });
    match outcome {
        Err(_) => {}
        Ok(page) => panic!(
            "a file that is not a database must fail rather than read as a quiet Stoa; \
             got a page of {} item(s)",
            page.items.len()
        ),
    }
}

/// Write a SQLite file carrying `version`, plus an optional statement batch.
///
/// A test helper rather than a fixture from the crate, because the crate's schema
/// DDL is private and this deliberately does not want it: the point is to build a
/// file the crate did NOT write.
fn rusqlite_stamp(path: &Path, version: i32, ddl: Option<&str>) -> rusqlite::Connection {
    let conn = rusqlite::Connection::open(path).expect("a database is creatable");
    if let Some(ddl) = ddl {
        conn.execute_batch(ddl).expect("the DDL applies");
    }
    conn.execute_batch(&format!("PRAGMA user_version = {version};"))
        .expect("the version is stampable");
    conn
}

// ─── What a reader refuses, with the store holding it anyway ────────────────

#[test]
fn a_forged_post_is_not_rendered_even_though_the_store_holds_it() {
    // §3.3's whole design in one test: the log stores junk, the reader refuses it.
    //
    // The rival explanation this fixture excludes: that the forgery is absent
    // from the feed because it was never stored. So the test asserts BOTH halves
    // against the SAME store — `iter_stoa` returns two entries and the feed
    // returns one. A test that only checked the feed would pass against a log
    // that had silently dropped the forgery on append, which is behaviour §3.3
    // specifically forbids.
    let dir = TempDir::new("forgery");
    let victim = a_key(1);
    let attacker = a_key(2);
    let genesis = a_genesis(&victim.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let mut store = dir.store();
    store
        .append(a_post(&stoa, &victim, "genuine"), Arrival::unordered())
        .expect("a genuine post is storable");
    // Attributed to the victim, signed by the attacker.
    store
        .append(
            a_forged_post(&stoa, &victim.public_key(), &attacker, "forged"),
            Arrival::unordered(),
        )
        .expect("the log stores a forgery deliberately");
    let store = dir.reopen(store);

    assert_eq!(
        store.iter_stoa(&stoa).expect("the store is readable").len(),
        2,
        "the LOG must hold both — it is not the log's job to filter (§3.3)"
    );

    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("a feed is readable");
    assert_eq!(
        bodies(&page),
        vec!["genuine"],
        "the READER must refuse the forgery"
    );
}

#[test]
fn a_reply_is_not_a_thread_head_in_the_feed() {
    // A `Post` with a parent is a reply, and the feed lists heads. The rival
    // explanation excluded: that the reply is missing because it failed to store
    // or failed to verify. Both are ruled out by asserting it IS in the store and
    // IS resolvable as a current version — it is present, valid, and correctly
    // not a head.
    let dir = TempDir::new("reply-not-head");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let root = a_post(&stoa, &author, "root");
    let root_id = root.op.id();
    let reply = a_reply(&stoa, &author, &root_id, "reply");
    let reply_id = reply.op.id();

    let mut store = dir.store();
    store.append(root, Arrival::unordered()).expect("storable");
    store.append(reply, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    assert_eq!(store.len(), Ok(2), "both ops are in the store");
    assert!(
        current_version(&store, &reply_id)
            .expect("the store is readable")
            .is_some(),
        "the reply is a valid, resolvable post — it is simply not a head"
    );

    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("a feed is readable");
    assert_eq!(
        bodies(&page),
        vec!["root"],
        "a feed lists thread heads, and a reply is not one"
    );
    // The head's thread id is its own op id, and the reply's is not rendered at
    // all. Naming both ids excludes the fixture where one is substituted for the
    // other and the count still comes to one.
    assert_eq!(page.items[0].thread, root_id.to_hex());
    assert_ne!(
        page.items[0].thread,
        reply_id.to_hex(),
        "the two ops must be distinguishable, or the count above proves nothing"
    );
}

// ─── Moderation across the store boundary ──────────────────────────────────

#[test]
fn a_hide_by_the_moderator_removes_a_thread_from_the_default_feed_and_marks_it_in_the_other() {
    // Moderation resolved on READ, from a reopened file. The fixture carries TWO
    // threads so that "hidden" is distinguishable from "the feed is empty" — the
    // defect family again: a resolver that hid everything, and one that hid the
    // right one, both produce a shorter list, and only a survivor tells them
    // apart.
    let dir = TempDir::new("hide");
    let founder = a_key(1);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let doomed = a_post(&stoa, &founder, "doomed");
    let doomed_id = doomed.op.id();
    let spared = a_post(&stoa, &founder, "spared");

    let mut store = dir.store();
    store
        .append(doomed, Arrival::unordered())
        .expect("storable");
    store
        .append(spared, Arrival::unordered())
        .expect("storable");
    store
        .append(
            a_moderation(&stoa, &founder, &doomed_id, ModerationAction::Hide),
            Arrival::unordered(),
        )
        .expect("storable");
    let store = dir.reopen(store);

    let moderators = Moderators::of(&genesis).expect("moderators");

    // The resolver's own answer, so that a feed change and a resolver change are
    // distinguishable rather than both showing up only as a shorter list.
    let resolved = moderation::resolve(&store, &moderators, &doomed_id).expect("readable");
    assert!(
        resolved.is_hidden(),
        "the founder is the sole moderator and hid this op"
    );
    assert!(
        resolved.deciding_op().is_some(),
        "a hide names the op that decided it"
    );

    let default = feed::list_threads(&store, &moderators, &stoa, 0, 20, false).expect("readable");
    assert_eq!(
        bodies(&default),
        vec!["spared"],
        "the default feed omits the hidden thread and keeps the other"
    );

    let shown = feed::list_threads(&store, &moderators, &stoa, 0, 20, true).expect("readable");
    // Both come back, and the hidden one is FLAGGED. §9.1: "a reader who asked to
    // see what was hidden is owed the knowledge of which ones those were." A
    // include_hidden that returned both unflagged would pass a count-only check.
    let mut shown_bodies = bodies(&shown);
    shown_bodies.sort_unstable();
    assert_eq!(shown_bodies, vec!["doomed", "spared"]);
    let doomed_row = shown
        .items
        .iter()
        .find(|r| r.body.text == "doomed")
        .expect("the hidden thread is in the include_hidden feed");
    let spared_row = shown
        .items
        .iter()
        .find(|r| r.body.text == "spared")
        .expect("the visible thread is too");
    assert!(doomed_row.is_hidden, "the hidden one is marked hidden");
    assert!(!spared_row.is_hidden, "the other is not");
}

#[test]
fn the_moderator_set_of_a_genesis_record_is_exactly_its_creator() {
    // `Moderators::contains` is public because a UI asks it to decide whether to
    // offer a hide button, so it is asserted in its own right rather than only as
    // a fixture guard in the tests below. A mutation making it answer `true`
    // unconditionally was caught ONLY by a guard assertion before this test
    // existed — and a guard aborts its test before the behaviour under test runs,
    // so the suite reported the right count for the wrong reason.
    //
    // Both directions are asserted, which is what a membership predicate needs: a
    // `contains` returning `true` always and one returning `false` always are
    // different bugs, and a one-sided test catches only one of them.
    let founder = a_key(1);
    let outsider = a_key(2);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let moderators = Moderators::of(&genesis).expect("moderators");

    assert!(
        moderators.contains(&founder.public_key()),
        "the creator named in the record is a moderator"
    );
    assert!(
        !moderators.contains(&outsider.public_key()),
        "nobody else is — the initial set is the creator alone, which is \
         `moderation-resolution`'s \"A Stoa's moderator set is derived from its \
         genesis record\""
    );
    // The set belongs to the Stoa the record addresses. Re-derived from the
    // record rather than read off the `Moderators`, so a `stoa()` returning
    // something else would be caught.
    assert_eq!(
        moderators.stoa(),
        &genesis.address().expect("a short title encodes"),
        "a moderator set is scoped to one Stoa"
    );
}

#[test]
fn a_hide_by_a_non_moderator_leaves_the_thread_visible() {
    // Authority, not merely signature validity. This hide is PROPERLY SIGNED by a
    // key that is simply not a moderator — so a resolver checking only `verify()`
    // would honour it. The rival explanation excluded: that the hide was ignored
    // because it was malformed or forged. It is neither; it is unauthorised.
    let dir = TempDir::new("unauthorised-hide");
    let founder = a_key(1);
    let rando = a_key(2);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let post = a_post(&stoa, &founder, "survives");
    let post_id = post.op.id();
    let hide = a_moderation(&stoa, &rando, &post_id, ModerationAction::Hide);
    // The hide is genuinely valid as a signature — that is what makes this test
    // about authority. Asserting it excludes the reading that it was skipped as a
    // forgery.
    assert!(
        hide.verify(),
        "the unauthorised hide must be validly signed, or this tests the wrong thing"
    );

    let mut store = dir.store();
    store.append(post, Arrival::unordered()).expect("storable");
    store.append(hide, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    let moderators = Moderators::of(&genesis).expect("moderators");
    assert_eq!(
        moderation::resolve(&store, &moderators, &post_id),
        Ok(Moderation::Unmoderated),
        "a hide from a non-moderator binds nothing"
    );

    let page = feed::list_threads(&store, &moderators, &stoa, 0, 20, false).expect("readable");
    assert_eq!(
        bodies(&page),
        vec!["survives"],
        "an unauthorised hide must not remove a thread"
    );

    // The fixture's premise, asserted AFTER the two claims above and not before
    // them — see note 5 in this file's header for the measurement that moved it,
    // and for why the same move is NOT right for the guard in
    // `a_forged_hide_does_not_displace…`. Kept rather than deleted: under a defect
    // in `Moderators::authorises` rather than in `contains`, the assertions above
    // still fire and this guard is the message that tells the two apart.
    assert!(
        !moderators.contains(&rando.public_key()),
        "the fixture's outsider must not be a moderator"
    );
}

#[test]
fn a_forged_hide_does_not_displace_the_genuine_one_that_sorts_after_it() {
    // The censorship-resistance case, and the reason `moderation::resolve` skips
    // non-binding ops rather than taking the leading moderation and then checking
    // it. A resolver doing the latter would report `Unmoderated` here — the
    // attacker's op sorts FIRST and is invalid, so checking-after-taking discards
    // the genuine hide behind it.
    //
    // The fixture's load-bearing part is the ORDER, and it is arranged rather
    // than hoped for: the forged op is given a higher Lamport timestamp, which
    // `cmp_ops` places first. Without that the test would pass or fail on a hash
    // coincidence — exactly the "hunted a shared byte" failure `log/mod.rs`'s own
    // test module records.
    let dir = TempDir::new("forged-hide-ordering");
    let founder = a_key(1);
    let attacker = a_key(2);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let post = a_post(&stoa, &founder, "hidden by a moderator");
    let post_id = post.op.id();

    // Signed by the attacker, attributed to the FOUNDER — so a resolver checking
    // `contains(author)` without `verify()` would honour it.
    let forged_unhide = Op {
        stoa,
        author: founder.public_key(),
        kind: OpKind::Moderate {
            target: post_id,
            action: ModerationAction::Unhide,
        },
    }
    .sign(&attacker);
    assert!(
        !forged_unhide.verify(),
        "the fixture's forgery must not verify, or it is not a forgery"
    );

    let genuine_hide = a_moderation(&stoa, &founder, &post_id, ModerationAction::Hide);

    let mut store = dir.store();
    store.append(post, Arrival::unordered()).expect("storable");
    // The forgery sorts FIRST: ordered ops lead, and among them the higher
    // Lamport value comes first.
    store
        .append(
            forged_unhide.clone(),
            Arrival::ordered(9, MessageId::new(vec![1])),
        )
        .expect("storable");
    store
        .append(
            genuine_hide.clone(),
            Arrival::ordered(1, MessageId::new(vec![2])),
        )
        .expect("storable");
    let store = dir.reopen(store);

    // Prove the ordering the test depends on, rather than assuming it. If this
    // ever stops holding, this test stops testing what it claims and says so here
    // instead of passing quietly.
    let about: Vec<OpId> = store
        .iter_target(&post_id)
        .expect("readable")
        .iter()
        .map(Entry::id)
        .collect();
    assert_eq!(
        about,
        vec![forged_unhide.op.id(), genuine_hide.op.id()],
        "the fixture requires the forgery to sort ahead of the genuine hide"
    );

    let moderators = Moderators::of(&genesis).expect("moderators");
    let resolved = moderation::resolve(&store, &moderators, &post_id).expect("readable");
    assert_eq!(
        resolved,
        Moderation::Hidden(
            store
                .get(&genuine_hide.op.id())
                .expect("readable")
                .expect("the genuine hide is stored")
        ),
        "a forgery sorting first must be skipped, not allowed to displace a genuine hide"
    );

    let page = feed::list_threads(&store, &moderators, &stoa, 0, 20, false).expect("readable");
    assert_eq!(
        bodies(&page),
        Vec::<&str>::new(),
        "the genuine hide still governs what is rendered"
    );
}

// ─── Persistence properties that are about a FILE ──────────────────────────

#[test]
fn a_stored_op_reads_back_byte_identical_across_a_restart() {
    // §3.3's byte-stability claim, against a file. The expected value is the
    // op's bytes taken BEFORE the store existed, so the comparison is against
    // something the store did not produce — the failure mode the project's
    // defect list names as `assert_eq!(bytes[0], VERSION_1)`, avoided by holding
    // a copy from outside.
    let dir = TempDir::new("byte-identical");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let op = a_post(&stoa, &author, "exactly these bytes");
    let expected_bytes = op.to_bytes();
    let expected_id = op.op.id();
    let expected_signature = op.signature.to_bytes();

    let mut store = dir.store();
    store.append(op, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    let back = store
        .get(&expected_id)
        .expect("readable")
        .expect("the op is in the store");
    assert_eq!(
        back.op.to_bytes(),
        expected_bytes,
        "the stored op must read back byte-identical"
    );
    assert_eq!(back.op.signature.to_bytes(), expected_signature);
    assert_eq!(back.id(), expected_id, "and its id is unchanged");
    assert!(back.op.verify(), "and it still verifies after a round trip");
}

#[test]
fn a_second_arrival_does_not_overwrite_the_first_ones_recorded_metadata() {
    // First-arrival-wins, across a restart. The rival explanation excluded: that
    // the second append was refused. It is not — it reports `AlreadyPresent`,
    // which is asserted, so the op WAS offered again and the metadata still did
    // not move.
    //
    // The two Lamport values are 1 and 9, and the SECOND is the higher, so a
    // richer-wins or last-wins implementation would show 9 here. A fixture whose
    // second value was lower would pass under both rules.
    let dir = TempDir::new("first-arrival-wins");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let op = a_post(&stoa, &author, "arrives twice");
    let id = op.op.id();

    let mut store = dir.store();
    let first = store.append(op.clone(), Arrival::ordered(1, MessageId::new(vec![0xAA])));
    let second = store.append(op, Arrival::ordered(9, MessageId::new(vec![0xBB])));
    let store = dir.reopen(store);

    // THE METADATA CLAIM IS ASSERTED FIRST, and the two `Appended` values after
    // it. That ordering is deliberate: an `INSERT OR REPLACE` mutation reports
    // `Stored` for the second append, and with the report asserted first the test
    // died there without ever checking the metadata — the substance of the
    // requirement went unexercised while the suite still showed one red test.
    // A test that reports the shallowest of its failures hides the rest.
    assert_eq!(store.len(), Ok(1), "one op id is one row");
    let back = store.get(&id).expect("readable").expect("stored");
    assert_eq!(
        back.arrival.lamport(),
        Some(1),
        "the FIRST arrival's Lamport value survives, not the higher second one"
    );
    assert_eq!(
        back.arrival.message_id().map(MessageId::as_bytes),
        Some(&[0xAA][..]),
        "and the first arrival's message id with it"
    );

    assert_eq!(first, Ok(Appended::Stored), "the first append stores");
    assert_eq!(
        second,
        Ok(Appended::AlreadyPresent),
        "a re-append is expected traffic, reported rather than refused"
    );
}

#[test]
fn two_stoas_whose_addresses_share_a_leading_byte_do_not_leak_into_each_other() {
    // A prefix-comparison bug in `iter_stoa` would pass a test using two
    // unrelated addresses, because unrelated addresses differ in byte 0. So this
    // CONSTRUCTS the shared prefix rather than hunting for titles that happen to
    // collide — the failure `log/mod.rs`'s test module records as having passed a
    // 2-byte prefix leak.
    //
    // `Address::from_bytes` takes a fixed array, so the two addresses here are
    // literals differing at exactly one byte. They are not derived from any
    // genesis record, which is fine: `iter_stoa` filters on the address in the
    // op, and nothing in this test reads a genesis.
    //
    // **16, and the number is not this file's to pick.**
    // `log::fixtures::SHARED_PREFIX_BYTES` is 16 with an argued rationale — far
    // past any plausible accidental truncation (a `substr(.., 8)`, a `u64` read
    // of the first 8 bytes, a hex-prefix comparison) while leaving 16 bytes for
    // the divergence to be unmistakable — and it explicitly rejects 31, on the
    // grounds that a fixture agreeing on 31 of 32 bytes tests the same property
    // and reads as a puzzle. An integration test cannot import that constant
    // (`log::fixtures` is `pub(crate)`), so the value is repeated here; what is
    // NOT repeated is a second, disagreeing rationale. If that constant moves,
    // this literal is the one to chase.
    const SHARED_PREFIX_BYTES: usize = 16;
    let dir = TempDir::new("shared-prefix");
    let author = a_key(1);
    let mut left_bytes = [0x5Au8; 32];
    let mut right_bytes = [0x5Au8; 32];
    // Diverge at exactly the first byte past the prefix, so the guarantee is
    // "agrees on N, differs at N" rather than "agrees on at least N somewhere".
    left_bytes[SHARED_PREFIX_BYTES] = 0x01;
    right_bytes[SHARED_PREFIX_BYTES] = 0x02;
    let left = Address::from_bytes(left_bytes);
    let right = Address::from_bytes(right_bytes);
    assert_eq!(
        left.as_bytes()[..SHARED_PREFIX_BYTES],
        right.as_bytes()[..SHARED_PREFIX_BYTES],
        "the fixture must share its whole declared prefix"
    );
    assert_ne!(
        left.as_bytes()[SHARED_PREFIX_BYTES],
        right.as_bytes()[SHARED_PREFIX_BYTES],
        "and must diverge immediately after it"
    );
    assert_ne!(left, right);

    let mut store = dir.store();
    store
        .append(a_post(&left, &author, "left"), Arrival::unordered())
        .expect("storable");
    store
        .append(a_post(&right, &author, "right"), Arrival::unordered())
        .expect("storable");
    let store = dir.reopen(store);

    let in_left: Vec<String> = store
        .iter_stoa(&left)
        .expect("readable")
        .iter()
        .map(|e| match &e.op.op.kind {
            OpKind::Post { body, .. } => body.clone(),
            other => panic!("the fixture stores only posts, got {other:?}"),
        })
        .collect();
    assert_eq!(
        in_left,
        vec!["left".to_string()],
        "a read restricted to one Stoa must match the COMPLETE address"
    );

    let in_right: Vec<String> = store
        .iter_stoa(&right)
        .expect("readable")
        .iter()
        .map(|e| match &e.op.op.kind {
            OpKind::Post { body, .. } => body.clone(),
            other => panic!("the fixture stores only posts, got {other:?}"),
        })
        .collect();
    assert_eq!(in_right, vec!["right".to_string()]);
}

/// A store on disk holding five distinguishable thread heads, after a restart.
///
/// Five and distinguishable BY NAME rather than by count, so a dropped or
/// duplicated row is visible as "post 3 is missing" rather than as "four, not
/// five" — a count alone is the fixture where two explanations give the same
/// answer. Returned after `reopen` so every paging claim below is a claim about
/// a file.
fn five_posts_on_disk(dir: &TempDir) -> (SqliteOpLog, Moderators, Address) {
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let mut store = dir.store();
    for n in 0..5u8 {
        store
            .append(
                a_post(&stoa, &author, &format!("post {n}")),
                Arrival::unordered(),
            )
            .expect("storable");
    }
    let store = dir.reopen(store);
    let moderators = Moderators::of(&genesis).expect("moderators");
    (store, moderators, stoa)
}

#[test]
fn the_pages_of_a_feed_tile_it_with_no_gap_and_no_repeat() {
    // Paging over a real file. The rival explanation excluded: that each page
    // looked right while the SET was wrong. Page 0 is asserted FULL and the two
    // pages are asserted to tile — every body appears exactly once across them —
    // so an off-by-one that dropped or duplicated a row fails here even though
    // each page's length would still look plausible.
    //
    // Split from the past-the-end test: tiling and the empty-page boundary are
    // two claims, and `feed.rs` splits the equivalent pair the same way.
    let dir = TempDir::new("paging-tile");
    let (store, moderators, stoa) = five_posts_on_disk(&dir);

    let first = feed::list_threads(&store, &moderators, &stoa, 0, 3, false).expect("readable");
    assert_eq!(first.items.len(), 3);
    assert_eq!(first.page, 0);
    assert!(first.has_more, "two of five remain after a page of three");

    let second = feed::list_threads(&store, &moderators, &stoa, 1, 3, false).expect("readable");
    assert_eq!(second.items.len(), 2);
    assert_eq!(second.page, 1);
    assert!(!second.has_more, "nothing follows the last two");

    let mut seen: Vec<&str> = bodies(&first);
    seen.extend(bodies(&second));
    seen.sort_unstable();
    assert_eq!(
        seen,
        vec!["post 0", "post 1", "post 2", "post 3", "post 4"],
        "the pages must tile the feed exactly — no gap, no repeat"
    );
}

#[test]
fn a_page_past_the_end_is_an_empty_page_rather_than_a_panic_or_a_wrapped_first_page() {
    // The case most likely to panic on a slice. The rival explanation excluded:
    // that the page came back empty because paging is broken rather than because
    // the feed ends — the tiling test above proves the same fixture pages
    // correctly, so an empty page 99 here is the boundary and not a breakage.
    //
    // `page` is asserted to be the page ASKED FOR: a wrapped read would return
    // page 0's rows, and a clamped one would report page 1.
    let dir = TempDir::new("paging-past-end");
    let (store, moderators, stoa) = five_posts_on_disk(&dir);

    // FIXTURE GUARD, and it is load-bearing rather than decorative: "page 99 is
    // empty" is also what a store holding NOTHING answers, so without this the
    // test passes against a fixture that persisted no rows at all. Making every
    // store in-memory is a mutation this file's table records, and this test
    // survived it until this assertion was added.
    let first = feed::list_threads(&store, &moderators, &stoa, 0, 3, false).expect("readable");
    assert_eq!(
        first.items.len(),
        3,
        "the fixture must hold rows for page 99 to be PAST anything"
    );

    let past = feed::list_threads(&store, &moderators, &stoa, 99, 3, false).expect("readable");
    assert_eq!(past.items, vec![], "a page past the end is empty");
    assert_eq!(past.page, 99, "and reports the page that was asked for");
    assert!(!past.has_more);
}

// ─── Hostile input reached from the public API ─────────────────────────────

#[test]
fn an_over_cap_body_signs_and_appends_and_then_poisons_every_read_forever() {
    // A LIVE CROSS-LAYER DEFECT, asserted as it stands. See this file's header.
    //
    // `Op::canonical_bytes` is infallible and writes a 4-byte length prefix for
    // any body; `Op::decode` refuses a field over 153,600 bytes. So one byte over
    // the cap signs, appends `Ok(Stored)`, and every subsequent ordered read of
    // the store fails `CorruptEntry` — across a restart, permanently, with no
    // public API able to remove the row. A peer that accepted such a body from
    // its own UI would brick every feed read it has.
    //
    // The AT-CAP half of the pair is what makes this a fencepost test rather than
    // an absurd-value test: exactly 153,600 must round-trip, so a cap tightened to
    // `>=` fails here. A test using only `u32::MAX` would pass under that
    // tightening, and `op.rs`'s own `the_field_cap_is_pinned_to_a_known_answer`
    // makes that argument at length: an absurd value is about 28,000x the cap, so
    // it proves *a* cap exists and nothing about *where* it is.
    let dir = TempDir::new("over-cap-body");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    // At the cap: encodes, stores, reads back, renders.
    //
    // Two stores in one directory, which is why `store_at`/`reopen_at` take a
    // filename: the at-cap store must stay READABLE while the over-cap store is
    // poisoned, so they cannot be the same file.
    {
        let at_cap = a_post(&stoa, &author, &"x".repeat(FIELD_CAP));
        let id = at_cap.op.id();
        let mut store = dir.store_at("at-cap.sqlite");
        store
            .append(at_cap, Arrival::unordered())
            .expect("storable");
        let store = dir.reopen_at(store, "at-cap.sqlite");
        let back = store
            .get(&id)
            .expect("a body of exactly the cap must read back")
            .expect("stored");
        match &back.op.op.kind {
            OpKind::Post { body, .. } => assert_eq!(
                body.len(),
                FIELD_CAP,
                "a field of exactly the cap survives the round trip"
            ),
            other => panic!("expected a post, got {other:?}"),
        }
        assert_eq!(
            feed::list_threads(
                &store,
                &Moderators::of(&genesis).expect("moderators"),
                &stoa,
                0,
                20,
                false
            )
            .expect("a feed over an at-cap body is readable")
            .items
            .len(),
            1
        );
    }

    // One byte over: signs, appends, and the store is unreadable from then on.
    let over = a_post(&stoa, &author, &"x".repeat(FIELD_CAP + 1));
    let over_id = over.op.id();
    // The encoder produced bytes the decoder refuses. Asserting both directions
    // is what names this as an ASYMMETRY rather than as a decoder limit: the
    // encode succeeded, so the two halves disagree.
    let encoded = over.to_bytes();
    assert!(
        encoded.len() > FIELD_CAP,
        "the fixture must actually exceed the cap"
    );
    assert!(
        SignedOp::from_bytes(&encoded).is_err(),
        "this peer's own encoder produced bytes its own decoder refuses — the defect"
    );

    let mut store = dir.store_at("over-cap.sqlite");
    assert_eq!(
        store.append(over, Arrival::unordered()),
        Ok(Appended::Stored),
        "the append SUCCEEDS, which is what makes this unrecoverable"
    );

    // Every ordered read now fails, before and after a restart.
    let store = dir.reopen_at(store, "over-cap.sqlite");
    match store.iter() {
        Err(OpLogError::CorruptEntry(_)) => {}
        other => panic!(
            "EXPECTED-DEFECT: an over-cap row must currently poison iter(); got {other:?}. \
             If the encoder now refuses an over-cap body, this test should be replaced by one \
             asserting the refusal happens BEFORE the append."
        ),
    }
    match store.iter_stoa(&stoa) {
        Err(OpLogError::CorruptEntry(_)) => {}
        other => panic!("EXPECTED-DEFECT: the Stoa read is poisoned too; got {other:?}"),
    }
    // `len` counts rows without decoding them, so the store still reports one op
    // it cannot hand back — which is what makes the row invisible to any repair a
    // caller could attempt through this API.
    assert_eq!(
        store.len(),
        Ok(1),
        "the row is counted but undecodable, and no public method can remove it"
    );
    assert!(
        matches!(store.get(&over_id), Err(OpLogError::CorruptEntry(_))),
        "not even a direct get can retrieve it"
    );
}

#[test]
fn an_over_cap_genesis_title_is_refused_before_it_can_name_a_stoa() {
    // The genesis record's cap, and the contrast that makes the defect above a
    // defect: here the encode is FALLIBLE, so an over-long title is refused at
    // `canonical_bytes` and never reaches an address. `Op::canonical_bytes` is
    // the one that is not.
    //
    // The at-cap half is the fencepost again: exactly 1024 must encode.
    let founder = a_key(1);

    let at_cap = a_genesis(&founder.public_key(), &"t".repeat(TITLE_CAP));
    assert!(
        at_cap.canonical_bytes().is_ok(),
        "a title of exactly the cap must encode"
    );
    assert!(
        at_cap.address().is_ok(),
        "and must therefore have an address"
    );

    let over = a_genesis(&founder.public_key(), &"t".repeat(TITLE_CAP + 1));
    assert_eq!(
        over.canonical_bytes(),
        Err(GenesisError::TitleTooLong(TITLE_CAP + 1)),
        "one byte over the cap is refused, naming the length found"
    );
    assert_eq!(
        over.address(),
        Err(GenesisError::TitleTooLong(TITLE_CAP + 1)),
        "so it never acquires an address"
    );
    assert_eq!(
        Moderators::of(&over),
        Err(GenesisError::TitleTooLong(TITLE_CAP + 1)),
        "and no moderator set can be built from it"
    );
}

#[test]
fn a_vote_is_stored_and_is_rendered_by_nothing() {
    // What a vote currently does, pinned so that the UI brief's claim — votes are
    // stored and read by nothing, so there is no score to render — is checkable
    // rather than asserted in prose.
    //
    // The rival explanation excluded: that the vote is absent from the feed
    // because it was not stored. `iter_target` is asserted to return it, so it IS
    // in the store and IS associated with the post; the feed simply does not
    // render it and no public API reports a count.
    //
    // NO SPEC: no live requirement in `openspec/specs/` says what a vote does to
    // a feed — `relevance-ordering` is an unarchived change. This pins today's
    // behaviour so that adding a score is a visible change rather than a silent
    // one.
    let dir = TempDir::new("vote");
    // Named `poster` rather than `author`, because the destructure below binds a
    // `FeedRow` field of that name and the two must stay tellable apart: the
    // whole point of the attribution assertion is that they are different keys.
    let poster = a_key(1);
    let voter = a_key(2);
    let genesis = a_genesis(&poster.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let post = a_post(&stoa, &poster, "voted on");
    let post_id = post.op.id();
    let vote = Op {
        stoa,
        author: voter.public_key(),
        kind: OpKind::Vote {
            target: post_id,
            direction: VoteDirection::Up,
        },
    }
    .sign(&voter);
    let vote_id = vote.op.id();

    let mut store = dir.store();
    store.append(post, Arrival::unordered()).expect("storable");
    store.append(vote, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    // The vote is in the store and names the post.
    assert_eq!(
        store
            .iter_target(&post_id)
            .expect("readable")
            .iter()
            .map(Entry::id)
            .collect::<Vec<_>>(),
        vec![vote_id],
        "a target read returns every op naming that target, whatever its kind"
    );

    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("readable");
    assert_eq!(
        bodies(&page),
        vec!["voted on"],
        "a vote is not a thread head and does not appear as a row"
    );
    // The row carries no score field at all — there is nothing for a view to
    // render.
    //
    // **Pinned by an exhaustive destructure, not by reading a field at a time.**
    // A review proved the earlier form of this could not do what its comment
    // claimed: it asserted three fields individually, and adding `pub score: i64`
    // to `FeedRow` left the test passing unchanged. `FeedRow` is not
    // `#[non_exhaustive]`, so a destructure naming every field is the one shape
    // that fails to COMPILE when a field is added — which is louder than a failed
    // assertion and cannot be skipped. Whoever adds a score has to come here and
    // decide what a view does with it.
    let FeedRow {
        thread,
        current_version,
        author,
        body,
        attachments,
        is_revised,
        is_hidden,
    } = &page.items[0];
    assert_eq!(
        thread, current_version,
        "an unedited post is its own version"
    );
    assert_eq!(
        thread,
        &post_id.to_hex(),
        "the thread is the root post, not the vote"
    );
    // Derived from the POST's author key, independently of anything the feed
    // returned — and asserted DIFFERENT from the voter's, which is the
    // attribution a vote-rendering bug would produce.
    assert_eq!(
        author,
        &poster.public_key().address().to_hex(),
        "the row is attributed to whoever posted it"
    );
    assert_ne!(
        author,
        &voter.public_key().address().to_hex(),
        "and never to whoever voted on it"
    );
    assert_eq!(body.text, "voted on");
    assert!(attachments.is_empty());
    assert!(!is_revised);
    assert!(!is_hidden);
}

#[test]
fn a_body_carrying_invisible_characters_is_sanitised_on_the_way_out_of_the_store() {
    // Sanitisation across the store boundary: the body goes onto disk verbatim
    // (it is inside the signature preimage and must not be altered) and comes out
    // of the FEED cleaned, with a count of what was removed.
    //
    // The rival explanation excluded: that the store altered the body. The stored
    // op is read back and asserted to hold the ORIGINAL bytes, so the change is
    // provably the feed's and not the store's. A test that only checked the feed
    // would pass if the store had silently rewritten the row — which would break
    // every signature.
    let dir = TempDir::new("sanitise");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    // A zero-width space (U+200B) between two visible words.
    let raw = "hello\u{200B}world";
    let post = a_post(&stoa, &author, raw);
    let id = post.op.id();
    let mut store = dir.store();
    store.append(post, Arrival::unordered()).expect("storable");
    let store = dir.reopen(store);

    // The STORE keeps the body exactly as signed.
    let back = store.get(&id).expect("readable").expect("stored");
    match &back.op.op.kind {
        OpKind::Post { body, .. } => assert_eq!(
            body, raw,
            "the stored body is inside the signature preimage and must be untouched"
        ),
        other => panic!("expected a post, got {other:?}"),
    }
    assert!(
        back.op.verify(),
        "and it still verifies, which it would not if the store had rewritten it"
    );

    // The FEED renders it cleaned, and says how much it removed.
    let page = feed::list_threads(
        &store,
        &Moderators::of(&genesis).expect("moderators"),
        &stoa,
        0,
        20,
        false,
    )
    .expect("readable");
    assert_eq!(
        page.items[0].body.text, "helloworld",
        "the invisible character is removed for display"
    );
    assert_eq!(
        page.items[0].body.removed, 1,
        "and counted, so a view can mark it"
    );
    assert!(!page.items[0].body.is_clean());
}

// ─── The JSON boundary: a request string in, a reply string out ─────────────
//
// The outermost boundary this crate has, and the one CLAUDE.md calls the
// deliverable: "the core module's API is the part of this project to be most
// deliberate about". `wire::list_threads_from_request` takes a JSON string and
// returns a JSON string, which is what a view actually calls — every test above
// stops one layer below it at `feed::list_threads`.
//
// That layer is not empty of behaviour, which is why stopping below it was a gap
// rather than a tidy boundary. `Err` and `{"error":...}` are two different
// things, and the mapping between them is exactly the encode/decode asymmetry
// class this file exists to catch: a handler that flattened a storage failure
// into an empty page would satisfy every assertion above and still hand a view
// a quiet, wrong answer.
//
// A REAL FILE on both tests, not a `MemoryOpLog`. The unit tests in `wire.rs`
// already cover the handler's parsing against an in-memory log; what they cannot
// do is prove the JSON a view receives reflects a store on disk, which is the
// only thing this section adds.

#[test]
fn a_request_naming_a_stoa_on_disk_comes_back_as_the_feed_in_json() {
    // The whole read path in one call: a JSON request string in, and a JSON reply
    // carrying the body that was signed and written to a file.
    //
    // The rival explanation excluded: that the handler echoed something from the
    // request. The body asserted for — "over the wire" — appears nowhere in the
    // request, which carries only the Stoa address and the genesis record. The
    // request is built from the genesis record's hex, so nothing here is taken
    // from a value the handler produced.
    let dir = TempDir::new("wire-happy");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let mut store = dir.store();
    store
        .append(
            a_post(&stoa, &author, "over the wire"),
            Arrival::unordered(),
        )
        .expect("storable");
    drop(store);

    // The request a view sends. Hex of the canonical record, which is how the
    // caller supplies the moderator set §4.8 makes self-authenticating.
    let request = format!(
        r#"{{"stoa":"{}","genesis":"{}"}}"#,
        stoa.to_hex(),
        hex::encode(genesis.canonical_bytes().expect("a short title encodes"))
    );

    // The store is opened INSIDE the handler, from the path, so the handler owns
    // the whole read including the open — which is what makes the failure test
    // below meaningful.
    let path = dir.file(TempDir::CONVENTIONAL_STORE);
    let reply = wire::list_threads_from_request(&request, || SqliteOpLog::open(&path));

    let v: serde_json::Value =
        serde_json::from_str(&reply).expect("every reply is valid JSON, whatever happened");
    assert!(
        v.get("error").is_none(),
        "a readable store must not produce the error shape, got {reply}"
    );
    // The pagination shape the ecosystem mandates, asserted as a shape rather
    // than only as a length.
    assert_eq!(v["items"].as_array().expect("items is an array").len(), 1);
    assert_eq!(v["items"][0]["body"]["text"], "over the wire");
    assert_eq!(v["page"], 0);
    assert_eq!(v["hasMore"], false);
    // The author reaches the view as the hex address and never as a name: names
    // are the view's to derive, and a name on the wire would be a second,
    // forgeable identifier beside the real one.
    assert_eq!(
        v["items"][0]["author"],
        serde_json::Value::String(author.public_key().address().to_hex())
    );
}

#[test]
fn the_json_envelope_reports_the_page_that_was_asked_for_and_whether_more_follows() {
    // THE ENVELOPE, over a store that can tell a real answer from a constant.
    //
    // The rival explanation excluded — and it is the one that got past review
    // here: that `page` and `hasMore` are ECHOED rather than computed. The happy
    // path above reads page 0 of a one-post store, where the correct answer is
    // `page: 0, hasMore: false` — which is exactly what a handler emitting fixed
    // literals writes. Both assertions there agree with a broken handler, and a
    // reviewer proved it: replacing `"page": page.page, "hasMore": page.has_more`
    // in `wire.rs` with `0` and `false` passed all 25 tests in this file.
    //
    // So this fixture is built so neither constant is the right answer:
    //
    //   * FIVE posts at `perPage: 2`, so the feed has three pages.
    //   * Read at page **1**, a non-zero index a hardcoded `0` cannot produce.
    //   * `hasMore` asserted **true** on that read (page 2 remains) and **false**
    //     on the last page, so a hardcoded `false` fails the first and a
    //     hardcoded `true` fails the second. One constant cannot satisfy both.
    //
    // `items` is asserted at each page too, because "reports page 1" is a claim
    // about the envelope and "serves page 1's rows" is a claim about the read —
    // an envelope that counted correctly while serving page 0's rows would pass
    // the first alone.
    let dir = TempDir::new("wire-pagination");
    let author = a_key(1);
    let genesis = a_genesis(&author.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    let mut store = dir.store();
    for n in 0..5u8 {
        store
            .append(
                a_post(&stoa, &author, &format!("post {n}")),
                Arrival::unordered(),
            )
            .expect("storable");
    }
    drop(store);

    let path = dir.file(TempDir::CONVENTIONAL_STORE);
    let genesis_hex = hex::encode(genesis.canonical_bytes().expect("a short title encodes"));
    let request = |page: usize| {
        format!(
            r#"{{"stoa":"{}","genesis":"{genesis_hex}","page":{page},"perPage":2}}"#,
            stoa.to_hex()
        )
    };
    let read = |page: usize| -> serde_json::Value {
        let reply = wire::list_threads_from_request(&request(page), || SqliteOpLog::open(&path));
        serde_json::from_str(&reply).expect("every reply is valid JSON, whatever happened")
    };

    // Expected lengths derived by hand from five rows at two per page, NOT read
    // back from the store: pages 0 and 1 hold two each, page 2 holds the fifth,
    // and only the last has nothing after it.
    let cases = [(0usize, 2usize, true), (1, 2, true), (2, 1, false)];
    for (page, expected_len, expected_more) in cases {
        let v = read(page);
        assert!(
            v.get("error").is_none(),
            "page {page} of a readable store must not be the error shape, got {v}"
        );
        assert_eq!(
            v["page"],
            serde_json::json!(page),
            "the envelope must report the page ASKED FOR, not a constant: page {page} gave {v}"
        );
        assert_eq!(
            v["hasMore"],
            serde_json::json!(expected_more),
            "hasMore must be computed from what remains: page {page} gave {v}"
        );
        assert_eq!(
            v["items"].as_array().expect("items is an array").len(),
            expected_len,
            "and the rows must be that page's rows: page {page} gave {v}"
        );
    }

    // The three pages must also TILE, so "page 1 reported 1" cannot be satisfied
    // by an envelope counting correctly over page 0's rows three times over.
    let mut seen: Vec<String> = (0..3)
        .flat_map(|page| {
            read(page)["items"]
                .as_array()
                .expect("items is an array")
                .iter()
                .map(|row| {
                    row["body"]["text"]
                        .as_str()
                        .expect("a body is a string")
                        .to_string()
                })
                .collect::<Vec<_>>()
        })
        .collect();
    seen.sort_unstable();
    assert_eq!(
        seen,
        vec!["post 0", "post 1", "post 2", "post 3", "post 4"],
        "the JSON pages must tile the feed exactly — no gap, no repeat"
    );
}

#[test]
fn a_store_on_disk_that_is_not_a_database_reaches_the_view_as_the_error_shape() {
    // THE SEAM THIS SECTION EXISTS FOR, and the one a test at `feed::list_threads`
    // can only half-prove. `a_store_that_is_not_a_database_is_a_storage_failure…`
    // above shows the read returns `Err`; it cannot show what a view receives,
    // because `Err` is not a JSON reply. The obligation is about what the READER
    // sees — `docs/UI-BRIEF.md`'s "Distinguish an empty result from a failed one",
    // which says in terms that a storage failure must never render as an empty
    // feed — so it is only discharged at the layer that produces what the reader
    // is shown.
    //
    // **Cited by its heading and NOT by its number**, deliberately. An earlier
    // version of this comment cited "§11.1 obligation 5"; `docs/PLAN.md` says at
    // §11's head that §11.1 arrives with the `vouching-state` change and is
    // absent until then, so the section was a phantom. The number 5 was real —
    // but it is UI-BRIEF's, and UI-BRIEF restarts its numbering per section and
    // contains a `2b`, so a bare ordinal does not locate anything there either.
    // The quoted heading is unique; grep for it.
    //
    // The promoted half of the same rule is `module-wire-contract`'s "Failure is
    // always the error shape, and never a partial success", which is what the
    // assertions below actually check; UI-BRIEF says why a view cannot recover
    // from getting it wrong.
    //
    // Same fixture as that test deliberately: identical bytes on disk, one layer
    // further out, so the pair shows the failure surviving the JSON crossing
    // rather than being flattened by it.
    let dir = TempDir::new("wire-not-a-database");
    let path = dir.file("garbage.sqlite");
    std::fs::write(
        &path,
        b"this is not a database, it is a text file\n".repeat(8),
    )
    .expect("a file is writable");

    let founder = a_key(1);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");
    let request = format!(
        r#"{{"stoa":"{}","genesis":"{}"}}"#,
        stoa.to_hex(),
        hex::encode(genesis.canonical_bytes().expect("a short title encodes"))
    );

    let reply = wire::list_threads_from_request(&request, || SqliteOpLog::open(&path));

    let v: serde_json::Value = serde_json::from_str(&reply).expect("a failure is still valid JSON");
    // The three assertions are one claim each, and the middle one is the
    // load-bearing one: a handler that returned `{"items":[],...}` here would
    // satisfy "valid JSON" and "no panic" while telling the view the Stoa is
    // empty. §2.5 makes the error shape EXCLUSIVE — never a partial success.
    assert!(
        v.get("error").is_some(),
        "an unreadable store must reach the view as an error, got {reply}"
    );
    assert!(
        v.get("items").is_none(),
        "and must NOT carry an items array beside it — the shapes are exclusive, got {reply}"
    );
    assert!(
        !v["error"].as_str().expect("error is a string").is_empty(),
        "the error must say something, not merely exist"
    );
}

// ─── The publish path, and the seeding sequence built on it ─────────────────
//
// **A new section under rule 2**: every test above appends a hand-built `SignedOp`
// directly, which is what the store sees and not what a caller does. These cross
// `authoring::post`/`reply`/`vote` — the public write entry point the file's
// preamble named as "the one known to be coming".
//
// **Why these two live here rather than in `examples/seed_store.rs`.** The seeder
// asserts these same properties inline and `cargo test` compiles that file without
// ever running it, so every assertion in it is behind "a person typed `cargo run
// --example`". That blind spot is not hypothetical: it is how a report line
// claiming "every seeded op is by ⟨one address⟩" went false for five of nine ops
// with every check in the file still green
// (`openspec/changes/seed-store/findings/spec-test.md`, the two `tester` boxes).
// A `#[test]` may never be added to that example — CI's count gate walks
// `examples/` and cargo does not run an example's tests, so one declared there
// makes `declared` exceed `ran` — so the property has to be re-established here,
// by a target the same `cargo test` invocation executes.
//
// These do not duplicate the seeder: they assert the properties, where it also
// prints a report. If the seeder's fixture changes, these keep holding; if the
// *publish path* stops distinguishing authors or stops chaining replies, both
// die here, in CI, without anybody running a binary.

/// The two-identity structure a seeded store must have, as a SET over every op.
///
/// **Existential is not universal, and that is the whole reason this test
/// exists.** The seeder's neighbouring checks are `authors.contains(founder)` and
/// `authors.contains(visitor)` — each says "at least one op has this author", and
/// no number of them can contradict a claim about *every* op. So a store with one
/// author and a store with two both satisfy them: this repo's recorded family, a
/// fixture where two explanations give the same answer.
///
/// The rival explanations this fixture excludes, one assertion each:
///
/// - **"the store collapsed to a single author"** — excluded by asserting the set
///   has two members, which a `contains` pair cannot do;
/// - **"a third identity leaked in"** — excluded by the same assertion, since the
///   set is compared for equality rather than for containment;
/// - **"the authors are whatever the ops say they are"** — excluded because the
///   expected pair is derived from the two SECRET KEYS, through
///   `public_key().address()`, never read out of an op. A publish path that
///   stamped a constant author, or the signer's key on somebody else's op, would
///   satisfy a self-referential version of this and fail this one.
///
/// **Both authors sign more than one KIND of op**, so the pair cannot be satisfied
/// by a path that attributes roots correctly and votes to whoever is handy: each
/// identity here signs a root, a reply and a vote.
#[test]
fn a_store_seeded_from_two_identities_carries_exactly_those_two_authors() {
    let dir = TempDir::new("two-authors");

    // The two identities, from fixed seeds so a failure is reproducible.
    let founder = a_key(1);
    let visitor = a_key(2);
    let genesis = a_genesis(&founder.public_key(), "Agora");
    let stoa = genesis.address().expect("a short title encodes");

    // The expected pair, derived from the KEYS rather than from any op. This is
    // the operand the implementation did not produce.
    let mut expected = vec![
        founder.public_key().address().to_hex(),
        visitor.public_key().address().to_hex(),
    ];
    expected.sort();
    assert_ne!(
        expected[0], expected[1],
        "two seeds must give two identities, or the set assertion proves nothing"
    );

    let mut store = dir.store();

    // One root each, so authorship is visible in the FEED and not only in the log.
    let founders_root = authoring::post(
        &mut store,
        &founder,
        stoa,
        "What does it mean for a forum to be decentralized?".to_string(),
    )
    .expect("a root post is publishable");
    let visitors_root = authoring::post(
        &mut store,
        &visitor,
        stoa,
        "On the difference between moderation and censorship".to_string(),
    )
    .expect("a root post is publishable");

    // A reply each, crossing identities in both directions.
    authoring::reply(
        &mut store,
        &visitor,
        stoa,
        founders_root.id,
        "That it has no single party who can switch it off.".to_string(),
    )
    .expect("a reply is publishable");
    authoring::reply(
        &mut store,
        &founder,
        stoa,
        visitors_root.id,
        "One is a Stoa deciding what it is.".to_string(),
    )
    .expect("a reply is publishable");

    // A vote each, so neither identity is present only as a poster.
    authoring::vote(
        &mut store,
        &visitor,
        stoa,
        founders_root.id,
        VoteDirection::Up,
    )
    .expect("a vote is publishable");
    authoring::vote(
        &mut store,
        &founder,
        stoa,
        visitors_root.id,
        VoteDirection::Down,
    )
    .expect("a vote is publishable");

    // THE RESTART, so this is a claim about the file and not about the handle
    // that wrote it.
    let store = dir.reopen(store);

    // Fixture guard, AFTER nothing it blocks: it is the count the set assertion is
    // taken over, and a store holding fewer ops would make a two-member set a
    // weaker claim than it reads as. Note 5's rule applies — the mutation this
    // test is aimed at is one that changes ATTRIBUTION, which cannot change the
    // count, so this guard cannot fire in front of the assertion it guards.
    assert_eq!(
        store.len(),
        Ok(6),
        "two roots, two replies and two votes is six ops"
    );

    let mut authors: Vec<String> = store
        .iter()
        .expect("the store is readable")
        .iter()
        .map(|e| e.op.op.author.address().to_hex())
        .collect();
    authors.sort();
    authors.dedup();

    assert_eq!(
        authors, expected,
        "the store must carry exactly the two identities that signed it — no \
         collapse to one, no third author, and each address as its own key derives it"
    );
}

/// The seeding sequence a developer tool performs, as an integration test.
///
/// Mint a keystore on disk, found a Stoa from it, record a chosen path, publish a
/// nested thread through the public write path, restart, and read it back. This is
/// `tasks.md`'s "the shape worth considering" for this piece, and it covers what
/// the example's nine inline assertions cover without needing anybody to run a
/// binary.
///
/// Three claims, each excluding a rival explanation:
///
/// 1. **The reply chain is a tree and not a flat list.** Asserted at THREE levels,
///    because at two "the parent's id" and "the parent's thread" are the same
///    value — a `thread: parent` bug is invisible in a two-level fixture, which is
///    the same-answer family again. `nested`'s parent is the reply and its thread
///    is the ROOT's, so the two fields carry different values and a reader that
///    confused them fails here.
/// 2. **The feed attributes the root to the key that signed it**, derived from the
///    keystore reopened from disk rather than read from the row.
/// 3. **The probe and the publish path disagree about which identity this user
///    posts under** — the three-derivations gap. Asserted as the state of the
///    world TODAY, self-invalidatingly: the operands are `wire::posting_identity`'s
///    own answer and the address the feed actually carries, both produced by the
///    module, so closing the gap anywhere makes this fail and name itself.
///
/// **Claim 3 is why this is not just the example re-typed.** The example asserts
/// the same inequality, and for a while asserted it between two keystore
/// derivations it made itself — two HD paths off one root, which differ for the
/// reason any two do, so closing the real gap left it green (`findings/spec-test.md`
/// entry 1). Here the right operand is the author the STORE reports through
/// `feed::list_threads`, so neither side is a derivation this test performed.
#[test]
fn the_seeding_sequence_builds_a_nested_thread_whose_author_the_probe_does_not_report() {
    let dir = TempDir::new("seeding-sequence");

    // ── A keystore on disk, as the seeder mints one ──
    let key_path = dir.file("identity.key");
    Keystore::generate()
        .expect("the test host has randomness")
        .create(&key_path, &Unlock::Unencrypted)
        .expect("a keystore is creatable in a 0700 directory");
    let keystore =
        Keystore::open(&key_path, &Unlock::Unencrypted).expect("the keystore reopens unencrypted");

    // ── The Stoa, founded on `identity_public_key` as `createStoa` names it ──
    let genesis = a_genesis(&keystore.identity_public_key(), "Ἀγορά");
    let stoa = genesis.address().expect("a short title encodes");

    // ── Membership, through the only constructor `join` accepts ──
    let mut memberships =
        MembershipStore::open(&dir.file("memberships.sqlite")).expect("a membership store opens");
    memberships
        .join(&Membership::verified(&stoa, &genesis).expect("the record matches its address"))
        .expect("a membership is recordable");
    assert_eq!(
        memberships.contains(&stoa),
        Ok(true),
        "the Stoa this peer founded must be one it has joined"
    );

    // ── The chosen path, so the probe has one to read ──
    //
    // Without this row `posting_identity` answers `NO_CHOICE_FOR_THIS_STOA` and
    // claim 3 below would be comparing against an error rather than an address.
    let seeded_path: u32 = 0;
    let paths = IdentityStore::open(&IdentityStore::default_path_in(&dir.0))
        .expect("an identity record opens");
    paths
        .record_path(&stoa, seeded_path)
        .expect("a chosen path is recordable");

    // ── A nested thread, through the publish path ──
    //
    // The founder signs with `stoa_key`, which is what the module's publish path
    // signs with. A visitor with no keystore behind it stands in for an op that
    // arrived from another peer.
    let founder = keystore.stoa_key(&stoa);
    let visitor = a_key(9);

    let mut store = dir.store();
    let root = authoring::post(&mut store, &founder, stoa, "root".to_string())
        .expect("a root post is publishable");
    let reply = authoring::reply(&mut store, &visitor, stoa, root.id, "reply".to_string())
        .expect("a reply is publishable");
    let nested = authoring::reply(&mut store, &founder, stoa, reply.id, "nested".to_string())
        .expect("a reply to a reply is publishable");

    let store = dir.reopen(store);

    // ── Claim 1: the chain is a tree ──
    //
    // Read back through `OpLog::get`, so these are the fields the STORE holds and
    // not the `Published` values the calls handed back.
    for (what, id, expected_parent, expected_thread) in [
        ("reply", reply.id, root.id, root.id),
        // The discriminating row: parent and thread are DIFFERENT values here.
        ("nested", nested.id, reply.id, root.id),
    ] {
        let entry = store
            .get(&id)
            .expect("the store is readable")
            .unwrap_or_else(|| panic!("the store must hold {what}"));
        match &entry.op.op.kind {
            OpKind::Post { parent, thread, .. } => {
                assert_eq!(
                    *parent,
                    Some(expected_parent),
                    "{what} must name its parent"
                );
                assert_eq!(
                    *thread,
                    Some(expected_thread),
                    "{what} must belong to the root's thread, not its parent's id"
                );
            }
            other => panic!("{what} must be a post, got {other:?}"),
        }
    }

    // ── Claim 2: the feed attributes the root to the signing key ──
    let moderators = Moderators::of(&genesis).expect("a genesis record yields its moderator set");
    let page = feed::list_threads(&store, &moderators, &stoa, 0, 20, false)
        .expect("a feed is readable from a reopened store");
    assert_eq!(
        bodies(&page),
        vec!["root"],
        "a feed lists thread heads, and both replies are not ones"
    );
    let feed_author = page.items[0].author.clone();
    assert_eq!(
        feed_author,
        keystore.stoa_address(&stoa).to_hex(),
        "the root must be attributed to the address the keystore derives for this Stoa"
    );

    // ── Claim 3: the probe reports an identity the feed does not carry ──
    //
    // `wire::posting_identity` is what `getCapabilities` calls. Both operands are
    // the module's own answers — the probe's, and the author the store reported —
    // so this cannot be satisfied by two derivations performed here.
    let probe_reports =
        wire::posting_identity(&stoa, &keystore, &paths).expect("a recorded path is readable");
    assert_ne!(
        probe_reports, feed_author,
        "the probe and the publish path have stopped disagreeing — the \
         three-derivations gap is closed. That is good news: delete this \
         assertion, the equivalent one in examples/seed_store.rs, and the \
         paragraphs in openspec/changes/seed-store/design.md that document the gap"
    );

    // The consequence, asserted rather than left as prose: the record's creator
    // moderates, and the key every op was signed with does not — so a hide
    // published through the module against a seeded Stoa is refused.
    assert!(
        moderators.contains(&genesis.creator),
        "the record's creator must moderate its own Stoa"
    );
    assert!(
        !moderators.contains(&founder.public_key()),
        "the signing key has BECOME a moderator — the same gap closing from the \
         other side, and the same deletions apply"
    );
}
