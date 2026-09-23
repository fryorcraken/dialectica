No findings. `design.md`'s Decisions (D1–D9) were checked against the code, against issue #149's text (`gh issue view 149`), and against the delta specs, and each holds:

- D1 (option (a) chosen over (b)): `Main.qml` has no `onboarding` state, no `createIdentityFor`/`closeOnboarding`/`identityWasKept`, and no mounted `DOnboardingScreen`; `qmldir` carries the `# UNINSTANTIATED:` record; `tst_navigation.qml`'s `test_the_per_stoa_onboarding_screen_is_instantiated_nowhere` walks the object tree by type and passes.
- D2 (`generateIdentitySlate`/`keepIdentity` stay live, unread by the identity-in-use path): confirmed by `keep_selection` and the onboarding wire tests; the two new tests D2 says were missing (a slate generated, and a selection refused, on a peer that already holds a machine key) exist — `generating_a_slate_writes_nothing_new_on_a_peer_that_already_holds_a_machine_key`, `an_out_of_range_selection_stores_nothing_new_on_a_peer_that_already_holds_a_machine_key`.
- D3/D4 (no Stoa/record parameter; one derivation position): `posting_identity(keystore)`, `publishing_key(keystore)`, `whoami_for(master)` take no Stoa and no record; `get_capabilities_from_stores` opens only the keystore. `NO_CHOICE_FOR_THIS_STOA` no longer exists anywhere in the tree.
- D5: confirmed gone (grep above).
- D6 (`whoAmI` drops `path`, keeps `recoveryNeedsTheRecord: false`): matches `Whoami::to_json`/`whoami_for`.
- D7 (Stoa still parsed, wire does not widen): `who_am_i`/`get_capabilities` still call `parse_stoa`/require it; adapter's `publishing` still calls `core::stoa_of` before opening the keystore; `ci.yml`'s adapter-gate comments were updated in this same change to say why the call remains for ordering rather than derivation.
- D8 (undo of a keystore a failed keep wrote, gated by `wrote_the_keystore`): matches `keep_selection`/`undo_a_keystore_this_keep_wrote`; the tester's mutation note (`if wrote_it {` → `if true {`) is recorded in the same file with a measured result, and `git log` shows the tester commit is real, not aspirational.
- D9 (view routes a missing identity to the Stoa list; chip caption reverts to `copy.json`'s `common.createIdentity`): matches `Main.qml`'s `acquireIdentity()` (`enterOnly("", null)`) and `DIdentityChip.qml`'s button text.

The issue's audit bullet ("audit whether `stoas.sqlite`/`chosen_paths` already holds per-Stoa rows") is answered concretely in `design.md`'s "Existing data: the audit, for #108" — the actual store path (`identity.sqlite`, not `stoas.sqlite` as the issue guessed), the read-only query used, and the one row found, hex `stoa` and `path` given verbatim, with an explicit statement of what #108 still has to decide about it. `proposal.md`'s "Existing data" section carries the accompanying behavioural decision (the row stays and is ignored; consequences for revision and vote de-duplication are named and explicitly not repaired here).

No reasoning found in issue #149 that is absent from `design.md`: the two-option framing, the "compounding evidence" complaint about the intro-phase "Show me some keys" button, and the two-Stoa regression test requirement are all present under D1–D3. Issue #108 and #136 are referenced only as forward pointers to later milestones and are not reasoning this change consumed.

`cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core` passes in full (1094 + 30 tests) on this tree, confirming the decisions above are not merely narrated.

No mutation was introduced or reverted by this review; `git status --short` shows only this findings file.
