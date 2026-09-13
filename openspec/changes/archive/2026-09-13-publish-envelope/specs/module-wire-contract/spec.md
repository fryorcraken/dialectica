## ADDED Requirements

### Requirement: A request is bounded, and the bound is checked before the request is parsed

Every method that reads a field of its request SHALL refuse a request larger than
a single stated limit, and SHALL refuse it **before** the request is parsed. The
refusal SHALL be the error shape, and its message SHALL say that the request is
over that limit — distinguishable from the three refusals it otherwise resembles:
a request that is not valid JSON, a request that is not an object, and an object
omitting a required field.

**The ordering is the requirement, not an optimisation of it.** Parsing a request
allocates on the order of twice its length before any check can run, so a limit
enforced after the parse bounds nothing: the cost the limit exists to refuse has
already been paid by the time it is consulted. The observable consequence of
failing to pay it is not an error reply — it is the module process aborting, the
caller waiting out its call timeout, and every later call to the module reporting
it as not loaded. That is why this is stated as a contract obligation rather than
left to an implementation to size.

**It is one limit for the surface, not a limit per method.** A per-method limit is
a second thing every new handler must declare, and the shape of that is a guard
whose presence at each call site has to be re-established. The tighter bounds that
matter are per *field*, and those belong to the field's own capability rather than
to the envelope.

**The limit's value is not fixed by this contract**, which would put a number in a
document no gate reads. What this contract fixes is that a limit exists, that it
is one number, that it is checked first, and that it is bounded from both sides:
it SHALL admit the largest request this surface can legitimately carry — a
composed op with its attachments, whose size `op-format`'s field bounds already
determine — and SHALL be materially below the size at which a request costs the
module its process. An implementation SHALL record where its number sits between
those two bounds, so that raising or lowering it is visibly a change to both.

**The limit is what buys the surface's leniency about unknown fields.** This
contract accepts a request carrying a field no method reads, which is a
forward-compatibility choice stated elsewhere in this capability. That leniency is
also what lets an oversized request be *valid*: every byte of the padding sits in
a field nothing reads. The two decisions are therefore one, and weakening the
limit weakens the leniency's price rather than only the limit.

#### Scenario: A request over the limit is refused

- **WHEN** a method that reads a field of its request is called with a request
  larger than the limit
- **THEN** the reply carries an error saying the request is over the limit
- **AND** the reply carries no result field

#### Scenario: Every field-reading method on the surface is bounded

- **WHEN** each method that reads a field of its request is called in turn with a
  request larger than the limit
- **THEN** every one of them refuses it for its size
- **AND** no method is exempted by which fields it happens to require

#### Scenario: The bound is checked before the parse

- **WHEN** a method is called with a request that is both over the limit and not
  valid JSON
- **THEN** the reply says the request is over the limit, rather than that it
  failed to parse
- **AND** the message is therefore the one refusal that could only have been
  reached without parsing

#### Scenario: A request within the limit is still served

- **WHEN** a method is called with a well-formed request far below the limit
- **THEN** the reply carries no error, so the bound refuses an oversized request
  rather than any request

#### Scenario: The limit admits the largest legitimate request

- **WHEN** the limit is compared against the size of a composed op carrying the
  largest payload `op-format`'s field bounds permit, encoded as a request
- **THEN** the limit exceeds it, so no request this surface can legitimately
  carry is refused for its size

## MODIFIED Requirements

### Requirement: Every method takes JSON and returns JSON

Every method on the module's surface SHALL take a JSON string and return a JSON
string, and every reply SHALL be a JSON object.

A method answering a question with no input SHALL still return JSON, so that a
view never branches on reply shape. A reply that is a bare scalar, an array, or
anything that is not an object is outside this contract.

**A request SHALL be a JSON object.** Every method that **reads a field of its
request** SHALL refuse a request that parses as any other JSON value: an array, a
number, a string, a boolean, or `null`. The refusal SHALL be the error shape.
Such a request SHALL NOT be read as an object in which every field is
absent, which is the one reading that turns a refusal into a served reply:
asking a non-object for a field yields the same answer as asking an object that
lacks it, so a method whose fields are all optional would otherwise answer a
request that named nothing.

This SHALL hold for **every** such method, whatever fields that method requires.
A method with a required field refuses a non-object today only as a side effect
of that field being absent from it, which is not the same rule and does not
survive the field becoming optional; a method whose fields are all optional has
no such side effect and serves the request. The rule is therefore stated once,
for the envelope, rather than left to each method's fields to imply.

**The scope is the field read, not the parameter**, and the three cases are stated
rather than left to be inferred:

- A method that **reads a field** of its request is inside this rule. Reading one
  field is enough; how many of its fields are required changes nothing, and a
  method requiring none is inside it.
