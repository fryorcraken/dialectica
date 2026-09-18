# Record four owner scope rulings on the MVP

## Why

The owner has made four rulings about what the MVP contains. All four are
**scope decisions — what ships first — and none is a design change.** Nothing is
withdrawn, nothing is deleted, and every capability named below stays merged,
tested and current.

They are recorded because scope that lives only in a conversation is scope the
next agent re-derives from the design documents, and the design documents argue
for building all of it. `docs/PLAN.md` §6 already carries one of these rulings in
exactly this shape — *"Out of the MVP, by owner decision — scope, not a design
change"* — and this change follows that model for the other three.

**The standing consequence, which is the reason the four cohere:** the MVP is an
MVP **of what core already serves**. Scope follows the contract rather than the
mockup.

## What Changes

Four entries in `docs/PLAN.md`, plus one new list section. **No code changes and
no spec delta.**

### 1. Voting is out of the MVP and out of the UI target

§7.2 specifies relevance ordering and vote weighting, and §9.2's MVP list carries
`Upvote / downvote` as item 5. **The design stays; its MVP membership does not.**

§7.2 opens *"This is where dialectica is investing"*, and that remains true of
the project. It is now false of the MVP, which is the thing worth being explicit
about — a reader taking that sentence as a staging instruction would build the
weighting first.

§9.2 already records that item 5 *"contradicts a §9.1 decision"*: §9.1 declined
to stage votes because §7.2 rule 2 ships no score, so a vote button publishes an
op that changes nothing a reader sees. **This ruling resolves that contradiction
in §9.1's direction.** The paragraph naming the tension stays — it is now
describing a settled question rather than an open one.

The contracts are untouched. `content-authoring` contracts publishing a vote;
`composer-view` contracts the control, including that it displays no score and
shows the viewer their own vote back. Both remain merged specs of behaviour the
system has.

### 2. Unread counts are out of the MVP

§9.1's open question #8 asks *"Whether `listThreads` needs an `unread` concept"*
and closes *"Until then the question is theoretical."*

It is no longer theoretical and no longer a question. **It is a decided
exclusion.** The entry moves from the open-questions list's register to a ruling,
keeping its reasoning — that unread needs peer-local, never-published state which
nothing in this design has yet required, and that inventing the first instance of
that as a feed field is how it gets designed badly.

The change in kind matters: an open question invites the next agent to answer it,
and this one is answered for the MVP.

### 3. Moderation stays out, and the MVP UI ships no moderation screen

§6 already carries the exclusion. **Two things are added.**

First, that the exclusion reaches the **UI**: the MVP ships no moderation screen.
§6 says *"no moderation UI and no moderation op-publishing path"*, so this is
consistent rather than new, but §9.2's MVP list is where a screen author looks
and it does not say so.

Second, and this is the part worth recording: **it is blocked at the contract,
not merely deferred.** The module trait in `dialectica/rust-lib/src/lib.rs`
exposes `publish_post`, `publish_reply` and `publish_vote` and **no
moderation-publishing method at all**. `publish_moderation` appears once in that
file, at `lib.rs:600`, and only hypothetically — as the fourth pair a reshaping
argument would have had to keep in step.

So a moderation screen is not a screen someone declined to write. There is
nothing for it to call. §6's read-path half — `moderation::resolve`, contracted
by `moderation-resolution` — is built and stays.

### 4. The core contract is the UI authority, and what an unserved control may do

**Where the design bundle asks for something core cannot honestly serve, the
bundle is amended.** Core does not grow to satisfy a mockup, and the sequencing
is: get an MVP in place with basic UI features first, then add the remaining core
features.

From that, a two-case rule for any control on a screen:

**Case 1 — core serves it: the control MUST be wired.** Inertness is not an
acceptable shortcut where the call exists; an unwired control is a defect rather
than a phase. The methods that exist today, read from the `Dialectica` trait in
`dialectica/rust-lib/src/lib.rs` rather than from any list:

| Method | Line |
|---|---|
| `get_capabilities` | `lib.rs:105` |
| `list_threads` (and its `includeHidden` argument — the SHOW HIDDEN toggle) | `lib.rs:121`, argument at `lib.rs:109` |
| `read_thread` | `lib.rs:150` |
| `create_stoa` | `lib.rs:175` |
| `join_stoa` | `lib.rs:193` |
| `list_stoas` | `lib.rs:211` |
| `generate_identity_slate` | `lib.rs:231` |
| `keep_identity` | `lib.rs:250` |
| `who_am_i` | `lib.rs:268` |
| `publish_post` | `lib.rs:288` |
| `publish_reply` | `lib.rs:303` |
| `publish_vote` | `lib.rs:316` |
| `display_name` | `lib.rs:341` |

