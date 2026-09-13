# Design review: `authoring-content`

Scope: did the code take the decisions `design.md` records, and were the
decisions worth recording recorded? Not code quality, not test coverage.

Gates run in the review worktree: `cargo test -p dialectica -p dialectica-core`
— **531 passed**, matching the stated baseline. `openspec validate
authoring-content --strict` — valid. Two mutations were applied and reverted;
`git status` is clean.

Decisions are, on the whole, in good shape: eight entries, each naming what was
rejected, and the two hardest calls (the thread derivation and what it trusts;
delivery as a sink whose outcome is discarded) are argued at the right depth.
The defects below are concentrated in two places — a Decisions entry that
describes a signature the code no longer has, and four decisions the code took
that no document records.

---

## The three leads

### Lead 1 — `Refusal::NoIdentity` has one source of its text. CLEARS, and proven by mutation.

There is genuinely **one** home for the wording, and the claim survives the
sharper test the author asked for.

- The sentence exists exactly once in the repo:
  `/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/dialectica/rust-lib/dialectica-core/src/authoring.rs:117-121`
  — `"no identity is available to sign with: {why}; nothing was published and no
  key was created"`. A repo-wide grep for that phrase over `dialectica/`,
  `dialectica-ui/`, `docs/` and `openspec/` returns that line plus two
  *requirement* sentences in `spec.md` that are not the message.
- The adapter **raises** and does not **word**:
  `/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/dialectica/rust-lib/src/lib.rs:323`
  — `Err(e) => return core::no_identity(&e.to_string()),`
- `no_identity` is one line that goes through the variant:
  `/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/dialectica/rust-lib/dialectica-core/src/wire.rs:567-569`
  — `error_json(&crate::authoring::Refusal::NoIdentity(why.to_string()).to_string())`

**The author's own doubt was the right doubt, and the test answers it.** Two
mutations, both run:

1. Reword `Display` only → `the_no_identity_refusal_is_the_error_shape_and_has_one_source_of_its_text`
   **passes**. Correct: with one source, both sides move together.
2. Reword `Display` *and* give `no_identity` its own `format!` carrying the
   **original** wording → the test **fails**, at `wire.rs:2825`:
   `left: "no identity is available…" right: "REWORDED: cannot sign…"`.

So a duplicate that agrees today is not invisible to this test — it is caught by
the first reword, which is exactly the drift the old arrangement could not
detect. No finding.

### Lead 2 — `docs/UI-BRIEF.md`. The wording claim is vacuous but harmless; the brief was updated, and one new line overstates what core can supply.

The premise of the lead is slightly off: the no-identity **refusal message never
appears in `UI-BRIEF.md` at all**. Greps for `no identity`, `no key was
created`, and `keystore` over
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/docs/UI-BRIEF.md`
return only `posting-capability`'s *probe* reasons at lines 312-314 ("no keystore
found; create one before posting"), which this change does not touch. So there
was no byte to compare, and no staleness created by the refusal wording. The
brief was **not** left alone: `git diff origin/main...HEAD -- docs/UI-BRIEF.md`
is +55 lines, adding the duplicate-publish obligation, the absent-parent
obligation, and the vote-control honesty block. Those three are the right
obligations and they are stated well.

One finding falls out of that addition — see **F5** below.

### Lead 3 — §10's corrected claim is honest. §11's two fresh claims: one is arguable-and-under-argued, one is **false**.

`tasks.md` §10's struck-through append-before-delivery paragraph is now correct,
and the tests it names are real and do what it says:
`the_append_completes_before_delivery_is_invoked_on_all_three_handlers`
(`wire.rs:2543-2590`) asserts a hardcoded `["append", "deliver"]` per handler
through two clones of one `Rc<RefCell<Vec<_>>>`, and
`a_refused_publish_reaches_neither_the_append_nor_delivery`
(`wire.rs:2594-2673`) asserts an empty journal across four refusals at three
depths. Both are evidence rather than argument.

Of the two new claims: the key-material one (**§11.2**) holds — see the note at
the end. The no-identity one (**§11.1**) does not hold as stated — **F2**. And
§11's closing paragraph carries a third claim of the same shape that is flatly
false — **F3**.

---

## Findings

- [x] **`dev-writer`** — **F1** — `design.md` records a signature the code does not have, and the dead end that changed it is missing from Decisions
      **Verified on conversion** (2026-09-13), all four parts: `design.md` now states
      `&mut dyn FnMut(&OpId)` and `Published { id, appended }`, carries a snippet that
      matches the real code, and records the `FnOnce` dead end in Decisions including the
      rustc-probe correction. The snippet was updated again by the panicking-sink fix, so
      it still matches — see readability R3.

**For:** `dev-writer`

**Prose:**
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/openspec/changes/authoring-content/design.md:70-74`

> "A publish returns while delivery is still outstanding" is that `deliver` is a
> `FnOnce(&OpId)` returning `()` — there is no outcome to wait for, so a call
> that waited on one cannot be written.

**Code:** `deliver` is **not** a `FnOnce`. All three handlers take
`&mut dyn FnMut(&crate::op::OpId)`:
- `.../dialectica-core/src/wire.rs:725`
- `.../dialectica-core/src/wire.rs:765`
- `.../dialectica-core/src/wire.rs:808`

