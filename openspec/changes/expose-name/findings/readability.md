# Readability findings — expose-name

Dimension reviewed: **readability only**. Correctness, security and architecture
are held by other instances.

Every number, count and quantity in every comment added by this change was
checked against the code or against a run. Five numeric claims were verified
sound and are listed in the prose at the bottom; **four are wrong** and have
boxes below.

The defects cluster into one family: the hex-length bound's "this is not a
validity check" argument is made in three files, and in each one the *example
chosen to prove it* is a length the bound itself catches. The argument is
correct; every instance of its evidence is not.

---

## The hex bound's rationale cites examples that disprove it

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:2754` —
      `MAX_PUBLIC_KEY_HEX_CHARS`'s doc says the identity layer refuses "the 62-
      and 66-character strings that clear this". A 66-character string does not
      clear a `> 64` bound — it is refused *by the bound*, which is the exact
      opposite of the sentence's point, and 66 is the one example that disproves
      the paragraph it appears in.
      **Scenario:** `{"publicKey":"ab"×33}` (66 chars) →
      `{"error":"publicKey is 66 hex characters, over the 64 a public key holds"}`.
      The doc predicts `cannot derive a display name: not a valid public key`.
      **Measured:** run against the branch; 62 chars gives the identity layer's
      message, 66 gives the size message. Only 62 supports the claim.
      **Severity:** genuine defect — a reader trusting this comment will believe
      the bound is looser than it is. Fix: cite 62 alone, or 60 and 62.

      **Fixed** in `ff041e8`. The doc now cites **58, 60 and 62** — three
      under-bound lengths, all measured — and states the division explicitly
      rather than leaving it to the examples: at or under 64 the bound is
      transparent, over 64 it is the sole decider.

      Ran every length rather than reasoning about them, as instructed. 29 bytes
      (58 chars), 30 (60), 31 (62) and 32 (64) all return
      `cannot derive a display name: not a valid public key`; 33 bytes (66) and
      34 (68) return the size refusal. So 66 was the one example in that
      sentence that disproved it, exactly as you measured.

      The doc also now names the error that was there, because "an earlier
      wording cited 66 as a string that clears this, which is exactly backwards"
      is the kind of thing a reader benefits from seeing corrected rather than
      silently absent — the wrong version is intuitive enough to be re-derived.

- [x] **`dev-writer`** — `openspec/changes/expose-name/design.md:121-123` —
      decision 3 makes the same error with different numbers: "A 30-byte or
      34-byte hex string passes this bound and is refused by the identity layer."
      A 34-byte hex string is 68 characters and does not pass the bound.
      **Scenario:** `{"publicKey":"ab"×34}` (68 chars) →
      `{"error":"publicKey is 68 hex characters, over the 64 a public key holds"}`,
      not the identity layer's refusal the sentence promises.
      **Measured:** run against the branch. 30 bytes (60 chars) behaves as
      described; 34 bytes does not.
      **Severity:** genuine defect. Fix: pick two under-bound lengths (e.g. 30
      and 31 bytes).

      **Fixed** in `ff041e8`. §3 now cites **29, 30 and 31 bytes** as the
      material that clears the bound and is refused by the identity layer —
      measured, not chosen by arithmetic — and says outright that above 64
      characters the bound is the sole decider and the identity layer is never
      reached.

      Confirmed your 68-character measurement (34 bytes →
      `publicKey is 68 bytes, over the 64 a public key's hex holds`). The entry
      also now records *which* half of the original sentence was true and which
      was backwards, because the over half was the half the sentence existed to
      defend — the entry's whole point was that the bound decides no validity,
      and on the over side it decides everything.

      The claim is no longer only prose:
      `the_bound_decides_every_over_length_refusal_and_the_identity_layer_never_sees_one`
      asserts both sides, so the next version of this sentence cannot drift from
      the behaviour without a red test.

