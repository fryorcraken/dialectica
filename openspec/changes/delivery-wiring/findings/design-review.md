# Design review: delivery-wiring (piece #176, PR #190)

Read at `7a2a3335`: `design.md`, `proposal.md`, `tasks.md`, both spec deltas,
`core/src/delivery.rs`, `core/src/sender.rs`, the adapter diff in
`dialectica/rust-lib/src/lib.rs`, issue #176 (body and comment), and `gh pr view 190`.

The code takes the decisions design.md records: the four-method seam has no
`stop`, `CALL_TIMEOUT` is 35 s, `INBOUND_BOUND` is 256, the node config is
`channels` / `logos.test` / `Edge`, the sender id is `/dialectica/1/p/` plus 32
random bytes, and `SETTLE_LIMIT` is 40 s. I found no code that contradicts a
recorded decision. The preset and mode are stated as an Open Question, and the code
uses `logos.test` and `Edge`, matching Decision 5. Each CLAUDE.md trap has a line in
the design table. Spec deltas, proposal and design agree with each other. Every gap
below is in the prose, the verification claims or the PR body.

Serious means the prose or the change contradicts the issue or itself. A gap is a
decision not recorded. A suggestion is a thin entry.

- [x] **`dev-writer`** (serious, contradicts issue #176 without argument) —
      `proposal.md:136-137` says "This PR is verified by hand with two `lgs basecamp
      launch` profiles", and `design.md:332` says two peers exchanging an op "is
      verified by hand". `tasks.md:56` (7.3) is unticked, and the PR body says "Not
      done here: two live peers exchanging a post". So two documents state a
      verification that has not happened. Issue #176 says "Verification has to be two
      live peers under `lgs basecamp launch`". The owner's comment asks for an
      automated test after the issue lands, and does not waive the manual check. The
      departure (the PR ships without the check the issue calls mandatory) is neither
      argued nor put to the owner. Either run 7.3 and record the result, or reword
      proposal and design to "not yet verified live" and put the missing live check
      to the owner as a stated departure with a reason.
      **Deferred** to the owner, the second option: `proposal.md` and `design.md`
      (Risks) now say the live check has not been run, and design.md's Open
      Questions states the departure from #176 and its reason (a live run needs a
      person to click each profile's tile; this pass could not run it) and asks the
      owner to accept it or hold the merge until 7.3 is run. The PR body's "Open
      for the owner" carries the same. 7.3 stays unticked.
- [ ] **`closer`** — the PR body's Receive bullet reads "the reverse of the closed
      #30". `closed #30` is a GitHub closing keyword followed by an issue number. It
      is harmless on the state of an already-closed #30, but it puts a second
      keyword-plus-number beside `Closes #176`, which the piece forbids (see the
      "PR-body prose closes issues" incident). Reword it to "the reverse of #30's
      choice" and re-scan the body for keyword+`#N`.
- [x] **`dev-writer`** (gap) — Decision 11 records the pending-open wait and its
      bound but not its cost, and no alternative other than "refuse as unknown".
      `Channels::await_settled` blocks the single processor for up to `CALL_TIMEOUT`
      (35 s, `SETTLE_LIMIT` 40 s), so while one `channelCreate` is slow, messages on
      every other, open channel wait behind it. Meanwhile the 256-deep queue of
      Decision 10 fills and discards arrivals, and a discard is final (Decision 10's
      own reasoning). Decisions 10 and 11 interact and neither entry says so. Also
      unrecorded: why the wait sits in the processor and is not a per-channel hold
      or a requeue, and what `SETTLE_LIMIT` costs when reached. Record that cost and
      the alternatives.
      **Fixed** in the docs commit: Decision 11 now has "What the wait costs" (the
      single processor stalls every open channel for up to one call; the queue then
      fills and discards; reaching `SETTLE_LIMIT` judges against what is open then,
      a lost op for a still-unanswered open) and "Alternatives, and why not"
      (refuse without waiting; a per-channel hold; requeue). Decision 10 ends by
      naming the interaction, and Risks carries the stall.
- [x] **`dev-writer`** (gap, mutation evidence) — design.md carries "removing this
      turns X red" for Decisions 3, 11 and 13 only. The other guards have theirs only
      in the PR body's mutation list, which goes with the PR:
      - Decision 10: `>=` to `>` (three bound tests) and discarding the oldest
        (`a_full_queue_keeps_what_it_holds_and_discards_the_arrival`).
      - Decision 12: refusal log carrying the text
        (`a_refusal_is_logged_by_kind_without_text_the_sender_chose`).
      - Decision 7: an unretained sender id still opening a channel
        (`a_sender_identifier_that_cannot_be_retained_opens_no_channel`).
      - Decision 9: sink called before verification (`a_refused_join_requests_no_channel`).
      - Decision 6: dropping the start-once guard
        (`node_creation_is_requested_once_however_often_startup_runs`), and the
        `declined` envelope mutation (three tests).
      - The clock-source mutation (seven tests) in Decision 10.
      Copy each into its Decision entry. Decision 2's source-reading test
      (`the_adapter_never_stops_a_node_and_creates_one_at_one_site`) and Decision 5's
      pin also state no "removing X turns Y red" line; add one for each.
      **Fixed** in the docs commit: each is in its Decision entry (2, 5, 6, 7, 8,
      9, 10, 12). Where the first pass's list gives a count and no test names (the
      `declined` envelope's "three tests", the clock's "seven"), the entry says so
      and cites that list rather than naming tests I did not re-run. This round's
      new guards (Decisions 11, 14, 15, 16, and the hand-over refusal in 10) carry
      the test that was seen red.
