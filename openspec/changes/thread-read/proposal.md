# Reading one thread: a root post and its replies

## Why

The MVP's item 9 is "view a feed; view a thread". The feed half is built —
`feed::list_threads` and `wire::list_threads_from_request` are on `main`. **The
thread half does not exist in any form.** `dialectica-ui/src/qml/Main.qml` says
so in a comment: there is no thread view, because it needs core methods that do
not exist. `docs/UI-BRIEF.md` specifies a "Thread — a post and its replies"
screen the view cannot render a single byte of, because Basecamp's sandbox means
every byte a view renders arrives through a core method.

A feed of thread heads that cannot be opened is a list of first paragraphs. This
is the one read that turns the built half into something a reader can use.

There is a second reason to do it now, and it is a security one. A reply carries
its `thread` inside its own signed bytes, and the publish path **derives** that
value from the parent rather than accepting it — but only for ops this peer
creates. An op arriving from a peer carries whatever its author chose, and the
log stores it, because the log decides nothing. `authoring.rs` records the
consequence and deliberately leaves it to the read side: *"an inbound op can name
a thread its own parent does not belong to [...] auditing a thread graph is a
read-side question about which ops render where"*. **This is the change that owes
that audit**, and nothing else in the system performs it. Until it lands, the
only written statement about thread membership is one that explicitly defers the
check.

## What Changes

- **One read: a thread, by the op id of its root post, paginated.** The root and
  its replies come back in one call, because every cross-module call is IPC and a
  method answering one post per call turns a thread of thirty into thirty round
  trips.

- **Membership is computed from the parent chain, never from the `thread` field
  an op carries.** A reply belongs to the thread its parent belongs to, resolved
  by following parents to a root this peer holds. A post whose signed `thread`
  field names one thread while its parent sits in another is placed by its
  parent, and the claim is discarded. This is the audit `authoring.rs` names and
  defers, and it is the requirement most worth reviewing hardest — it is the
  difference between a reply-forging peer being able to inject a post into any
  thread it likes and not being able to.

- **The items are flat, and each names its parent.** PLAN's Phase-3 section
  leaves "paginate by reply order or by reply tree" as a genuinely open question
  and says what would decide it: running against real traffic, which has not
  happened. A flat page carrying each item's parent is the shape that does not
  foreclose the answer — a view can render it as a list or nest it, and the core
  is not re-deciding when the answer arrives. A tree would have to invent a page
  boundary before anyone knows where one should fall.

  **Checked against the design bundle's screen 05, which is labelled
  "nesting".** The mockup does nest, and the flat shape serves it: in
  `reference/Dialectica App.dc.html` the thread's posts are four sibling
  elements in one linear sequence, nested purely by a left indent taking three
  values across the screen (`reference/reference-design.dc.html` carries the
  same bytes under a name with no space in it), with a rule drawn down the
  gutter. That is a flat
  list rendered at a depth, and depth is a function of the parent chain the
  items already carry — so the view computes it without core sending one. The
  mockup is therefore evidence *for* this shape rather than against it, and no
  requirement here reports a depth or an indentation level, because a depth core
  computed would be a second answer to a question the parent field already
  settles.

- **The author is reported as an address and a public key, not an address
  alone.** This is a correction rather than a widening, and it is made here
  because this is a fresh surface. A generated display name is
  `H(NAME_PREFIX || public_key)` while an address is a one-way hash of a record,
  and the two are deliberately independent digests — which means **a view
  holding only an address cannot compute the name.** `docs/UI-BRIEF.md`
  obligation 6 states this and concludes core must supply what the name needs.
  The bundle's `PostHeader.qml` is built around it, taking `generatedName` and
  `identityAddress` as separate properties, and its first implementation rule is
  "never render a name without its address beside it".

  So the read returns both. It returns the **key** rather than a rendered name,
  which keeps the derivation and the wordlist out of core and off the wire, and
  keeps to the rule that core sends identifiers rather than display text. The
  address stays and stays mandatory: it is the only unforgeable one, the mark is
  drawn from it, and a name without it is three recognition aids and no
  guarantee.

- **A thread whose root is hidden is returned, flagged, with its body withheld
  by default** — not omitted. The feed may drop a hidden thread from a list of
  many; a thread read that dropped its own subject would answer a caller with
  nothing and be indistinguishable from a thread that does not exist. That
  confusion is the one this project has already been bitten by.

- **A hidden reply is omitted by default and included, flagged, under the
  explicit parameter** — the same rule and the same parameter the feed uses,
  applied one level down.

- **An absent thread and an empty one are different answers.** A root the peer
  does not hold is a refusal; a root it holds with no replies is a served page
  carrying the root and nothing else.

- **No ordering parameter**, matching the feed. The order is the one core can
  honestly compute, and it is convergent rather than chronological.

## Capabilities

### New Capabilities

