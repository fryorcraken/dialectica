# Spec/test findings — `wire-request-envelope`

From `spec-test-reviewer`, which reads **the spec and the tests only** and is
blind to the implementation. **Reconstructed from the commit record** — see the
note at the top of `correctness.md` for why that is itself a finding about how
this piece was run.

Entries T1–T5 are closed. **F1–F4 are deferred to the `spec-writer` and are the
reason this file must be read before the change is archived** — each is behaviour
that was *chosen* rather than specified, and the marker in the code is what makes
it findable.

---

## T1. The three-refusals test could not see a prefix collision — **Fixed**

For: `tester`

`the_three_refusals_a_caller_can_earn_are_three_different_messages` compared whole
strings with `assert_ne!`. So rewording `REQUEST_NOT_AN_OBJECT` to
`"invalid JSON: the request must be a JSON object"` left it **green** — while
telling a caller exactly what the spec says it must not be told, that its array
failed to parse. Same with a `"missing field: "` prefix.

Only the literal pin went red, and a pin fails for "the string changed", which is
not the reason the requirement names.

**Fixed** in `81e1e27`.
`the_non_object_message_does_not_read_as_either_refusal_it_must_be_told_from`
asserts the property directly, against prefixes written out in the test rather
than read from the code, **plus** a check that each prefix is still what the
neighbouring refusal actually says — so the guard cannot quietly become a guard
against phrases nothing uses.

## T2. `Request::get` dropping explicit nulls passed 486 of 487 tests — **Fixed**

For: `tester`

`self.0.get(field).filter(|v| !v.is_null())` is a plausible "helpful" spelling.
The two readers the sweeps covered (`parse_index`, `includeHidden`) treat `None`
and `Some(Null)` alike, so nothing saw it. `get` is public and re-exported.

**Fixed** in `81e1e27` and `66de130`.
`a_parsed_request_hands_back_the_fields_it_was_given_and_only_those` closes the
envelope-level half — and also catches `parse` accepting an object and handing on
an **empty** map, which is the other way "read as an object in which every field
is absent" comes back and the one no handler test can attribute to the envelope.
The handler-level half is four fixtures, one per differing reader; see C3 in
`correctness.md` for the table.

## T3. The unknown-field fixture was built by string-splicing — **Fixed**

For: `tester`

And the fragility was real rather than theoretical. Respelling
`a_served_request("ping")` from `{"payload":1}` to the equally valid
`{"payload":{"n":1}}` made `trim_end_matches('}')` strip **both** braces; the test
then failed with `invalid JSON: EOF while parsing an object`, reporting a refusal
of the extra field that never happened.

**Fixed** in `81e1e27`. Rebuilt through a `serde_json::Map` insert, which cannot
emit malformed JSON, plus a round-trip check that every original field survived —
so a broken construction fires its own assertion instead of masquerading as a
refusal. The respelling now passes.

## T4. The envelope scope had two readings that disagreed about one method — **Fixed (spec)**

For: `spec-writer`

"Every method that accepts a request" put `panic_probe` inside the rule under
"takes a request-shaped parameter" and outside it under "reads a field from it".
The dev-writer chose the second and left a `NO SPEC:` marker; the tester declined
to write a test either way and escalated. Both readings were testable, which is
what made this a spec question rather than a coding one.

**Fixed** in `23c1516` and `2d8cc3f`. The spec now scopes the rule **by the field
read**, and enumerates all three cases rather than implying them:

| case | example | inside the rule? |
|---|---|---|
| reads a field | `ping`, `get_capabilities`, `list_threads`, … | yes |
| takes no request | `version` | no |
| takes a request, reads no field | `panic_probe` | no, with its own contract |

The exclusion is **governed rather than silent**: the panic-guard requirement
gained the probe's own contract — its request is opaque text, it reaches its panic
for every request shape including a non-object and including text that is not
JSON, it refuses none, and its message carries what it was given. That closes a
gap older than this change.

Consequence for the tests: the `NO SPEC:` marker on
`panic_probe_still_panics_on_a_non_object_rather_than_refusing_it` is **gone
rather than reworded around**, because the test now cites a requirement instead of
a chosen default.

## T5. `Request::parse`'s `Err` being the wire shape — **Rejected (over-pinning)**

For: `tester`

Proposed: specify that `parse`'s `Err` arm is already the serialised wire reply.

**Rejected, with the argument.** `parse` is internal and no caller outside the
crate can observe it, so a test pinning that over-pins rather than covering a
gap — and promoting a convenience into a contract would freeze it. Recorded in
the proposal's Impact so the next reader finds the decision rather than the
absence.

---

# Deferred to the `spec-writer`

