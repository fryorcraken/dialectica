# Design: the op log on disk, and the score column it deliberately does not have

## Decisions

### Decision: The §7.2 rule 5 answer — store the inputs and the decay epoch, never a score

**This is the change's central decision**, because §7.2 rule 5 names this change
specifically and says the alternative is expensive:

> §2.5's paginated API has to `ORDER BY … LIMIT` in SQLite, and a score
> recomputed from the current clock on every read cannot be indexed. Store a
> decay-free score plus a timestamp and apply decay in the `ORDER BY`
> expression, or bucket age coarsely and recompute on a timer. **Decide this
> when the projection schema is designed, in Phase 1** — retrofitting an index
> onto a time-varying score is the expensive version.

Nothing here computes a score. The decision is not "what is the score"; it is
"can the schema accept one without a migration".

**Chosen: rule 5's first option — decay separated from magnitude, with the decay
epoch stored and decay applied in the `ORDER BY` expression rather than folded
into a stored value.** The second option (bucket age coarsely, recompute on a
timer) is rejected below.

**But rule 5's phrasing has to be read carefully, because the schema does not
store a "decay-free score" either.** Rule 5 assumes the projection holds a
score, and while it was written §7.2 rule 2 shipped none. Since then the owner
has brought voting forward, with a moderator's vote weighted above an ordinary
one and §7.3's vouching weighting per reader — and that makes a stored score
wrong for a reason rule 5 does not contemplate. So this change takes rule 5's
**principle** (separate what decays from what does not; index over stored
values) and applies it to a schema that stores the score's *inputs*. The next
two sections are that argument.

#### What is reserved, and what the first draft got wrong

**An earlier draft of this decision reserved a `score REAL` column populated at
append. That was wrong, and recording why is more useful than the conclusion.**

The forum's owner has since decided that upvotes and downvotes are to affect
relevance sooner than §7.2 rule 2 contemplates, **with a moderator's upvote
weighted more heavily than an ordinary one**. That makes the reservation live
rather than precautionary, and it exposes the defect:

> `moderation.rs` decides moderator authority **on read, every time, from the
> genesis record** — nothing cached, nothing decided at append.

So **a vote's weight is not a property of the vote.** It depends on the
moderator set at the moment of reading. A `score` column multiplied at insert
would contradict the module this project landed to decide authority, and would
go silently stale the moment a moderator set changed — with no error anywhere,
which is this project's characteristic failure.

What is reserved instead is the **shape an aggregate can be built from**, not an
aggregate:

| Column | Type | What it is for |
|---|---|---|
| `author` | `BLOB NOT NULL` | The op's signer, lifted out of `op_bytes` so it can be indexed and joined. Stable at write time, unlike any weighting of it. |
| `score_epoch` | `INTEGER NOT NULL` | The time a score decays **from**. Written once at append, never rewritten. |

plus an index on `(target, author)` — "the ops about this subject, grouped by
who signed them" — so that a later ranking change can count votes **partitioned
by voter** and apply the weighting at query time against the moderator set it
resolves, which is the only shape consistent with authority being a read-time
decision.

**There is no `score` column and that is the decision, not an omission.**

#### `score_epoch` is a Lamport timestamp, and the wall clock is the trap

`score_epoch` is the op's **Lamport timestamp where the transport supplied one,
and `-1` where it did not** — never a wall-clock reading. Naming the wrong
answer matters as much as the right one here, because `CURRENT_TIMESTAMP` is the
obvious default for a decay column and it is barred:

- A wall-clock reading is **per-peer**. `arrival.rs` refuses
  `channelMessageReceived`'s `timestamp` for exactly this reason — "recording it
  here would be recording arrival sequence while believing we recorded a shared
  order" — and a decay epoch taken from the local clock reintroduces that one
  layer up.
- §7.2 rule 1 blesses two peers ranking differently, which is what makes this
  look permissible. It is not, and the distinction is worth stating in the terms
  it was put to us:

  > Rule 1 blesses divergence following from different op sets; **clock skew is
  > a wrong answer wearing its clothes.**

  That generalises past decay: any local reading that varies between peers
  holding identical ops is a defect, whatever it is dressed as.
- A Lamport timestamp is the transport's, identical on every peer that received
  the op, and is already the axis §7.2 rule 2's `new` and `active` orderings use.
- `-1` rather than `NULL`. A `NULL` in an `ORDER BY` expression propagates
  through arithmetic and lands the row wherever the surrounding `COALESCE`
  happens to put it; a sentinel below every real Lamport value places it where
  `cmp_ops` already places it — after every ordered op — in one place rather
  than at each future call site. This is the schema's one sentinel, and it is
  defensible only because the domain is `u64` and `-1` is outside it.

**One consequence to state rather than let a reader discover: decay ships at
`1.0` and is blocked indefinitely.** The op format forbids a self-asserted wall
clock, `op-ordering` forbids substituting a local one, and a Lamport timestamp
is a **counter, not a duration** — there is no conversion from "twelve Lamport
ticks" to "two days". So this reserves for a decay that cannot be computed until
the upstream gap closes. Reserving is still right: the column is one `INTEGER`,
and adding it later is the migration this decision exists to avoid. But nothing
here should be read as saying decay is imminent.

#### Why decay in the `ORDER BY` and not in the stored value

A stored `score · decay(now - epoch)` is a function of the read's clock. Every
row's value changes between two reads with no write in between, so no index over
it can be correct, and `ORDER BY score_now LIMIT 20 OFFSET 40` — which is exactly
what §2.5's `(page, perPage)` compiles to — degrades to a full scan and a sort on
every page. That is rule 5's "expensive version", and it is expensive in a way
that is invisible until a store is large.

With the decay-free value stored, the read is

```sql
ORDER BY score * <decay expression over :now - score_epoch> DESC, op_id ASC
```

