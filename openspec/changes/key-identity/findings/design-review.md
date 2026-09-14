# Design review — `key-identity`

Reviewed `design.md` against the code, and against `docs/PLAN.md` as it stands on
**`origin/main`** (read via `git show origin/main:docs/PLAN.md`, not the branch's
copy).

**The decisions in this change are in good shape, and the record is unusually
honest.** Every decision I went looking for in the code was taken as recorded:
the name reads `key_bytes[name_key_bytes()]` at `18..24` with no hash and no
separator; `NAME_PREFIX` and `name_digest()` are gone; `OP_SIGNING_PREFIX` and
Stoa addresses are untouched; the mark stayed at `4..11`; `DKeyNameWindow.qml`
carries indices and no wordlists; the four falsified-retraction sites all
describe the mechanism that now exists, and `docs/IDENTICON.md` records three
versions of the argument with a stated reason for keeping all three rather than
replacing the passage. The `#80` split boundary is recorded three times over
(`design.md` Non-Goals, the Migration Plan, and a comment at `Identicon.qml`'s
`address` property) and is specific enough that the sweep's author knows what is
theirs.

**The accepted versioning cost IS recorded** — `design.md:88-92`, the spec's
*The scheme and its wordlists are frozen, with no version to bump*, and
`names.rs:24-31`, which states it is a one-way door in those words. Verified
against the owner's ruling; nothing is softened.

**I verified the three re-groundings the dev reported, against the spec text
rather than against the summary.** All three check out; there is no repeat of the
sibling piece's fabricated decision record.

- `names.rs:51-52` cites *"the `generated-names` spec's own requirement that no
  reply carries a display name"*. `openspec/specs/generated-names/spec.md:58-62`
  is `Requirement: The name SHALL NOT travel`, whose first sentence is *"No reply
  SHALL carry a display name: not a feed row, not a thread item, not an
  onboarding slate candidate"*. Verbatim match, and the enumeration the claim
  relies on is in the requirement body, not invented.
- `docs/IDENTICON.md:15` cites the spec for *"a name is never unique, never an
  identifier"*. `spec.md:586` is `Requirement: A name is never unique, never an
  identifier, and never numbered`. Exact.
- `docs/IDENTICON.md:17-19` asserts *"The public key is the identity"* and that
  the key must be on screen. `spec.md:288` (the change's delta, line 288) states
  *"**The public key is the identity.**"* in bold, and the surrounding paragraph
  carries the "not one click away" obligation. Exact.

No wordlist-exclusion table was invented, and nothing cites a PLAN.md section
about credential expiry. The only wordlist screen these documents lean on is the
compound-noun one.

**On `// NO SPEC:` markers: I agree with the author that none were required in
the Rust code, but one QML choice qualifies** — see the third box below. It is a
gap in the record, not a defect in the code.

Four boxes. One is a PLAN.md contradiction this change introduced, one is a
dangling pointer this change wrote into PLAN.md, and two are decisions taken
without being recorded.

- [ ] **`dev-writer`** — `docs/PLAN.md:1061` still says *"The address — the only
      thing that settles identity"* — the rule this change inverted, applied at
      three sites and missed at a fourth
      The change rewrote §5.2.1's *Rendering obligations* bullets (PLAN.md:1065-1078
      on `origin/main`) to strike *"The address is the identity"* and replace it
      with *"**The public key is the identity**"*, and struck *"the address must
      be present"* to *"the public key must be present"*. Both corrections are
      right. But the **four-layer list nine lines above them** was left alone, and
      layer 4 still reads *"**The address — the only thing that settles
      identity.** Unforgeable, unmintable, and what every signature binds to"*,
      with the closing sentence at `PLAN.md:1066` still reading *"An interface
      showing a name and a glyph and no address has shipped three recognition
      aids and zero guarantees."*
      **Scenario:** a reader arrives at §5.2.1 to settle "what is an author
      identified by?". Layer 4 says the address; the bullet list twelve lines
      later says the public key and adds *"a requirement still pointing at it
      would point at a value no longer carried"*. The section now answers its own
      question both ways, in the same subsection, and the wrong half is the one
      written in the emphatic voice a reader trusts.
      **Measured:** `grep -n "The address — the only thing that settles identity"
      docs/PLAN.md` returns line 1061 on the branch, unchanged from `origin/main`.
      The same grep for the corrected bullet returns the struck-through form. This
      is the *partially applied decision* shape — the guard got right at three
      call sites and missed at a fourth — and it is worse here than a stale line,
      because the two halves sit inside one section and contradict each other.
      Also check layer 2's *"the costs multiply rather than add"*: that conclusion
      survives, but its stated reason (independent channels) now rests on byte
      disjointness rather than on separate digests, and §5.2.1 does not say so.

