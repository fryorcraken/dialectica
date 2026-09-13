# Readability findings — `publish-envelope`

Reviewed at `d055c0c`, in a worktree of my own. Dimension: **readability** (a
sibling file carries architecture; correctness and security were reviewed before
me and I did not re-report their entries). Suite green at **737 + 26** before any
mutation, which is the count `tasks.md` 7.1 claims. Every probe below was run in
my own worktree and reverted; the worktree is deleted.

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7671`
      and `:7682` — the sweep list's doc still says nothing checks it, in the
      same change that made something check it
      **Scenario:** `every_request_taking_method`'s doc comment reads
      *"**If you are adding a method ... add it to this list** ... Nothing checks
      it"* (7671) and *"a source-scanning test was rejected for failing on
      unrelated things"* (7682-7683). Both are now false, and false **because of
      this change**: `the_sweep_covers_every_request_taking_method_the_dispatch_
      trait_declares` is a source-scanning test that checks exactly this list,
      and `design.md` decision 6 spends fourteen paragraphs arguing why the
      earlier rejection does not apply. The new test 51 lines below even quotes
      the doc in the **past tense** — *"That list's doc **said** 'ADD YOUR METHOD
      HERE ... Nothing checks it'. Something checks it now"* (8039-8041) — while
      the doc it quotes still says it in the present tense, unedited.
      **Measured:** `grep -n "Nothing checks it"` returns 7671 (the live doc) and
      8039 (the test quoting it as past). A reader who reaches 7671 first — which
      is the order the file is in, and the order an author adding a method
      arrives in, since 7665 is the heading addressed to them — learns that
      forgetting the list is silent. It is not: it is a red test naming their
      method. **Severity: medium** as a defect, not a preference. This is the
      documented failure family in `docs/OPENSPEC-ARCHIVE.md`'s neighbourhood and
      in this file's own history: the `with_membership_store` doc at 2279 and the
      adapter header at 621-624 each carry a correction for exactly this — *"was
      false of the file by the time it was read"*. The fix is to rewrite 7665-7685
      to say what now holds: add your method here, and if you do not, the trait
      sweep fails naming it.

- [ ] **`dev-writer`** — `.github/workflows/ci.yml:363-373` — a red on
      `core::stoa_of` prints a message entirely about key derivation
      **Scenario:** the `want` loop now guards four strings, and two of them —
      `core::wire::publishing_key` and `core::stoa_of` — were added by this
      change. The message body was not. It still reads *"which key the creator
      is, and which the poster is, are decisions that must live in core ...
      Deriving either here restores the bug 4313cf6 fixed"*. For a missing
      `core::stoa_of` that is wrong in three ways at once: `stoa_of` derives no
      key, neither "the creator" nor "the poster" is what went missing, and
      `4313cf6` is a commit about creator/poster key agreement, so an author who
      follows the citation reads a fix unrelated to the ordering property the
      call actually holds. The gate's own 10-line comment block at 353-362 states
      the right reason — *"`core::stoa_of` IS REQUIRED FOR AN ORDERING REASON,
      not a derivation one"* — and that reasoning reaches the YAML reader and
      never reaches the CI log.
      **Measured:** I ran the gate's own `want` loop, verbatim, against
      `35fc859` (this piece's own previous commit, where `stoa_of` is absent).
      It printed:
      `::error file=...::core::stoa_of is gone from the adapter — which key the
      creator is, and which the poster is, are decisions that must live in core
      where a test can reach them. Deriving either here restores the bug 4313cf6
      fixed, with every other gate green.`
      The brief asks whether a red is diagnosable in under a minute from its
      message alone. For the two pre-existing wants, yes. For `core::stoa_of`,
      no — the message actively misdirects. **Severity: medium.** The fix is a
      per-want reason, e.g. a `{want: reason}` dict, so each red says why *that*
      call must be in the adapter.

- [ ] **`dev-writer`** — `dialectica/rust-lib/src/lib.rs:508-533` — a 26-line
      comment on a 4-line call, of which ~20 lines are the history of a defect
      already fixed
      **Scenario:** the `core::stoa_of` call site carries 26 lines of comment for
      `let stoa = match core::stoa_of(request) { Ok(a) => a, Err(e) => return e };`.
      CLAUDE.md's rule is that a comment earns its place by saying what a command
      cannot. Two of the five paragraphs do: *"It reads the Stoa and nothing
      else"* (527-529) answers "why does the adapter not validate the rest?",
      which is the question a reader of this line actually has, and the naming of
      the test and the CI gate (531-533) answers "what holds this?". The other
      three paragraphs (511-522) narrate what the code used to be and what went
      wrong with it — a bare `from_str`, the four-arm ladder, ~2N heap,
      PHASE0-FINDINGS §3 — which is **already stated twice** in this repo, in
      `core::stoa_of`'s own doc (`wire.rs:2027-2034`) and in `design.md`
      decision 7, both nearly verbatim.
      **Measured:** 26 comment lines to 4 code lines. The same narration appears
      at `wire.rs:2027-2034` (8 lines), at `design.md:188-196`, and at
      `ci.yml:353-362` — four copies of one paragraph, three of which a reader of
      this call site does not need. **Severity: low, and this one is a
      preference with a concrete cost** rather than a defect: the two paragraphs
      that answer a reader's live question are buried under three that answer a
      question about a commit. The fix is to keep 524-533 and cut 511-522 to a
      pointer at `design.md` decision 7.

## What I checked and found clean

**The refactor commit `4c30aaa` improved readability, not merely length — and
the distinction is measurable here.** The three handlers lost 75 lines of
`let x = match f(&parsed) { Ok(v) => v, Err(e) => return e };` ladder and gained
9 lines of `?`. `publish_post`'s body is now three lines and `publish_vote`'s is
four; each reads as "this operation's fields, then the operation", with nothing
between the signature and the thing the function is for. The commit's +116/-75
looks like growth, but the code shrank by ~66 lines and the insertions are
almost entirely doc comment on two new items. That is the right direction: the
guard is no longer something a reader must check each handler for, because a
handler that holds a `PublishRequest` cannot have skipped it.

**The `core::stoa_of` boundary is legible at the call site**, which is the
brief's question. The adapter's comment states the rule in one sentence — it
reads the Stoa and nothing else, the guard and required-field reads stay the
handler's — and `stoa_of`'s own doc gives the reason under a heading that says
it (*"It reads the Stoa and nothing else, deliberately"*). A reader at either end
gets the same answer. Both `stoa_of` and `PublishRequest::parse` call the single
`parse_stoa` (`wire.rs:468`), so the two readers cannot word the same refusal
differently — the property the prose claims is the one the code has.

**The classifier's preconditions are stated where an author hits them.** The
brief asks whether someone adding a method would understand the required shape
*before* it panics. Mostly yes, and by the right route: the panic message itself
(8011-8016) names the method, says it cannot tell whether it reads a request,
says it will not assume, and names both the function to teach and the list to
add to. The doc's own "Its preconditions" heading (7904-7913) states all three —
rustfmt-shaped Rust, a declaration ending in `;`, `String` by value. The
trailing-comma comment (7968-7974) is the best comment added by this change: it
records that the gate once failed for the *wrong* reason and why that is worse
than failing for the right one, which is a thing no command can tell you.

**Both new CI gates fire for the right reason and say so.** Against `35fc859`
the `from_str` ban prints the method count and names three legitimate
replacements (`core::stoa_of`, `core::parse_channel_id`, or a handler) — that
one is diagnosable well inside a minute. My finding 2 is about the `want` loop
only.

**`tasks.md`'s numbers check out.** 7.1 claims 737 + 26; I measured 737 + 26.
`4c30aaa`'s message claims 733 + 26, and correctness review independently
measured 733 by reverting the fix line.

## What I could not check

`dialectica/rust-lib/src/lib.rs` is behind `cfg(logos_scaffold)`, so finding 3 is
a reading of the file rather than an execution of it. I could not run
`cargo fmt --check` or clippy over that file for the same reason, and no gate in
CI reads it as Rust — only the two text gates and Build LGX.
