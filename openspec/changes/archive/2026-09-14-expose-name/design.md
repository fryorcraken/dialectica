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

A third shape — dropping bad keys silently — is worse than either: it returns
fewer names than keys with no way to say which, so a caller could not align the
answers with the rows it asked about, and would render somebody's name against
somebody else's post. That is the worst outcome available here.

**What this does not establish is that no batch shape exists.** An earlier
version of this entry said "there is no third shape" and then, fourteen lines
later, described one — the name-or-null list below, which is uniform per item
and so is not a partial success at all. The real ground for one key per call is
weaker than "forbidden" and entirely sufficient: **the batch is unnecessary
until measured, and the per-item cost is the cheapest call on this surface.**
Someone revisiting this should go and measure round trips, not conclude the
door is shut. For the avoidance of the specific wrong turn: `module-wire-contract`
(`openspec/specs/module-wire-contract/spec.md:271`) forbids *"A reply SHALL NOT
carry both an error and a result"*, which is a statement about the top-level
reply envelope; a uniform name-or-null list does not violate it.

**The cost of one-per-call is real and is bounded.** A feed page is
`clamp_per_page`-bounded, so a screen costs at most that many calls, and each is
a pure function over 32 bytes with no store, no log and no lock — the cheapest
call on this surface. §2.4's warning is about hot loops over expensive calls;
this is a hot loop over the cheapest one. If measurement later shows the round
trips dominate, a batch method **added alongside this one** satisfies every
requirement written in the spec, so **this decision is reversible** and the
spec's one-key contract does not foreclose it.

What a future batch would have to solve first is the failure shape above. The
honest form is probably `{"names":[...]}` where each entry is a name **or null**,
with the refusal reason omitted entirely — a shape that is uniform per item and
so not a partial success. I have not specified it, because designing an unused
method against an unmeasured cost is exactly the speculative widening CLAUDE.md
asks me not to do.

**That batch would be additive, not a substitute, and the distinction is
load-bearing.** Carrying no message per key, it cannot tell absent key material
from bad key material — the very distinction §4 is built around and which the
spec states as *"the two refusals carry different messages"*. So a batch that
**replaced** this method would breach a requirement the spec makes; a batch
beside it leaves the single call as the route that distinguishes, which is why
the reversibility claim above survives. Anyone adding one must keep this method.

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
**abort**, not an error reply.

**The `32` in `32 * 2` is a duplicated literal, not a derivation.** An earlier
version of this entry claimed the bound "is derived from the key size rather
than spelled as a literal, so it cannot drift from it". It cannot: `identity.rs`
exports no key-size constant — every site spells `[u8; 32]` inline — so nothing
ties the expression to `PublicKey` and a change of representation would not
carry. What pins the value is a test, `the_hex_bound_is_pinned_to_a_known_answer`,
asserting a hardcoded `64`. That matters because `cargo mutants` does not mutate
a `const`: the bound was measured loosening 128-fold, to `4096 * 2`, with all
970 tests green — restoring exactly the 2 MiB transient allocation it exists to
prevent. The pin follows `the_request_cap_is_pinned_to_a_known_answer`'s shape
and records the same reasoning.

**Where it decides nothing, and where it decides everything.** At or under 64
characters the bound is transparent: material goes to `PublicKey::from_bytes`,
which remains the sole authority on whether decoded bytes are a key, so a 29-,
30- or 31-byte hex string clears this bound and is refused *there* — which is
what the spec requires.

Above 64 the bound is the **sole** decider, and saying otherwise was the error
worth recording. An earlier wording here claimed "a 30-byte or 34-byte hex
string passes this bound and is refused by the identity layer". Measured: a
34-byte key is 68 characters and a 33-byte key is 66; both exceed 64, so both
are refused by the bound with `publicKey is N bytes, over the 64 a public key's
hex holds`, and neither ever reaches `hex::decode` or `PublicKey::from_bytes`.
Only the short half of that sentence was ever true, and it was the over half the
sentence existed to defend. The honest claim is the narrower one above, and both
halves of it are now asserted by
`the_bound_decides_every_over_length_refusal_and_the_identity_layer_never_sees_one`
rather than left to a reader's arithmetic.

### 4. Five distinguishable refusals, because the spec requires two of them to differ

The spec requires that "no key material at all" and "bad key material" carry
**different messages**, so a caller can tell a request it malformed from a key it
should stop trusting. The handler therefore reports:

- `missing field: publicKey` — the field is absent.
- `publicKey must be a string` — present and wrong-typed.
- `publicKey is N bytes, over the 64 a public key's hex holds` — the allocation
  bound of decision 3, reached before the decode.
- `publicKey is not valid hex` — a string that is not hex.
- `cannot derive a display name: <identity layer's words>` — hex that decoded to
  bytes the identity layer refuses. This carries `NameError`'s own `Display`,
  which carries `KeyError`'s, so a **low-order point** reads as
  *"low-order public key, which can never verify a signature"* and a wrong-length
  or non-point key reads as *"not a valid public key"*.

