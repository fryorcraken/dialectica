# Design: the request envelope

## Context

See `proposal.md` — Why, for the motivation. The constraint that shapes the
approach is what the code actually looks like. Every handler in `wire.rs` opens
the same two steps:

```rust
let parsed: serde_json::Value = match serde_json::from_str(request) { … };
let stoa = match parsed.get("stoa") { … };
```

`serde_json::Value::get` returns `None` for **every** non-object — an array, a
number, a string, `null`. So `parsed` being `[]` and `parsed` being `{}` are
indistinguishable to every line that follows. The parse is the only boundary
there is, and it does not check the type of what it parsed.

There were five such parse sites (`ping`, `get_capabilities`,
`list_threads_inner`, `list_threads_from_request`, `parse_channel_id`), plus two
helpers that read fields off an already-parsed `Value` (`genesis_for`,
`parse_index`). Three other changes are adding wire methods in parallel, so the
count is rising.

**Two of those five were the same request parsed twice** —
`list_threads_from_request` parsed it, then handed the raw `&str` to
`list_threads`, which parsed it again. Decision 1's Risks entry covers the fix; the
count after this change is four `Request::parse` calls for four distinct requests.

## Goals / Non-Goals

**Goals:**

- A non-object request is refused, with a message distinguishable from
  "invalid JSON" and from "missing field".
- A method **that reads a field of its request**, added *after* this change, is
  envelope-checked without its author having to know this change happened — insofar
  as that is achievable, which Decision 1 is careful to bound.
- A request cannot cost the module unbounded work before it is refused
  (Decision 6).

**Non-Goals:**

- Field-level typing. The envelope constrains the request's outer type; a
  wrong-typed *field* stays each handler's own error with its own message.
- Unknown-field strictness — argued in `proposal.md` — Impact. Decision 6 records
  that this leniency and the size cap are one decision, not two.
- The reply half, already contracted, and the cross-module reply decoder, which
  is the opposite direction.
- **Widening what `parse_index` accepts.** `as_u64` refuses by spelling rather than
  by value: `{"page":1e2}` is JSON for exactly 100 and `{"page":0.0}` for exactly 0,
  and both are refused because serde parses a number carrying a `.` or an `e` as
  `f64`. Several serialisers emit `1e2` for 100, so this is a spelling a legitimate
  caller sends. **The message was corrected** — it claimed the value was not a whole
  number, which was false — and the acceptance was deliberately left alone: whether
  an exactly-integral float is a valid index is a contract question the spec does not
  answer, and it does not belong in a change about the envelope. Filed rather than
  guessed at; a caller told the truth can restring its number today.

## Decisions

### 1. The check lives in a type, in a module of its own

**Chosen:** a `Request` newtype over `serde_json::Map<String, Value>` with a
private field and one fallible constructor, **defined in `wire::request` — a file
that contains no handler.** Handlers read fields through `Request`, never through
a `Value`.

```rust
// wire/request.rs
pub struct Request(serde_json::Map<String, serde_json::Value>);
pub fn parse(request: &str) -> Result<Request, String>   // Err is already the wire shape
pub fn get(&self, field: &str) -> Option<&serde_json::Value>
```

**Alternative considered — one guard per handler:**

```rust
if !parsed.is_object() { return error_json(REQUEST_NOT_AN_OBJECT); }
```

This is the smaller diff and it was rejected. CLAUDE.md names the shape
directly: "when you find yourself writing the fourth slightly-different copy of
a guard, that is the signal to reshape". There are five sites and more arriving.

**What ruled it out is not repetition but checkability.** With a `Value` in
hand, "did this handler check?" is a question answered by reading every handler
— and a new handler that omits the branch compiles, passes clippy, and silently
reintroduces the defect. That is how the defect arrived.

**The separate module is load-bearing, and the first version of this change got
it wrong.** A tuple struct's private field is private to its **defining module**,
not to its defining type. `Request` originally lived in `wire.rs` beside every
handler, so this compiled and ran from inside that file — verified, returning 0:

