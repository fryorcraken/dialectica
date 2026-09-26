## Context

See `proposal.md` for why. This change adds tests and two spec deltas. **No
production code changes.** Both deltas pin behaviour the code already has:

- `identity-onboarding` ADDS *A malformed `index` in a keep request is refused
  with a message naming `index`*. The owner decided this on issue #166
  (2026-09-25), and `proposal.md` records the decision. The keep handler already
  refuses that way, through the parser it shares with the feed's `page` and
  `perPage`.
- `thread-read` MODIFIES *An item carries its ordering position and the author's
  asserted time, as two separate fields*, so that a position is determined by
  the item's place alone. `thread::read_thread` already assigns it that way.
  `resolve_item` drops a hidden reply the read did not ask for, and only after
  that does `read_thread` enumerate what is left (`placed.at(index)`). The
  position is therefore the item's index in the sequence that read returned.

Two facts about the code decide where the tests go.

- **A thread item's position is assigned in `thread::read_thread` and copied out
  in `wire::thread_page_json`.** `thread::tests::every_item_carries_its_index_in_the_whole_thread_as_its_position`
  already pins the core value. The mutation issue #166 reports
  (`"position": item.author`) is in `thread_page_json`, one layer above anything
  that core test can see. On the #91 branch, the only test that went red was
  `no_thread_item_holds_its_signers_key_under_any_key_but_author`, and only
  because the value copied in happened to be the signer's key. That was the
  issue's own measurement, and the spec-test reviewer got the same result.
- **`index` is parsed by `parse_index`, the same function behind the feed's
  `page` and `perPage`.** The field name is a parameter formatted into all
  three of its refusal messages. On the #91 branch, dropping the name from the
  wrong-type message was caught only by the feed's
  `malformed_pagination_fields_are_refused_by_name`. That test is about a
  different method, under a requirement that does not cover `index`.

## Goals / Non-Goals

**Goals:**

- Put each position scenario in *An item carries its ordering position…* under
  a wire-level test. Between them, the tests must go red for every mutation the
  issue names (a constant, a per-author value, an index that restarts on every
  page) and for a position taken from the item itself, such as its op id.
- Put each scenario of the new `identity-onboarding` requirement under a
  wire-level test. The test must fail when `index` is dropped from any
  `parse_index` message reachable on this target.

**Non-Goals:**

- The `NO SPEC:` marker on the field *name* `position` in `thread_page_json`.
  The issue excludes it.
- `{"index":null}` answering `missing field: index`. `proposal.md` leaves it
  as an open question, and it is out of scope for this piece.
- Pinning the message text for any malformed `index`. The spec requires the
  message to name `index` and nothing more. `proposal.md`'s *Deliberately left
  unspecified* gives the reasons.

## Decisions

### D1. `index` keeps sharing `parse_index` with `page` and `perPage`

**Chosen:** no parser change. `keep_identity` calls `parse_index(&parsed,
"index")` exactly as it did before, and the new tests pin that the name reaches
the reply.

**Considered:** a dedicated `parse_keep_index` whose messages spell `index` as
a literal, so that the keep requirement would not hang on a function shared
with the feed.

**Ruled out** because the sharing is what makes the requirement hold by
construction. `parse_index` formats `{field}` into every message it produces, so
a caller cannot get a refusal that names some other field. A second parser would
be a second copy of the refusal set: `as_u64` refusing by spelling, `try_from`
rather than `as usize`, and wrong type refused rather than coerced. The two
copies would then have to be kept in step by hand. `parse_index`'s own doc
comment explains why the shared guard is written for the strictest of its
callers: a coerced `index` stores an identity nobody chose.

