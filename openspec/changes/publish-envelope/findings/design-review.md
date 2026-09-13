# Design findings — `publish-envelope`

Reviewed at `510259c`, in a worktree of my own, which is now deleted. Dimension:
**design** — whether the code took the decisions `design.md` records, whether
decisions worth recording were recorded, and whether any of it contradicts
`docs/PLAN.md` **as it stands on `origin/main`**. Correctness, security,
readability, architecture and spec-test were reviewed before me and I did not
re-report their entries.

Suite green at **737 + 26**, run in my own worktree with the `-p` flags
(`cargo test --manifest-path …/dialectica/rust-lib/Cargo.toml -p dialectica -p
dialectica-core`). Every probe below was executed, not reasoned about, and every
one was reverted — `git status --porcelain` is empty in my worktree.

**The cited line had drifted.** `findings/architecture.md:80` points at
`wire.rs:7954`; the `;` precondition it is about is now at **`wire.rs:7978`**,
moved by `16af013`. The finding's substance still holds, which is why I am
closing it rather than retiring it.

## The box I was given

- [x] **`design-reviewer`** — `findings/architecture.md:80` — whether
      `design.md` should state the "a defaulted method is not on the wire"
      premise that the `spec-writer` just recorded in the spec

      **Resolved: `design.md` should NOT restate the premise — the spec is the
      right home for it — but decision 6 must stop claiming the opposite of what
      the code does, and the code must stop being silent.** Two of the three
      parts of this are still open, so they are boxes below rather than ticks
      here.

      The premise itself belongs in the spec and only there. It is a statement
      about **what the wire surface is**, which is `module-wire-contract`'s
      subject, and the spec-writer framed it correctly as *"we depend on upstream
      behaviour X, verified at this revision"* with `lidl-gen`'s Rust frontend
      and the revision `dialectica/flake.nix` pins. A second copy in `design.md`
      would be the failure `.claude/agents/README.md:53-54` names outright —
      *"two copies drift and the wrong one gets read"* — and it would drift in
      the worst direction, because a `design.md` is **archived and frozen** at
      merge while the spec is live and re-read on the next pin bump. A frozen
      copy of a claim whose whole value is that a pin bump invalidates it is a
      copy that cannot be invalidated.

      What `design.md` owes is different and is not the premise: decision 6
      states a **property of the classifier** that the classifier does not have,
      and that is decision 6's own subject. See the next two boxes.

## The code contradicts a recorded decision

