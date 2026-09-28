# The SQLite op log: a second implementor, and a schema that can accept a score

## Why

PLAN.md §9 Phase 1 names five things. Four have landed. The remainder is "the op
log and **its SQLite projection, query indexing**", and the `op-log` change
shipped the trait with an in-memory implementation only — recording the SQLite
half as a **deferred Phase 1 item**, not a phase boundary.

That change's `design.md` wrote its own reversal condition:

> `MemoryOpLog` is the only implementor of `OpLog`; when `grep 'impl OpLog'`
> still returns one line at the end of Phase 1, the trait did not earn itself.

This change makes that grep return two lines. Until it does, two claims in
PLAN.md are aspirational rather than true — §3.3's "each peer is to keep a local
SQLite store" and §4.7's middle durability tier, which "buys nothing across a
restart yet". A peer restarting today loses every op it has ever seen.

**And there is a decision here that gets more expensive the longer it waits.**
§7.2 rule 5 names this change by name:

> **Decide this when the projection schema is designed, in Phase 1** —
> retrofitting an index onto a time-varying score is the expensive version.

This change implements no relevance. What it does is decide whether the schema
it writes can accept one later **as a column and an index rather than as a
migration on every peer's store**. That decision is the change's centre and is
argued in `design.md`.

The answer turned out to be shaped by a module that landed while this was being
written. `moderation.rs` resolves moderator authority **on read, every time,
from the genesis record** — so a vote's weight is not a property of the vote,
and a stored score would be a stored *weighting* that goes wrong the moment a
moderator set changes. With per-reader vouching (§7.3) it is worse than stale:
two readers of one store compute different weights from the same rows, so there
is no single correct value to store at all.

**So the log stores inputs and never conclusions**: who signed each op, what it
acts on, and where the transport placed it. An aggregate is the view's to build,
and `design.md` records the shape it will need.

## What Changes

- **`SqliteOpLog`**: a second implementor of `OpLog`, backed by `rusqlite`.
  Durable across a restart, which is the property the trait existed to allow and
  nothing has yet delivered.
- **The trait's reads return owned values.** `Vec<&Entry>` and `Option<&Entry>`
  cannot be produced by a store that has not already materialised its rows, and
  the `op-log` design named this as the return type "most likely to be wrong".
  Reads become `Vec<Entry>` / `Option<Entry>`, and every method becomes
  fallible — a database call can fail, and PHASE0-FINDINGS §3 makes an `unwrap`
  a process abort rather than a bad line.
- **A schema that reserves for ranking without storing one**, and a sort key
  materialised on write so that the defined read order is indexable. The §7.2
  rule 5 answer: the decay **epoch** is stored — as a Lamport timestamp, never a
  wall-clock reading — so that decay can be applied in an `ORDER BY` over stored
  values rather than recomputed from the clock on every read.
- **One behavioural contract, run against both implementations.** A trait with
  two implementors is worth having only if the second is a drop-in, and the way
  to know is to run the same assertions against both.
- **A schema version, checked on open.** A store written by a future version is
  refused by name rather than misread.

Not changed: the op format, `arrival.rs`, `cmp_ops`, the wire contract, any
resolver's logic, or any transport wiring. No relevance scoring is implemented.

## Capabilities

**Modified Capabilities**

- `op-log` — the existing requirements are unchanged in their behaviour and are
  now required of **every** implementation rather than of the one that existed.
  Three requirements gain scenarios for durability, for failure being reported
  rather than panicked, and for the two implementations agreeing.

**New Capabilities**

None. This is the same contract with a second implementation behind it; a new
capability would claim a second contract exists.

## Impact

- New `dialectica/rust-lib/dialectica-core/src/log/sqlite.rs`; `log.rs` becomes
  `log/mod.rs` with the trait, `Entry`, and `MemoryOpLog`.
- **One new dependency, `rusqlite` with `bundled`.** `design.md` argues it
  against the keystore change's posture — unavoidable, or strictly smaller than
  what it replaces. `bundled` is load-bearing and not a convenience; the reason
  is in `design.md` and in `Cargo.toml`.
- `OpLog`'s signature changes, and **so do both resolvers' public types**. An
  architecture review predicted a two-line change at their call sites; it was
  right about the folds, which are untouched, and wrong about the signatures —
  both returned types borrowed from the log, and those borrows have nothing left
  to point at. `CurrentVersion<'a>` and `Moderation<'a>` become owned, and both
  functions become fallible. `design.md` records exactly what changed and
  confirms that nothing was weakened to make the types fit.
- PLAN.md §3.3 and §4.7 both carry a correction saying SQLite does not exist.
  This change makes those corrections false, so it updates them.

## Sequencing

The `op-log` change is not yet archived: its spec delta still lives in
`openspec/changes/op-log/`. **This change's `MODIFIED` headings name requirements
that exist only there**, so `op-log` must archive first or `archive` will apply
these modifications against nothing and report success. That ordering is a
precondition, not a preference.