```rust
let bypass = Request(serde_json::Map::new());
let inner_read = bypass.0.len();
```

Which means the claim this design made — that a handler holding a `Request`
provably went through the check — was **false for exactly the population it
named.** And the residual recorded against it ("a second `from_str` is visible in
review as an anomaly") did not cover it either: neither of those lines contains a
`from_str`.

Moving the type to `wire::request` makes the claim true. The bypass now fails to
compile from `wire.rs` with `E0423: cannot initialize a tuple struct which
contains private fields`, and the module holds no handler, so nothing inside it
wants to.

**How that is proved, and why half of it is a note rather than a test.** The
compile failure cannot be a `#[test]` in the file where it matters, and a
`compile_fail` doctest would compile as an *external* consumer — proving the
field is private to other crates, which is a weaker statement than the one at
issue. So the proof is split:

- `wire::request::tests::request_is_constructible_here_because_this_module_defines_it`
  compiles the same line **inside** the defining module, where it is legal. That
  is what makes the failure in `wire.rs` attributable to the boundary rather than
  to a typo.
- `wire::tests::the_bypass_this_module_boundary_closes` records the verified
  compiler error and **says plainly that it is a note, not a test.**

This is the instruction to put complexity in the data structure rather than the
logic, and it is also the mechanism by which the three parallel branches adopt
the rule on merge without anyone patching them individually.

#### What this does NOT buy, measured

**A handler need not hold a `Request` at all**, and the boundary cannot change
that. A reviewer built the sixth method — a handler parsing `Value` directly with
all-optional fields — and it served `[]` as a request that named nothing with the
whole suite green. The same handler was rebuilt after the module move and **still
compiles and still serves `[]`**: `the_sixth_method_the_boundary_does_not_stop`
is that method, kept as a test so the scope of the fix is demonstrated rather
than asserted.

So the guarantee is precisely: **a handler that reads fields through `Request`
went through the envelope check.** It is not "every handler is checked". What
stands against a handler that skips the type is the sweep in
`every_request_taking_method`, whose doc now states the obligation in those words,
and review. Both are human, and that is the residual.

**Read that residual with the two rejected alternatives beside it**, or it reads
as an assumption nobody tested. It is not: reviewer attention is the mitigation
that was *measured* to fail — the sixth method passed a 487-test suite — and both
mechanical replacements are ruled out, the source-scanning one on false positives
and the trait-driven one because the crate that holds the trait cannot run a test
over it. See Rejected alternatives.

### 2. Parse to `Value` first, rather than deserialising straight into a `Map`

**Rejected:** `serde_json::from_str::<Map<String, Value>>(request)`, which would
get the envelope check for free — serde refuses an array for a map.

**It fails with the wrong message.** serde's error for `[]` is
`invalid type: sequence, expected a map at line 1 column 1`, arriving through
the same `Err` arm as a genuine syntax error and therefore reported as
`invalid JSON: …`. The spec requires an unparseable request and a non-object
request be told apart. Keeping two steps — parse untyped, then refuse a
non-object by name — is what makes three caller mistakes produce three messages.

### 3. The message is a pinned `const`, and the tests assert the literal

The spec's obligation is about what the refusal *says*. A message that differs
from its neighbours only by accident satisfies a test but not the requirement —
reword the missing-field message tomorrow and the distinction can vanish
silently. So the text is `const REQUEST_NOT_AN_OBJECT` and tests compare against
the literal, not against `!= other_message`.

**The `const` is not there to avoid duplication.** There is exactly one
production use, inside `Request::parse`, and the type exists precisely so there
is only one. It is named because the string is **contract surface a view may
render**: naming it makes rewording it a deliberate act, and
`the_non_object_message_is_pinned_to_a_known_answer` pins it to a hardcoded
answer so the reword cannot pass unnoticed. That reason survives the number of
uses changing, which a duplication argument would not.

(Worth stating because the first version of this decision — and the constant's
own doc comment — argued from "five call sites". Five was the count under the
per-handler design Decision 1 rejects. It was the third comment on this change
found arguing from a false premise; see the Risks section.)

Hardcoded expectations are this project's recorded fix for tests that ask the
implementation what it did and then agree with it.

### 4. `panic_probe` is deliberately not envelope-checked

It takes a request and never decodes it — it formats the raw string into a panic
message, reading no field. There is nothing for the envelope to protect, and
checking it would make `panic_probe("[]")` a refusal instead of an exercise of
the panic guard, which is the only reason that method exists.

**This was a `NO SPEC` decision and is now specified, in both directions.** The
spec scopes the rule to a method that **reads a field of its request** — not to
one that accepts a request — and states all three cases: reads a field (inside),
takes no request (outside, `version`), takes a request and reads no field
(outside, the probe). The probe also gained a contract of its own under the
panic-guard requirement: its request is opaque text, it reaches its panic for
every request shape, it refuses none, and its message carries what it was given.

So the `NO SPEC` marker is **removed rather than reworded around**, and the test
now cites the requirement. The `Request` type still makes the scope legible: a
method holding a `Request` is checked; a method holding a `&str` it never decodes
has nothing to check.

### 5. `{}` is not special-cased

It falls out: `{}` parses as `Value::Object`, so `parse` accepts it and the
handler's own field checks run. No code is needed — but a test is, because "`{}`
is refused for its field, not its shape" is precisely the assertion that
separates the two refusals.

### 6. A 4 MiB cap on the request, checked before the parse

**Chosen:** `MAX_REQUEST_BYTES = 4 * 1024 * 1024`, compared against
`request.len()` in `Request::parse` **before** `serde_json::from_str`.

There was no cap. Measured, release build, against a valid feed request padded
with one ignored string field:

| padding | request bytes | reply bytes | wall time |
|---|---|---|---|
| 0 | 184 | 373 | 0.13 ms |
| 16 MiB | 16,777,400 | 373 | 13.3 ms |
| 64 MiB | 67,109,048 | 373 | 92.6 ms |

All accepted and served, at ~1.4 ms/MB and ~2N bytes of transient heap. **The
amplification is inverted**, so nothing in the reply signals the cost and nothing
downstream can notice, rate-limit or log it. `ping` is sharper because it echoes
`payload`: 32 MiB in produced a 33,554,443-byte reply. And per
`docs/PHASE0-FINDINGS.md` §3, an allocation failure here is not a slow reply — the
module process **aborts**, the caller waits out a 20-second timeout, and every
later call reports `MODULE_NOT_LOADED`.

**Why this is the one place it belongs.** `Request::parse` is the only way a
handler reaches a field, so one comparison bounds every request-taking method,
including methods nobody has written yet. That is Decision 1's argument one step
further along.

**Why 4 MiB, derived rather than round.** The largest legitimate request this
contract will hold is a composed op: §4.4's SDS message cap gives `op.rs` a
150 KiB per-field bound, and `op.rs` records — measured there — that a `Post` with
attachments decodes to **768,076 bytes** at those bounds. A request carrying that
as JSON with a hex-encoded genesis record beside it (hex doubles) lands near
1.6 MB. 4 MiB clears that with room for a field the future adds, and still refuses
three orders of magnitude below the 64 MiB that was served.

**One number rather than a per-method table**, because a per-method cap is a
second thing each new handler has to declare — the guard-at-every-call-site shape
this design exists to avoid. The tighter bounds that matter are per *field*, and
they live with the field.

**THE CAP IS WHAT BUYS THE LENIENCY, and the two decisions are one.** Unknown
fields are deliberately ignored rather than refused (argued in `proposal.md` —
Impact), because a strict envelope means a newer view cannot talk to an older core
— the worse failure for two modules that update independently. But ignoring
unknown fields is exactly what made the 64 MiB padded request *valid* rather than
refused: every byte sat in a field no method reads. So **dropping this cap also
costs the leniency**, and a future reader tempted to raise it a long way should
price it as a change to both.

**`NO SPEC`.** The spec set mentions no size limit at all, so this is an **absent**
decision rather than a rejected one, and the number is the dev-writer's pending the
spec-writer. Marked on
`every_request_taking_method_refuses_an_oversized_request`.

**Two narrower bounds, kept alongside because they are different layers:**

- `Address::from_hex` allocated `s.len() / 2` bytes before its length check, so a
  64 MiB hex `stoa` built a 32 MiB `Vec` and only then met "must be 32 bytes".
  Guarded on length first — but **only for the over-long case**, because the
  obvious `s.len() != 64` would reclassify `from_hex("nothex!!")` from `NotHex` to
  `WrongLength(4)`, and a security fix that quietly changes a caller-visible error
  is two changes in one diff.
- `genesis_for` did the same with a hex genesis record. Bounded by
  `stoa::MAX_CANONICAL_BYTES * 2`, newly exported from `stoa` — the largest record
  the format can hold is that module's knowledge, and a number copied into
  `wire.rs` would drift from it silently.

Each of the three is tested for its **ordering**, not merely for refusing: every
fixture is over-long *and* malformed, so only an implementation checking length
first can produce the refusal asserted. Reversing the two lines turns each red
while its neighbours stay green. The request cap is tested from both sides of the
boundary, which is what catches a `>` written as `>=`.

### 7. An explicit `null` field is preserved by the envelope and decided by the reader

**Chosen:** `Request::get` returns `Some(Value::Null)` for `{"f":null}` and `None`
for `{}`, faithfully. The envelope preserves the distinction and decides none of
it; each reader applies the contract's reading for its own field.

**Rejected — collapsing a null into absent in `get`** (`self.0.get(field).filter(|v|
!v.is_null())`). That is one line and it would have made "a null means absent" true
everywhere at once. Two things rule it out:

- **It is a behaviour change, not a simplification.** Measured on both sides: of
  the seven production field reads in `wire.rs`, **four observe the difference.**
  `{"payload":null}` goes from `{"pong":null}` — a success — to
  `{"error":"missing field: payload"}`; `{"stoa":null}`, `{"channelId":null}` and
  `{"genesis":null}` each go from `"… must be a string"` to `"missing field: …"`,
  collapsing the wrong-type-against-missing distinction this contract requires.
- **It would fix the reading for a field nobody has written yet.** The contract
  keys a null's meaning to the field's declared type and optionality, so the choice
  belongs at the field and not at the envelope.

That mutation survived **486 of 487 tests**, and the conclusion first drawn from
that — "nothing observes the difference, so the pin is enough" — was wrong. It was
a coverage gap in the handler sweeps. Closed with one handler-level fixture per
differing reader: four independent kills rather than one test's word.

**This is not a departure from PLAN §9.1's "Absent, not null-and-present", in
either direction.** That rule governs `decidedBy` in a **reply**, and rests on
§2.5's no-partial-success: a reply carrying a meaningless field is the partly
successful shape the contract forbids. An omitted *request* field is an unexercised
option, not a half-success, so the argument does not transfer — and the spec now
says so explicitly rather than leaving a reader to wonder which rule won.

**The load-bearing half is the direction, and it is a spec `SHALL NOT`**: a field
must not read a null as absent where the resulting default is the *permissive*
choice; such a field refuses the null as a wrong type instead. `page`, `perPage`
and `includeHidden` qualify only because `0`, the module default and `false` are
each the restrictive answer. `parse_index`'s doc says which half is load-bearing,
where the next author will copy the match arm from; `includeHidden` carries the
local note. A future `includeRemoved`, `asModerator` or `bypassPolicy` written to
the same spelling would be an authorisation bypass.

### 8. Duplicate keys are accepted, last-wins

`{"stoa":"00","stoa":"<64 valid hex>"}` is served on the second value; the first
is discarded without comment. **Kept**, and recorded because undecided-and-unstated
was the finding rather than either behaviour.

It is harmless today — every handler reads each field once, so there is no second
reader to disagree with the first. It is the HTTP parameter-pollution shape, and it
bites the moment a log line, policy check or audit record reads one occurrence
while the handler reads another.

**Refusing it was rejected on cost, not on principle.** `serde_json` has no
`deny_duplicate_keys`, so it needs a custom `Deserialize` visitor or a post-parse
scan of the raw text — a hand-written parser in the one place every request passes
through, to close a gap with no reader today. If a second reader of one field ever
appears, that is the moment to build the visitor, and this entry is the note saying
so.

### 9. The recursion bound is `serde_json`'s, and is being relied on

Deep nesting is not exploitable: `from_str` returns `Err("recursion limit
exceeded")` at depth 1,000 through 1,000,000 — verified — and routes to the
`invalid JSON` arm. An array nested 127 deep is refused correctly by the `_` arm.

But that is a property of the **dependency**, not of this code, and it would
silently vanish under `disable_recursion_limit()` or a `from_reader` variant.
Recorded in `Request::parse`'s doc comment so that neither is introduced without a
depth check taking its place.

## Rejected alternatives

### An `include_str!` test enforcing the "every method" sweep

**Considered and rejected.** The sweep in `every_request_taking_method` is a
manual list, and nothing checks that a new request-taking method is added to it —
the cost of which is measured, not imagined: a reviewer built a sixth method and
the whole suite passed. The obvious mechanisation is a test that `include_str!`s
`wire.rs` and looks for a `serde_json::from_str`, or for a `pub fn` taking a
request, that the list does not account for.

**It would fail for reasons other than the one it names**, and that is what rules
it out rather than effort. A doc comment mentioning `from_str` trips it — this file
has several, deliberately, because the two-step parse is worth explaining. A test
helper legitimately parsing a fixture trips it — this file has several of those
too. A reply *decoder* trips it, and that is the opposite direction entirely. A
test that goes red for three unrelated reasons is a test the next author learns to
delete or to `#[ignore]`, and then the real signal is gone with it.

So the obligation is written where an author adding a method has to be in order to
add it: `every_request_taking_method`'s doc comment now says **ADD YOUR METHOD
HERE**, states that nothing checks it, and names the measured sixth-method result
as the reason. Recorded here so that the next author who reaches for the
source-scanning test finds the objection stated rather than re-litigating it.

### A trait-driven sweep over the real dispatch surface

**The right way to do this, and it cannot be done here.** The principled version
of the test above is not source-scanning at all: enumerate the wire surface from
the dispatch trait — the one thing that definitionally *is* the surface — and
assert that every method taking a request appears in
`every_request_taking_method`. No string matching, no false positives, and it
fails for exactly the reason it names.

It is impossible in this repository for two independent reasons, both checked
against the code rather than reasoned about:

1. **The dependency points the wrong way.** The trait (`DialecticaModule`) lives
   in the `dialectica` crate, and `dialectica` *depends on* `dialectica-core`
   (`rust-lib/Cargo.toml`: `dialectica-core = { path = "dialectica-core" }`).
   `dialectica-core`, where the sweep and every handler live, cannot see the
   trait. So the sweep would have to live in `dialectica`.

2. **And `dialectica` cannot run it.** The trait and its impl are behind
   `#[cfg(logos_scaffold)]`, a cfg `build.rs` sets only when the builder has
   staged `generated/provider_gen.rs`. A plain `cargo test` never sets it — and
   per the comment in `rust-lib/src/lib.rs`, "no arrangement makes it able to":
   committing a copy of the generated file would recreate exactly the
   contract/code drift `codegen.rust.trait` exists to prevent.

**A trait-driven sweep would therefore live in the one crate that cannot run
it.** That is not a gap waiting for effort; it is a property of the module
build, and it will stay true until the SDK offers a surface `cargo test` can
reach.

**Why this is recorded rather than left as silence.** Without it, this document
says the residual is "the sweep and review. Both are human" — which a reader
reasonably takes to mean nobody tried to mechanise it. Two things make that
reading actively misleading:

- The mechanical alternative was investigated and is **structurally
  unavailable**, per the two reasons above. The next agent that reaches for it
  would spend the same afternoon reaching the same conclusion.
- **Reviewer attention is the mitigation that was measured not to work.** The
  sixth method — a handler parsing `Value` directly with all-optional fields —
  was built, and the whole suite passed with it in place at **487 tests**. It was
  rebuilt after the module move and still served `[]`. So "the sweep and review"
  is not an untested assumption being relied on; it is the thing that demonstrably
  failed to catch a handler built specifically to evade it, and it is what remains
  only because the two alternatives above are ruled out.

That is the honest state: the residual is human attention, human attention has
been measured to miss this, and the mechanisation that would replace it is
blocked by the crate graph rather than by anyone's effort.

## Risks / Trade-offs

- **The tests could pass without the fix existing** → This is the defect family
  this project has recorded, and it is live here: `get_capabilities("[]")`
  already errors with `"missing field: stoa"` before any change. Mitigated by
  asserting on the *message* against a pinned constant, and by a test
  (`a_handler_whose_fields_are_all_optional_refuses_a_non_object`) that builds
  the all-optional handler today's surface does not have — verified to fail, with
  `[]` served as `{"hasMore":false,"items":[],"page":0}`, when `parse` is
  mutated to accept non-objects.

- **`Map` is not `Value`, so two helper signatures change** → `genesis_for` and
  `parse_index` move from `&serde_json::Value` to `&Request`. Both are
  `wire.rs`-internal in effect: `genesis_for` is `pub` but has no caller outside
  the file and is not re-exported from the crate root, so no consumer moves.

- **A future handler could reach around the type** by never holding a `Request` at
  all → Nothing in the compiler prevents it, and the module boundary does not
  change that: `the_sixth_method_the_boundary_does_not_stop` builds such a handler
  and demonstrates it serving `[]`. What stands against it is the sweep and review,
  and the sweep's doc now states the obligation rather than only explaining its two
  exclusions. **Do not read Decision 1 as "every handler is checked".**

  An earlier version of this section named the residual as "a future handler could
  call `serde_json::from_str` itself", mitigated by that being "visible in review as
  an anomaly". That mitigation is real but it covered the *wrong* hole — the one
  actually open was constructing a `Request` directly, which contains no `from_str`
  and would have looked like nothing at all in review. Decision 1 records the fix.

- **The refusal message is now contract surface a view may render** → Changing
  its wording is a contract change, which is why it is a `const` with a test
  pinning the literal. That is the intended cost, not an accident.

  **`docs/UI-BRIEF.md` needs no change for it, and that is a decision rather than
  an omission.** The brief states what a view must show, hide or refuse to claim;
  it does not enumerate core's error strings, and none of the four refusals this
  change adds or reworks (`REQUEST_NOT_AN_OBJECT`, the size refusal, the genesis
  length refusal, the corrected index refusal) changes what a view is obliged to
  render. Every one of them arrives through the single `{"error":"..."}` shape the
  brief already tells a designer to render as one error branch. A view renders the
  string; it is not asked to recognise it.

  If a future change gives a view a reason to *branch* on which refusal came back —
  retry on one, re-prompt on another — that is the point at which the brief acquires
  an obligation, because a designer cannot see the strings from where they sit.

- **The parse moved for the feed path, and `list_threads_inner` now takes a
  `&Request`** → `list_threads_from_request` parsed the request and then handed the
  raw `&str` to `list_threads`, which parsed it again: two `Value` trees live at
  once and two nested `guarded` frames, on the one path carrying the larger payload.
  Harmless in output — both replies were byte-identical, which is why it survived on
  main — and not harmless in cost, since it doubled the price of the very lever
  Decision 6 exists to close.

  Fixed rather than conceded in prose. The `&Request` signature is what makes the
  second parse unspellable rather than merely removed; correcting this document to
  admit a second parse would have left the cost in place to keep a sentence true.
