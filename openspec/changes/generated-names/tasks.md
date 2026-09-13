# Tasks

## Stages

- [x] spec — `spec-writer` — **Second pass: owner ruling, a scope reduction.** The
      capability is now the derivation alone: three words from an author's public
      key, and **the three words do not go over the wire**. The requirement that a
      reply reporting an author carry either the name or the key is **deleted**,
      with every scenario about what a feed row or any other reply carries. The
      cross-capability contradiction with `thread-read` dissolves rather than being
      narrowed — `thread-read` carries no name and now nothing does. What core
      exposes is the **derivation**, which matters more under the ruling, not less.
      The gloss stays, decided explicitly: it is not a name and is not derived, but
      it is data on a wordlist entry and this is the only capability that
      enumerates the entries. `docs/PLAN.md` and `docs/UI-BRIEF.md` need no
      correction — obligation 6 already states the position the ruling adopts.
- [x] design + code — `dev-writer` — The four lists are written and the crate is
      green. Sizes are exactly 8,192 / 1,024 / 1,024 and the denylist holds 199
      pairs. **All four modules are generated from text files by `tmp/gen.rs`
      rather than transcribed**, which is what makes the attestation screen
      auditable: a reviewer can trace an entry to a census file and a census file
      to its source. Yields were 17,349 / 1,892 / 1,131, so every list was cut
      down rather than padded up.
- [x] tests — `tester` — Two gaps closed, both measured: a wordlist **reordering**
      was invisible (exchanging places 100/101 left the suite green), and the
      **collision** case had no fixture. The colliding pair was found by searching
      the shipped 2³³ scheme rather than stubbing the derivation. Mutations run
      with predictions stated first; one prediction was wrong and is recorded.
      **Three gaps stay open and are reviewer-visible rather than fixed**:
      `feed.rs:292` cites a test that does not exist, and neither the scheme
      version nor a wordlist removal can be varied without widening the API for
      tests alone.
- [x] review: correctness — `code-reviewer`
- [x] review: security — `code-reviewer` — Seven findings in
      `findings/security.md`, two high. A **fabricated placeholder name** replacing
      the `feed.rs:293` failure path ships on the wire with **946 of 946** tests
      green — the spec forbids a placeholder in terms, and the comment citing a
      test that covers it names a test that does not exist. And
      `tst_identicon.qml:103` is **one-sided**: it hardcodes the displayed byte
      groups instead of deriving them from `Theme.headChars`, so setting
      `headChars: 24` puts the mark's whole `4..11` window back on screen with all
      **42** QML tests green. Clean: the ASCII bidi screen (verified per entry —
      every non-ASCII byte in the wordlists is an em-dash in a doc comment), the
      malformed-key path, and the multiply-not-add domain-separation argument.
- [x] review: readability — `code-reviewer` — 15 findings in
      `findings/readability.md`. The module's numeric claims are the problem: the
      denylist arithmetic in `names.rs:358-362` is the paragraph PLAN.md struck
      through and proposal.md withdrew, and the shipped list holds 199 pairs
      against its "about 960". Every worked example in the module — *measured
      aporia of lampsacus*, `brittle`, `damp`, `alexandria troas` — names entries
      that are not in the lists. `feed.rs:292` cites a test that does not exist.
- [x] review: architecture — `code-reviewer` — Six findings in
      `findings/architecture.md`. The change added `displayName` to the feed row
      and never noticed the **second** author-reporting surface: `thread_page_json`
      ships `authorKey` and no name, and `openspec/specs/thread-read/spec.md:192`
      requires it carry none — so this change's "a name wherever core reports an
      author" and a merged requirement now contradict each other, with the losing
      one enforced by a green test. Also: `tmp/gen.rs` and every wordlist source
      file are **gitignored and absent**, so D1's auditability argument has no
      artefact behind it once the author's worktree is pruned.
