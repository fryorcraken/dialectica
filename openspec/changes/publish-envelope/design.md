# Design

## Decisions

### 1. The prologue is a type, and it lands as its own commit

The three handlers each opened with the same four statements. CLAUDE.md names
the threshold: *"when you find yourself writing the fourth slightly-different
copy of a guard, that is the signal to reshape rather than to add a fourth
test"*. `PublishRequest::parse` runs the envelope, the forbidden-field guard and
the `stoa` read, so those are not something a handler calls — they are what
constructing the value is. A handler holding a `PublishRequest` provably went
through all three.

Two reasons it is more than tidying, both from PLAN.md §9.2 rather than invented
here:

- **It is a precondition of `publish_moderation`.** A fourth publish operation
  written against three copies of a guard is one where a missed copy is an
  *authorisation* defect, not a wrong reply.
- **The adapter unlocks before it validates.** `open_from_env` runs a 64 MiB
  Argon2id derivation, and it runs before any field but `stoa` has been looked
  at, so a request destined for refusal pays for a full unlock first. One
  constructor holding the whole prologue makes "validate, then unlock" a change
  to one function rather than to three.

**The split into two commits is what makes either reviewable.** The refactor
commit is green at 733 + 26 with **no test changed** — that is its proof, and it
is a stronger one than any assertion, because a behaviour change with no failing
test would have to be one no test covers. The envelope fix then arrives as one
constructor line plus the tests that were red before it.

### 2. `Request::parse` is the fix, and it is one line because of decision 1

The three obligations the publish path was outside were not three fixes. All
three follow from the envelope type the crate already had:

| Obligation | Before | After |
|---|---|---|
| A non-object is refused for its shape | `[]` → `missing field: stoa` | `[]` → `the request must be a JSON object` |
| Three caller mistakes, three messages | all three were the missing-field message | three distinct messages |
| A size cap | none; a request over 4 MiB was parsed | refused before the parse |

`Request::parse` already checks the size **before** calling `from_str`, which is
the property the cap exists for — its own test,
`an_oversized_request_is_refused_before_it_is_parsed`, pins the ordering by
feeding in something both oversized and unparseable and asserting which refusal
comes back. The publish path inherits that rather than re-deriving it.

### 3. `required_stoa` is deleted rather than converted

It read `stoa` through `required_string` and hex-decoded it — the same three
answers for the same three cases that `parse_stoa` already gives, and
`parse_stoa` already takes a `&Request`. Its doc argues for being the only
reader of that field: *"a new Stoa-taking handler cannot reach `stoa` without a
`Request` in hand, because this is the only place that reads the field."*

Keeping a second reader would have been keeping exactly what let the publish
path drift outside the envelope: two functions reading one field, which
eventually disagree about whether a missing field and a wrong-typed one are one
mistake.

### 4. `publishing_key` lives in `core`, and that is what makes it testable

The spec's scenario is *"the published op's author is the identity the probe
reported"*. The probe reports `posting_identity`, which is
`stoa_address_at_path(stoa, recorded_path)`. The publish path signed with
`keystore.stoa_key(&stoa)` — the pathless scheme, under a different salt, which
`identity.rs` asserts **must** disagree.

**No test could see it where it lived.** The call was in
`dialectica/rust-lib/src/lib.rs`, which `cargo test` does not compile, clippy
does not lint and fmt does not check. This is the identical shape the probe's
own half had, and the identical fix: move the choice into `core`, where
`the_key_a_publish_signs_with_is_the_identity_the_probe_reports` can assert an
op's author against the probe's reply.

**A missing choice is a refusal, not a fallback.** `posting_identity` reports
`CannotPost` when no path is recorded. Signing with anything at all here would
produce a worse disagreement than the one being fixed — the probe saying the
user cannot post while the publish succeeds under a key the probe refuses to
name, and silent on the publishing side. So the same state gives the same
constant.

### 5. The CI exemption is deleted, and its removal was checked rather than assumed

The exemption read: *"It is exempted rather than fixed because WHICH key a
publish signs with is a spec question this gate cannot answer ... Delete this
exemption when the spec decides."* `content-authoring`'s "The signing identity
is the one the probe reports" is the spec deciding.

**It was load-bearing, not vestigial**, and that was measured rather than
inferred: running the gate's own Python, minus the exemption, against
`origin/main`'s adapter fails naming `stoa_key`. So removing it is safe only
because the call it fenced is gone, and `core::wire::publishing_key` is added to
the calls the gate *requires* the adapter to route through — which is what keeps
the derivation from coming back to a file no test compiles.

### 6. The sweep list is derived from the dispatch trait, so it cannot go stale

This is the decision worth the most scrutiny, because the change that built the
sweep recorded a mechanised version as **structurally impossible**, and this
change does it anyway. The earlier reasoning is not wrong — it is about a
different thing, and saying which is the point.

`wire-request-envelope`'s `design.md` rejected two candidates:

- **An `include_str!` test looking for `serde_json::from_str`.** Rejected on
  false positives: *"A doc comment mentioning `from_str` trips it ... A test
  helper legitimately parsing a fixture trips it ... A reply decoder trips it."*
  All true. **This is not that test.** It reads *parameter lists*, not call
  sites. A doc comment does not contain a parameter list, and a test helper is
  not declared in the dispatch trait.
