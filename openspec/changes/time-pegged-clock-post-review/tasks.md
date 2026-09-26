## Stages

Every review row covers this change's own diff **and** `git diff 2eada33 c1f1a8f`, the four commits merged with #165 after its review round. `proposal.md`, "Review scope", names them.

- [x] spec — `spec-writer`
- [x] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — does not apply: this change alters no behaviour, so there is nothing new to test. The four #165 commits under review carry their own tests, and `spec-test-reviewer` reads those.
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer`
- [x] review: readability — `code-reviewer`
- [x] review: architecture — `code-reviewer`
- [x] review: spec-test — `spec-test-reviewer`
- [x] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. Carry the removed reasoning

- [x] 1.1 Write `design.md` so that every label in `proposal.md`, "Reasoning removed from the specs" (O1–O18, F1–F4, T1–T5, V1, R1, H1–H2), is either argued in a Decision or cited to the archived `2026-09-25-time-pegged-clock/design.md` Decision that argues it. Verify against the table in `design.md` Decision 1, which lists every label once.
- [x] 1.2 Carry no reasoning into the archived `2026-09-25-time-pegged-clock/` folder (`design.md` Decision 1). Its only edit is the two forward pointers of 4.5. Verify with `git diff origin/main... -- openspec/changes/archive/`, which shows those two notes and nothing else.

## 2. Check nothing outside the specs relied on the removed text

- [x] 2.1 Sweep `dialectica/rust-lib` for comments that cite a removed passage, with `git grep -n -F -e "<capability>" -- dialectica/rust-lib` once for each of `op-ordering`, `op-transport`, `op-format`, `thread-read`, `feed-view` and `post-revision`, checking each hit against `proposal.md`'s removed passages. Result: four doc comments cite removed reasoning. Three say "`op-ordering` states the cost" (O18): `arrival.rs` on `RECEIVE_WINDOW_MS`, and `op.rs` twice. One says the window's "reasoning is `op-ordering`'s" (T5): `transport.rs` on `receive`. No other hit cites a removed passage.
- [x] 2.2 Repoint those four comments to the archived `2026-09-25-time-pegged-clock/design.md`, Decision 11 (and Decision 1 for `transport.rs`), after checking that Decision 11 works the cost through. Doc comments only: no test and no code line changes, and `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` and the CI clippy command pass.

## 3. Gates

- [x] 3.1 `openspec validate time-pegged-clock-post-review --strict` passes.
- [x] 3.2 `nix build ./dialectica#lgx` succeeds from the tree root.
- [x] 3.3 Open the PR on `piece/162-post-review`, naming PR #165 and #162 as what it follows up, without a closing keyword (#162 is closed), and stating that it carries the review of `git diff 2eada33 c1f1a8f`.

## 4. Review findings addressed to `dev-writer`

- [x] 4.1 Cite the archived design by change name in the four repointed doc comments (`arrival.rs`, `op.rs` twice, `transport.rs`), matching the crate's idiom, and record why in `design.md`, Risks. Doc comments only.
- [x] 4.2 Record in `design.md`, Decision 11, that archived Decision 3's "What pins it" missed the test `c1f1a8f` added. Re-measure the mutation it names rather than quoting it.
- [x] 4.3 Add the self-only one-hour exposure to `revision::current_version`'s doc, which archived Decision 10 says is there. Record the correction in `design.md`, Decision 11. Doc comment only; no test can see it.
- [x] 4.4 In that paragraph, name `ADVANCE_BOUND` as what "permanently" describes, in place of "Before the window", which review read as the hour before the time passes the counter. Doc comment only.
- [x] 4.5 Add a note to archived Decisions 3 and 10, marked as added by a later change, pointing at `design.md` Decision 11. Record why in Decision 11, and correct Context and Decision 1, which stated the archive as not edited.
- [x] 4.6 Rewrap `op.rs`'s `asserted_ms` doc and `design.md`'s first Risks bullet where the citation rewording left an over-long line.