which SQLite can serve from an **expression index** over the same expression
(`CREATE INDEX ... ON ops(score, score_epoch)` for the covering case, or a
generated column plus an index over it when the decay function is settled). The
index is over stored columns whose values change only on write. That is the
property rule 5 is asking for, and it holds because the two factors were
separated at the schema rather than multiplied before storage.

#### Why not the other option rule 5 offers

"Bucket age coarsely and recompute on a timer" was the alternative, and it is
worse here for a reason particular to this project rather than on general
grounds:

- **There is no timer.** A dialectica peer is a Basecamp module answering RPC
  calls; it has no scheduler of its own, and giving it one means a background
  thread in a module whose panic aborts the process (PHASE0-FINDINGS §3). A
  recompute pass is a write over every row, and a write that fails halfway
  leaves a store ranked by a mixture of two epochs.
- **Coarse buckets make the ordering jump.** A feed reordering in steps when a
  timer fires is a worse artefact than one that drifts smoothly, and it makes
  "why did this move" unanswerable from the data.
- **It does not avoid the index question**, it defers it: the bucket column still
  needs an index, and the timer still has to be right.

The cost of the chosen option is honest and worth stating: the decay expression
must be **index-compatible**, which constrains the eventual function to one
SQLite can evaluate from the two stored columns and a bound `:now`. That rules
out a decay depending on anything not in the row. It is a real constraint and it
is accepted, because it is the constraint that makes the ordering indexable at
all.

#### What a future scoring change still has to do

Reserving is not implementing. Stated precisely, so nobody reads this as more
than it is — and specifically enough that the view's author inherits the
constraints rather than rediscovering them.

