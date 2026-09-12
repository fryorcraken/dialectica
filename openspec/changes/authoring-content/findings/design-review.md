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

### F1 — `design.md` records a signature the code does not have, and the dead end that changed it is missing from Decisions. (serious: code contradicts a recorded decision)

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

**Outcome:**

---

### F2 — §11.1's "untestable as specified" is a claim about the chosen design, not about the capability, and the seam that would make it testable is the one the sibling handler already uses. (serious: the exact shape of the error this change's history proves)

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

**Outcome:**

---

### F3 — §11's claim that the spec does not choose `address` is false; `spec.md` chooses it explicitly. (serious: a false claim licensing rework)

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

**Outcome:**

---

### F4 — Four decisions the code took that no document records. (gap)

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

**Outcome:**

---

### F5 — `UI-BRIEF.md` tells the designer a vote count is safe to show; nothing in the wire API can supply one. (moderate: a live document designed against)

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

**Outcome:**

---

### F6 — A publish whose delivery panics answers with the error shape, contradicting the spec scenario, and the departure is recorded nowhere. (serious: the code contradicts the contract, and the test's name hides it)

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

**Outcome:**

---

### F7 — `docs/PLAN.md` shed the behaviour correctly, but kept a fresh copy of reasoning that now also lives in `proposal.md`. (moderate)

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

**Outcome:**

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
