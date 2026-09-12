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
- The rule applies to **every** method, including ones that take no parameters
  and ones whose fields are all optional. Those are precisely the methods where
  the check has no side effect visible to a correct caller, and precisely the
  methods where its absence is invisible.
- **An empty object `{}` stays valid** for a method with no required fields. The
  contract refuses non-objects, not empty ones — stated explicitly so the new
  requirement cannot be read as forbidding the request shape a
  no-argument method is called with.
- **Unknown fields remain accepted, and that is now stated rather than
  implied.** See Impact for why it is deliberately not tightened here.

## Capabilities

**Modified Capabilities**

- `module-wire-contract` — one existing requirement, **"Every method takes JSON
  and returns JSON"**, is amended. It is the requirement that already words the
  envelope's two halves and already says of the reply that "anything that is not
  an object is outside this contract"; the request half belongs beside it, in the
  same requirement, rather than in a second requirement asserting the mirror rule
  somewhere else in the same file. Its existing scenarios are carried unchanged
  and new ones added.

  The heading is verbatim from `openspec/specs/module-wire-contract/spec.md`'s
  own `### Requirement:` line, checked against the file: a `MODIFIED` heading
  that matches nothing applies nothing, silently.

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
- **No behaviour change for any correct caller.** Every request the surface
  serves today is an object; this refuses inputs that are currently served only
  by accident, and on today's surface not even that.
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
