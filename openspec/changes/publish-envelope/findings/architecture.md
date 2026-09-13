# Architecture findings — `publish-envelope`

Reviewed at `d055c0c`, in a worktree of my own. Dimension: **architecture** (a
sibling file carries readability; correctness and security were reviewed before
me and I did not re-report their entries). Suite green at **737 + 26**. Every
probe below was run in my own worktree and reverted; the worktree is deleted.

- [x] **`dev-writer`** — `dialectica/rust-lib/src/lib.rs:804-814` — the three
      publish arms are three hand-written (name, function) pairs that must agree,
      and this file has already been bitten by exactly that
      **Scenario:** `self.publishing(&request, "publish_post", core::publish_post)`
      is written three times, and the string and the function are independent
      arguments. `self.publishing(&request, "publish_vote", core::publish_post)`
      compiles, runs, publishes a post, and reports any panic in the keystore
      open, the path resolve or the SQLite open as `panic in publish_vote`. No
      gate can see it: `cargo test` does not compile this file, the CI text gates
      check only which `core::` names appear, and Build LGX proves it compiles.
      Worse, the name is then **doubly** supplied — the adapter's
      `core::guarded(method, …)` at 507 wraps `core::wire::publishing`, which
      calls `guarded(method, …)` again at 2088 with its **own hardcoded** name.
      So the guard is nested and the outer name is a second, caller-supplied copy
      of something the inner one already knows.
      **Measured:** this is not a hypothetical shape — it is a defect this repo
      has already found and fixed once, in this same crate, and the fix is
      written down eleven lines from here. `with_membership_store`'s doc
      (`wire.rs:2288-2302`) records it: *"This used to take the method name and
      pass it to `guarded`, while each of the three handlers **also** called
      `guarded` with its own hardcoded name ... It could disagree and nothing
      noticed: `with_membership_store("list_stoas", …, |s| create_stoa(…, s))`
      compiled, ran, and reported a panic in `open` as `panic in list_stoas`.
      Five hand-written pairs had to be kept in step for no gain."* The fix
      applied there was to drop the parameter and give the outer guard a
      **generic label**, because the only panics it can catch that the inner one
      cannot are in the open. That reasoning transfers here exactly: the adapter's
      outer guard can only ever catch `open_from_env`, `Self::paths` and
      `SqliteOpLog::open`. **Severity: medium.** `publish_moderation` makes it
      four pairs, and this change's whole thesis is that a fourth copy is the
      signal to reshape. The fix is the one already proven in this crate: delete
      `publishing`'s `method` parameter and label the outer guard
      `"opening the publish path's stores"` or similar.

      **FIXED**, exactly as prescribed, including your suggested label.
      `publishing`'s `method` parameter is gone, the outer guard reads
      `core::guarded("opening the publish path's stores", …)`, and the three call
      sites are now `self.publishing(&request, core::publish_post)` and its two
      siblings. There is nothing left for a pair to disagree about: the only
      argument that names the operation *is* the operation.

      Citing `with_membership_store`'s own doc is what made this a five-minute
      fix rather than a judgement call — the argument was already made, in this
      crate, with the measured symptom (`panic in open` reported as `panic in
      list_stoas`) attached. I had written a comment eleven lines away from that
      doc and reintroduced the shape it records fixing, which is the "unfixed
      patterns get copied" failure in the direction nobody expects: the fixed one
      was the neighbour.

      `publishing`'s doc now carries the same section heading the precedent uses
      — *"There is no `method` parameter, and that is the fix for a parameter
      nobody could keep right"* — with your `publish_vote`/`publish_post`
      mismatch as the worked example, and states what the generic label is
      accurate for: the outer guard can only ever catch `core::stoa_of`,
      `open_from_env`, `Self::paths` and `SqliteOpLog::open`, because everything
      after that is inside `core::wire::publishing` where the inner guard names
      the method.

      **No test can fail on this**, and that is worth stating rather than
      glossing: `cargo test` does not compile this file, so the mismatch you
      constructed was never catchable by a test and is not now. What changed is
      that it is no longer *expressible* — there is no name argument to get
      wrong. That is the CLAUDE.md preference for a data shape over a checked
      branch, applied to the one file where a checked branch could never have
      been checked.

      While in there I also corrected a stale claim in the same doc: it still
      said threading a parsed Stoa in from the adapter "would put half the
      request's validation in the one file no test can reach", which `stoa_of`
      resolved — the read goes through `core` now, and the remaining reshape is
      decision 8's.

