## Context

See `proposal.md` — Why. The constraints this design has to fit between are all
already in the tree, and three of them decide most of it:

- **`SqliteOpLog` refuses any store whose `PRAGMA user_version` is not exactly
  `LAYOUT_VERSION`**, in both directions, and `check_layout` additionally names
  every column of `ops` that a read touches. That refusal is deliberate and
  documented at length; `sqlite.rs`'s own comments state there is no migration
  path by design.
- **`create_schema` runs only when `user_version` reads `0`** — the never-stamped
  value. There is no `CREATE TABLE IF NOT EXISTS` anywhere in the crate, so a
  table added to that batch reaches fresh stores only and never an existing one.
- **`Moderators::of` takes a `&Genesis`, and it is the only constructor.** The
  `moderation-resolution` capability's "A Stoa's moderator set is derived from
  its genesis record" is what makes that the fail-closed shape. It is also why
  membership must retain the record: an address is a one-way hash, so a peer
  holding only addresses holds a list of Stoas it can store content for and
  cannot judge.

The core crate holds no opinion about where storage lives. `SqliteOpLog::open`
takes a path, `keystore::default_path_in` takes a directory, and the adapter in
`dialectica/rust-lib/src/lib.rs` is the one thing holding the host's
`instance_persistence_path`. This design follows that convention rather than
introducing a second one.

## Goals / Non-Goals

**Goals:**

- Membership as state beside the op log, readable and writable without the op
  log being consulted at all.
- Retain the complete genesis record per Stoa, in a form that still verifies
  against the address it is filed under after a restart.
- Three wire handlers — create, join, list — in `wire.rs`, each inside the panic
  guard and each answering the one failure shape.
- A signing key taken as an input, never minted.

**Non-Goals:**

- **No metadata resolution.** `stoa-metadata` owns current-versus-founding, and
  nothing resolves those ops. Every title this change reports is a founding
  title and says so on the wire.
- **No moderation call, no feed change, no compose.** Creation leaves the peer
  holding what a moderator set would later be derived from; it does not derive
  one.
- **No change to the op log**: not its schema, not its version, not its refusals.
- **No `getStoa`.** PLAN.md §9.1 names it, and it is the metadata-resolution
  call. Out of scope with metadata.

## Decisions

### Membership lives in its own SQLite file, not in a second table beside `ops`

**Chosen:** a separate store, `stoas.sqlite`, with its own
`PRAGMA user_version` and its own open path. `MembershipStore::open` never
touches the op log's file.

The spec requires that a store already holding ops stay readable, that opening
it not be refused for predating membership, and that a membership be
**recordable into such a store**. Put the table beside `ops` and each of the
three ways to get there is worse:

| Alternative | Why it loses |
|---|---|
| Add `memberships` to `create_schema` | That batch runs only at `user_version == 0`. An existing v1 store never gets the table; `check_layout` passes it (it names only `ops` columns), and the first membership read fails as `Storage("no such table: memberships")`. "Opening succeeds and ops are readable" holds; "a membership is recordable" does not. |
| Bump `LAYOUT_VERSION` to 2 | The spec forbids it in as many words, and `sqlite.rs` refuses an older layout on purpose — a v1 store would become permanently unopenable, discarding the user's ops to avoid a migration that is not needed. |
| `CREATE TABLE IF NOT EXISTS` on every open | Silently repairs a file that `check_layout` exists to refuse, and makes "what layout does version 1 mean" have two answers depending on when the file was opened. `sqlite.rs`'s documentation names a later "add a migration" refactor as the change its warning is addressed to; this would be that change, done by accident. |

The separate file makes the requirement hold **by construction rather than by a
check**, which is CLAUDE.md's standing instruction applied to a file boundary: a
store predating membership has no membership file, `MembershipStore::open`
creates one, and there is no version of the op log's layout that has to mean two
things. Nothing in `SqliteOpLog` is edited by this change — which is also what
makes the spec's requirement testable as written, by opening an op store that
this change cannot have touched and reading its ops back afterwards.

The cost is two files where one would do, and one more `user_version` to keep
straight. Both are cheap; the op log's own comments make the case that a version
number that has to mean two layouts is not.

### The record is stored as its canonical bytes, and the address is derived on read

**Chosen:** one row per Stoa: `stoa` (the 32-byte address) as `PRIMARY KEY`, and
`genesis_bytes` — `Genesis::canonical_bytes()` verbatim.

