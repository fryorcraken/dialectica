# spec-test review — `expose-name`

Read: the delta (`specs/generated-names/spec.md`, two ADDED requirements), the
live capability (`openspec/specs/generated-names/spec.md`, 15 requirements), and
the tests in `git diff origin/main...piece/expose-name` — three dots. The
implementation handler body was not read except for the lines mutated below.

Baseline, restored tree: **940 + 30 tests, all green.**

## Findings

- [x] **`tester`** — `wire.rs:2845` — the `words` field is unpinned against the
      one thing its own contract says it must not be. The trait doc at
      `dialectica/rust-lib/src/lib.rs` says `words` "is not derivable from `name`
      by splitting on spaces — a place may be a two-word toponym." **No test can
      tell the two apart.**
      **Scenario:** replace the handler's `"words": name.words()` with
      `name.render().split(' ').filter(|w| *w != "of").collect::<Vec<_>>()`.
      **Measured:** the mutation **SURVIVES** — 940 + 30 tests pass, zero
      failures. Probed why: `wordlists/places.txt` holds 10 two-word entries out
      of 1024, and **none of the 39 seeded keys (`1u8..40`) nor `PINNED_KEY_HEX`
      draws one** — measured by probe, `seeds_with_two_word_place=0
      pinned_two_word_place=false`. Every fixture with a hardcoded `words`
      expectation is a case where splitting gives the identical answer: the
      repo's own defect family, a fixture where two explanations agree.
      **Fix:** add a key whose place is a two-word toponym (search the seed space
      for one, as `a_short_key_is_refused_rather_than_padded_to_one_that_names`
      already searches) and assert `words[2]` contains a space while
      `words.len() == 3`. Severity: high — this is the field the view renders
      when it cannot fit the connector, and the bug ships a four-element array.

      **Fixed** in `ff041e8`, by the route you prescribe — searched the seed
      space rather than assuming a fixture.

      **Seed 166** is the one: `feed_key(166)` names
      `intaxable eidos of thermai himeraiai`, so `words` is
      `["intaxable", "eidos", "thermai himeraiai"]` — three elements, the last
      containing a space — while the render is five space-separated tokens.
      `words_is_the_three_drawn_words_and_not_the_rendered_name_split_on_spaces`
      asserts `words.len() == 3`, `words[2] == "thermai himeraiai"`, and — the
      part that keeps the fixture honest — that the render still has five
      tokens, so the test says so rather than going quietly vacuous if the
      fixture ever stops being a two-word case.

      Re-ran your mutation verbatim. With
      `name.render().split(' ').filter(|w| *w != "of")` in place, the new test
      fails with
      `words: ["intaxable","eidos","thermai","himeraiai"], left: 4, right: 3`
      and 944 of the other 945 pass — the four-element array you predicted,
      now visible. Reverted.

      Your probe result (`seeds_with_two_word_place=0` over `1u8..40` plus the
      pinned key) explains why every existing hardcoded `words` expectation
      agreed with a split: 10 entries in 1,024, and no fixture drew one.

- [x] **`tester`** — `wire.rs:12760`,
      `a_public_key_alone_is_enough_with_no_address_supplied` second half — the
      address assertion cannot fail, by either branch.
      **Scenario:** the test feeds `key.address().to_hex()` where a key goes and
      asserts only `assert_ne!(from_address["name"], Some(PINNED_NAME))`.
      **Measured:** probed the fixture — the pinned key's address
      `36af7f21…f7c59614` **does not parse as a public key**
      (`parses_as_key=false`), so the reply is
      `{"error":"cannot derive a display name: not a valid public key"}`,
      `from_address["name"]` is `Null`, `.as_str()` is `None`, and
      `None != Some(_)` is a tautology. The other branch is no better: under the
      identity-layer-bypass mutation below the address **did** decode and name,
      and this test still passed, because the name it produced was merely a
      different name. The only input that could fail this assertion is a SHA-256
      collision between the name digest of the key and of its address.
      **Fix:** split it. Half one ("a key alone suffices") is sound and should
      stay. Half two should assert the *refusal* directly —
      `assert!(from_address.get("error").is_some())` for this fixture, pinned
      with the measured fact that this address is not a curve point — or be
      dropped, since `the_entry_point_admits_exactly_what_the_identity_layer_admits`
      already covers "the entry point does not name non-keys". Severity: medium
      — the test reads as covering the wrong-field-read case and covers nothing.

      **Fixed** in `ff041e8`. Half one kept as you say; half two made real
      rather than dropped.

      Your "other branch is no better" point is the one that decided the fix.
      Asserting the *refusal* would have pinned this fixture's accident — that
      `PINNED_KEY_HEX`'s address is not a curve point — rather than the
      property. So instead the fixture was changed to one where the address
      **does** parse: seed 3, whose key names `temptatious eremia of donousa`
      and whose address `88494840…c135e159` names
      `subjectable syllogismos of rhode iberias`. The assertion is now a
      comparison of two real names and can fail.

      Both replies are `.expect()`ed, so if the fixture ever stops being an
      address-that-parses the test fails loudly instead of reverting to
      `None != Some(_)`.

      Note the correctness reviewer measured 111 of 200 seeded addresses parsing
      as keys — so the vacuous case was not even the likely one, and "an address
      usually is not a curve point" is the wrong intuition that made the
      original look adequate.

      The wrong-field-read case this test *read* as covering is now covered
      directly and separately, by
      `the_key_is_read_from_the_public_key_field_and_from_no_other` — see the
      correctness reviewer's first box.

