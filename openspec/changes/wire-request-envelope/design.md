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

There are five such parse sites (`ping`, `get_capabilities`,
`list_threads_inner`, `list_threads_from_request`, `parse_channel_id`), plus two
helpers that read fields off an already-parsed `Value` (`genesis_for`,
`parse_index`). Three other changes are adding wire methods in parallel, so the
count is rising.

## Goals / Non-Goals

**Goals:**

- A non-object request is refused, with a message distinguishable from
  "invalid JSON" and from "missing field".
- A method added *after* this change is envelope-checked without its author
  having to know this change happened.

**Non-Goals:**

- Field-level typing. The envelope constrains the request's outer type; a
  wrong-typed *field* stays each handler's own error with its own message.
- Unknown-field strictness — argued in `proposal.md` — Impact.
- The reply half, already contracted, and the cross-module reply decoder, which
  is the opposite direction.

## Decisions

### 1. The check lives in a type, not in a branch per handler

**Chosen:** a `Request` newtype over `serde_json::Map<String, Value>` with a
private field and one fallible constructor. Handlers read fields through
`Request`, never through a `Value`.

```rust
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
reintroduces the defect. That is how the defect arrived. With a `Request`, the
compiler answers: the inner map is private and `parse` is the only constructor,
so a handler holding a `Request` provably went through the check, and a handler
written next month inherits it for free.

This is the instruction to put complexity in the data structure rather than the
logic, and it is also the mechanism by which the three parallel branches adopt
the rule on merge without anyone patching them individually.

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
silently. So the text is `const REQUEST_NOT_AN_OBJECT` (one string, not five)
and tests compare against the literal, not against `!= other_message`.

Hardcoded expectations are this project's recorded fix for tests that ask the
implementation what it did and then agree with it.

### 4. `panic_probe` is deliberately not envelope-checked

It takes a request and never decodes it — it formats the raw string into a panic
message, reading no field. There is nothing for the envelope to protect, and
checking it would make `panic_probe("[]")` a refusal instead of an exercise of
the panic guard, which is the only reason that method exists.

The spec scopes the rule to "every method that **accepts a request**", and the
`Request` type makes that scope legible: a method holding a `Request` is checked;
a method holding a `&str` it never decodes has nothing to check. Recorded here,
and marked `NO SPEC` in the test, so it reads as a decision rather than an
omission.

### 5. `{}` is not special-cased

It falls out: `{}` parses as `Value::Object`, so `parse` accepts it and the
handler's own field checks run. No code is needed — but a test is, because "`{}`
is refused for its field, not its shape" is precisely the assertion that
separates the two refusals.

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

- **A future handler could reach around the type** by calling
  `serde_json::from_str` itself → Nothing in the compiler prevents it. The
  mitigation is that `Request::parse` is the obvious path, is re-exported at the
  crate root beside the handlers, and is the only parse of a *request* left in
  the crate — so a second one is visible in review as an anomaly rather than
  hidden among five identical ones.

- **The refusal message is now contract surface a view may render** → Changing
  its wording is a contract change, which is why it is a `const` with a test
  pinning the literal. That is the intended cost, not an accident.
