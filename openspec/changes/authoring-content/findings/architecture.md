# Architecture review: `authoring-content`

**Dimension reviewed: architecture only.** Correctness, security and readability
are other reviewers'. Findings only — nothing was fixed.

Baseline suite confirmed at **531 passing** before and after. Four mutations were
applied and reverted; `git status --porcelain` is empty at hand-off. A standalone
`rustc` probe used for A1 was written to `tmp/probe/` (gitignored) and deleted.

---

## A1 — The `&mut dyn FnMut` sink's stated justification is a false premise, and the adapter contradicts it

**For:** `dev-writer`
**Where:** `dialectica/rust-lib/dialectica-core/src/wire.rs:698-715` (the doc
comment), repeated in `openspec/changes/authoring-content/tasks.md:170-189` §8,
and in the test comment at `wire.rs:3099-3108`.
**Principle:** *comments earn their place by saying what a command cannot* — and
this one says something a command disproves.

The doc comment argues the sink had to become `&mut dyn FnMut(&OpId)` because:

> The module adapter assembles the keystore, the key, the store and the sink once
> and dispatches over the three handlers **through one function-pointer type**. A
> generic `impl FnOnce` monomorphises per call site, so the three are three types
> and cannot share one pointer — and coercing them fails on a higher-ranked
> lifetime […]

**The adapter does not dispatch through a function-pointer type.** It is generic
over the handler (`dialectica/rust-lib/src/lib.rs:276-284`):

```rust
fn publishing<F>(&mut self, request: &str, method: &str, handler: F) -> String
where
    F: FnOnce(
        &str,
        &mut core::log::SqliteOpLog,
        &core::identity::SecretKey,
        &mut dyn FnMut(&core::op::OpId),
    ) -> String,
```

A generic type parameter monomorphises per call site, which is exactly what
accommodates three distinct handler types. The three `self.publishing(...)` calls
at `lib.rs:431/435/439` each instantiate `F` separately. There is no shared
pointer for the handlers to fail to coerce into.

