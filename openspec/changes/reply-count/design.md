## Context

See `proposal.md` for why a feed row needs a reply count and a latest reply, and
`specs/feed-read/spec.md` for what each must be. This records how they are
computed and why that way.

The constraints the code had to work inside:

- `feed::list_threads` already makes one pass over `OpLog::iter_stoa`, which
  returns ops in `cmp_ops` order. The module writes no comparison: its ordering
  is the log's, filtered.
- `thread::thread_of` is the parent-chain membership rule `thread-read` owns. It
  verifies every link, refuses a chain it cannot complete, and terminates on a
  visited set over any parent references a peer chose.
- `moderation::resolve` is resolved on read, every time, and never cached.
- The head loop already resolves every head in the Stoa on every call, whatever
  page was asked for. So a feed read already costs the whole Stoa.

## Goals / Non-Goals

**Goals:**

- Compute both values from the same two functions a thread read uses, so the
  feed and the thread screen cannot disagree about which replies exist.
- Make "a count of zero beside a latest reply" impossible to construct, rather
  than something each call site has to avoid.
- Keep `feed.rs` free of any comparison.

**Non-Goals:**

- Rendering either value. No view changes.
- Restricting the work to the requested page. Decision 5 says why not yet.
- Memoising chain walks within one read. See Risks.
- Fixing the thread read's reply order. That defect was found while building
  this, and Risks records it.

## Decisions

### 1. One fold over the Stoa, keyed by the root `thread_of` reaches

`visible_replies_by_thread` makes one pass over the entries `list_threads`
already holds. For each op that verifies and is a `Post` with a parent, it asks
`thread_of` for the root and `moderation::resolve` whether the reply is hidden.
It then adds the reply to that root's tally. `list_threads` hands each head its
tally with `HashMap::remove`.

The issue asked for exactly this membership, in its own words: *"via the same
parent-chain membership `thread-read` already establishes — not the post's own
`thread` field, which is an unverified author claim"*. It also asked for
moderation-resolved state: *"a count that includes hidden replies looks
entirely plausible and is wrong."* Both are met by calling the owning
functions, not by restating their rules.

**Alternatives considered:**

- **A thread read per row, counting its items less the root.** This would make
  agreement with the thread read true by definition, which is what the spec's
  count definition literally says. It was rejected on cost and fit. Each
  `read_thread` iterates the whole Stoa, so a feed of T threads costs T whole
  passes, not one. It also sanitises and formats every reply body only to throw
  them away, needs a `now_ms` the feed has no use for, and caps a page at
  `thread::MAX_PER_PAGE`, so counting "across all of its pages" would need a
  paging loop inside the feed.
- **A projection updated on append.** Rejected because moderation is resolved on
  read and never cached. A stored count would be wrong from the first hide or
  unhide until something recomputed it, and nothing would say so.

Because the per-row thread read was rejected, agreement rests on both reads
calling the same `thread_of` and `moderation::resolve`.
`the_count_agrees_with_the_thread_read` pins that agreement on one fixture
holding every case that subtracts: a hidden reply, a reply beneath it, a forged
reply, and a post that claims the thread in its `thread` field with no parent in
it.

### 2. The latest reply is the first visible reply the fold meets

The entries arrive in `cmp_ops` order, so the first visible reply met for a
thread is the one the ordering rule places first. `or_insert_with` records that
one, and every later reply only adds to the count. No value is compared.

**Alternatives considered:**

- **`max_by` with `cmp_ops`.** That would be a second site applying the rule.
  `feed.rs`, `thread.rs`, `revision.rs` and `moderation.rs` all hold the same
  discipline, for the reason `feed.rs`'s header gives: two orders that disagree
  produce no error anywhere.
- **Tracking the highest counter seen.** That reads the counter directly, and it
  would be a partial second implementation of `op-ordering`, without the op-id
  tiebreak and without the rule that ops carrying no counter sort below every op
  that carries one.

Because only posts are tallied, a reply's place is its own post's. A revision is
a different op, skipped by its kind, so revising an old reply cannot lift it to
the front. The id recorded is the post's for the same reason.

**What breaks without it:** replacing `or_insert_with` with an insert that
overwrites would make the last reply met win. That should turn
`the_latest_reply_is_the_one_the_ordering_rule_places_first`,
`replies_at_every_depth_are_counted_and_a_deeper_one_can_be_latest` and
`a_hidden_reply_is_neither_counted_nor_latest` red. **This is a prediction and
has not been measured.** The mutation was refused in the implementing session,
and the tester's run is what establishes it.

### 3. `FeedRow::replies: Option<Replies>`, with a `NonZeroUsize` count

The row carries one field, not a count and an optional id side by side.
`Replies` holds a `NonZeroUsize` count and the latest reply's hex id.
`FeedRow::reply_count()` and `FeedRow::latest_reply()` derive the two wire
values from it.

Two independent fields could disagree: a count of zero with a latest reply, or a
count of three with none. The wire would then send whichever disagreement the
struct held. With this shape, a row with replies has a latest one and a row
without has neither, and the non-zero count stops a `Some` from saying zero.

**What breaks without it:** no test turns red, because this is a type-level
guard. No current code path builds a disagreeing pair. Removing the guard makes
those states constructible, and the next code path to build a row would have to
avoid them unaided.

It also meant one new field for the three places that pin the row's shape: the
exhaustive destructures in `feed.rs`
(`a_row_carries_the_public_key_and_no_derived_display_name`) and in
`tests/end_to_end.rs` (the vote test), and the key-set assertion in `wire.rs`.

### 4. `latestReply` is omitted, never null. `replyCount` is always present

