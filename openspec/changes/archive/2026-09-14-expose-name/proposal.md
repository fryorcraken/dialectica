# Expose the name derivation, so a caller can reach it

## Why

`generated-names` (PR #78, merged as `6eec84f`) builds the whole derivation —
`display_name(&PublicKey) -> DisplayName`, three wordlists, the pins, the
determinism contract — and **no caller can reach it.** `grep -n "display_name"`
across `dialectica/rust-lib/dialectica-core/src/wire.rs` returns test code only;
the one production call site was `feed.rs`, which rendered the name onto the feed
row, and the owner's ruling removed that because a name must never travel.
Nothing replaced it. Core holds a derivation that no view can invoke.

**This is not untidiness, it is the feature being unusable.** Basecamp sandboxes
the QML engine with a deny-all network access manager and no filesystem access
outside the plugin directory, so a view holds none of the 10,240 wordlist entries
and cannot derive a name for itself. The alternative to a core method is a second
implementation of the derivation written in QML — 10,240 entries and exact byte
arithmetic that must agree with the Rust one forever. Two implementations that
must agree is precisely the "two peers render one identity differently" failure
that `generated-names`' pinning requirements exist to prevent.

The `generated-names` capability is explicit that this gap is real and deliberately
left open. Under *The name SHALL NOT travel*:

> **How a caller reaches the derivation is not settled here**, and no requirement
> of this capability obliges core to expose one. […] which leaves the derivation
> reachable by no caller until an entry point exists. Tracked as issue #81, and
> out of scope here.

That sentence is the contract this change closes. **The requirement does not
exist yet** — the positive half (*"Core SHALL expose a way to derive a display
name from a public key"*) was written during #78 and **removed from the delta
before it archived**, because nothing implemented it and a live requirement no
code answers is a contract nobody can trust. So this change **adds** the
requirement rather than implementing a standing one.

## What Changes

- **One new wire method: a derivation service.** It takes a public key and
  returns that key's three drawn words. A caller supplies a key it already holds;
  core answers what the key derives to. Nothing is looked up, nothing is stored,
  and no state participates — which is what `generated-names`' determinism
  requirement already obliges of the derivation and what this method inherits by
  calling it.

- **The only failure is key material that is not a public key.** The derivation is
  **total** over well-formed keys — `display_name` returns `DisplayName`, not a
  `Result` — so once the key parses there is no second error condition to report.
  Anything that is not a well-formed public key is refused at the parse, in the
  error shape, and is never reported as a name.

  **"Not a public key" is wider than "the wrong length", and the spec says so
  deliberately.** `PublicKey::from_bytes` also refuses a **low-order point** —
  right length, decodes to a valid Edwards point, and rejected because a key that
  can never verify a signature is not a key worth holding. A test suite written
  against a length check alone would pass over an entry point that named one, so
  the requirement is stated against the identity layer's answer rather than
  against a list of shapes.

- **The method is a derivation service and not a licence to attach names to
  replies.** *The name SHALL NOT travel* is untouched and this change adds nothing
  that weakens it: no feed row, thread item or slate candidate gains a name field.
  A caller reaches a name by asking for one with a key in hand, which is the only
  route, and it is the route that keeps the derived value from riding beside the
  material it derives from.

**Deliberately not in scope**, each because it is another piece:

- **The author address is not removed.** Issue #80 makes the public key the sole
  author identifier, and the owner has ruled its end state: name and mark read
  **disjoint raw byte slices of the public key, with no prefixes at all** — no
  `AUTHOR_ADDRESS_PREFIX`, no `NAME_PREFIX`. This change is specified **against a
  public key input** and states nothing about how the name is derived from it, so
  #80 can land without rewriting a requirement here.
- **The per-word glosses** (issue #82). No reply and no method gains a gloss.
- **The feed read still does not carry the public key** a caller would need to call
  this method for a feed row. That is `content-authoring`'s reply shape, and
  `thread-read` and `identity-onboarding` already ship a key. Closing the feed's
  gap is its own piece; this one makes the method exist.

## Capabilities

### New Capabilities

None. This change adds no capability — the behaviour belongs to the derivation
that `generated-names` already defines, and naming a second capability for "the
method that reaches it" would split one contract across two files.

### Modified Capabilities

- `generated-names`: gains a requirement that the derivation is reachable by a
  caller, and that the reachable entry point refuses malformed key material rather
  than answering with a name. The capability's Purpose already scopes it to "what
  the derivation takes, what it guarantees … and what a name may never be used
  for", and reachability is what makes every other requirement in it observable
  from outside core.

## Impact

- **`dialectica/rust-lib/dialectica-core/src/wire.rs`** gains one handler, and the
  `DialecticaModule` trait in `dialectica/rust-lib/src/lib.rs` — the surface
  declaration the whole `module-wire-contract` capability is derived from — gains
  one method, forwarded from the `#[cfg(logos_scaffold)]` impl. This is a
  **widening of the core API**, which `CLAUDE.md` asks be done on purpose: the
  deliverable is the wire contract, and one more method on it is a decision rather
  than a side effect.
- **Four hand-maintained lists in `wire.rs`' tests** must account for the new
  method — `every_request_taking_method()`, `a_served_request()`,
  `one_field_has_one_null_reading()`'s `cases` vec, and
  `every_method_with_a_required_field()`, which needs **no edit** because it
  filters the first list rather than being a second hand-written one. So three
  gain an entry and the fourth inherits; `tasks.md` records which is which.
  This is the "sweep lists go stale silently" trap,
  and the repo has two trip-wires for it:
  `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares` fails
  naming a method on the trait but missing from the list, and `a_served_request()`'s
  catch-all arm panics naming a method with no fixture. A new field-reading method
  inherits the whole envelope through those sweeps: the non-object refusal, the
  size cap, the null readings, the panic guard. **Neither trip-wire is a reason to
  skip the entries** — they turn an omission into a red test rather than into
  coverage nobody notices missing.

  **`one_field_has_one_null_reading()` has no trip-wire at all**, which is why it
  is the one this change first missed: it enumerates *fields*, and nothing in the
  source enumerates those, so an omission is silent and every gate stays green.
  `design.md` §8 records why deriving it would be worse than maintaining it.
- **`docs/PLAN.md`** §9's method sketch gains nothing to strike through — it never
  sketched a name method, and §5.2.1 is already pruned to "Built — see the
  `generated-names` spec". The reachability gap it records is closed by this change
  rather than restated.
- **One of the two gaps that leave a feed row's name unrenderable closes here;
  the other does not.** This change makes the derivation reachable by a caller
  holding a key. It does **not** put a key in a feed row: the feed read still
  reports an address per row and drops the key, and
  `openspec/specs/generated-names/spec.md:39-45` is explicit that a caller
  holding only an address cannot arrive at the right name — the name derived
  from address bytes differs from the name for that key. So a feed row's name
  stays underivable after this change, for one reason rather than two. That
  second gap is `content-authoring`'s reply shape and its own piece;
  `design.md`'s "What this change does not do" records it as staying open.
- **No QML changes.** Rendering a name is the view pieces' work. This change makes
  the call available.

## The shape question this change does not settle, and why it is design rather than spec

**§2.4 says every `modules().x` call is IPC and warns against per-item calls**, and
a name per feed row is exactly a per-item call. That is a real tension and it is
deliberately left to `design.md` rather than fixed here, because it is a question
about *how a caller uses the method* and not about what the method guarantees.

The spec therefore states the derivation service's contract for one key and says
nothing about batching. If the design concludes a caller needs many names in one
call, that is a shape decision with its own trade — a batch reply is a list whose
failure mode is partial, which `module-wire-contract` forbids outright — and it
belongs in Decisions with the alternatives that lost. Specifying one-key behaviour
does not foreclose it: a batch method answering the same question per key would
satisfy every requirement written here.
