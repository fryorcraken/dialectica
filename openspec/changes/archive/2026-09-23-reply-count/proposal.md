# A feed row reports its thread's reply count and latest reply

## Why

A feed row names a thread and shows its root post, and nothing on it says whether
anyone has answered. `listThreads` computes neither a reply count nor the thread's
latest reply, and `feed.rs` records the absence as a deliberate deferral. Issue
#100 ends it.

Both values have to be folds over **moderation-resolved** state, and that is why
they need a contract rather than just a field. A count that includes hidden
replies looks right and is wrong, and so does a "latest reply" that points at one.
Both also have to use the thread read's parent-chain membership. A count built on
each post's own `thread` field would let any peer inflate any thread's count, or
become its latest reply, by naming that thread in a post it never replied into.

The latest reply matters now for a second reason. A later `active` ordering, which
places the thread with the latest reply first, needs this value to exist. That
ordering is not built here, and this change only makes the field available for it.

## What Changes

- **Every feed row carries `replyCount`**: how many of the thread's replies this
  peer holds that are not hidden. Membership follows the parent-chain rule
  `thread-read` already contracts, and a reply's own `thread` field never counts.
  The count is defined as what a default thread read of that thread returns, less
  its root. So the feed and the thread screen cannot disagree about which replies
  exist.
- **A row carries `latestReply` where the thread has any non-hidden reply**: the op
  id of the reply that `op-ordering`'s rule places first among them. It is omitted,
  not null, when there is none. It names the reply post's own op id, and it is
  ranked by the reply's own position, not its revision's. Revising an old reply
  therefore does not make it the latest.
- **Neither value varies with the include-hidden flag.** The flag decides which
  rows appear. It does not widen what a row counts.
- **The count is over this peer's copy**, and the contract says so. It is not the
  thread's total size, which no peer can know.
- **A store failure met on any reply in the Stoa fails the read**, including one
  on a reply whose thread's row is on another page or is not returned at all. It
  is never reported as a smaller count or a missing row.
- **No ordering changes.** The feed still has one ordering and no ordering
  parameter. `active` stays unbuilt.

## Capabilities

### New Capabilities

- `feed-read`: reading a Stoa's feed of thread heads. For now it covers only the
  two per-row reply fields this change adds.

The name was checked against `openspec list --specs`, and the alternatives were
each rejected:

- **`thread-read`** owns reading *one* thread. A value reported on every row of a
  list of threads is a feed question, and adding it there would make one capability
  cover two reads. This capability depends on `thread-read`'s membership rule by
  name and does not restate it.
- **`feed-view`** is the screen's rendering contract. These fields are core's, and
  the view renders neither of them in this change.
- **A bare `feed`**, the name #91 uses in passing, would be the only capability
  for a core read whose name drops the `-read` its sibling `thread-read` carries.
  `feed-read` pairs with `thread-read` the way `feed-view` pairs with the thread
  screen.

### Modified Capabilities

None. `thread-read`, `op-ordering` and `moderation-resolution` are depended on
by name and none of their requirements change. `module-wire-contract`'s rule that
a meaningless reply field is omitted rather than sent as null is obeyed as it
stands.

## What #91 should pick up once this lands

#91 is the issue about there being no contract for the feed row's shape. It should
**add to `feed-read`** rather than open a second feed capability. When it does:

- **The closed field set includes `replyCount` and `latestReply`**, along with the
  fields a row already carries: `thread`, `currentVersion`, `author`, `body`,
  `attachments`, `isRevised` and `isHidden`. `latestReply` is the one field in the
  set that may be absent, and the closed-set assertion has to allow for that.
- **This capability's Purpose names what it does not yet cover**, and that
  sentence is the one #91 replaces.
- **The moderation shape divergence is #91's too.** `thread-read`'s requirement *An
  item's moderation state is three-valued and names the op that decided it* records
  that the feed reports a boolean where a thread item reports three states, and
  says whichever change closes the gap brings the feed up to the thread's shape.
  That change is the feed-row contract. This one is not, because it adds no
  moderation field.

## Impact

- **`dialectica-core`'s feed read** gains a per-row fold over thread membership and
  moderation state. The paragraph in the header of `feed.rs` that explains why no
  reply count or last reply exists has to go in the same change, along with any
  test that pins the row's fields by enumerating them. They would otherwise argue
  for an absence the code no longer has.
- **The wire reply for `listThreads`** gains two keys. The test pinning that row's
  exact key set changes with it. This widens the core API on purpose, as issue
  #100 asks.
- **No view change.** `FeedScreen.qml` renders neither field. Rendering a count
  raises its own honesty question, and whichever change first renders it has to
  answer it in `feed-view`: a number shown bare reads as the thread's total, and
  this count is only what one machine holds. `thread-read` refuses to report a
  thread's total for exactly that reason, and the thread screen already words its
  "more replies" notice around it.
- **Out of scope**: the `active` ordering, any ordering parameter, a vote score, an
  unread count, and anything about a reply beyond its op id, such as its author or
  an excerpt of its body.