- [x] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:13169-13171` —
      the comment inside
      `the_hex_bound_refuses_an_oversized_key_without_deciding_validity` says
      "62 and 66 hex characters are both under or at the cap's neighbourhood",
      but the loop below it is `for bytes in [31usize, 32]` — 62 and **64**
      characters, not 66. The comment describes a case the test deliberately does
      not run, and could not run, since 66 would be refused by the size message
      the assertion forbids.
      **Scenario:** a reader adding 33 bytes to that array on the comment's
      authority gets a failing test: the reply is
      `publicKey is 66 hex characters, over the 64 a public key holds`, which
      contains `"over the"` and trips the `assert!`.
      **Measured:** loop runs 62 and 64 chars; 66 chars produces the size
      message.
      **Severity:** genuine defect — the comment misnames the fixture beside it.

      **Fixed** in `ff041e8`. The comment now says **62 and 64** — what
      `[31usize, 32]` actually runs — and goes further, because the trap you
      describe is worth closing rather than just correcting: it states that 66
      is deliberately **not** in the array, that it is over the bound, and that
      the size message is the *correct* answer for it, so a reader adding 33
      bytes on the comment's authority is warned off before writing the failing
      test you predict.

      Verified your prediction directly — 66 characters returns a message
      containing `over the`, which is what the `assert!` in that loop forbids.

      The comment also points at the new
      `the_bound_decides_every_over_length_refusal_and_the_identity_layer_never_sees_one`,
      which is where the over-64 cases now live, so the two tests read as a pair
      with a stated division rather than as one test with a confusing gap.

---

## A fabricated wordlist citation

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:2788`
      and `openspec/changes/expose-name/design.md:154` — the two-word toponym
      offered as proof that `words` cannot be recovered by splitting `name` is
      `alexandria troas`, which **is not in `wordlists/places.txt`**. The list
      holds bare `alexandria` (line 49) and no `troas` compound. The example was
      carried over from the archived `generated-names` change's prose
      (`openspec/changes/archive/2026-09-14-generated-names/tasks.md:128`) without
      being checked against the list that shipped.
      **Scenario:** a reader verifying the justification greps `places.txt` for
      `alexandria troas`, finds nothing, and concludes the `words` field is
      unjustified — when it is in fact justified by ten *real* multi-word entries
      the comment failed to name.
      **Measured:** `grep -c " " places.txt` → **10** multi-word entries, none of
      them `alexandria troas`; the real ones include `antiocheia maiandros`,
      `euxeinos pontos`, `kimmerian bosporos`, `lokroi epizephyrioi`.
      **Severity:** genuine defect, and this repo's recorded "persuasive
      citations get fabricated" failure mode. The reasoning is sound — replace
      the invented example with one of the ten that exist.

      **Fixed** in `ff041e8`.

      Confirmed both halves independently: `grep -c " " places.txt` → 10, and
      `alexandria` is at line 49 with no `troas` anywhere in the file. The ten
      are `antiocheia maiandros`, `arsinoe kyprou`, `euxeinos pontos`,
      `herakleion egyptou`, `kimmerian bosporos`, `lokroi epizephyrioi`,
      `makaron nesoi`, `rhode iberias`, `seleukeia kalykadnos`,
      `thermai himeraiai`.

      The code comment now cites `thermai himeraiai` and `kimmerian bosporos`;
      `design.md` §5 lists **all ten**, so the claim "the list holds ten" is
      checkable against the list rather than against a sample. Both places also
      record that the earlier example was invented, since the reader most likely
      to check a citation is the one a fabricated one misleads — your phrasing,
      and it is the reason this is worth a note rather than a silent swap.

      The stronger fix is beside it: the spec-test reviewer found that the same
      claim was pinned by nothing (a split-on-spaces mutation passed the whole
      suite, because no fixture drew one of the ten). Seed 166 draws
      `thermai himeraiai`, and
      `words_is_the_three_drawn_words_and_not_the_rendered_name_split_on_spaces`
      now asserts against it — so the justification is executable, not just
      correctly cited.

---

## A rationale whose own cited precedent contradicts it