- [x] **`spec-writer`** — delta, *The entry point refuses anything that is not a
      public key*, scenario **"The entry point admits exactly what the identity
      layer admits"** — the requirement contradicts the behaviour the delta's own
      other scenarios require, and is untestable as written.
      **Scenario:** the scenario says the entry point "returns a name in exactly
      the cases the identity layer accepts the material, and refuses in exactly
      the cases it refuses". But the entry point also refuses inputs the identity
      layer never evaluates: a non-hex string (`{"publicKey":"zz"}`), a
      wrong-typed field (`{"publicKey":7}`), and a hex string over the allocation
      bound — all three pinned as refusals by
      `a_refusal_is_never_reported_as_a_name` (wire.rs:12999). For those the
      identity layer has no verdict at all, so "exactly the cases it refuses" has
      no truth value.
      **Measured:** the test at wire.rs:12908 silently narrows the claim — its
      corpus is entirely `hex::encode`d byte vectors, so the decode always
      succeeds and the bound is never reached. The test is honest about its own
      corpus; the scenario is not honest about its own scope.
      **Fix:** restate as "for key material that reaches the identity layer — a
      hex string within the bound — the entry point's verdict is the identity
      layer's", and enumerate the request-shape refusals (absent, wrong-typed,
      non-hex, over-bound) as their own scenario. Severity: medium.

      **Fixed** in `ff041e8` by the `dev-writer`, since the `spec-writer` is not
      returning to this piece and an unanswered box blocks the merge. Both parts
      as prescribed.

      The scenario is now scoped — *"WHEN key material **that reaches the
      identity layer** is supplied…"* — so "exactly the cases it refuses" is
      about material the identity layer actually rendered a verdict on, and has
      a truth value.

      The request-shape refusals are a scenario of their own: *"A request the
      entry point cannot read as key material is refused before the identity
      layer is reached"*, covering absent, wrong-typed, non-hex and over-bound,
      and stating that the refusal is the entry point's own **because the
      identity layer is given nothing to judge**. That last clause is the part
      your finding made me write — it is what makes the two scenarios
      non-overlapping rather than merely differently worded.

      Your observation that the test at `wire.rs:12908` "is honest about its own
      corpus; the scenario is not honest about its own scope" is exactly right,
      and the test needed no change — it was measuring the narrower claim all
      along. The spec now says the claim the test was already checking.

