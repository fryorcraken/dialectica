# Design — expose-name

## Context

`generated-names` builds `display_name(&PublicKey) -> DisplayName` and
`display_name_from_bytes(&[u8]) -> Result<DisplayName, NameError>`, and no caller
can reach either. This change adds the one wire method that closes that, and
nothing else.

The derivation already exists and is already total over well-formed keys. So the
whole of this design is about the **boundary**: what the method takes, what shape
it answers in, and what it refuses. There is no new logic behind it — the handler
parses a field, hands the bytes to `display_name_from_bytes`, and renders the
answer.

## Goals / Non-Goals

**Goals:**

- One wire method that turns a supplied public key into that key's name, so the
  derivation `generated-names` specifies becomes observable from outside core.
- Refuse exactly what the identity layer refuses, by deferring to it rather than
  by restating its rules — including the low-order point a length check misses.
- Inherit the whole request envelope (non-object refusal, size cap, null
  readings, panic guard) rather than reimplementing any part of it.

**Non-Goals:**

- **No batch shape**, decided rather than deferred — see decision 1.
- **No caching or memoisation.** The derivation is a pure function of 32 bytes;
  a cache would be state where the spec requires none, and *"obtaining a name
  changes nothing"* is easiest to guarantee when there is nothing to change.
- **No change to any existing reply.** No feed row, thread item or slate
  candidate gains a name field.

## Decisions

### 1. One key per call, not a batch — and the reason is the failure shape, not the cost

**Chosen:** `display_name` takes exactly one `publicKey` and answers exactly one
name.

This is the decision the spec deliberately left open, and the brief flagged the
real tension: `docs/PLAN.md` §2.4 says every `modules().x` call is IPC, and a
name per feed row is exactly the per-item call §2.4 warns about. PLAN.md §9
already declined `getPost` on precisely this ground. So the pressure toward a
batch is genuine and was not dismissed.

**What ruled the batch out is that its reply shape is forbidden.** A batch takes
N keys, and some subset of them will be malformed — this method's whole error
condition is attacker-supplied key material, which arrives per key rather than
per request. A batch reply must therefore either:

- report per-key outcomes, which is a **partial success**: some names, some
  errors, in one reply. `module-wire-contract` forbids that outright — failure is
  always `{"error":"..."}` and never a partial shape. This is not a convention I
  could argue around: a view that renders a list where entry 3 is an error object
  and entry 4 is a name has to branch on shape per item, which is the thing the
  source-independence convention exists to stop.
- or fail the **whole batch** on one bad key, which is worse than it sounds. One
  peer sending one malformed key would blank every name on the screen — a hostile
  peer's free way to erase a whole page's attribution. Key material reaching this
  entry point is attacker-controlled by the spec's own account of it ("it arrives
  inside an inbound op and is handed on by a view rendering somebody else's
  authorship"), so one bad key among many is the expected case rather than a rare
  one, and a shape that lets it take out the good keys' answers is the wrong
  shape.

There is no third shape. A batch that dropped bad keys silently would return
fewer names than keys with no way to say which, so a caller could not align the
answers with the rows it asked about, and would render somebody's name against
somebody else's post. That is the worst outcome available here.

**The cost of one-per-call is real and is bounded.** A feed page is
`clamp_per_page`-bounded, so a screen costs at most that many calls, and each is
a pure function over 32 bytes with no store, no log and no lock — the cheapest
call on this surface. §2.4's warning is about hot loops over expensive calls;
this is a hot loop over the cheapest one. If measurement later shows the round
trips dominate, a batch method answering the same question per key satisfies
every requirement written in the spec, so **this decision is reversible** and the
spec's one-key contract does not foreclose it.

What a future batch would have to solve first is the failure shape above. The
honest form is probably `{"names":[...]}` where each entry is a name **or null**,
with the refusal reason omitted entirely — a shape that is uniform per item and
so not a partial success. I have not specified it, because designing an unused
method against an unmeasured cost is exactly the speculative widening CLAUDE.md
asks me not to do.

### 2. The key travels as hex, because that is what this surface already speaks

**Chosen:** `{"publicKey":"<64 hex chars>"}`.

Every public key already on this surface is hex: `posting_identity` and
`whoami_for` both emit `"publicKey": <hex>`, and `thread-read` ships a key per
item the same way. A caller reaching this method holds a key it got from one of
those replies, so hex means it forwards the value it was handed, unmodified.

Base64 or a byte array would each be a second encoding for one type on one
surface, which is a per-call decision a caller can get wrong. Rejected for that
reason alone — there is no efficiency argument at 32 bytes worth weighing
against it.

**The field is named `publicKey` to match what the replies emit**, so the value
and the field name travel together.

### 3. A hex-length bound before the decode, copied from `genesis_for`'s reasoning

`hex::decode` allocates `len/2` bytes from a length the caller chose. A public
key has a known, fixed size, so a 4 MiB hex string (the envelope cap admits one)
can be refused for nothing rather than decoded into a 2 MiB `Vec` that
`PublicKey::from_bytes` then rejects for its length.

This is the same ordering `genesis_for` uses and for the same measured reason:
per PHASE0-FINDINGS §3 the price of an allocation failure here is a module
**abort**, not an error reply. The bound is derived from the key size rather than
spelled as a literal, so it cannot drift from it.

**This is not a second length check competing with the identity layer's.** It
bounds the *hex string* before allocating; `PublicKey::from_bytes` remains the
sole authority on whether the decoded bytes are a key. A 30-byte or 34-byte hex
string passes this bound and is refused by the identity layer, which is what the
spec requires — the refusal that matters is still the identity layer's answer.

### 4. Three distinguishable refusals, because the spec requires two of them to differ

