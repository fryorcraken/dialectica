# Reading one thread: how

## Context

See `proposal.md` for why. What shapes the approach:

- **The sibling exists.** `feed::list_threads` already does filter → verify →
  resolve version → resolve moderation → drop hidden → page, and
  `wire::list_threads_inner` already parses the request half. A thread read that
  looked unlike it would be two shapes for one job.
- **The log answers three questions and no more.** `OpLog` offers `get`,
  `iter`, `iter_stoa` and `iter_target` — and `iter_target` means "ops that
  *act on* this op", which `Entry::target` defines as `None` for a `Post`. **A
  reply is therefore not reachable from its parent through any existing read.**
  There is no index by parent, and adding one is `sqlite-projection`'s work.
- **`authoring.rs` deferred the thread audit to the read side and nothing has
  performed it.** The `thread` field inside a post's signed bytes is its
  author's claim; this is the first code that must not believe it.
- **The order is not this capability's.** `iter_stoa` already returns
  `cmp_ops` order, and `revision`, `moderation` and `feed` all take the position
  the log put an entry in rather than comparing anything themselves.

## Goals / Non-Goals

**Goals:**

- One core function answering a whole page of a thread, and one wire handler in
  front of it, shaped like the feed's pair so a reader learns one shape.
- Membership derived from the parent chain, with the walk terminating over any
  set of ops the log may hold — including adversarial ones.
- The three-valued moderation state and the two author fields reaching the wire
  without being flattened on the way.

**Non-Goals:**

- No index by parent, no projection, no caching. The read is O(ops in Stoa) per
  call over an in-memory or SQLite `iter_stoa`, which is what the feed already
  costs. Where that stops being enough is `sqlite-projection`'s problem, and it
  is the change that will have the index to fix it with.
- No change to `feed.rs`. Its author gap is real and is reported, not absorbed.
- No naming, no depth, no vote score, no attachment fetch, no edit history.

## Decisions

### 1. Membership is a chain walk with a visited set, run per candidate post

**Chosen:** for each authentic `Post` in the Stoa, follow `parent` until either
a post with no parent is reached (that op id is the thread) or the walk fails.
A walk fails when a parent is not held, names a non-post, or revisits an op id
already seen on *this* walk. A post whose walk fails is placed under no thread.

**Considered and rejected:**

- **Trust the `thread` field.** This is the attack. A peer signs an honest op
  naming any thread it likes; the signature verifies because the peer really
  did write it, and the post renders inside a conversation it was never part
  of. `a_forged_thread_claim_cannot_inject_a_post_into_a_thread` is the test
  that fails against this, and it was written and watched fail first.
- **Trust the `thread` field but check the parent's `thread` matches.** One
  link deep is not a chain: a reply to a reply whose *own* claim was wrong
  inherits the wrong answer and agrees with its parent about it.
- **A depth limit instead of a visited set.** A limit terminates, but it makes
  a legitimate deep thread stop being readable at an arbitrary line, and the
  line would be a number nobody could derive. The visited set terminates for
  the reason that matters — a chain either reaches a root or reaches an op it
  has already been to — and bounds the walk at the number of ops held.

The visited set is per walk rather than shared across walks because sharing it
would be a memo of "where does this post's chain end", which is a different
value per starting post; the correct shared memo is `op id -> thread`, and
that is the optimisation `sqlite-projection` should build with an index rather
than one this change should build without one.

### 2. Verification runs before the parent is read, on both ends of a link