- [x] **`spec-writer`** — delta, both ADDED requirements — **the reply shape is
      entirely unspecified, and the tests pin it anyway.** No requirement or
      scenario in the delta or the live capability mentions a `words` field, the
      field name `publicKey`, or hex as the key's encoding. The tests hardcode
      all three: `serde_json::json!(["riskful","megaron","anemourion"])`
      (wire.rs:12722), `assert_eq!(absent, "missing field: publicKey")`
      (wire.rs:12981), and every fixture is hex.
      **Scenario:** a second implementation reading the delta alone ships
      `{"key":"<base64>"}` returning `{"name":...}` with no `words`, satisfies
      every scenario, and no caller written against this one works against it.
      These are unmarked `NO SPEC:` gaps — **the piece adds no `NO SPEC:` marker
      at all** (grepped: 35 in the crate, none in the new tests), so nothing
      flags them for capture.
      **Fix:** either specify the reply shape (at minimum: the derivation's three
      words are reachable separately from the rendered name, which is what makes
      the live capability's *the connector may be elided* relaxation usable by a
      caller), or have `tester` mark each pinned literal `NO SPEC:`. Severity:
      medium — `words` is a whole field of the deliverable API with no
      requirement behind it.

      **Fixed** in `ff041e8`, taking your second option (mark them) and adding
      the design record the convention pairs with it.

      Your grep was right — the piece added no `NO SPEC:` marker at all. There
      are now two, both in the tests that pin the literals:
      `a_name_is_obtainable_for_a_supplied_public_key` carries one naming
      `name`, `words`, `publicKey` and hex as this change's choices, and
      `absent_key_material_is_refused_distinguishably_from_bad_key_material`
      carries one for the `missing field: publicKey` literal. `design.md` gains
      **§5a**, which argues each choice and says plainly that a second
      implementation reading the delta alone could ship `{"key":"<base64>"}`
      with no `words` and satisfy every scenario — your scenario, kept because
      it is the clearest statement of what is unspecified.

      Did **not** take the first option (specify the reply shape in the delta).
      Field spellings on the module surface are a decision about the deliverable
      API, and the design reviewer's parallel box establishes the convention
      here is marker-plus-design-entry rather than spec text — `wire.rs:1797` is
      the precedent, and I read it before following it. If the owner later
      decides the reply shape belongs in the capability, that is a spec change
      with a `spec-writer` on it, and the marker is what will surface it.

      One thing your box implies that I want to state rather than leave: the
      live capability *does* require the three words be reachable separately
      from the rendered name — that is what makes "the connector may be elided"
      usable — so `words` is not a field with no requirement behind it. What is
      unspecified is its **name**, not its existence. §5a says so.

- [x] **`spec-writer`** — delta, *"Failing to supply key material at all is a
      distinct refusal from supplying bad key material"* — the requirement
      enumerates **two** refusal classes; the implementation has **four**, and
      the tests pin a third one the spec never names.
      **Scenario:** `{"publicKey":7}` is refused with `"publicKey must be a
      string"` — neither "absent" nor "not a well-formed public key". It is
      swept by `a_refusal_is_never_reported_as_a_name` but nothing requires it,
      nothing requires it be distinguishable from the other two, and a build
      collapsing it into `"missing field: publicKey"` passes every test.
      **Fix:** say whether a wrong-typed field is its own refusal, and if so that
      it too must be distinguishable. Severity: low-medium.

      **Fixed** in `ff041e8` by the `dev-writer`, for the reason given on the
      box above. The answer is **yes, it is its own refusal and it must be
      distinguishable** — and the spec now says so for all three of the
      request-shaped classes, not only the wrong-typed one.

      The delta's second requirement gains a paragraph: key material of a
      non-string type, key material that is not valid hex, and key material
      longer than a public key's encoding admits are each refused by the entry
      point itself, are a third class rather than a variety of absent-or-bad,
      and SHALL each be distinguishable from absent — *"a caller that reads
      'nothing supplied' when it supplied something malformed looks for the
      wrong bug"*. It also states that the entry point MAY refuse over-long
      material without examining it, which is what licenses bounding the length
      before allocating from it.

      Your "a build collapsing it into `missing field: publicKey` passes every
      test" is now false, and measured so.
      `every_request_shaped_refusal_is_distinguishable_from_the_others` pins all
      five messages to literals and asserts every pair distinct; applying
      exactly that collapse makes it fail with
      `left: "missing field: publicKey", right: "publicKey must be a string"`.
      Reverted after measuring.

      That the implementation had four refusal classes to the spec's two was the
      thread worth pulling — it is five, and `design.md` §4 said "three" over a
      list of four. See the design reviewer's third box; both are fixed together.

