## ADDED Requirements

### Requirement: An item's author key travels under the JSON key `author`

Each item MUST carry the public key that *An author is reported as a public key, and never as a name* requires, under the JSON key `author`, as 64 lowercase hexadecimal characters.

An item MUST NOT carry the key `authorKey`, or any other key holding the author's public key a second time.

#### Scenario: The author key is under `author`

- **WHEN** a thread whose root is signed by one key and whose reply is signed by another is read
- **THEN** each item's `author` is the lowercase hex of the public key that signed that item's op

#### Scenario: No item carries `authorKey`

- **WHEN** a thread is read and each item's keys are enumerated
- **THEN** no item carries a key named `authorKey`
- **AND** no key other than `author` holds the hex of the key that signed the item's op