`SignedOp::verify` is called on a candidate before its `parent` field is used,
and on each op reached along the chain before *its* parent is used. The spec
requires it ("so that a forged op cannot place a genuine one or be placed by
one") and the reason is that `parent` is a field like any other: it is a claim
until the signature is checked. A walk that verified only the starting post
would let a forged op with a chosen `parent` act as a bridge between two
genuine ones.

### 3. Two functions, not one: `thread_of` and `read_thread`

`thread_of(log, id)` answers "which thread does this op belong to, if any" and
is where the whole security property lives. `read_thread` is the page builder.
Separating them is CLAUDE.md's "a guard is a job": the question "is the
membership rule called everywhere?" has one place to look, and the chain walk
is testable on its own against a log full of cycles without building a page.

### 4. The root's absence is settled before anything else is read

`read_thread` resolves the named op first and refuses three ways — not held, held
but not a post, held but has a parent — before walking anything. Three refusals
because the spec requires three distinguishable messages, and because a root
that is not a root makes every later step meaningless.

The refusal for *a reply's op id* is the one worth naming: it is not an error in
the caller's data, it is a caller one level too deep, and the message says which
thread to read instead is derivable from the reply's own parent chain. (The
message names the mistake; it does not carry the thread id, because computing it
would be doing the caller's next call for them inside a refusal.)

### 5. A hidden root keeps its row and loses its body; a hidden reply loses its row

The asymmetry is the spec's, and the implementation makes it structural rather
than remembered: `ThreadItem::body` is an `Option<Sanitised>`, and `None` is the
only way a body is absent. A revision that cleared a body is `Some` holding an
empty string. The two states are therefore different values rather than the same
empty string with a flag beside it — the distinction the spec requires a caller
to be able to draw, made unrepresentable-if-wrong instead of checked.

`attachments` is `Option<Vec<Sanitised>>` for the same reason and by the same
rule: withheld is not the same as none.

### 6. Moderation reaches the wire as a tagged string plus an optional op id

`ThreadItem::moderation` carries `crate::moderation::Moderation` directly rather
than a `bool` plus an id. The type already distinguishes the three states and
already carries the deciding `Entry`, so re-deriving a pair from it would be a
second representation of a decision `moderation.rs` already made, and the two
could disagree. The wire shape is
`{"state":"unmoderated"|"hidden"|"unhidden"} (+ "decidedBy":"<hex>")`, with
`decidedBy` **omitted** when unmoderated rather than sent as `null`, which the
wire contract requires of a field with no meaning.

`NO SPEC:` the three state *spellings* are this change's choice. The spec
requires three distinguishable values and fixes no strings. A view branches on
them, so changing one later is a breaking change to the module surface.

### 7. The author is an address and a key, both taken from the verified op

Two fields on the item, `author` (address hex) and `authorKey` (public key hex).
Both are read from `entry.op.op.author`, which verification has already bound to
the key that signed. The key is not derived from the address and cannot be — an
address is a one-way hash — so this is a widening of what core returns and is
made deliberately.

`NO SPEC:` the field *name* `authorKey` is this change's choice; the spec
requires both values and names neither field.

### 8. Hidden replies are filtered before paging, and the root occupies slot zero

The sequence is built as: the root item first, then every placed reply in
`iter_stoa` order, hidden replies already removed. Paging then slices that one
sequence. Filtering after slicing is the bug the feed already pins
(`hidden_threads_are_dropped_before_paging_not_after`) and it is the same bug
here; building the root into the sequence rather than beside it is what makes
"concatenating the pages reproduces the read" true.

### 9. Nothing here sorts

The replies come back in the order `iter_stoa` returned them, filtered. There is
no `sort`, no `cmp` and no `max_by` in the new module, which is the discipline
`revision`, `moderation` and `feed` each hold and state. A second implementation
of the ordering rule could disagree with the first, and two orders that disagree
produce no error anywhere.

The cost is that the *chain walk* must not reorder either — so placement is
computed per entry as `iter_stoa` hands it over, and the entry's position is
never moved.

## Risks / Trade-offs

- **Quadratic-ish cost.** Placing N posts walks up to N chains of up to N
  links, each link an `OpLog::get`. → Accepted for this change: the feed is
  already O(N) with a version and a moderation resolution per row, and the
  in-memory log makes a `get` a map lookup. The projection with an index by
  parent is where this is fixed, and `proposal.md` records that the index is
  owed to whoever builds it. **A peer holding a very large Stoa will feel this
  before the projection lands**, and that is a real limit rather than a
  theoretical one.
- **A partial op set silently narrows a thread.** A reply whose parent has not
  arrived is returned under no thread, so a reader sees a smaller thread than
  a better-connected peer. → This is contracted, not hidden: the spec makes it
  the answer, `UI-BRIEF.md` tells the view to expect it, and the post becomes
  placeable the moment the parent arrives. The alternative — placing it by its
  claim — is the attack.
- **`decidedBy` names an op a caller may not hold.** → Same shape as a reported
  parent: it is an identifier, not a promise of delivery, and the spec already
  says so of parents.

## Where the spec is silent or disagrees with itself

Two things the implementation had to settle that the spec does not settle, both
marked in the code where a reviewer will find them.

### The spec names two different refusals for a revision's op id

"Reading by a revision's op id is not reading the thread" says the refusal "is
the one for a thread this peer does not hold". The refusals requirement says a
read is refused "when the op it holds under that id is not a post", that the two
SHALL be distinguishable, and that the message SHALL say "the op named is not a
post". A revision **is** an op held that is not a post, so one input has two
contracted answers.

**Resolved toward the requirement**, on the requirement's own argument: three
mistakes call for three responses, and telling a caller "this peer holds no op
under that id" about an op the peer demonstrably holds is false, and sends them
to wait for propagation of something that already arrived. Both halves of the
scenario's observable claim still hold — the thread is not returned, and the
reply is a refusal. `reading_by_a_revisions_op_id_is_not_reading_the_thread`
carries the argument. **The spec-writer should pick one.**

### A chain may cross a Stoa boundary mid-walk

The spec requires that only ops in the named Stoa are **returned**, and says
nothing about an op in another Stoa appearing as an intermediate **link** of a
chain whose two ends are in this one. `thread_of` follows `parent` alone, so the
bridge is accepted and the in-Stoa reply beneath it is placed; the foreign op is
never an item, because the Stoa filter is on what is iterated.

The alternative reading — refuse to cross a boundary mid-chain — drops the
deeper reply instead. Nothing in the spec picks between them, so
`a_chain_through_another_stoa_still_places_only_this_stoas_posts` is marked
`NO SPEC:` and asserts both what the spec does require and which way this chose.

## What this change found and did not fix

- **`feed::list_threads` returns an address and discards the public key**, so a
  feed row's generated name is uncomputable exactly as a thread item's would
  have been. Out of scope: `feed.rs` is not this piece's, and the feed read has
  no spec to modify. Reported rather than absorbed.
- **`docs/UI-BRIEF.md`'s vote-count passage said "there is no thread read at
  all"**, which this change makes false. The conclusion it supports — that no
  call returns a vote count — is untouched and still correct, so the clause is
  corrected in place rather than the paragraph rewritten.