Storing the record as decomposed columns (creator, policy, title) and
re-encoding on read would make the address depend on this module's re-encoding
agreeing with `stoa.rs`'s forever. The address **is** the hash of the canonical
encoding, so re-encoding is the one operation that must not be duplicated. Same
argument `sqlite.rs` makes for storing `op_bytes` verbatim, and the same test
shape holds it: the retained record is read back, its address recomputed, and
compared against the key it was filed under.

`stoa` is the primary key rather than a `UNIQUE` index over the bytes, because
the address is the identity. `INSERT OR IGNORE` then gives repeated-join
idempotence structurally: a second join of the same Stoa cannot produce a second
row and cannot overwrite the first, so "MUST NOT disturb what was already
retained" is a property of the statement rather than a branch that must be right
at each call site. `OR REPLACE` would satisfy the same-record case and quietly
break the spirit of the requirement — a join would become a way to overwrite
what a peer already holds.

**The write verifies before it inserts, and `MembershipStore::join` takes the
address and the record separately** so that the verification is inside the store
rather than in a caller that could forget it. A caller holding a `Genesis` can
always derive its address, so a store taking only the record could not express a
mismatch at all — and the whole point of the join shape is that a mismatch is
refusable.

### The address is a parameter of `join` even though the record determines it

This reads as redundant and is not. The spec's requirement is that a record
differing from the one the address names in **any** field is refused, *and that
the peer is not in the Stoa the supplied record would name either*. A `join`
taking only a record could not fail: it would join whatever it was handed. The
address is the caller's claim about what it thinks it is joining, the record is
the material, and the refusal is the two disagreeing. That is exactly
`Genesis::matches`, which `wire.rs::genesis_for` already uses for the feed path.

### Creation reuses `join`'s write path rather than having one of its own

`create_stoa` builds the record, computes the address, and records the
membership through the same `join` call a paste goes through. Two consequences
the spec asks for fall out:

- "Creating the same title twice yields one Stoa" is then the same
  `INSERT OR IGNORE` idempotence as a repeated join, not a second mechanism that
  has to agree with it.
- "Creating and then joining the same Stoa is one membership" is true because
  there is one write path, not because two were checked against each other.

The title bound is refused **before** the write, and that ordering is not
incidental: `Genesis::canonical_bytes()` is fallible for an over-cap title, and
building the record's bytes is what the store needs, so a failed encode returns
before any statement runs. The spec's "the refusal happens first, so a failed
creation leaves nothing behind" is therefore structural.

### The creator key is taken as a `PublicKey`, and the handler takes a closure

`create_stoa_from_request` takes `impl FnOnce() -> Result<PublicKey, KeystoreError>`
— the same shape `get_capabilities` takes for its identity lookup, and for the
same reason: this crate cannot read the environment or know the host's layout.
The adapter has the keystore path and supplies the closure.

**A `PublicKey`, not a `SecretKey` and not a `Keystore`.** A genesis record needs
the creator's public key and nothing else — no signature is made, because a
genesis record is not an op and carries none. Taking a secret would be taking
authority the operation does not use.

**The failure mode is a `KeystoreError`, not a bool.** "Creation MUST fail when
the peer has no usable signing key, and MUST NOT proceed by generating a key" is
satisfied by there being no path from this handler to `Keystore::generate()`.
Whether a key is usable and why not is `posting-capability`'s probe: this
handler surfaces the error's own message and adds no reason vocabulary of its
own, exactly as `capability_for` does.

**Which key.** §5.2 gives a user one identity per Stoa, derived from the root and
the Stoa address — and a Stoa's address is not known until the record naming its
creator exists, so the per-Stoa derivation is circular for the creator field.
This is unspecified behaviour and marked as such in the code; see NO SPEC below.

### The reply names its title as founding

`{"stoa":"<hex>","foundingTitle":"…","policy":"open"}` for create and join;
`{"items":[{"stoa":…,"foundingTitle":…}],"page":N,"hasMore":bool}` for list.

