# Architecture review — `thread-read`

Scope: is the shape right, and does it commit the code that comes next to
something awkward? Not correctness, security, readability, spec-test or
design-record — those are reviewed in the five other files beside this one.

**Three of the four questions the task set come out positive, and I checked each
against a command rather than against the design record.**

- **The O(posts × chain) cost does not commit `sqlite-projection` to anything
  awkward.** The debt is filed where it will be found — `PLAN.md:3686-3693`,
  which outlives this change's archived folder — and it asks for exactly the
  index a projection would build anyway: a lookup by **parent**. The correction
  from the original "posts whose `thread` is T" is the one that matters, since
  that field is the untrustworthy one, and it is stated in the bullet rather
  than left to be inferred. Nothing in `thread.rs` presumes the absence of an
  index: `thread_of` is a free function over `&L: OpLog`, so a projection adds a
  memo beside it rather than reshaping it.
- **The Stoa guard is in the right place, and I verified the claim that decided
  it.** Putting it in `revision.rs::is_valid_revision` (`revision.rs:327`)
  rather than filtering in `thread.rs` repairs `feed.rs` for free — `feed.rs:238`
  really does call `current_version`, which routes through `is_valid_revision`,
  so the claim is real rather than asserted. It is also right for callers that
  do not exist yet, and that is the stronger half: the comparison is against the
  **target's** Stoa, which the function already holds, so no call site gains an
  argument and no future call site can pair one wrongly. A filter in `thread.rs`
  would have been the fourth copy of a guard `moderation.rs` and `authoring.rs`
  already make, and would have left `feed.rs` broken. This is CLAUDE.md's
  root-cause rule applied correctly, and it is the best decision in the piece.
- **The wire surface conforms without widening the contract.** `readThread` takes
  and returns JSON, fails as `{"error":…}` through `guarded`, and returns
  `{"items":[…],"page":N,"hasMore":bool}`. `read_thread_inner` takes a
  `&Request` so the double-parse `list_threads` once paid for is unspellable
  here; neither public entry point nests a second `guarded` frame. `read_thread`'s
  own arity is `feed::list_threads`'s six plus `root`, which is the sibling shape
  the design aimed for rather than a function acquiring parameters.

Two findings below. The second is the `per_page` deferral, which I am **not**
asking to be reversed — the box is for one sentence the deferral is missing, not
for the newtype.

- [ ] **`dev-writer`** — `wire.rs:1481` against `wire.rs:1733` — the feed and the
      thread report the **same** moderation state in two different wire shapes,
      and nothing records that as a decision. **Scenario:** a feed row carries
      `"isHidden": <bool>` (`feed_page_json`, `wire.rs:1481`); a thread item
      carries `"moderation": {"state":"unmoderated"|"hidden"|"unhidden"}` plus an
      optional `"decidedBy"` (`thread_page_json`, `wire.rs:1733`, `wire.rs:1757`).
      Both are built from the same `crate::moderation::Moderation`. A view
      rendering a post's moderation state must therefore branch on *which call
      produced the item* — which is precisely what CLAUDE.md's wire conventions
      forbid: "JSON shapes are source-independent, so a view renders without
      branching on where the data came from." The thread's shape is the correct
      one and the design argues for it well (`design.md` §6: a bool collapses
      "nobody moderated" and "a moderator restored it" into one `false`, and the
      difference cannot be recovered afterwards) — so the finding is not that
      this piece chose wrongly, it is that choosing rightly created a divergence
      nobody wrote down. **Measured:** `grep -n "isHidden\|moderation"` over
      `wire.rs` shows `isHidden` built only at `wire.rs:1481` (feed) and asserted
      at `wire.rs:5414`; the `moderation` object built only at `wire.rs:1733` and
      asserted at `wire.rs:5874` onward. `grep -n "isHidden\|source-independent\|branching"`
      over `design.md`, `proposal.md` and `spec.md` returns **nothing** — the
      divergence is recorded in none of the three, where the feed's *author* gap
      is recorded in two of them. Severity: **medium**, and it is a record gap
      rather than a defect: nothing is broken today, `dialectica-ui` has no
      thread screen yet, and the fix when it comes belongs in `feed.rs`. But
      `docs/UI-BRIEF.md` is designed against by someone who cannot read the code,
      and this change updated it thoroughly in every other respect while leaving
      a designer to discover that "hidden" means a richer thing on one screen
      than on the other. One entry in "What this change found and did not fix",
      beside the feed's author gap which is exactly the same shape of debt.

