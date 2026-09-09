{
  description = "Dialectica core — a decentralized forum on the Logos stack";

  inputs = {
    # Pinned >= 0.2.5: earlier builders deliver EMPTY binary event payloads
    # (PLAN.md §11), which for a forum syncing over delivery_module's
    # channelMessageReceived would mean every inbound op arrives blank. A tag
    # rather than a branch, so the generator and the SDK it stages cannot move
    # under this module without a commit here saying so.
    #
    # This one input supplies BOTH the lidl-gen generator and the logos-rust-sdk
    # source the crate links, at the same rev — so there is no generator/runtime
    # skew and this flake needs no logos-rust-sdk input of its own.
    # PINNED PAST THE NEWEST TAG, deliberately. 0.2.6 is the newest tag and it
    # CANNOT BUILD A RUST MODULE on a cold cache: `importCargoLock` fetches each
    # crate through the pinned nixpkgs' `fetchurl`, which sends no User-Agent,
    # and crates.io answers those with HTTP 403 — surfacing as
    #
    #     curl: (22) The requested URL returned error: 403
    #     error: cannot download crate-syn-3.0.5.tar.gz from any mirror
    #
    # which reads like a dead mirror rather than a policy. Reproduce the
    # distinction in one command: `curl -o /dev/null -w '%{http_code}'
    # https://crates.io/api/v1/crates/syn/3.0.5/download` gives 403, and the
    # same with `-A anything` gives 200.
    #
    # The radicle module works around this in its OWN flake with a second
    # nixpkgs feeding `fetchCargoVendor`. That escape hatch does not exist here:
    # on the `codegen.rust` path the BUILDER owns the vendoring, so a module's
    # flake has no override point. Pinning is the only fix available to us.
    #
    # 9f420c2 ("finish the crates.io wiring") routes crate fetches through the
    # CDN overlay and is what makes a Rust module build at all. It is on master
    # and in no tag. MOVE THIS TO THE FIRST TAG THAT CONTAINS IT — check with
    # `gh api repos/logos-co/logos-module-builder/tags` and
    # `git log 0.2.6..<tag> --oneline`.
    #
    # This commit is a descendant of 0.2.6, so PLAN.md §11's ">= 0.2.5" pin
    # (earlier builders deliver empty binary event payloads) still holds.
    logos-module-builder.url = "github:logos-co/logos-module-builder/9f420c2901e35a16ba8fc77383e796480000a1d2";

    # PHASE 0 EXPERIMENT — see docs/PHASE0-FINDINGS.md before keeping this.
    #
    # delivery_module v0.2.1 publishes no `lidl` flake output, so the
    # `dependency_overrides` entry in metadata.json points the generator at its
    # impl header instead (PLAN.md §3.2). One line, per PLAN.md §11.
    delivery_module.url = "github:logos-co/logos-delivery-module/v0.2.1";
    # Mandatory (PLAN.md §11): without it flake.lock gets two
    # logos-module-builder nodes, the stale one silently wins under
    # --override-input, and the build fails with "no 'main' field in
    # metadata.json".
    delivery_module.inputs.logos-module-builder.follows = "logos-module-builder";
  };

  outputs = inputs@{ logos-module-builder, ... }:
    logos-module-builder.lib.mkLogosModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
    };
}
