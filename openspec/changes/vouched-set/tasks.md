# Tasks

This change is **planning only**. It writes no code, and the implementation tasks
below are recorded so that the change that does write it inherits them rather
than rediscovering them.

## 1. Design (this change)

- [x] `proposal.md` — why the storage question is its own change, and the
      provenance table that makes the seam visible.
- [x] `design.md` — where each half lives and why not the keystore or the
      projection; the replay answer and what breaking it costs; what a vouch for
      an unseen identity resolves to; the fail-open argument with "safe" and
      "correct" separated; export between devices against the §5.2 risk; the
      minimum wire surface and why the reader's own list is safe to expose; what
      §7.3 costs more than it reads; what cannot be tested.
- [x] `specs/vouched-set/spec.md` — the delta. **Not promoted**: merging it into
      `openspec/specs/` belongs to the implementing change.
- [x] PLAN.md §11.1 — a home for the rendering obligations, and cross references
      from the three sections that had been carrying theirs alone.

## 2. Inherited by the implementing change

- [ ] The local store's **authoritative** and **derived** regions named
      explicitly, with rebuild defined as an operation over the derived region.
      The delta's rebuild scenarios are what pins it. Spell rebuild as "drop the
      tables in the derived set", never as "drop everything except the ones I
      remembered" — `design.md` argues why the second is the silent-loss shape.
- [ ] Declared vouches and dismissals in the authoritative region. Earned weight
      in the derived region, recomputed from the reader's own assessment ops.
- [ ] The unreadable state, distinguishable from empty, **read-only on the write
      path**. This is the requirement most likely to be implemented as "treat it
      as empty" in both directions, which is permanent data loss.
- [ ] The wire surface: vouch, unvouch, dismiss, list. Provenance on every list
      entry; no numeric weight; no per-identity probe; no count; no parameter
      naming a reader. Idempotent writes.
- [ ] Export and import, per Stoa, carrying dismissals, carrying no secret
      material, refusing a Stoa mismatch. Blocked on the encryption question in
      `design.md`'s Open Questions, which decides the wire shape.

## 3. Inherited by the scorer change, not by this one

Recorded here because an obligation split across a change boundary is where one
usually gets lost.

- [ ] **Vouching amplifies `constructive` only, never `noise`.** §7.3 and §7.2
      rule 4. Not testable in this change — there is no scorer, and a scenario
      about behaviour that does not exist is the thing
      `.claude/agents/README.md` names explicitly as not to write.
- [ ] **Earned weight capped below `K_vouch`, and `K_vouch < K_mod`.** The
      constants are the scorer's; this change only keeps the two provenances
      distinguishable, which is the precondition for the cap being expressible.
- [ ] **Weights joined at query time, never stored against a vote.** §7.2 rule 5.
      Earned weight living in the derived region satisfies this by construction;
      the scorer must not undo it by materialising a weighted total.
- [ ] **Decay of earned weight**, deferred with §7.2 rule 5's absent age. Whether
      a dismissal expires is open and wants the same input — `design.md`'s Open
      Questions.

## 4. Gates

- [x] `cargo test --manifest-path <WT>/dialectica/rust-lib/Cargo.toml -p dialectica-core`
      — unchanged by this change, run to confirm nothing broke.
- [x] `cargo fmt --manifest-path <WT>/dialectica/rust-lib/Cargo.toml --check`
      — no `-p`; expect 0.
