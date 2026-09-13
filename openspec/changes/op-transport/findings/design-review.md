# Design review — `op-transport`

Checked `design.md` and `tasks.md` against
`dialectica/rust-lib/dialectica-core/src/transport.rs`, the spec delta beside
them, and `docs/PLAN.md` read from `origin/main`.

**The decisions are in good shape.** All four of the decisions this piece made
under pressure are recorded, and recorded honestly:

- **`publish` returns a `Publishable` and never sends** — recorded at
  `design.md:265-291` and again as the seam at `design.md:369-395`. The code
  matches: `publish` takes `&OpenChannels` (not `&mut`), so opening a channel is
  unreachable from it by type, and the single `log.append` precedes the channel
  lookup, so a Stoa with no channel still stores the op. Verified by reading
  `transport.rs:605-634`.
- **The "cannot be done" rewrite says scope, not impossibility.** The spec is
  explicit — `specs/op-transport/spec.md:255`: *"None of them is discharged by
  this change, and that is a scope statement rather than an impossibility."* It
  then names the three owed things and the one thing genuinely ruled out (that a
  publish's *reply* could carry the answer, which follows from event timing).
  `design.md:369-395` names the seam the tracker attaches to. This is the honest
  shape, not the false-impossibility tell.
- **The prescribed fix is recorded as *unavailable*, not merely untaken.**
  `design.md:216-247` names the merged requirement that forbids it, and I
  verified the citation rather than trusting it: `content-authoring/spec.md:184-189`
  carries *"A body at the cap is published"* exactly as quoted. Three defensible
  answers are costed with their blast radii, and a **fourth** is ruled out by
  measurement at `design.md:254-263` — raising `MAX_MESSAGE_BYTES` by 1024 broke
  `an_op_at_the_limit_is_admitted` with `Undecodable(FieldTooLong(154484))`,
  because that fixture reaches the limit by padding a body. The coupling the
  brief asked about **is** written down, in the words the brief used.
- **`transport::publish` having no production caller** is recorded at
  `design.md:445-480`, with the reason: wiring it means removing one of two
  appends across two files with two suites pinning the same ordering. I verified
  the premise independently — `grep -rn "transport::"` over
  `dialectica/rust-lib/` returns **zero** lines, and `content-authoring/spec.md:27-32`
  does require publish to *"hand it to delivery"*. The ruling and its reason are
  in the document, not only in the runner's head. `design.md:477-480` also states
  what deferring costs, which is the part usually left out.

The known-issue list in the brief matches what I found; I have not re-reported
the pathless per-Stoa key, the creator's two keys, or the `cfg(logos_scaffold)`
and `cargo fmt` gate blindness.

Gate re-run on this worktree: `cargo test -p dialectica -p dialectica-core`
gives **718 + 26 = 744 passing, 0 failed, 0 ignored** — the brief's baseline
exactly.

Three entries follow. One is a claim the code does not honour; two are
unrecorded decisions.

- [x] **`dev-writer`** — `design.md:155` and `design.md:296` name a type that
      does not exist, and the name they use belongs to a *different live type*.
      Both lines cite `Refusal::Undecodable`. The type is `InboundRefusal`
      (`transport.rs:327`), renamed during review precisely to avoid this
      collision — and `Refusal` is not a dangling name: `authoring::Refusal`
      exists (`authoring.rs:84`) with a disjoint variant set and **no
      `Undecodable` variant**. So this is worse than the `PublishOutcome`
      phantom already fixed on this branch: a reader who greps `Refusal::` lands
      on the authoring enum, finds no `Undecodable`, and concludes the document
      describes code that was removed. The irony is that `transport.rs:302-312`
      argues the rename at length for exactly this reason — "two public
      `Refusal`s in one crate is a collision that gets resolved by whoever needs
      both first, under time pressure" — so `design.md` is now the instance of
      the confusion the code comment predicted. **Verified:** `grep -n
      "PublishOutcome\|Refusal::" design.md` returns only these two lines, and
      `grep -rn "Refusal" authoring.rs` confirms the variant is absent there.
      This is the third phantom on this piece; the sweep the brief asked for
      found it.

      **Fixed.** Both cited lines now read `InboundRefusal::Undecodable`, and the
      sweep found **two more instances your grep could not have seen**, because it
      covered `design.md` only:

      - `design.md:151` rendered the signature as `Result<Admitted, Refusal>`; the
        real one is `Result<Admitted, InboundRefusal>` (`transport.rs:458`).
      - `design.md`'s section heading read "One `Refusal` enum".
      - `tasks.md:84` and `tasks.md:87` both carried the bare name — the same
        phantom, in the document a reader checks to see what was built.

      All four corrected. Beyond the rename I added a paragraph under that section
      recording **why** the type is `InboundRefusal`, so the collision argument lives
      in the design document and not only in the code comment that predicted it —
      and it names this instance, since a document that demonstrated the confusion it
      was warned about is the most useful possible illustration of why the bare name
      is not available. Your framing that a resolving-but-wrong name is worse than a
      dangling one is the part I carried across verbatim in substance.

      No test pins prose. What makes this checkable rather than asserted is that the
      corrected names are the ones the compiler accepts: `grep -rn "Refusal::"` over
      `design.md`, `tasks.md` and the spec now returns only the qualified form plus
      one line of my own prose describing the defect.

- [x] **`dev-writer`** — the `#[must_use]` on `Publishable` is a decision with a
      real alternative, argued in a 15-line code comment, and `design.md` does
      not mention it anywhere. `transport.rs:553-567` gives the whole argument:
      the attribute is "the only compiler-visible signal separating" a
      `Publishable` dropped by a path that forgot to send from one sent and never
      propagated — and telling those two apart *is* one of the three things this
      change records as owed. That is a type chosen to make a mistake
      unrepresentable, tied to the change's own central gap, and by the brief's
      account it **fired on two real sites nobody had predicted** (silent
      `publish(...).unwrap();` discards, not the deliberate drop a reviewer had
      named). `grep -n "must_use\|Publishable" design.md` finds four mentions of
      `Publishable` and **zero** of `must_use`. What the next person
      re-litigates: whether the attribute is a lint preference to be removed when
      it becomes inconvenient at the first real call site — which is exactly when
      the wiring lands and the discards stop being tests. The measurement (two
      unpredicted sites) is the part that would settle it and is nowhere in a
      durable document; `findings/` is deleted before merge.

      A second, smaller problem inside the same comment: it says the attribute
      "is paired with an explicit `let _ =` at that one site", naming
      `a_send_failure_does_not_lose_the_op`. There are **three** deliberate
      discard sites, and that test is not one of them — it uses
      `drop(publishable)` (`transport.rs:2325`), while the `let _ =` sites are
      `transport.rs:2384`, `transport.rs:2400` and `transport.rs:880`. "That one
      site" is a count the code disproves.

      **Fixed, both halves.**

      The decision is now recorded in `design.md` as a subsection under the
      delivery-outcome seam — deliberately there rather than in a list of attributes,
      because the attribute's justification *is* the tracker gap: it exists to keep a
      dropped `Publishable` distinguishable from a sent-and-never-propagated one, which
      is one of the three owed things. Placing it anywhere else would have separated the
      attribute from the only argument that makes it more than a lint preference. The
      entry carries the two-unpredicted-sites measurement, since as you note that is
      the part that settles the re-litigation, and it names the re-litigation
      explicitly: the first real call site is when the attribute starts earning its
      keep, not when it stops.

      The count is corrected in the code comment too, and I took the numbers from
      `grep -n "let _ = \|drop(" transport.rs` rather than from the comment. That
      returns six lines; three are `Publishable` discards (`:880`, `:2384`, `:2400`),
      two discard `receive` (`:2039`, `:2077`, a different type), and `:2325` is the
      `drop(publishable)` inside `a_send_failure_does_not_lose_the_op` (which starts
      at `:2308`). So your count and your site attribution are both exactly right, and
      the comment was wrong on both. The comment now states the three-site count, says
      that test uses `drop` and is not one of them, and says to get the count from a
      `grep` rather than from itself.

      Least confident part: nothing here is test-pinned, because a `#[must_use]`
      firing is a compile-time warning rather than a runtime assertion, and the
      discards are already explicit so no current build emits it. The attribute's
      value is therefore demonstrated by the two sites it caught historically — which
      is exactly why that measurement needed a durable home before `findings/` is
      deleted.

- [x] **`dev-writer`** — `OpenChannels::close_all` returns its ids **sorted**,
      and the reason is only in the code. `transport.rs:244-249` sorts so that "a
      caller's sequence of `channelClose` calls does not depend on hash iteration
      order", and the comment is careful to say this "is not a correctness
      property of the close" but makes a test assertable against a fixed
      expectation. That is a decision with a real alternative — returning
      insertion order, or not sorting and having the test sort — and it is the
      shape where a later reader *removes* the sort as pointless (the comment
      itself concedes it is not correctness) and silently breaks
      `closing_every_open_channel_yields_each_channels_identifier`, which
      compares against a sorted `expected`. `grep -n "close_all\|sorted"
      design.md` returns nothing. A thin entry naming the alternative and the
      cost would be enough; this is a suggestion rather than a defect.

      **Fixed** — taken as the suggestion it was labelled, and it landed slightly
      larger than "thin" for one reason worth naming. Your diagnosis of the failure
      mode is right (the comment's own concession is what invites the deletion), but
      while writing the entry I checked the test and the alternative is a little
      worse than "have the test sort": `closing_every_open_channel_yields_each_channels_identifier`
      (`transport.rs:2588`) already sorts its `expected` (`:2607`), so moving the sort
      to the caller means comparing a sorted vector against a sorted copy — which stops
      distinguishing "returned every id" from "returned every id in some order". The
      recorded entry therefore names determinism as the property to preserve rather
      than the specific ordering, and states that a caller must not depend on the order
      either way, since nothing about closing at the transport is order-sensitive.

      The entry sits under the `OpenChannels` section, beside the map-shape decision it
      belongs to. It names both alternatives you listed plus insertion order, and says
      what deleting the sort actually breaks, so the next reader meets the consequence
      rather than the concession.

## Two notes that are not findings

**PLAN.md shedding is done properly, and is the best I have seen on this repo.**
Reasoning this change acted on moved out and the behaviour lines are struck
through with a pointer: §4.1's channel/topic bullets, §4.3's "no per-peer state"
and "pure function of the addressed object", §4.4's ordering-metadata bullet,
§4.5's derivation clause, §9.2 item 8, §13's timestamp finding. Each keeps the
*judgement* PLAN is for and sheds the *rule* the spec now carries, which is the
split `.claude/agents/README.md:41-52` asks for. I found no reasoning duplicated
across both documents and no PLAN paragraph left explaining something this change
built. §4.3's `#4116` withdrawal and the shared-node close argument are correctly
**kept** — the spec states those obligations without the reasoning.

**One thing for the runner, not a design defect.** This branch is behind
`origin/main` by two commits — `fde0fd0` (#50, create/join/list Stoas) and
`733544d` (#59, the closer). That is why `git diff origin/main
origin/piece/op-transport --stat` reads as though this change *deletes*
`membership.rs` (−1842), `openspec/specs/stoa-membership/spec.md` (−414), the
archived `stoa-lifecycle` change and `.github/workflows/ci.yml` (−89). It does
not; the branch simply predates them. It matters here because `tasks.md:226-238`
records a previous merge done for exactly this reason ("its diff against `main`
deleted content it never touched"), and because the branch's own PLAN.md edits
would revert `main`'s §4.8 and §9.1 rewrites about joining taking the address
**and** the genesis record. A third `origin/main` merge is owed before this
merges, and the PLAN.md hunks need re-reconciling rather than fast-forwarding —
the same trap `tasks.md` says `mergeStateStatus` reported as `UNKNOWN` rather
than as a conflict.