and the adapter's dispatch type at
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/dialectica/rust-lib/src/lib.rs:278-283`
requires exactly that.

**Why it matters.** This is the Decisions section naming the *mechanism* that
discharges a requirement, and naming the wrong one. A reader auditing "can a
call wait on delivery?" goes to `design.md`, reads `FnOnce`, and checks a
property (at-most-once) the code does not have — while the property that
actually carries the requirement (the return type `()`) is mentioned only in
passing. `tasks.md` §8 has the correct story and `wire.rs:698-720` has it in
full; `design.md` was never updated when the signature moved.

**And the dead end is not recorded beside the decision it rules out.** The
discovery — `impl FnOnce` monomorphises per call site, the three handlers become
three types, and coercion fails on a higher-ranked lifetime against the
adapter's `for<'d> fn(…, &'d mut dyn …)` — is precisely the class of result the
review brief says is worth as much as the review, and it exists **only** in
`tasks.md` §8 and a doc comment in a file nobody reads for decisions. The next
agent who reaches for the honest bound `FnOnce` will spend the same afternoon,
because `design.md` still recommends it.

**Also wrong in the same entry, same cause:** `design.md:50` says `publish`
reports `Published { id, was_new }`. The struct is `Published { id, appended:
Appended }` with a `was_new()` accessor
(`.../dialectica-core/src/authoring.rs:59-72`). The `Appended`-not-`bool` choice
*is* the thing the later entry "passed through rather than recomputed"
(`design.md:142-152`) argues for, so stating the field as a bool here quietly
contradicts the entry two sections down. The illustrative snippet at
`design.md:64-68` also names `authoring::post(log, key, req)` and `reply_json`,
neither of which exists.

**Outcome: fixed, all four parts, in `4324364`.**

- The snippet is now the real `match` — `authoring::post(log, key, stoa, body)`,
  a bare `deliver(&published.id);`, `published_json`, and the `Err` arm — so no
  name in it is invented.
- The three requirements are re-assigned to the mechanisms that carry them:
  statement order inside the `Ok` arm, `deliver` named only on the `Ok` arm, and
  the sink's `()` return. The `let _` claim is gone; you are right that it was
  worse than stale, since a `let _` on a `()` call discards nothing.
- `Published { id, appended }` with the reason it is not flattened to a bool, and
  a pointer to the "passed through rather than recomputed" entry it was
  contradicting.
- **The dead end is now in Decisions**, which was the substantive half of this
  finding: why `impl FnOnce` cannot coexist with pinning one function-pointer
  type, and — per architecture A1, which disproved the original reasoning with a
  rustc probe — that the *adapter* does not force the erasure, so recovering
  `FnOnce` costs one test's `Handler` type. Your framing that the next agent
  would "spend the same afternoon" is exactly what that entry is there to prevent.

---

- [x] **`dev-writer`** — **F2's documentation half** — §11.1's "untestable as specified" is a claim about the chosen design, not about the capability
      **Verified on conversion** (2026-09-13): `tasks.md` §11.1 now names the old
      "structurally cannot" reason as wrong, cites `get_capabilities`'s `lookup` closure
      and `capability_for` tested against a *failing* lookup as the counter-example, and
      states the narrower honest claim. The cost is recorded beside the benefit with the
      closure alternative named as not-taken, and the section says what the spec should
      not be told.

- [ ] **`spec-writer`** + **`dev-writer`** — **F2's routing question** — whether to close the no-identity gap in code
      **Still open, and deliberately so** (2026-09-13): swapping `&SecretKey` for a
      fallible key-supplier would trade away the structural "a publish creates no key
      material" property, which is a decision to take on purpose with the `spec-writer`
      rather than as a review fix. `tasks.md` §11.1 sets out the three routes. The code is
      unchanged — handlers still take `&crate::identity::SecretKey` — so the requirement
      remains undischarged in core, which is what the unticked box records.

      **Still open after the `spec-writer` pass of 2026-09-13, and this is the one box I
      am leaving unticked on purpose.** I could have closed it unilaterally by taking
      route 2 or 3 — both are spec-only edits I own outright — and that would have been
      the wrong move, because either one narrows or relocates the contract in order to
      make an unticked box go away, which is the cost hidden in doing it alone. What a
      decision costs, having verified each route rather than reading it off §11.1:

      - **Route 1 (fallible key-supplier in core)** is the only route that discharges the
        requirement as written, and it is **not mine to take** — it changes three handler
        signatures. It also trades a real property for a testable one:
        `a_refused_publish_creates_no_key_material` is currently true *structurally* —
        `authoring` cannot create key material because it never holds anything that could
        — and a key-supplier closure replaces "cannot" with "does not, and here is a test
        saying so". Confirmed the shape exists and works: `get_capabilities` takes
        `lookup: impl Fn(&Address) -> Result<String, KeystoreError>` and `capability_for`
        is tested against a *failing* lookup, so F2's counter-example to the old
        "structurally cannot" claim holds. The gap is a consequence of the chosen shape,
        exactly as this finding says.
      - **Route 2 (scope the requirement to the wire shape)** costs the trigger. The spec
        would then contract the *wording and shape* of a refusal that only the adapter can
        raise, and nothing would require that a publish with no identity be refused at
        all — a module built without the guard would satisfy the narrowed requirement. That
        is a weaker contract bought with a tickable box, and this repo has just spent a
        review pass on the converse mistake (a requirement satisfied by defective
        behaviour — see `findings/correctness.md` C1's closing sentence).
      - **Route 3 (move it to `posting-capability` or `keystore`)** has the strongest claim
        of the three and is still not free. I checked: `posting-capability` already owns
        eight requirements about exactly this question, including "The answer carries an
        identity or a reason, never both and never neither" and "The reason names the fix".
        So the generality is arguably demonstrated. But an extraction is `ADDED` in one
        capability and `REMOVED` in the other **with Reason and Migration, in one change**,
        and it must move the requirement text verbatim — which would put a cross-capability
        spec reorganisation inside a change whose six reviews are already complete and
        whose other five spec edits are behaviour ratifications. The two should not ride
        together; neither half would be reviewable.

      **What I did instead: nothing to this requirement.** It stands as written, undischarged
      in core, with `tasks.md` §11.1's honest narrower claim in front of whoever takes it.
      That is the state a reader can act on; a ticked box with route 2 applied would not be.

      **My recommendation, for whoever decides** — route 3, and as its own change after this
      one merges, with route 1 considered on its merits separately rather than as the price
      of testability. Route 3 is the only one that neither weakens the contract nor trades a
      structural security property; it relocates a requirement to the capability that
      already answers the same question one step earlier. It needs a `spec-writer` with
      both capabilities in scope, which this piece is not.

**For:** `dev-writer` (and `spec-writer` for the routing question §11.1 already raises)

**Prose:**
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/openspec/changes/authoring-content/tasks.md:311-318`

