## Context

See `proposal.md` — Why. In short: a Stoa address is `stoa_address(&genesis.canonical_bytes())`, a one-way hash, and three calls (`read_feed`, `read_thread`, `join_stoa`) take a genesis record the caller must supply. No reply carried one, so a caller holding only an address held something it could never invert into the record those calls need.

Two constraints shape the approach, and between them they leave very little room:

- **The data was already on hand.** `stoa-membership` requires the peer retain the record for every Stoa it is in, and `membership.rs` stores `canonical_bytes()` verbatim precisely so it can be handed back. Nothing had to be stored, fetched or derived — only the wire shape omitted it.
- **The view cannot work around the absence.** Basecamp sandboxes the QML engine with a deny-all network access manager and no filesystem access outside the plugin directory, so a view cannot fetch what core did not send. This is the core/UI split being forced rather than chosen, and it is why the fix had to be a core change even though the symptom appeared in the view.

## Goals / Non-Goals

**Goals:**

- Every reply naming a Stoa also carries the record that address is the hash of, in the encoding the record-taking calls already accept.
- An encode failure is reported in the module's one failure shape, never as a success carrying an empty record.

**Non-Goals:**

- **No new call, no storage change, no migration.** This is a widening of three existing reply shapes: every existing field keeps its name and meaning, so a caller reading only the old fields is unaffected.
- **No verification moved into the view.** `join_stoa` remains the single place the address/record relation is checked. A view that re-derived an address would be a second implementation of the one check this design rests on.
- **Not the `policy` field on list items.** The spec fixes a list item as carrying address and founding title; widening the paginated item shape further is a decision for whoever needs it.

## Decisions

### The reported diagnosis was refused, and that refusal is the most reusable thing here

The brief that opened this work carried a diagnosis with it: a decode failure reading `genesis: genesis record ended mid-field` meant a codec bug, and an agent was dispatched to find it. **There was no codec bug, and the premise was refused rather than pursued.**

What the evidence showed: the owner's record decoded cleanly out of the live store — 48 bytes, version `01`, a 32-byte creator, policy `00`, title length `0000000A`, title `test0.0.01`. `stoa.rs::decode_of_encode_is_the_identity` already round-trips titles including the empty string, unicode and emoji, and passes.

The truncation came from a **different input entirely**. `wire.rs` decodes a `genesis` hex string supplied by the *caller* in the request; `list_stoas` never returned one, so the view passed `""`, which hex-decodes to zero bytes and fails the version-byte take. **Core was correctly refusing an empty record the view sent.** The decoder was behaving exactly as designed on an empty input; the bug was that the input was empty.

This is recorded because the general shape recurs and the error message actively misleads: *a decoder reporting truncation is a claim about the bytes it was given, not about where they were stored.* The question to ask first is whether the input was ever supplied — not whether the codec is broken. Reading it as a storage or codec defect sends the investigation to the furthest possible point from the cause, which is a wire shape that omitted a field.

It is written here rather than left in the PR body deliberately: `openspec archive` moves `design.md` into `changes/archive/`, and a PR description is not part of the repository history that survives. Without this entry, the next truncation error in this codebase re-derives the whole investigation from scratch. This is README.md's "write the dead end down" — except that the dead end was the brief's own premise, which makes it worth more, not less.

### `membership_page_json`: a `for` loop rather than a `map` closure

**Chosen:** rewrite the item-building `map` closure as a loop whose encode-failure arm is `return error_json(...)`.

**Why:** a closure cannot return from the enclosing function. Inside a `map`, a `canonical_bytes` error had exactly two possible fates — swallow it (emitting an item with an empty or absent record) or panic. Both are refused: an empty `genesis` is *precisely* the input that produces "ended mid-field", so a swallowed error would have reintroduced this very defect wearing a success reply, moving the error to a later call that cannot explain it. A panic is refused outright — the SDK has no panic guard, and an unguarded panic aborts the module process.

The loop's `return` exits the whole function, discarding any already-pushed items, so a mid-page failure surfaces as the one failure shape rather than a partial listing. That is the module contract: `{"error":"..."}`, never a partial success.

**Alternative considered:** keep the `map` and collect into `Result<Vec<_>, _>`, which does preserve the error. Rejected as the more obscure of two correct options — the loop states the control flow it is doing, and this function is on the path of the defect being fixed. This is a readability call, not a correctness one.

**What breaks without it:** `a_record_that_cannot_be_encoded_is_a_failure_rather_than_an_empty_field` pins the guarantee at `stoa_reply`. Replacing the error arm with `unwrap_or_default()` turns it red, on exactly the forbidden shape — measured, the failure output reads `{"foundingTitle":"xxx…","genesis":"","policy":"open","stoa":"0000…"}`, a success reply carrying the empty record. That is the owner's original bug reproduced as a passing call.

