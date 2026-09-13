# Tasks

## Stages

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [x] tests — `tester`
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [x] findings all ticked, `findings/` deleted — closer
- [x] `openspec validate --strict`, then `archive` — closer

## Implementation

- [x] `Core.qml` gains three named wrappers — `generateIdentitySlate`,
      `keepIdentity`, `whoAmI` — each through the existing `call()`, none
      taking an identity. `call()` itself is unchanged.
- [x] `OnboardingScreen.qml`: one `phase` string over five states, so "at most
      one of kept, refused and failed is shown" holds by construction.
- [x] The opening state requests nothing. The one action reaching the module is
      the slate request.
- [x] Candidates, their count and their order all come from the reply. A reply
      with no candidate array is the failed state, not an empty slate.
- [x] Refresh is unlimited, replaces the set, and clears the selection. A failed
      refresh clears the set rather than leaving it on screen as current.
- [x] Nothing is pre-selected; `selectedIndex` starts and returns to `-1`.
      `keepSelected()` refuses independently of the button's appearance, since
      `FlatButton` emits `clicked()` regardless.
- [x] The keep's three outcomes are three phases. A refusal keeps the set on
      screen and shows the module's reason verbatim.
- [x] The kept identity is built from the **reply**, and a `kept:true` carrying
      no address is a failure rather than an empty identity.
- [x] `encrypted` and `recoveryNeedsTheRecord` are held three-valued, so an
      omitted field produces no claim.
- [x] Every candidate address renders through `AddressLabel { full: true }`. No
      elision is hand-rolled; no row shows a name, a path or an index.
- [x] Copy: the cross-Stoa unlinkability clause is dropped with no replacement
      claim, and the uniqueness note states its obligation with **no word count
      at all** — the count has moved three times, so a screen carrying a number
      goes stale on the next move. `design.md` records why.
- [x] Every `Text` sets `textFormat: Text.PlainText` explicitly.
- [x] `Main.qml` branches on `who_am_i` alone, re-asks after a keep, routes both
      absent cases to onboarding with the reason held unparsed, and shows the
      failure rather than guessing a branch.
- [x] `qmldir` registers `OnboardingScreen`.
- [x] `docs/UI-BRIEF.md`: the onboarding bullet now says core serves **no name**
      and what the row must therefore show. Fixed here rather than deferred,
      because this change is what makes the omission concrete.
- [x] Tests: `tst_onboarding_states.qml` and `tst_launch_branch.qml`, both new.
      Counts are deliberately not written here — run
      `dialectica-ui/tests/run-qml-tests.sh`, which reports them and cannot go
      stale. Every fix in this change was a failing test first, and each
      mechanism was mutation-checked; the findings files record which mutation
      reaches which test.