**The count is five, and getting it wrong hid one from the caller.** This
heading said "three" over a list of four while the code emitted five; the size
message was the one appearing in no list, because decision 3 framed it as an
allocation bound rather than as something a caller receives. It is both. A
caller reading this section to learn what it must handle was handed an
incomplete set.

**Which of the spec's two categories does the size refusal fall in? Neither.**
The spec requires "no key material" and "bad key material" be told apart; an
over-length hex string is key material that was supplied and is not merely
malformed-as-a-key — it was never examined as one. It is a *request*-shaped
mistake, like the absent field and the wrong-typed field, rather than a verdict
on key material. Three of the five are request-shaped (absent, wrong-typed,
over-length), one is encoding-shaped (not hex), and exactly one is the spec's
"bad key material". The spec's requirement is satisfied because its two named
categories are distinguishable from each other; the other three are additional
and the spec does not speak to them.

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
may be a two-word toponym, so `name.split(' ')` yields five tokens and the naive
split truncates the place to its first word.

**The example must be a real one.** `wordlists/places.txt` holds ten multi-word
entries in 1,024: `antiocheia maiandros`, `arsinoe kyprou`, `euxeinos pontos`,
`herakleion egyptou`, `kimmerian bosporos`, `lokroi epizephyrioi`,
`makaron nesoi`, `rhode iberias`, `seleukeia kalykadnos`, `thermai himeraiai`.
An earlier version of this section cited `alexandria troas`, which is **not**
among them — the list holds bare `alexandria` and no `troas` compound. The
reasoning was sound and the example invented, which is the failure mode where
the reader most likely to check the citation is the one it misleads.

So emitting both is not redundancy: `name` is what to render, `words` is the
structure, and shipping only `name` would oblige every caller to reimplement a
split that the wordlists make unsound.

**And that claim is now pinned rather than merely argued.** Replacing
`name.words()` with a split of the rendered name on spaces was measured
**passing the entire suite**, because no fixture drew one of the ten. Seed 166
does — `intaxable eidos of thermai himeraiai`, three drawn words rendered as
five tokens — and
`words_is_the_three_drawn_words_and_not_the_rendered_name_split_on_spaces`
asserts against it, so the mutation now returns a four-element array and fails.

**No gloss field**, per the proposal — that is issue #82.

### 5a. The reply's field names are this change's choice, not the spec's

`name` and `words` are **unspecified**. The `generated-names` capability requires
the derivation and requires the three words be reachable separately from the
rendered name — which is what makes its *the connector may be elided* relaxation
usable by a caller — but it names no field, fixes no encoding, and does not
spell `publicKey` either. A second implementation reading the delta alone could
ship `{"key":"<base64>"}` returning `{"name":…}` with no `words` and satisfy
every scenario.

These are recorded here **and** marked `NO SPEC:` in the tests, because the
convention in this file treats the two as complementary rather than alternative:
`wire.rs:1797` pairs exactly such a marker with a design entry, and "it belongs
in `design.md`" was the reasoning that left these unmarked. A marker is how the
spec reviewer finds behaviour that was chosen rather than required; a design
entry is where the choice is argued. Both, or the choice becomes permanent by
accident.

The choices, and why: **`name`** because the value is the rendered display name
and nothing shorter distinguishes it; **`words`** because it is literally the
three drawn words; **`publicKey`** because that is the spelling every emitter on
this surface already uses (decision 2). Changing any of the three later is a
breaking change to the module surface.

### 8. The fourth sweep list, and why it alone has no trip-wire

This surface carries four hand-maintained lists a new request-taking method must
join. Three were met deliberately; the fourth was missed, silently, with every
gate green — `one_field_has_one_null_reading` builds its own `cases` vec of
eleven fields and `publicKey` was not among them.

**Why it has no trip-wire, recorded so the absence reads as a decision.** The
other three enumerate *methods*, and the dispatch trait also enumerates methods,
so `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`
can compare the two and fail naming what is missing. This one enumerates
*fields*, and nothing in the source enumerates those — each is a string literal
inside one handler's body. Deriving the list would mean parsing handler bodies
for `parsed.get("…")`, which is a gate whose input the next handler's formatting
can corrupt, and this project has recorded that a gate the defect satisfies is
worse than no gate because it closes the question.

So the list stays hand-maintained and the obligation is recorded in `tasks.md`
where the next author reads it. That is a weaker guarantee than the other three
have, and naming it as weaker is the point.

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
Rust function name disagree, which is worse than a longer path.