- [ ] **`dev-writer`** — `thread.rs:309` and `authoring.rs:292` — two functions
      named `thread_of` in one crate, with **opposite** rules about the field
      they are both about, and neither doc mentions the other. **Scenario:**
      `authoring::thread_of(op, id)` reads the `thread` field and trusts it —
      `OpKind::Post { thread, .. } => thread.unwrap_or(id)` — which is correct on
      the publish path, where the op is this peer's own. `thread::thread_of(log,
      id)` must never read that field, and its doc says so in the strongest terms
      available. A reader who has met one and then meets the other has every
      reason to assume they are the same rule at two layers; they are the exact
      inverse, and the inverse is the attack. **Measured:** `grep -rn "fn
      thread_of"` over `dialectica-core/src/` returns exactly these two.
      `authoring.rs:279-283` (the deferral this piece discharges) does **not**
      name `thread.rs` as the code that now performs the audit, and
      `op.rs:327-330` — the `thread` field's own doc, which is where a reader who
      wants to *use* the field arrives — states the derivation as "its own
      `thread` when it has one, its own op id when it does not" and points only
      at `crate::authoring`. Before this change that was the whole truth, because
      `authoring` was the only consumer. It now has two consumers with opposite
      rules and names only the one that trusts it. Severity: **medium**. A
      claim-trusting edit to `thread::thread_of` is a one-line change, and the
      three places a reader would look for a warning before making it —
      `op.rs`'s field doc, `authoring.rs`'s deferral, and the sibling function's
      name — currently point the other way. The cheapest fix is two pointers:
      `op.rs`'s field doc gaining "**and never read on the read side** — see
      [`crate::thread`]", and `authoring.rs`'s deferral naming the module that
      discharged it. Renaming either function is a larger call and not required.

## The `per_page` deferral: sound, and one sentence short

**I would keep the deferral.** The reasoning in `design.md` §12 is honest about
the count (three), states the real four-file cost, names an owner and names the
decision that owner must make first — which of the two zero answers becomes the
single one. That is a brief, not an evasion, and the three surfaces are each
individually correct today. I verified the claim that makes it safe: all three
wire paths clamp up to the default, `list_stoas` included (`wire.rs:1271`,
`wire.rs:1609`, `wire.rs:2329` — the last calling `feed::clamp_per_page`), so no
caller reaching this crate through the module surface can observe the
disagreement. Reshaping `feed.rs` and `membership.rs`'s public surfaces from a
findings-response commit on a thread-read piece is the overreach this project has
a rule against, and taking the newtype here would be that.

One thing is missing from the record and it is worth a sentence rather than a
box, because nothing is blocked on it: **this exact question was raised once
before and answered differently.** `membership.rs:595` cites
`findings/architecture.md` entry 6 — a prior piece's architecture review — and
the answer there was an unconditional guard with a comment arguing that a
storage module whose correctness rests on a caller one layer up "invites someone
to delete the guard when that caller changes". §12 quotes that comment but
presents it as an independent arrival at this change's argument; it is actually
the second time a reviewer has asked for the shape and the second time a guard
was written instead. Whoever picks the work up should know it is the third
request, not the first — otherwise the same deferral is available a fourth time
on the same reasoning.

## Checked and sound

- **`read_thread` and `resolve_item` are one job each.** The split is at the
  right seam: `resolve_item` holds the hidden-root/hidden-reply asymmetry in one
  expression (`thread.rs:589-595`) where it can be read whole, and `read_thread`
  holds the page. Neither name carries an `And` or a vague verb, and neither
  reaches for ambient state — `resolve_item` takes `is_root` rather than
  recomputing it.
- **The hidden-root asymmetry is in the data structure, not a branch.**
  `body: Option<Sanitised>` with `None` reachable only through `withhold` makes
  "withheld" and "cleared by the author" different values rather than one value
  plus a flag, and `attachments: Option<Vec<_>>` follows the same rule. That is
  CLAUDE.md's "complexity in the data structure" applied where it pays.
- **Nothing in the new module sorts.** No `sort`, no `cmp`, no `max_by`; the one
  position fixed is the root's, by `items.insert(0, item)`, and
  `the_root_is_the_first_item_whatever_the_logs_order_puts_first` searches for a
  log whose order does not already lead with the root rather than branching on
  which way the hashes fell.
- **`parse_op_id` is deliberately not shared with `required_op_id`**, and the
  reason given (`wire.rs:1513-1516`) is the right one — the two have different
  envelope guarantees, and merging them would be one function serving two callers
  with different needs. That is the rule applied, not dodged.
