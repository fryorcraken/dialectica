## Why

Issue #166 carries two test gaps the tester found on #91 (PR #163), both left out
of that change's scope.

1. **A thread item's `position` can be replaced with a per-author value and the
   older tests stay green.** `thread-read`'s *An item carries its ordering position
   and the author's asserted time, as two separate fields* already forbids it:
   *Two items of one thread never share a position* fails whenever two items share
   an author. The existing fixtures never have two items by one author, so no test
   sees it. **The requirement exists, and the tests are what is missing.**
2. **Nothing requires a malformed `index` in a keep request to be refused by
   name.** `index` is parsed by the same code as the feed's `page` and `perPage`,
   and `feed-read` requires a refusal of those to name the field. No spec says the
   same about `index`, so dropping the field name from the refusal went unnoticed.
   It was caught only by the feed test. The owner decided this on 2026-09-25
   (issue #166, comment 5832956024): a malformed `index` is refused with an error
   that names `index`, as `feed-read` requires for `page` and `perPage`. Leaving the
   message unspecified is ruled out.

## What Changes

- **`identity-onboarding` gains one requirement:** a keep request's `index` that
  is negative, fractional, written with a decimal point or an exponent, not a
  number, or too large for this peer to represent is refused with the wire
  contract's error shape. The message names `index`, and nothing is stored. The
  list of malformed kinds follows `feed-read`'s list for `page` and `perPage`.
  The requirement also says where it stops. A well-formed integer that names no
  candidate is not malformed: *A selection outside the current set is refused*
  already governs it, with a not-kept reply rather than the error shape.
- **Item 1 adds no requirement.** Its work is tests: a fixture holding two items
  by the same author, and mutations (a constant position, a per-author position,
  a position that restarts on every page) each shown red.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `identity-onboarding`: ADDED *A malformed `index` in a keep request is refused
  with a message naming `index`*. This is the capability that owns keeping a
  candidate, so the requirement goes here. `feed-read`'s requirement is not
  touched and is not restated, because it governs different fields. The existing
  *Every entry point refuses malformed input rather than guessing* is left
  unchanged: it requires the refusal but says nothing about the message.

`thread-read` is not modified. Item 1 is tests against a requirement it already
has.

## Impact

- **No behaviour change is intended.** On this tree the keep handler already
  refuses each malformed kind with the error shape and a message naming `index`,
  because it shares the parser with `page` and `perPage`. The requirement pins
  what the code does, so that the name can no longer be dropped silently.
- **Tests:**
  - A keep request whose `stoa` and `slate` are valid and live, with each
    malformed `index` in turn. Assert the error shape and a message naming
    `index`, and prove it by mutating the field name out of the message.
  - A thread fixture with two items by the same author, read across pages, with
    the mutations above.
- **Out of scope:**
  - The `NO SPEC:` marker in `thread_page_json` on the field *name* `position`.
    The issue excludes it.

## Open questions for the owner

### An explicit `null` `index` is reported as missing

`{"index":null}` gets `missing field: index`. `module-wire-contract`'s null rule,
reading 3, says a required field whose type does not admit `null` is refused as
the wrong type. Its scenario *A null required field is refused as a wrong type*
says the message must not call the field missing. `index` is required, so the
current reply looks non-conforming to that existing requirement. This change does
not restate the null rule for `index`, since that would put one rule in two
capabilities. Nothing in this change touches the behaviour either.
`one_field_has_one_null_reading` does not sweep `index`, which is how the gap went
unnoticed. Should this piece fix it, or should it get its own issue?
