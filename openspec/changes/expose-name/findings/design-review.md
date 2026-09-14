# Design review — expose-name

Reviewed `design.md` against the code on `piece/expose-name` at `9be0329`, and
against `docs/PLAN.md` read from `origin/main`.

**The decisions asked about were each taken as recorded.** One key per call is
what `wire.rs:2812` implements; the refusal genuinely defers to
`names::display_name_from_bytes` and this file enumerates no key shape; the
request field is `publicKey` and hex, matching what `posting_identity` and
`who_am_i` emit. No code contradicts a recorded decision.

**The batch decision is revisitable from the record** — decision 1 names the
constraint (the forbidden reply shapes), both alternatives, what ruled each out,
and the cost it accepts, and the reversibility claim holds: the spec's
requirements are stated per key and a batch method added alongside would leave
every one of them true. It is the strongest entry in the file.

**No contradiction with `docs/PLAN.md`.** §9's "methods deliberately NOT
proposed" declines `getPost` on §2.4's per-item ground, and this change adds a
per-item method — but decision 1 argues the departure at length rather than
taking it in passing, which is what PLAN.md's status as intent requires. §5.2.1
is already pruned to a one-line "Built — see the `generated-names` spec" and
carries no reachability reasoning, so proposal.md's claim that nothing needs
striking through is correct and **no reasoning was left behind in PLAN.md**.

**The re-grounding away from `docs/UI-BRIEF.md` is sound**, checked claim by
claim against `openspec/specs/generated-names/spec.md`: the connector being
droppable by a cramped caller and being the only undrawn part is lines 254-259
verbatim; the two-word toponym is the scenario at line 535; "not a credential"
and "anyone willing to press a regeneration button reaches any name they like"
are lines 629-640. Nothing was repointed at a spec that does not say it.

**The `cfg(logos_scaffold)` gate blindness is already recorded durably** —
`.claude/agents/README.md:400-404` states it as a standing rule, and design.md §6
adds the concrete instance and names `nix build .#lgx` as the only gate that
sees the adapter. That is in good shape and needs no box.

The seven boxes below are: three claims in Decisions that the code contradicts,
one refusal the record does not enumerate, two decisions taken silently, and one
entry whose closing sentence overreaches.

---

- [x] **`dev-writer`** — `design.md:120-123` and `wire.rs:2754` claim a
      length the bound does not let through
      `design.md` §3: *"A 30-byte or 34-byte hex string passes this bound and is
      refused by the identity layer"*. `wire.rs:2754`: *"including the 62- and
      66-character strings that clear this and are refused there"*.
      **Measured from the source:** `MAX_PUBLIC_KEY_HEX_CHARS = 32 * 2` = 64, and
      the guard is `hex_str.len() > MAX_PUBLIC_KEY_HEX_CHARS` (`wire.rs:2830`).
      A 34-byte key is 68 hex characters and a 33-byte key is 66; both are
      **greater than 64**, so both are refused by the bound with
      `publicKey is N hex characters, over the 64 a public key holds` and neither
      ever reaches `hex::decode` or `PublicKey::from_bytes`.
      Only the *short* half of the claim is true (30 bytes = 60 chars does clear
      it). The over half is exactly backwards, and it is the half the entry is
      defending: the sentence exists to say the bound decides no validity, and on
      the over side the bound **is** the sole decider for every wrong-length key.
      The honest entry is narrower and still defensible — the bound decides
      nothing for any string at or under 64 characters, which is where every
      valid key and every short refusal lives — but it must be written that way,
      because the current wording tells the next reader something they can act on
      and be wrong.

      **Fixed** in `ff041e8`, and your sentence is very nearly the one that
      landed: both `design.md` §3 and the constant's doc now say the bound
      decides nothing at or under 64 — where every valid key and every short
      refusal lives — and is the **sole** decider above it.

      Measured every length rather than reasoning from the comparison: 58, 60,
      62 and 64 characters reach `cannot derive a display name: not a valid
      public key`; 66 and 68 reach `publicKey is N bytes, over the 64 a public
      key's hex holds`. So the short half was true and the over half exactly
      backwards, as you say.

      Both places also record *which* half was wrong and that it was the half
      the entry existed to defend. The examples are now three under-bound
      lengths (29/30/31 bytes) instead of one under and one over.

      Two other reviewers found the same defect in two more copies of this
      argument — the test comment at `wire.rs:13169` said "62 and 66" over a
      loop running `[31usize, 32]`. All three instances are fixed together, and
      the argument is now pinned by a test rather than repeated in prose: see
      the next box.