- `thread-read` — reading one thread: what identifies it, which posts belong to
  it, what order they come back in, how a page is cut, what a hidden root and a
  hidden reply each do, and what a caller is told when the thread is not held.

`openspec list --specs` was checked before naming it. Three near-duplicates were
considered and each rejected for a reason:

- **A `feed` capability does not exist** — the feed read is a named,
  acknowledged spec debt, and PLAN records both that it is owed and that it
  belongs to whoever next touches the feed read. **This change does not discharge
  it**, and the boundary is deliberate: this piece does not touch `feed.rs`, so
  writing the feed's contract here would promote requirements for code that
  merged in other pieces past the review those pieces had — the same argument
  that kept the debt out of the `core-e2e` change. What this change does instead
  is make the thread read's contract exist so the two are not both missing.

- **`post-revision` is not the place.** It answers "which version of this post
  do I render", per post. Which posts are in a thread is a different question
  over a different input, and folding it in would make a resolver about one op
  answer a question about a set.

- **`moderation-resolution` is not the place.** It answers whether a target is
  hidden. What a *thread read* does with that answer — withhold the root's body
  but keep the row, drop a reply — is this capability's, and stating it there
  would put a rendering decision inside the resolver that must not make one.

### Modified Capabilities

None, and two declinations are worth defending.

- **`op-format` is not amended.** The `thread` field already exists in the
  signed bytes and this change does not alter what an op may contain. What it
  adds is a *reader's* rule about how much weight that field carries, which is
  read-side behaviour and not a format change. The field stays in the op: it is
  the author's own claim, it is what a future per-thread routing split needs, and
  removing it is a wire-format decision this change has no business making.

- **`module-wire-contract` is not amended.** The thread read obeys the envelope,
  the pagination shape and the single error shape it already contracts, as a
  caller of them. Restating any of that here is how two copies drift.

## What the design bundle's screen 05 shows that core cannot answer

The mockup is a visual reference, and three things on it are not obtainable from
this read or from any merged contract. They are named here rather than
specified, so that nobody reads the mockup as a list of fields core owes.

- **A vote score beside every post** — the mockup's thread shows 12, 4, 1 and 0.
  Nothing in the merged contract counts a vote: publishing one is contracted to
  carry an op id "and nothing that describes an effect", and no ordering, count
  or tally reads a vote. A score is the in-flight relevance work's to define,
  and a thread read reporting one today would be the first surface to claim an
  effect publishing deliberately refuses to claim. **Not specified here.**
- **An attachment rendered with a file name, a size, and "held on this
  machine"** — attachments are out of the MVP, and this read carries an op's
  attachment references as sanitised strings without asserting anything about
  retrieving one, its size, or whether this peer holds its bytes. The mockup's
  attachment card needs a fetch path that does not exist.
- **"read the earlier versions"** — edit history is a separate call that is not
  built. This read reports *that* a post was revised, which is the honest amount
  and is what the mockup's `edited` label needs; the link beside it has nothing
  behind it yet.

One further mismatch is worth recording because it looks like a contradiction
and is not. The bundle's mark is drawn from the **address** while the generated
name comes from the **public key**. Both statements are correct — they are two
independent digests — and it is exactly why an author needs both fields rather
than either one.

## Impact

- `dialectica-core` gains a thread read, and `wire.rs` gains a method. This is a
  widening of the deliverable, made on purpose.
- **The feed read has the same author gap and this change does not close it.**
  `feed::list_threads` returns an address per row and discards the public key,
  so a feed row's name is uncomputable for the same reason a thread item's would
  have been; the UI's feed screen works around it by rendering an empty name.
  Fixing it there means touching `feed.rs`, which this piece does not touch, and
  the feed read has **no spec in `openspec/specs/` at all** — so there is no
  requirement to modify and nothing here cites one. That is its own piece, and
  it is reported rather than absorbed.
- The reply the view gets is the pagination shape the feed already established —
  `items` / `page` / `hasMore` — so no second precedent is set.
- **Out of scope, and must stay out:**
  - **Publishing anything.** No new op kind, no new publish path, no moderation
    action. The read renders what the log holds.
  - **Edit history.** Superseded versions stay in the log and a separate call
    would return them; this read reports only that a post was revised. That is
    the honest amount today, and returning every version of every post would
    return most of a thread twice for a facility most readers never open.
  - **A reply count and a most-recent reply on the feed's rows.** Both are folds
    over moderation-resolved state, both sit beside an ordering that carries no
    recency, and neither belongs to a change that reads one thread.
  - **Attachments.** Out of the MVP; a post is text. The read carries the
    attachment references an op holds, sanitised like any other peer-supplied
    string, and asserts nothing about fetching them.
  - **A vote score or tally on a post.** Nothing ranks a vote in the merged
    contract, and a thread read reporting one would be the first thing to claim
    an effect publishing deliberately does not claim.