**`publish_vote` is on that list and is the case worth reading carefully**, because
ruling 1 puts voting out of the MVP. The two do not conflict: ruling 1 decides
whether the MVP *has* a vote control, and case 1 decides what a control must do
*if the screen has one*. A vote control that ships must be wired, because
`publish_vote` exists.

**Case 2 — core cannot serve it: an inert control or a placeholder value is
acceptable for this MVP phase, on one condition — it is documented in
`docs/PLAN.md`.** The documentation requirement is what makes it acceptable; an
undocumented placeholder is still a defect, because it is indistinguishable from
one nobody noticed. §9.2 gains a **list section** for these, so the set is
findable in one place and can be worked off later rather than being rediscovered
screen by screen.

**Two live requirements narrow case 2, and a PLAN.md scope note does not override
them.** This is the boundary a later reader is most likely to get wrong, so it is
stated rather than left to be found:

- `stoa-navigation-view`'s **"Every number rendered is one this peer can actually
  answer"** forbids a rendered count on the Stoa list and the join preview,
  including substituting another call's page length.
- `composer-view`'s **"The vote control displays no score"** forbids a rendered
  score on the vote control.

Both are merged, tested contracts. Case 2 permits a placeholder where nothing
forbids one; where a requirement forbids one, the requirement governs, and
relaxing it is a spec change rather than a scope note.

## Where core cannot serve what the bundle asks

Each verified against the tree rather than relayed. These are the seeded entries
for §9.2's new list.

**No score anywhere.** `feed.rs:96-98` — *"No vote score. §9.1 is explicit that
votes are not staged... Nothing reads `Vote` ops, so no row carries a score."*
`wire.rs:1984` states the same absence structurally, across a post, a reply and a
vote alike: *"No score, count, tally, rank or position — on a vote reply least of
all."* `lib.rs:310-315` contracts it on the method: *"The reply carries an op id
and nothing describing an effect."* The UI already refuses to fake one —
`VoteControl.qml:17-23` records that a rendered `0` *"reads as the tally ZERO, a
claim that this post is known to have received no votes"*.

**Exactly one ordering, and no `order` parameter.** `feed.rs:38-41` — *"There is
no ordering parameter, no comparator to select, and no enum with variants nothing
implements."* `lib.rs:113-116` — *"There is no `order` argument, because core
computes exactly one ordering and an accepted-but-degraded parameter would be a
method telling its caller a falsehood."* **So "by relevance" is not
implementable**, which is ruling 1 seen from the contract side.

**No post count and no unread count.** `DStoaListScreen.qml:390-399` records that
nothing computes it: *"no call answers how many posts this peer holds for a Stoa,
and the thread listing reports whether a further page exists rather than a
total."*

**History is kept, but no contract method reads earlier versions.**
`revision.rs:11-13` states the three properties the module implements, the third
being *"History is kept. Superseded versions stay in the op log."* The trait
exposes no method reading them — `read_thread` is the thread read, and the table
above is the whole surface. So "read the earlier versions" is a case 2 control.

**No moderation-publishing method**, per ruling 3.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. `.openspec.yaml` sets `skip_specs: true` alongside the required `schema:`
key, with the measurement behind it.

**This change alters no behaviour and no requirement.** It records staging
decisions, and staging has never been a spec-level fact in this project:
`openspec/specs/` contracts what the system does, not which release it does it
in. Every capability the four rulings touch stays exactly as it is —
`content-authoring` and `composer-view` for voting, `moderation-resolution` for
moderation's read path, `feed-view` and `stoa-navigation-view` for what a screen
may render.

Ruling 4 is the one that could be mistaken for contract material, and
`.openspec.yaml` carries the argument for why it is not: its first case has no
fixed subject a test could enumerate (the set of wired-able controls changes with
every trait method added, which is this repo's `hand-maintained sweep lists go
stale silently` trap), and its second case conditions on a document outside
`openspec/specs/` entirely, which no requirement in this tree can hold.

## Impact

- **Modified:** `docs/PLAN.md` only.
- **No code changes.** No file under `dialectica/`, `dialectica-ui/` or
  `openspec/specs/` is touched.
- **Nothing is deleted or withdrawn.** Superseded passages are struck through and
  pointed at their ruling, per CLAUDE.md's *"Keeping this file true"*: strike and
  point rather than delete, so a question's history stays legible.
- **`FeedScreen.qml:344-350` is left alone.** Its note argues that an inert
  ordering row *"reads as a working control"* and is *"worse than absent"*. That
  is a local observation about one row, it is **not** carried into PLAN.md as a
  repo rule, and it is not cited as authority for one. A separate piece deals
  with the comment itself.
