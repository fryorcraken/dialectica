## MODIFIED Requirements

### Requirement: A Stoa address's display form parses strictly

A Stoa address MUST have a display form a user can copy: its bytes as
lowercase hexadecimal, two characters per byte. Parsing MUST accept only text of
exactly that length, every character of which is a hexadecimal digit, with a
failure that distinguishes a wrong alphabet from a wrong length.

**Letter case is not part of what parsing checks.** The hexadecimal digits
include the letters `a` to `f` in either case, so parsing MUST accept text whose
letters are uppercase, or any mix of cases, as naming the same address the
display form names. Any character that is not a hexadecimal digit in either case
is outside the alphabet.

**Wherever the module reports a Stoa address, it MUST report the display form**,
whatever case the address arrived in. An address taken in with uppercase letters
and reported back is reported in lowercase.

This parser meets attacker-supplied content, because a Stoa address may appear
inside a post. A lenient parser that accepted a truncated or over-long address
would let two different Stoas collide in what a reader sees, and would let a
moderation action name something other than what it appears to name.

**The requirement is stated over a Stoa address because that is the only kind
there is.** It previously said "an address" while the same type served authors
too; naming the Stoa explicitly is what stops a later reader taking the narrower
scope for a case the requirement lost.

#### Scenario: An address survives its display form

- **WHEN** a Stoa address is rendered and parsed back
- **THEN** the result equals the original

#### Scenario: A wrong alphabet is refused

- **WHEN** text outside the display form's alphabet is parsed as a Stoa address
- **THEN** parsing fails reporting the alphabet, not the length

#### Scenario: A wrong length is refused on both sides

- **WHEN** text one unit shorter or one unit longer than a Stoa address is parsed
- **THEN** parsing fails reporting the length it found

#### Scenario: An address in uppercase or mixed case parses to the same address

- **WHEN** a Stoa address's display form is parsed with every letter
  uppercased, and again with only some letters uppercased
- **THEN** both results equal the address the display form names
- **AND** rendering either result gives the display form, in lowercase

#### Scenario: A reply reports an address in its display form whatever case it was asked in

- **WHEN** a call that takes a Stoa address and reports one back in its reply is
  given that address with every letter uppercased
- **THEN** the call succeeds as it would for the display form
- **AND** the address its reply reports is the display form, in lowercase
