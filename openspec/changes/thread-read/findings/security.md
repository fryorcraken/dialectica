# thread-read — security review

Reviewed: `dialectica-core/src/thread.rs`, the `read_thread` half of
`dialectica-core/src/wire.rs`, and `rust-lib/src/lib.rs`'s `readThread` adapter
(read by eye, since `cfg(logos_scaffold)` keeps it out of `cargo test`).

**The central security property holds.** `thread_of` never mentions the `thread`
field, verifies at every link before reading `parent`, and terminates on a
visited set. Replacing its body with a claim-trusting one fails **12 of 807**
core tests (the dev-writer's 9 plus the tester's additions), including the wire-
level `the_wire_never_places_a_post_by_its_claimed_thread`. **I could not inject
a post into a thread by any route**: entering thread T requires naming as
`parent` an op whose own chain reaches T, which is ordinary replying — the
`thread` claim, a forged op, a cycle, a self-parent and a missing parent each
place the post under no thread. A chain crossing a Stoa boundary mid-walk
(`a_chain_through_another_stoa_still_places_only_this_stoas_posts`) admits only
the in-Stoa reply beneath an honest foreign parent, which is placement rather
than injection.

`cargo mutants --file dialectica-core/src/thread.rs`: 30 mutants, **27 caught,
3 unviable, 0 missed**.

Pagination arithmetic is sound against peer input — `saturating_mul`, `min`,
`saturating_add` hold at `page = usize::MAX` and `per_page = usize::MAX`. No
indexing, no slicing outside the clamped range, no `unwrap` on a peer-derived
value, no secret compared or emitted: `authorKey` is the 32-byte ed25519
verifying key and `author` its one-way address digest, and
`every_item_carries_both_an_address_and_the_key_that_signed` checks the pairing
rather than asserting it. The withheld body is genuinely absent from the JSON
object, not an empty string.

## Findings

- [x] **`dev-writer`** — `thread.rs:520-521` via `revision.rs:292` —
      a revision stamped with **another Stoa's address** rewrites a thread
      item's body, so a thread read renders content from an op that does not
      belong to the Stoa it was asked for.
      **Scenario:** root (key 2) and reply (key 3) in Stoa A. Key 3 signs
      afresh — not replays — `Revise { target: reply, body: "REWRITTEN FROM
      ANOTHER STOA" }` with `stoa: <Stoa B>`. `read_thread` over Stoa A returns
      the reply carrying `body = "REWRITTEN FROM ANOTHER STOA"`, `isRevised =
      true`. Run and observed, not reasoned: the probe printed exactly that.
      `is_valid_revision` checks kind, `verify()` and author equality, and
      **never the Stoa**; `iter_target` filters by target alone. The
      `moderation` resolver does check it — `Moderators::authorises` leads with
      `entry.op.op.stoa == self.stoa` — so the two resolvers disagree about
      whether a Stoa is part of an op's standing.
      **Why the existing test does not cover it:**
      `revision.rs:674 a_revision_lifted_into_another_stoa_is_dropped` builds a
      *replayed* op — the signature is copied, so `verify()` fails — and its
      comment then generalises: *"the resolver never looks at the Stoa itself —
      it relies entirely on `verify()` to catch this."* Execution disproves the
      generalisation: `verify()` catches the replay and not a fresh signature.
      **Severity: high.** The `thread-read` spec states "It SHALL return only
      ops belonging to the Stoa named in the request", and the rendered body is
      the largest attacker-supplied string this read returns. The root cause is
      in `revision.rs` and predates this piece — `feed.rs` shares it — but this
      is the piece whose spec makes the claim, so the fix or the explicit
      deferral belongs here. If deferred, say so in `design.md`'s "found and did
      not fix" section beside the `feed.rs` author-key gap.

      **Fixed at the root, in `revision.rs`, rather than deferred or filtered
      here.** `is_valid_revision` gains a fourth condition —
      `candidate.op.op.stoa == original.op.op.stoa` — compared against the
      **target's** Stoa, which the function already holds, so no call site gains
      an argument it could get wrong. `design.md` §10 carries the argument; the
      short form is that a filter in `thread.rs` would be the fourth
      slightly-different copy of a guard `moderation.rs` and `authoring.rs`
      already make, and narrowing the spec instead would mean writing down that a
      thread read may render another Stoa's content.

      **It repairs `feed.rs` for free**, since the feed calls the same resolver —
      which is the argument for fixing the root cause rather than the call site,
      and is recorded in `design.md`'s "found and did not fix".

      Two tests, and each fails without the line. `revision.rs`'s
      `a_revision_freshly_signed_for_another_stoa_is_dropped` is the resolver-level
      one: it asserts the fixture **verifies** and is **by the post's own author**
      before asserting the outcome, so it can only pass for the reason it names —
      which is exactly what the replay fixture could not do. `thread.rs`'s
      `a_revision_stamped_with_another_stoa_does_not_rewrite_a_post` is your
      scenario as written, and **measured failing first**: `left: "REWRITTEN FROM
      ANOTHER STOA", right: "what the author actually wrote"`. Reverting the
      condition fails both: 812 passed, 2 failed.
      `a_revision_in_this_stoa_still_rewrites_the_post` and
      `a_revision_in_the_posts_own_stoa_is_still_accepted` are the positive halves,
      without which a fix refusing every revision would pass both.

      **The misleading comment is corrected too.**
      `a_revision_lifted_into_another_stoa_is_dropped` no longer claims the
      resolver "relies entirely on `verify()`"; it now states what it actually
      covers — a replay, refused by the signature — and points at the new test for
      the case it wrongly implied. Your observation that it passed for a reason
      narrower than its comment claimed is the whole reason this was invisible,
      and it is recorded in `design.md` §10 rather than only fixed.

