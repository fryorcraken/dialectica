# Correctness findings — `wire-request-envelope`

**Reconstructed, and that is a defect in how this piece was run.** These
findings were relayed to the fixer through briefs rather than through this file,
which `.claude/agents/README.md` forbids: "Never relay a finding through a
brief — name the file." They are written down here from the evidence that
survived in the commit messages and the PR body, so the runner can verify the
piece against the file rather than against the runner's own memory of what was
said. Where a measurement is recorded below it came from the reviewer's own
words in the commit that fixed it; where it did not survive, that is said.

---

## C1. The feed path parsed its request twice — **Fixed**

For: `dev-writer`

`list_threads_from_request` parsed the request to read `stoa` and `genesis`,
then handed the raw `&str` to `list_threads`, which parsed it again. Two
`Value` trees live at once and two nested `guarded` frames for one call.

Failure scenario: it is not a wrong answer — both replies were byte-identical,
which is why it survived on `main`. The cost is the one the size cap exists to
bound: the double parse doubles the transient heap on the one path carrying the
larger payload (a hex genesis record beside the rest of the request), and per
`docs/PHASE0-FINDINGS.md` §3 an allocation failure there aborts the module
process rather than returning an error.

`wire.rs`, `list_threads_from_request` → `list_threads`.

**Fixed** in `f2f0f7f` — "Parse the feed request once, and make the second parse
unspellable". `list_threads_inner` now takes `&Request` instead of `&str`, so
the second parse cannot be written without widening a private signature on
purpose; the nested `guarded` frame went with it. Note what the fix did *not*
do: correcting `design.md`'s "the only parse of a request left in the crate" to
concede a second parse would have left the cost in place to keep a sentence
true.

The test that came with it, `the_feed_path_parses_its_request_once`, is the
piece's weakest — see F3 in `spec-test.md`. Renamed in this pass.

## C2. `parse_index`'s refusal message was factually false — **Fixed**

For: `dev-writer`

`{"page":1e2}` → `"page must be a non-negative whole number"`. `1e2` is JSON for
exactly 100; `{"page":0.0}` is exactly 0. Both are non-negative and whole, and
both were told they were not. `as_u64` refuses any number serde parsed as `f64`
— that is, any number carrying a `.` or an `e` — so the refusal is by
**spelling**, not by value. Several JSON serialisers emit `1e2` for 100, so this
is a spelling a legitimate caller sends.

**Fixed** in `134240b` — the message now names the spelling.
`the_index_refusal_says_what_it_actually_refuses` pins the new message for all
four spellings that earn it, and asserts `100` is still served so a function
refusing every number cannot satisfy it.

**The acceptance is deliberately unchanged**, and that is deferred rather than
fixed — see F4 in `spec-test.md`. Whether an exactly-integral float is a valid
index is a contract question the spec does not answer, and an envelope change
should not settle it.

## C3. A comment stated its own premise backwards — **Fixed**

For: `dev-writer`

The null-is-present comment said "`parse_index` and `includeHidden` both
distinguish them". Those two are the only readers that **don't**. Four of the
seven production field reads do:

| request | before | under a `get` that drops nulls |
|---|---|---|
| `{"payload":null}` | `{"pong":null}` | `missing field: payload` |
| `{"channelId":null}` | `channelId must be a string` | `missing field: channelId` |
| `{"stoa":null}` | `stoa must be a string` | `missing field: stoa` |
| `{"genesis":null}` | `genesis must be a string` | `missing field: genesis` |

`ping` flips from *success* to error, which is a behaviour change and not a
reworded message; the other three collapse the wrong-type-against-missing
distinction the contract requires.

Why this matters beyond the comment: the mutation
`self.0.get(field).filter(|v| !v.is_null())` survived **486 of 487** tests, and
the conclusion first drawn from that — that nothing observes the difference —
was wrong. It was a coverage gap in the handler sweeps. The comment invited
exactly that inference, and a future reader would have inherited it.

**Fixed** in `134240b`; the comment now carries the measurement rather than the
claim. The gap is closed in `66de130` with one fixture per differing reader —
four independent kills.
