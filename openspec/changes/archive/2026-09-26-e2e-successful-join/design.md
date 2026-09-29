# Design — a successful join, end to end, with a committed seeded reference

## Context

See proposal.md for why this piece exists and what the specification must
observe. The harness is the archived `e2e-ui-suite` change's, as corrected by
`e2e-suite-review` and extended by `e2e-created-stoa-flow`: `lgs basecamp
setup` and `install` build and populate a profile, sitometres 0.1.2 launches
Basecamp against it, and the adjudicator requires a passing verdict, every step
passed, and the report's step count equal to the spec's. Nothing about that
harness changes here except the matrix.

**The owner's decision on #134 is what makes this piece 0.0.1 work, and what
lets its PR close the issue.** The comment on the issue, 2026-09-25: *"the full
end-to-end suite stays in 0.0.1. The owner wants good testing foundations in
this release. PR #164 landed the join-refusal spec, and that is not enough to
close this issue. Still required in 0.0.1: a successful join, which needs a
seeding peer; the feed screen; and the thread screen, each run end to end
against an `lgs`-installed Basecamp. Splitting the remainder out to 0.0.2 was
considered and rejected."* #181 delivered the feed and the thread, with key and
Stoa creation and the moderation screen; this piece is the successful join, the
last item. The owner's second comment on #134 reopened the issue after #181's
body closed it by accident, and says of this piece: *"The successful join is
the last piece and its PR will close this."*

Three facts about sitometres shape D1, each read at the pinned tag `v0.1.2`
(`6dc23e2`):

- **A spec is read as a file and nothing else.** `src/commands/run.ts`,
  `loadSpec`: `fs.readFileSync`, then `YAML.parse`, then `validateSpec`. The
  schema (`src/spec/schema.ts`) has no variable, include or substitution key,
  and `type.text` is a plain string.
- **No flag supplies a value to a spec.** `src/cli.ts`'s `run` flags are the
  boot flags plus `--json`, `--junit`, `--artifacts`, `--strict`, `--debug` and
  `--breakpoint`. `--env K=V` sets a variable in the APP's environment, and the
  QML view cannot read its environment, because Basecamp sandboxes the engine.
- **`eval:` and `set:` run in the app**, so they can move a value that is
  already in the spec, and cannot fetch one that is not.

So a reference generated per run can reach a spec only by rewriting the spec
file inside the job before sitometres reads it.

## Goals / Non-Goals

**Goals:** the successful-join specification green in CI on this piece's tip,
seen red in CI for the reason it exists, with both runs recorded; a seeded
reference whose validity against the current core is checked on every PR, in
seconds, in the workflow that runs first.