> ### "A publish requires a usable identity and says so when there is none" — untestable as specified
>
> … The requirement's subject is *the absence of an identity*, and discovering
> that absence means reaching a keystore. `dialectica-core` structurally cannot
> — that is the design's whole point … so the refusal can only be raised in the
> adapter, which no `cargo test` compiles. **There is no test in this repo, as
> the spec is worded, that can discharge it.**

**Code that contradicts the reasoning:** the sibling probe on the *same
question* already has the seam.
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/dialectica/rust-lib/dialectica-core/src/wire.rs:201-243`:

```
pub fn get_capabilities(
    request: &str,
    lookup: impl Fn(&crate::identity::Address) -> Result<String, crate::keystore::KeystoreError>,
) -> String
```

`capability_for` (`wire.rs:228-243`) is then tested directly against a fake
lookup that returns `KeystoreError`s — `a_successful_probe_carries_no_reason_and_a_failed_one_no_identity`
at `wire.rs:1297` is one of them. `dialectica-core` does not reach a keystore
there either; **the adapter passes a closure that does**, and core is tested
against a closure that fails.

The publish handlers deliberately do not take that shape: they take a
`&crate::identity::SecretKey` already resolved
(`wire.rs:724`, `wire.rs:764`, `wire.rs:807`), so the keystore open happens in
`src/lib.rs:320-324`, above core, in the file no gate compiles.

**Why it matters.** "No test can see this" is load-bearing — §10 says so in as
many words, about the last time this change made the same mistake: *"A note
saying a property is unobservable is a note that stops anyone looking for a way
to observe it, so it has to be right."* Here the note is not right. The honest
statement is narrower: *given that `authoring` takes a resolved `&SecretKey`,
the trigger is raised above core and no `cargo test` reaches it* — and the
alternative that removes the gap is the `lookup`-closure shape used fifteen
hundred lines up the same file, which would let a fake key-supplier fail and
discharge both scenarios in core.

Taking `&SecretKey` may still be the right call — it is what makes "a publish
creates no key material as a side effect" structural, and `design.md:27-33`
argues that half well. But `design.md` records the benefit and **not the cost**,
and §11 then presents the cost as a property of the spec rather than of the
choice. A Decisions entry has to name what it forecloses; this one forecloses
testing the no-identity trigger in core, and neither document says so or names
the closure alternative as considered-and-rejected.

**Outcome: fixed** in `4324364`. This is the finding I'd have most regretted
missing, because my own brief relayed §11.1 as settled and told the reviewer not
to re-litigate it.

`tasks.md` §11.1 is rewritten. It now says the old "structurally cannot" reason
was **wrong** and names it as the same shape as the §10 claim this change already
retracted; cites `get_capabilities`'s `lookup` closure and `capability_for` being
tested against a *failing* lookup as the counter-example; and states the honest
narrower claim — because `authoring` takes an already-resolved `&SecretKey`, the
keystore open happens above core in the file no `cargo test` compiles.

The cost is now recorded beside the benefit, with the closure alternative named as
not-taken and what it would trade (the structural no-key-material property). The
spec-writer gets **three** routes instead of two, and the new first one is "close
the gap in code" — which is the only route that discharges the requirement as
written, and is as much a `dev-writer` call as a spec one. The section ends by
saying what the spec should *not* be told: that it asks for something
unobservable. It does not; this change chose a shape that cannot observe it.

I have deliberately not made the code change. Swapping `&SecretKey` for a fallible
key-supplier trades away a structural security property, which is a decision to
take on purpose with the spec-writer rather than as a review fix.

---

- [x] **`dev-writer`** — **F3** — §11's claim that the spec does not choose `address` is false; `spec.md` chooses it explicitly
      **Verified on conversion** (2026-09-13), in both places it was said: `tasks.md` §11
      now quotes the spec's "or an address" and records the scenario-versus-requirement
      misread, and the `NO SPEC:` marker names `author`/`identity`/`key`/`address` as
      specified while marking only `thread` as the chosen one.

**For:** `dev-writer`

**Prose:**
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/openspec/changes/authoring-content/tasks.md:356-360`