**The aggregate belongs to the view, not to the log.** §3.3 puts the score on
the materialised view and §7.2 rule 1 agrees ("a score is a column in the SQLite
view"). This change builds the log. The shape the view will need is now known,
and is recorded here so it is not lost:

```
(stoa_id, target_op_id) → is_hidden, engagement_score, decay_epoch,
                          counts partitioned by voter class
```

Six things that change would still have to do, four of which are constraints
rather than work:

1. **Build that per-target table.** Nothing here aggregates anything; the log
   holds one row per op, which is what an aggregate is built *from*.
2. **Index `(stoa_id, is_hidden, engagement_score DESC, op_id ASC)`**, with
   `is_hidden` **leading** — so SQLite seeks to `(stoa, 0)` and walks, and
   `LIMIT 20` reads twenty rows rather than rank-filter-repeat.
3. **Keep the `op_id` tail.** It is not decoration. Equal scores are the
   *ordinary* case — every unvoted post ties — and without a total order
   `LIMIT`/`OFFSET` silently repeats and skips rows across pages. That is §2.5's
   paginated contract broken with no error, which is the same class of defect as
   an undefined sort order in `cmp_ops`.
4. **Write `is_hidden` by CALLING the moderation resolver**, never by a second
   copy of the authority rule in SQL. Two copies of an authority check is §6.2's
   measured defect with the sides swapped, and only one of them would be tested.
   It must also be **recomputed** rather than toggled from the arriving op's
   action — the arriving op may not be the deciding one — and it needs a **bulk
   sweep when a genesis record arrives after the ops it authorises**, which is
   not hypothetical: §4.7's backfill delivers history out of order by design.
5. **Never encode exclusion as a large negative score offset.** It is indexable
   too, which is what makes it tempting, and it silently restores the percentage
   haircut §7.2 rule 4 rejects — a sufficiently upvoted hidden post outranking a
   visible one.
6. **Settle the decay function**, subject to the index-compatibility constraint
   above and to the counter-not-a-duration problem recorded earlier.

**Why exclusion is tractable where decay is not**, since this looked at first
like rule 5 wearing another hat and is not:

> The property that makes something unindexable is not "resolved on read" — it
> is whether **invalidation points are enumerable**. A decayed score changes
> every row's key continuously, with no finite fix-up set. The hidden set is
> small, bounded, and changes **only when a moderation op arrives.**

So `is_hidden` is an ordinary materialised column with a known invalidation
trigger, and the read-time authority check is preserved by *calling* the
resolver rather than by giving up on the index.

#### The boundary: inputs are the log's, conclusions are the view's

§3.3 distinguishes the **op log** (the authority) from the **materialised view**
(a cache rebuilt by replay). This change implements the log, so the line has to
be drawn somewhere, and it is drawn here:

> **The log stores inputs. The view stores conclusions.**

The log's rows carry everything an aggregate could be built from — who signed
each op, what it acts on, when the transport placed it — and no aggregate. No
summed score, no vote tally, no `is_hidden`.

That line is not merely tidy. It is what keeps a promise §7.2 rule 1 makes:

> ranking can be retuned without a protocol version bump, which is the property
> you want for the one part of the system that will be tuned repeatedly.

**That promise rests on an assumption rule 1 does not state: that the projection
stores inputs rather than conclusions.** A pre-summed `weighted_total` bakes the
multiplier into stored rows, so retuning the weights means rewriting every row —
and the promise is false. Storing raw facts partitioned by voter keeps it true.

This was chosen for the staleness reason given above — a stored weight goes
wrong when a moderator set changes — and it turns out to be right for this
second reason as well. Two independent arguments for one shape is worth
recording, because the next author will meet only one of them and might think it
the whole case.

**And a third, which is the one that makes this structural rather than
prudent.** The owner has since added **vouching**: a reader may vouch for an
identity, whose votes then carry more weight *for that reader*. Three weight
classes — moderator, vouched, plain — and a vouch is **never published**, because
§5.2 makes identities unlinkable across Stoas by construction and a travelling
vouch list would re-link those pseudonyms using the reader's own social graph.
So a vouched set is per-reader local state.

The consequence for this schema:

> A vouched voter's weight is not a property of the vote, **or of the Stoa** — it
> differs **per reader**.

Under a moderator set alone, a summed score is *stale* when the set changes.
Under vouching it is **unanswerable**: two readers of the same peer's own
database compute different weights from the same rows, so there is no single
correct value a `weighted_total` column could hold.

**So: never pre-sum the vote counts into a score column.** A future contributor
looking at a raw-counts table will see an obvious optimisation, and this is what
it would break — not a performance trade, a correctness one. Storing raw facts
partitioned by voter is what makes a per-reader weight expressible at all, and
the class count in the query-time join simply goes from two to three. No column
changes, no index changes; the shape was already right.

**What this change is therefore NOT reserving:** any per-target aggregate table.
Its shape is now known (recorded above) and it is the view's column set to
create. Reserving it here would be building half of someone else's change on a
guess about the other half.

### Decision: `rusqlite` with `bundled`, argued against the keystore posture

The keystore change set the standard for this and stated the posture once so it
is not re-derived:

> a dependency must be either unavoidable (a primitive we must not hand-roll) or
> strictly smaller than what it replaces. Convenience is not a reason, and
> "defence in depth" is not a reason when the depth is already there.

| Crate | Why it cannot be avoided | Surface taken |
|---|---|---|
| `rusqlite` | §3.3 specifies SQLite by name, and the alternative to a SQLite binding is writing a durable indexed store by hand — a B-tree, a write-ahead log and a crash-recovery path, on the component that is the authority for all forum state. That is not a candidate. | `bundled` only. 43 of its 48 features are off, including `serde_json`, `chrono`, `time`, `uuid`, `url`, `blob`, `hooks`, `functions`, `vtab`, `backup` and `trace`. |

**`bundled` is load-bearing, not a convenience**, and it is the part of this
decision most likely to be questioned, so the argument is here rather than left
implicit:

- Without it, `libsqlite3-sys` links the **host's** libsqlite3. The Nix build
  path has no override point for that — `mkLogosModule.nix` owns the vendoring
  (`dialectica/flake.nix` records that a module's flake has no escape hatch on
  the `codegen.rust` path), so there is nowhere to declare a `buildInputs`. A
  system-library dependency would surface as a link failure inside a build whose
  inputs this repo cannot edit.
- It also removes a whole class of correctness question. A store read by whatever
  SQLite the host happens to ship is a store whose behaviour varies by machine —
  and this store is the authority for forum state. `bundled` makes the SQLite
  version a property of `Cargo.lock`, which is the same guarantee every other
  dependency here gives.

  **A review made that concrete and it is sharper than stated above:** the schema
  uses `STRICT`, which SQLite gained in **3.37 (2021)**. Against a host library
  older than that, schema creation fails outright — so without `bundled` this
  module would not merely behave differently on an old host, it would not open a
  store at all.
- The cost is honest: it compiles the SQLite amalgamation from C, which pulls
  `cc` and adds build time. **Verified to build here**, producing
  `libsqlite3.a`; `cc` is present in the toolchain the `rust` CI job uses.

**What was refused**, since the refusals are the part worth recording:

- **`serde_json` as a rusqlite feature.** The crate already depends on
  `serde_json` directly, and enabling rusqlite's integration would let a `Value`
  be bound to a column — inviting ops to be stored as JSON. Ops have a canonical
  byte encoding (`op.rs`) that is the signature preimage; storing them as
  anything else means a re-encode on read that could differ from the bytes
  signed. The column is a `BLOB` of `to_bytes()` and nothing else.
- **`chrono` / `time`.** Nothing here reads a clock — see the `score_epoch`
  argument above, which is precisely a refusal to.
- **A migration framework** (`refinery`, `rusqlite_migration`). There is one
  layout version and one table set. A framework for a sequence of one is larger
  than what it replaces, which is the posture's second test failing. The version
  is a `PRAGMA user_version` read and a comparison, in ten lines.
- **`r2d2` / a connection pool.** One `Connection`, owned by the log, used behind
  `&mut self` on write and `&self` on read. A pool solves a concurrency problem
  this type does not have.

### Decision: Reads return owned values

The `op-log` design named this as the thing most likely to need redrawing, and it
was right:

> if the SQLite implementation turns out to want a different shape (streaming
> reads rather than `Vec<&Entry>` [...]), the trait was drawn at the wrong
> altitude and should be redrawn with two implementations in hand

**`Vec<&Entry>` is not implementable by a database**, and the reason is not
performance. A borrowed reference must point at something the log already owns.
`MemoryOpLog` owns its entries so it can lend them; `SqliteOpLog` owns rows in a
file, and a `&Entry` would have to point into a cache the log materialised for
the call — which means the cache must outlive the call, which means it lives in
the struct, which means a read needs `&mut self` or interior mutability, and the
next read invalidates the last read's references. Every route ends somewhere
worse than owning.

Four candidates were evaluated:

| Candidate | Why not |
|---|---|
| `Cow<'_, [Entry]>` | Would be `Borrowed` only if an implementation already holds its entries **contiguously and in `cmp_ops` order**. `MemoryOpLog` holds a `HashMap` and sorts on read, so it constructs a `Vec` either way and returns `Owned` every time. A `Cow` that is always `Owned` is a type-level claim of an optimisation that never occurs. |
| `impl Iterator<Item = Entry>` | Not object-safe, so `&dyn OpLog` stops existing; needs a GAT or a named associated type per method to be written in a trait at all. And SQLite's own shape is to materialise from a prepared statement, so the laziness is not free — it would hold the statement, and therefore the borrow, across the caller's fold. Complexity paid for a property neither implementation has. |
| `Box<dyn Iterator<Item = Entry> + '_>` | Object-safe, but allocates per read like the `Vec` does and loses `len()`, `is_empty()` and slice indexing at every call site. The `Vec` with the allocation is the same cost with a better API. |
| **`Vec<Entry>`** | **Chosen.** Both implementations produce it naturally. |

**The cost is a clone per entry per read in `MemoryOpLog`, and it was measured
rather than assumed**, because the brief asked and because §7.2's paginated reads
are the thing that would feel it. See the "What the clone costs" section below.

### Decision: every method is fallible, and a `Result` rather than a fourth variant

**This is forced and not a style choice.** `rusqlite` returns `Result` from every
call, and PHASE0-FINDINGS §3 measured that a panic **aborts the module process**
— the caller is told `timeout` after 20 seconds, the next call reports
`MODULE_NOT_LOADED`, and the word "panic" appears only in a daemon log. An
`unwrap` on a disk error is therefore a denial of service against the peer,
reachable by filling a disk. There is no `unwrap`, no `expect` and no `panic!`
in `sqlite.rs`'s non-test code.

`MemoryOpLog` cannot fail and returns `Ok` unconditionally. That asymmetry is
real, and the alternative — a fallible trait for one implementation and an
infallible one for the other — is two traits, which is no trait. A `Result` that
is always `Ok` on one side is the cheap half of the trade.

**`get` becomes `Result<Option<Entry>, _>`** rather than `Option<Result<..>>`.
The outer layer is "could the store be consulted", the inner is "did it hold
this" — and only that nesting lets a caller write `log.get(id)?` and then match
on a genuine absence. Inverted, every caller unwraps a failure to ask a question
before learning whether the question was answerable.

#### Why not a `Moderation::Unreadable` variant

This was proposed on fail-closed grounds — a read failure means the resolver
**cannot say** whether a target is moderated, which under `moderation.rs`'s own
posture is closer to "treat as hidden" than to "treat as unmoderated" — and a
variant makes the decision unrepresentable rather than pushing it to a caller
who may get it wrong.

**It was investigated and it fails open.** `Moderation::is_hidden` is
`matches!(self, Moderation::Hidden(_))`, so a new variant answers `false` there
and **every existing caller silently renders the post**. The repair makes it
worse: forcing `is_hidden` to answer `true` for `Unreadable` makes a disk error
indistinguishable from a moderator's decision at the one call site that matters,
and leaves `deciding_op()` returning `None` for something reported as hidden — a
state the type currently makes impossible. Adding a variant to gain fail-closure
would have destroyed an invariant the type already enforces.

`Result` has the opposite failure mode: **a caller who ignores it gets a compile
error, not a wrong render.** The fail-closed decision then belongs to the caller
that knows what it is rendering, and the compiler makes it impossible to skip.

That is this project's house pattern rather than a new idea, and it is worth
naming as such because this is its third instance: `Entry::target`'s match
carries no wildcard, so a new op kind forces a decision; `Moderators::of` is the
sole constructor, so a moderator set cannot be built without a verified genesis
record; and now this. **Make the mistake unrepresentable rather than unlikely.**

#### What a caller should do with an `OpLogError`

Recorded because there is no caller yet, and a `Result` nobody handles
meaningfully becomes `.unwrap()` at the first call site that meets it.

At the wire boundary a storage failure becomes `{"error":"..."}` — §2.5's shape,
never a partial success. **Specifically not an empty feed**: an empty feed is
indistinguishable from a Stoa nobody has posted in, so swallowing this would
render a forum whose store is broken as a forum that is merely quiet. That is
the same confusion the `Result`/`Option` nesting exists to prevent, reintroduced
one layer up.

`guarded` is not the mechanism. It converts a *panic* into the error shape; this
type exists so that there is no panic to convert.

### Decision: `cmp_ops` order is produced by SQL, from a materialised sort key

**This was a real question with a real cost either way**, so both sides are here.

`cmp_ops` is defined in Rust over `Option` fields: descending Lamport with
ordered ops before unordered ones, then ascending message id with
id-bearing before id-less, then ascending op id. Two ways to get it out of
SQLite:

**(a) `SELECT` everything matching, sort in Rust with `cmp_ops`.** The comparator
stays the single definition of the order — `log.rs`'s `sorted` already exists and
the SQLite implementation would reuse it verbatim. Nothing can drift, because
there is nothing to drift from.

The cost is that it is **unindexable by construction**. Every read loads every
matching row into memory and sorts it, so `iter()` over a large store is O(n) rows
materialised and O(n log n) comparisons regardless of how few the caller wants.
§2.5's paginated reads would then be "load everything, sort, discard all but
twenty", which is the shape §7.2 rule 5 exists to forbid one layer up. Choosing
this for the ordering while reserving an index for the score would be incoherent:
the same paginated read needs both.

**(b) Materialise a sort key at append, `ORDER BY` it in SQL.** Chosen.

Three stored columns, written once per op and never rewritten (first-wins makes
that sound — the arrival is fixed at first append):

| Column | Value | Why |
|---|---|---|
| `sort_lamport` | `-(lamport as i128)` clamped into `INTEGER`, or `i64::MAX` when unordered | One column expressing both "ordered before unordered" and "descending Lamport". Ascending over it is `cmp_ops`'s first two branches. |
| `sort_msg` | the message id bytes, or `X''` … see below | Ascending, with a sentinel for absent |
| `op_id` | the op id bytes | The last resort, ascending — already the primary key |

so that `ORDER BY sort_lamport ASC, sort_msg_present DESC, sort_msg ASC, op_id ASC`
is `cmp_ops`, expressed over stored values, and **indexable**. An index over that
tuple serves `iter()` directly, and with `stoa` or `target` prefixed it serves the
restricted reads.

**The danger this creates, stated plainly: two definitions of one order.** The
comparator in `arrival.rs` and the `ORDER BY` here must agree, and a compiler
checks neither against the other. That is exactly the "fourth slightly-different
guard" CLAUDE.md warns about, and it is the price of (b).

**What makes it survivable is a test rather than a comment.** The contract suite
runs every ordering assertion against both implementations, and one test builds
a population crossing every branch of `cmp_ops` — ordered/unordered, with and
without a message id, equal Lamport values, distinct op ids — appends it to both
logs, and asserts the two sequences are identical element for element. A
divergence between the SQL and the comparator is then a failing test naming the
pair that disagreed, not a rendering difference in production. **`cmp_ops`
remains the definition**; the `ORDER BY` is an implementation of it that a test
holds to account.

Two details where the encoding is subtle enough to be worth writing down:

- **Descending Lamport by negation, not by `ORDER BY ... DESC`.** A `DESC` on the
  Lamport column alone cannot also express "unordered ops last", because the
  sentinel would have to be simultaneously the largest value for one purpose and
  the smallest for the other. Negating at write time makes one ascending sort
  correct for both, which is what lets a single index serve the whole clause.
  `u64::MAX` negated does not fit `i64`, so the value is stored as
  `i64::MIN + 1 + (u64::MAX - lamport)` mapped into range — a monotone decreasing
  map from `u64` into `i64` with `i64::MAX` left free as the unordered sentinel.
  The mapping is one function with its own test asserting it is order-reversing
  across `0`, `1`, `u64::MAX - 1` and `u64::MAX`.
- **The message-id sentinel is a separate column, not a magic byte string.**
  `cmp_tiebreak` puts an op *with* a message id before one *without*, and no
  byte string is "less than every possible byte string but not equal to the empty
  one" — the empty `MessageId` is a legal value (`storing_adversarial_ops_never_panics`
  constructs it). So presence is its own `INTEGER` column sorted `DESC`, and the
  bytes are compared only among rows that have them. A single column with a
  sentinel would make the empty message id collide with absence, which is
  `absence_is_not_equal_to_a_zero_lamport_timestamp`'s defect in another costume.

