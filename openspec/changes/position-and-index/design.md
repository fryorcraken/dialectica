## Context

See `proposal.md` for why. This change adds tests. The one spec addition, in
`identity-onboarding`, pins behaviour the code already has. **No production code
changes.**

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
  a wire-level test. Each test must go red under at least one of the issue's
  three mutations, and every mutation must turn at least one test red.
- Put each scenario of the new `identity-onboarding` requirement under a
  wire-level test. The test must fail when `index` is dropped from any
  `parse_index` message reachable on this target.

**Non-Goals:**

- The `NO SPEC:` marker on the field *name* `position` in `thread_page_json`.
  The issue excludes it.
- `{"index":null}` answering `missing field: index`. `proposal.md` leaves it
  as an open question, and it is out of scope for this piece.
- Pinning the message text for any malformed `index`. The spec requires the
  message to name `index` and nothing more.

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
`malformed_pagination_fields_are_refused_by_name`. I measured the first by
mutation while writing this. The `tester` owns the full proofs.

### D2. The position tests assert properties at the wire, not values

**Chosen:** two tests in `wire.rs` that read through `read_thread`, the wire
handler. They compare positions to each other rather than to literals.

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

**Considered:** asserting the literal positions `"0".."4"` at the wire, as the
core test does one layer down.

**Ruled out** because `thread-read` deliberately leaves the token's form open:
*"Contracting only that keeps a later change free to alter the token's form
without breaking a caller that stayed inside the contract."* A wire test pinning `"0"` would turn that
freedom into a breaking change. The core test already pins the current value,
where it is an implementation fact rather than a contract.

**Where this is blind:** a position equal to the item's own id is unique and
the same at every page size, so both tests pass it. Whether that breaks *"The
position SHALL identify where an item sits in the whole thread's sequence"* is
a question the spec's wording cannot settle by a test that stays out of the
token's form. The core value test is what stands in its way today. It is not
among the issue's mutations and is recorded here so it is not mistaken for
covered.

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

## Risks / Trade-offs

- **[`u64::MAX + 1` never reaches the "too large" message]** →
  `18446744073709551616` is not held as an integer by serde_json. It falls back
  to `f64` and is refused as *"index must be a non-negative integer written
  without a decimal point or exponent"*, which I confirmed by running it. The
  message names `index`, which is all the spec requires, but the reason it gives
  is wrong for that input. `parse_index`'s `try_from` arm ("larger than this
  build can represent") is reachable only where `usize` is narrower than 64 bits.
  The message text is observable behaviour the spec leaves open, so this is
  reported to the spec-writer rather than changed here.
- **[The page-size test alone would pass a constant]** → By design (D2). The
  uniqueness test covers it, and both tests' comments say which mutation each
  one catches.