- [ ] **`dev-writer`** — `docs/PLAN.md:986-987` points at a `design.md` entry that
      does not exist
      The paragraph this change added to PLAN.md ends: *"the change's `design.md`
      carries why the alternative (a per-channel hash under its own separator)
      was not taken."* It does not. `design.md` records two alternatives for how
      the window is *expressed* (a start-plus-length pair, and slicing at the call
      site) and two for the QML probe (`Core`, and a cross-language script). The
      per-channel-hash alternative — keep a hash between the key and each channel,
      under a separator per channel, and get independence from domain separation
      as before — appears nowhere in it.
      **Scenario:** the next person asks why #80 did not simply give each channel
      its own separator, which would have preserved the versioning seam the change
      accepts losing. PLAN.md tells them design.md answers it. They open design.md,
      find nothing, and re-litigate a decision the owner already ruled on — which
      is exactly the outcome an "alternatives considered" entry exists to prevent.
      **Measured:** `grep -n "per-channel\|separator\|alternative" design.md`
      returns six lines, none of them about a per-channel hash.
      Either write the entry (what was chosen: no hash anywhere; the constraint:
      the owner's ruling on #80; what ruled the alternative out; what it costs —
      the versioning seam, already recorded) or correct the PLAN.md sentence to
      point at the owner's ruling instead. Writing the entry is better: the cost
      is already recorded as accepted, and an accepted cost with no visible
      rejected alternative reads as though there was no choice.

- [ ] **`dev-writer`** — `design.md` — the byte allocation itself is not a
      recorded decision, and it is the decision this piece exists to make
      `design.md`'s first Decisions entry is titled *"The name reads
      `key.to_bytes()[18..23]`, and the type carries the window"*, but every word
      under it is about **how the window is expressed** — one `Range` versus a
      start and a length, a `const fn` versus a `const`, slicing here versus at
      the call site. Why the window is `18..23` is never argued. Two real
      alternatives went unrecorded:
      **(a) Why contiguous rather than split across the gaps.** The allocation
      leaves seven bytes unallocated in two runs, `12..13` and `24..28`. Six of
      those seven would have held the name, placing it entirely in what the
      abbreviation hides and leaving a contiguous run free. Splitting a slot's
      reads across non-adjacent bytes is a real option with a real cost — the
      derivation stops being one slice and `pin_name.rs`'s `NAME_FIRST_BYTE + 2`
      arithmetic stops working — and nothing says so.
      **(b) Why seven bytes are unallocated rather than reserved.** This appears
      only as a one-line Risks bullet (*"They are unallocated, not reserved"*),
      which states the conclusion and the wording rule but not the decision. The
      spec's own warning is that a range read by nothing gets taken as
      load-bearing by the next reader; the code is careful about this in four
      places (`names.rs:100-102`, `names.rs:905-917`, `Identicon.qml`,
      `IDENTICON.md`), so the care was deliberate — but `design.md` is where a
      reader learns it was a choice rather than an accident of arithmetic.
      **Scenario:** a future piece wants four bytes for a fourth channel. It finds
      seven free bytes and no recorded reason they are free, and either takes them
      (fine, if the spec changes with it) or refuses to (believing they are
      reserved for something). Both are guesses the record should have settled.
      **What is missing, against the four-part test:** what was chosen is present;
      the constraint is present; **the alternatives and what ruled each out are
      absent**; what it costs (it forecloses extending any channel without a spec
      change) is present but as prose in the Risks section rather than attached to
      the decision.