### Decision: Dedup is the `PRIMARY KEY`, and the causality is the other way round this time

The `op-log` design was explicit that its `HashMap` acquired the anti-duplicate
property by accident and warned the next author not to inherit the claim:

> The `HashMap` was written first, for §3.1's idempotence alone. The `cmp_ops`
> precondition was raised mid-change [...] The safety was discovered, not
> designed for. An author who believes the map was chosen for the precondition
> will assume any dedup-ing schema inherits the property. It does not follow
> automatically.

**Here it is the other way round, and saying so is the point.** `op_id BLOB
PRIMARY KEY` was chosen *because* the precondition had already been stated. Both
properties were required before the `CREATE TABLE` was written:

1. §3.1's idempotence — one op id, one row.
2. `cmp_ops`'s precondition — no two entries sharing an op id may reach the
   comparator, or the ordering is decided by something undefined.

A `PRIMARY KEY` gives both, and the design warning names the shape that would
give only the first: "a schema that dedups on write but materialises rows into an
intermediate list before ordering — a `SELECT ... ORDER BY` over a table with a
**non-unique** index, say — would re-open exactly this hole". The ordering index
here is over `(sort_lamport, sort_msg_present, sort_msg, op_id)`, whose last
component is the primary key, so it is unique by construction and no
`SELECT ... ORDER BY` over it can emit one op id twice.