- [x] review: spec-test — `spec-test-reviewer` — 10 findings in
      `findings/spec-test.md`. Nine mutations run, **four survived**, three of
      those `const` or QML `Theme` values that `cargo mutants` cannot reach:
      the **denylist's 199 pairs are pinned by nothing** (deleted two real
      figures' pairs, 946/946 green); the **abbreviation side of the
      channel-disjointness requirement has no test at all** (`headChars: 24` or
      `middleChars: 24` puts the mark's window back on screen, 42/42 QML green);
      and `feed.rs:293`'s spec-forbidden placeholder passes. The blocker is a
      **cross-capability contradiction** — this spec's "every reply carrying an
      author carries a name" against `thread-read`'s "no item SHALL carry a
      display name", both live, `validate --strict` blind to it, and the tests
      firmly enforcing the side this change loses. Two `tasks.md` claims of
      untestability were disproved by writing the tests.
- [x] review: design — `design-reviewer` — 14 findings. D3/D4/D6–D10/D13 verified
      against the code and the exclusion withdrawal is clean. **One behavioural
      finding**: `feed.rs:293` silently drops a post whose author's name cannot be
      derived — unrecorded, unpinned, and content censorship by a name failure.
      The denylist is **199** pairs, so D5's "~1,000" and the 0.092% / 1-in-1,090
      / 1-in-1.2-million figures in `names.rs` and `feed.rs` are wrong by factors
      of 5 to 23 — they import the premise PLAN.md withdrew. D2's recorded
      `NAME_PREFIX` has seven NULs where the code has six, and the
      `identity.rs` assertion it cites as its justification does not exist.
      D1's provenance chain is under gitignored `tmp/` and does not survive merge.
      PLAN.md still says the mark reads `12..19` in four places.
- [ ] findings all ticked, `findings/` deleted — runner
- [ ] `openspec validate --strict`, then `archive` — runner

## Three owner decisions, all now in the spec

The `spec-writer` pass has landed all three. What follows is the record of what
was decided; the contract is in the spec and the reasoning belongs in
`design.md`. A `dev-writer` pass implements them.

**3. No filter on any word and no filter on any output.** The true-attribution
denylist is deleted along with the redraw, the reserve bytes and
`ReserveExhausted` — see section 3 below. The accepted consequence is that an
identity may derive to a real historical figure's canonical name and, there
being no rotation, sign every post with it. **Exactly one rule keeps anything
out**: no wordlist entry may contain the connector as a word, which is a rule
about an entry's spelling and not about any word's meaning.

**1. Nouns and places each need a one-sentence gloss.** A user shown *pensive
aporia of lampsakos* has no way to learn what `aporia` or `lampsakos` mean, and
the QML sandbox forbids a view looking anything up, so the gloss can only come
from core. **Scoped to noun and place only** — an English adjective needs no
translation for an English-speaking reader. ~2,048 glosses, not 10,240.

**Fetched per word.** The spec requires core to answer a gloss request for any
entry of either Greek list. Under the owner's second ruling there is no bundling
question left to answer — no reply carries a name, so there is no reply for a
gloss to ride beside. A gloss does not participate in the derivation, and
changing one is **not** a scheme version bump — which is the opposite of every
other change to an entry.

**Kept in this capability, decided explicitly rather than left ambiguous.** A
gloss is not a name and is not derived from a key, so it is not one of the three
words the ruling scopes this to. It stays because it is data attached to a
wordlist entry, this capability is the only place the entries are enumerated, and
the sandbox argument that puts the derivation in core puts the gloss there for
the same reason.

**2. PLAN.md's section on what an identity is called has been SHED.** It ran
1,039 lines (956–1994, about a fifth of the document) and now runs 135. The
derivation, byte budget, arithmetic, list sizes and screens are the spec's; the
reasoning behind each is `design.md`'s. What stayed in PLAN.md is what is not
built (two open questions), the grinding threat-model conclusion — which is
about the product rather than this derivation, and whose answer is *nothing in
this scheme defends against impersonation; the address is the identity* — and
the rendering obligations bound for §11.1.

## Implementation

> **The sizes are settled at 8,192 / 1,024 / 1,024 and the options in `design.md`
> are closed.** The owner has ruled; the powers of two are load-bearing for the
> modulo argument and are not reopening.
>
> **The earlier censuses do not measure the current scope.** Both were written
> against a noun slot read as "the vocabulary of Greek thought plus named
> thinkers", which is a narrow technical vocabulary. The slot is now **any
> attested ancient Greek noun**, and the two pools that were absent entirely —
> mythological figures and ordinary concrete nouns — are the larger half. A count
> taken under the old scope is not evidence about the new one, in either
> direction. The place census (`tmp/places-census/`, 1,070 deduplicated) is under
> unchanged scope and does still apply, with the caveat its own author recorded:
> about 850 entries they stand behind and about 220 they would cut first, so 1,024
> clears only by keeping soft material.
>
> **What every remaining list task must not do is pad.** Near-duplicate
> transliterations, Latinised doublets and invented toponyms each fail the spec's
> attestation screen, and **a fabricated Greek word is invisible to a reviewer and
> uncatchable by any test** — it is the one requirement the suite structurally
> cannot fail on. Build the lists from sources, not from recall.

## 1. The derivation

- [x] 1.1 `names.rs` carries `NAME_PREFIX`, `NAME_DIGEST_BOUND = 12` and
      `name_from_digest`. `the_derivation_reads_no_byte_past_its_bound` builds two
      digests agreeing inside the bound and differing on **every** byte beyond it
- [x] 1.2 `DisplayName` is three `&'static str`; `render()` is the one place the
      connector is emitted, so "the connector is not a slot" holds **by
      construction** — it does not exist in the data.
      `a_name_is_three_drawn_words_and_a_fixed_connector` counts by **slot, not by
      space-separated token**, because a two-word place makes the token count 5
- [x] 1.3 `display_name(&PublicKey)` hashes `NAME_PREFIX || key`;
      `the_name_digest_is_neither_the_address_nor_a_bare_hash` pins it against both
- [x] 1.4 `display_name_from_bytes(&[u8])` parses first. Lengths `0, 1, 31, 33, 64,
      1024`, a non-point, and the all-zero low-order key are each refused. **The
      wrong-length case is satisfied-by-construction for every caller holding a
      parsed key**: `display_name` takes `&PublicKey`, which cannot be built from
      malformed bytes

## 2. The wordlists

- [x] 2.1 `names/adjectives.rs` — 8,192 entries, derived from
      `/usr/share/dict/words` by suffix (see design D12). Swept for size, ASCII,
      lowercase, well-formed spacing and duplicates
- [x] 2.2 `names/nouns.rs` — 1,024 entries from the four pools, 1,892 counted.
      Same sweep, plus `no_noun_entry_carries_the_connector_as_a_word`. **No
      exclusion test**: the three that asserted `stoa`, `platon`, `sokrates`,
      `archon` and `strategos` were ABSENT are deleted, and
      `the_lists_carry_no_exclusion_of_any_kind` asserts they are **present** —
      their absence would now be the defect. That test is pinned positively rather
      than merely omitted, so a later curation pass that quietly dropped them
      cannot reintroduce the withdrawn screen with the suite still green
- [x] 2.3 `names/places.rs` — 1,024 entries, 1,131 counted, 10 multi-word.
      `a_multi_word_place_entry_is_accepted_and_renders_as_one_place` asserts the
      list holds at least one, so the single-word screen cannot creep back
- [x] 2.4 `every_index_of_every_list_is_reachable_and_uniformly_so` sweeps the full
      16-bit range and **counts into a vector** rather than asserting from the
      arithmetic, since the arithmetic is what is under test

## 3. Removing the denylist and the redraw

**Owner ruling: no filter on any word and no filter on any output.** Tasks 3.1
to 3.4 built a denylist of noun–place pairs and the whole-name redraw that
served it. All of it comes out. The spec now requires that nothing filters a
drawn name, and the only rule left that keeps anything out is the ` of `
spelling rule on wordlist entries.

- [ ] 3.1 Delete `names/denylist.rs` and every reference to it
- [ ] 3.2 Delete the redraw: `draw_at` is called once, at offset 0, and there is
      no second draw and no refusal check
- [ ] 3.3 Delete `NameError::ReserveExhausted`. It has no remaining trigger, and
      an error variant no input can produce is an unreachable branch a reader
      takes as evidence the failure exists
- [ ] 3.4 Delete the tests that covered the refusal, the redraw and the reserve —
      `a_refused_pair_redraws_every_slot_from_the_reserve`,
      `exhausting_the_reserve_fails_rather_than_reading_on`,
      `an_unrefused_draw_never_consults_the_reserve`,
      `the_redraw_path_is_pinned_to_a_written_down_name`,
      `a_refused_pair_leaves_both_of_its_words_drawing_freely`,
      `the_denylist_is_sorted_deduplicated_and_in_range`
- [ ] 3.5 `NAME_DIGEST_BOUND` becomes **6**, and it must bound something rather
      than be asserted against its own literal — the readability finding on the
      tautological `debug_assert_eq!` applies to the new constant too

## 4. The pins

- [x] 4.1 Three written-down names, produced by `tmp/pin.rs`, which reads the
      wordlists from the **text files** and does the index arithmetic itself rather
      than calling `name_from_digest`. `the_name_scheme_is_pinned_to_known_answers`
      pins the common path and the digest separately;
      `the_pinned_name_is_derivable_by_hand_from_the_pinned_digest` re-derives the
      indices in the test and asserts them as literals.
      **Proved by mutation**: renaming one noun entry (`karpos`) failed both pins
      and the character sweep — 3 failures, 29 passes
- [ ] 4.2 **Re-pin after the denylist comes out.** Any pinned case whose name was
      produced by a redraw now derives from its first draw instead, so the
      written-down name changes. Re-derive with `tmp/pin.rs` rather than by
      running the implementation and copying what it prints, which is the whole
      point of the pin. The spec now also requires pinned cases to span the lists
      — a low and a high index in each slot — rather than clustering

## 5. The feed row — REVERSED by the owner ruling

**The name does not go over the wire, on any reply.** Tasks 5.1 to 5.3 put a
`displayName` on the feed row; the spec no longer has a requirement they satisfy,
and the requirement they were written against is deleted. The row goes back to
carrying the address and nothing added. What the feed *should* carry instead —
the public key, so a holder can derive — is filed as its own issue rather than
done here, because it changes the wire contract's `author` field and several
merged specs.

- [x] ~~5.1 `FeedRow::display_name` is filled from `entry.op.op.author`~~ — the
      **false doc comment correction stays**; the field does not
- [x] ~~5.2 `displayName` is in `feed_page_json` and in the key-set test~~
- [x] ~~5.3 `a_rows_name_follows_the_key_that_signed_not_the_rows_position`~~
- [ ] 5.4 Remove `FeedRow::display_name`, its `feed_page_json` field and its
      key-set entry. Keep the corrected doc comment: the false claim it replaced
      ("the name is a pure function of this address") is what this change exists to
      correct, and it is false under the ruling too
- [ ] 5.5 Delete 5.1–5.3's tests, and the `feed.rs:293` drop-the-row path with
      them — the failure it handled cannot arise once no row derives a name. This
      closes the security, correctness and design findings against the placeholder
      and the silent drop by removing what they were about
- [ ] 5.6 Assert **positively** that no feed row carries a display name, so the
      absence is pinned rather than merely current — the same shape as
      `the_wire_reports_the_author_as_an_address_and_a_key_and_no_name` already
      uses for the thread item

## 5b. The glosses, and deriving a name from a key

- [ ] 5b.1 Every noun and every place carries a gloss. The wordlist shape changes
      from `&[&str]` to a pair; **the adjective list does not change** and carries
      no glosses. ~2,048 glosses, sourced rather than recalled — the attestation
      argument applies to a gloss exactly as it does to a word, and a fabricated
      gloss is likewise invisible to a reviewer and uncatchable by a test
- [ ] 5b.2 A gloss lookup by word, answering for any entry of either Greek list
      and **refusing** anything else — including an adjective, which is the case
      that distinguishes a real check from one that returns empty for a miss
- [ ] 5b.3 Glosses are ASCII, asserted over every entry, for the same bidi reason
      the word lists are
- [ ] 5b.4 No reply gains a gloss field. This is satisfied by construction once
      5.4 lands — with no name on any reply there is nothing for a gloss to ride
      beside — so assert it rather than arrange it
- [ ] 5b.5 **A way to derive a name from a public key, reachable by a caller.**
      Under the ruling this is the **only** way a name is obtained, so it is the
      load-bearing surface of the change rather than a convenience beside a
      returned name. The QML sandbox holds none of the wordlists; without this
      call a second implementation of a consensus-critical derivation gets written
      in QML, which is the silent divergence the pins exist to prevent

## 6. The identicon window

- [x] 6.1 Move `Identicon.qml`'s eight reads from `12..19` to `4..11` and correct the
      false "bytes 0..11 are reserved" comment; verify `tst_identicon.qml`'s pinned
      selectors move with it — `e62ddde`. The ink-distinctness sweep moved too: it
      varied bytes 13/14/15, which the mark no longer reads, so it would have swept
      a constant and passed while proving nothing.
- [x] 6.2 Update `docs/IDENTICON.md`'s byte-layout section and its "two flaws" note;
      verify by grep that no `12..19` claim about the mark survives — `e62ddde`.
      The surviving mentions are historical and read as history.

## 7. Documents

- [x] 7.1 Update `docs/PLAN.md`'s overlap paragraph and `docs/UI-BRIEF.md`'s stale
      four-word sentence; verify by grep that no four-word claim survives —
      `4a42336`. PLAN.md's correction of the old "reserved for the identicon"
      draft keeps its quoted window deliberately: the point it makes is that
      *which* eight bytes the mark reads has never mattered, so editing the
      window out of it would have orphaned the argument.