**Failure scenario (verified).** A standalone `rustc --edition 2021` probe
reproducing the adapter's shape — handlers declared as
`fn publish_post<L, D: FnMut(&OpId)>(…, deliver: D) -> String`, adapter declared
as `fn publishing<F, D>(…, sink: D, handler: F) where D: FnMut(&OpId), F: FnOnce(…, D) -> String`
— **compiles clean** (one dead-code warning on the probe's own stub struct). The
generic sink survives an adapter generic over the handler. The premise that it
"does not survive the adapter" is false as the adapter is written.

The *only* thing forcing `dyn` is `wire.rs:3109-3114`, where the test declares
`type Handler = fn(…, &mut dyn FnMut(&OpId)) -> String` **by choice**. So the
constraint is self-imposed by the test, and the test's own comment
(`wire.rs:3104`: "This is the only gate that can check that") asserts it is
checking a property of the adapter that the adapter does not have.

**Why this is a finding and not pedantry.** The comment is exactly the kind a
reader inherits and acts on. It tells the next author that a generic sink is
*impossible* here, so the honest `FnOnce` bound ("a sink is called at most once")
can never be recovered — and the comment pre-emptively closes the question by
arguing nothing is lost. A reader who believes it will not retry. This is the
same failure family as the `tasks.md` §10 "the ordering is unobservable" claim
that commit `cbaacdc` corrected: a note saying a thing is impossible stops anyone
attempting it, so a wrong one costs whatever the attempt would have gained.

**What to do:** either reword to the true reason (*"the shared-signature test
pins a function-pointer type, which requires an erased sink"*), or — if
at-most-once is worth having — change the test's `Handler` type and recover
`impl FnOnce`. Do not leave the current reasoning standing; a future reader will
cite it.

**Outcome:**

---

## A2 — The append-then-deliver tail is three byte-identical copies, so the ordering invariant is a branch got right three times rather than a shape right once

**For:** `dev-writer`
**Where:** `wire.rs:744-750`, `wire.rs:788-794`, `wire.rs:831-837`.
**Principle:** *Put the complexity in the data structure, not the logic* —
specifically "a branch must be got right at every call site; a data shape is
right everywhere at once." Also *a guard is a job.*

All three handlers end in the same five lines, identical to the byte:

```rust
    match crate::authoring::{post|reply|vote}(log, key, stoa, …) {
        Ok(published) => {
            deliver(&published.id);
            published_json(&published)
        }
        Err(refusal) => error_json(&refusal.to_string()),
    }
```

`design.md:59-83` and `wire.rs:683-697` both argue that three requirements are
carried *structurally* by this shape: the append precedes delivery, a declined
handoff leaves the op published, and delivery is never reached by a refusal
because "it sits on the success arm of the `Result` and no refusal path reaches
it." All three claims are true — **of each copy independently.** Nothing makes
them true of the third copy; they are true because three separate `match`
expressions were each written correctly.

**Failure scenario (verified by mutation, then reverted).**

1. Deleting `deliver(&published.id);` from `publish_vote` only (`wire.rs:833`):
   suite goes **529 passed / 2 failed** —
   `the_append_completes_before_delivery_is_invoked_on_all_three_handlers` and
   `the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`.
2. Hoisting delivery ahead of the append in `publish_post` only, with a correctly
   precomputed op id so the reply JSON is unchanged (replacing `wire.rs:744-748`):
   suite goes **529 passed / 2 failed** —
   `the_append_completes_before_delivery_is_invoked_on_all_three_handlers` and
   `a_publish_whose_delivery_panics_still_reports_the_op_as_published`.

Both mutations are *caught*, so this is a shape finding rather than a coverage
gap — the tester's work is what holds it. But note what the mutations demonstrate:
**the hoist is expressible per handler precisely because the tail is copied.** A
per-handler mutation only exists as a thing to write because there are three
independent places to write it. Commit `cbaacdc`'s own message records the cost
this already imposed — §7's mutation 8 "was killed through `publish_reply` only"
and had to be re-run in the other two, with the discovery that one of the three
was caught only by accident (a sink-call *count*, not an ordering).

**The shape that makes it hold by construction.** One function owning the tail,
with the per-kind work reduced to producing a `Result<Published, Refusal>`:

```rust
fn deliver_and_reply(
    result: Result<crate::authoring::Published, crate::authoring::Refusal>,
    deliver: &mut dyn FnMut(&crate::op::OpId),
) -> String {
    match result {
        Ok(published) => {
            deliver(&published.id);
            published_json(&published)
        }
        Err(refusal) => error_json(&refusal.to_string()),
    }
}
```

Each handler then ends `deliver_and_reply(crate::authoring::vote(log, key, stoa, target, direction), deliver)`.
There is then **one** place where `deliver` is named, on **one** success arm, and
a hoist is no longer writable in a handler at all — the handler has no access to
an op id before the call. This also converts §7's "re-run the mutation in all
three" into a single mutation against a single function, which is the payoff
`design.md:53-57` already claims for `authoring::publish` and did not take here.

Note the asymmetry that makes this a real omission rather than taste: the change
*did* apply exactly this reasoning one layer down. `authoring::publish`
(`authoring.rs:166-175`) exists so that sign-then-append "is asserted once rather
than three times against three near-copies" (`authoring.rs:157-158`). The
append-then-deliver half of the same ordering requirement got three near-copies.

**Outcome:**

---

## A3 — The parse-then-decide prologue is three copies of one sequence, and `design.md` records a data shape the code does not have

**For:** `dev-writer` (with a note for `design-reviewer`, whose dimension the
second half is)
**Where:** `wire.rs:728-742`, `wire.rs:768-786`, `wire.rs:811-829`; the recorded
decision at `openspec/changes/authoring-content/design.md:37-45`.
**Principle:** *Put the complexity in the data structure, not the logic.*

`design.md` §"The request is parsed into a typed intent before anything is
signed" says:

> Each handler parses its request into one of three small structs
> (`PostRequest`, `ReplyRequest`, `VoteRequest`) and only then calls into
> `authoring.rs`.

**No such struct exists.** `grep` finds no `PostRequest`, `ReplyRequest` or
`VoteRequest` anywhere in the crate. What shipped is a per-handler sequence of
inline `match` arms over `serde_json::Value`, repeated three times:

- `parsed_object(request)` match — 4 lines, identical ×3
- `reject_forbidden_fields(&parsed)` if-let — 3 lines, identical ×3
- `required_stoa(&parsed)` match — 4 lines, identical ×3
- then 1–2 further field matches, in the same 4-line `match … { Ok(v) => v, Err(e) => return e }` shape

The recorded decision was a *data* shape ("parse into a struct, then act"), and
the implementation is a *logic* shape (the same statement sequence, three times).
The requirement it was chosen to carry — "a refused publish appends nothing and
does not invoke delivery is structural, because there is nothing to append until
every field has been read" (`design.md:43-45`, restated at `wire.rs:670-676`) —
is again true of each copy separately rather than by construction. There is no
type in the program whose existence means "every field parsed"; there is only the
fact that each handler happens to place its `crate::authoring::*` call last.

**Failure scenario.** Adding a fourth operation (the obvious next one is
`publish_moderation`, given `OpKind::Moderate` already exists and
`authoring.rs:1028-1038` hand-builds one in a test) requires copying that
prologue a fourth time. The `reject_forbidden_fields` line is the guard most
likely to be dropped in the copy, and `tasks.md:164-168` already identifies
exactly this: *"One handler forgetting the guard is invisible without checking
all three."* Its mitigation is a test that loops over all three — which is the
CLAUDE.md signal quoted verbatim in the project guidance: *"When you find
yourself writing the fourth slightly-different copy of a guard, that is the
signal to reshape rather than to add a fourth test."* The change wrote the
looping test. It is the right test; it is not a reshape.

**The shape that makes it hold by construction.** The one `design.md` already
recorded. A `PublishRequest` (or the three named structs) with a single
constructor that runs `parsed_object`, `reject_forbidden_fields` and
`required_stoa` before any per-kind field, so the prologue has exactly one home
and a value of that type is *evidence* the guard ran. A fourth operation then
inherits the guard by taking the type, rather than by its author remembering to
copy three lines — and "is the guard called everywhere?" becomes a question the
type system answers instead of a question a test loop answers.

For `design-reviewer`: whichever way this is resolved, `design.md:39-40` names
three structs that do not exist and must stop doing so in the same change.

**Outcome:**

---

## A4 — `authoring::reply` and `authoring::vote` carry two copies of the held-and-in-this-Stoa guard, with `&'static str` as the discriminator

**For:** `dev-writer`
**Where:** `authoring.rs:274-292` (`reply`) and `authoring.rs:338-348` (`vote`).
**Principle:** *A guard is a job — keep it separate, so "is it called
everywhere?" stays a question with an answer.*

Both functions open with the same two-step guard, differing only in the
`what: &'static str` label and in whether the kind check is present:

```rust
// reply
let entry = log.get(&parent)?.ok_or(Refusal::NotHeld { what: "parent", id: parent })?;
…
if parent_op.stoa != stoa { return Err(Refusal::WrongStoa { what: "parent", … }); }

// vote
let entry = log.get(&target)?.ok_or(Refusal::NotHeld { what: "target", id: target })?;
if entry.op.op.stoa != stoa { return Err(Refusal::WrongStoa { what: "target", … }); }
```

`grep -c "NotHeld\|WrongStoa"` returns 15 across the file (declarations, `Display`,
construction sites and test assertions). There is no shared helper —
`grep` for `held_in_stoa`, `target_in`, `held_op` finds nothing.

This is the second copy, not the fourth, so it is **below** the reshape
threshold the project sets and I am not calling it a defect today. It is recorded
because the third copy is already visible in the roadmap: any op naming a target
it must hold — a moderation, a revision — needs the identical two steps, and at
that point the label-as-string pattern is carrying the difference between three
guards.

**The shape, when the third arrives.** A single
`fn held_in_stoa<L: OpLog>(log: &L, what: &'static str, id: OpId, stoa: Address) -> Result<Entry, Refusal>`
doing get → `NotHeld` → Stoa compare → `WrongStoa`, returning the entry. `reply`
then adds only its kind check, which is genuinely its own (a vote deliberately
does not have one — `authoring.rs:313-323` argues that well and it should stay
outside the shared helper).

**Outcome:**

---

## What is right, and why

These are results the next reader needs as much as the defects.

### The `authoring.rs` / `wire.rs` split is load-bearing, not copied ceremony

`authoring.rs:11-17` claims the split "is `feed.rs`'s and it is not cosmetic."
Verified against the code: `authoring.rs` takes `Address`, `OpId`, `SecretKey`,
`VoteDirection` and never a `&str` of JSON, and `wire.rs` holds every
`serde_json` reference on the publish path. The payoff is visible in the test
files — `authoring.rs`'s tests assert on `Refusal::WrongStoa { … }` variants with
no JSON anywhere, and `wire.rs`'s assert on JSON shapes. That is the thing the
split buys, and it was actually collected.

### `authoring::publish` earns its existence

`authoring.rs:166-175` is the sign-then-append ordering in **one** place for all
three operations, exactly as `design.md:47-57` argues. This is the correct
application of the principle that A2 says was not carried through to the
delivery half — which is what makes A2 an omission rather than a disagreement
about style.

### Deriving the thread instead of accepting it is the right kind of fix

`authoring.rs:249-266` removes a parameter rather than adding a check. A reply
filed under a thread its parent does not belong to is *unrepresentable* — there
is no argument to get wrong — rather than guarded at each call site. This is the
"put the complexity in the data structure" principle applied correctly, and
`thread_of` (`authoring.rs:239-247`) is one expression satisfying both the
reply-to-root and reply-to-reply cases. The contracted cost (a reply to an
unheld parent cannot be published) is stated rather than hidden.

### The core API was widened deliberately

Three methods, one reply shape `{"opId","wasNew"}` across all three, and failure
always `{"error":"…"}` via `error_json`. `wire.rs:527-536` argues the *absence*
of a score/count/tally on the vote reply, and pins it as structural by making the
vote reply identical to the post reply — so a caller cannot infer an effect that
does not exist. `design.md:129-141` explicitly declines to widen the request with
a genesis record, with the reason stated ("widening the deliverable for
nothing"). `wasNew` is passed through from `op-log` rather than recomputed
(`authoring.rs:49-58`), with the get-before-append alternative rejected on the
record. This is a widening made on purpose.

### The core/UI split is respected

No filesystem or network access anywhere in `authoring.rs` or in the publish
handlers. Every host-reachable thing — keystore path, store, delivery sink — is
supplied by the adapter, on the same pattern `get_capabilities` (a lookup
closure) and `list_threads_from_request` (a store opener) already use.
`authoring.rs:19-23` states why, and the code matches.

### `no_identity` is the right resolution of a genuinely awkward constraint

`wire.rs:545-569` puts the refusal's *text* in `Refusal::NoIdentity`'s `Display`
(compiled and asserted by `cargo test`) while the refusal is *raised* in the
adapter (the only code that can open a keystore). The alternative — a `format!`
in `lib.rs` — would have shipped a message behind `cfg(logos_scaffold)` that no
gate compiles, which `wire.rs:558-562` records as what the first draft actually
did. This is the awkwardness of the `cfg` boundary absorbed in the direction that
*gains* coverage, and it is the good example of what A1 and A3 did not do.

One shape observation, not a defect: no function in `authoring.rs` can construct
`Refusal::NoIdentity` (`grep` confirms the only two mentions are its `Display`
arm and one test). So `post`/`reply`/`vote` return `Result<_, Refusal>` over an
enum carrying a variant they cannot produce, and an exhaustive `match` at a
future call site must handle an unreachable arm. Splitting the enum would fix
that and would cost the tested single home for the message — the current trade is
the right one. Recorded so the next reader does not "tidy" it.

### Answering the question put to this review

**Are the three handlers' shared shape carrying its weight, or are they three
copies of a guard?** Both, in different places, and the line falls where the
principles predict:

- **Genuinely common, correctly factored out:** `reject_forbidden_fields`,
  `required_string`, `required_stoa`, `required_op_id`, `required_direction`,
  `parsed_object`, `published_json`, `error_json`, `guarded`. Each is one job
  with one home. `required_string`'s missing-versus-wrong-typed distinction is
  made once and inherited by all three handlers; `parse_index`'s doc
  (`wire.rs:449-451`) states the reason a second copy is a hazard, and the same
  reasoning applied here.
- **Three near-identical copies:** the parse prologue (A3) and the
  append-then-deliver tail (A2). What is common in these is the *sequence*, and a
  sequence is precisely what a shape can hold and a copy cannot.
- **Is the shared signature load-bearing?** Not for the reason recorded (A1). It
  is load-bearing for the *test* that pins it, and that test does catch real
  regressions — it was one of the two failures in both of my mutations. But its
  stated justification, that the adapter needs one function-pointer type, is
  false: the adapter is generic over the handler. So the `dyn FnMut` was not
  forced on everything for one call site's benefit — it was forced by one
  *test's* choice of type, which is a much easier thing to revisit than the
  comment implies.
- **Could a fourth handler be added without copying a guard?** **No.** It would
  copy the prologue (A3) including `reject_forbidden_fields`, and copy the tail
  (A2) including the `deliver`-on-success-arm-only placement. Both copies are the
  kind that stay green while being wrong in a way only a per-handler mutation
  finds — which `cbaacdc`'s own message documents happening once already.

## Areas reviewed and found clean

- `guarded` wrapping every handler, including the publish path: one job, applied
  at every entry point, and the panic-to-error-shape conversion has one home.
- The `FORBIDDEN_FIELDS` table: complexity in the data structure rather than in
  five branches, with the union-across-operations decision stated and the
  NO SPEC marker where the spec is silent.
- No function on the publish path has an `And` in its name, a vague
  `handle`/`process`/`update` verb, or a mid-body comment introducing a second
  phase. The `publish_*` names say what each does.
- `feed.rs`'s split was the stated model and the publish path follows it
  faithfully at the `authoring`/`wire` boundary.
- No new dependency was introduced by this change.

## Tree state

Four mutations applied and individually reverted (two in `publish_vote`'s tail,
one hoist in `publish_post`, and the A1 `rustc` probe which never touched tracked
files). `git -C … status --porcelain` produced no output after the last revert,
and the 531-test baseline was re-confirmed. `tmp/probe/` deleted. The gitignored
`dialectica/logos-rust-sdk-src` symlink was created as instructed and is not
committed.