**First-wins is `INSERT OR IGNORE`**, and `changes()` reports which happened. Not
`INSERT OR REPLACE`, which would overwrite the stored arrival with the later
one — richer-wins by the back door, and the one rule `arrival.rs` warns two
modules' worth of prose about. The distinction is pinned by a test that appends
unordered-then-ordered and asserts the *poorer* arrival survived; that direction
is the only one where first-wins and richer-wins disagree, which the `op-log`
change discovered the hard way.

### Decision: One contract suite, run against both implementations, as ordinary functions

A trait with two implementors is worth having only if the second is a drop-in,
and the only way to know is to run the same assertions against both.

**The mechanism is a plain generic function per behaviour, called from two
`#[test]`s** — not a macro that generates tests, and the reason is a CI gate
rather than taste. `ci.yml`'s test-count step counts `^\s*#\[test\]\s*$` textually
across the sources and fails the build when the number cargo ran differs:

> cargo ran {ran} tests but the source declares {declared}. A test that is
> compiled out (a cfg, a removed mod) does not fail — it silently stops being
> checked.

A `macro_rules!` block containing `#[test]` and invoked twice declares one
matching line and runs two tests. The gate would fail — correctly, by its own
logic, for a reason having nothing to do with a compiled-out test. That is the
"a gate can fail for a real reason while naming the wrong cause" family, and it
is avoidable by not reaching for the macro: every contract test is
`fn behaviour<L: OpLog>(log: &mut L)`, with `#[test] fn behaviour_in_memory()`
and `#[test] fn behaviour_in_sqlite()` calling it. Two `#[test]` lines, two
tests, and the gate counts what it thinks it is counting.

It also reads better at a failure: the failing test name says which
implementation broke.

#### Were the existing tests a contract, or implementation tests in disguise?

The question worth asking before running them against a second implementation,
because a test that asserts *how the map behaves* rather than *what a caller
observes* will either fail against SQLite for the right reason or pass for the
wrong one.

**The answer: they were a contract, with two exceptions, and neither exception
is a defect.** Every behavioural test in `log.rs` moved to `contract.rs` and
runs against both implementations unchanged — the assertions are byte-identical,
only `MemoryOpLog::new()` became a parameter. 72 tests, 36 behaviours, both
implementations, and **none of them had to be weakened to make SQLite pass.**

That is a better result than expected and it is worth saying why: the `op-log`
change wrote its tests against the spec's scenarios rather than against the
`HashMap`, and it shows. The one place the in-memory implementation leaks into a
test name — `no_two_entries_in_a_read_ever_share_an_op_id`, which exists because
a map key makes the duplicate unconstructible — turns out to be a genuine
contract requirement that SQLite satisfies by a different mechanism (a unique
ordering index). The test did not need changing; only the *reason* it passes
did, which is exactly what a second implementation is supposed to expose.

**The two exceptions, and why they are not exceptions to the contract:**

- **`an_entry_reports_the_target_its_kind_names` is not about the log.** It
  constructs an `Entry` and calls a method on it. No implementation is involved,
  so there is nothing to run twice. It stays in `log/mod.rs`.
- **`two_logs_with_the_same_ops_read_the_same_order` is about two logs**, so it
  is already a cross-instance test. Running it "against both" would mean four
  logs and would assert nothing the agreement tests do not assert better.

