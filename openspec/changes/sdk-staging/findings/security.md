# Security review — #168 sdk-staging

Dimension covered: **security only** (correctness, readability and
architecture are each a separate reviewer's row).

Read `gh issue view 168 --json body,comments` before starting, including the
owner's 2026-09-25 comment that corrects the issue body (the local-run concern
under "The problem" was withdrawn, and `--inputs-from ./dialectica` is
confirmed as the mechanism, superseding the proposal's earlier `flake.nix`
re-export draft).

## What I checked

This piece is docs (`README.md`, `.gitignore`, `CLAUDE.md`) and one CI step
(`.github/workflows/ci.yml`, the `rust` job's "Stage the logos-rust-sdk source
the builder pins" step). No application code, wire format, storage or
moderation logic is touched, so the "never trust an inbound message" /
"moderation must be authenticated" standing rules do not apply here — there is
no peer-input boundary in this diff.

The specific security question this kind of change raises is: **does
replacing the Python lock-parser + explicit `github`/`rev` assertions with
`nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src`
weaken how the builder pin is resolved, e.g. by letting CI silently fall back
to an unpinned or attacker-influenceable builder?**

I did not just read the claim in `design.md` D1 — I reproduced it:

- Ran the actual staging command from this tree's root:
  `nix build --inputs-from ./dialectica logos-module-builder#rust-sdk-src -o dialectica/logos-rust-sdk-src`.
  It resolved to `/nix/store/lsdgw1fqrxd6zcwd9i05gviv8dj9ljn3-logos-rust-sdk-src`,
  matching the store path `design.md` and the issue's owner comment both cite
  as measured on 2026-09-25.
- Independently re-ran the negative case design.md cites as the security
  guard: `nix build logos-module-builder#rust-sdk-src -o /tmp/should-not-exist`
  **without** `--inputs-from`. Result: `error: cannot find flake
  'flake:logos-module-builder' in the flake registries` — it fails closed, it
  does not fall back to a same-named entry from the global flake registry or
  build anything unpinned. This is the property that matters: a PR that
  strips `--inputs-from` from the CI step breaks the build rather than
  silently building against a different, attacker-reachable
  `logos-module-builder`.
- Confirmed the `test -f dialectica/logos-rust-sdk-src/Cargo.toml` guard after
  the `nix build` is kept (diff shows it unchanged), so a builder output whose
  shape changed still fails at the staging step rather than surfacing as an
  unrelated manifest error later.
- Confirmed `set -eu` is retained in the step's `run:` block, so a failed
  `nix build` fails the step rather than continuing to the `test -f` on stale
  state.
- Checked the workflow's trigger (`on: push` to `main`/tags, `on: pull_request`
  to `main` — not `pull_request_target`) is unchanged by this diff, so the
  trust boundary for what can influence `dialectica/flake.lock` (and hence
  which builder pin gets resolved) is the same one that existed before this
  piece: whoever can land a change to `flake.lock` could already point CI at
  an arbitrary `github:` rev under either the old Python-based step or the new
  one. This piece does not change who can do that or what token/secrets
  posture the job runs under — it only changes *how* the already-locked rev is
  resolved, and the resolution mechanism was independently verified to fail
  closed.
- Read the `Risks / Trade-offs` section of `design.md`: it explicitly
  discloses an unmeasured edge case (a `~/.config/nix/registry.json` entry
  named `logos-module-builder` could in principle compete with
  `--inputs-from`, precedence untested). This is honestly disclosed rather
  than hidden, and is not exploitable by a peer/PR author — GitHub Actions
  runners are ephemeral with no pre-existing user registry, and on a
  developer's own machine such an entry would require the developer's own
  config to already be compromised, at which point this command is not the
  weak point. I checked this box mentally as "acceptable, disclosed residual
  risk," not as a defect to flag.
- Ran the two commands the brief required as verification, both from this
  tree: `cargo test --manifest-path dialectica/rust-lib/Cargo.toml -p
  dialectica -p dialectica-core` (1180 + 30 passed, 0 failed) and `nix build
  ./dialectica#lgx` (clean exit, confirming the `CLAUDE.md` correction from
  `.#lgx` to `./dialectica#lgx` is in fact necessary and correct — there is no
  root `flake.nix`, confirmed with `git ls-files flake.nix` returning
  nothing).
- Confirmed no new external dependency, action, or registry is introduced —
  the diff *removes* a `python3` heredoc dependency from the CI step and adds
  none. No pinned-action SHAs, `permissions:` blocks, or trigger conditions in
  `ci.yml` are touched by this diff.

## Findings

None. This piece does not weaken the builder-pin resolution, does not widen
CI's trust boundary or token posture, introduces no new dependency, and its
one load-bearing security-relevant claim (fail-closed without
`--inputs-from`) was independently reproduced rather than taken on the
proposal's word.