- **A trait-driven sweep over the real dispatch surface**, called *"the right
  way to do this, and it cannot be done here"* for two reasons: the dependency
  points the wrong way (`DialecticaModule` lives in `dialectica`, which depends
  on `dialectica-core`), and the trait is behind `cfg(logos_scaffold)`, which no
  `cargo test` sets.

**Both objections are about *compiling* the trait.** Reading its declaration as
text needs neither: `include_str!` does not compile the file, does not link it,
and no `cfg` gates a file read. That is the part the earlier analysis did not
consider rather than a part it got wrong — and it is why the conclusion changed
without any of its premises being false.

`DialecticaModule` is the authority on the surface: `interface: "universal"`
derives the RPC table from it, so a method not declared there is not on the
wire. Within it the shape is uniform and total — `fn <name>(&mut self, request:
String) -> String;` takes a request, `fn <name>(&mut self) -> String;` takes
none, `on_context_ready` takes a context — so the signature is the
discriminator, with no heuristic involved.

**The path survives the Nix build.** `mkLogosModule.nix` stages
`codegen.rust.crate` — `rust-lib/` — with `cp -r`, and `dialectica-core` is
nested *inside* it, so `rust-lib/src/lib.rs` and
`rust-lib/dialectica-core/src/wire.rs` are staged together with the relative path
between them unchanged. That nesting is already load-bearing for the build (see
`dialectica-core/Cargo.toml`'s placement note); this inherits it rather than
adding a requirement.

**Two exceptions, both named rather than filtered silently.** `panic_probe` is
the envelope rule's third case — a method that takes a request and reads no
field of it — and the spec says the two rules cannot both reach it. `version` is
the second case and is outside by signature, so it never reaches the list at
all. `parse_channel_id` is in the sweep and is not a trait method: it is the
request-reading half of `delivery_channel_exists`, and the whole of that method a
test binary can reach, so the mapping is stated rather than left to look like a
mismatch.

**Proved by failing.** Removing `publish_vote` from the list makes it report
`["publish_vote"]` and nothing else. It is the test that would have caught this
change's own defect.

## Rejected alternatives

### Adding three entries to the sweep and stopping there

This was the smaller diff and the brief explicitly warned against it: *"That
leaves the next method to be forgotten the same way."* The list's own doc said
**ADD YOUR METHOD HERE ... Nothing checks it**, and the measured cost of nothing
checking it is this change — three methods entered the crate's wire surface,
did not enter the list, and five sweeps went green over eleven methods while the
surface had fourteen.

### A `MODIFIED` requirement scoping the envelope to the publish handlers

Considered seriously, because a contract that does not reach the code has a gap
worth closing. It does reach: `module-wire-contract` scopes the rule to *"every
method that reads a field of its request"* and names the panic probe as the
surface's only exception.

Adding a requirement naming these handlers would make the contract **weaker**.
The requirement is explicit about why it is stated once: *"This SHALL hold for
every such method, whatever fields that method requires ... The rule is
therefore stated once, for the envelope, rather than left to each method's
fields to imply."* A rule restated per method is a rule the next method is
outside — which is the exact failure mode this change is fixing at the test
layer.

### Signing with the pathless key and changing the probe to match

The other way to make the two agree, and wrong in the direction that matters.
The path-taking scheme is what `identity-onboarding` records, what `whoAmI`
reports, and what a user chose at onboarding; the pathless one is what nothing
records. Changing the probe would make two methods agree about an identity the
user never selected and that no recorded value reproduces.

### A per-method size cap on the publish path

`MAX_REQUEST_BYTES`'s own doc rules this out and the reasoning holds here: *"A
per-method cap would be a second thing each new handler has to declare, which is
the guard-at-every-call-site shape this file exists to avoid; the tighter bounds
that actually matter are per field, and they live with the field."* The publish
path's per-field bound is `authoring::MAX_BODY_LEN`, which is `op-format`'s own
field cap and already refuses an over-cap body before anything is signed.

## What this change did NOT find, recorded so the next agent does not re-check

**The null readings were already right.** The brief asked for a test on a `null`
in a required field and in an optional one. Added, for `body`, `parent` and
`direction` — and all three **passed on the unfixed code**, because
`required_string` already distinguishes a present-but-wrong-typed field from an
absent one. They are in `one_field_has_one_null_reading` as coverage of a
property, not as a regression test, and the comment there says so. What was
wrong was the envelope around those readers, not the readers.

**An over-cap *body* does not brick the peer, and did not.** A prior review's
"an over-cap body bricks every feed read on the peer" was checked against
today's code and is closed: `authoring::body_within_cap` refuses a body over
`MAX_BODY_LEN` — which is `op::MAX_FIELD_LEN`, pinned as one value by
`the_publish_body_cap_is_the_format_field_cap` — *before* the op is signed or
appended, so no undecodable op reaches the store. The live over-size defect was
the one a level up: the **request** had no cap at all, so the allocation rather
than the store was what was unbounded. Both are cited together because they read
alike and are not the same defect.