These four are **not defects and must not be "fixed" in code.** Each is a
contract question an envelope change should not settle, marked in the code so it
is findable. What a spec decision would need to say is stated for each.

## F1. No size limit exists anywhere in the spec set — **Deferred**

For: `spec-writer`

`MAX_REQUEST_BYTES = 4 MiB` is the dev-writer's number, carried by
`NO SPEC:` on `every_request_taking_method_refuses_an_oversized_request`.

**This is an *absent* decision, not a rejected one.** The spec set bounds no
request length at all — it does not say requests are unbounded, and it does not
say who chooses. That distinction is the whole reason this is deferred rather
than closed: a reader of the archive must not conclude that unboundedness was
considered and accepted.

The measurement forcing *some* bound is in `security.md` S1 and is not in
question. What a spec decision would need to say:

- **Whether a request length bound is part of the wire contract at all**, or an
  implementation matter a view may not rely on. This decides whether a view can
  be written to pre-check its own request size, or must handle the refusal
  reactively.
- **If it is contract: the number, and whether it is one number or per-method.**
  The code chose one number deliberately (a per-method cap is a second thing each
  new handler must declare, which is the guard-at-every-call-site shape the piece
  exists to avoid), but that is a design argument, not a contract statement.
- **Whether the refusal message is contract surface**, as the three existing
  refusals now are. The current message names the actual and permitted sizes
  (`"the request is N bytes, over the M byte limit"`), which leaks the cap to any
  caller — fine, possibly desirable, but chosen rather than specified.
- **Whether the bound is on the request string's bytes or on some decoded
  measure.** The code bounds the raw `&str` length, which is what makes the check
  cheap enough to run before parsing.

## F2. The genesis-hex bound — **Deferred**

For: `spec-writer`

`genesis_for` bounds its hex input by `stoa::MAX_CANONICAL_BYTES`, carrying a
`NO SPEC:` marker. Same class as F1: the security need is established
(`security.md` S2 — it allocated half an attacker-chosen length before checking),
but no spec requirement says a genesis record arriving over the wire has a maximum
size.

What a spec decision would need to say:

- **Whether the wire contract restates the bound or defers to the `stoa`
  capability's own canonical-size requirement.** Deferring is probably right and
  is what the code does in spirit — the constant is exported from `stoa` rather
  than copied into `wire.rs`, precisely so it cannot drift — but "the wire
  contract inherits the record format's bound" is a sentence no spec currently
  contains.
- **Which refusal a caller earns**, and whether it is distinguishable from the
  request-size refusal in F1. There are now potentially two size refusals on one
  path.

## F3. `parse_index` accepts by spelling, not by value — **Deferred**

For: `spec-writer`

`{"page":1e2}` is JSON for exactly 100 and is **refused**. So is `{"page":0.0}`
for exactly 0. `as_u64` refuses any number serde parsed as `f64` — anything
carrying a `.` or an `e`.

**The message was fixed; the acceptance was deliberately left alone.** See C2 in
`correctness.md`. The false message was a defect on any reading. Whether the
*value* 100 spelled `1e2` is a valid page index is a contract question, and
several JSON serialisers emit that spelling, so the population affected is real
callers rather than hostile ones.

**Do not change the acceptance to close this finding.** It is filed, not guessed
at, and a caller told the truth can restring its number today.

What a spec decision would need to say:

- **Whether an index field's contract is about the value or the JSON spelling.**
  If the value: `1e2`, `1.0e2` and `100.0` must all be accepted as 100, and a
  non-integral float like `1.5` must still be refused — which is a different
  check from the one now written, not a loosened one.
- **What `-0.0` does**, and `1e400` (which serde parses as a float, not an
  integer), since a value-based rule has to answer both.
- **Whether the refusal message names the spelling or the value.** The current
  message names the spelling because that is what the code refuses; a value-based
  contract would make that message wrong again in the other direction.

`the_index_refusal_says_what_it_actually_refuses` pins the current behaviour for
all four spellings that earn it, and asserts `100` is still served — so whichever
way the spec goes, the test that has to change is identifiable.

## F4. Unknown fields are accepted, and the cap is what pays for it — **Deferred (stated, but coupled)**

For: `spec-writer`

The spec **does** put unknown-field strictness out of scope and states such
requests are accepted, so this is not an unspecified behaviour. It is listed here
because the spec states the leniency without stating what makes it affordable.

Per `design.md` §6 and `security.md` S1, ignoring unknown fields is exactly what
made the 64 MiB padded request *valid* rather than refused — every byte sat in a
field no method reads. The two are one decision, and the spec currently carries
one half of it.

What a spec decision would need to say: **if F1 resolves against having a
contractual size bound, the leniency needs re-pricing in the same breath.** A
spec that grants unbounded unknown fields and no length bound has specified the
64 MiB request as valid.
