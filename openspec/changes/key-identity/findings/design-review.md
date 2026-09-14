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

- [x] **`dev-writer`** — `docs/PLAN.md:1061` still says *"The address — the only
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

      **Fixed, both halves** — and the second half was the more valuable catch,
      because nothing in the diff pointed at it.

      Layer 4 now reads *"~~**The address**~~ **The public key — the only thing
      that settles identity**"*, in the same strikethrough-plus-authority shape
      the bullets below use rather than a silent rewrite, and carries the same
      three things they do: that #80 deletes the author address so a layer
      pointing at it would point at a value no longer carried, the
      `generated-names` requirement that is the authority, and **Stoa addresses
      are untouched**. The closing sentence follows it: *"a name and a glyph and
      no ~~address~~ public key"*.

      Layer 2 now states that the conclusion survives #80 **but its reason
      changed**, and says which: independence used to come from domain separation
      — two digests, so whatever bytes each read they could not overlap — and now
      rests on the byte allocation being pairwise disjoint, which makes it a
      property to gate rather than one for free. It cites *The three channels read
      pairwise disjoint bytes of the public key*, which I checked exists verbatim
      in the delta (`spec.md:378`) rather than citing from memory.

      No test covers PLAN.md prose. What is verifiable is that your grep
      (`grep -n "The address — the only thing that settles identity"`) now returns
      nothing, and that the subsection no longer answers its own question both
      ways.

- [x] **`dev-writer`** — `docs/PLAN.md:986-987` points at a `design.md` entry that
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

      **Fixed by writing the entry**, for the reason you give — an accepted cost
      with no visible rejected alternative reads as though there was no choice,
      and PLAN.md's sentence is then true rather than needing to be weakened.

      `design.md`'s *`NAME_PREFIX` and `name_digest()` are deleted rather than
      deprecated* now carries **Alternative considered: keep a hash, but one per
      channel, each under its own separator**, with the four parts:
      - *what was chosen:* no hash anywhere, per the owner's ruling on #80.
      - *what the alternative was:* `H(NAME_V1 || key)`, `H(MARK_V1 || key)` —
        which, as you note, is exactly the shape that would have preserved the
        versioning seam, so it is the thing the next reader reaches for once the
        accepted cost is stated.
      - *what ruled it out:* the ruling, plus what the ruling buys — a name
        verifiable by hand (three 16-bit big-endian draws off bytes a holder can
        read on screen, so a second implementation is a page of arithmetic rather
        than a SHA-256 and a separator that must match byte for byte), and
        independence that lives in a table anyone can check and a gate can measure
        rather than in two separator strings a reader cannot eyeball.
      - *what it costs:* the seam, and **only** the seam. That last point is the
        one worth having written down: independence is not among the costs. It
        moved from domain separation to the byte allocation rather than being
        given up, and this change is what turns that allocation into a stated
        requirement with a gate on both sides of it.

      PLAN.md:986-987 is left as written, because it is now true.