- [ ] **`tester`** — `wire.rs:1593-1597` — the genesis/Stoa pairing check in
      `read_thread_inner` is **entirely untested**, and it is the check that
      stops a caller applying one Stoa's moderator set to another Stoa's posts.
      **Measured:** replacing `if genesis_address != stoa` with
      `if false && genesis_address != stoa` leaves **807 of 807 core tests and
      26 of 26 integration tests green.**
      **Scenario under that mutation:** `read_thread` is called with
      `{"stoa":"<A>","thread":"<root in A>"}` and a `Genesis` for Stoa B. The
      read is served rather than refused, `Moderators::of` builds a set
      governing B, and `Moderators::authorises` — which leads with
      `entry.op.op.stoa == self.stoa` — then binds nothing, so **every
      moderation of Stoa A silently stops binding**: a hidden reply reappears
      and a hidden root renders its withheld body. Observed: the probe returned
      both items with `"moderation":{"state":"unmoderated"}`.
      A test asserting the refusal message pins it. Note the sibling at
      `wire.rs:1253` has the same untested check — this piece copied the shape
      — but only the `read_thread` one is this change's to cover.
      **Severity: high** (the check is present and correct; the gap is that
      nothing would notice its removal). Not reachable from a peer today: the
      `read_thread_from_request` path additionally calls
      `Membership::verified(stoa, &genesis)`. It is the `read_thread(request,
      log, genesis)` entry point — public, re-exported at the crate root — that
      relies on this line alone.

- [ ] **`tester`** — `dialectica-core/tests/end_to_end.rs` — **no integration
      test exercises `read_thread` against a real `SqliteOpLog`.** Every one of
      the 26 integration tests drives `feed::list_threads` or
      `wire::list_threads_from_request`; `grep -n "read_thread"` over
      `end_to_end.rs` returns nothing, and `grep -n "SqliteOpLog"` over
      `thread.rs` returns only a doc comment.
      **Why it matters here specifically:** that file's own header records a
      defect the `feed::list_threads`-layer test could not see and the
      `wire`-layer-over-SQLite test did — *"the `feed::list_threads`-level test
      one layer down **stayed green**, which is why that layer could not cover
      this seam."* The new read has the identical two-layer shape and only the
      upper layer, over `MemoryOpLog`, is covered. The piece also *cites*
      `SqliteOpLog` behaviour as load-bearing (`thread.rs:1016`) while never
      running against it.
      **Severity: medium.** One test — write a thread to a real file, reopen,
      `read_thread_from_request`, assert the root and a reply come back in
      `cmp_ops` order — closes it.