- [ ] **`dev-writer`** — `DKeyNameWindow.qml:50-53` pads a malformed key and
      derives from it, where core refuses — an unrecorded divergence, and the one
      place a `// NO SPEC:` marker was owed
      The spec is explicit that *"malformed key material is the only failure this
      capability has"* and that the derivation refuses rather than crashes; Rust
      implements exactly that (`NameError::NotAValidPublicKey`). `DKeyNameWindow`
      does the opposite: it strips any `[a-z]+:` prefix, drops every non-hex
      character, right-pads with zeroes to 64 characters, and returns in-range
      indices for `""`, `"k:"`, `"stoa:zzzz"` and `"not-a-key"` —
      `test_a_malformed_key_still_yields_indices_in_range` pins this.
      The choice is defensible and the component's comment defends it (an index of
      `NaN` would render nothing, and the component inherits `Identicon`'s
      existing obligation). I am **not** asking for the behaviour to change. What
      is missing is that the spec is silent on this component specifically — it is
      a probe target the spec does not describe — so a reasonable default was
      chosen, pinned by a test, and is now permanent by accident, which is the
      precise thing the `// NO SPEC:` convention exists to stop. The repo's own
      rule (`.claude/agents/dev-writer.md:49-54`) is that unspecified behaviour
      the dev chooses carries the marker.
      **Scenario:** a later change makes `DKeyNameWindow` reachable from anything
      other than the test — or a reader compares the two languages and concludes
      core's refusal is the bug, since the QML side accepts. Nothing in the tree
      records that the divergence was noticed and accepted.
      **Measured:** `grep -rn "NO SPEC" dialectica-ui/ dialectica/` returns 30
      markers across the tree and **none** in any file this change touches.
      Either add `// NO SPEC:` at the padding in `DKeyNameWindow.qml` naming the
      choice (pad-and-derive rather than refuse, because the component exists only
      to be probed and a refusal has no representation in its return type), or
      record it in `design.md` as a decision with the alternative (return `-1`, or
      expose a validity flag the probe checks) and what ruled it out. The marker is
      cheaper and is where the next reader will be standing.

## Checked and clean, recorded so it is not re-checked

- **`OP_SIGNING_PREFIX`** — untouched. `grep -rn "OP_SIGNING_PREFIX"` shows no hit
  in the diff, and the spec's `REMOVED` block for the domain-separation
  requirement states in its **Migration** that it *"is unaffected and remains
  required"* and says why (signature replay across preimages). Correctly not
  swept.
- **Stoa addresses** — untouched, and actively defended in three places: the
  spec's never-unique requirement (*"**Stoa addresses are untouched**"*),
  `AddressLabel.qml`'s closing paragraph, and `names.rs`'s
  `a_stoa_address_beside_this_key` fixture, which exists because the test it
  serves was rewritten away from the deleted author address rather than left to
  pass about nothing.
- **The two-language duplication** — recorded at `design.md:109-144` with the
  reasoning, the rejected alternatives (`Core`; a cross-language script), and why
  indices-only is the right cut (the wordlists are 10,240 consensus-critical
  entries; an index moves iff a byte it reads moves; CLAUDE.md's core/UI split).
  The cross-language pin that keeps the duplication honest is recorded with its
  measurement. This is the best entry in the document.
- **The cross-language pin's own figures** — hand-verified rather than trusted.
  For key `ea4a…d22c`, bytes 18-19 are `76 ae`: `0x76ae = 30382`, and
  `30382 − 3×8192 = 5806`, which is the pinned adjective index. Bytes 20-21 are
  `be be`: `0xbebe = 48830`, `48830 − 47×1024 = 702`, the pinned noun index — and
  the two bytes being equal is exactly the endian-blind coincidence the tester
  reports finding, so that finding is real and its description is accurate.
  Bytes 22-23 are `7b 92`: `0x7b92 = 31634`, `31634 − 30×1024 = 914`, the pinned
  place index.

## Two notes for `code-reviewer`, not blocking this pass

Neither is a decision-record problem, so neither gets a box here — but both are
places where `design.md`'s prose and the code have drifted apart, and a design
reviewer is where that usually surfaces first.

- `design.md:52-59` describes the window as *"one `Range<usize>` **constant**,
  `NAME_KEY_BYTES`"* and shows `key_bytes[NAME_KEY_BYTES]`. The code has neither:
  it is `const fn name_key_bytes()`, sliced as `key_bytes[name_key_bytes()]`. The
  entry at `design.md:71-79` explains the rename and why the `const` did not
  survive — so the reasoning is recorded and correct — but the entry above it was
  never updated to match, and a reader meets the stale version first.
- `names.rs:347` carries the intra-doc link `[`NAME_KEY_BYTES`]`, which no longer
  resolves to anything. `rustdoc` will report it as a broken link.