- [x] **`dev-writer`** — `design.md:230-235` claims a test pins that
      separation, and the test cannot see it
      The Risks entry: *"a wrong-length key that clears it is still refused by
      `PublicKey::from_bytes`. Pinned by a test asserting the entry point's
      verdict matches the identity layer's over a corpus that includes
      wrong-length material on both sides."*
      **Scenario:** `the_entry_point_admits_exactly_what_the_identity_layer_admits`
      (`wire.rs:12908`) carries `vec![0xab; 33]` and `vec![0xab; 64]` — 66 and 128
      hex characters — and asserts only `entry_point_named == identity_layer_accepts`,
      i.e. `false == false`. Both are refused by the hex bound, and the test
      passes identically whether the identity layer was ever reached. The same
      goes for `key_material_of_the_wrong_length_is_refused`, which asserts
      refusal without asserting which layer refused, and for
      `the_hex_bound_refuses_an_oversized_key_without_deciding_validity`, whose
      "must not be refused for size" half tests only 31 and 32 bytes (62 and 64
      characters) and never a single length above the bound.
      This is the gate whose input the defect satisfies: the claim in Decisions
      is pinned by nothing, and the test reads as though it were. Either narrow
      the claim per the box above, or say plainly that the over-length side is
      decided by the bound and unpinned.

      **Fixed** in `ff041e8` — took the first option, and then made the narrowed
      claim actually pinned rather than merely honest.

      Your `false == false` observation is the sharpest thing in this review and
      I verified it: `vec![0xab; 33]` and `vec![0xab; 64]` are 66 and 128
      characters, both refused by the bound, and the assertion
      `entry_point_named == identity_layer_accepts` holds identically whether
      `PublicKey::from_bytes` is ever called. The test passes for a reason
      unrelated to what the Risks entry claimed it showed.

      The Risks entry is now narrowed (decides nothing at or under 64; sole
      decider above) **and** points at a new test that can see the division:
      `the_bound_decides_every_over_length_refusal_and_the_identity_layer_never_sees_one`
      walks 29/30/31/32 bytes asserting the identity layer's message comes back,
      then 33/34/64 asserting the bound's message comes back **and** that the
      identity layer's does not. That asserts *which layer refused*, which is
      the thing an `is_err()`-shaped assertion structurally cannot show.

      It is not a decorative addition: it fails independently under the
      constant-loosening mutation from the architecture review, because at
      `4096 * 2` the 66-character case stops being the bound's and becomes the
      identity layer's.

      The entry also now names itself as having been an instance of "a gate
      whose input the defect satisfies", since that framing is what makes the
      correction worth recording rather than just applying.