**On reachability, stated precisely so it is not mistaken for a live guard:** `canonical_bytes()`'s only failure mode is `TitleTooLong`, and every record reaching either encode site has been encoded once already (hashed to compute its address, or decoded out of storage, where `decode_row` re-derives the relation). So the arm is **defensive rather than reachable through `create_stoa` or `list_stoas` today.** The test reaches it by calling `stoa_reply` directly with a record built over the cap, because forcing it through a handler would need a store holding bytes the encoder refuses — a state `decode_row` rejects on the way in. Anyone deleting this arm as dead code should read this paragraph first: it is cheap, and it is the difference between a bug and a silently wrong success.

### The QML record map is reassigned, never mutated in place

**Chosen:** `var next = screen.genesisByStoa; next[stoa] = genesis; screen.genesisByStoa = next`, followed by an explicit `genesisByStoaChanged()`.

**Why:** a QML `var` property holding a JS object **does not emit a property-changed signal on an in-place key write** — only on reassignment of the property itself. `canShare(row.rowStoa)` is a binding that reads `genesisByStoa`, so without the reassignment the map would hold the record while the share button stayed hidden and `Open` stayed inert. The data would be right and the screen would be wrong, which is this codebase's house style of silent failure: basecamp swallows QML errors, so nothing would have reported it.

**Alternative considered:** the obvious simplification, `next[stoa] = genesis; screen.genesisByStoaChanged()` without the reassignment. It looks equivalent and is not. It is recorded here because it is exactly what a later editor tidying this function would reach for.

**What breaks without it — stated honestly, because the answer is "not enough":** the existing QML specs assert the map's final contents and `canShare`'s value, both of which are computed in JS rather than through a live binding, so **they would very likely stay green** through a "simplification" that broke on-screen re-evaluation. This is a known, accepted gap rather than a covered case: catching it needs a spec that instantiates the row and asserts the button's `visible` after a reload, which is the rendering layer no component test here currently reaches. Named so it is not mistaken for tested.

### The record is pinned as a relation, not as a literal

**Chosen:** every new Rust test decodes the reply's record and asserts `genesis.address() == reply["stoa"]`, through the shared `genesis_of` helper.

**Why:** a reply could carry a well-formed record *of some other Stoa* and satisfy "the field decodes" while being useless to the caller — what makes the field usable is exactly that the pair verifies. A pinned hex literal would also pass while proving less, and would fail on a reword rather than on misinformation.

This is the repo's standing test-defect family: a fixture where two explanations give the same answer. The relation has one explanation.

### The guard against an empty record is asserted against the map's keys

**Chosen:** `test_an_item_short_of_its_record_is_not_recorded_as_an_empty_one` asserts `!screen.genesisByStoa.hasOwnProperty(addr)` for both the missing-field and empty-field rows, alongside the existing `genesisFor`/`canShare` assertions.

**Why:** the test as first written was **vacuous in the direction it existed to guard**, found independently by three reviewers. `genesisFor` is `typeof g === "string" ? g : ""`, which collapses "key never written" and "key written as `""`" to the same observable `""`; `canShare` is derived from `genesisFor` and inherits the blindness. Every assertion phrased through either accessor therefore passes whichever branch `rememberGenesis` takes — **measured at 81/81 green with the guard's `|| genesis === ""` half removed, by three separate mutation approaches.**

`hasOwnProperty` is the only accessor that distinguishes the two states, so it is the only one that can witness the guard. **What breaks without it:** removing that half of the guard now fails this test by name — measured, 80/81 with this test red on *"an empty field must leave no key either"*. Before the change the same mutation was caught only incidentally, by three pre-existing tests that seed `genesisByStoa` and happen to have it overwritten; a fixture change in any of them would have silently ended even that coverage.

The general rule this instance is worth remembering for: **ask what the null implementation would produce.** An assertion that holds whether or not the code under test ran is decoration, and a lossy accessor is the usual way one gets written by accident.

## Risks / Trade-offs

- **The record is now on every listing item, so the reply grows by the encoded record per Stoa** → Bounded by construction: `MAX_CANONICAL_BYTES` caps one record at 1062 bytes (2124 hex chars), and the call is paginated, so the growth is per page rather than per peer. No network traffic is added — this is a local module reply.
- **A caller that ignores the new field is unaffected, but a caller that *depends* on the old shape's field set would see an addition** → Accepted, and it is why this is framed as a widening: every existing field keeps its name and meaning. The core API is the deliverable and widening it is the decision being made on purpose here, per CLAUDE.md.
- **The share affordance becomes reachable for the first time** → It was gated on the same lookup as `Open`, so this un-hides it rather than changing what it shares. `DStoaReference.shareText` carries both halves in full hex, untouched by this change, and the receiving end still defers verification to `join_stoa` alone.
- **The click-through is unverified** → Recorded in `tasks.md` 5.4 and left unticked rather than ticked on the strength of the suite. Loading the plugin needs a human to click the dialectica tile, which an agent cannot do. What *is* established: the reply carries the record, the view fills its map from it, and both forwarding layers pass the reply through unnarrowed. The remaining gap is the rendering itself.

## Open Questions

None that affect the spec, the approach or the task breakdown.
