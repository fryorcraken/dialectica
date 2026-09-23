## Why

A Stoa's `StoaMetadata` op is defined, encoded and stored, and nothing reads
it: every title the module reports is the **founding** one from the genesis
record, however long ago that was chosen, and no call can say whether a Stoa has
been renamed since. Issue #98 (milestone 0.0.1) asks for the read side — a
resolver that picks the current metadata from the ops a peer holds, and a
`getStoa` call that reports it together with whether it fell back to the
genesis values.

The `stoa-metadata` capability already fixes the rule in outline, but its text
predates the op clock: it says "most recent" is decided by the transport's order
and that a metadata op may not carry its own position, both of which
`op-ordering` has since reversed. The rule has to be restated against the order
the system actually uses before anything implements it.

## What Changes

- **A metadata resolver.** Given a Stoa's genesis record, it considers every
  `StoaMetadata` op the peer holds for that Stoa, keeps only those that bind —
  authentic, signed by a moderator from the moderator set that record derives,
  and naming that Stoa — and takes the one `op-ordering`'s rule places first.
  Where none binds, it falls back to the genesis values. The binding check is
  the one `moderation-resolution` already makes; it is not respecified.
- **A new wire method, `getStoa`.** It takes a Stoa address and the genesis
  record that address names, as `listThreads` and `readThread` already do, and
  answers `{stoa, title, description, policy, isGenesisFallback}`, the shape the
  issue sketches. It works for a Stoa the peer has not joined, so a preview can
  use it before a join.
- **The reply carries no founding title of its own.** Where it falls back,
  `title` is the founding title; where it does not, `title` is the current one
  and the founding title is not reported beside it. `stoa-metadata`'s
  requirement that both remain readable does not need `getStoa` to carry the
  founding value: the caller supplies the genesis record that holds it, and
  `listStoas`, `createStoa` and `joinStoa` already report it.
- **The request carries the genesis record, which departs from the issue's
  `getStoa({stoa})` sketch.** The moderator set, and therefore which metadata
  ops bind, is derived from the record, and an address cannot be turned back
  into one. Taking the record in the request is the pattern the other Stoa
  reads already use, and it is what lets an un-joined Stoa be answered.
- **`stoa-metadata`'s resolution requirement is rewritten.** The withdrawn
  transport-order wording and the "not implemented" disclaimer are replaced by
  the rule as built. Its "never an identifier" requirement loses one stale
  clause, which said that any peer's metadata op takes effect.
- **A stale sentence in `stoa-membership` is corrected** without changing its
  behaviour: it said that nothing resolves metadata ops.
- **`stoa-navigation-view` is brought in line with what `getStoa` answers.**
  That capability made the join preview's founding title conditional on a core
  call answering one before a join, and expected `getStoa` to lift the
  condition. With this reply shape it lifts it only for a Stoa `getStoa`
  reports as falling back, whose `title` is the founding title. For a Stoa it
  reports as renamed, `title` is a current title, and the founding title stays
  unavailable until a join. The three requirements that relied on the old
  expectation are restated to say which case is which, and which reply field
  decides it. The same-title comparison, which runs over founding titles, runs
  at preview time for the first case only.
- **Not in this change:** publishing a `StoaMetadata` op (issue #125, milestone
  0.0.2), and any view that calls `getStoa` (issue #143, milestone 0.0.1).
  Until #125 lands this build publishes no metadata op, so `getStoa` falls back
  to the genesis values unless a metadata op arrives from elsewhere. The resolver does not depend on who
  published the op: it reads what the log holds, and the tests stage those ops
  directly. It can therefore be built and tested without #125.

## Capabilities

### New Capabilities

None. Resolving current metadata is already the declared purpose of
`stoa-metadata` ("how a reader resolves the current ones and falls back when it
holds none"), so the resolver and the call that exposes it belong there, the
same way `stoa-membership` and `thread-read` each hold both a behaviour and its
wire method.

### Modified Capabilities

- `stoa-metadata`: MODIFIED "Current metadata resolves by last-write-wins,
  falling back to genesis" (restated against `op-ordering`'s counter-carried
  order and the moderator-set authority check, and no longer disclaimed as
  unimplemented); MODIFIED "A displayed title is never an identifier" (stale
  claim that no authority check exists); ADDED requirements for the `getStoa`
  call, the shape of its reply, its failure cases, and its never aborting.
- `stoa-membership`: MODIFIED "A listed title is a founding title, and is
  identified as such" — one sentence citing `stoa-metadata` as saying nothing
  resolves metadata ops; behaviour unchanged.
- `stoa-navigation-view`: MODIFIED "Joining shows what is being joined, and
  joins nothing until the user acts" (a founding title is available before a
  join exactly when `getStoa` reports `isGenesisFallback: true`, as its
  `title`; a `title` from a non-fallback reply is never labelled founding; the
  no-title note says no *founding* title is available, since a current one may
  be); MODIFIED "No current title is rendered until one has been resolved"
  (only a non-fallback reply supplies a current title; the stale "resolution is
  not implemented" reasoning is replaced); MODIFIED "A Stoa already held whose
  title matches is shown as a distinct Stoa, not as a duplicate" (`getStoa`
  lets the founding-title comparison run at preview time for a fallback Stoa
  only, where the old text said it closed the gap outright).

## Impact

- `dialectica/rust-lib/dialectica-core`: a metadata resolver beside
  `moderation.rs`, reading `OpLog::iter_stoa` because a metadata op names no
  target op; a `get_stoa` handler in `wire.rs`; dispatch through the module
  adapter in `dialectica/rust-lib/src/lib.rs`.
- **The core API gains one method.** It widens the wire contract on purpose,
  following the `module-wire-contract` envelope and error shape. It does not
  change any existing method's request or reply. `listStoas`, `joinStoa` and
  `createStoa` keep reporting `foundingTitle` alone.
- **The join preview falls out of step with its spec until #143 lands.** Once
  `getStoa` exists, a founding title is available before a join for every Stoa
  it reports as falling back, so the preview is obliged to render it — as
  `title`, labelled founding — and to run the same-title comparison against it.
  The current view calls nothing before a join and says no title is known, so
  it does neither. For a Stoa `getStoa` reports as renamed, the view's present
  rendering (no founding title, the comparison stated as not made) still meets
  the spec; rendering the current title is permitted there but not required.
  Wiring the preview is issue #143 (milestone 0.0.1), not this change.
