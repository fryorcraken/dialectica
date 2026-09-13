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
  `MODULE_NOT_LOADED`. **This third one was not a contract obligation at all
  until this change made it one** — see below;
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
- **`core::stoa_of`, and the adapter's own pre-parse deleted.** The adapter must
  read `stoa` before it can derive a per-Stoa key, and it was doing so with its
  own bare `serde_json::from_str` — which **shadowed every envelope fix above on
  the shipped module**, however correct `dialectica-core` was in isolation.
  Review caught this; no gate did. It now reads through `core`, so the envelope
  is crossed once, and CI both requires that call and bans `serde_json::from_str`
  in the adapter outright.
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
  It **classifies** every method rather than filtering for one shape, so a
  declaration written unusually is a loud failure naming it rather than a silent
  omission — the correction for two evasions review measured against the first
  version. Its preconditions are stated in `design.md` §6 rather than implied.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `module-wire-contract`: the request **size bound** is added as a requirement —
  that one exists, that it is one number for the surface rather than per method,
  and that it is checked **before** the request is parsed. And the existing
  "Every method takes JSON and returns JSON" requirement gains the two sentences
  that say what *the surface* is: it is derivable from one declaration, and a
  method the generator does not emit is outside it — recorded with its citation
  because that behaviour is upstream's, not ours.

## Which obligations were already the contract's, and which was not

**Two of three were. The third was a `dev-writer` default with a `NO SPEC:`
marker on it, and this change is what made it load-bearing.** An earlier version
of this proposal counted all three as `module-wire-contract` obligations the
publish handlers had escaped. That reads as three *contract* violations; it was
two. Corrected here rather than softened, because the correction is the
interesting half.

**`module-wire-contract` already reached them, for the non-object rule and the
three-messages rule.** It scopes the envelope rule to *"every method that **reads
a field** of its request"*, states that scope's three cases explicitly, and says
the surface carries *"exactly one method in that third case: the panic probe"*.
The publish handlers read `stoa`, `body`, `parent`, `target` and `direction`.
They are inside the rule's first case, and they were violating it — an
implementation defect against a contract that was already correct.

The requirement is also emphatic on precisely this point: *"This SHALL hold for
**every** such method, whatever fields that method requires ... The rule is
therefore stated once, for the envelope, rather than left to each method's
fields to imply."* Adding a requirement naming the publish handlers would make
the contract weaker, not stronger: a rule restated per method is a rule the next
method is outside. So the delta below names no handler.

**`content-authoring` already decides the signing key**, in the scenario quoted
above. Nothing about it needed widening; the code needed to catch up with it.

**The size cap was in no merged spec at all.** Not in `module-wire-contract`, not
in the archived `wire-request-envelope` delta that built the envelope. The one
place in the merged spec set that bounds a size is `keystore`, and that is about
a keystore *file's* content — a different thing. The cap existed only as code,
carrying a `NO SPEC:` marker saying so in as many words, and this change pinned
it across the whole request-taking surface. That is the moment an absent decision
stops being cheap to revisit: the sweep is now the thing a fifteenth method
inherits, and it would inherit an unspecified number.

**So the cap is promoted rather than the prose softened**, for a reason that is
about the cap and not about tidiness: the property that matters is the
**ordering**, and the ordering is the half a reader cannot infer. A limit checked
after the parse bounds nothing — the ~2N allocation it exists to refuse has
already been paid — and what failing to pay it costs is not an error reply but
the module process, per `docs/PHASE0-FINDINGS.md` §3. A security property whose
whole content is "this check runs first" is exactly what a behaviour contract is
for, and leaving it in a code comment leaves the next implementation free to get
the order wrong while every test still passes.

What the delta does **not** do is put the number in the spec. It requires that a
limit exists, that it is one number, that it is checked first, and that it is
bounded from both sides — large enough for the biggest op `op-format` permits,
materially below what costs the module its process — and it requires the
implementation to record where its number sits between those two. A number in a
spec is a claim no gate reads and that rots in silence; the bracket is the part
that stays true.

## The anti-staleness gate rests on an upstream premise, now recorded

The sweep that makes the method list unable to go stale classifies every method
in the dispatch trait, and lets exactly one shape through unswept: a method with
a **default body**. That is correct — the generator does not put such a method on
the wire — and it was verified by reading the generator rather than the comment
asserting it.

But "a defaulted method is not on the module's wire surface" had become a premise
of this repo's only gate against the method list going stale, and it lived in one
code comment plus one pinned flake input's source. `logos-module-builder` is
pinned in `dialectica/flake.nix` and supplies the generator; a bump that started
emitting defaulted methods would put a request-taking method on the wire with
every gate in this PR green.

So `module-wire-contract` — the capability that defines what "the surface" is —
now carries it, as a stated assumption with its citation rather than as a bare
claim. The difference matters and is CLAUDE.md's rule: *"we depend on upstream
behaviour X, verified at this revision"* fails visibly when the pin moves;
*"X is true"* does not fail at all.

## Impact

- `dialectica-core`: `wire.rs` — the three handlers, the prologue type,
  `stoa_of`, `publishing_key`, and the sweep.
- `dialectica`: `rust-lib/src/lib.rs` — the adapter's publish assembly, which
  now parses nothing itself.
- `.github/workflows/ci.yml` — the exemption removed, two calls required, and a
  ban on the adapter parsing a request at all.
- `docs/PLAN.md` §9.2 — the prologue reshape struck through as done, the unlock
  ordering written out as what remains.
- `openspec/specs/module-wire-contract` — one `ADDED` requirement for the request
  bound, and the surface requirement `MODIFIED` to say what the surface is. No
  behaviour changes with them: both specify what the code already does, which is
  the point — an unspecified obligation pinned across fourteen methods is what
  the delta exists to stop being unspecified.
- No change to any wire reply a correct caller receives. What changes is which
  requests are refused, what those refusals say, and which key signs.

## What this change deliberately does not fix

**The adapter still unlocks before it validates.** `open_from_env` runs a 64 MiB
Argon2id derivation before the forbidden-field guard and every required-field
read, so a malformed request buys a full memory-hard KDF and is then refused.

That is a real DoS amplifier, it predates this change, and the change's original
error was *claiming* to have fixed it rather than leaving it. The fix means the
three handlers taking a fallible key supplier instead of a key — a second
reshape of the same three signatures, which PLAN.md §9.2 already flags as one to
judge on its own merits. `design.md` decision 8 carries the argument and says
where it goes; PLAN.md §9.2 records it so it outlives this change folder.