- [x] **`dev-writer`** — `design.md:178-183` states a classifier property the
      code does not have, and the spec now requires the property the code lacks

      Decision 6's preconditions paragraph reads: *"it assumes rustfmt-shaped
      Rust, a declaration ending in `;`, and a request parameter typed `String`
      by value. **Anything else fails loudly instead of passing**"*. The
      function's own doc (`wire.rs:1931-1937`) says the same. That is true of two
      of the three preconditions and **false of the `;` one**, and it is now
      false against a live spec requirement rather than only against the prose.

      **Verified by execution, both directions.** I added `fn
      publish_moderation(&mut self, request: String) -> String { request }` to
      `DialecticaModule` and ran
      `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`:
      **it passed**, with a request-taking method declared on the dispatch trait
      and absent from `every_request_taking_method`. The silent `continue` is
      `wire.rs:7978-7980`. With a body containing a `;` — `{ let _ = request;
      String::new() }` — it *did* fail, but in the **returns** bucket:
      `["publish_moderation (returns \`-> String { let _ = request\`)"]`. So the
      one shape that fails does so by accident of where the first `;` fell, and
      reports a wrong diagnosis: a reader is told the method's *return type* is
      unrecognised when what is unrecognised is that it has a body.

      **What makes this a `dev-writer` box now rather than a note.** When
      architecture review filed it, the defaulted-method escape rested on an
      unverified comment, so the honest reading was "plausibly fine, record the
      premise". The `spec-writer` has since verified the premise **and turned it
      into a requirement**, and the code does not meet the requirement's second
      half. `specs/module-wire-contract/spec.md:141-143`:

      > Anything checking this contract's coverage of the surface SHALL be able
      > to state that assumption and SHALL **fail visibly rather than silently**
      > if a generator that no longer honours it puts a defaulted method on the
      > wire.

      Neither half holds. The classifier cannot *state* the assumption — I
      grepped `wire.rs` for `lidl-gen`, `rust_frontend`, "default body" and
      "defaulted": the only hit in the classifier is the bare comment at
      `wire.rs:7975-7977`, *"A defaulted method carries a body, so its
      declaration does not end at a `;`"*, which says what the code does and
      names no upstream behaviour, no generator and no revision. And it does not
      fail visibly: it drops the method with a `continue`, which is precisely the
      *"a filter's failure mode is silence"* shape the classifier rewrite exists
      to eliminate, surviving in the one branch the rewrite did not convert.

      **The fix is the one decision 6 already argues for**, applied to the branch
      that was missed: give the `;`-less arm a bucket rather than a `continue`.
      A third bucket — "has a default body" — pushed to `unclassified` would fail
      loudly naming the method, and the panic message is the natural place to
      state the assumption and its citation, which discharges both halves of the
      SHALL at once. That also fixes the misdiagnosis above, because a defaulted
      method would then be recognised as defaulted whether or not its body
      happens to contain a `;`.

      Then `design.md:181-183` can say what holds. Today it claims the property
      the fix would create, in a document that will be archived as the record of
      what was decided.

      **Fixed** in `d052d0a`, as the third bucket you prescribed. Both halves of
      the SHALL are discharged by the same mechanism, which is what your
      suggestion bought.

      **The arm no longer depends on where the first `;` falls.** It is decided
      on whether `{` or `;` comes first after the parameter list, so a defaulted
      method is recognised as defaulted whether its body is `{ request }` or
      `{ let _ = request; String::new() }`. That is what fixes the misdiagnosis
      as well as the silent pass — the body is never read as a return type
      again.

      **The panic states the premise with its citation.** It names the method,
      names `lidl-gen`, quotes the frontend's own doc sentence, and gives the
      `logos-module-builder` revision `dialectica/flake.nix` pins
      (`9f420c2901e35a16ba8fc77383e796480000a1d2`) — so an author who has just
      bumped the pin has the three things needed to decide whether the premise
      still holds. The set the premise excuses is
      `NOT_EMITTED_ONTO_THE_WIRE`, a list of names rather than a filter, which
      is the half that makes a NEW defaulted method red rather than an
      unnoticed subtraction from the swept surface.

      **The tests exist because the classifier now takes its source as a
      parameter**, and I want to flag that as the part of this fix I think
      matters most beyond the box. While it read `ADAPTER_SOURCE` directly, the
      only way to reach a branch was the mutation probe you and the architecture
      reviewer both had to write and revert — which is precisely why this arm
      was found twice and fixed neither time: nothing committed could reach it.
      `request_taking_methods_declared_in(&str)` is the classifier;
      `the_dispatch_traits_request_taking_methods()` is a one-line caller.

      **Each of the six proved failing first**, against the old arm restored:

      - `a_defaulted_method_is_recognised_as_defaulted_whether_or_not_its_body_has_a_semicolon`
        — four body shapes. Against the old arm the `;`-body case reproduced
        your exact string,
        `["publish_moderation (returns \`-> String { let _ = request\`)"]`, and
        the `;`-less case failed with `a defaulted method must not pass silently
        — body { request } did: ["ping"]`: the classifier returned only `ping`,
        silently dropping the method. It asserts the defaulted wording AND the
        absence of `(returns \``, so a future reordering cannot quietly restore
        the misdiagnosis while keeping the test green.
      - `an_unexcused_defaulted_method_names_itself_and_cites_the_generator_pin`
        — `should_panic`; against the old arm it "did not panic as expected".
      - `the_defaulted_panic_states_the_premise_and_its_citation` — asserts the
        message contains the method, `lidl-gen`, the revision and
        `NOT_EMITTED_ONTO_THE_WIRE`. It exists because a `should_panic` matcher
        reads a prefix and would not notice the citation being dropped, which
        is the half the spec actually requires.
      - `the_defaulted_bucket_excuses_only_the_methods_the_generators_premise_covers`
        — the other direction: `on_context_ready` must still pass.
      - `the_classifier_buckets_each_declaration_shape` and
        `a_borrowed_request_parameter_fails_loudly_rather_than_passing` — the
        buckets that were already right, now pinned from a committed test
        instead of from a probe someone has to remember to revert.

      `design.md` decision 6 now says what holds, names the false claim as
      false, and records the spec obligation it discharges.

## Decisions made and not recorded

