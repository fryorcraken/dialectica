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

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:2754` —
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

- [ ] **`dev-writer`** — `openspec/changes/expose-name/design.md:121-123` —
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

- [ ] **`tester`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:13169-13171` —
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

---

## A fabricated wordlist citation

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/wire.rs:2788`
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

---

## A rationale whose own cited precedent contradicts it

- [ ] **`dev-writer`** — `dialectica/rust-lib/dialectica-core/src/lib.rs:49-58`,
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

---

## A superseded mandate left contradicting its own change

- [ ] **`spec-writer`** — `openspec/changes/expose-name/proposal.md:122-131` —
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
