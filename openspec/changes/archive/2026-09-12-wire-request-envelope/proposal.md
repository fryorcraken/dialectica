# The request envelope: a request that is not an object is refused

## Why

`module-wire-contract` contracts the **reply** half of the envelope — "every
reply SHALL be a JSON object" — and says nothing about the **request** half.
That asymmetry is not cosmetic. It is a live defect in merged code.

A handler parses its request into a generic JSON value and then reaches for the
fields it needs. **Reaching for a field of an array, a number or a string yields
the same "absent" as a missing field of an object.** So a request that is not an
object is not refused; it is read as an object in which every field happens to be
absent. For a handler whose fields are all optional, that is a *successful*
reply: a page of results served to a request that named nothing.

Today every handler on the surface has at least one required field, so the
defect is latent rather than reachable — `{"stoa":…}` is mandatory everywhere,
and its absence is what refuses the array. That is exactly the shape of a defect
worth closing before it is reachable: the *first* method whose fields are all
optional makes it live, and nothing in the contract or the suite would say so.

**Why no test caught it.** Every hostile-input fixture in the repo is either
unparseable text (`"not json"`, which dies at the parse) or `{}` (an object
missing its fields). **Both are refused whether or not the handler checks that
the request is an object.** Two explanations, one answer — this project's
recorded defect family, and the reason the spec-driven flow exists.

The security posture is "never trust an inbound message; validate at the
boundary, before it reaches any state machine". The request envelope *is* that
boundary, and it currently has no stated shape.

## What Changes

- The wire contract gains the request half: a request MUST be a JSON **object**.
  A request that parses as any other JSON value — an array, a number, a string,
  a boolean, `null` — is the error shape, and is **not** read as an object whose
  fields are absent.
- The refusal is **distinguishable by message** from the two neighbouring
  refusals it is currently indistinguishable from: a request that is not valid
  JSON at all, and an object that is missing a required field. Three different
  caller mistakes, three different messages.
- The rule applies to **every method that reads a field of its request**,
  including ones whose fields are all optional. Those are precisely the methods
  where the check has no side effect visible to a correct caller, and precisely
  the methods where its absence is invisible. A method taking **no request at
  all** is outside it, there being nothing to check — `version` is the one on
  today's surface — and so is a method that takes a request and reads no field of
  it.
- **The scope is stated as the field read rather than the parameter**, because
  "every method that accepts a request" had two readings that disagreed about the
  panic probe — which takes a request string and never decodes it. The probe is
  outside the envelope rule, and the panic-guard requirement is amended to say so
  positively: the probe's request is opaque text, it reaches its panic for every
  request shape, and it refuses none. An excluded method with no rule of its own
  would be a gap rather than a decision.

  The requirement now enumerates all three cases — reads a field, takes no
  request, takes one and reads no field — rather than stating one and leaving the
  others to be inferred from the harm it names.
- **An explicit `null` as a field value is now specified, and not as a blanket
  equivalence.** A `null` field is present rather than absent; where a method
  reads it as absent, the default it then acts on MUST be the **restrictive** one.
  A blanket "null means absent" would be inherited by a future optional field
  whose default is permissive — a moderator's view, removed content, a policy
  bypass — and would let a caller reach the permissive branch by naming a field
  with no value. Where a field is required, a `null` is the wrong type rather than
  a missing field; where a field carries an arbitrary JSON value, the `null` is
  that value.
- **An empty object `{}` stays valid** for a method with no required fields. The
  contract refuses non-objects, not empty ones — stated explicitly so the new
  requirement cannot be read as forbidding the request shape a
  no-argument method is called with.
- **Unknown fields remain accepted, and that is now stated rather than
  implied.** See Impact for why it is deliberately not tightened here.

## Capabilities

**Modified Capabilities**

- `module-wire-contract` — **two** existing requirements are amended.

  1. **"Every method takes JSON and returns JSON"** gains the request half. It is
     the requirement that already words the envelope's two halves and already says
     of the reply that "anything that is not an object is outside this contract";
     the request half belongs beside it, in the same requirement, rather than in a
     second requirement asserting the mirror rule somewhere else in the same file.
  2. **"A panic in a handler becomes the error shape and the module keeps
     serving"** gains the panic probe's own contract: its request is opaque text,
     it reaches its panic for every request shape including a non-object, and it
     refuses none. This is what the envelope rule's one exclusion rests on, and
     stating it here rather than nowhere is what makes the exclusion a decision
     instead of a silence. It also closes a gap that predates this change — the
     requirement's "exercised rather than merely asserted" scenario depends on a
     probe whose contract was never written down, so nothing said the probe must
     panic on *whatever* it is given.

  Both headings are verbatim from `openspec/specs/module-wire-contract/spec.md`'s
  own `### Requirement:` lines, checked against the file, and each MODIFIED block
  carries every existing scenario of its requirement forward: a `MODIFIED` heading
  that matches nothing applies nothing, silently, and a `MODIFIED` block replaces
  the whole requirement including its scenarios.