- [ ] **`design-reviewer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:7954`
      — the classifier's `;` precondition is enforced by a silent `continue`,
      not by the loud bucket the design claims
      **Scenario:** `design.md` decision 6 and the function's own doc both state
      the preconditions as *"rustfmt-shaped Rust, a declaration ending in `;`,
      and a request parameter typed `String` by value. **Anything else fails
      loudly instead of passing**"*. That is true of two of the three and false
      of the `;` one. A declaration with no `;` takes `let Some((params_and_ret,
      _)) = rest.split_once(';') else { continue; }` — a silent drop, the exact
      failure mode (*"a filter's failure mode is silence"*) the classifier was
      rewritten to eliminate.
      **Measured:** I added
      `fn publish_moderation(&mut self, request: String) -> String { request }`
      to the dispatch trait and ran
      `the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`:
      **it passed**, with a request-taking method on the trait and absent from
      `every_request_taking_method`. With a body containing a `;`
      (`{ let _ = request; String::new() }`) it *did* fail — but in the
      **returns** bucket, reporting
      `["publish_moderation (returns \`-> String { let _ = request\`)"]`, i.e. it
      caught it by accident of where the first `;` fell, not because it
      recognised a body. **Severity: low as a live risk, medium as a claim.**
      The trait's own comment at `lib.rs:287-288` says the generator *skips
      defaulted methods when deriving the `.lidl`*, so a defaulted method is
      plausibly not on the wire and the sweep would owe it nothing — which is why
      this is not a `dev-writer` box. But that is the claim the silent `continue`
      is resting on, and the design records the opposite property. Either the
      `else` arm should push to `unclassified` with "has a default body" (loud,
      matching the claim), or decision 6 should say a defaulted method is
      deliberately skipped **because the generator skips it**, with the
      `.lidl`-derivation claim cited rather than assumed.

## Judgements the brief asked for

**Is a CI text gate the right instrument for the ordering property?** For now,
yes — and the change is honest about why, which is the part that matters. The
property is "the adapter's first touch of the request bytes goes through
`Request::parse`", and the three candidate instruments are: a unit test (cannot
reach the file — `cfg(logos_scaffold)`, and the dependency points the wrong
way), Build LGX (compiles the file and asserts nothing about statement order —
`tasks.md` 7.6 states this and leaves the row unticked deliberately), or a text
gate. A text gate is the only one available, and the change did not stop at it:
it paired the *requirement* (`core::stoa_of` present) with a *ban*
(`serde_json::from_str` absent), which together are much stronger than either.
The ban is the better half, because it closes the shape rather than the
instance — a future author cannot satisfy the requirement and then add a second
parse beside it.

**What is structurally available and was not taken**: making `Request::parse`
the only way to obtain the bytes at all, e.g. having the adapter hold a
`Request` rather than a `&str` and passing that to the handlers. That is
strictly the better shape — it makes the ordering hold by construction, which is
CLAUDE.md's "complexity in the data structure" rule — and it is *also* the
supplier-closure reshape of decision 8 wearing a different hat, since both move
the handlers' signatures. Deferring one and not the other would be arbitrary, so
deferring both together is coherent. I would ask that decision 8's follow-up
piece consider taking a `&Request` at the same time as the key supplier, since
the two reshapes touch the same three signatures and doing them separately pays
the `Handler`-type and sweep-fixture cost twice.

**The deferral of "validate, then unlock": correct, and recorded correctly.**
The argument is about the *shape* of the fix and not its size, which is the
right axis: `handler` takes `&SecretKey`, so reordering means a fallible
supplier, which moves three signatures, the `Handler` type test and every sweep
fixture — a second reshape of the same three functions inside a diff that
already reshaped them once. That is precisely the "two commits, not one" rule
this change otherwise obeys well. Three things make the deferral sound rather
than convenient: the claim was **withdrawn** rather than narrowed (design.md §1
now says the ordering is *unchanged on every path* and names both findings); the
outstanding half is written into `docs/PLAN.md` §9.2 with the concrete attack
input, so it survives the deletion of `findings/` at merge; and §9.2's text is
self-invalidating in the way CLAUDE.md asks — the reshape half is struck through
as done and the unlock half is prose about a specific call at a specific line.
I checked the PLAN.md diff: it does all three.

