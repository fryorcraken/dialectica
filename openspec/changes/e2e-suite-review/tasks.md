# Tasks

## Stages

- [ ] ~~spec — `spec-writer`~~ — no spec delta: a test-only piece, `skip_specs: true` in `.openspec.yaml` says why; `proposal.md` is written
- [ ] design + code — `dev-writer`
- [ ] tests — `tester`
- [ ] review: correctness — `code-reviewer`
- [ ] review: security — `code-reviewer`
- [ ] review: readability — `code-reviewer`
- [ ] review: architecture — `code-reviewer`
- [ ] review: spec-test — `spec-test-reviewer`
- [ ] review: design — `design-reviewer`
- [ ] findings all ticked, `findings/` deleted — `closer`
- [ ] `openspec validate --strict`, then `archive` — `closer`
- [ ] CI green, title/body checked, PR merged — `closer`

## Implementation

The two proofs are CI runs and nothing local can stand in for them: the owner's
rules keep sitometres and a directly launched Basecamp, and `lgs basecamp
setup`/`install`, out of local hands. Every box in 2 and 3 names the run that
ticked it and the head SHA it ran on. design.md D3 says why each run is waited
out before the next push.

### 1. The owner's rule, and the archive's claims it rests on

- [x] 1.1 CLAUDE.md names `yq` for YAML and `jq` for JSON, never Python, in
      "When you forbid a tool, name the replacement" (design.md D4). Verified
      by reading the diff: one entry, phrased by the approval click it costs
- [x] 1.2 No Python in the suite parses YAML or JSON: `git grep -n -i -e python
      -e "import yaml" -e "json.load"` over the files proposal part 1 lists
      returns only `check_qml_reachable.py`'s shebang, and that file reads QML
      (design.md D5)
- [x] 1.3 Archived D12's reasons for keeping `validate-ui-specs.mjs` re-read at
      the pinned tag `v0.1.2` (`6dc23e2`) rather than the fork commit the
      archive cites; all hold, and the citation is corrected in design.md D5

### 2. The no-key step goes red with the create affordance present

- [ ] 2.1 Break: `createBlockLoader`'s `active:` set to `true` in
      `DStoaListScreen.qml`, pushed alone. Predicted (design.md D2): the
      `sitometres join spec` job fails; the adjudicator prints `verdict: fail`,
      four `[pass]`, one `[fail]` on "a fresh profile is offered a key and not
      a Stoa", and three errors (verdict, `report has 5 steps, spec has 14`,
      the step by name). ci.yml's `qml` job also red, on exactly the three
      `tst_stoa_screens.qml` tests the break reddened locally (Qt 6.10.3,
      133 passed, 3 failed; `tst_navigation.qml` and `tst_render_probe.qml`,
      the other two specs that instantiate the screen, stayed green):
      `test_the_create_affordance_is_not_instantiated_when_no_key_is_held`,
      `test_a_failed_query_is_the_could_not_be_read_state_carrying_the_cores_words`
      and `test_a_reply_claiming_a_key_without_naming_one_is_not_the_key_held_state`
- [ ] 2.2 Revert pushed; both workflows green on it

### 3. The scaffold guard goes red on a value `lgs` itself changed

- [ ] 3.1 Break: an unknown key under `[modules.dialectica_ui]` in
      `scaffold.toml`, pushed alone. Predicted (design.md D1): "lgs left
      scaffold.toml's values alone" exits 1 with its `::error::`, its diff
      showing that key present before and absent after, and the job stops
      there, before sitometres. ci.yml green. Answers: `lgs` 0.3.1's own
      rewrite drops a key it does not know, and the real step in CI reports it
      as a changed value
- [ ] 3.2 Revert pushed; both workflows green on it

### 4. Hand-back

- [ ] 4.1 Branch tip is not a break: `git diff 8368b2f -- scaffold.toml
      dialectica-ui/src/qml/DStoaListScreen.qml` is empty, and both workflows
      are green on the tip
