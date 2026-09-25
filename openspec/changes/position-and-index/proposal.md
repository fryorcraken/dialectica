## Why

Issue #166 carries two test gaps the tester found on #91 (PR #163), both left out
of that change's scope.

1. **A thread item's `position` can be replaced with a per-author value and the
   older tests stay green.** `thread-read`'s *An item carries its ordering position
   and the author's asserted time, as two separate fields* already forbids it:
   *Two items of one thread never share a position* fails whenever two items share
   an author. The existing fixtures never have two items by one author, so no test
   sees it. **The requirement exists, and the tests are what is missing**, with
   one exception, given under What Changes: a position equal to the item's op id.
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
- **Item 1 is mostly tests**: a fixture holding two items by the same author, and
  mutations (a constant position, a per-author position, a position that restarts
  on every page) each shown red.
- **Item 1 also clarifies `thread-read`'s position requirement, in one respect.**
  The issue asks for a test that fails for *any* position breaking *An item
  carries its ordering position…*. One value could not be judged against the
  text: a position equal to the item's own op id. It is unique within a read and
  the same at every page size, so it passes every existing scenario. The
  requirement says the position *"SHALL identify where an item sits"*, and an op
  id identifies the item, not where it sits. But no scenario tested that, so the
  question could not be settled. The requirement now states it: the position is
  determined by the place alone, so two reads of one thread that return different
  items at the same place give them the same position. It gains a scenario that
  checks this by reading a thread with a hidden reply once with hidden content
  included and once without. The uniqueness sentence is narrowed to *within one
  read*, because under the place rule one position can belong to two different
  items when two reads return different sequences. The token's form stays open,
  as the requirement already intends.

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

- `thread-read`: MODIFIED *An item carries its ordering position and the
  author's asserted time, as two separate fields*. Two edits and one scenario;
  the rest of the block is copied unchanged. Its uniqueness sentence now reads
  *"for two distinct items returned by one read of a thread, taken across all of
  that read's pages"*. A new paragraph says the position is determined by the
  place alone. A new scenario, *The item at a place carries that place's position
  in every read*, tests that paragraph. The current code already behaves this way:
  the position is the item's index in the sequence the read returned.

## Deliberately left unspecified

### The reason a malformed `index`'s message gives

`18446744073709551616` (one past `u64::MAX`) is refused with a message naming
`index`. The reason it gives is the one for a decimal point or an exponent, which
is wrong for that input (see `design.md`, Risks). The spec requires the message
to **name `index`**. It does not require the message to give the right reason,
and this change does not add that. The reasons are:

- The owner's decision (issue #166, comment 5832956024) fixes the name and nothing
  else about the message. `feed-read` requires the same for `page` and `perPage`,
  which share this parser. Contracting the reason for `index` alone would make
  `index` the only one of the three whose wording is contracted.
- The error shape is one string, and no caller branches on its wording. The
  field name is the part a caller or a person needs in order to find which input
  was wrong. The rest is diagnostic prose.
- If the wording were contracted, a test would have to pin the text of each
  kind's message. That test would fail on a harmless rewording and still pass a
  message that is misleading in some new way.

The wrong reason is a defect in wording, not in the contract. If it is worth
fixing, it belongs in its own issue against the parser, which would fix it for
all three fields at once.

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
  - A thread with a hidden reply that has a reply after it, read with and without
    hidden content. The positions at each place match across the two reads.
    Proven by mutating the position to the item's op id.
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