- [x] **`dev-writer`** — `design.md` records neither of the two substantive
      decisions the `spec-writer` made in `510259c`, and one of them changes a
      decision `design.md` already carries

      `510259c` made two choices with real alternatives, argued both at length in
      `proposal.md:120-166`, and `design.md` was not touched by that commit —
      `git log -- openspec/changes/publish-envelope/design.md` shows its last
      edit is `d055c0c`, two commits earlier.

      **Choice one: the request bound was promoted into the spec rather than the
      proposal's claim being corrected.** The alternative was live and cheaper —
      spec-test review found the change's headline claim one-third wrong (two of
      the three "contract obligations" were the contract's, the third was in no
      merged spec at all), and softening the prose would have closed the finding.
      What ruled it out is that *the property is the ordering*: a bound checked
      after the parse bounds nothing, because the ~2N allocation it refuses has
      already been paid, and per PHASE0-FINDINGS §3 what that costs is the module
      process rather than the call. That argument is a design decision by every
      test in this role's brief — a real alternative, a constraint that forced
      the question, and a cost (a contract obligation that every future
      request-taking method inherits).

      **Choice two: the spec states a bracket, not the number 4 MiB.** Also a
      real alternative — the number exists, is derived with arithmetic shown in
      `MAX_REQUEST_BYTES`'s doc, and putting it in the spec would have been the
      obvious move. It was refused because *a number in a spec is a claim no gate
      reads*. What it costs is that the spec cannot be violated by a bad number,
      only by an absent or unbracketed one, and the compensating obligation —
      *"An implementation SHALL record where its number sits between those two
      bounds"* (`spec.md:33-34`) — is discharged in a doc comment rather than by
      a gate.

      **Why this is not merely tidy-up.** `design.md:48` currently carries, in
      decision 2's table, the row `| A size cap | none; a request over 4 MiB was
      parsed | refused before the parse |`. That row is now the *only* statement
      in `design.md` about the cap, and it reads as an implementation detail
      inherited from `Request::parse`. It is no longer that: the ordering is a
      contracted obligation across the whole request-taking surface, and the
      number's absence from the spec is deliberate. A reader of the archived
      change folder — which is where a future agent goes to find out why — would
      learn from `design.md` that a cap was inherited, and nothing about why it
      was specified as a bracket or why the ordering is the requirement.

      One entry under Decisions covering both, with the alternatives named
      (soften the claim; state the number), closes it. `proposal.md:120-166`
      already has the prose; the work is deciding what belongs in a decision
      record versus a proposal, not writing it from scratch.

      **Fixed** in `d052d0a` as decision 9, "The request bound is promoted into
      the contract, and as a bracket rather than a number" — one entry, both
      choices, in the shape you asked for.

      Both alternatives are named as live rather than as strawmen. For the
      promotion: softening the prose was cheaper and would have closed the
      spec-test finding outright, and what ruled it out is that the property is
      the *ordering* — a bound checked after the parse bounds nothing, and per
      PHASE0-FINDINGS §3 the cost is the module process rather than the call.
      For the bracket: the number exists and is derived with arithmetic shown,
      so stating it was the obvious move, refused because a number in a spec is
      a claim no gate reads.

      **The costs are what I took most from your framing**, since a decision
      record naming only the reasons is a justification rather than a decision.
      The promotion costs a contract obligation every future request-taking
      method inherits, including from authors who never read this folder. The
      bracket costs a spec that cannot be violated by a *bad* number, only by an
      absent or unbracketed one, with the compensating "record where your number
      sits" obligation discharged in a doc comment rather than by a gate. I
      wrote that second one as a real weakness rather than a neutral trade,
      because it is one.

      **Decision 2's table row is now pointed rather than left to mislead.** You
      were right that it was the only surviving statement about the cap and read
      as an inherited implementation detail. I did not rewrite the row — it is
      accurate about the code, and the table's job is the before/after — but it
      now carries a paragraph saying it describes the code rather than the whole
      decision, and naming decision 9 as where the choice and its costs live.

      While there I corrected a citation in the same paragraph:
      `an_oversized_request_is_refused_before_it_is_parsed` was cited bare, and
      it lives in `dialectica-core/src/wire/request.rs`, not in `wire.rs` where
      a reader would look first. Verified by grep before editing.

## An entry that is thinner than the code it describes

- [x] **`dev-writer`** — `design.md:11-13` claims a guarantee the type does not
      give, and the concession that would have been honest is absent

      Decision 1 closes: *"`PublishRequest::parse` runs the envelope, the
      forbidden-field guard and the `stoa` read, so those are not something a
      handler calls — they are what constructing the value is. **A handler
      holding a `PublishRequest` provably went through all three.**"* The same
      sentence is in the type's own doc at `wire.rs:1953-1956`.

      **Verified: it compiles.** `PublishRequest` (`wire.rs:1991-1996`) is a
      private struct whose two fields are private-to-module, and every publish
      handler lives in that same module, so a handler can build one directly. I
      inserted this into `publish_post` and it built clean:

      ```rust
      let _bypass = PublishRequest {
          stoa: crate::identity::Address::from_bytes([0u8; 32]),
          fields: Request::parse(request).unwrap(),
      };
      ```

      That value went through **one** of the three: `Request::parse`, which the
      `fields: Request` type does enforce. `reject_forbidden_fields` and
      `parse_stoa` are both bypassed, and the `stoa` is an address the request
      never named.

      **Severity: low as a live risk, and I want to be exact about why, because
      overstating it would be the same error as the sentence.** No handler does
      this; `publishing` (`wire.rs:2089`) is the only constructor call site and
      it goes through `parse`. The exposure is a future handler written inside
      `wire.rs`, which is where handlers go. So this is not the headline-defect
      shape — it is the recorded *reason* being stronger than the mechanism, in a
      decision whose whole subject is preferring a data shape over a checked
      branch.

      This repo has already been bitten by exactly this and the precedent is
      worth citing rather than re-deriving: `every_request_taking_method`'s doc
      (`wire.rs:7694-7702`) makes the honest version of the same concession —
      *"**What the compiler still cannot force**, so that the sweep is not read
      as covering more than it does: nothing obliges a handler to hold a
      `Request` at all"* — and names the test that demonstrates it,
      `the_sixth_method_the_boundary_does_not_stop`. Decision 1 needs the
      equivalent clause: constructing a `PublishRequest` through `parse` is the
      only route a call site takes and the only one the crate's tests exercise,
      but the struct literal is reachable inside `wire.rs`, so the guarantee is
      a convention the module boundary does not enforce. Say which of the three
      the type does enforce (`Request::parse`, via the `fields` type) and which
      two it does not.

      **Fixed** in `d052d0a`, in both places the sentence appeared — `design.md`
      decision 1 and `PublishRequest`'s own doc — because leaving the doc
      standing is the failure family this file already records elsewhere.

      Both now say exactly what you asked: **one** of the three is forced, the
      envelope, because `fields` is a `Request` and a `Request` cannot exist
      without `Request::parse`; `reject_forbidden_fields` and `parse_stoa` are
      run by `PublishRequest::parse` and by nothing the type insists on, so a
      struct literal written inside the module skips both and can name a `stoa`
      the request never carried.

      I kept your severity calibration rather than flattening it, because the
      calibration is the finding's point: `publishing` is the only construction
      site and it goes through `parse`, so this is the recorded *reason* being
      stronger than the mechanism inside a decision whose subject is preferring
      a data shape over a checked branch — and overstating it would be the same
      error as the sentence. The doc says the exposure is the future handler
      written inside `wire.rs`, which is where handlers go.

      **No test.** I want to be explicit rather than let the unticked-box
      convention imply one exists: the corrected claim is *"the type does not
      force this"*, and a test cannot demonstrate the absence of a compiler
      guarantee — the code that would prove it is code that compiles, which is
      what you already demonstrated by compiling it. What would make it testable
      is making it true, and that is recorded rather than done: decision 1 now
      names the available fix (a private constructor behind a module boundary,
      forcing all three by construction) and defers it with decision 8's
      reshape, on the architecture reviewer's argument that both move the same
      three signatures and doing them apart pays the `Handler`-type and
      sweep-fixture cost twice.

## PLAN.md

**No contradiction with `origin/main`'s PLAN.md, and I read that copy rather than
the branch's.** I also checked the direction this role exists to catch — a change
reasoned against a superseded section. There is nothing to supersede: searching
`git show origin/main:docs/PLAN.md` for `MAX_REQUEST_BYTES`, `4 MiB`, `lidl-gen`,
`request size`, `size cap` and `defaulted` returns **zero** hits on every one of
them. The cap, the bracket and the generator premise originate in this change;
none of them was migrated out of PLAN.md, because none of them was ever in it.

**The §9.2 migration is done correctly, with one wrinkle worth naming and not
worth a box.** The three-bullet "publish prologue/tail wants reshaping" paragraph
is struck through as done, the unbuilt half is written out with the concrete
attack input (`{"stoa":"<valid hex>","author":"x"}`), and it closes with *"whose
`design.md` decision 8 carries the full argument"*. That pointer is the right
shape. The wrinkle is that PLAN.md's replacement is **longer** than what it
replaced and now restates the signature-change cost — `handler` takes
`&SecretKey`, the fallible supplier, the `Handler` type test, the sweep fixtures
— which `design.md` decision 8 also states. Two copies of a reasoning is what
README.md:53-54 warns about. I am not filing it, for a reason I want on the
record so the next reviewer does not re-open it: the deferred fix is **not built
yet**, which is exactly what PLAN.md is for, and the forward reference makes
which copy is authoritative unambiguous. If a later change builds it, that change
should cut PLAN.md's copy to a line rather than leaving both.

**One pre-existing PLAN.md claim this change silently made true**, recorded so
nobody spends the afternoon I nearly did. `docs/PLAN.md:4224` (and
`origin/main:docs/PLAN.md:4304`, so it predates the branch) says *"the pathless
one has no production caller left"*. That was **false when written** — the
adapter was calling `keystore.stoa_key`, which is `derive_stoa_key`, the pathless
scheme (`keystore.rs:798-799`). Decision 4 deleted that call, so the sentence is
true as of this branch and `stoa_key` now appears in the adapter only inside a
comment at `lib.rs:580`. No action: the change did not touch the line, and
correcting a sentence that is now accurate would be churn.

## What else I checked and found clean

**Every test name `design.md` cites exists**, checked by grep rather than
assumed: `an_oversized_request_is_refused_before_it_is_parsed`,
`the_adapters_early_stoa_read_crosses_the_same_envelope_the_handler_does`,
`the_key_a_publish_signs_with_is_the_identity_the_probe_reports`,
`the_three_handlers_share_one_signature_the_adapter_can_dispatch_over`,
`the_publish_body_cap_is_the_format_field_cap` and
`one_field_has_one_null_reading`. No citation in `design.md` points at a test
that does not exist.

**Decision 6's counts are right**, which I checked because a count on a sibling
branch was wrong by seventeen. The trait declares 16 methods; 14 take a request
and return `String` (every one but `version`, which takes none, and
`on_context_ready`, which is defaulted). So *"the other fourteen still parse"*
and *"five sweeps ran green over eleven methods while the surface had fourteen"*
are both accurate against the declaration as it stands.

**Decisions 3, 5 and 7 match the code.** `required_stoa` is gone and `parse_stoa`
(`wire.rs:468`) is the single reader, so decision 3's "two readers cannot
disagree" is a property of the code. `stoa_of` (`wire.rs:2055-2058`) is
`Request::parse` then `parse_stoa` and reads nothing else, which is decision 7's
claim exactly — the forbidden-field guard and every required-field read are still
the handler's.

**`docs/UI-BRIEF.md` is not made wrong by this change**, which I checked rather
than assumed given it is a live document. The change alters which requests are
refused and what those refusals say; the brief's publish section
(`UI-BRIEF.md:390-433`) already carries the general obligation — *"Surface the
reason instead"*, and the draft must survive a refusal — and the two new refusals
(over the size bound, non-object) are caller-shape errors a UI composing a draft
cannot produce. Nothing in the brief claims a behaviour this change falsifies.

## What I could not check

`dialectica/rust-lib/src/lib.rs` is `cfg(logos_scaffold)` and no runnable gate
compiles it, so both of my trait probes are reads of a file `cargo test` does not
build — what they proved is what the **classifier** does when the trait
declaration contains that text, which is the whole of what the classifier reads,
so the probe is sound for the property at issue. It says nothing about whether
the adapter still compiles: only Build LGX sees that, and it proves compilation
rather than dispatch.

`cargo fmt --check` does not follow path dependencies, so it never reaches
`dialectica-core`, where every claim in this review lives.

I did not run Build LGX, `nix build`, or the module against a live basecamp host.
The `include_str!` path's survival of the Nix staging — decision 6's *"the path
survives the Nix build"* — is therefore argued from `mkLogosModule.nix` rather
than measured by me, as the architecture reviewer also recorded. Its failure mode
is a compile error rather than a silent pass, which is the acceptable version of
that gap.

I did not re-verify the `lidl-gen` reading behind the spec's premise. My brief
records it as confirmed — `lidl-gen/src/rust_frontend.rs:350-354` does `if
f.default.is_some() { continue; }` — and my boxes above do not depend on it being
true: the first one is about the classifier failing *silently* whether or not the
premise holds, which the spec requires independently.