**What genuinely cannot be generic**, stated so the asymmetry is visible rather
than inferred:

- **Persistence.** `MemoryOpLog` has no storage to reopen. The spec states these
  requirements as applying to "an implementation that persists", which is the
  honest scope rather than a carve-out.
- **The layout version.** Same reason: there is no layout to version.
- **Failure reporting.** There is no way to make a `HashMap` fail, so
  `OpLogError`'s variants can only be exercised on the SQLite side.
- **The sort key.** `SortKey` is `SqliteOpLog`'s internal, and testing it
  through the trait would be testing the ordering twice over. It is tested
  directly *and* through the agreement test, which is the pair that matters: one
  says the key is right, the other says the key is what the reads use.

**The original `MemoryOpLog` tests were kept rather than deleted.** They now
duplicate the contract suite's in-memory half, which looks like redundancy and
is one thing: their failure output names the concrete type and the original test
name, so a regression in `MemoryOpLog` reports as a `MemoryOpLog` failure rather
than as one arm of a generic function. The duplication costs a few seconds of
suite time and buys a clearer failure; if it ever becomes a maintenance burden,
they are the copy to delete.

### Decision: The schema version is `PRAGMA user_version`, refused rather than migrated

One `INTEGER` SQLite already maintains per database file, read on open and
compared against a `const`. A store written under a version this build does not
know is **refused by name**, reporting both numbers.

Refusing rather than best-effort reading follows from what this table is: ops are
the authority for all forum state, so a layout misread produces a forum that is
wrong with no error anywhere — the failure mode this project's security posture
is organised around. A store that cannot be read is a visible problem; a store
read wrongly is not.

The constant is pinned by a hardcoded `assert_eq!`, following `identity.rs`'s
wire constants, because `cargo mutants` mutates functions and not `const`s and
this project has already shipped a `VERSION_1` defect that left the suite green.

## Mutation testing: two survivors, and what they were hiding

Run by hand rather than through `cargo mutants`, because the interesting
mutations here are in **SQL string literals and a schema**, which it does not
mutate at all — it mutates function bodies. That is a coverage gap worth naming:
the ordering, the primary key and the version check are all expressed as text
that `cargo mutants` is structurally unable to see.

The table lists **survivors as well as kills**, because a table of only kills
hides its own misses. Each mutation was applied, the suite run, and the source
restored.

| Mutation | Result | Killed by |
|---|---|---|
| `iter_stoa` matches a **2**-byte prefix | **SURVIVED** (review) | now 1 test |
| `iter_target` matches an **8**-byte prefix | **SURVIVED** (review) | now 1 test |
| `score_epoch` sentinel `-1` instead of `NULL` | **SURVIVED** (review) | now 1 test |
| Drop `sort_msg` from the `ORDER BY` | **partially survived** (review) | now 3 tests incl. the headline one |
| `PRIMARY KEY` on `op_bytes` instead of `op_id` | **SURVIVED** | now 1 test |
| Accept any layout version below this build's (`!=` → `>`) | **SURVIVED** | now 1 test |
| `sort_ordered` removed (unordered filler `1` → `0`) | not run initially | now 5 tests |
| Index `sort_msg_present DESC` → ascending | **SURVIVED** (review) | now 1 test (query plan) |
| `iter_stoa` matches a **1**-byte prefix | killed | 1 test |
| `op_id ASC` → `op_id DESC` in the `ORDER BY` | killed | 3 tests |
| `INSERT OR IGNORE` → `INSERT OR REPLACE` | killed | 9 tests |
| Keep the message id in an unordered op's sort key | killed | 3 tests |
| Store the sort key's message id as the recorded arrival | killed | 2 tests |

**Six survivors in total, five of which a review found after this table first
claimed two.** That is the honest number, and the pattern in it is the lesson:
every single one was the *one step weaker* form of a mutation already recorded
as a kill. A one-byte prefix died; two bytes lived. Removing the primary key
died; moving it lived. Refusing nothing died; refusing only the future lived.

**The four the review found, and why each hid:**

- **Prefix matching at 2 and 8 bytes.** The fixtures picked two titles whose
  hashes happened to agree in byte 0 and *guarded that coincidence* — the guard
  documented the luck it had as though it were the property the test needed.
  Fixed by **constructing** the addresses and ids from raw bytes rather than
  hunting hash collisions, with the shared length a named constant
  (`SHARED_PREFIX_BYTES`) and guards pinning both that the prefix agrees *and*
  that the next byte differs. A 9-byte hash agreement is not findable by trying
  titles, which is why the old approach could not have been strengthened in
  place.
- **`score_epoch = -1`.** `u64::MAX as i64` **is** `-1`, so an unordered op and
  one ordered at the maximum stored the same value. This is the exact in-band
  sentinel collision `lamport_sort_key`'s own documentation rejects for its own
  column — solved correctly there with `sort_ordered`, and reintroduced one
  column over. It hid because the column is reserved: *"nothing writes this and
  no read consults it"* is a comment that excuses a column from every
  behavioural test while the rows go to every peer's disk. Inert today; a §7.2
  ranking defect the moment rule 5 lands, in rows already written.
- **Dropping `sort_msg`.** Killed two tests but **not the headline agreement
  test**, whose entire purpose is catching divergence. `every_ordering_shape`
  used only `vec![seed; 32]` and `vec![]`, so every pair was equal-length or
  empty and length-vs-lexicographic could never disagree. Fixed by adding
  prefix-related, differing-length ids (`[0x01]`, `[0x01,0x00]`, `[0x01,0xFF]`,
  `[0x02]`) at one Lamport value.
- **The index's `DESC`.** Results identical, `EXPLAIN QUERY PLAN` shows
  `SCAN USING COVERING INDEX` becoming `... USE TEMP B-TREE FOR LAST 3 TERMS` —
  precisely the degradation the sort key exists to prevent, invisible to every
  gate because only the *cost* changes. Now asserted directly.

