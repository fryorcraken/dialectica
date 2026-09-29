## Why

Five changes had shipped their code and were never archived, so their
requirements never became part of the live contract in `openspec/specs/`. A
sixth, `relevance-votes`, was also still open, but it built no code. The
live specs had also picked up citations to things that no longer exist: sections
of the deleted `docs/PLAN.md`, closed GitHub issues, and a `design.md` with no
path. One live spec contradicted itself.

The owner asked for the specs to be tidied up. This change does that and
touches no production code.

## What Changes

### Unarchived changes

Each change was checked against its merged PR and against the code and tests on
main, not only against its `tasks.md`. A delta is corrected against the live spec
before it is promoted, and each promoted spec was diffed against its delta.

- **`sqlite-projection`: archived** (shipped in #20). Before promotion, its
  op-log delta was rebased onto the live spec (`1f5df8f`). As written, it would
  have deleted the blank-title encoding refusal, and it would have promoted a
  decay point taken from transport metadata, which `op-clock` had stopped using.
  It was then archived (`0519416`).
- **`first-run-identity`: archived** (shipped in #128). Before promotion, two
  sentences that later merges had made false were corrected (`b1d8966`). It was
  then archived (`b545d3b`).
- **`ui-thread-view`: archived** as a new `thread-view` capability (shipped in
  #122). Before promotion (`584359b`):
  - Two citations were moved to `view-navigation`, where the requirements they
    name now live.
  - Two `docs/PLAN.md` citations were dropped.
  - A scenario for a reply affordance the screen does not offer was taken out of
    scope.

  It was then archived (`d70f662`).
- **`ui-remaining-screens`: archived** (shipped in #124), adding one
  `stoa-navigation-view` requirement (`2fa2b11`).
- **`moderation-screen`: archived** as a new `moderation-view` capability, with
  one `stoa-navigation-view` requirement amended (shipped in #127). The screen,
  its tests, the route in and out, and the row placeholder are all on main. No
  moderation-publishing method exists, and the screen's inertness rests on that
  absence. Before promotion, four corrections were made (`8e508d8`), and it was then archived (`eb8be6b`):
  - The `MODIFIED` block had silently dropped a live paragraph banning any phrase
    that asserts nothing has been received for a Stoa. `validate --strict`
    cannot see a dropped paragraph. The paragraph is restored, applied to a
    placeholder as well, and given a scenario.
  - "this change's own `design.md`" now names the archived `moderation-screen`
    design.
  - `moderation-view` gains a Purpose.
  - A requirement is added that the screen does not state or imply that the user
    moderates the Stoa. A `NO SPEC:` marker in `tst_moderation_screen.qml` cited
    it as existing when no requirement said so. The code already meets it.
- **`relevance-votes`: out of scope for this piece, on the owner's instruction.
  It stays unarchived.**

### Live-spec corrections, carried as deltas in this change

- **`moderation-resolution`**: "The deciding moderation is named, not merely
  counted" cited §5.7 of the deleted PLAN.md. It now cites `post-revision`'s "A
  post is never edited in place".
- **`stoa-genesis`**: "An address verifies the record it names" cited §4.8. It
  now cites `stoa-navigation-view`'s "Joining shows what is being joined, and
  joins nothing until the user acts".
- **`generated-names`**:
  - "The name SHALL NOT travel" said that no requirement obliges core to expose
    the derivation, and that the gap was tracked in issue #81. The same spec's
    "Core exposes the derivation to a caller" requires exactly that, #81 is
    closed, and the entry point exists. The paragraph now points at that
    requirement.
  - "A name is never unique, never an identifier, and never numbered" loses its
    issue-#80 and "this change" wording.
- **`feed-view`**: "A displayed time is marked as the author's assertion, never
  as a verified instant":
  - A bare `design.md` citation now names the archived `op-clock` design.
  - "not widened by this change" now states what `feed-read` contracts: a feed
    row carries no time.
  - A clause saying the positive half "binds for exactly as long as the field is
    absent" is corrected to say it has nothing to bind while the field is absent.

### `view-navigation` becomes the single home for screen entry and exit

This is the owner's decision. `view-navigation`'s Purpose says it owns
transitions and nothing a screen renders. But `thread-view` and `moderation-view`
each carried a route requirement for their own screen, and `thread-view`'s
duplicated one of `view-navigation`'s. What each removed requirement carried now
lives here:

- **`thread-view`: "The thread screen is reached from the feed and can be left
  again" is `REMOVED`.** Its substance goes into `view-navigation`'s `MODIFIED`
  requirement "A thread is opened from a feed row and can be left":
  - The rule that the return is not withdrawn by any state the thread screen can
    be in, a refused read included, becomes a new paragraph there. It keeps the
    scenario "The way out survives a refused read".
  - The rule that the thread screen carries no thread identifier, Stoa address
    or genesis record of its own becomes a new paragraph there. It keeps the
    scenario, renamed "The thread screen carries no thread of its own".
  - The rule that no thread screen and no thread read exist before a thread is
    chosen goes into that paragraph. It keeps the scenario "No thread is
    rendered before one has been chosen".
  - The rule that the thread read names the acted-on row's thread identifier and
    Stoa becomes an extra `AND` on the existing scenario "Acting on a feed row
    opens its thread".
  - The return without a restart, the rule that no record is invented, and the
    return as an outcome rather than a mechanism were already stated there. The
    last is now cited to "Every state a user can enter has a specified way out".
- **`moderation-view`: "The screen is reachable, and leaving it returns where
  the user was" is `REMOVED`.** It is `ADDED` to `view-navigation` as "The
  moderation screen is reachable, and leaving it returns where the user was".
  - Its text and three scenarios are kept, except that "the screen" now names
    the moderation screen and "`view-navigation`" now reads "this capability".
  - One sentence is added there, leaving what the screen renders to
    `moderation-view`.
  - `moderation-screen` had already been archived when the decision came, so the
    move is a `spec-tidy` delta rather than a correction to that change's delta.

### Live-spec corrections made in place

A delta cannot carry a Purpose, so these are direct edits to the live specs:

- **`view-navigation`'s Purpose** now names `thread-view` and `moderation-view`
  where it said "the thread screen's own capability" (`06bfd48`).
- **`thread-view`'s and `moderation-view`'s Purposes** now say that
  `view-navigation` owns how their screen is entered and left. `thread-view` no
  longer lists "how the screen is entered and left" among what it defines
  (`ff55cf7`).

No requirement in this change's own deltas changes behaviour. Each one replaces
a citation, removes a contradiction, fixes a sentence that said the opposite of
its argument, or moves an obligation from one capability to another without
loss. The two behaviour-level additions made in this piece belong to
`moderation-screen`'s corrected delta, and the code already met both: the
restored emptiness ban and the no-authority requirement.

## Capabilities

### New Capabilities

None in this change's own deltas. `moderation-view` became live through
`moderation-screen`'s archive, above.

### Modified Capabilities

- `moderation-resolution`: citation in "The deciding moderation is named, not
  merely counted".
- `stoa-genesis`: citation in "An address verifies the record it names".
- `generated-names`: the contradiction in "The name SHALL NOT travel"; the
  issue-number sentence in "A name is never unique, never an identifier, and
  never numbered".
- `feed-view`: citation and one clause in "A displayed time is marked as the
  author's assertion, never as a verified instant".
- `view-navigation`: "A thread is opened from a feed row and can be left"
  amended; "The moderation screen is reachable, and leaving it returns where the
  user was" added.
- `thread-view`: "The thread screen is reached from the feed and can be left
  again" removed.
- `moderation-view`: "The screen is reachable, and leaving it returns where the
  user was" removed.

## Impact

Spec text in `openspec/specs/` and `openspec/changes/`. Outside those:

- **Four comments change**: two in `dialectica-ui/src/qml/` and the headers of
  two end-to-end specs under `dialectica-ui/tests/ui/`. Each cited a requirement
  this piece moved, or called a change this piece archives "unarchived".
- **Tests are added** for requirements the archives promoted: two `#[test]`
  functions and a helper in `sqlite.rs`'s `mod tests`, and five QML test
  functions in `tst_thread_reply.qml`. Two `NO SPEC:` comments in
  `tst_moderation_screen.qml` and `tst_stoa_screens.qml` are reworded to cite
  the requirements that now cover them. `design.md`'s Non-Goals names each test
  and its commit.
- **No tests are pending.** Every `tester` finding is closed. One,
  `thread-view`'s "No ordering is offered as vote-based", is deferred with no
  test; `design.md`'s Non-Goals says why and what triggers one.

No production code, existing test assertion or wire contract changes.

## Tracked separately

- **Sweeping "this change" out of live specs.** About twenty passages across
  twelve live specs still say "this change", which means nothing once a change
  is archived. On the owner's instruction this is tracked in its own GitHub
  issue and is not part of this piece.

## Open questions for the owner

1. **Do `feed-view` and `feed-read` disagree about a feed row's time?**
   `feed-view` says a time field on a feed row is "expected to arrive — deferred
   rather than declined". `feed-read` says a row MUST NOT carry "a time of any
   kind, asserted or otherwise". Both can hold while no time arrives, but they
   disagree about whether one ever should. Which capability's position stands
   is a design call, so this piece records it and changes neither.