- [x] **`dev-writer`** — `design.md:125` — the over-length refusal is a fifth
      message that no record enumerates
      §4 is headed *"Three distinguishable refusals"* and lists **four** bullets;
      `tasks.md:30` says *"Four distinguishable refusals"*; the code emits
      **five**: `missing field: publicKey`, `publicKey must be a string`,
      `publicKey is N hex characters, over the 64 a public key holds`,
      `publicKey is not valid hex`, and `cannot derive a display name: …`.
      The size message is the one a caller can receive that appears in no list —
      it lives only in §3, framed as an allocation bound rather than as a refusal
      a caller sees. It is also the one refusal that is neither "no key material"
      nor "bad key material", which are the two the spec requires be told apart,
      so a caller reading §4 to learn what it must handle is handed an incomplete
      set. Fix the count in the heading, add the fifth bullet, and say which of
      the spec's two categories it falls in — or that it falls in neither.

      **Fixed** in `ff041e8`, all three parts, including the third — the answer
      is **neither**.

      §4 is now headed "Five distinguishable refusals" over five bullets, with
      the size refusal in its proper place between wrong-typed and bad-hex (it
      runs there, before the decode). `tasks.md:30` said "Four" and now says
      five. Both record that the size message was the one appearing in no list,
      and why: §3 framed it as an allocation bound rather than as something a
      caller receives, and it is both.

      On the categories: three of the five are **request**-shaped (absent,
      wrong-typed, over-length), one is encoding-shaped (not hex), and exactly
      one is the spec's "bad key material". The over-length refusal is key
      material that was supplied and never examined as a key — the identity
      layer renders no verdict on it — so it is neither of the two the spec
      names. The spec's requirement is satisfied because its two named
      categories are distinguishable from each other; §4 now says that rather
      than implying the five map onto the two.

      The spec-test reviewer's parallel box carried this further: the delta
      itself enumerated only two classes while the code had more, so the delta
      now requires the three request-shaped refusals be distinguishable from
      absent, and
      `every_request_shaped_refusal_is_distinguishable_from_the_others` pins all
      five messages and asserts every pair distinct. So the fifth message is
      enumerated in the design, the tasks, the spec and a test.

- [x] **`dev-writer`** — `design.md:167-182` and
      `dialectica-core/src/lib.rs:55-56` — the precedent cited for
      `core::wire::` is half false
      §6: *"`core::wire::` is already the established form for handlers outside
      the list (`get_capabilities_from_stores`, `publishing_key`)"*, repeated
      almost verbatim in the code comment.
      **Verified:** `get_capabilities_from_stores` is **in** the root re-export
      list — `dialectica-core/src/lib.rs:61`, two lines below the comment that
      names it as outside. The adapter does spell it `core::wire::` at
      `rust-lib/src/lib.rs:764`, but that is a caller's habit, not the list
      membership the sentence claims. `publishing_key` is the only genuine
      example, and it is genuine.
      The decision itself survives — one real precedent plus the confusion
      argument is enough — but a Decisions entry resting on two examples where
      one is refuted by the file it sits in is the shape that gets re-litigated
      the moment somebody checks. Drop the wrong example from both places, or
      restate it as "the form the adapter already uses for
      `get_capabilities_from_stores` and `publishing_key`", which is true.

      **Fixed** in `ff041e8`, taking your restatement in both places — and
      adding the inference, because the restatement alone leaves the reader to
      work out what the precedent now proves.

      Verified independently of your report: `get_capabilities_from_stores` is
      in the list at `dialectica-core/src/lib.rs:61`, the adapter spells it
      `core::wire::` at `rust-lib/src/lib.rs:764`, and `publishing_key` appears
      in `dialectica-core/src/lib.rs` only inside the comment — so it really is
      the one genuine example.

      What both places now say: the adapter uses the long form for handlers
      **both in and out of** the list, so spelling and membership are
      **independent**, and the precedent is therefore not evidence that the
      omission is required. The omission rests on the ambiguity argument alone,
      and saying "alone" is what stops the next reader looking for a second
      reason and finding the counter-example instead.

      The wrong wording is named in both places rather than silently replaced,
      since a reader who half-remembers "handlers outside the list" should see
      it withdrawn.

      The readability reviewer filed the same defect from the other direction
      (~30 lines across three files); both boxes are answered by this change.
      Noted there too: the adapter's own comment at `rust-lib/src/lib.rs:911-918`
      never made the false claim, so it needed no edit — checked, not assumed.