**And one this change had not thought to run:** removing `sort_ordered`. The
review noted that no test used `i64::MAX as u64`, the one Lamport value whose
key collides with the unordered filler — so the leading column's necessity
rested on a comment. Now asserted as a collision, so a future edit removing the
column fails with a name that points at the cause.

**Every replacement test was watched failing under its mutation before being
kept.**

**The two this change found for itself**, both also the one-step-weaker form:

- **"Remove the `PRIMARY KEY`"** dies on every dedup test. **"Put it on
  `op_bytes`"** survived the entire suite, because the two columns agree on
  every input the public API can construct — identical ops have identical bytes.
  It matters because §3.1 makes the op **id** the identity, and an id is a
  domain-separated hash rather than the bytes themselves, so keying on the
  encoding means any future encoding change that is not identity-preserving
  splits one op into two rows and violates `cmp_ops`'s precondition silently.
  Killed now by `the_primary_key_is_the_op_id_and_not_any_other_column`, which
  asserts against `pragma_table_info` — because the distinguishing case
  **cannot be built through the API**, which is exactly why every behavioural
  test missed it.
- **"Refuse nothing"** dies on the version test. **"Refuse only the future"**
  survived, because every version test used a *higher* version. It is the shape
  a well-meaning "backwards compatible" edit takes, and it would read a
  version-0-shaped file through version-1 column positions. Killed now by
  `a_store_from_an_older_layout_version_is_refused_too`, with
  `a_fresh_store_is_created_rather_than_refused` pinning the other side of the
  boundary — `0` is the absence of a version, not an unknown one, and must not
  be refused.

**The index gap, which this change first declared open and has now closed.** An
earlier draft said nothing mutation-tests the `CREATE INDEX` statements, because
an index is a performance object and a wrong column order changes no result — so
a silent scan is invisible to every gate. That was the right diagnosis and the
wrong conclusion: it *is* checkable, with one `EXPLAIN QUERY PLAN` assertion per
read, and `every_ordered_read_is_served_by_an_index_rather_than_a_sort` now makes
it. The assertion is on the **absence of a temp B-tree** rather than on the plan
text, because the wording is SQLite's to change between versions while the
property is not.

**What remains genuinely unchecked**, so the claim is bounded: the plan test
runs against an *empty* table, so it establishes that the index *can* serve the
order, not that SQLite's planner will still choose it once statistics exist. A
store large enough to change that decision is not something a unit test builds.

## What the clone costs, measured rather than assumed

The brief asked for this rather than an assertion, and §7.2's paginated reads are
what would feel it.

`Entry` is a `SignedOp` (a 32-byte Stoa address, a 32-byte public key, a 64-byte
signature, and an `OpKind` carrying `String`s and a `Vec<String>`) plus an
`Arrival` (an `Option<u64>` and an `Option<Vec<u8>>`). A clone is therefore a
handful of `memcpy`s plus one heap allocation per owned string — dominated by the
body, which `op.rs` caps at 150 KiB.

The three things that make it not matter here, in ascending order of weight:

1. **The read traffic is not on this type.** §3.3 puts the query traffic on the
   materialised view; the log is read to *rebuild* that view, not to render a
   frame. `op-log`'s own design says the same: "an authority is read to rebuild a
   cache, not to render a frame."
2. **The SQLite implementation allocates anyway.** Every row read from a database
   is decoded into owned Rust values — there is no borrow to preserve. The clone
   is a cost only on the `MemoryOpLog` side, and only relative to a `&Entry` that
   the other implementation cannot produce at all.
3. **`MemoryOpLog` already allocates a `Vec` per read**, collects into it, and
   sorts it. The change is from a vector of pointers to a vector of values; it is
   a constant factor on an operation that was already O(n) allocating, not a new
   order of growth.

**Where it would matter, and the pressure this design does not relieve:** a
paginated read that returns 20 entries out of 100,000 currently costs, in both
implementations, materialising all 100,000 and discarding 99,980. That is a
missing method — `iter_stoa_page(stoa, page, per_page)` — not a wrong return
type, and it is the `ORDER BY … LIMIT` §2.5 asks for. The SQL and the index are
in place for it; the method is deliberately not added here, because no caller
exists and §2.5 says widening the API is a decision to make on purpose. **It is
the first method to add when a caller appears**, and the schema was shaped so
that adding it is a query rather than a migration.

## What this change deliberately does not build

- **Any relevance score.** §7.2 rule 2 ships none in v1. Two columns are
  reserved; nothing writes them and no read consults them.
- **The materialised view.** §3.3's cache, where "newest threads, paginated
  replies, a Stoa's index" live. This is the log.
- **Paginated reads.** Argued above: the index supports them, no caller needs
  them, and §2.5 makes widening deliberate.
- **Eviction, retention or compaction.** §4.7 makes local retention "what makes
  v1 usable", so a log that discarded ops would remove the tier the plan leans
  on. Unchanged from `op-log`'s position.
- **Deciding where the database file lives.** `SqliteOpLog::open` is handed a
  path, exactly as `Keystore` is handed a file and for the same reason: a fixed
  path baked into a pure crate is untestable and would mean this crate reading
  the environment at a moment its caller does not control. The module crate has
  the host-stamped `instance_persistence_path`.
- **Concurrent access from two processes.** One peer, one store. SQLite would
  handle it; nothing here is tested for it, and claiming it untested would be a
  coverage claim this project has learned not to make.
- **Any storage for per-reader local state**, and this one is flagged rather
  than merely omitted because it is a question this schema does not answer and
  somebody will soon need it to.

  Everything this store holds is an **op**: authored by someone, signed,
  replicated, and replayable. §7.3's vouched set is none of those — it is
  authored by the user, never published, and must **survive replay rather than
  be derived by it**. That is a different relationship to storage than anything
  here: replay rebuilds the view from the log, and a table that replay must
  *not* touch has no precedent in this schema.

  It would be a second table with its own rules, not a column on `ops`, and the
  layout version is what would carry it. Naming it here so that whoever adds it
  knows they are adding a kind of state this store has never held, rather than
  fitting it into the one it has.

