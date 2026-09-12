## MODIFIED Requirements

### Requirement: Every method takes JSON and returns JSON

Every method on the module's surface SHALL take a JSON string and return a JSON
string, and every reply SHALL be a JSON object.

A method answering a question with no input SHALL still return JSON, so that a
view never branches on reply shape. A reply that is a bare scalar, an array, or
anything that is not an object is outside this contract.

**A request SHALL be a JSON object.** Every method that accepts a request SHALL
refuse one that parses as any other JSON value — an array, a number, a string, a
boolean, or `null` — with the error shape. Such a request SHALL NOT be read as an
object in which every field is absent, which is the one reading that turns a
refusal into a served reply: asking a non-object for a field yields the same
answer as asking an object that lacks it, so a method whose fields are all
optional would otherwise answer a request that named nothing.

This SHALL hold for **every** method that accepts a request, whatever that method
requires. A method with a required field refuses a non-object today only as a
side effect of that field being absent from it, which is not the same rule and
does not survive the field becoming optional; a method whose fields are all
optional has no such side effect and serves the request. The rule is therefore
stated once, for the envelope, rather than left to each method's fields to imply.

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

- **WHEN** a method that accepts a request is called with a JSON array
- **THEN** the reply carries an error
- **AND** the reply carries no result field

#### Scenario: A request that is a scalar is refused

- **WHEN** a method that accepts a request is called with a JSON number, string,
  boolean or `null`
- **THEN** the reply carries an error
- **AND** the reply carries no result field

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