- [x] **`dev-writer`** — `tasks.md:43` — there are **four** hand-maintained
      sweep lists, and the fourth was not updated
      `tasks.md` is headed *"The three hand-maintained sweep lists"* and works
      through `every_request_taking_method()`,
      `every_method_with_a_required_field()` and `a_served_request()`.
      **Measured:** `one_field_has_one_null_reading` (`wire.rs:10395`) builds its
      own hand-listed `cases` vec at `wire.rs:10406` — eleven entries, one per
      field the surface reads — and `publicKey` is not among them. Unlike
      `every_request_taking_method()`, that list has **no trip-wire**: nothing
      relates it to the dispatch trait, so its omission is silent and the suite
      is green.
      The behaviour is right — `Request::get` returns `Some(Value::Null)`
      (`request.rs:229`), so `{"publicKey":null}` takes the `Some(_)` arm and
      reads as `publicKey must be a string`, the wrong-type reading. What is
      missing is membership in the sweep that asserts a null is never reported as
      *missing* and lands in exactly one bucket; `a_refusal_is_never_reported_as_a_name`
      includes the null request but asserts only that it errors and carries no
      name. This is the repo's own "hand-maintained sweep lists go stale
      silently" trap, applied at three sites and missed at the fourth. Record the
      fourth list in `tasks.md`, add the `publicKey` case, and — since this list
      is the one without a trip-wire — say in `design.md` why it has none, or
      that it should.

      **Fixed** in `ff041e8`, all three parts as prescribed.

      `one_field_has_one_null_reading`'s `cases` vec gains the `publicKey` entry.
      `tasks.md`'s heading is now "The four hand-maintained sweep lists", with
      the fourth listed and flagged as the one without a trip-wire;
      `proposal.md` is corrected in the same direction. `design.md` gains **§8**,
      which answers the "why none" question.

      The answer, since you left it open: the other three enumerate **methods**,
      and the dispatch trait also enumerates methods, so the two can be compared
      and a trip-wire falls out. This one enumerates **fields**, and nothing in
      the source enumerates those — each is a string literal inside one
      handler's body. A derived version would have to parse handler bodies for
      `parsed.get("…")`, which is a gate the next handler's formatting can
      corrupt into reporting clean — the failure mode this repo records as worse
      than no gate, because it closes the question. So it stays hand-maintained,
      and §8 says plainly that this is a weaker guarantee than the other three
      have.

      Your point that the *behaviour* was already right (`Request::get` returns
      `Some(Value::Null)`, so `{"publicKey":null}` reads as the wrong-type case)
      is why this is coverage of a property rather than a bug fix — which is
      exactly what the existing comment on that vec says about the publish
      path's three fields, so the new entry is consistent with its neighbours.

      While there: `proposal.md` also said three lists "must gain the new
      method", which the correctness reviewer noted overstates it —
      `every_method_with_a_required_field()` filters the first list and needs no
      edit. Now recorded as three gaining an entry and one inheriting.

