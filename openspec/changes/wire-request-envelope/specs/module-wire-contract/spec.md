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

**The scope is the field read, not the parameter.** Reading one field is enough to
be inside this rule, and requiring none is not enough to be outside it. A method
is outside it only by treating its request as **opaque text** — never reading a
field, so there is no field whose absence could be misread — and the surface
carries exactly one such method: the panic probe, whose contract under "A panic in
a handler becomes the error shape and the module keeps serving" is to reach its
panic on whatever it is given. A request the probe cannot refuse is that
requirement's obligation rather than a hole in this one, and any other method
claiming the exception SHALL be one that reads no field either.

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