## On the `CyclicLog` boundary, and the rootless page it exposes

The tester's judgement that `CyclicLog` is acceptable is **right**: a parent
cycle is a preimage naming its own hash and is unmintable against SHA-256, so a
fake is the only honest fixture, and the alternative — skipping the test —
leaves a walk that *hangs* rather than fails. The flagged gap ("nothing asserts
`SqliteOpLog` actually returns a key-disagreeing row when the file holds one")
is also correctly drawn: `SqliteOpLog::get` selects `WHERE op_id = ?1` and
`decode_entry` re-derives nothing, so the disagreement is structural and
`sqlite-projection` is the right owner of proving it end to end.

But the fake was only ever driven through `thread_of`, never through
`read_thread`, and that leaves a reachable state nobody looked at:

- [x] **`dev-writer`** — `thread.rs:384-463` — a store row whose key disagrees
      with its bytes produces a **successful page with no root and no items** —
      the one reply the spec says must never be served.
      **Scenario:** a row filed under id `X` holding a genuinely signed,
      parentless, in-Stoa post whose real id is `Y`. `read_thread(log, .., X)`
      passes all four root checks (`get(X)` yields it; it verifies; its Stoa
      matches; it is a `Post { parent: None }`), then iterates: `iter_stoa`
      hands the entry over under `entry.id() == Y`, `thread_of(log, &Y)` calls
      `get(Y)` which finds nothing, so the entry is skipped. Result, run
      against a `CyclicLog` holding one `entry_at(an_id(0x55), None, 7)`:
      `Ok(Ok(ThreadPage { items: [], page: 0, has_more: false }))`.
      **Why that is the forbidden answer:** the spec's own argument is that "an
      empty listing is indistinguishable from a subject nobody has posted in,
      so a read that answered an unknown thread with an empty page would render
      a peer that has never received a thread exactly as it renders a thread
      whose author wrote one post" — and `read_thread`'s doc promises "the root
      item first" unconditionally.
      **Severity: medium.** Not remotely triggerable — it needs a corrupted or
      hand-edited store file, exactly the threat model `CyclicLog`'s own doc
      comment invokes to justify itself. The narrow fix is to refuse when the
      root entry's `entry.id() != *root`, which is a one-line guard on the
      value the walk already computes; the broader one belongs to
      `sqlite-projection`. Either way the piece should not be able to serve a
      rootless page, and today it can.

      **Fixed — and the same disagreement one link further along was reachable
      too, which your finding led straight to.** The guard you named is in
      `read_thread` (`root_entry.id() != *root` → `NotHeld`), and reproducing it
      showed the walk had the same trust: `thread_of` took whatever `get` returned
      and read **its** `parent`, so a lying row mid-chain let one op's bytes decide
      where a different op's id leads. Measured before the second guard:
      `thread_of` answered `Some(6666…66)` — "this chain reaches a root" naming an
      id under which no root exists. So `thread_of` carries the same comparison
      (`entry.id() != current` → `Ok(None)`).

      Two tests, each failing without its own guard —
      `a_store_row_filed_under_the_wrong_id_is_refused_rather_than_served_rootless`
      (measured first as "served a page with 0 items rather than refusing") and
      `a_store_row_filed_under_the_wrong_id_places_nothing_mid_chain_either`. Both
      assert the fixture genuinely disagrees with itself and that the op is
      otherwise a perfectly good root, so neither can pass for absence. Stubbing
      both guards out fails exactly those two: 812 passed, 2 failed.

      **`NotHeld` rather than a fourth variant**, because it is literally true —
      the peer holds no op *under that id* — and because a caller can do nothing
      different with "your store is corrupt" than with "wait for it to arrive".
      The spec fixes three refusals and this adds none. `design.md` §11 records
      it, including that `sqlite-projection` still owns proving a real file
      produces such a row end to end; this piece owns not serving a rootless page
      when one does.