- [x] **`spec-writer`** — `docs/PLAN.md:2763` on `origin/main` — a stale premise
      the delta now contradicts, in the §9.1 feed description:
      `- the author, as the per-Stoa address (§5.2) — never a name, because there are` / `  no names`.
      **Scenario:** "never a name" is still right; **"because there are no
      names" is not** — names are built, and the delta makes one obtainable by a
      caller holding a key. The adjacent thread-read material has already been
      corrected in the opposite direction (it now says the author is reported as
      address **and** public key "because the generated name derives from the
      key"), so this bullet is the one place left asserting the withdrawn
      premise, and §9.1 flags the feed read as built-with-no-contract, so nothing
      else corrects it.
      **Fix:** keep "never a name", replace the reason with a pointer to the
      `generated-names` capability's *The name SHALL NOT travel*. Severity: low,
      but it is the exact shape CLAUDE.md's self-invalidating rule exists to
      catch. Verified against `origin/main`, not the branch copy.

      **Fixed** in `ff041e8`, exactly as prescribed. The bullet keeps "never a
      name" and the reason is now the prohibition rather than the absence:
      *"because the `generated-names` capability's The name SHALL NOT travel
      forbids a reply carrying one"*.

      Added one sentence you did not ask for, and it is the CLAUDE.md point you
      make: the bullet now records that the old reason was *"true when it was
      written and is not now"*. A reader who remembers "there are no names"
      needs to see it withdrawn, not just replaced — otherwise the withdrawn
      premise survives in the reader rather than in the file.

      Confirmed the line on the branch matches what you read on `origin/main`
      (2763-2764, unmodified by this piece before now), so the edit is to the
      same text you reviewed and not to a copy that had already drifted.

## Mutations run, and what they measured

Five mutations, each applied to the worktree, run, and reverted; `git status`
clean and the suite back to 940 + 30 green afterwards, confirmed.

| Mutation | Result |
|---|---|
| `names.rs:259` — bypass `PublicKey::from_bytes`, name any 32 bytes | **5 failed**, incl. `a_low_order_point_is_refused_rather_than_named` (named `promptive moschos of gitana`), `the_entry_point_admits_exactly_what_the_identity_layer_admits`, `a_refusal_is_never_reported_as_a_name`, `absent_key_material_…`, and pre-existing `names::…::malformed_key_material_…` |
| `identity.rs:529` — collapse `WeakPublicKey`'s message into `"not a valid public key"` | **2 failed**, 1 of them this piece's `a_low_order_point_is_refused_rather_than_named` (the other pre-existing) |
| `wire.rs:2830` — `>` → `!=`, making the allocation bound decide validity | **1 failed**: `the_hex_bound_refuses_an_oversized_key_without_deciding_validity` |
| `wire.rs:2845` — `words` by splitting the rendered name | **SURVIVED — 0 failed.** See finding 1. |
| probe (not a mutation) — does `PINNED_KEY_HEX`'s address parse as a key? | `parses_as_key=false`. See finding 2. |

**The author's mutation claim is substantially verified.** The identity-layer
bypass does fail the low-order test — the security-critical case — and the
collapsed-message and allocation-bound mutations each fail exactly the one test
that names them. My bypass was narrower than the author's (it kept wrong-length
material refused), so 5 rather than 10 is expected and not a discrepancy.

## What is clean

**Every delta scenario has a test**, mapped one by one: the six under *Core
exposes the derivation* map to `a_name_is_obtainable_for_a_supplied_public_key`,
`the_name_returned_is_this_derivation_and_not_a_second_one`,
`a_public_key_alone_is_enough_with_no_address_supplied` (first half),
`the_same_key_reaches_the_same_name_on_every_call`,
`a_name_is_obtainable_for_a_key_belonging_to_no_known_identity` and
`obtaining_a_name_changes_nothing_that_a_later_call_answers`; the seven under
*The entry point refuses…* map to `key_material_of_the_wrong_length_is_refused`,
`a_low_order_point_is_refused_rather_than_named`,
`the_entry_point_admits_exactly_what_the_identity_layer_admits`,
`absent_key_material_is_refused_distinguishably_from_bad_key_material`,
`a_refusal_is_never_reported_as_a_name` (plus
`a_short_key_is_refused_rather_than_padded_to_one_that_names` for the no-padding
half), `arbitrary_key_material_does_not_take_the_module_down` and
`a_well_formed_key_reaches_no_failure`.

**The pinning is done right**, and this is the part most likely to have gone
wrong. `PINNED_NAME_ON_THE_WIRE` is a hardcoded literal produced by
`examples/pin_name.rs`, which does not link the derivation — not
`assert_eq!(reply, display_name(key))`, which would move with the code. The
low-order refusal is asserted against the literal `"low-order public key"` rather
than `KeyError::WeakPublicKey.to_string()`, so a reword is visible rather than
made on both sides at once. `a_well_formed_key_reaches_no_failure` guards against
a constant-returning method (`names.len() > 50` over 64 random keys), and
`the_entry_point_admits_exactly…` guards against a refuse-everything method
(`accepted >= 5`). `a_short_key_is_refused_rather_than_padded_to_one_that_names`
**searches** for a prefix whose zero-padding actually parses rather than assuming
one, which is the fix for exactly the family this repo keeps shipping. These are
the right instincts throughout; the `words` gap is the one place the same care
was not applied.

**No requirement moved between capabilities** — the delta is two `## ADDED
Requirements` with no `REMOVED` half, so item 4 of the review does not apply. The
live capability's *The name SHALL NOT travel* still carries its "How a caller
reaches the derivation is not settled here … Tracked as issue #81, and out of
scope here" paragraph; that becomes stale when this change archives, but that is
`closer`'s archive step rather than a defect here.

**No test asserts something no requirement requires**, with the reply-shape
exception already filed above.