The spec requires that "no key material at all" and "bad key material" carry
**different messages**, so a caller can tell a request it malformed from a key it
should stop trusting. The handler therefore reports:

- `missing field: publicKey` — the field is absent.
- `publicKey must be a string` — present and wrong-typed.
- `publicKey is not valid hex` — a string that is not hex.
- `cannot derive a display name: <identity layer's words>` — hex that decoded to
  bytes the identity layer refuses. This carries `NameError`'s own `Display`,
  which carries `KeyError`'s, so a **low-order point** reads as
  *"low-order public key, which can never verify a signature"* and a wrong-length
  or non-point key reads as *"not a valid public key"*.

The last of those is the load-bearing one. Deferring the wording to `NameError`
rather than writing a message here is what makes the requirement *"the entry
point admits exactly what the identity layer admits"* true by construction: there
is no second list of shapes in this file that could drift from `PublicKey::from_bytes`.

### 5. The reply carries the rendered name and its three words

**Chosen:** `{"name":"pensive aporia of lampsakos","words":["pensive","aporia","lampsakos"]}`.

`DisplayName` has both `render()` and `words()`, and `generated-names` states why
`words()` exists: the connector is the one part of a name a cramped caller may
drop, and it is droppable precisely because it is the only part not derived from
the key. A view with a narrow column needs the three words without re-parsing
`name` on a space — which it cannot do correctly anyway, because a place entry
may be a two-word toponym (`alexandria troas`), so `name.split(' ')` yields four
or five tokens and the naive split is wrong.

So emitting both is not redundancy: `name` is what to render, `words` is the
structure, and shipping only `name` would oblige every caller to reimplement a
split that the wordlists make unsound.

**No gloss field**, per the proposal — that is issue #82.

### 6. The handler is not re-exported at the crate root, unlike every other one

`dialectica-core`'s root re-exports every wire handler by name, so the adapter
can call `core::ping` without knowing which submodule it lives in. This handler
is the one exception: it is reached as `core::wire::display_name`.

The reason is that `names::display_name` already holds that name one import depth
from the same root, and the two are different in kind — the `names` one takes a
`&PublicKey` and returns a `DisplayName`, the `wire` one takes and returns wire
JSON and is the boundary that refuses malformed key material. Re-exporting would
put `core::display_name` and `core::names::display_name` a single word apart,
where picking the wrong one is easy and the compiler would only sometimes catch
it.

The alternative considered was renaming one of them. Rejected both ways: renaming
the `names` function churns an archived, pinned capability for a caller's
convenience, and renaming the wire method would make the JSON method name and the
Rust function name disagree, which is worse than a longer path. `core::wire::` is
already the established form for handlers outside the list
(`get_capabilities_from_stores`, `publishing_key`).

**This was found by the scaffold build, not by review.** `cargo test`, `clippy`
and the whole local suite passed with the adapter calling a function that does
not exist, because the `impl DialecticaModule` block is `#[cfg(logos_scaffold)]`
and no local gate compiles it. `nix build .#lgx` is the only gate that sees the
adapter, and it must be run for any change that touches it.

### 7. `guarded` wraps it like every other handler

The spec requires that arbitrary key material not take the module down. Two
things deliver that, and they are worth keeping distinct:

- **`display_name_from_bytes` is a `Result`, not a panic** — it is the identity
  layer's refusal, already tested in `names.rs`.
- **`guarded` is the backstop**, as on every handler. It is not load-bearing for
  any input I can construct; it is there because a handler without it is the one
  that aborts the process when something below it panics for a reason nobody
  predicted.

Stating both is the point: the panic guard being untriggerable by this method's
inputs is a property worth *having*, not a reason to drop the guard.

## Risks / Trade-offs

- **A name per feed row is N IPC calls, and §2.4 warns about exactly that.** →
  Mitigated by what the call is rather than by batching it: a pure function over
  32 bytes with no store, no log and no lock, over a `clamp_per_page`-bounded
  row count. Decision 1 records why the batch shapes available are each worse
  than the round trips, and that a batch remains addable without changing any
  requirement written here.

- **The method names a key that no identity this peer knows.** → Not a risk but
  a requirement (*"a name is obtainable for a key belonging to no known
  identity"*): a name is a function of the key alone, consulting nothing. What
  would be a risk is a caller reading a returned name as evidence the key is
  known or trusted. `generated-names` is explicit that a name is **not a
  credential** and **not an identifier** — anyone willing to press a regeneration
  button reaches any name they like — so the reply deliberately carries no field
  suggesting either, and returning a name says nothing about its bearer.

- **A caller could put a returned name onto the wire**, which *The name SHALL NOT
  travel* forbids. → Nothing in core can prevent that, and this change does not
  make it easier: the name is already computable by anyone holding the key, so
  the method adds no capability an attacker lacked. What it removes is the
  pressure toward a second QML implementation, which is the divergence risk that
  actually matters.

- **The hex bound in decision 3 could be read as a second length check** that
  drifts from the identity layer's. → It bounds the hex string's length before
  allocating and decides nothing about validity; a wrong-length key that clears
  it is still refused by `PublicKey::from_bytes`. Pinned by a test asserting the
  entry point's verdict matches the identity layer's over a corpus that includes
  wrong-length material on both sides.

## What this change does not do

- **No feed row gains a key.** The feed read still returns an address per row and
  drops the key, so a feed row's name remains underivable — a caller holding only
  an address cannot arrive at the right name, which `generated-names` already
  establishes. That is `content-authoring`'s reply shape and its own piece. This
  change makes the method exist; it does not close that gap and does not claim
  to.
- **No reply anywhere gains a name field.** *The name SHALL NOT travel* is
  untouched: the only way to a name is to ask for one with a key in hand.