**The precedent, stated accurately.** An earlier version of this entry claimed
`core::wire::` is "the established form for handlers outside the list
(`get_capabilities_from_stores`, `publishing_key`)" — and named a handler that is
*inside* it. Verified: `get_capabilities_from_stores` is in the root re-export
list (`dialectica-core/src/lib.rs:61`) and the adapter still spells it
`core::wire::` at `rust-lib/src/lib.rs:764`; `publishing_key` is genuinely
absent from the list and is the one real example. So what the precedent shows is
that **the long path and list membership are independent** — the adapter uses
the long form for handlers both in and out of the list. It is not evidence that
omission is required. The omission stands on the ambiguity argument above and on
nothing else, which is enough.

**The name itself is an open question, not a settled one.** `display_name` is a
noun phrase among fifteen verb phrases on this trait, and it collides with the
key `display_name` in `dialectica/metadata.json:3`, which binds the *module's*
human label (`"Dialectica"`). A verb-first name — `derive_display_name`,
`name_for_key` — would dissolve this decision's ambiguity outright rather than
routing around it with a longer import path. That is a change to the core API,
which is the deliverable, so it is the owner's call and has been raised; this
section describes the shape as built under the current name.

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
  row count. **The bound is 100** (`feed.rs:96` and `thread.rs:95` both set
  `MAX_PER_PAGE = 100`, `DEFAULT_PER_PAGE = 20`), so the worst case is 100 IPC
  round trips for names alone and the typical case 20. Decision 1 records why
  the batch shapes available are each worse than the round trips, and that a
  batch remains addable alongside without changing any requirement written here.

  **This is a reframing of PLAN.md, and saying so is the point.**
  `docs/PLAN.md:2679-2681` argues on **round-trip count**: *"Every cross-module
  call is IPC (§2.4), and a feed is a loop. A method that answers one post per
  call turns a page of thirty into thirty IPC round trips. The paginated shape
  is not politeness; it is the only shape that works."* The counter above is
  that §2.4's warning is about hot loops over *expensive* calls and this is the
  cheapest one — a fair distinction, and I think it holds, but it answers a
  cost-per-call argument to a passage framed on call *count*. A future reader
  weighing a batch should have both the number (100) and the fact that PLAN.md's
  wording points the other way.

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
  allocating and decides nothing about validity **for any string at or under 64
  bytes**, which is where every valid key and every short refusal lives; a
  wrong-length key that clears it is still refused by `PublicKey::from_bytes`.
  Above 64 the bound is the sole decider and the identity layer is not consulted
  at all, which is the correct behaviour for an allocation bound and is stated
  here rather than glossed over.

  **The earlier version of this entry claimed a test pinned that separation, and
  no test could see it.** `the_entry_point_admits_exactly_what_the_identity_layer_admits`
  carries `vec![0xab; 33]` and `vec![0xab; 64]` — 66 and 128 characters — and
  asserts `entry_point_named == identity_layer_accepts`, i.e. `false == false`.
  Both are refused by the bound, so it passes identically whether the identity
  layer is ever reached: a gate whose input the defect satisfies. The claim is
  now narrowed to what is true, and the division it describes is pinned by
  `the_bound_decides_every_over_length_refusal_and_the_identity_layer_never_sees_one`,
  which asserts *which* layer refuses at each length rather than only that
  something did.

- **The bound's ordering, value and field source were each unguarded**, and the
  code was correct on all three. → What was missing was anything keeping it
  correct, so three tests were added, each proved red against the mutation it
  names:
  - **Ordering** — `the_hex_bound_is_checked_before_the_decode_allocates` feeds
    material both over the bound *and* not valid hex, asserting the size refusal
    comes back and `not valid hex` does not. Swapping the two blocks was measured
    passing 940/940 before, at 17x the cost (39.56 ms vs 2.32 ms) plus a 2 MiB
    transient allocation per call; it now fails exactly one test.
  - **Value** — `the_hex_bound_is_pinned_to_a_known_answer` asserts a hardcoded
    64, because `cargo mutants` does not mutate a `const` and the bound was
    measured loosening 128-fold with everything green.
  - **Field source** — `the_key_is_read_from_the_public_key_field_and_from_no_other`
    asserts that key material under `authorAddress` (and four neighbouring
    spellings) is not read as the key. Adding an `.or_else` fallback made the
    handler answer an address-holder with a name — the outcome *"an author
    address SHALL NOT be required, in addition to or in place of the key"*
    exists to forbid — with 970/970 passing.

## What this change does not do

- **No feed row gains a key.** The feed read still returns an address per row and
  drops the key, so a feed row's name remains underivable — a caller holding only
  an address cannot arrive at the right name, which `generated-names` already
  establishes. That is `content-authoring`'s reply shape and its own piece. This
  change makes the method exist; it does not close that gap and does not claim
  to.
- **No reply anywhere gains a name field.** *The name SHALL NOT travel* is
  untouched: the only way to a name is to ask for one with a key in hand.
