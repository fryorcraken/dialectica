# Bring the publish path inside the request envelope, and sign with the key the probe names

## Why

Two defects were live on `main`, both already recorded and neither owned.

**The three publish handlers bypassed the request envelope.** `publish_post`,
`publish_reply` and `publish_vote` parsed with a bare `serde_json::from_str`
rather than through `Request::parse`, so none of `module-wire-contract`'s
obligations reached them:

- a request that was not an object was **served as a missing field** —
  `serde_json::Value::get` answers `None` for an array exactly as it does for
  an object without the key, so `[]` came back as
  `{"error":"missing field: stoa"}` and the spec's "three caller mistakes, three
  messages" collapsed into one;
- there was **no size cap**, so a request over `MAX_REQUEST_BYTES` was parsed at
  roughly 2N transient heap. `docs/PHASE0-FINDINGS.md` §3 measured what an
  allocation failure in a dispatch handler costs: the module process aborts, the
  caller waits out a 20-second timeout, and every later call reports
  `MODULE_NOT_LOADED`;
- and all three were **absent from `every_request_taking_method`**, the sweep
  whose whole job is applying those rules across the surface. Five sweeps ran
  green over eleven methods while the surface had fourteen.

This is a security boundary. CLAUDE.md: *"Never trust an inbound message ...
Validate at the boundary, before it reaches any state machine."*

**The publish path signed with a pathless per-Stoa key.** The adapter called
`keystore.stoa_key(&stoa)` while `getCapabilities` and `whoAmI` both report
`stoa_address_at_path`. `identity.rs`'s own test asserts those two schemes
**must** disagree, so every op a user published was authored by an identity
neither method on the surface would ever name — the failure
`posting-capability` calls out as *"the user sees one handle and posts under
another"*, one layer deeper, because this is what reaches the network rather
than what a screen displays.

CI fenced it with a named exemption reading *"WHICH key a publish signs with is
a spec question this gate cannot answer ... Delete this exemption when the spec
decides."* The spec has decided, in `content-authoring`: *"WHEN the
posting-capability probe reports an identity for a Stoa and a post is then
published into that Stoa, THEN the published op's author is the identity the
probe reported."*

## What changes

- **`PublishRequest`**, a type carrying the prologue all three handlers shared
  — envelope, forbidden-field guard, `stoa` parse — so the guards are what
  constructing the value *is* rather than four statements each handler repeats.
  PLAN.md §9.2 named this reshape and deferred it deliberately; it lands as its
  own commit, changing no behaviour, with the envelope fix on top.
- **`Request::parse` on the publish path**, which is the whole of the first two
  envelope fixes and is one line because of that reshape.
- **`publishing_key` in `core`**, the probe's own derivation, replacing the
  adapter's pathless call. A Stoa with no recorded choice is refused with the
  same constant `getCapabilities` and `whoAmI` give, rather than signed under
  some other key.
- **The CI exemption is deleted**, and `core::wire::publishing_key` joins the
  two calls that gate already requires the adapter to route through.
- **The sweep list can no longer go stale.**
  `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`
  reads the dispatch trait's declaration out of the adapter with `include_str!`
  and fails, naming the method, when one is on the surface and not in the sweep.

## No spec delta, and the reasoning rather than the conclusion

**Whether this needed one was a real question**, because a contract that does
not reach these handlers would have a gap worth closing as a `MODIFIED`
requirement. It was checked against the text rather than assumed, and the answer
is no for both halves.

**`module-wire-contract` already reaches them.** It scopes the envelope rule to
*"every method that **reads a field** of its request"*, states that scope's
three cases explicitly, and says the surface carries *"exactly one method in
that third case: the panic probe"*. The publish handlers read `stoa`, `body`,
`parent`, `target` and `direction`. They are inside the rule's first case, and
they were violating it — an implementation defect against a contract that was
already correct, not a contract that failed to say so.

The requirement is also emphatic on precisely this point: *"This SHALL hold for
**every** such method, whatever fields that method requires ... The rule is
therefore stated once, for the envelope, rather than left to each method's
fields to imply."* Adding a requirement naming the publish handlers would make
the contract weaker, not stronger: a rule restated per method is a rule the next
method is outside.

**`content-authoring` already decides the signing key**, in the scenario quoted
above. Nothing about it needed widening; the code needed to catch up with it.

So the delta directory is absent on purpose rather than forgotten, and the
absence is the finding: two live defects, zero contract changes, because both
contracts were already right.

## Impact

- `dialectica-core`: `wire.rs` — the three handlers, the prologue type,
  `publishing_key`, and the sweep.
- `dialectica`: `rust-lib/src/lib.rs` — the adapter's publish assembly.
- `.github/workflows/ci.yml` — the exemption removed, the required call added.
- No change to any wire reply a correct caller receives. What changes is which
  requests are refused, what those refusals say, and which key signs.