> Refusing `thread` on a post and a vote, and **including `address` alongside
> `author`/`identity`/`key`, are choices the spec does not make.** They are for
> the spec-writer to ratify or overturn.

**Spec:**
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/openspec/changes/authoring-content/specs/content-authoring/spec.md:86`

> No publish operation SHALL accept an author, an identity, a key, **or an
> address** as a parameter.

**Code:** `address` is in `FORBIDDEN_FIELDS` at
`.../dialectica-core/src/wire.rs:601-604`, which the spec requirement above
directly authorises.

**Why it matters.** It sends the spec-writer to ratify something already
ratified in the very requirement the field implements, and — worse — it is the
same shape as the §10 defect this change already had to correct: a confident
statement about what a document says, made without re-reading the document. The
`thread`-on-a-post half of the sentence is true (the spec's reply requirement at
`spec.md:191-192` covers a reply only) and is correctly marked NO SPEC in the
code at `wire.rs:584-587`; the `address` half is not. `tasks.md:204-206` gets
closer — "the spec requires 'a field that names an author, an identity or a key'
to be refused" — but that is the *scenario* text at `spec.md:105-106`, not the
requirement text at `spec.md:86`, and the requirement is the contract.

**Outcome: fixed** in `4324364`, in both places it was said.

`tasks.md` §11's closing section now says "one choice, not two": `thread` on a post
or a vote is the unspecified one, and `address` is **not** — quoting the
requirement, and recording that the wrong claim was read off the scenario one
screen down while the requirement is the contract. The `NO SPEC:` marker on
`a_forbidden_field_is_refused_on_every_operation` said the same thing by omitting
`address` from the list it attributed to the spec; it now names all four specified
fields and marks only `thread` as the choice.

Your diagnosis of *why* is the part worth keeping: a confident statement about what
a document says, made without re-reading the document. That is the fourth instance
of this family on this branch, and the reason my own brief passed it on as an open
question for the spec-writer — it would have sent them to ratify something the
requirement they were reading already said.

---

- [x] **`dev-writer`** — **F4 (a), (b), (d)** — decisions the code took that no document records
      **Verified on conversion** (2026-09-13): (a) the `unordered` arrival has its own
      Decisions entry naming the three constructors and the sort-after cost; (b) is folded
      into the thread entry, noting `reply` refuses a non-post parent first and that
      `thread_of` has one caller, with the unrepresentable-arm reshape deferred alongside
      A2/A3; (d) the double-`stoa`-read is now a named decision rather than only a code
      comment.

- [x] **`spec-writer`** — **F4 (c)** — no requirement owns `Refusal::Storage`
      **Still open, verified on conversion** (2026-09-13): `design.md` has the
      `Refusal::Storage` entry and routes the question, but grepping the spec delta for
      storage finds **nothing** — no requirement says a storage failure is a
      distinguishable refusal rather than an absent parent. The behaviour is implemented
      and tested; it is simply uncontracted.

      **Fixed, in the shape you proposed.** Confirmed the gap independently before writing
      (grep for `storage`/`store`/`unreadable` over the delta: nothing), and confirmed the
      behaviour exists to ratify — `Refusal::Storage(OpLogError)` at `authoring.rs:127`,
      the `From<OpLogError>` at `:172` routing every `?`, `Display` at `:165` forwarding
      the store's own reason, and `a_store_that_cannot_be_read_is_a_refusal_and_not_an_absent_parent`
      at `:1560` covering post, reply and vote.

      New requirement, *"A publish that cannot reach the store is refused
      distinguishably"*:

      > A publish that fails because the store cannot be read or written SHALL be refused,
      > and the refusal SHALL be distinguishable from a refusal for a parent or target the
      > peer does not hold. The underlying reason the store gave SHALL survive into the
      > message.

      Your suggested wording was *"refused distinguishably from one whose parent or target
      is not held"*, which I kept. Two additions, both from things in your entry rather
      than from me:

      - **"The underlying reason the store gave SHALL survive into the message"**, because
        the test you cite asserts exactly that (`"the disk is on fire"` survives) and it
        was otherwise uncontracted — a `Display` that dropped the inner error would still
        have satisfied the distinguishability clause alone.
      - **"A storage failure SHALL NOT be reported as a success"**, which is the
        write-path half of the read-path rule you name. Your framing — *opposite user
        responses, wait versus fix the disk* — is the requirement's rationale, stated in
        one clause.

      Two scenarios: all three operations against a store whose reads and writes fail,
      each distinguishable from the not-held refusal and each carrying the store's reason;
      and a reply naming a parent against a failing store, which must not state that the
      parent is not held.

      **Your point about the PLAN citation is why this needed a spec home and not just a
      comment.** `authoring.rs:1561` still opens "§11.1 obligation 5 at the write path" —
      a pointer a spec reader does not have. I have not edited that comment, since code is
      not mine, but the requirement it was standing in for now exists in a document that
      archives with the change. Left for the `dev-writer` or a later readability pass:
      that comment could now cite the requirement by name instead.

**For:** `dev-writer` for a, b and d; `spec-writer` for c.

None of these appear in `design.md`'s Decisions, and each has a real alternative
a reader would plausibly have taken.

**(a) `Arrival::unordered()` on a locally-published op.**
`.../dialectica-core/src/authoring.rs:169-173`. `Arrival` offers three
constructors — `ordered(lamport, message_id)`, `unordered()`, and `from_parts`
(`.../dialectica-core/src/arrival.rs:134-165`) — so this is a genuine choice,
and `arrival.rs:176-186` makes its consequence explicit: an op with no Lamport
value **sorts after every transport-ordered op**, in the degraded ascending-op-id
block. So a user's own just-published post takes no position advantage in any
ordering, permanently, by this line. That is a decision with a cost worth
stating. Recorded only in `tasks.md:45-47` as a ticked bullet and in a code
comment; `design.md`, `spec.md` and `proposal.md` never mention `Arrival` or
`unordered`.

**(b) `thread_of`'s non-post arm returns the op's own id.**
`.../dialectica-core/src/authoring.rs:243-246`. The comment argues it ("an
answer rather than a panic, because a panic aborts the module process"), which by
the review brief's own test — *anything a comment justifies at length was a
decision* — puts it in Decisions. The alternatives are a panic, an
`Option<OpId>`, or a type that makes the arm unreachable (taking the already-
matched `Post` fields rather than `&Op`). The last is the "make the mistake
unrepresentable" move this project prefers and is not mentioned. `design.md`'s
thread entry (`design.md:97-127`) covers only the `Post` arm.

**(c) A store failure on the publish path is a distinguishable refusal — and the spec is silent.**
`.../dialectica-core/src/authoring.rs:110-112` (`Refusal::Storage`),
`authoring.rs:149-153` (the `From<OpLogError>` that routes every `?`), and the
test `a_store_that_cannot_be_read_is_a_refusal_and_not_an_absent_parent` at
`authoring.rs:1358-1408`, which asserts a broken store is never reported as an
absent parent and that the underlying reason survives. This is observable
behaviour with opposite user responses — *wait* versus *fix the disk* — and it is
the write-path twin of the `list_threads` rule that a storage failure is never an
empty page. Greps for `store`/`storage`/`unreadable` over
`openspec/changes/authoring-content/specs/content-authoring/spec.md` and over
`openspec/specs/module-wire-contract/spec.md` find no requirement owning it. The
code's justification cites "§11.1 obligation 5" — a PLAN section — which is
precisely the pointer a spec reader does not have. **For the spec-writer:** this
wants a requirement, in the shape of "a publish that cannot reach the store SHALL
be refused distinguishably from one whose parent or target is not held."

**(d) The adapter reads `stoa` twice.**
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/dialectica/rust-lib/src/lib.rs:296-307`
parses the request and the Stoa to derive a key, then the handler re-parses both
(`wire.rs:728-738`). The code comment at `lib.rs:254-262` argues it, and
`tasks.md:281` lists it as unchecked-by-any-gate — but it is a decision, with the
alternative (thread the parsed `Address` in from the adapter) named in the
comment and nowhere in Decisions. It matters because the double-read is the one
place a request is interpreted twice by two parsers that could disagree, in the
file no gate compiles.