**The guard, and what breaks without it.** The guard is the `{field}` in each
of `parse_index`'s messages. Drop it from the wrong-type arm (`"must be a
number"`) and `each_malformed_kind_of_index_is_refused_by_name` goes red on
`"two"`, `[]` and `true`, as does the feed's
`malformed_pagination_fields_are_refused_by_name`. Drop it instead from the arm
for a number that is not a plain non-negative integer (`"must be a non-negative
integer …"`) and both go red, first on `-1`, along with
`the_largest_page_index_is_an_empty_page_and_one_larger_is_refused_by_name`.
Both arms were measured by mutation, in `1889eaf` and again after the fixture
refactor. The `try_from` arm is unreachable on a 64-bit target (see Risks).

### D2. The position tests assert properties at the wire, not values

**Chosen:** tests in `wire.rs` that read through `read_thread`, the wire
handler. They compare positions to each other rather than to literals. Two are
written:

- `no_two_items_of_a_thread_share_a_position_even_when_they_share_an_author`
  reads every page at `perPage` 2 and asserts pairwise inequality. It goes red
  under all three of the issue's mutations: a constant position, a per-author
  value (`item.author`), and an index that restarts on every page. All three
  were measured.
- `a_position_is_the_same_whatever_page_size_the_read_used` maps item id to
  position at `perPage` 1, 2 and 3 and compares each map to a one-page read.
  It goes red under the per-page restart and **stays green under the constant
  and the per-author value**, because both are the same at every page size.
  The test says so in a comment, so nobody reads its green as covering them.

The third, for the place rule, is D5.

**Considered:** asserting the literal positions `"0".."4"` at the wire, as the
core test does one layer down.

**Ruled out** because `thread-read` deliberately leaves the token's form open:
*"Contracting only that keeps a later change free to alter the token's form
without breaking a caller that stayed inside the contract."* A wire test pinning
`"0"` would turn that freedom into a breaking change. The core test already pins
the current value, where it is an implementation fact rather than a contract.

**What these two cannot see.** A position taken from the item itself, such as
its op id, is unique within a read and the same at every page size, so both
tests pass it. The scenario *The item at a place carries that place's position
in every read* is what fails it, and D5 is how that is tested. Without D5's
test, the only thing standing in the way of an op-id position would be the core
value test, which sits one layer below `thread_page_json`, where #166's mutation
was made.

### D3. The thread fixture has two pairs of items sharing an author, and five items

`a_thread_log_with_shared_authors` holds the root and one reply by key 2, two
replies by key 3, and one by key 4. `a_thread_log` has one item per author, so
any value derived from the author alone is unique there. That is why the older
tests stayed green. Two shared pairs, rather than one, mean the per-author
mutation collides whichever pair happens to come first. Five items at `perPage`
2 make three pages with a partial last page, so a per-page index repeats. The
test also asserts that the fixture has fewer authors than items, so a later edit
that gives each item its own author fails loudly instead of quietly removing the
property.

### D4. The keep tests send `index` as raw JSON text

`keep_through_the_wire` took an `i64`, which cannot spell `1.5`, `1e2`, `"two"`,
`[]`, `true` or `18446744073709551616`. A test-only refactor, in its own commit
with no behaviour change, moved the request building into `keep_with_raw_index`,
and the `i64` helper now delegates to it. The malformed kinds sit in one table,
`MALFORMED_INDEXES`, each paired with the spec bullet it stands for. The by-name
test and the stores-nothing test both iterate it, so the two cannot disagree
about what "malformed" means.

### D5. A position belongs to the place, and the test compares two reads rather than pinning a value

**Why the op id is not a position.** The issue asks for a test that fails for
*any* position breaking *An item carries its ordering position…*. One value
could not be judged against the text as it stood: the item's own op id. It is
unique within a read and the same at every page size, so it passed every
scenario the requirement had. The requirement did say the position *"SHALL
identify where an item sits in the whole thread's sequence"*, and an op id
identifies **the item, not where the item sits**. No scenario tested that
sentence, though, so no test could fail an op id without pinning the token's
form, which D2 rules out. Which values count as a position is observable
behaviour, so it was settled in the spec rather than here. `thread-read` now
says the position is determined by the place alone, and that two reads returning
different items at one place give them the same position.

