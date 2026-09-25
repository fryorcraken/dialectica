## Stages

Every review row covers this change's own diff **and** `git diff 2eada33 c1f1a8f`, the four commits merged with #165 after its review round. `proposal.md`, "Review scope", names them.

- [x] spec — `spec-writer`
- [ ] design + code — `dev-writer`
- [ ] ~~tests — `tester`~~ — does not apply: this change alters no behaviour, so there is nothing new to test. The four #165 commits under review carry their own tests, and `spec-test-reviewer` reads those.
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## 1. Carry the removed reasoning

- [x] 1.1 Write `design.md` so that every label in `proposal.md`, "Reasoning removed from the specs" (O1–O18, F1–F4, T1–T5, V1, R1, H1–H2), is either argued in a Decision or cited to the archived `2026-09-25-time-pegged-clock/design.md` Decision that argues it. Verify against the table in `design.md` Decision 1, which lists every label once.
- [x] 1.2 Leave the archived `2026-09-25-time-pegged-clock/` folder unedited. Verify with `git diff origin/main... -- openspec/changes/archive/`, which is empty.

## 2. Check nothing outside the specs relied on the removed text

- [x] 2.1 Search `dialectica/` for comments or tests that point at a removed passage as their authority, with `git grep -n -F` on the removed passages' distinctive phrases and on "`op-ordering` states". Result: three doc comments, in `arrival.rs` and twice in `op.rs`, say "`op-ordering` states the cost", which O18's removal makes false. Not edited, because this change changes no code. Recorded in `design.md`, Risks, and reported to the runner.
- [x] 2.2 Confirm no test changes. No file under `dialectica/` or `dialectica-ui/` is in this change's diff, and `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` passes.

## 3. Gates

- [x] 3.1 `openspec validate time-pegged-clock-post-review --strict` passes.
- [x] 3.2 `nix build ./dialectica#lgx` succeeds from the tree root.
- [ ] 3.3 Open the PR on `piece/162-post-review`, naming PR #165 and #162 as what it follows up, without a closing keyword (#162 is closed), and stating that it carries the review of `git diff 2eada33 c1f1a8f`.