- [x] **`dev-writer`** — `design.md` — the byte allocation itself is not a
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

      **Fixed** with a new first Decisions entry, *Why the name's six bytes are
      `18..23`: contiguous, and the gaps left empty* — placed **before** the
      existing entry rather than folded into it, because your diagnosis is right
      that the two are different decisions sharing one title. The old entry keeps
      its title and is now honestly about how the window is *expressed*.

      Both alternatives you named are recorded with what ruled each out:

      **(a) Split across the gaps.** Ruled out because a split window stops being
      one fact: the whole argument in the entry below (one range cannot be
      half-moved) depends on there being one range, and a split needs a list of
      ranges plus a slot-to-range mapping, each a place two implementations can
      disagree about how far to read. Your `NAME_FIRST_BYTE + 2` observation is
      recorded as one of the concrete costs. Added beyond your list: a contiguous
      window is the half a *human* can check — six consecutive hex pairs a holder
      reads off the screen, where bytes 12, 13, 24, 25, 26, 27 cannot be
      hand-verified without a diagram, and hand-verifiability is what the no-hash
      ruling buys. And the gain is speculative, for a fourth channel nobody has
      specified.

      **(b) Unallocated rather than reserved.** Ruled out with the wording named
      as load-bearing: "reserved" claims something is coming and nothing is. I
      grounded this in the repo's own history rather than in the abstract warning
      — `docs/IDENTICON.md:707-711` (*"First version, wrong when written"*)
      records that an earlier version of that very document called bytes 0..11
      "reserved for the generated-name scheme" (the phrase wraps a line, so grep
      it as `"reserved for the"`) and
      derived the two channels' independence from it when no such mechanism
      existed, and that **two documents invented the same false reservation
      independently**. The word is what did that. The cost is attached to the
      decision rather than left in Risks: no channel can be widened without
      changing the spec's table, and a fourth channel must be specified into the
      gaps rather than helping itself to them — which is the intended direction,
      not an oversight.

      The gate is named where the reader is standing:
      `the_name_reads_no_unallocated_byte`, which (as of the spec-test box in the
      sibling file) now measures over every value of every byte rather than seven.

- [x] **`dev-writer`** — `DKeyNameWindow.qml:50-53` pads a malformed key and
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

      **Fixed at the code, where you say the next reader is standing — but as a
      spec citation rather than a `// NO SPEC:` marker, and you should know why
      the answer differs from what you asked for.**

      I drafted exactly the marker you specified. Then the `spec-test` reviewer's
      second box raised the same divergence from the other side and asked the spec
      to *decide* whether the index-only component is exempt, which I did: *The
      three channels read pairwise disjoint bytes of the public key* now requires
      the measurement apparatus to render no name and to accept malformed input,
      with the reason — a measurement that refused would report the channel as
      reading no byte, and "disjoint" is satisfied by a measurement that found
      nothing.

      Once a requirement mandates the behaviour, a `NO SPEC:` marker on it is
      false, and a false marker is worse than a missing one: it tells the next
      reader nobody decided, when someone did. So the block says the same things
      your marker would have, and cites the requirement instead of announcing its
      absence. If you disagree that the spec change was in scope for this piece,
      that is the thing to push back on — not the marker's absence.

      What the comment carries, all of which your marker asked for:
      - the divergence concretely: core **refuses** where this **pads**,
        `NameError::NotAValidPublicKey` against three in-range indices, for the
        same four inputs (`""`, `"k:"`, `"k:0"`, `"not-a-key"`), and an opening
        line saying it is deliberate — so a reader meeting it does not conclude
        core's refusal is the bug, which is the second half of your scenario;
      - which rule governs which, and why: *Malformed key material is refused
        rather than crashed on* forbids a padded derivation because such a name
        "would render as an ordinary participant" — a hazard that exists only
        where a name is **rendered**, so the requirement is scoped by its own
        stated reason rather than narrowed to suit the code;
      - all three alternatives you listed with what ruled each out — `NaN` renders
        nothing and would make the measurement meaningless rather than failing
        loudly, `-1` is a sentinel every caller must remember to check, a validity
        flag is a second thing for a probe to get wrong — and that padding keeps
        the one-job rule (three in-range indices, always);
      - the transition, in bold: **giving this component a rendering consumer
        moves it under the first rule and obliges it to refuse.** That is the
        first half of your scenario, and it is now a spec obligation rather than
        a hope.

      **The test that fails without it**, which is what your box could not have
      had: `test_the_name_window_exposes_no_name_only_draws` asserts the surface
      is three numeric draws and that eleven rendering-shaped members are
      `undefined`. Measured — adding a one-line `function render()` to
      `DKeyNameWindow.qml` fails it, naming `render`, with everything else in the
      file green. Reverted; 19/19. So the divergence is no longer merely recorded
      as accepted: the condition that makes it safe is gated.

      The behaviour is unchanged, as you asked, and
      `test_a_malformed_key_still_yields_indices_in_range` still pins it with the
      reasoning now in its comment.

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