`feed_page_json` builds the row with `json!` and inserts `latestReply` through
`as_object_mut` only when there is one. Passing an `Option` straight to `json!`
would send `null`, which the wire contract forbids for a field with no meaning.

The two `if let`s are nested, not matched as one tuple, which is the shape
`thread_page_json` uses. The outer one cannot fail on an object the closure has
just built, and is an `if let` only because a panic aborts the module process.
The inner one is the one that varies. Written as
`if let (Some(map), Some(latest)) = …`, the two look equally likely to be `None`.
The tuple form behaves identically, and no test tells the two apart.

The key-set test now pins two exact sets. The row with no reply has no
`latestReply` key (`the_feed_reply_is_the_ecosystems_pagination_shape`). The row
with a reply has one and nothing else new
(`a_feed_row_with_a_reply_carries_latest_reply_and_nothing_else_new`). If
`latestReply` were sent as `null`, the key would be present, so the first set
would go red.

### 5. The fold covers the whole Stoa, not the requested page

Every reply in the Stoa is placed and resolved on every call. The head loop
already resolves every head in the Stoa on every call, so this keeps the read in
the cost class it was already in rather than adding a second one.

**Alternative considered: fold only the replies of threads on the page.** Every
reply would still need `thread_of`, because that is how its thread is known. So
the only saving is the `moderation::resolve` calls for replies under off-page
threads. Getting that saving means building rows before replies can be attached,
which needs a two-stage row type like `thread.rs`'s `Placed`. That is too much
structure to add before anything shows the cost matters.

**Observable consequence, not in the spec:** a store failure met on *any* reply
fails the read. That includes a reply whose thread is not on the page asked for,
or is hidden and not being returned. `feed-read` requires a failure met while
computing a row's reply fields to fail the read, and says nothing about these
cases. `a_store_failure_on_a_reply_off_the_page_fails_the_page` carries a
`NO SPEC:` marker and pins what the code does. The head loop already fails the
same way when an off-page head cannot be resolved, so the behaviour is
consistent. It is still a choice the spec should either make or refuse.

### 6. The argument in `feed.rs` against a reply count is withdrawn

`feed.rs`'s header argued that a fold over every reply's moderation state, per
row, *"for a number no caller has asked for, is not the smallest thing that
works — so it is not here"*. It added that the view rendering no count was
honest where a wrong one would not be.

Both premises have changed. Issue #100 is the caller: without the count, a row
cannot say whether anyone has answered. And a later `active` ordering needs the
latest reply. The issue put it this way: *"without it, an `active` ordering and a
`new` ordering cannot be told apart in what a row shows even once both orderings
exist."* The cost question the old paragraph left open, *"whether the count is
worth its cost"*, is answered by Decision 5: the fold stays in the cost class the
feed read was already in.

One part of the old argument survives, and the new header keeps it: both values
are folds over moderation-resolved state and never over raw ops. The "honest
rather than wrong" point now binds whatever first renders the count. `proposal.md`
records that the first view to render it has to answer in `feed-view` how a
count of one peer's copy is kept from reading as a total.

### 7. The fold verifies an op before reading its kind, although `thread_of` verifies it again

The fold verifies each op before matching its `Post { parent: Some(_) }` kind,
and `thread_of` then verifies the same op as the first link of its walk. The
fold's check is there because this codebase reads no field of an op before its
signature is checked, and the kind match reads `parent`. `read_thread` does the
same.

**Removing the fold's check turns no test red**, because `thread_of` refuses the
forgery anyway. That redundancy is recorded here so that nobody reads it as a
gap, and nobody deletes `thread_of`'s check thinking this one covers it.
`thread_of`'s check is the one that also guards every later link.

## Risks / Trade-offs

- **[Walking the chain is quadratic in its depth]** Each reply walks to its root,
  so a linear chain of N replies costs about N²/2 `get`s per feed read. A peer
  can publish such a chain cheaply. `read_thread` already pays the same cost per
  thread read, and the feed now pays it per feed read, over every thread in the
  Stoa. → Memoising each op's root within one read would make it linear. That
  belongs in `thread.rs` as a variant of `thread_of` that takes a cache. A second
  walk written in `feed.rs` would be a second membership rule. Deferred until a
  slow feed is measured, and recorded here so that measurement has somewhere to
  start.
- **[A store failure anywhere in the Stoa fails every page]** See Decision 5. →
  Consistent with how the head loop already behaves. The spec-writer is to
  decide whether it stands.
- **[The count reads as a total once rendered]** → Out of scope here, because no
  view renders it. `proposal.md` puts the obligation on whichever change first
  does.

### Found while implementing, outside this change: the thread read orders a reply before the reply it answers

`thread-read`'s scenario *"A reply orders after the reply it answers"* fails
against the current code. **This was measured, not read.** The fixture had a
root at counter 1, a reply to it at counter 2, and a reply to that reply at
counter 3. `read_thread` returned `[root, counter 3, counter 2]`, so the second
reply ordered before the reply it answers. `cmp_ops` places the higher counter
first ("newest first"), and `read_thread` pushes replies in that order after
inserting the root at the front.

No existing test pins that scenario, which is how the mismatch shipped. It is not
fixed here. The two ways to fix it are to reverse the thread read's reply
sequence or to amend the scenario, and choosing between them is outside this
change.

**This change does not depend on the answer.** `latestReply` is defined by
`op-ordering`'s "places first", and that is `cmp_ops`'s first, the highest
counter. If the thread read is reversed, the feed's latest reply becomes the last
of the thread read's replies. Each is still correct under its own spec, and
`the_count_agrees_with_the_thread_read` checks membership, not position.

## Migration Plan

None needed. The wire change is two added keys on a reply, and a view that
ignores unknown keys is unaffected. No op format, stored state or request shape
changes, so there is nothing to roll back beyond the code.