- A method that **takes no request at all** is outside it, there being nothing to
  check. Such a method still returns JSON, which "A method with no input still
  answers in JSON" requires of it.
- A method that **takes a request and reads no field of it** — passing it through
  as opaque text — is outside it, because the harm this rule prevents is a field's
  absence being misread, and a method that reads no field has no such absence to
  misread.

The surface carries exactly one method in that third case: the panic probe, whose
contract under "A panic in a handler becomes the error shape and the module keeps
serving" is to reach its panic on whatever it is given. A request the probe cannot
refuse is that requirement's obligation rather than a hole in this one, and any
other method claiming this exception SHALL be one that reads no field either.

**What "the surface" is SHALL be derivable from one declaration**, so that the
question "does this rule reach every method?" has an answer something can check
rather than an answer someone re-establishes by reading. Every method reachable by
a caller SHALL be one this contract's obligations reach, and no method SHALL be on
the wire without appearing in the declaration the surface is derived from.

**That derivation rests on an upstream behaviour, which is stated here because it
is a premise rather than a property of this module.** The declaration a module
author writes also carries plumbing that is not contract surface, and the
distinction the generator draws between the two is: a method with a **default
body** is framework plumbing and is not emitted onto the wire, while a method
without one is. Verified at the pinned generator revision — `lidl-gen`'s Rust
frontend skips a trait method whose default body is present, and says so in its
module doc: *"required methods (no default body) are the module's IPC methods.
Methods WITH default bodies (e.g. the framework's `on_context_ready`) are not part
of the contract."* Anything checking this contract's coverage of the surface
SHALL be able to state that assumption and SHALL fail visibly rather than silently
if a generator that no longer honours it puts a defaulted method on the wire. The
assumption is recorded with its citation rather than asserted, because the
generator is a pinned dependency: a version bump is exactly the event that would
make it false, and a bare assertion of the behaviour would survive that bump
unchanged while a citation to a revision does not.

The refusal's message SHALL say that the request is not an object, so that it is
distinguishable from the two refusals it otherwise resembles: a request that is
not valid JSON at all, and an object that omits a field the method requires.
Three caller mistakes, three messages. This is the same obligation
"Failure is always the error shape, and never a partial success" places on a
wrong-typed field against a missing one, applied one level out — to the envelope
rather than to a field within it.

An **empty object** SHALL satisfy this requirement. `{}` is an object, so it
reaches the method's own field checks rather than being refused by the envelope
check; whether it is then served depends only on whether that method requires a
field. The contract refuses a request of the wrong *type*, never one that is
merely empty. Where a method does require a field, `{}` SHALL be refused for that
missing field and SHALL NOT be refused for the envelope's shape.

A request that is an object carrying a field no method reads SHALL be accepted.
Whether an unrecognised field is a caller's typo or a newer view talking to an
older core is a compatibility question this contract does not settle, and
refusing one here would answer it by accident.

**A field holding an explicit `null` is present, not absent.** This is a different
question from the envelope's outer type and is settled separately: `null` at the
top level is a request that named nothing and is refused, while `null` as a field
value is a caller who named the field and supplied no value for it. There is no
blanket equivalence between such a field and an omitted one.

What happens to it SHALL follow from the field's **declared type and optionality**,
so that it is decided once where the field is specified rather than per method or
per call site. Every field the surface reads SHALL have exactly one of these three
readings, and a field SHALL NOT have two:

1. A field whose declared type **admits `null` as a value** — one documented as
   carrying any JSON value — SHALL carry the `null` through as that value. This
   reading takes precedence over the other two, because for such a field a `null`
   is not a malformed parameter but the parameter.
2. An **optional** field SHALL treat a `null` as absent, and SHALL then act on its
   **restrictive** default — the value that grants the caller no more than omitting
   the field would.
3. A **required** field whose type does not admit `null` SHALL refuse a `null` as
   the wrong type rather than report it as missing, because the caller did name
   it. That is the distinction "Failure is always the error shape, and never a
   partial success" already requires between a wrong-typed field and an absent
   one.

**Reading 2 is licensed only in the restrictive direction, and that limit is the
point of stating it.** A field SHALL NOT read `null` as absent where the resulting
default is the permissive choice; such a field SHALL refuse a `null` under reading
3 instead. A blanket "null means absent" would lose the direction and be inherited
by a future optional field whose default widens what a caller may see or do —
hidden content, removed content, a moderator's view, a policy bypass — letting a
caller reach the permissive branch by naming a field with no value. Refused as a
wrong type, that same request gets nothing. So the equivalence holds only where it
cannot become an authorisation bypass.

This settles the request half only. The reply half's rule — a field that would be
meaningless is **omitted** rather than sent as `null`, because a reply carrying a
meaningless field is the partly-successful shape this contract forbids — is a
separate obligation in the opposite direction, and nothing here relaxes it.

Widening this surface SHALL be a deliberate act. The contract outlives any
particular view, and it is what keeps dependency churn behind a wall.

#### Scenario: Every handler answers with a JSON object

- **WHEN** any method is called, with a well-formed or a malformed request
- **THEN** the reply parses as JSON
- **AND** the reply is a JSON object

#### Scenario: A method with no input still answers in JSON

- **WHEN** a method that takes no request is called
- **THEN** the reply is a JSON object like any other

#### Scenario: A request that is an array is refused

- **WHEN** a method that reads a field of its request is called with a JSON array
- **THEN** the reply carries an error
- **AND** the reply carries no result field

#### Scenario: A request that is a scalar is refused

- **WHEN** a method that reads a field of its request is called with a JSON
  number, string, boolean or `null`
- **THEN** the reply carries an error
- **AND** the reply carries no result field

#### Scenario: Every field-reading method on the surface refuses a non-object

- **WHEN** each method that reads a field of its request is called in turn with a
  JSON array
- **THEN** every one of them refuses it with the same message
- **AND** no method is exempted by which fields it happens to require

#### Scenario: The set of methods checked is derived from the surface declaration

- **WHEN** a method is added to the declaration the module's surface is derived
  from, and the checks this contract's obligations are applied through are not
  extended to it
- **THEN** that omission is reported, naming the method
- **AND** a declaration written in a shape the derivation does not recognise is
  reported too, rather than passing as though it carried no method

#### Scenario: A method excluded from the surface is one the generator does not emit

- **WHEN** the declaration carries a method with a default body, which the pinned
  generator does not put on the wire
- **THEN** this contract's obligations do not reach it, no caller can reach it,
  and the exclusion is stated with its citation rather than assumed

#### Scenario: The non-object refusal is not the missing-field refusal

- **WHEN** a method is called with a JSON array, and then with an object omitting
  a field it requires
- **THEN** both replies carry an error
- **AND** the array's message says the request is not an object, which is not
  what the omitting object's message says

#### Scenario: The non-object refusal is not the unparseable-request refusal

- **WHEN** a method is called with a JSON array, and then with text that is not
  valid JSON
- **THEN** both replies carry an error
- **AND** the two messages differ, so a caller is not told its array failed to
  parse

#### Scenario: An empty object is refused for its missing field, not its shape

- **WHEN** a method that requires a field is called with `{}`
- **THEN** the reply carries an error naming that missing field
- **AND** the message does not say the request is not an object

#### Scenario: An object carrying every required field is served

- **WHEN** a method is called with an object supplying the fields it requires and
  omitting every optional one
- **THEN** the reply carries no error, so the envelope check refuses a wrong type
  rather than an absent optional field

#### Scenario: An unrecognised field does not refuse the request

- **WHEN** a method is called with an object carrying every field it requires
  plus one it does not read
- **THEN** the reply carries no error

#### Scenario: The one method outside this rule is the panic probe

- **WHEN** the panic probe is called with a JSON array
- **THEN** the reply is the panic guard's error naming the probe
- **AND** the message is not the not-an-object refusal, so the probe reached its
  panic rather than being refused before it
- **AND** every other method on the surface refuses that same array

#### Scenario: An explicit null is present rather than absent

- **WHEN** a request supplies a field holding `null`
- **THEN** the method distinguishes it from a request omitting that field
  entirely, rather than treating the two as the same request

#### Scenario: A null optional field defaults to the restrictive value

- **WHEN** an optional field is supplied as `null` — the flag that includes hidden
  content, and the page index
- **THEN** the reply is the one the caller would have received by omitting the
  field
- **AND** it is the restrictive reply: hidden content stays excluded, so no caller
  reaches a wider answer by naming a field without a value

#### Scenario: A null required field is refused as a wrong type

- **WHEN** a required field whose type does not admit `null` is supplied as `null`
- **THEN** the reply carries an error saying that field is the wrong type
- **AND** the message does not say the field is missing, which the caller would
  read as a request it did not make

#### Scenario: A field that carries an arbitrary value carries a null too

- **WHEN** a field documented as carrying any JSON value is supplied as `null`
- **THEN** that `null` is carried through as the value it is, rather than refused
  or defaulted
- **AND** this holds even though the field is required, the value-carrying reading
  taking precedence over the wrong-type one

#### Scenario: One field has one null reading

- **WHEN** each field the surface reads is supplied as `null` in turn
- **THEN** each produces exactly one of the three outcomes — carried as a value,
  defaulted restrictively, or refused as a wrong type — and never two for one
  field
- **AND** which outcome a field produces is predictable from its declared type and
  whether it is required, rather than from which method reads it