**No new capability.** A standalone `wire-request-envelope` capability would have
had to restate this contract's error shape, its one-failure-shape rule and its
reply-is-an-object rule in order to say anything useful about a refused request —
and two capabilities asserting one rule is the duplication this project has
already paid for once.

## Impact

- `dialectica/rust-lib/dialectica-core/src/wire.rs` — each handler that parses a
  request gains one check between the parse and the first field read, and the
  hostile-input fixtures gain a non-object case. No reply shape changes for any
  request that was already being served.
- **One `NO SPEC:` marker is now specified and should be reworded, not deleted.**
  `panic_probe_still_panics_on_a_non_object_rather_than_refusing_it` carries a
  marker saying the spec "says nothing about a method that takes a request string
  it never decodes". It now does: the panic-guard requirement states the probe's
  request is opaque text and that it refuses no request for its shape. The test
  becomes a spec-pinning test rather than an unspecified default, so the marker
  goes and a reference to the requirement replaces it. The test's assertions do
  not change; only the comment above them does. **Code is not edited in this
  change** — reviewers are reading that file — and the replacement text is
  recorded in the report handed to whoever picks it up.
- **No behaviour change for any correct caller.** Every request the surface
  serves today is an object; this refuses inputs that are currently served only
  by accident, and on today's surface not even that.
- **The null-field rule is satisfied by every site on today's surface, which is
  why it is stated now rather than argued later.** Read from the code, not
  inferred:

  | site | field | `{"f":null}` today | under the rule |
  |---|---|---|---|
  | `parse_index` (`page`, `perPage`) | optional | absent → `0` | restrictive default; permitted |
  | `includeHidden` | optional | absent → `false` | restrictive default; permitted |
  | `genesis_for` (`genesis`) | required | `Err` wrong type | required-field reading; permitted |
  | `parse_channel_id` (`channelId`) | required | `Err` wrong type | required-field reading; permitted |
  | `ping` (`payload`) | required | `{"pong":null}` | a field carrying any JSON value; permitted |

  So the rule is a **pin on existing behaviour**, not a request for a change. It
  was worth writing because the pattern is currently visible only by reading five
  call sites, and the one shape it forbids — `null` as absent where the default is
  permissive — has no instance to argue against it today and would be written the
  wrong way by a contributor generalising from `includeHidden`.

  `ping`'s `payload` is the case that looks like a third reading and is not: the
  field is documented as carrying any value, so `null` is a value rather than an
  omission. The rule says a method must not hold both readings for one field,
  which is the property that keeps that case from spreading.

  The **code** consistency work — making the readers state which of the three
  cases each is, and recording it in `design.md` — belongs to the `dev-writer` on
  `fix/wire-newtype`. This change states the contract only, and edits no code.
- **Left unspecified on purpose: that `Request::parse`'s failure is already the
  serialised wire shape.** The test at `wire.rs` asserting it is over-pinning
  rather than a gap — `Request::parse` is internal, its `Err` being returnable
  verbatim is a convenience handlers exploit, and no caller outside the crate can
  observe the difference. A spec is a contract on observable behaviour, so
  promoting an internal constructor's error representation into one would freeze a
  convenience as an obligation. Noted for the tester rather than specified.
- **Out of scope: unknown fields.** A request that is an object but carries a
  field no method reads is a different question — it is about forward
  compatibility and typo detection, not about the envelope's type, and the two
  answers pull opposite ways (strict rejection catches a caller's typo; lenient
  acceptance lets a newer view talk to an older core). Deciding it here would
  bundle a compatibility policy into a defect fix. The contract states that such
  a request is accepted, which is what the code does today, and leaves tightening
  to a change that argues for it.
- **Out of scope: the reply half**, which this contract already states, and the
  cross-module *reply* decoder, which is `module-wire-contract`'s "A reply from
  another module is decoded, never guessed at" and already requires an object
  before recognising a failure envelope. Nothing about inbound requests changes
  it.
- **Noted, not fixed here:** `openspec/specs/module-wire-contract/spec.md` has no
  `## Purpose` section, so `openspec show module-wire-contract --type spec` fails
  with "Spec must have a Purpose section" and `openspec list --specs` reports it
  as having 0 requirements. `identity` and `stoa-metadata` are in the same state.
  A delta cannot supply a Purpose for an existing capability — it is edited in the
  live spec — and doing it inside a defect fix would put an unreviewable prose
  edit in the same diff. It is recorded here so the next reader finds it stated.