- [x] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/lib.rs:49-58`,
      `dialectica/rust-lib/src/lib.rs:911-918` and
      `openspec/changes/expose-name/design.md:180-182` — roughly 30 lines across
      three files argue that `display_name` is spelled `core::wire::` *because*
      it is not re-exported, calling it "the one handler that is not" and citing
      `get_capabilities_from_stores` and `publishing_key` as the established
      precedent for that form. But `get_capabilities_from_stores` **is** in the
      re-export list (`dialectica-core/src/lib.rs:61`), and the adapter still
      spells it `core::wire::` (`dialectica/rust-lib/src/lib.rs:764`). The cited
      precedent shows that spelling and re-export are independent, so it
      undercuts rather than supports the claim built on it. design.md:181 states
      it most plainly — "the established form for handlers outside the list" —
      naming a handler that is inside the list.
      **Scenario:** a reader who checks the citation finds the counter-example,
      and cannot tell whether the omission from the re-export list is load-bearing
      or incidental — which is precisely the question 30 lines were spent
      answering.
      **Severity:** genuine defect in the *reasoning*, not the code. The decision
      (omit from the re-export list to avoid `core::display_name` sitting beside
      `core::names::display_name`) stands on its own; drop the false precedent
      claim, or restate it as "the adapter already uses the `core::wire::` form
      for some handlers regardless of re-export".

      **Fixed** in `ff041e8`, taking your restatement and making the inference
      explicit rather than leaving it to be re-derived.

      Verified every leg: `get_capabilities_from_stores` is in the re-export
      list (`dialectica-core/src/lib.rs:61`) and the adapter still spells it
      `core::wire::` (`rust-lib/src/lib.rs:764`); `publishing_key` appears in
      `dialectica-core/src/lib.rs` **only** in the comment, so it is genuinely
      absent from the list and is the one real example.

      Both `lib.rs` and `design.md` §6 now say what the precedent shows and what
      it does not: the adapter uses the long form for handlers **both in and out
      of** the list, so spelling and list membership are **independent**, and
      the precedent is therefore not evidence that omission is required. The
      omission rests on the ambiguity argument alone — which is enough, and
      saying "alone" is what stops the next reader hunting for a second reason.

      Both places name the wrong wording ("the established form for handlers
      outside the list") so that a reader who half-remembers it can see it was
      withdrawn. Note the adapter's own comment at `rust-lib/src/lib.rs:911-918`
      never carried the false claim — it argues purely from the ambiguity — so
      it needed no change, which I checked rather than assumed.

---

## A superseded mandate left contradicting its own change

- [x] **`spec-writer`** — `openspec/changes/expose-name/proposal.md:122-131` —
      the proposal still carries the instruction that **"`docs/UI-BRIEF.md`
      Obligation 6 becomes wrong when this lands, and the `dev-writer` MUST fix
      it in this change"**, and quotes the brief as authority for what core does
      and does not reach. `tasks.md:80-88` records that the owner overrode this
      mid-task: the file's content is ruled misleading, it is being deleted by
      another piece, and it is not to be edited, cited, or treated as a
      requirement.
      **Scenario:** the two documents in one change now give opposite
      instructions. A reader arriving at `proposal.md` first — the document that
      is read first by construction — acts on a MUST that `tasks.md` two files
      later says was withdrawn, and edits a file scheduled for deletion.
      **Severity:** genuine defect. The citation should be removed and the claim
      re-grounded in `openspec/specs/generated-names/spec.md`, as `tasks.md`
      records was already done for `design.md`. Dropping the bullet entirely is
      also fine — `design.md`'s "What this change does not do" already records
      that the feed-row gap stays open, which is the only durable half of it.

      **Fixed** in `ff041e8` by the `dev-writer`, since the `spec-writer` is not
      coming back to this piece and an unanswered box blocks the merge.

      Took your first option. The bullet no longer mentions `docs/UI-BRIEF.md`,
      carries no MUST, and quotes it as authority for nothing. The durable half
      — one of the two gaps closes here, the other does not — is re-grounded in
      `openspec/specs/generated-names/spec.md:39-45`, which I read before
      repointing at it: it says a name derived from an author address differs
      from the name for that key, "**AND** so a caller holding only an address
      cannot arrive at the right name". That is exactly the claim the bullet
      needs, so the re-grounding is a real one rather than a citation swap.

      Kept the bullet rather than dropping it because it is the only place the
      proposal says what this change does *not* deliver, and a reader deciding
      whether a feed row can render a name should meet that in the proposal
      rather than only in `design.md`.

      The owner's override stays recorded at `tasks.md:80-88`, which is the
      right place for it — it is a record of a decision, not an instruction, so
      it does not contradict anything now that the proposal's MUST is gone.

---

## What was clean

The numeric claims that **do** hold, each checked by running the code rather
than by reading it:

- **10,240 wordlist entries** (`wire.rs:2769`, `dialectica/rust-lib/src/lib.rs`)
  — `wc -l` gives 8192 + 1024 + 1024 = 10,240 exactly, in both places it appears.
- **`PINNED_KEY_HEX` is seed `[13u8; 32]`** (`wire.rs:194`) — `feed_key(13)` is
  `SecretKey::from_bytes(&[13; 32])`, whose public key is
  `91a28a0b…ba9a4b3a`, matching the constant character for character.
- **`PINNED_NAME_ON_THE_WIRE`** (`wire.rs:212`) — that key derives
  `riskful megaron of anemourion`, as written.
- **The digest `3892664a…247f73` and indices (6290, 586, 81)** (`wire.rs:200-203`)
  — computed independently: the digest is
  `3892664a58512a2020d8361b6a50d4e80bab06fa4b74b45a3f8b37bead247f73` and the
  three reductions give 6290, 586, 81. All three exact.
- **The 4 MiB / 2 MiB allocation arithmetic** (`wire.rs:2745-2746`,
  `design.md:109-112`) — `MAX_REQUEST_BYTES` is `4 * 1024 * 1024`, and a hex
  string of that length decodes to half, so "a 2 MiB `Vec`" is right.
- **`examples/pin_name.rs` exists and does what the comment says it does** — it
  reads the three text files, asserts their sizes, and carries no
  `use dialectica_core::names`, so the pin really is independent of the
  derivation. The comment citing it cites something real.
- **`get_capabilities_from_stores` and `publishing_key` really are spelled
  `core::wire::` in the adapter** (lines 764 and 647) — the citation is accurate
  even though the inference drawn from it is not (see the box above).

Other readability dimensions checked and found clean:

- **One function, one job.** `display_name` parses a field, bounds it, decodes,
  delegates, renders. No `And`, no vague verb, no mid-body phase comment. The
  adapter method is a genuine one-liner.
- **Idiom match with `wire.rs`.** The file carries 1363 doc-comment lines and 66
  `/// # ` headings across its handlers, so the new method's four headings and
  long rationale block are squarely in house style rather than unlike its
  neighbours. The `// ─── … ───` section banner matches the file's existing
  dividers.
