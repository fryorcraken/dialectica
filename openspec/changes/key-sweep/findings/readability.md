# Readability findings — key-sweep

Dimension covered: **readability only**. Correctness, security and architecture
are other instances' — an unticked row for those means nobody has done them.

Reviewed at `a5f83ba`, diffed three-dot against `origin/main`.

The sweep this piece is mostly made of is, on the evidence, very good. I checked
every surviving spelling of the deleted claim across `dialectica/`,
`dialectica-ui/` and `docs/` — `re-deriv`, `rederiv`, `binds`, `bind to`,
`address check`, `claimed author`, `claimed address`, `author address`,
`the address is the identity`, `attributed to the address`, `authorAddress`,
`identityAddress`, `.address()` — and **every hit in the six Rust modules the
piece rewrote, in every QML file, and in `docs/PLAN.md` and `docs/IDENTICON.md`
is either about a Stoa address (which survives), or is a past-tense correction
carrying its own "until issue #80" attribution.** Two false claims survived the
author's and the tester's sweeps, both in `tests/end_to_end.rs`, which is the one
file neither pass reached; they are entries 1 and 2 below.

Numeric claims came back almost entirely clean, which is worth saying given this
repo's record: the `op-format` "eleven ways a byte" (11 `OpError` variants), the
"five refusals" in `transport.rs` (5 spec bullets), `design.md` §4's "six
production reply sites" (3 repointed + 3 deleted), the Identicon byte-allocation
table (4+8+4+6+3 = 25 allocated, 7 unallocated, 11 of 32 shown), and the "eight
selectors" in `tst_onboarding_states.qml` all verify against the code. One pair
of numbers is fabricated — entry 3, which originates in the spec and was copied
into the code.

Every cited test name I checked exists, and every quoted spec requirement I
checked appears verbatim in `openspec/`. One cited test name does not exist
(entry 6), and it is pre-existing.

**Mutation run and reverted:** I reinstated the deleted author-address derivation
inline in `identity.rs` to check the retired pin. `SHA256(b"/dialectica/1/Address/Author\0\0\0\0"
|| 0x01 || key)` for `SecretKey::from_bytes(&[7; 32])` reproduces
`f875158a79d255a6dd83307d5918298cd819eafb8218ef14cd34ebf2bc385ef4` exactly, so
`no_derivation_turns_a_public_key_into_an_address`'s "MEASURED, not asserted"
claim is **true**. The tree was restored and is clean.