**Non-Goals:** a second Basecamp profile or any peer-to-peer delivery (#176);
any change to core behaviour or to the wire contract; new root handles. The
view gains one `objectName` and nothing else.

## Decisions

### D1 — The reference is committed, not generated per run

**Chosen:** the reference is a literal in `seeded-join.yaml`, written by the
seeder once and checked against the core on every PR by
`dialectica-core/tests/seeded_reference.rs` (D2).

The proposal set out constraints for each side. Answered in turn:

**A committed reference.**

- *Its creator key cannot be arbitrary bytes.* It is the public half of a fixed
  32-byte Ed25519 seed (`CREATOR_SEED`, `[0x5e; 32]`). Every 32-byte string is
  a valid seed, and the public key of a real secret decodes and is not one of
  the eight low-order points `PublicKey::from_bytes` refuses. The secret is
  public by construction, which is harmless for a fixture Stoa nobody posts in;
  the test file says so where it defines it.
- *Its record is fixed at the encoding it was written under*, genesis version
  1. That is the property wanted, not a cost of it: `stoa-navigation-view`
  makes the reference encoding a compatibility surface, and a committed
  reference is a reference in the wild that the suite holds on to.
- *What catches the drift first, and in which workflow.* `ci.yml`'s `rust` job,
  on the PR that changes the encoding, by `seeded_reference.rs`. It decodes the
  committed bytes and sends the committed reference through `wire::get_stoa`
  and `wire::join_stoa`, the handlers the view's preview and join reach. That
  job runs in seconds after its cache warms. `ui-tests.yml` would go red too,
  minutes later, at the preview step.

  **It is not a second copy of the known-answer test**, and the difference is
  the point. `stoa.rs`'s `the_wire_format_is_pinned_to_a_known_answer` pins
  what the current ENCODER writes. A deliberate version bump updates its
  constants and it goes green again. `seeded_reference.rs` pins that a
  reference written EARLIER still joins, and consults none of those constants,
  so updating them leaves it red. Measured with `VERSION_1` set to 2: the
  known-answer test failed on "the genesis encoding version changed", and all
  three seeded tests failed on "unknown genesis record version 1".

**A generated reference, rejected.**

- *Getting it into the spec.* Per Context, sitometres 0.1.2 cannot take a value
  from outside the spec, so the job would rewrite `$SPEC` before the run. The
  three readers of the committed file would then disagree about what the file
  is. The `ui-specs` validator (`validate-ui-specs.mjs`) and the matrix count
  check would have checked a placeholder, while the run and the adjudicator's
  step count read the rewritten file. The proposal requires that whatever the
  run executes is what those three checked.
- *Building core code in `ui-tests.yml`.* That means a Rust toolchain and the
  SDK staging step (`sdk-staging`) in a job that today builds nothing with
  cargo, which adds cost to every run of it.
- *No drift detection.* A reference produced by the core that verifies it
  moves with that core. An encoding change moves both halves together, and the
  spec stays green through exactly the change that strands every reference
  already shared. That rules it out on its own.

**The seeder, among the candidate sources the proposal named.**

- **`seed_store`, rejected.** It mints a fresh keystore on every run and writes
  a whole store to a directory. The first property makes its output a new Stoa
  each time. The second is a write this piece must not make anywhere near a
  profile.
- **A creation reply from a throwaway core, rejected.** `create_stoa` names the
  keystore's machine key as the creator, so it would need a keystore, and that
  keystore's key is random.
- **Chosen:** `seeded_record()` in `seeded_reference.rs`. It is the fixed seed
  and title put through `Genesis::canonical_bytes` and `Genesis::address`, the
  functions `create_stoa` itself uses. The seeder writes nothing anywhere. Its
  output is the reference, which every failure message in the file prints
  ready to paste. That is how the literal was produced: the tests were written
  first, run with no spec present, and the reference was taken from the
  failure.

**What breaks without the check, measured:** with the committed address's
first hex digit changed, all three tests go red. The record test reports "the
committed address is not the hash of the committed record". The preview and
join tests each report the core's refusal, "the genesis record does not hash to
the Stoa address it was given with".

### D2 — The check reads the literal out of the spec, as text

**Chosen:** `seeded_reference.rs` reads `seeded-join.yaml` at run time and
takes the one substring opening with `{"stoa":"` up to the next `}`.

- **Why the spec and not a constant in the test.** A constant would be a second
  copy of the one value, and it would need a third thing to keep the two equal.
  Reading the spec means the check covers exactly what the run types.
- **Why as text and not as YAML.** `dialectica-core` has no YAML parser, and a
  dev-dependency for one test would widen the dependency wall `Cargo.toml`
  argues for crate by crate. It is also not needed. The literal is a
  single-quoted YAML scalar, whose content is taken verbatim, and a reference
  holds no quote, brace or escape of its own. `yq '.steps[3].type.text'`
  returned the same string when checked.
- **Fails closed.** The test requires exactly one occurrence. A reference
  rewritten into an escaped double-quoted form has zero, and a second
  reference pasted into the file makes two. Either is red, with the path and
  the regenerated reference in the message.
- **At run time, not with `include_str!`.** The spec is outside the crate. A
  moved spec is then a failing test that names the path, rather than a crate
  that does not compile.

### D3 — A new file, `seeded-join.yaml`, and not steps added to `join.yaml`

The cost on each side is `e2e-created-stoa-flow`'s D1.

**Chosen: a sixth matrix job.** sitometres 0.1.2 stops at the first failed step
and records every later one as `inconclusive` (`e2e-suite-review` design.md
D2, measured). Appended to `join.yaml`, a defect in the refusal path would
leave the successful join unreported either way. The appended steps would also
start on a join screen showing a refusal, so they would need a Cancel and a
second paste. Their preview assertion would then be about the second reference
of a session, which is not the case a user meets first. As its own file, a red
job's name says which path broke, and the D6 break reddens this job alone among
the six.

**The cost, stated:** one more job, about three minutes warm (the
`e2e-ui-suite` change's design.md D7). It runs in parallel with the other five,
so the wall-clock cost is unchanged, and the store cache is shared.

`join.yaml`'s "does NOT cover" paragraph now points here instead of naming a
gap.

### D4 — What each step asserts

- **The empty listing is asserted here, although `join.yaml` asserts it too.**
  `e2e-created-stoa-flow`'s D2 keeps a spec's route-in free of assertions that
  belong to another spec. This one is not route-in. It is the precondition
  that makes the listed row after the join evidence of the join, which is why
  the proposal lists it as the spec's first observation.
- **The preview is asserted from the element tree:** `foundingTitleText`
  holding the seeded title, and `fallbackNote` drawn. The title position is
  filled before a join only by a lookup that fell back, and the note is drawn
  for exactly that reply, so the two together are "the lookup answered a
  fallback and its title is rendered as the founding title".
  `joinState === 'previewing'` is "no join call has been made for this
  reference", as in `join.yaml`. It is an `expect:`, because the lookup runs
  synchronously when the reference lands (`e2e-created-stoa-flow` D5).
- **The joined outcome pairs a presence with an absence** (the `e2e-ui-suite`
  change's design.md D5). `joinedPanel` is drawn, and `joinFailurePanel` is
  not.
- **The listed row is compared with the seeded address**, not counted:
  `listedStoas.length === 1` and `listedStoas[0] === '<address>'`. A listing
  holding some other single Stoa fails. The share button is that row's, there
  being one row.
- **The feed read is the last observation, as `feed.yaml`'s is:**
  `feedReadState === 'ok'` beside `feedRowCount === 0`, because a failed read
  also leaves no rows. The read carries the record the view kept from the join
  (`Main.qml`'s `onJoined` records it) or from the reloaded listing. The core
  refuses a read whose record does not hash to the address, so a read that
  succeeded shows the record handed on was the right one.
- **No new root handles.** Every observation reads a handle an earlier piece
  pinned in `tst_e2e_handles.qml`, or an `objectName`. The one view change is
  `joinCancelButton` on the join screen's Cancel, which is the way back and
  had no name. `tst_navigation.qml`'s
  `test_a_joined_stoa_is_listed_once_the_join_screen_is_left` presses it by
  that name after a successful join.
- **Every click follows a `wait_for:` or `expect:` step**, so the one-second
  settle floor (`e2e-created-stoa-flow` Context) runs before it. That matters
  for the way back: Cancel moves when the join button hides and the joined
  panel appears above it, and a click aimed at its old geometry is the D9
  shape from that change. The step before it waits for the joined panel.
- **No `calls:`**, for the `e2e-ui-suite` change's design.md D10 reason.

The spec is 14 steps.

### D5 — The seeder is a core-level fixture, not the seeding peer #134 names

#134's body lists "a successful join (needs a seeder peer)", and the owner's
comment says the same: "a successful join, which needs a seeding peer". This
piece supplies a seeded REFERENCE instead of a seeding PEER. The departure is
deliberate, and the owner can overrule it. The reasons are the archived
`e2e-created-stoa-flow` proposal's ("The second piece"), each checkable:

- **A join consults nothing but its two inputs** (`stoa-membership`,
  "Verification consults nothing but the two inputs"). On a peer holding no
  ops for the Stoa, the preview's `getStoa` answers the record's own founding
  title as a fallback (`stoa-metadata`). A live peer contributes nothing that
  either call checks. `seeded_reference.rs` makes both calls against an empty
  store and they succeed.
- **A second peer could not supply content either.** Ops never leave the peer
  that authored them (#176). The publish sink logs "delivery is not wired yet"
  and sends nothing.
- **sitometres drives one app.** A second Basecamp in the same job would be a
  host no step can observe.

**When this stops being right:** once #176 lands, receipt from another peer is
what a test should exercise, and a second profile under `lgs basecamp launch`
becomes the seeder. That is #102's item 7. It is not on #134's list.

### D6 — The proof: one break, pushed alone, then reverted

The `e2e-suite-review` change's model (its design.md D3): the break is its own
commit, pushed alone, and both workflows are waited out. Then a `git revert`
is pushed and waited out. Both workflows cancel in progress per ref, so a push
made before a run finishes records nothing.

**The break:** `Main.qml`'s `onJoined` no longer calls `list.reload()`.

**Why this one.** It is aimed at the `view-navigation` scenario this change
adds, "A joined Stoa reaches the list without a restart", which no other spec
walks. Its red run also shows every step before it passing against the real
host, so the red is itself evidence that the join succeeded end to end. A
break at the join outcome would fail at step 9 and show nothing after it.

**Predicted:** `seeded-join` goes red on step 11, "the joined Stoa is listed
without a restart, and can be shared". It fails on `listedStoas.length === 1`,
`listedStoas[0] === …` and the `shareButton` text, after its 30-second
timeout. Steps 12 to 14 are `inconclusive`, and the verdict is `fail`. The
other five `ui-tests.yml` jobs stay green, since none of them joins
successfully. In `ci.yml`, the `qml` job goes red on
`test_a_joined_stoa_is_listed_once_the_join_screen_is_left`. That red is in
the other workflow and is expected. Measured locally with the break applied:
that test fails, on "the listing is read again once the join has succeeded",
and every other test in the four component files that drive a join
(`tst_navigation.qml`, `tst_stoa_screens.qml`, `tst_e2e_handles.qml`,
`tst_render_probe.qml`) passes. **Before this change nothing in the component
suite could see the break**, which is the gap the `view-navigation` delta
names: "The return is still available after a join succeeds" required the list
to render, and nothing required the joined Stoa to be on it.

`tasks.md` records the observed runs against this prediction.

## Risks / Trade-offs

- **The textual read has one blind case** (D2). A reference in a comment,
  combined with a step typing an escaped form, would pass the check on the
  comment's copy. → The spec's header says to keep the reference the only one
  in the file. `ui-tests.yml` would still go red on the typed one, one
  workflow later.
- **The fixture's creator secret is public.** Anyone can sign as its
  moderator. → It is a fixture no one posts in. `seeded_reference.rs` says it
  must never be offered to a user as anything but a test.
- **A join needs no key today, and this spec relies on that:** the profile is
  in the no-key state throughout. → If a later change gates the join on a key,
  step 9 goes red. That is the right direction, since the spec would then be
  asserting something the view no longer does.
- **The full run is proven only in CI.** The owner's rules keep sitometres and
  a local Basecamp run out of local hands. → CI is the proof, and `tasks.md`
  claims no local green for the spec.
- **A pull_request run tests the merge of the branch with `main`.** → Each run
  is recorded with the head SHA it ran on.
