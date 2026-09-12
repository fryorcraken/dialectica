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

Widening this surface SHALL be a deliberate act. The contract outlives any
particular view, and it is what keeps dependency churn behind a wall.

#### Scenario: Every handler answers with a JSON object

- **WHEN** any method is called, with a well-formed or a malformed request
- **THEN** the reply parses as JSON
- **AND** the reply is a JSON object

#### Scenario: A method with no input still answers in JSON

- **WHEN** a method that takes no request is called
- **THEN** the reply is a JSON object like any other

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
