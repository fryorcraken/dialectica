# Security review — `spec-tidy`

Scope: `git diff 5ec1295...HEAD` (three-dot, merge-base diff against the piece's
start point) — the five archive promotions, this change's own deltas, the four
comment edits, the `main` merge (`cb46319d`), and the tester's new
`sqlite.rs`/`tst_thread_reply.qml` tests. Reviewed against CLAUDE.md's two
standing rules (never trust an inbound message; moderation must be
authenticated and authorised) and the specific pointers in the brief.

## What was checked

- **`op-log`'s storage-failure and layout checks** (`1f5df8f3` rebase,
  `05194161` archive): diffed the archive-commit's own claim ("The diff against
  the live spec is additions only: no existing line changed or went") against
  `git show 05194161 -- openspec/specs/op-log/spec.md` — confirmed true, every
  hunk is a pure addition. The persistence non-filtering guard ("Persistence
  SHALL NOT become an occasion to filter by signature or authority…") and the
  distinct-failure-not-empty-result requirement are both present, and are
  stronger than the pre-existing text (the blank-title refusal is explicitly
  scoped as "the op format's and not a judgement of the op's authenticity or
  authority").
- **`moderation-view`'s "doesn't imply the user moderates"**
  (`8e508d8a` correction, `eb8be6b4` archive): the requirement "The screen does
  not state or imply that the user moderates the Stoa" was newly added (it did
  not exist before, per the `NO SPEC:` marker it now resolves) and its
  scenario matches the test it backs (`tst_moderation_screen.qml`'s
  "No moderator standing is claimed for the reader"). No weakening — this is a
  net-new guard, correctly justified against the code ("the route is not gated
  on moderator standing").
- **`stoa-genesis`'s attacker-supplied-address citation**
  (`49b98ca1`): compared the live requirement text
  (`openspec/specs/stoa-genesis/spec.md:116-135`) against the delta
  (`openspec/changes/spec-tidy/specs/stoa-genesis/spec.md`) line by line. Only
  the citing sentence changed — "§4.8 has Stoa addresses appearing inside
  posts, which is attacker-supplied content" became a citation to
  `stoa-navigation-view`'s live requirement naming the same fact. The MUST/SHALL
  text and all three scenarios (matching record verifies, substituted record
  fails, verification consults nothing external) are byte-identical. Same check
  done for `moderation-resolution`'s "The deciding moderation is named, not
  merely counted" (§5.7 → `post-revision`'s "A post is never edited in
  place") — same result, citation-only.
- **The `view-navigation` fold** (`ff55cf77`, plus the `spec-tidy` delta):
  read the live `thread-view` and `moderation-view` requirements being removed
  in full, and the `view-navigation` `MODIFIED`/`ADDED` blocks receiving them,
  and matched every scenario against the `REMOVED` blocks' Migration notes.
  Nothing was silently dropped: "The way out survives a refused read", "No
  record is invented for a Stoa the view has none for", "No thread is rendered
  before one has been chosen" and "The thread screen carries no thread of its
  own" all carry across with their normative text intact, and the one dropped
  scenario ("The feed is reachable again from the thread") is genuinely
  redundant with the live, unmodified "The feed is reached again from the
  thread" already in `view-navigation` (same outcome: feed rendered, view not
  restarted) — confirmed by reading that scenario directly rather than trusting
  the claim.
- **The `main` merge** (`cb46319d`): `git show --remerge-diff cb46319d` shows
  the only real conflict was a clean, non-lossy concatenation of two appended
  requirement blocks in `identity-onboarding/spec.md` (this piece's two
  master-key requirements, then `#180`'s malformed-index requirement) — no
  markers left, no line lost, no requirement's text altered. `wire.rs`'s 439
  added lines and the `position-and-index` archive under
  `openspec/changes/archive/` are pure `main` content pulled in by the merge;
  `git diff d57edafe cb46319d -- .../wire.rs` is empty, confirming this piece
  did not touch them.
- **The new `sqlite.rs` tests dropping a table on an open connection**
  (`a98c4345`, `5f95e7e9`): both new tests (`the_stored_author_is_the_signer…`
  and `a_read_against_storage_broken_after_open_is_a_failure_not_an_empty_
  result`) build their log with `SqliteOpLog::in_memory()`, which is
  `Connection::open_in_memory()` — SQLite's `:memory:` backend, private to the
  test process and discarded on drop. The `DROP TABLE ops` runs against that
  connection's own handle, not any file on disk. No reach outside a disposable
  test store, and no interaction with the owner's Basecamp configuration or
  any real database.
- **Whether an archive or correction weakened a guard elsewhere**: spot-checked
  `generated-names` and `feed-view`'s deltas (`92b31622`, `1bcf2bf3`) — both are
  citation/prose fixes with no MUST/SHALL text touched, consistent with
  `design.md`'s claim that these are prose-only.
- Confirmed `spec-tidy`'s `.openspec.yaml` carries no `skip_specs: true`
  (consistent with Decision 3: this piece changes requirements, so it goes
  through the normal delta/promote path rather than direct edits).

## Findings

- [x] **none** — no defect found on the security dimension. Every citation
      swap preserves its requirement's normative text and scenarios
      byte-for-byte; the `view-navigation` fold accounts for every scenario it
      removes; the `main` merge lost no requirement text; and the new
      `sqlite.rs` storage-failure test operates entirely on a private
      in-memory SQLite connection.
