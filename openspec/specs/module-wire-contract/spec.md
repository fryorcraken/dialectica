# module-wire-contract Specification

## Purpose

Defines the shape of every call across the module's surface — what a reply looks like, what a failure looks like, and what happens to a handler that panics or to a reply from another module — so that a view has exactly one error branch and no failure on this path can take the module down or be read as a value.

## Requirements

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

### Requirement: Failure is always the error shape, and never a partial success

Every failure SHALL be reported as an object carrying an `error` field holding a
human-readable message. A reply SHALL NOT carry both an error and a result.

This is the single convention the whole error-handling model rests on: a view
has exactly one error branch precisely because there is exactly one failure
shape. A partial success — an error field alongside a result field, or a result
field holding a value invented to stand in for a failure — puts the view in the
position of deciding which half to believe.

The message SHALL survive being embedded in the reply whatever it contains.
Messages are built from attacker-influenced material the moment a handler
formats a request into one, so a reply whose message contains a quote or a
newline must still be valid JSON rather than a parse failure at the view.

#### Scenario: A malformed request is the error shape

- **WHEN** a method is called with a request that is not valid JSON
- **THEN** the reply carries an error

#### Scenario: A missing field is refused rather than defaulted

- **WHEN** a request omits a field the method requires
- **THEN** the reply carries an error
- **AND** the reply carries no result field

#### Scenario: A field of the wrong type is distinguishable from a missing one

- **WHEN** a request carries a required field holding the wrong type
- **THEN** the reply's message says the field is the wrong type, not that it is
  missing

#### Scenario: An error message containing JSON metacharacters stays valid JSON

- **WHEN** a failure's message contains quotes, backslashes or newlines
- **THEN** the reply still parses as JSON

### Requirement: A panic in a handler becomes the error shape and the module keeps serving

No handler SHALL be able to unwind out of the module's surface. A panic in any
handler SHALL be converted into the error shape, and the module SHALL continue
to serve subsequent calls.

**This is load-bearing rather than hardening, and the cost was measured.** The
generated dispatch calls straight into handler code across an `extern "C"`
frame, and the SDK catches nothing. An unguarded panic does not fail the call —
the module process aborts, the caller waits out a twenty-second timeout and is
then told its call timed out, every later call reports the module as not loaded,
and nothing restarts it. The word "panic" appears only in a daemon log.

The guard SHALL therefore cover **every** handler, including ones that look
incapable of panicking and including cross-module calls. The damage is to the
process rather than to the call, so there is no handler too trivial to guard and
no subset where it matters less.

The converted error SHALL carry the panic's own message and SHALL name the
handler that panicked. With one failure shape shared by every method, the
handler name is the only thing distinguishing which one died.

The surface SHALL carry a **panic probe** — a method that panics on purpose —
because a guard nothing exercises is a claim no gate can see. The probe SHALL
treat its request as **opaque text**: it SHALL reach its panic for every request
it is given, including one that is not a JSON object and one that is not valid
JSON at all, and SHALL NOT refuse a request for its shape. Its message SHALL
carry the request it was given, which is the observable difference between
passing the text through and decoding it.

That is the whole of the probe's contract, and it is what places the probe outside
the request-envelope rule stated under "Every method takes JSON and returns JSON".
A probe that can refuse a request is a probe there are requests the guard is not
exercised against — so the two rules cannot both reach it, and this one wins.

The probe is apparatus rather than forum surface, and removing it is a change to
this requirement rather than to the code alone: whatever removes it SHALL put
another method in its place that exercises the guard, or this requirement loses
the only thing that proves it.

#### Scenario: A panicking handler answers with the error shape

- **WHEN** a handler panics
- **THEN** the reply carries an error rather than unwinding

#### Scenario: The panic's message reaches the caller

- **WHEN** a handler panics with a message
- **THEN** that message appears in the reply
- **AND** this holds whether the message was a literal or was formatted

#### Scenario: The reply names the handler that panicked

- **WHEN** a handler panics
- **THEN** the reply names that handler

#### Scenario: A successful reply passes through the guard untouched

- **WHEN** a handler returns normally
- **THEN** the guard returns that reply unchanged

#### Scenario: The guard is exercised rather than merely asserted

- **WHEN** the module is asked to panic on purpose
- **THEN** the reply is the error shape
- **AND** the module answers subsequent calls

#### Scenario: The probe panics on every request shape it is given

- **WHEN** the panic probe is called with an object, with an array, with a scalar,
  and with text that is not valid JSON
- **THEN** every reply is the panic guard's error naming the probe
- **AND** no reply is a refusal of the request's shape or a parse complaint

#### Scenario: The probe's message carries the request it was given

- **WHEN** the panic probe is called with a request that is not a JSON object
- **THEN** that request's text appears in the reply's message, which is the
  observable difference between passing the text through and decoding it

### Requirement: A reply from another module is decoded, never guessed at

A reply from a module this one calls SHALL be recognised as that module's own
failure before it is interpreted as a value, and that module's message SHALL
reach the caller unchanged rather than quoted inside a complaint about our
failure to parse it.

A callee's happy path and its failure path are separate contracts, and only the
happy one is visible in a generated signature: a module declining a call answers
successfully at the language level with a failure envelope in the body. A
decoder that knows only the success shapes reads that as junk.

A value that is not recognised SHALL be an error. It SHALL NOT be mapped onto
any recognised value — coercing an unrecognised reply to a negative answer
reports something the callee never said, which is the partial-success shape this
contract forbids, with the failure disguised as a successful negative.

Recognising a failure envelope SHALL key off the error field alone rather than a
callee's additional fields, because a callee spelling its envelope with only
that field is the expensive direction to miss. Recognition SHALL require the
reply to be an object and the message to be a string, so that a legitimate value
is not misread as a failure.

#### Scenario: A callee's own failure reaches the caller unchanged

- **WHEN** a called module answers with a failure envelope
- **THEN** the reply carries that module's message verbatim
- **AND** the reply carries no result field

#### Scenario: A failure envelope is recognised without its callee's extra fields

- **WHEN** a reply is an object carrying an error message and nothing else
- **THEN** it is recognised as a failure

#### Scenario: A legitimate value is not misread as a failure

- **WHEN** a reply is not an object, or carries an error field that is not a
  string
- **THEN** it is not recognised as a failure

#### Scenario: An unrecognised value is an error rather than a default

- **WHEN** a called module answers with a value this contract does not recognise
- **THEN** the reply carries an error
- **AND** the reply carries no result field

#### Scenario: Both spellings of a boolean answer are accepted

- **WHEN** a called module answers a boolean question with either a JSON boolean
  or its string spelling
- **THEN** both are normalised to the same boolean result