---

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/tests/end_to_end.rs:121-123` —
      the module doc still teaches the deleted derivation as a live technique.
      It reads: *"Where an expected value can be derived independently — an
      address is a hash of a record, **an author address is a hash of a key** —
      it is derived that way"*. The second clause is false: no author address
      exists, and no test in this file derives one.
      **Scenario:** a reader adding a fixture to this file follows the stated
      technique, writes `key.public_key().address()`, and does not compile —
      having been told by the file's own governing paragraph that this is how
      independent derivation is done here.
      **Measured:** all 26 surviving `address()` calls in this file are
      `Genesis::address()` (a Stoa address), verified by reading every one; the
      piece repointed all seven author-side call sites in this file to
      `stoa_public_key`/`public_key`, and left the paragraph that describes them.
      Severity: **defect** — this is the largest category of work the piece
      claims to have done, and this is a survivor of it.

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/tests/end_to_end.rs:650-655` —
      the rival-explanation comment on
      `a_keystore_on_disk_signs_a_post_that_a_reopened_store_still_attributes_to_it`
      describes an author address three times over, for a test whose body the
      piece changed to a public key. It says *"that the **author address** came
      out of the row we just wrote … the expected **address** is re-derived from
      the keystore REOPENED FROM DISK … a store that stashed the **address** as a
      column"*.
      **Scenario:** the code eleven lines below is
      `let expected_author = reopened_keystore.stoa_public_key(&stoa).to_hex();`
      and the assertion message the piece rewrote says *"attributed to the **key**
      the reopened keystore derives"*. A reader reconciling the comment with the
      assertion finds them naming two different values, and the comment is the
      one that states the test's whole reason for existing.
      **Measured:** the piece edited lines 701 and 716 of this same function and
      did not touch 650-655; `git diff origin/main...piece/key-sweep` shows the
      hunk starting at 698. Note line 672's *"The address is the hash of the
      record"* in the same function is **correct** — that one is the Stoa
      address — which is what makes this worth fixing rather than blanket-editing.
      Severity: **defect**.

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/identity.rs:863` —
      *"**THREE pins, where there were four.**"* Both numbers are wrong by two.
      `the_wire_constants_are_pinned_to_known_answers` contains **five**
      `assert_eq!` pins on this branch (lines 870, 875, 880, 906, 911 — Stoa
      address, signing digest, `derive_stoa_key`, `derive_stoa_key_at_path` at
      path 1, and at path 0) and had **six** on `origin/main`.
      **Scenario:** a reader who takes the count literally and finds five pins
      concludes two are undocumented strays and deletes one — or, reading in the
      other direction, believes the path-taking scheme is unpinned and adds a
      duplicate pin for it.
      **Measured:** `grep -c "assert_eq!"` over the test body on both sides;
      `origin/main`'s copy at lines 858/863/868/873/899/904 of
      `git show origin/main:…/identity.rs`. The comment is otherwise the good
      kind — it explains *why* the fourth was retired rather than relaxed — so
      only the tally needs correcting, not the paragraph.
      Severity: **defect** — a fabricated count, the failure mode this repo has
      recorded.

- [ ] **`spec-writer`** — `openspec/changes/key-sweep/specs/identity/spec.md:53-54` —
      the same wrong numbers, and this is where they came from: *"**Three pins
      remain because three derivations remain**; the fourth is gone"*. Four
      derivations remain (`stoa_address`, `signing_digest`, `derive_stoa_key`,
      `derive_stoa_key_at_path`) and five pins cover them.
      **Scenario:** the code comment in entry 3 is downstream of this sentence,
      so correcting only the comment leaves the spec contradicting the
      implementation it governs, and the next reader re-derives the wrong number
      from the authority. Fix both, or the tally reappears.
      **Measured:** the requirement's own scenario list names three pinned
      things (*A Stoa address derivation*, *The signing digest*, and the retired
      author-address one) and does **not** have a scenario for either
      key-derivation pin, which is how the undercount arose — the two
      `derive_stoa_key*` pins are in the test but in no scenario.
      Severity: **defect**.

- [ ] **`dev-writer`** — `openspec/changes/key-sweep/tasks.md:127-128` — task 5.2
      still reads *"Correct **the two** test comments crediting the address check
      … `op.rs`'s forged-op test and `revision.rs`'s `a_forged_revision_is_dropped`"*
      and is ticked. There were **four** such comments in three files, and this
      line names neither of the two the tester actually had to find.
      **Scenario:** `tasks.md` is the change's record of what was done. A reader
      auditing coverage later reads this line, sees `op.rs` and `revision.rs`
      named and ticked, and does not learn that `transport.rs:1441` was ever in
      scope — which is precisely the gap that let it survive the first time. The
      stage row at line 16 records the truth; the task line contradicts it.
      **Measured:** commit `a5f83ba` touched `op.rs` (two test comments, 1427 and
      1444), `transport.rs:1441` and `revision.rs:42` — four sites, three files.
      Severity: **defect** — the piece's own instruction is to correct claims the
      code disproves, and this is one.

- [ ] **`dev-writer`** — `openspec/changes/key-sweep/tasks.md:135-136` — task 5.3's
      recorded verification is still `grep -i "author address"`, ticked, with no
      note that it was proved insufficient. The tester demonstrated that the
      surviving false claims said *"re-deriving the address"* and contained no
      such substring.
      **Scenario:** the next sweep in this repo copies the recorded verification
      — that is what a ticked verification line is for — and reproduces the same
      blind spot. This is the recorded "gate the defect satisfies" pattern left
      in the document that teaches it.
      **Measured:** `grep -i "author address"` over `transport.rs:1441` and
      `revision.rs:42` as they stood before `a5f83ba` returns nothing; both said
      "re-deriving the address". Fix is to record the several spellings the
      claim takes, not just the one phrasing.
      Severity: **defect**.

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:12628-12631` —
      a mutation claim lost its attestation while keeping the claim. `origin/main`
      read *"The mutation it catches, **verified by running it**: change the
      probe's lookup to …"*; the rewrite reads *"The mutation it catches: change
      the probe's lookup to `ks.stoa_public_key(&stoa)` … and this test fails,
      where the whole rest of the suite passes."*
      **Scenario:** CLAUDE.md's rule is that a test nobody has watched fail is a
      test nobody knows works. The surviving sentence asserts a *measured*
      outcome ("the whole rest of the suite passes") in the voice of a
      measurement, with the words that said it was measured removed — so a
      reader cannot tell an experiment from a prediction. Either re-run it and
      restore the three words, or mark it as a prediction.
      **Measured:** `grep -n "verified by running it"` against
      `git show origin/main:…/wire.rs` returns line 12564; the branch's
      corresponding comment has no such phrase. I did not re-run the mutation,
      so this is about the attestation, not about whether the claim is true.
      Severity: **defect**, low cost.

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/identity.rs:936-941` —
      the comment describes a reconstruction the code does not perform. It says
      the retired value *"is reconstructed here from the hardcoded prefix bytes —
      NOT from a constant this file still holds"*, but nothing is reconstructed:
      line 956 is a hardcoded hex literal.
      **Scenario:** a reader trusting the sentence looks for the `Sha256::new()`
      the paragraph implies, does not find it, and cannot tell whether the
      reconstruction was removed or was never there — which matters, because the
      distinction the paragraph is drawing (recomputed vs. pinned) is the whole
      argument for why this test is a witness rather than a tautology.
      **Measured:** I ran the reconstruction the comment describes as a probe.
      The pinned hex **is** the retired derivation's output, so the substance is
      correct and only the verb is wrong — say "carried over verbatim from the
      retired pin, and separately measured to be that derivation's output",
      which the next paragraph already almost says.
      Severity: **stylistic** — imprecise wording, not a false claim about
      behaviour.

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:5589` —
      cites a test that does not exist:
      `the_path_taking_scheme_does_not_collide_with_the_pathless_one`. The real
      name is `the_path_taking_scheme_does_not_collide_with_the_scheme_without_one`
      (`identity.rs:1421`).
      **Scenario:** the comment tells a reader that `identity.rs` asserts the two
      schemes must disagree — a load-bearing claim, since it is what makes the
      defect "not a near-miss". A reader grepping the quoted name finds nothing
      and cannot confirm it.
      **Measured:** `grep -rn "fn the_path_taking_scheme_does_not_collide"`
      returns only the `..._with_the_scheme_without_one` form. **Pre-existing** —
      `origin/main:wire.rs:5526` carries the identical wrong name — but it sits
      four lines above a hunk this piece edited, so it is cheap to fix here.
      Severity: **defect**, pre-existing.