- [x] **`dev-writer`** (gap, reasoning that migrates) — the traps this change
      learned are needed by whoever next touches `lib.rs` or `delivery.rs`, and they
      sit only in this change's `design.md`, which will be archived. Put a short
      entry in CLAUDE.md's "Module contract traps" (or a `docs/` file linked from its
      table) for each:
      - delivery answers a decline as `Ok` with an error envelope (`declined`); a
        second `createNode` says "Context already initialized".
      - delivery v0.2.1 can emit `channelMessageReceived` before the `channelCreate`
        answer reaches the caller (the pending-open state).
      - delivery's 30 s callback timeout outlasts the IPC 20 s default, hence 35 s.
      - `cargo test` does not compile the adapter; only `nix build ./dialectica#lgx`
        does.
      CLAUDE.md's `RET_STALE_WARN` line still says "Handle" and "Ignoring it
      double-counts". Decision-table row 3 says it is unreachable on this surface,
      confirmed at v0.2.1. Amend that line so the two do not read as contradicting.
      **Fixed** in the docs commit: CLAUDE.md's "Module contract traps" gains four
      entries (decline as `Ok` with an envelope, empty `error` and "Context already
      initialized"; the 30 s callback against the 20 s IPC default, and the
      "already exists" answer after it; `channelMessageReceived` before the
      `channelCreate` answer; `cargo test` not compiling the adapter), and the
      `RET_STALE_WARN` entry now says it reaches outcome-event counting and not the
      module's own calls at v0.2.1.
- [x] **`dev-writer`** (gap, proposal is stale against the code) —
      `proposal.md:156-157` says the spec takes a position on open questions 1-4 and
      "each can be reversed before the code lands". The code has landed. `design.md`
      Open Questions and the PR body carry only the preset and mode. Questions 1-4
      (a failed open leaves the join unchanged, sender-id privacy, discard the
      arrival, re-publish sends again) are neither recorded as resolved nor still put
      to the owner. Decisions 7, 9 and 10 cover 1-3 by implication, and 4 is only in
      tasks 4.4. State in design.md which of the four are settled by the spec and
      which remain open, and align the PR body's "Open for the owner".
      **Fixed** in the docs commit: design.md's Open Questions says 1–4 are settled
      by the spec and implemented, with the Decision for each, 5 is the preset and
      mode, and 6 (added by the spec-writer since) is the owner's. The proposal's
      sentence was already corrected by the spec-writer in `9c96a41a`. The PR body's
      "Open for the owner" is aligned.
- [x] **`dev-writer`** (suggestion, thin entries) —
      - Decision 5 gives the `logos.test` cluster number, the 150 KiB maximum
        message size and "Edge publishes and receives through the service nodes" with
        no source, and it says "matches #30". Nothing shows an `Edge` node receives
        `channelMessageReceived` on a reliable channel, which is the whole feature.
        Decision 7 says "Not measured" for its SDS claim, and Decision 5 should say
        the same about these claims, or cite the file (`docs/SOURCES.md` names the
        checkouts).
      - Decision 7 states the format but not why: the `/dialectica/1/p/` structure
        against a bare hex string, the `p`, and the choice of 32 bytes. The reason is
        a paragraph in the `sender.rs` doc comment, and a paragraph means it was a
        decision.
      - Decision 6 says unanswered means decline, and the cost is unrecorded. A
        `createNode` that delivery accepted but answered after 35 s is treated as
        declined, so `start` is never requested and no channel on that node can work.
      **Fixed** in the docs commit. Decision 5 cites `networks_config.nim` and
      `default_values.nim` at `logos-delivery` `bfdb5afd`, the rev delivery v0.2.1's
      `flake.lock` pins — and notes that a later checkout (`4a85db1b`) names cluster
      2 for both presets, so the claim is about the pin — and says in bold that an
      `Edge` node receiving `channelMessageReceived` is not measured. Decision 7
      gains "The format" (32 bytes, the structured head, `p`, the newtype).
      Decision 6 gains the cost of an unanswered `createNode`, and why a node has
      no "already exists" recovery the way a channel does (Decision 14).
