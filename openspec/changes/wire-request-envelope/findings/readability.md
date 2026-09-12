# Readability findings — `wire-request-envelope`

**Reconstructed from the commit record** — see the note at the top of
`correctness.md` for why that is itself a finding about how this piece was run.

The theme of this dimension on this piece is one defect repeated: **a comment
that argues from a premise which is checkably false.** Four instances were found,
across three separate passes, and the last two were found only because the third
prompted a deliberate sweep. Each is recorded separately, because "the comments
were wrong" is not actionable and "this sentence asserts five of something that
there is one of" is.

---

## R1. `REQUEST_NOT_AN_OBJECT`'s doc argues a false premise — **Fixed**

For: `dev-writer`

`wire/request.rs:42-48`. The doc comment said the string is a `const` rather than
a literal because it appears "at five call sites" and "five copies would drift
and one would eventually collide with a neighbour".

There is **one** production use, at `wire/request.rs:188`. Every other occurrence
is a test assertion — which is the opposite of a call site that could drift, since
a test asserting the literal is the mechanism that *catches* drift.

The five-call-sites argument belongs to the per-handler `if !parsed.is_object()`
design this change **rejected** (`design.md` §1). It is an argument for a
structure that does not exist, left behind when the structure was replaced — the
worst kind of stale comment, because it reads as a justification and a reader
checking it finds the code disagrees.

The real reason is stronger, and `design.md` §3 states it correctly: the message
is **contract surface a view may render**, so its wording is a contract change,
pinned by a known-answer test
(`the_non_object_message_is_pinned_to_a_known_answer`). That reason survives the
number of call sites changing.

**Fixed** in this pass. `design.md` §3's parenthetical "(one string, not five)"
carried the same false premise in milder form and is fixed with it — one instance
left standing is the template the next author copies (see the project's recorded
"unfixed test patterns get copied").

## R2. Two test names claim more than their bodies assert — **Fixed**

For: `tester`

A name that survives the behaviour changing is a name that describes a
mechanism, not a property. Both of these did.

**`request_parse_is_the_only_way_to_reach_a_field_read`** — the name asserts
exclusivity. The body asserts that `Request::parse` refuses every non-object
`Value` variant, that `{}` is accepted, and that an unparseable request gets a
different message. None of that is exclusivity, and exclusivity is **false** in
the sense the name implies: `the_sixth_method_the_boundary_does_not_stop` builds
a handler that reaches a field read without `Request::parse` at all, and it is a
passing test in the same file. So the name contradicts a neighbouring test.

Renamed to `request_parse_refuses_every_non_object_json_value`, which is what the
loop falsifies.

**`every_handler_answers_with_an_object_carrying_exactly_one_top_level_shape`** —
"exactly one shape" is not asserted anywhere in the body, which checks
`v.is_object()` for each of two sweeps. Asserting "exactly one top-level shape"
would mean asserting the key set, and the bodies deliberately do not (the success
shapes differ per method — `pong`, `channelId`, `items`/`page`/`hasMore`).

Renamed to `every_handler_answers_with_a_json_object_for_any_request_shape`,
which is what it checks and is worth checking: the reply half must hold for a
refused request too, and a refusal built by hand rather than through `error_json`
is how it breaks.

**Fixed** in this pass. Neither rename changes an assertion, so the suite count
is unmoved.

## R3. A comment stated its own premise backwards — **Fixed**

For: `dev-writer`

The null-is-present comment named the two readers that *don't* distinguish a null
as the two that do. Recorded in full as C3 in `correctness.md`, because the
measurement it suppressed was a correctness gap rather than a wording problem.
Fixed in `134240b`.

## R4. `design.md` claimed "the only parse of a request left in the crate" — **Fixed**

For: `dev-writer`

Contradicted by `list_threads_from_request` → `list_threads`, two parses for one
call. Recorded as C1 in `correctness.md`. **Fixed in the code rather than in the
sentence**, which is the part worth keeping: correcting the document would have
left the cost in place to keep a claim true.

## R5. The sweep for a fourth found three — **Fixed**

For: `dev-writer`

R1 was the third instance of this defect in this file, which made "is there a
fourth?" a question worth asking rather than a courtesy. A dedicated sweep of
both files ran in this pass, checking every numeric claim, every "X is the only
Y", every claim about which items are in a set, and every test or function name
cited by a comment (each verified to exist).

**It found three more, bringing the total to six.** Each was independently
re-verified before being fixed, because a confidently-argued finding is exactly
the shape this project's "persuasive citations get fabricated" note warns about.

### R5a. A fabricated duration used as evidence — **Fixed**

`wire.rs`, `the_index_refusal_says_what_it_actually_refuses`:

> because the reason this **was wrong for two years** is that nothing read the
> message beside the input that produced it