- [ ] **`dev-writer`** — `openspec/changes/key-sweep/tasks.md:62` — task 1.2's
      verification is *"Verify by reading: the phrase 'One type for both' appears
      nowhere"*, ticked, but the phrase survives at `identity.rs:108` (*"one type
      for both, the same construction over two prefixes"*).
      **Scenario:** the surviving instance is a past-tense historical clause and
      is correct prose — the `Address` doc genuinely does read as a Stoa type
      positively now, opening *"A 32-byte **Stoa** address"* and *"An address
      identifies a Stoa, and nothing else."* So the code is right and the
      *verification* is what is false. A reader re-running the stated check to
      confirm the task finds it fails and cannot tell whether the task regressed.
      **Measured:** `grep -rn "one type for both"` over `dialectica/` returns
      exactly one hit, `identity.rs:108`. Restate the verification as what was
      actually intended — that the phrase appears only in a past-tense clause.
      Severity: **stylistic** — a wrong verification recipe, not wrong code.

---

## Clean, and worth recording as checked

**The prose scars the brief asked about are not there.** `identity.rs`'s
`Address` doc leads with what the type *is* (`A 32-byte **Stoa** address: the
domain-separated hash of a genesis record`) and states the narrowing positively
before mentioning what was removed; it also earns its place by explaining why
nothing in the type system enforces it and why `derive_stoa_key` taking an
`Address` is an input rather than a leftover. `Identicon.qml`'s retained
`address` property is explained correctly and for the right reason — the
component is genuinely generic, and I verified it still has live Stoa callers in
`DStoaListScreen.qml`, `DJoinScreen.qml` and `FeedScreen.qml`, so the comment's
claim that renaming "would narrow a component that still has a Stoa-address
caller" is true rather than a rationalisation. `PostHeader.qml` was renamed to
`identityKey` and its header comment rewritten in the same pass;
`AddressLabel.qml` correctly keeps `address` and says why.

**The `thread.rs` two-fields-to-one collapse is the best-documented change in
the piece** — the struct doc explains what the second field was for, why that
reason is gone, and why a second identifier beside the key is worse than none,
and the test gained a destructure so re-adding `author_key` stops compiling.

**The three repetitions of "the mark read the address" are accurate history**,
not a fabricated justification: `origin/main`'s `Identicon.qml` records that
earlier design explicitly.

**One paraphrase loosens without becoming false.** `wire.rs:13777` widens
*"An author address SHALL NOT be required, in addition to or in place of the
key"* to *"nothing beyond the key be required"*, keeping the trailing fragment.
The broadening is deliberate and the sense is preserved; I raise no box for it.

**Two pre-existing numeric contradictions I did not open boxes for**, because
neither is this piece's and neither is in a line it touched:
`Identicon.qml:151` says "eleven forms" while `:263` says "Ten pooled forms"
(eleven is right — `_byte(4) % 11`, 11 switch branches); and
`docs/IDENTICON.md:724` attributes `NAME_PREFIX`'s removal to #80 when
`names.rs` attributes it to the earlier `key-identity` change. Worth someone's
attention eventually; not a gate on this merge.

**Function-length and naming**: no function in the diff acquired a second job.
The one collapse — `creator_and_poster_in`/`poster_address_in` into
`creator_key_in` — removes a name, and `verify_authored_op` lost a parameter
rather than gaining one. Nothing here reads as an `And` or a vague `handle`.

**The tree was left clean.** The `dialectica/logos-rust-sdk-src` symlink I
created to make `cargo test` resolve is gitignored and outside the index.