**The rejected alternative — guards in the adapter before the unlock — was
rejected correctly.** Running `reject_forbidden_fields` and the required-field
reads in the adapter would cut the DoS today, and it would put a second reader
of "what a publish request must contain" into the one file no test compiles.
That is the same two-readers-of-one-field shape that produced the headline
defect this piece exists to fix, and the same one decision 3 deleted
`required_stoa` to avoid. The asymmetry the design draws is the right one: a
bounded, measurable CPU cost versus an unbounded correctness risk in an untested
file. Note the argument has teeth precisely because `stoa_of` was held to
reading one field — had the adapter been allowed to validate "just a bit more",
this rejection would have had no principle left to stand on.

**The derived sweep list is a sound shape, not a clever one**, and I probed for
the cleverness. Three properties decide it. (1) The authority is right:
`DialecticaModule` is what `interface: "universal"` derives the RPC table from,
so "declared there" and "on the wire" are the same set by construction, not by
convention. (2) The overturning of the "structurally impossible" claim is
legitimate — both recorded objections were about *compiling* the trait, and
`include_str!` compiles nothing; decision 6 says which part of the old analysis
was unconsidered rather than wrong, which is the honest framing. (3) The failure
direction is right in **both** halves: an unrecognised shape panics naming the
method, and `a_served_request`'s catch-all arm panics `no served request known
for {other}` (`wire.rs:8144`), so adding a method to the list without a fixture
also fails loudly. A derived list whose derivation errs toward red is not the
same object as a hand-maintained one, and this one errs toward red everywhere I
could reach except the `;` case in my second box.

**One coupling worth naming, not a defect.** `ADAPTER_SOURCE` is
`include_str!("../../src/lib.rs")`, so `dialectica-core`'s test suite now depends
on the adapter crate's file layout. The doc argues the path survives the Nix
build (`mkLogosModule.nix` stages `rust-lib/` with `cp -r`, this crate nested
inside) and that the nesting is already load-bearing. I could not run that build,
but the argument is structurally right: a broken path is a **compile error**, not
a silent pass, so this coupling cannot fail quietly. That is the acceptable
version of this trade.

## What else I checked and found clean

**The refactor is behaviour-preserving in shape as well as in outcome.**
`PublishRequest::parse` runs parse → `reject_forbidden_fields` → `parse_stoa`,
which is the order all three handlers had; `publishing` wraps `guarded` at the
same scope; the per-field read order inside each closure is preserved, which
matters because the first failing read is the message a caller gets.

**`stoa_of` and `PublishRequest::parse` cannot drift on the Stoa read**, because
both call the single `parse_stoa` (`wire.rs:468`) rather than re-deriving. The
"two readers cannot disagree" claim is a property of the code and not of the
prose asserting it.

**Both new gates fire against the commits they claim to.** I ran the gate's own
logic against `35fc859` (fails: `core::stoa_of` missing, 1 banned parse) and
against `origin/main` (fails: `publishing_key` missing, `stoa_of` missing,
`stoa_key` accessor, 1 banned parse). On this tree it passes. On the brief's
question of a tunable matcher: neither gate is tunable toward green without the
edit being visible in the same diff — `want` is a literal tuple and the ban is a
`re.findall` whose only relaxation is deleting it, and the surrounding comment
says *"If a new one is genuinely needed, add it to `core` ... rather than
relaxing this."*

**No new dependency** is introduced by any commit on this branch.

## What I could not check

`dialectica/rust-lib/src/lib.rs` is `cfg(logos_scaffold)` and compiled by nothing
runnable here, so finding 1 is a reading of the call sites rather than an
execution of them. I did not run Build LGX or exercise the module against a live
basecamp host, so the `include_str!` path's survival of the Nix staging is
argued rather than measured. I did not verify the claim that the `.lidl`
generator skips defaulted methods; that claim is load-bearing for my second box
and its only source in the repo is the trait's own comment at `lib.rs:287-288`.