**Outcome: all four now in `design.md` Decisions.** Your test for what belongs
there — *anything a comment justifies at length was a decision* — is the right one
and is what I used.

- **(a) `Arrival::unordered()`** — its own entry, with the reason (claiming a
  Lamport value for an op that did not arrive is self-asserted ordering, which
  CLAUDE.md's SDS section forbids) and, more importantly, **the cost you
  identified**: the op sorts after every transport-ordered op, so a user's own
  just-published post takes no position advantage in any ordering until it returns
  through delivery with real metadata. That is the half a reader needs and no
  document had.
- **(b) `thread_of`'s non-post arm** — folded into the existing thread entry rather
  than given its own, since it is the same derivation. All three alternatives you
  named are recorded with why not, including that taking the already-matched `Post`
  fields would make the arm unrepresentable and is the better shape — deferred with
  the A2/A3 reshape because it moves a guard.
- **(c) `Refusal::Storage`** — its own entry, stating that it is the write-path twin
  of `list_threads`'s never-an-empty-page rule, and **that the spec does not require
  it**. Routed to the `spec-writer` with your suggested requirement shape. The code
  comment's "§11.1 obligation 5" PLAN citation is exactly the pointer a spec reader
  does not have, which is why this needed a home in a document that gets archived
  with the change.
- **(d) the double `stoa` read** — its own entry, with the rejected alternative and
  the cost named: one field interpreted by two parsers, either of which may produce
  the error a caller sees, with the handler's parse identified as the authority and
  the adapter's as a key-derivation lookup.

---

- [x] **`dev-writer`** — **F5** — `UI-BRIEF.md` tells the designer a vote count is safe to show; nothing in the wire API can supply one
      **Verified on conversion** (2026-09-13): the bullet now reads "**Not available yet,
      though it would be safe:**", and no wire method returns a count — `publish_vote`
      replies with an op id and `wasNew` only, and its doc comment says so. The live
      document no longer promises the designer a field the API cannot fill.

**For:** `dev-writer`

**Prose:**
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/docs/UI-BRIEF.md:544-546`, added by this change:

> - **Safe:** a plain count of votes on a post, if you show one, presented as a
>   count and never as a position, a rank, or a reason this post appears where it
>   does.

**Code:** the wire surface after this change is `error_json`, `version`, `ping`,
`panic_probe`, `get_capabilities`, `capability_for`, `list_threads`,
`list_threads_from_request`, `no_identity`, `publish_post`, `publish_reply`,
`publish_vote`, `parse_channel_id`, `callee_error`, `channel_exists_reply`
(`.../dialectica-core/src/wire.rs`, `pub fn` sweep). **None returns a vote
count, and there is no thread-read method at all** — `OpLog::iter_target` exists
but is reachable only from inside core. `spec.md:315-353` requires the vote reply
to carry an op id "and nothing that describes an effect", and `design.md:176-177`
says "No score, count, tally or on a vote reply."

**Why it matters.** CLAUDE.md's rule is that the brief legitimately states
obligations the core does not yet meet — so a count is a fair thing to ask for.
But the sentence is written as a permission ("safe … if you show one"), not as a
future obligation, and it sits inside the very block whose job is to stop the
designer claiming something the core does not do. A designer reading it lays out
a count beside every post, and the first implementer discovers there is no call
that returns one. The same block's other two bullets are explicit about what is
and is not real; this one is not. One clause — that no read path exposes votes
yet, so a count is a later obligation — restores it.

**Outcome: fixed** in `4324364`, essentially as you prescribed. The bullet is now
labelled **"Not available yet, though it would be safe"**, says no call returns a
count (there is no thread read at all, and the publish reply carries only an op
id), instructs the designer to design for its absence, and keeps the
count-not-a-rank condition for whenever a later change exposes one. The block's
lead-in, which said "two things are safe and one is not", now reads "one thing is
safe, one is not available yet, and one is never safe" — otherwise the correction
would have left the enumeration wrong, which is the failure mode this brief is
most prone to.

Worth recording that your re-framing of lead 2 was the useful part: I had told you
to compare the refusal wording byte-for-byte against the brief, and the wording
never appears in the brief at all — so there was no byte to compare, and the real
defect was in the +55 lines the change *added*. A lead that sends a reviewer to the
wrong place still found the right thing because you checked the premise first.

---

- [x] **`dev-writer`** — **F6** — A publish whose delivery panics answers with the error shape, contradicting the spec scenario
      **Fixed** (2026-09-13). Same defect as `findings/spec-test.md` entry 1, found
      independently by two reviewers; both close together. The owner settled it: catch the
      panic at the handoff and report the publish as successful. **The requirement was
      right and the code was wrong** — so of the three routes offered, the outcome matches
      (3) only in that the behaviour was kept for the op, and not in its reasoning: the
      reply carries no fault at all, because there is no fault to report about a published
      op.
      Route (1), moving `deliver` outside `guarded`, is rejected on the PHASE0-FINDINGS §3
      measurement — an unguarded panic aborts the module process, costs the caller a
      20-second timeout, and makes every later call report `MODULE_NOT_LOADED`.
      Route (2), scoping the requirement to a sink that returns rather than unwinds, is
      ruled out by `delivery_module.lidl`: the outcome arrives asynchronously, so a
      synchronous reply could never carry it, and the distinction would have contracted
      nothing informative. **This is the point where the reviewer's sharpest argument —
      "nothing in this API distinguishes declined from panicked" — turned out to argue for
      the fix rather than against it**: if neither can be reported synchronously, neither
      belongs in the reply.
      Implemented as `wire::delivered_and_published`; pinned by
      `a_panicking_delivery_sink_still_reports_the_op_as_published_on_all_three_handlers`,
      which fails without the fix. Reasoning is in `design.md` Decisions and
      `docs/PLAN.md` §9.2, which carries the obligation handed to `op-transport`. The
      documents that described this as an open question — `tasks.md` §11 and the sibling
      test's comment — were updated in the same change.

**For:** `spec-writer` (the requirement is the thing to settle), cc `dev-writer`

**Spec:**
`/home/fryorcraken/src/rad/dialectica/.claude/worktrees/rev-auth-design/openspec/changes/authoring-content/specs/content-authoring/spec.md:37-39` and `53-57`:

> Where delivery declines the handoff, the op SHALL remain in the log and the
> reply SHALL report the op as published. **A publish SHALL NOT be reported as
> having failed on the strength of a delivery outcome** …
>
> #### Scenario: A declined handoff leaves the op published
> - **WHEN** delivery **refuses or errors** on the handoff
> - **THEN** the reply reports the op as published and names its op id

**Code:** `deliver(&published.id)` sits **inside** the `guarded(…)` closure in
all three handlers — `.../dialectica-core/src/wire.rs:727-751` (post), `767-795`
(reply), `810-838` (vote). So a sink that panics is caught by `guarded` and the
reply becomes `{"error":"panic in publish_post: …"}`, with no `opId`, for an op
that **is** in the log. The change's own test confirms this and says so in its
comment:
`.../dialectica-core/src/wire.rs:2705-2707`

> // The guard caught it, so this is an error shape rather than an aborted
> // process — but the op is published either way.
> as_json(&out);

`as_json(&out)` is the only thing asserted about the reply — the test never
checks for `opId`, and its **name**,
`a_publish_whose_delivery_panics_still_reports_the_op_as_published`, asserts in
English exactly the thing it does not assert in code. `tasks.md` cites that test
twice (lines 138 and the §10 table) and never notes the discrepancy.

**Why it matters.** A panicking sink is the strongest form of "delivery errors on
the handoff", which is the scenario's own wording — so either the spec means
something narrower than it says, or the code is wrong. Both are fixable; what is
not acceptable is that neither document records a choice here. Three routes, and
the spec-writer owns picking one: (i) scope the requirement to a sink that
*returns* rather than unwinds, and say so; (ii) move the `deliver` call outside
`guarded` so a sink panic cannot overwrite a success reply; (iii) keep the
behaviour and contract it — a delivery panic is an infrastructure fault reported
as one, with the op still published — and state it, because a caller that reads
`{"error":…}` and retries publishes nothing new and is told so by `wasNew`, which
is a confusing path to leave unspecified. Whichever is chosen, the test's name
must stop claiming the reply reports the op as published.

**Outcome: the misleading name fixed; the contradiction itself routed to the
`spec-writer`, undecided.** This is the most serious finding of the six reviews and
I am deliberately not resolving it as a review fix.

Done now:

- The test is renamed `a_publish_whose_delivery_panics_leaves_the_op_in_the_log` —
  what it actually pins. Your point that the name "asserts in English exactly the
  thing it does not assert in code" is the defect; the rename removes the false
  claim without pretending to add the coverage.
- Its comment now states the contradiction outright: `guarded` wraps `deliver`, so
  a panicking sink yields the error shape with no `opId` for an op that is in the
  log, and either the spec means something narrower than "refuses or errors" or the
  call belongs outside the guard. It points at this finding and at spec-test entry 1
  rather than restating the requirement at itself, which is what the old comment did.
- **`tasks.md` §11 gains it as a third undischarged requirement**, with your three
  routes, the measurement (adding the two assertions gives 530 passed / 1 failed),
  and the user-visible cost spelled out: the view shows "posting failed", the user
  retypes and resubmits, and the op is already in the log — told the opposite of the
  truth, which is what the requirement's second paragraph exists to prevent. §11's
  framing now says the three are different *in kind*, this one being a contradiction
  someone must resolve rather than a gap.
- The mutation table's reference to the old test name is updated with it.

Not done: choosing a route. (i) and (iii) are spec edits; (ii) changes behaviour and
needs care that a sink panic still cannot abort the module process, which is what
`guarded` is there for. Picking one on the spec-writer's behalf would be the
"neither document records a choice" failure in a new form.

---

- [x] **`dev-writer`** — **F7** — `docs/PLAN.md` shed the behaviour correctly, but kept a fresh copy of reasoning that now also lives in `proposal.md`
      **Verified on conversion** (2026-09-13): split along the line the reviewer drew.
      `design.md` carries the declination reasoning as its own content and says so in as
      many words ("this is the one place that reasoning lives"); PLAN keeps the
      forward-looking half. No second copy of the reasoning survives to drift.

**For:** `dev-writer`

The shedding is mostly exemplary: `PLAN.md`'s Stage B method list, the
`createPost`/`createReply` sketches, the three shared properties, and the
"methods deliberately NOT proposed" `vote` entry are struck through with one-line
summaries pointing at the spec (diff at `docs/PLAN.md`, §9.1 Stage B and the API
sketch). The `createReply`-takes-no-`thread` departure is flagged in PLAN
precisely because PLAN sketched it the other way, which is the right call. No
requirement in `openspec/changes/authoring-content/specs/content-authoring/spec.md`
cites a PLAN section number — a grep for `§` over the change's four documents
returns nothing — which is an improvement on the two existing specs that do
(`openspec/specs/moderation-resolution/spec.md:211`,
`openspec/specs/stoa-genesis/spec.md:120`).

The exception is the `createdAt` reasoning. This change **acted on** it — it
considered the field, declined it, and contracted the consequence — so by the
convention the reasoning moves out of PLAN. Instead the change **added** 36 new
lines of it to PLAN's open questions (`docs/PLAN.md`, the new
"Should an op carry an author-asserted `createdAt`" bullet), carrying the same
three declination reasons already argued at
`.../openspec/changes/authoring-content/proposal.md:76-103`: the `op-format`
`MODIFIED`, the unspecified clamp, and "a change that adds an authoring API and
re-versions the op format cannot be reviewed for either". Two copies, and they
are already not identical — PLAN's version adds the nonce-as-narrower-alternative
argument that `proposal.md` does not have, and `proposal.md` has the
nearest-comparable-project measurement that PLAN's compresses to a parenthesis.

**Why it matters.** This is the documented failure mode: two copies drift and the
wrong one gets read. The forward-looking half legitimately belongs in PLAN —
*what would decide it*, and that whoever adds the field must modify the
`content-authoring` requirement. The retrospective half — why **this** change
declined it — belongs in `design.md` under Decisions, where it currently does not
appear at all: `design.md:173-176` only says "the proposal argues this at length",
which is a pointer to a change document that gets archived, from the document that
does not.

**Outcome: fixed**, split exactly along the line you drew.

- `design.md` now carries the three declination reasons as its own Decisions
  content rather than pointing at `proposal.md`: the `op-format` `MODIFIED` reaching
  two capabilities this change does not own, the unspecified clamp being the whole
  defence (with the far-future-timestamp consequence stated), and the
  cannot-review-both argument. It ends by saying what PLAN keeps and what it does
  not, so the split is self-describing.
- `docs/PLAN.md`'s "Why it was declined there rather than taken" paragraph is
  replaced by one sentence pointing at that `design.md` entry plus a two-clause
  summary, and says in line why it is not copied. PLAN keeps the forward-looking
  half — the two symptoms, the nonce-as-narrower-alternative, and what would decide
  it — which is the half that is about work not yet done.

Your observation that the two copies *already differed* is what made this worth
doing rather than tidying: PLAN had the nonce argument and `proposal.md` had the
comparable-project measurement. Drift had started before either was read twice.

Note the residual asymmetry, deliberately left: `proposal.md` still argues the
declination at length. That is correct — a proposal records why the change was
shaped as it was, and it is archived alongside `design.md`, so the two travel
together. The defect was PLAN holding a third copy that outlives both.

---

## Claims checked that hold

Stated with evidence, so the next reader does not redo them.

- **§11.2, "a refused publish creates no key material" is satisfied
  structurally.** Verified. `authoring::post`/`reply`/`vote` take
  `&mut L: OpLog` and `&SecretKey` and nothing else
  (`.../dialectica-core/src/authoring.rs:187-192`, `267-273`, `331-337`); the
  whole module imports only `arrival`, `identity`, `log` and `op`
  (`authoring.rs:44-47`) and touches no filesystem path. The adapter's only
  keystore call is `open_from_env` (`src/lib.rs:321`) — never `create`, which
  exists at `keystore.rs:683` and is called nowhere in the publish path. §11.2's
  reasoning that recording this as coverage would make it weaker is sound, and
  its named residual risk (the adapter growing a `generate`/`create` call being a
  review question about `src/lib.rs`) is the correct residual risk.
- **The `op.rs` doc-comment correction is real, and the read side already agreed
  with the corrected version.** `.../dialectica-core/src/op.rs:300-317` now says
  nothing fills `thread` in and explains why it cannot. `feed.rs:251` sets a
  row's `thread` to the root's own `id` for `parent: None` posts, so the read
  side was already deriving rather than reading a store-filled field — the
  comment was wrong, not the code, exactly as `design.md:122-127` says.
- **The thread derivation matches the recorded decision.**
  `thread_of` is `thread.unwrap_or(id)` (`authoring.rs:239-247`), `reply` calls
  it on the parent with the parent's id (`authoring.rs:294`), and no caller can
  supply a thread — `thread` is in `FORBIDDEN_FIELDS` (`wire.rs:605-608`) and
  `publish_reply` parses only `stoa`, `parent`, `body` (`wire.rs:775-786`).
- **`Refusal` is a typed enum and tests assert on the variant, with the wording
  pinned once.** `authoring.rs:83-112`; `a_refusal_names_the_id_or_stoa_it_is_about`
  at `authoring.rs:1152-1183` is the single wording pin, and every other refusal
  assertion compares variants (e.g. `authoring.rs:863-869`, `893-901`, `951-958`).
  This matches `design.md:86-96`.
- **One `reject_forbidden_fields` over a union list, called from all three.**
  `wire.rs:617-624`, called at `wire.rs:732`, `772`, `815`. The guard is its own
  function, so "is it called everywhere?" has an answer — `design.md:161-165`
  and CLAUDE.md's rule, applied.
- **Publishing takes no genesis record.** None of the three `authoring` entry
  points takes one, and `Genesis` appears in `authoring.rs` only inside `#[cfg(test)]`
  fixtures (`authoring.rs:373-383`, `1122-1129`). `design.md:129-140`'s honest
  consequence — no posting-policy enforcement — is stated and is true.
- **`Appended` is passed through, not recomputed.** `publish` returns the value
  `log.append` gave it (`authoring.rs:173-174`); there is no `get`-before-`append`
  anywhere in the three paths. `design.md:142-152` holds.
- **The spec carries rationale, and that is this project's convention rather than
  a defect.** The two existing specs do the same
  (`openspec/specs/moderation-resolution/spec.md:211`,
  `openspec/specs/stoa-genesis/spec.md:120`), so the prose in
  `content-authoring/spec.md` is not a finding. What would have been one — a
  PLAN section number inside a requirement — is absent.