- **Error messages are intelligible to a caller who cannot read the source.**
  `missing field: publicKey`, `publicKey must be a string`,
  `publicKey is not valid hex`, `publicKey is N hex characters, over the 64 a
  public key holds`, and `cannot derive a display name: <identity layer's
  words>` each name the field and the fix. The low-order case reads
  `low-order public key, which can never verify a signature`, which tells a
  caller something actionable rather than leaking internals.
- **Comments say why, not what.** The load-bearing ones — why the bound precedes
  the decode, why both `name` and `words` ship, why no reply gains a name field,
  why the pinned test must not be updated to match — all record a rejected
  alternative rather than restating the line below them.
- **Tests cited by comments exist.** Every test named in a comment
  (`the_sweep_covers_every_request_taking_method_the_dispatch_trait_declares`,
  `a_served_request`, the `names.rs` refusal tests) is present and passes; no
  comment justifies a branch by citing a test that was never written.
- **`design.md` is followable by someone who was not here.** Decision 1's
  batch-shape argument in particular is complete: it names the pressure, walks
  the three candidate shapes, says which convention forbids which, and records
  that the decision is reversible. Decision 6 is the one whose reasoning does
  not survive checking (box above).

The full suite was run on this branch: **90 passed, 0 failed** for the
name-related filter, with `end_to_end` green alongside it.