That rule is also why the uniqueness sentence was narrowed to *one read*. Under
the place rule, the position at a place belongs to the hidden reply in a read
that includes it and to the reply after it in a read that does not. "No two
items of a thread ever share a position" would contradict that; "no two items
of one read" does not.

**Chosen for the test:** read one thread twice across every page, once with
`includeHidden` true and once without, where a moderator has hidden a reply that
has another reply after it. Compare the two reads place by place, and compare the
following reply's position across the two reads. `includeHidden` is the wire's
way to get two different sequences out of one log without changing the log.

The scenario has two clauses, and each fails a different kind of wrong value:

- **The following reply carries a different position in each read.** Any value
  computed from the item alone is the same for one item in both reads, so this
  clause fails all of them together: the op id, the author, a hash of either, and
  a constant. It is the clause that does the work against #166's class of defect.
- **At each place both reads fill, the two items carry the same position.** This
  fails a value that changes between reads but not with the place, such as a
  per-read counter or a random token.

It is not designed to catch an index that restarts on every page. Both reads use
the same page size, so a per-page index agrees at every place. The page-size test
in D2 catches that.

**Considered:**

- **Asserting the position is not the item's id and not its author.** Ruled out
  because it is a list of known-bad values. It passes a hash of the id, or any
  other field of the item, and nothing would notice the list had gone stale.
  The place rule fails every item-derived value at once, including ones nobody
  has thought of.
- **Pinning literal positions at the wire.** Ruled out for the reason in D2.

**What breaks without it**, measured: set the position to the item's op id in
`thread_page_json`, the mutation the proposal names, and
`the_item_at_a_place_carries_that_places_position_in_every_read` goes red while
D2's two tests stay green. It also goes red under a constant and under
`item.author`. It stays green under a per-page index, which it is not designed to
catch. The `tester` proved it in `1889eaf`, and the proof was re-run after the
fixture refactor that answered the architecture review.

**The fixture's replies carry explicit ascending clocks, and that is a guard.**
The test needs `after` to sit directly behind the reply that gets hidden.
`arrival::cmp_ops` orders two ops without a clock by ascending op id, which is
a hash, so appending the replies in order does not put them in that order. The
first version of the fixture had no clocks, and it failed against the correct
implementation. Take the counters out of
`a_thread_log_with_a_hidden_reply_and_a_reply_after_it` and the test's
adjacency depends on hash order. It then fails on correct code, or passes
without testing the place it names. `a_thread_post_with_clock` is the one
constructor for fixture posts, so this fixture and `a_thread_post` build the
`Op` the same way. They differ only in the clock.

## Risks / Trade-offs

- **[`u64::MAX + 1` gives the wrong reason]** → `18446744073709551616` is not
  held as an integer by serde_json. It falls back to `f64` and is refused as
  *"index must be a non-negative integer written without a decimal point or
  exponent"*, which I confirmed by running it. `parse_index`'s `try_from` arm
  ("larger than this build can represent") is reachable only where `usize` is
  narrower than 64 bits. The message names `index`, which is all the spec
  requires. `proposal.md`'s *Deliberately left unspecified* records why the
  reason is not contracted, and that a fix belongs in its own issue against the
  parser, covering `page` and `perPage` at the same time.
- **[The page-size test alone would pass a constant]** → By design (D2). The
  uniqueness test covers it, and both tests' comments say which mutation each
  one catches.
- **[The place test depends on a hidden reply being dropped from the default
  read]** → That is `thread-read`'s moderation behaviour, not something this
  change adds. If a later change kept hidden replies in the default read, the two
  reads would return the same sequence, and the test would lose its power to
  tell a place-derived value from an item-derived one. The scenario's AND clause
  expects the following reply's position to differ between the reads, so that
  change turns the test red rather than leaving it green and blind.
