{
  description = "Dialectica UI — a QML view over the dialectica core module";

  inputs = {
    # Same pin as dialectica/flake.nix, and it has to be: the two are built
    # together and a skew between them is a skew between the contract the core
    # publishes and the one this module's dependency wrapper is generated from.
    # See dialectica/flake.nix for why this is a commit rather than 0.2.6.
    logos-module-builder.url = "github:logos-co/logos-module-builder/9f420c2901e35a16ba8fc77383e796480000a1d2";

    # ONE LINE, deliberately (PLAN.md §11): scaffold's override parser is
    # line-based and cannot see a multi-line `inputs.x = { url = …; }` form, so
    # `lgs basecamp build` would fail to derive the sibling --override-input.
    dialectica.url = "path:../dialectica";

    # MANDATORY (PLAN.md §11). dialectica itself depends on
    # logos-module-builder, so without this follows, flake.lock carries TWO
    # builder nodes. Under --override-input the stale one silently wins and the
    # build fails with "no 'main' field in metadata.json" — an error that names
    # nothing to do with the actual cause.
    dialectica.inputs.logos-module-builder.follows = "logos-module-builder";
  };

  outputs = inputs@{ logos-module-builder, ... }:
    # mkLogosQmlModule, not mkLogosModule: this is a view.
    #
    # There is no CMakeLists.txt and no C++ in this module at all. A QML-only
    # view declares `view` and omits `main`, so nothing is compiled — the
    # builder copies the QML tree beside the manifest. That is the Phase 0
    # shape on purpose: it keeps the view provably thin, and it means the whole
    # UI half of Phase 0 cannot accidentally acquire logic that belongs in core
    # (PLAN.md §2.1).
    logos-module-builder.lib.mkLogosQmlModule {
      src = ./.;
      configFile = ./metadata.json;
      flakeInputs = inputs;
    };
}