The field is **named** `foundingTitle` rather than carried as `title` beside a
boolean flag. The spec requires the reply make it distinguishable from a
resolved current title; a `{"title":…,"isFounding":true}` shape puts a view one
forgotten branch away from rendering a founding title as current, and `title`
would then have to mean two things depending on a sibling field. A name that
cannot be misread costs nothing. When metadata resolution lands it adds `title`
and `isGenesisFallback` beside this field rather than redefining it — which is
PLAN.md §9.1's own shape for `getStoa`, and this change does not pre-empt it.

`policy` is included on the create and join replies because the spec requires
the posting policy be **answerable** from what was retained, and the join reply
is where a view shows what is being joined. It is deliberately **not** on the
list items: the spec fixes list items as carrying the address and the founding
title, and widening the paginated envelope's item shape is a decision for
whoever needs it.

### Listing pages over an ordered read, and the order is the address

`ORDER BY stoa ASC` — the primary key, so it is an index walk. The spec requires
every Stoa be reachable by paging and none appear twice, which is a property of
the order being **total and stable**; the address is 32 bytes of hash and
unique by primary key, so it is both. Insertion order was the alternative and is
worse for the reason `log/mod.rs` gives about its own reads: it is per-peer, and
a reader that could reach it would be one refactor away from presenting it as
meaningful. Nothing in the spec asks for a join order, and a `joined_at`
timestamp would be a local wall-clock reading — the thing `arrival.rs` refuses.

`page`/`perPage` reuse `wire.rs::parse_index` and `feed::clamp_per_page`
unchanged, so the envelope's arguments behave identically to the feed's rather
than acquiring a second interpretation of a negative page.

### An op for an unjoined Stoa creates no membership, and that needs no code

The requirement is satisfied by the absence of a call. There is no path from
`OpLog::append` to `MembershipStore`, the two types share no state, and nothing
in the receive path this change does not own could reach membership without a
new call site. **So the tests for this requirement are written against the
absence**: append ops through the real log, then read the real membership store,
and assert the listing is untouched. That is the honest shape — a test asserting
a function was not called would be asserting on an implementation, where this
asserts on the observable state the spec constrains.

## Risks / Trade-offs

- **Two stores, two `user_version`s** → `MEMBERSHIP_LAYOUT_VERSION` is pinned by
  a hardcoded assertion, following `identity.rs`'s wire constants and
  `sqlite.rs`'s own layout version, because `cargo mutants` cannot see a wrong
  `const`. The two versions are independent by design and neither refusal
  mentions the other.

- **A membership store refuses an unknown layout, and the op log is then still
  readable** → this is the asymmetry the separate file buys and it is worth
  naming: a peer holding a membership store from a future build cannot list its
  Stoas and can still read every op it holds. That is the recoverable direction
  — the alternative, one file, makes an unknown membership layout cost the user
  their ops.

- **A row whose `genesis_bytes` no longer decode** → reported as
  `MembershipError::CorruptEntry`, never skipped. Skipping would make a
  corrupted row indistinguishable from a Stoa the user never joined, which is
  `OpLogError`'s "an empty feed is not an unreadable store" argument applied to a
  listing.

- **`stoas.sqlite` is a new file in the host's persistence directory** → no
  migration and nothing to clean up: a peer that never creates or joins a Stoa
  never gets the file, and a peer that does gets it created on first write.

- **The creator's key is the root identity rather than a per-Stoa one** → see
  the NO SPEC marker. The consequence is that a Stoa's creator key is linkable
  across the Stoas one peer created, which §5.2's per-Stoa identity exists to
  prevent for *posting*. It is recorded rather than resolved because the
  derivation is circular and the spec is silent; whoever specifies it should know
  the privacy cost is real.

## Unspecified behaviour, marked in the code

Each of these is a `// NO SPEC:` marker on a test, and each is a question for the
spec-writer rather than a decision this change should own:

1. **Which key is the creator key.** The spec says "the key the caller would sign
   an op with" and §5.2 makes that per-Stoa — but the Stoa address is not known
   until the record exists, so the per-Stoa derivation is circular for this one
   field. The root identity's public key is used. A view showing "you created
   this" is unaffected; cross-Stoa linkability of creator keys is not.
2. **What `policy` a created Stoa declares.** `Policy::Open` is the only variant
   and the spec does not say creation may choose. Creation accepts no policy
   parameter and always declares `Open`.
3. **Whether an unknown field in a request is refused.** Ignored, matching
   `list_threads`'s treatment of an offered `order`.
4. **What an empty listing's `hasMore` is.** `false`.
