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

- [ ] **`dev-writer`** — `design.md:120-123` and `wire.rs:2754` claim a
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

- [ ] **`dev-writer`** — `design.md:230-235` claims a test pins that
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

- [ ] **`dev-writer`** — `design.md:125` — the over-length refusal is a fifth
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

- [ ] **`dev-writer`** — `design.md:167-182` and
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

- [ ] **`dev-writer`** — `tasks.md:43` — there are **four** hand-maintained
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

- [ ] **`dev-writer`** — no `NO SPEC:` marker was written, and at least one
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

- [ ] **`dev-writer`** — `design.md:83-88` — the sketched future batch shape
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
</content>
</invoke>