## What a real database made awkward

The finding a second implementor exists to produce. Three things, in ascending
order of how much they say about the contract rather than about SQLite.

### 1. `Vec<&Entry>` — the one that was predicted

Covered above. The `op-log` design named it as most likely to be wrong and it
was. Worth noting only that the prediction was right about the *return type* and
wrong about the *blast radius*: it reached both resolvers' public types, not
only their call sites.

### 2. `cmp_ops` short-circuits and SQL does not — and the contract does not say so

**This is the real finding.** `cmp_ops` is documented as total, transitive,
antisymmetric and a pure function of its arguments. All true, and none of it
warns that its `(None, None)` Lamport arm **never reaches `cmp_tiebreak`** — so
for two ops the transport did not order, the message id is not consulted at all.

In Rust that is invisible, because the short-circuit is the control flow. In SQL
there is no short-circuit: `ORDER BY a, b, c` compares `b` whatever `a` held. A
faithful-looking column-by-column translation of the comparator is therefore
**not** the comparator, and the difference only shows on a shape that is real —
an op with a message id and no Lamport value, which is what an SDS ephemeral
message produces.

The agreement test caught it on the first run. Nothing else would have: every
single-implementation ordering test passed, both before and after.

**What this says about the contract**: a comparator's *documented properties*
are not enough to reimplement it from. `cmp_ops` is correct and its
documentation is thorough, and a second implementor still needed the source. If
a third implementation is ever written — a different embedded store, a remote
one — this is the paragraph to read first.

The fix is structural rather than remembered: `SortKey::of` matches on the
Lamport value and the message id is only reachable inside the `Some` arm, so
there is no path that puts one into an unordered op's sort key.

### 3. One stored value cannot be both the record and the ordering

Falling directly out of (2). The first draft derived the `Arrival` back out of
the sort columns, reasoning that they carried everything an `Arrival` holds.
Once the sort key correctly **dropped** an unordered op's message id, that
became a bug in the other direction: the peer really did receive that id, and
`whether_an_arrival_was_ordered_survives_storage` is a spec requirement.

So the row carries both — `arrival_lamport`/`arrival_msg` for what was
**recorded**, and the sort columns for what is **compared**. That is two columns
of apparent redundancy which is not redundancy at all, and the schema says so at
the point a reader would otherwise try to remove it.

**They differ in two ways, not one.** An earlier draft here claimed one, and a
review corrected it — which matters, because an exactness claim is exactly what
invites a future editor to collapse the columns:

1. An op the transport did not order but for which it supplied a message id.
   The record keeps the id; the sort key must not carry it.
2. **Every** unordered op: `arrival_lamport` is `NULL` while `sort_lamport` is
   the filler `0` — and `0` is a real Lamport value's image, so the sort column
   cannot represent absence at all. Harmless only because `sort_ordered` leads.

**The general shape, which is not about SQLite**: a materialised ordering is a
*projection* of the recorded facts, not a re-encoding of them. Anywhere the two
are collapsed, one of them is wrong. `MemoryOpLog` never had to confront this
because it sorts the records themselves.

### 4. Everything else was unremarkable, which is itself the result

No other method needed redrawing. `append`'s `Appended` return, `iter_target`
being one method rather than one per kind, dedup by `OpId`, "nothing is filtered
on the way in", absence-is-not-an-error — all of it translated to SQL without
argument. The trait was drawn at close to the right altitude, and the two places
it was wrong are both about the *order*, which is the one part the trait
delegates to another module.

## What the resolvers changed, and why it was bigger than predicted

An architecture review predicted this would be a two-line change:

> Both resolvers consume it as `.into_iter().find_map(..)`, which is already
> iterator-shaped. When the signature becomes `impl Iterator` or a `Cow`, both
> call sites change by deleting `.into_iter()` — the folds themselves are
> untouched.

**The half about the folds was right and the half about the signatures was
wrong, and the difference is worth recording** because it is the finding a
second implementor exists to produce. The folds really are untouched. But both
resolvers returned types **borrowed from the log**:

- `revision::current_version<'a, L>(log: &'a L, ..) -> Option<CurrentVersion<'a>>`
- `moderation::resolve<'a, L>(log: &'a L, ..) -> Moderation<'a>`

Those `'a`s had nothing left to borrow from, so the change reached the resolvers'
public types and not only their call sites:

1. **`CurrentVersion<'a>` → `CurrentVersion`**, and `Moderation<'a>` →
   `Moderation`. Both lose `Copy` with their references and are `Clone` only.
   `Moderation::deciding_op` now borrows from the value rather than from the log,
   which is why a few call sites must bind the resolution to a `let` before
   reaching into it.
2. **Both functions became fallible**, so `Option<CurrentVersion>` became
   `Result<Option<CurrentVersion>, OpLogError>` and `Moderation` became
   `Result<Moderation, OpLogError>`. `Err` is a third outcome and not a kind of
   absence — the store could not be consulted, so the resolver has no opinion
   rather than the opinion that nothing binds.
3. **`resolve` selects the deciding entry by index** rather than by reference,
   so it can be moved out of the candidate vector. The two branches are the same
   two branches and the fail-closed `Hide` preference is identical in shape.
   The index is read back with `nth` rather than `[]` **because indexing
   panics**: the index is sound by construction today, but a future edit to the
   arithmetic that broke that would abort the module process, and the fallible
   form costs nothing.

**Nothing was weakened to make the types fit.** `resolve`'s three checks, its
filter and its order are byte-identical; `is_valid_revision`'s four conditions
and their order are untouched. Where the ownership change made something
awkward, the awkwardness was absorbed at the resolver rather than paid for by a
looser check.

This is why the change ships as three commits rather than one: the ownership
move (no behaviour change, every gate green on its own), then the fallibility
(one behaviour change, argued above, with no database in the diff), then SQLite.