**Verified false**: `git log -S "non-negative whole number"` on `wire.rs` returns
exactly two commits — `0538c0d` introducing the message and `134240b` fixing it —
**both dated 2026-09-12**. The repository's first commit is five days older than
that, so no comment in it can truthfully describe a two-year-old defect.

**This is the worst of the six**, and the reason is not the arithmetic. The
duration was the load-bearing half of the sentence: *"the reason this was wrong
for two years is…"* offers the timespan as the evidence for the diagnosis. The
structural point it was supporting is true and needs no duration at all. This is
the project's recorded "persuasive citations get fabricated" trap wearing a
number instead of a reference — and it was written in the very commit whose
subject is "Stop three comments and one message from saying the wrong thing".

Fixed: the comment now states the structural reason alone, and records the
invented duration and the `git log -S` that disproves it, so the next reader sees
the trap rather than a clean sentence.

### R5b. "Three orders of magnitude" is a factor of 16 — **Fixed**

`wire/request.rs`, `MAX_REQUEST_BYTES`:

> 4 MiB clears that with room for a field the future adds, and still refuses
> **three orders of magnitude** below the 64 MiB that was served.

**Arithmetic**: 64 MiB = 67,108,864; the cap is 4 × 1024 × 1024 = 4,194,304.
67,108,864 ÷ 4,194,304 = **16**, which is ~1.2 orders of magnitude. Wrong by
about 60×.

It is also **self-contradicting**: three orders of magnitude below 64 MiB is
about 67 KiB, a cap that would refuse the ~1.6 MB legitimate request the same
paragraph says must be cleared.

Incidental rather than load-bearing — the conclusion (4 MiB clears the legitimate
maximum and refuses well below the measured abuse) survives, and the rest of the
derivation checks out. But it overstated the safety margin by 60× in a comment
whose whole job is justifying a security number, so the fix states the factor as
arithmetic and says explicitly that the cap is bounded from both sides.

### R5c. "Three request shapes, compared against each other" — five, and not compared — **Fixed**

`wire.rs`, the feed-path refactor test:

> Two entry points, **three request shapes each, compared against each other** —
> because the refactor's whole claim is that these agree.

**Both halves false.** The loop iterates **five** shapes (`full_request()`,
`"[]"`, `"not json"`, `"{}"`, `feed_request("\"page\":1")`). And inside the loop
the two entry points are never compared to each other — each is only checked to be
a JSON object carrying no doubled guard frame. The cross-entry-point equality runs
**once, after the loop, for the well-formed request alone**, as the adjacent
comment itself says ("the one case where the two entry points must agree
exactly").

The count is incidental; **"compared against each other" is load-bearing**,
because this comment's entire job is explaining what the test *can* assert given
that no output difference is observable. It claims a cross-form comparison the
loop does not make, so a reader trusts that five malformed shapes are pinned as
agreeing between entry points. They are not, and they should not be — for a
malformed request the two are not obliged to agree, since only one consults the
genesis.

Note the commit that introduced this test said "across five request shapes" in its
own message. The number was right in the commit and wrong in the code comment, in
the same change — which is the argument for the comment being the thing that gets
checked.

Fixed as part of R6's rename of that test.

### Two candidates deliberately not fixed

Recorded so the next reviewer does not re-raise them as new:

- **`wire.rs`'s "the five states a user can actually be in"**, beside a
  seven-element `makers` array. The sweep could not resolve whether "five" was
  meant as the sweep's size (in which case it is a fourth numeric falsehood) or as
  a claim about reachable user situations (unsourced rather than false), and no
  document anywhere pins the number. **Out of scope for this change** — that
  comment belongs to the keystore probe, not the request envelope, and editing it
  here would put an unrelated file in this diff. Left for whoever owns that area,
  and it cannot be reconciled with the array two lines below it either way.
- **`wire.rs`'s "check 3 of `moderation.rs`'s three"** is a loose analogy — the
  wire check neither implements nor substitutes for `moderation.rs`'s per-op test
  — but its actual conclusion is correct. Imprecise, not false.

## R6. `the_feed_path_parses_its_request_once` claimed what the compiler proves — **Fixed (renamed)**

For: `tester`

Flagged by the previous fixer as its own least-confident test, which is the right
call. The single parse is enforced by `list_threads_inner`'s `&Request` signature
— the compiler — and **not** by anything the test asserts. Reverting that
signature to `&str` would let the defect back in with this test still green.

Renamed to `the_two_feed_entry_points_agree_after_the_parse_moved`, and **kept
rather than cut**, with the reason stated inside it: it is a refactor-safety net,
it is what would have caught the parse-move going wrong, and that is worth having
as long as it does not claim to be the guard. The comment now says in capitals
that the signature is the thing guarding the fix, and corrects the false count
from R5c in the same edit.

Cutting it was the alternative. Rejected because the refactor it protects (moving
a parse and dropping a `guarded` frame across two entry points) is exactly the
kind of change a reply-equality test catches, and nothing else in the file
compares the two entry points at all.