- [x] **`dev-writer`** — no `NO SPEC:` marker was written, and at least one
      choice has a direct precedent for one
      The reported reasoning — that the two open questions were strategy
      decisions belonging in `design.md` — is right about batching and about hex,
      both of which are properly recorded as decisions 1 and 2. It does not reach
      the **reply** field spellings. The spec fixes no field names anywhere; the
      handler chose `name` and `words` (`wire.rs:2843-2846`), a view will branch
      on both, and changing either later is a breaking change to the module
      surface.
      **The precedent is in the same file:** `wire.rs:1797` — *"NO SPEC: the
      field NAME is this change's choice — the spec requires the value and names
      no field. `design.md` carries it."* — and `wire.rs:1758` for a set of three
      value spellings. Both pair a marker with a design entry, so the convention
      treats them as complementary rather than as alternatives; "it belongs in
      `design.md`" is not a reason to omit the marker.
      Two things to fix, and they are separate: mark the reply spellings in the
      test, and **record them in `design.md`**, which currently does not. §2 argues
      the *request* field's name and encoding; §5 argues why both values are
      emitted, not why they are called `name` and `words`. A reader looking for
      the reply contract's decision finds only the example JSON.

      **Fixed** in `ff041e8`, both things, and kept separate as you insist.

      Your correction of my reasoning is accepted: "it belongs in `design.md`"
      is not a reason to omit the marker. I read `wire.rs:1797` and `wire.rs:1758`
      before writing these, and both do pair a marker with a design entry — the
      convention treats them as complementary, so the argument I made was for a
      choice the file had already rejected.

      **Marked:** `a_name_is_obtainable_for_a_supplied_public_key` carries a
      `NO SPEC:` naming `name`, `words`, `publicKey` and hex as this change's
      choices, with the observation that a second implementation could ship
      `{"key":"<base64>"}` with no `words` and satisfy every scenario.
      `absent_key_material_is_refused_distinguishably_from_bad_key_material`
      carries one for the `missing field: publicKey` literal.

      **Recorded:** `design.md` gains **§5a**, which is the section you say a
      reader currently cannot find — it states that the field names are
      unspecified, argues each of the three, and says changing any is a breaking
      change to the module surface. §2 keeps the request half; §5 keeps the
      why-both-values half; §5a is the reply *contract's* decision, which was
      the gap.

      One distinction §5a draws that your box does not, because it matters for
      what is actually unspecified: the live capability **does** require the
      three words be reachable separately from the rendered name (that is what
      makes its connector relaxation usable). So what is unspecified is the
      field's **name**, not its existence.

      The spec-test reviewer filed the same gap from the spec side and confirmed
      the grep — 35 markers in the crate, none in this piece. Both boxes are
      answered by this change.

- [x] **`dev-writer`** — `design.md:83-88` — the sketched future batch shape
      would not satisfy the requirement the same entry cites
      Decision 1 closes with the honest form of a future batch being
      `{"names":[...]}` with each entry a name **or null**, *"with the refusal
      reason omitted entirely"*. That shape carries no message per key, so it
      cannot distinguish absent key material from bad key material — the very
      requirement §4 is built around and which the spec states as
      *"the two refusals carry different messages"*.
      This does not break the reversibility claim, because the batch is framed as
      **addable alongside** the one-key method (Risks:207-212 says so), which
      leaves the single call as the route that distinguishes. But the entry says
      the batch *"satisfies every requirement written in the spec"*, and as a
      replacement it would not. One sentence saying the batch is additive and why
      — not a substitute — closes the question before the next person reopens it.

      **Fixed** in `ff041e8`. The sentence is written, and the loose phrasing
      that made it necessary is gone.

      Decision 1's reversibility claim now reads "a batch method **added
      alongside this one** satisfies every requirement written in the spec",
      rather than the unqualified form, and the entry closes with a paragraph
      making your argument explicitly: a name-or-null list carries no message
      per key, so it cannot tell absent key material from bad key material — the
      distinction §4 is built around and which the spec states as *"the two
      refusals carry different messages"*. A batch that **replaced** this method
      would breach a requirement the spec makes; one beside it leaves the single
      call as the route that distinguishes, which is why reversibility survives.
      It ends with the operative instruction: anyone adding a batch must keep
      this method.

      Your reading is exactly right that this does not break reversibility — it
      breaks a *sentence*. Recording the distinction matters because
      "reversible" and "replaceable" read alike at a glance, and the next person
      reaching for a batch will be reading quickly.

      The architecture reviewer's first box lands in the same decision (the
      "no third shape" claim that this very passage refuted). Both are fixed
      together, so decision 1 now names the honest ground for one-key-per-call
      and the honest shape of any future batch.
</content>
</invoke>
