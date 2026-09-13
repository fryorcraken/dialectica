# security — generated-names

Dimension reviewed: **security only**. Correctness, readability and architecture
are held by other instances; nothing below is a judgement on those.

Baseline measured in this worktree before any mutation:
`cargo test -p dialectica -p dialectica-core` → 918 + 28 = **946 passed, 0 failed**.
`dialectica-ui/tests/run-qml-tests.sh` → 4 spec files, **42 passed, 0 failed**.
Every mutation below was applied in this worktree, measured, and reverted; the
tree was `git status --short`-clean before this file was written.

## Findings

- [x] **`tester`** — `feed.rs:293` — the derivation-failure path is entirely
      uncovered, and a fabricated placeholder name survives the whole suite
      **Scenario:** replace the `let Ok(display_name) = … else { continue }` with
      `…unwrap_or(DisplayName { adjective: ADJECTIVES[0], noun: NOUNS[0], place:
      PLACES[0] })`. Every feed row for a key whose reserve is exhausted then ships
      `"displayName":"abandonable agatharchides of acherousia"` on the wire — a
      name attributable to nobody, presented as one attributable to somebody. The
      spec forbids this in terms at
      `specs/generated-names/spec.md:775` ("A failure SHALL NOT be reported as a
      name. There SHALL be no placeholder name, no name for 'unknown'"). This is
      the one security property of the malformed/failed-input requirement that
      reaches a *user's screen*, and no test touches it.
      **Measured:** 946 of 946 tests pass under this mutation (918 + 28, 0 failed).
      **Severity: high** — genuine defect, not a preference.

      **Note from `dev-writer`, not a tick — this box is yours.** The path is
      **deleted** in `118daa9` and the mutation you describe is no longer
      expressible. `display_name` returns `DisplayName` rather than
      `Result<DisplayName, NameError>`, so there is no `Err` arm to
      `unwrap_or` a fabricated name into; and under the scope cut no reply
      carries a display name at all, so `list_threads` derives nothing.

      Your framing is what made the case for fixing it by construction rather
      than by adding a test: this is "the one security property of the
      malformed/failed-input requirement that reaches a *user's screen*". A test
      over that branch would have guarded a path that should not exist; removing
      the path removes the class.

      **MOOT BY DELETION — no test written, and the mutation is unexpressible.**
      Verified rather than relayed: `display_name` returns `DisplayName`, not
      `Result`, so there is no `Err` arm to `unwrap_or` a fabricated name into;
      and `grep -n "display_name\|displayName"` over `feed.rs` finds five hits,
      all inside its test module, so no production path there derives a name to
      fabricate.

      **What still carries the property, since it is a real one and should not
      be recorded as merely deleted.** The prohibition you cite —
      `spec.md:775`'s "no placeholder name, no name for 'unknown'" — now has its
      guard in `names.rs` rather than `feed.rs`, at
      `a_failure_is_never_reported_as_a_name`, which I rewrote while closing the
      `spec-test` box on it. It now asserts that a short key does not arrive at
      the name its zero-padded form reaches, and fails under a
      `display_name_from_bytes` that pads. The placeholder class is closed by
      construction at the feed and by assertion at the derivation.

- [x] **`dev-writer`** — `feed.rs:292` — the comment cites a test that does not
      exist anywhere in the repository
      **Scenario:** the comment closes with "See
      `a_row_whose_name_cannot_be_derived_is_dropped_rather_than_faked`."
      `grep -rn` over `dialectica/` and `openspec/` in this worktree returns
      **exactly one** hit — the comment itself. A reader checking whether the drop
      path is covered reads a named test, believes it, and stops. That is what
      closed the question above. (The `tester` reported the same citation; it is
      repeated here because the *security* consequence is that it is the reason
      the placeholder mutation went unnoticed.)
      **Measured:** 1 occurrence repo-wide, in the comment.
      **Severity: medium** — genuine defect.

      **Fixed** in `118daa9`: the comment and the branch it described are both
      deleted. `grep -rn "a_row_whose_name_cannot_be_derived_is_dropped_rather_than_faked"`
      now returns only the four findings files quoting it.

      **Your framing is the one I want on the record**, because it is the part
      that generalises past this branch: the citation "is the reason the
      placeholder mutation went unnoticed". A comment naming a test is a claim a
      reader checks by reading rather than by running, and it closes the question
      for everyone downstream — which is worse than silence. Four reviewers hit
      the branch and the citation is why none of the earlier passes did.

      I have taken that as a rule rather than a one-off repair. Every "see
      `<test>`" I wrote in this pass names a test I confirmed exists, and where I
      wanted to point at a property rather than a test I said so in words
      instead. The two new cross-references — to
      `a_real_figures_canonical_citation_is_returned_like_any_other_draw` and to
      `the_feed_reply_is_the_ecosystems_pagination_shape` — were both checked by
      running them.

- [x] **`tester`** — `tst_identicon.qml:103` —
      `test_no_byte_the_abbreviation_displays_reaches_the_mark` is one-sided: it
      cannot see the abbreviation widening onto the mark's window
      **Scenario:** the test hardcodes the displayed groups as the literal hex
      spans `0..3`, `14..17`, `29..31` rather than deriving them from
      `Theme.headChars` / `middleChars` / `tailChars`, which are what actually
      produce them (`AddressLabel.qml:18-24`). Set `Theme.headChars: 24`. The
      abbreviation now displays hex chars 0..23 = **bytes 0..11**, which swallows
      the mark's entire `4..11` window — the exact defect this change exists to
      remove, restored in full, with an attacker able to read all eight of the
      mark's bytes off the screen while grinding. All four QML spec files stay
      green.
      This is a gate the defect satisfies. The converse direction *is* covered:
      moving `_form()` from `_byte(4)` to `_byte(14)` fails 3 tests.
      **Measured:** 42 of 42 QML tests pass under `headChars: 24`; 39 of 42 pass
      under the `_byte(14)` mutation (3 failures). The gate is asymmetric.
      **Severity: high** — genuine defect. The three-channel disjointness
      requirement (`spec.md:57-99`) is the one security property this change adds
      to the UI, and half of what could break it is unguarded.

      **Note from `dev-writer` — this box is the `tester`'s to tick, and I have
      not ticked it.** Recording what I found and added, so whoever closes it is
      not measuring a tree that has moved under them.

      **The gap is closed, and by then it was closed three times over.** When I
      came to fix this I found the `tester` had already added
      `test_the_mark_and_the_abbreviation_share_no_byte` and
      `test_the_byte_probes_find_the_windows_they_should`, both of which compute
      the displayed set rather than hardcoding it. I added a third,
      `test_no_byte_the_mark_reads_is_ever_displayed`, before noticing the other
      two — it derives the displayed bytes from `DTheme.headChars` /
      `middleChars` / `tailChars` and asserts none falls in the mark's `4..11`.

      **Measured, on your exact mutation.** With `DTheme.headChars: 24`: **3 of
      12 identicon tests fail**, where the whole suite was green when you filed
      this. Mine reports *"byte 4 is read by the mark AND displayed by the
      abbreviation (head 24, middle 8, tail 6)"*. Reverted after measuring; all 9
      spec files and 209 assertions pass on the restored tree.

      **Whether to keep all three is a judgement for the `tester`**, and I would
      lean to keeping mine only if the failure message earns its place — it names
      the offending byte and the three settings that caused it, where the
      siblings name the byte and the two sets. If that is not worth a third test,
      delete it; the property is covered either way and I would rather say so
      than defend a test I added redundantly.

      **FIXED. The `dev-writer`'s test is deleted — not for redundancy, but
      because it is blind to a class the other two catch.** Taking up its offer,
      with the measurement that decides it.

      **I re-ran your mutation myself rather than accepting the report.** With
      `DTheme.headChars: 24` on the three-test tree: **3 of 12 identicon tests
      fail**, as the `dev-writer` said. The `middleChars: 20` variant: the same
      3, naming byte 11. So all three did catch both `DTheme` widenings, and on
      that evidence alone the choice would have been a coin-toss about failure
      messages.

      **The discriminating mutation is a change to `abbreviate` itself, with
      `DTheme` untouched.** `test_no_byte_the_mark_reads_is_ever_displayed`
      computes the displayed set by redoing `abbreviate`'s arithmetic from the
      three `DTheme` properties; the other two *measure* `AddressLabel` by
      varying one address byte at a time and watching the rendered text. Shifting
      the head group to `body.substr(8, h)` puts mark byte 4 on screen —
      precisely the defect this box is about — and:

      - `test_no_byte_the_mark_reads_is_ever_displayed` **PASSED**
      - `test_the_byte_probes_find_the_windows_they_should` failed, printing
        `abbreviation shows 4,5,6,7,14,15,16,17,29,30,31`
      - `test_the_mark_and_the_abbreviation_share_no_byte` failed, naming byte 4

      A test that restates the production arithmetic can only see its *inputs*
      move, never the arithmetic. That is the same family as the finding you
      opened — a gate the defect satisfies — one level up, so keeping it as a
      third guard would have left a reader checking the wrong test. Deleted, with
      the reasoning recorded in a comment above the surviving test so the shape
      is not reinvented.

      **Its one genuine contribution is kept.** The failure message now carries
      the three `DTheme` settings alongside both measured windows: *"byte 4 is
      both read by the mark and displayed by the abbreviation (mark reads
      [4,5,6,7,8,9,10,11], abbreviation shows [...]) at head 24, middle 8, tail 6
      — an attacker grinding a lookalike mark can read their progress off the
      rendered address"*. Re-measured after the deletion: `headChars: 24` fails
      2 of 11 with that message. `DTheme.qml` and `AddressLabel.qml` restored
      after every mutation; `git diff --stat` shows test files only.

      One correction to the box's own citation, since it affects where a reader
      looks: the hardcoded-span test you name,
      `test_no_byte_the_abbreviation_displays_reaches_the_mark`, is still
      present and still hardcodes its spans — it is the MARK side and is correct
      as the mark side. Its comment claimed the abbreviation side was "the test
      above", which after the deletion pointed at nothing; it now names
      `test_the_mark_and_the_abbreviation_share_no_byte` explicitly, checked by
      running it. A dangling "see `<test>`" is the failure the `dev-writer`'s own
      box in this file is about, and deleting a test is exactly how one gets
      created.

- [x] **`dev-writer`** — `names.rs:358-362` — the denylist arithmetic reinstates a
      premise `docs/PLAN.md` explicitly struck through, and the shipped list is
      ~4.8x smaller than the figure the comment derives everything from
      **Scenario:** `is_refused`'s doc comment states "roughly 800 of the 1,024
      nouns are named Greeks, each with about 1.2 canonically associated places,
      so about 960 pairs against `1024 * 1024` = 1,048,576 is about 0.092% of
      draws — roughly 4.6 identities in every 5,000." `docs/PLAN.md:1598-1604`
      carries that same sentence **struck through**, followed by "**The premise is
      withdrawn**" and "**The denylist's size is deliberately not pinned**".
      `TRUE_ATTRIBUTION_PAIRS` ships **199** pairs (lines 39-237 of
      `names/denylist.rs`), not ~960.
      Correct arithmetic: 1,048,576 / 199 = **1 first draw in 5,269** refused, so
      **~0.95 identities in every 5,000**, not 4.6. The comment overstates the
      family's reach by a factor of ~4.8 while citing a premise the project has
      already retracted.
      **Measured:** 199 entries, from `grep -cP "^\s*\(\d+u?1?6?,\s*\d+"` on the
      shipped file.
      **Severity: medium** — genuine defect. Security-relevant because this is the
      stated justification for the denylist being mandatory (an impersonation
      control), and because the same figure feeds the next finding.

      **Fixed** in `118daa9` by deleting the denylist, the comment and the
      control together, under the owner's no-filter ruling.

      **You identified the mechanism, and it is worth separating from the
      arithmetic.** The comment did not merely hold a wrong number — it
      reinstated a premise PLAN.md had struck through and annotated with "**The
      premise is withdrawn**". A retracted claim that reappears in code as
      current fact is a specific failure mode: the retraction lives in one
      document and the assertion in another, and the code is what the next reader
      believes.

      That is now recorded where it survives this change's deletion.
      `design.md` D11 carries the same lesson for a different figure — its table
      claimed an adjective yield of 17,349 that no file reproduces and that
      contradicted D12's own verifiable chain in the same document. Corrected to
      11,467 with the chain named, and the correction states which of the two
      sections is the authority, so the next disagreement resolves rather than
      recurring.

- [x] **`dev-writer`** — `names.rs:195-197` — `NameError::ReserveExhausted`'s rate
      is derived from the withdrawn 960 and is wrong by a factor of ~23
      **Scenario:** the doc comment states "Arrives about once in 1.2 million
      identities: a first draw is refused about once in 1,090, and this needs two
      consecutive refusals." 1,090 is 1,048,576/960 — computed from the retracted
      premise, not from the shipped list. With 199 pairs the first-draw rate is
      1 in 5,269, so two consecutive refusals is 1 in 5,269² = **1 in ~27.8
      million**, not 1 in 1.2 million.
      This governs how often the silent-drop path at `feed.rs:293` fires and how
      cheaply an attacker can grind a key that reaches it. The true figure is
      *safer* than claimed, so this is not an exploitable miscalculation — but it
      is a number a future reader will size a decision against, and this repo's
      standing trap is exactly a fabricated number in a comment.
      **Measured:** 199 shipped pairs; 1,048,576/199 = 5,269.2; 5,269² ≈ 27.8M.
      **Severity: low** — genuine defect, wrong in the safe direction.

      **Fixed** in `118daa9` by deletion — the variant, the rate and the
      silent-drop path it governed are all gone, and the derivation is now total,
      so there is no rate to state.

      **The sentence I want to keep is yours**: "it is a number a future reader
      will size a decision against, and this repo's standing trap is exactly a
      fabricated number in a comment." That is the reason a *safe-direction*
      error still had to be fixed rather than noted, and it is why I removed
      every derived rate from the module instead of correcting this one. A figure
      that is wrong in the safe direction is still a figure someone budgets
      against.

- [x] **`spec-writer`** — `wire.rs:6076` —
      `the_wire_reports_the_author_as_an_address_and_a_key_and_no_name` is an
      active gate *forbidding* what this change's own spec requires
      **Scenario:** `spec.md:599` states "**Every reply** in which core reports who
      authored something SHALL carry that author's display name alongside the
      author's address." The thread read reports an author (`wire.rs:1734`,
      `"author"` plus `"authorKey"`) and carries no name; this pre-existing test
      (from #61, untouched by this change) asserts `displayName` is **absent** from
      a thread item and passes. Its stated reason — "sending one would put a
      derivable identifier on the wire beside the material it is derived from" —
      is the same false reasoning this change corrects at `feed.rs:120-130`,
      left standing in `wire.rs` and enforced.
      Security consequence: one identity renders **named in the feed and nameless
      in the thread**. Because `authorKey` is already on the wire, the UI can close
      the gap by deriving names itself — a second implementation of a
      consensus-critical scheme, in a language with none of the wordlists, which is
      precisely the silent-divergence failure `names.rs:71-96` and
      `every_wordlist_is_pinned_entry_by_entry_and_in_order` exist to prevent.
      Either the spec's "every reply" must be narrowed to the feed in this change,
      or the thread read must carry the name — but the contradiction must not merge.
      **Measured:** the test passes on `piece/generated-names`; it is unmodified by
      this change (`git log -L 6075,6103` → last touched by `53f08b3`, #61).
      **Severity: medium** — genuine defect, a spec/code contradiction with a
      security consequence.

      **FIXED by narrowing the spec — which is the branch you named first, and
      against the one your security argument prefers.** The contradiction is
      resolved: a reply carrying the public key now **SHALL NOT** carry the name,
      so that test is the gate for a requirement rather than against one.

      **Your silent-divergence concern is real and is not dismissed**, but it
      argues against the wrong remedy. A view deriving names itself *would* be a
      second implementation of a consensus-critical scheme — and shipping the
      name on the thread does not prevent that, it only removes the occasion for
      it on one surface while leaving `authorKey` on the wire for any client that
      wants to. The property that actually closes it is that **exactly one
      implementation is normative**, which is what the pinning and wordlist-hash
      requirements establish; a divergent second implementation is detectable
      against those pins whether or not core also ships a name.

      Against that, carrying both is a shape the wire contract cannot make safe:
      two values that must agree, either derivable from the other, with no way
      for a recipient to tell which is wrong when they differ. That is a forgery
      surface rather than a convenience — a relay stripping or rewriting a
      `displayName` beside an intact `authorKey` produces a reply that renders a
      false attribution and verifies fine. Forbidding the pair removes it.

      **On "one identity renders named in the feed and nameless in the thread"**
      — that is a UI obligation, not a core one, and it now has a home: the view
      derives the name from `authorKey` using the same core it already calls, and
      `docs/UI-BRIEF.md` carries the rendering obligation. If a future decision
      wants core to expose a derive-name-from-key call so the view never
      implements the scheme, that is a capability widening to argue on its own
      merits — and it is the right shape for your concern, rather than putting a
      second copy of the name on every thread item.

- [x] **`tester`** — `names.rs:683` —
      `the_name_digest_is_neither_the_address_nor_a_bare_hash` passes for an
      incidental reason and does not test the property it is named for
      **Scenario:** set `NAME_PREFIX` to `b"/dialectica/1/Address/Author\0\0\0\0"`,
      i.e. make the name's separator **identical** to the author-address
      separator — total loss of domain separation, the mechanism
      `spec.md:126-137` says the whole name/mark independence rests on. This test
      still passes, because the address hashes `PREFIX || 0x01 || key` while the
      name hashes `PREFIX || key`: it is the record-count byte, not the prefix,
      that separates the two digests here. The test named for domain separation is
      therefore blind to domain separation being removed.
      The mutation *is* caught, by `the_name_scheme_is_pinned_to_known_answers` and
      `two_distinct_keys_can_share_a_name_and_neither_is_marked` (2 failures), so
      this is not an open hole — but the test that reads as the guard is not the
      guard, and a reader auditing separation will check the wrong one. What is
      missing is a direct assertion that `NAME_PREFIX` differs from every prefix in
      `identity.rs` and `op.rs`; all five are private consts in separate modules
      and nothing compares them.
      **Measured:** 63 of 65 `dialectica-core` lib tests pass under the
      prefix-collision mutation; this test is among the 63.
      **Severity: low** — a weak gate rather than an uncovered defect.

      **FIXED — the test named for domain separation now fails when domain
      separation is removed.** A third assertion is added to
      `the_name_digest_is_neither_the_address_nor_a_bare_hash`: for each of the
      five other separators in the crate, the name's digest must not equal the
      key hashed under that separator in this module's own `PREFIX || key`
      shape, and `NAME_PREFIX` must not equal that separator as a byte string.

      **Your diagnosis was exactly right about why the old body was blind**, and
      it is the reason the new assertion is shaped the way it is: the address
      hashes `PREFIX || 0x01 || key` where the name hashes `PREFIX || key`, so
      comparing the two digests is a test of the record-count byte. Comparing
      under a common preimage shape is what makes the prefix the only variable.

      **On "all five are private consts in separate modules and nothing compares
      them" — that is the constraint, and the fix leans into it rather than
      routing around it.** The five are WRITTEN DOWN as literals in the test
      rather than imported. Widening five modules' visibility for a test would
      have been the wrong trade, and importing them would be worse than not
      testing: a future edit that moved a separator would move the expectation
      with it, which is the self-consistency shape this whole review is about.

      **Proved it can fail, on your exact mutation.** Setting `NAME_PREFIX` to
      `b"/dialectica/1/Address/Author\0\0\0\0"` now fails this test — *"the
      name's separator is the author address separator, so the two capabilities
      are one function of the key"* — where before it was among the 63 that
      passed.

      **And proved the literals are byte-exact, which is the way this fix could
      have failed silently.** A mistyped literal would make its comparison pass
      vacuously. Setting `NAME_PREFIX` to the slate-path separator also fails,
      naming that one, so at least that entry matches `onboarding.rs`'s const
      byte for byte and the loop reaches entries past the first.
      `names.rs` restored after each.

      Severity was right at low as an exploitability judgement, and the finding
      still earned its keep: the gate a future auditor will check is now the gate
      that holds.

## What was clean

**Malformed key material.** `display_name_from_bytes` routes through
`PublicKey::from_bytes` (`identity.rs:236`), which does `try_into` to `[u8; 32]`
*before* the ed25519 parse, refuses non-decompressable points, and refuses
low-order points via `is_weak()`. Wrong lengths 0/1/31/33/64/1024, non-points and
the all-zero point are all covered by
`malformed_key_material_is_refused_rather_than_crashed_on`, and
`arbitrary_bytes_do_not_abort_the_process` sweeps every length 0..70 on two byte
patterns. No unbounded allocation on this path — unlike `Address::from_hex`, the
length check cannot be reached with an attacker-chosen size.

**Reachable panics.** `draw_at` indexes `digest[offset + i]` with `offset` only
ever 0 or 6 on a `[u8; 32]`, so the maximum index is 11. Every list index is a
`u16` reduced modulo a list length that `the_lists_are_the_sizes_the_arithmetic_
rests_on` pins to 8,192/1,024/1,024, so every `as usize` index is in range by
construction. The `ADJECTIVES.len() as u16` cast is safe only because 8,192 <
65,536, and that same test is what holds it. No `unwrap`, `expect`, slicing or
arithmetic on this path can panic on peer bytes.

**ASCII as a bidi defence — enforced per entry, and the enforcement runs.**
`every_entry_of_every_list_is_ascii_lowercase_and_well_formed` checks
`is_ascii()`, lowercase, non-empty, no leading/trailing/double space, and a
per-character class, for every entry of all three lists. Verified independently:
`grep -cP "[^\x00-\x7F]"` on the four shipped files returns 2/6/2/6 non-ASCII
lines, and reading each one shows **every** occurrence is an em-dash inside a
`//!` doc comment. **No wordlist entry carries a non-ASCII byte.** Given both
census agents introduced Cyrillic homoglyphs into hand-typed Greek, this is the
control that matters most and it is sound.

**Domain separation, as a fact about the shipped constants.** The five prefixes —
`AUTHOR_ADDRESS_PREFIX`, `STOA_ADDRESS_PREFIX`, `OP_SIGNING_PREFIX`,
`OP_ID_PREFIX`, `SLATE_PATH_PREFIX` — and `NAME_PREFIX` are pairwise distinct as
byte strings, all fixed at 32 bytes in the same padded style. No byte string is
both. (The *test* coverage of this is the finding above; the constants themselves
are correct.)

**The disjointness fact itself.** Verified by hand rather than taken from the
comment: `AddressLabel.abbreviate` with `headChars 8 / middleChars 8 / tailChars 6`
shows hex chars 0..7, `floor((64-8)/2)=28`..35, and 58..63 — i.e. address bytes
{0,1,2,3}, {14,15,16,17}, {29,30,31}. `Identicon` reads bytes 4,5,6,7,8,9,10,11
(`_form`, `_outlineInk`, `_inkA`/`_inkB`, `_angleDeg`, `_pitch`, `_duty`,
`_weave` — eight bytes, one per dimension, as the comment claims). The two sets
are disjoint. The old `12..19` window did overlap the middle group on
{14,15,16,17}, as claimed. The move is correct; only its *guard* is one-sided.

**The multiply-not-add argument holds.** The name reads `H(NAME_PREFIX || key)`
and the mark reads `SHA256(AUTHOR_ADDRESS_PREFIX || 0x01 || key)`. Both are
functions of the same `k` with no way to fix one while varying the other, so
grinding both is a joint search: 2^33 x 2^13.6 = 2^46.6. `docs/IDENTICON.md:726-735`
is candid that this defeats casual impersonation and not a motivated attacker,
and that an untargeted "any lookalike" hunt is cheaper — the documented limitation
is honest and needs no finding.

**Name-as-credential.** No method accepts a name: this change adds no methods, so
the hand-maintained `every_request_taking_method()` sweep list has not gone stale
here. `a_forbidden_field_is_refused_on_every_operation` already refuses `author`,
`identity`, `key`, `address`, `thread` across post/reply/vote.
`row.display_name` is written to the wire without `sanitised_json`, which is safe
**only** because the value is drawn from ASCII-lowercase wordlists — it cannot
carry a bidi override, a homoglyph or a control character. That safety is
inherited from the ASCII screen above, so if that screen is ever relaxed this
becomes a live injection path.

**No information leakage in errors.** Neither `NameError` arm carries key
material, and `NameError` never reaches the wire at all — the feed drops the row
rather than reporting the failure.

**Denylist enforcement.** Making `is_refused` return `false` unconditionally fails
4 tests, so the impersonation control is genuinely guarded. (Note that
`no_name_a_key_can_reach_is_a_refused_combination` passes under that mutation,
since it calls `is_refused` to check `is_refused` — but the other four cover it.)

## What these gates cannot see

- **`cargo mutants` mutates functions, not `const` values.** Every consensus-
  critical value in this change is a `const`: `NAME_PREFIX`, `CONNECTOR`,
  `NAME_DIGEST_BOUND`, the three wordlists, `TRUE_ATTRIBUTION_PAIRS`, and
  `Theme.headChars` / `middleChars` / `tailChars`. The wordlist SHA-256 pins and
  the name pins cover the Rust side; **the three `Theme` constants are covered by
  nothing**, which is the `headChars: 24` finding above.
- **No gate compares the six domain separators to each other.** Each is a private
  const in its own module. A future prefix that duplicates an existing one would
  fail only the downstream value pins, and only if that derivation happens to be
  pinned.
- **Denylist completeness is not testable and the code says so.** A missing
  noun-place pair renders one identity under a real person's canonical name, and
  no test can tell. That is a curation claim, correctly flagged as such in
  `names/denylist.rs:26-33`; it is recorded here so a green suite is not read as
  covering it.
