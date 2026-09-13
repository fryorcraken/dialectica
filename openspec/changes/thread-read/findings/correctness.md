# thread-read — correctness review

Reviewed: `dialectica-core/src/thread.rs`, the `read_thread` half of
`dialectica-core/src/wire.rs`, and `rust-lib/src/lib.rs`'s `readThread` adapter
(read by eye — `cfg(logos_scaffold)` keeps it out of `cargo test`).

Baseline confirmed by command, not by the PR text: **807 core and 26 integration
tests pass** on `piece/thread-read`. `cargo mutants --file
dialectica-core/src/thread.rs --package dialectica-core`: 30 mutants, **27
caught, 3 unviable, 0 missed**.

What I tried to break and could not. Truncated and lying inputs are refused by
`Request::parse` and `parse_op_id` before `thread.rs` sees them, with
missing/wrong-type/bad-hex each a distinct message. `thread_of` is total over
every log I could build: an absent parent, a non-post parent, a self-parent, a
cycle of length 2, 3 and 4, a chain entering a cycle from outside, and a forged
op used as a bridge all return `Ok(None)`, and `a_long_legitimate_chain_still_
reaches_its_root` is the positive half that stops "always `None`" passing them
all. The root's three refusals are genuinely three messages and none of them is
`NotHeld` for an op the log holds — the `c6ce3c7` commit resolved that
contradiction the right way, toward the requirement rather than the scenario,
and `design.md` records the argument.

Pagination arithmetic is correct at every boundary I could reach through the
wire: `page = usize::MAX` with any `per_page`, `per_page` clamped from
`usize::MAX` to `MAX_PER_PAGE` and from `0` to `DEFAULT_PER_PAGE`, a page past
the end, and `perPage` refused as a float or a negative by spelling. The two
`MembershipStore::list` bugs that this piece was warned about do **not** recur
on the wire path. `rust-lib/src/lib.rs`'s `read_thread` is a faithful mirror of
`list_threads` — `storage_dir()` then the same per-call `SqliteOpLog::open`
closure, in the same order, with no step added or dropped — and both new
methods were correctly added to `every_request_taking_method()`, the sweep list
this repo has watched go stale before.

The `authorKey`/`author` pair carries what it claims: both are read from the op
verification has already bound, and the test decodes `authorKey` back into a
`PublicKey` and checks its `address()` equals `author` rather than pinning two
literals.

## Findings

- [ ] **`tester`** — `thread.rs:2427 an_enormous_page_index_does_not_overflow`
      — the test cannot observe the failure its own comment describes, because
      its fixture is one where wrapping and saturating agree.
      **Measured:** replacing `page.saturating_mul(per_page)` (`thread.rs:455`)
      with `page.wrapping_mul(per_page)` leaves **807 of 807 core tests and 26
      of 26 integration tests green.** `cargo mutants` cannot see this either —
      it does not substitute one method for a sibling method.
      **Why the fixture misses:** the test uses `page = usize::MAX`,
      `per_page = 20`. `usize::MAX.wrapping_mul(20)` is `2^64 - 20`, which
      `.min(items.len())` clamps to an empty page — the same answer saturating
      gives. The comment claims the mutation "would serve a page from the
      middle of the thread"; for this input it does not.
      **The fixture that does:** `page = 1 << 63`, `per_page = 2`. Under the
      mutation `(2^63).wrapping_mul(2) == 0`, so a thread of a root and one
      reply returns **both items while reporting `page:
      9223372036854775808`** — exactly the spec scenario's forbidden "a page
      from the middle of the thread". Run and observed, not reasoned: the probe
      printed `items=2 page=9223372036854775808 has_more=false`.
      **Severity: medium** — the production code is correct; nothing would
      notice if it stopped being. Note `per_page = 2` is reachable from the
      wire, so a regression here would be peer-triggerable. Adding this second
      case beside the existing one costs four lines.

- [ ] **`dev-writer`** — `thread.rs:455-457` — `read_thread` called with
      `per_page == 0` reports `has_more: true` on every page forever, so a
      caller paging on `has_more` never terminates.
      **Scenario:** any readable thread (every one has at least the root, so
      `items` is never empty). `start = page.saturating_mul(0) = 0`,
      `end = 0.saturating_add(0) = 0`, `has_more = 0 < items.len()` — `true`,
      for `page = 0`, `page = 1`, and every index. The page is empty and always
      claims another follows.
      **Not reachable from a peer**: `wire.rs:1609` clamps through
      `clamp_per_page`, which maps `Some(0)` up to `DEFAULT_PER_PAGE`. But
      `read_thread` is `pub` and re-exported at the crate root, and
      `clamp_per_page` is a separate function the caller must remember — which
      is CLAUDE.md's own "a guard is a job, so *is it called everywhere?* stays
      a question with an answer", and today the answer is "at one of two entry
      points". This is the same shape as `MembershipStore::list`'s
      `per_page == 0` never terminating, which the tester was pointed at.
      **Severity: low** as shipped, **medium** as a shape: the next caller of
      `read_thread` inherits it. Either clamp inside `read_thread` or make the
      parameter a type that cannot be zero; a test then pins whichever is
      chosen.

- [ ] **`tester`** — `thread.rs:1239 a_forged_root_is_not_readable_as_a_thread`
      — the test asserts the refusal *variant* but not the spec's disclosure
      clause, so the one sentence the requirement singles out is unpinned.
      **What the spec requires:** "the message does not reveal that the store
      holds bytes under that id" and "A refusal SHALL NOT disclose whether the
      store holds bytes that failed verification under an op id it refuses as
      not held."
      **What the test checks:** `Err(NotAThread::NotHeld(id))` — the variant
      only. The message is produced by `NotAThread::Display` at
      `thread.rs:231`, a different function; a reword there that added "(bytes
      present but unverifiable)" would violate the requirement with the suite
      green.
      **Severity: low** — the current message is correct and reveals nothing.
      This is a pin the repo has already been bitten by the absence of: assert
      the relation (the forged-root message is byte-identical to the
      never-arrived message, with only the id differing), rather than pinning
      the literal, so a reword fails on misinformation rather than on wording.

## Not defects

`design.md`'s two `NO SPEC` marks are both honestly drawn and both assert what
the spec *does* require alongside the choice made, which is the right shape for
an unsettled question. The Stoa-boundary-crossing chain in particular is
accepted with the alternative reading stated; I agree it is the better of the
two, since refusing mid-chain would drop a legitimate in-Stoa reply for a
property of an op that is never returned.

The `thread.rs:482` and `thread.rs:509` "unreachable" arms answer rather than
panic, and the reasoning given (a panic aborts the module process) is the
correct trade here rather than defensive noise.

## What I could not check

- **`rust-lib/src/lib.rs` compiles only under `cfg(logos_scaffold)`**, so I read
  the diff by eye against its `list_threads` sibling and found it identical in
  structure and ordering. I did not run an LGX build, so I cannot say the
  generated dispatch table names `readThread`; only that the trait method and
  impl are present and shaped like the one that already works.
- **`dialectica-ui`** has no `readThread` in `Core.qml`. That is consistent with
  this being a core-only piece, and is not a finding against it.
- I did not run `cargo mutants` over `wire.rs` — it is ~11k lines and the run
  would have gone well past the "abandon after a couple of minutes" line. The
  two wire-path mutations I did run by hand are recorded above and in
  `security.md`.
