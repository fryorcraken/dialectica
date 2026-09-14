## Context

See `proposal.md` — *Why*. The short version is that the three recognition
channels stop being independent by construction and start being independent only
because someone allocated bytes.

Two facts about this tree shape everything below.

**The three channels live in two languages.** The name is derived in
`dialectica-core` (Rust); the mark and the abbreviation are derived in QML
(`Identicon.qml`, `AddressLabel.qml`). Nothing in the repository computes all
three, so there is no single place a disjointness check can read all three sets
from. `PostHeader.qml` takes `generatedName` as a **string handed to it by core**
and never derives anything.

**The existing gate measures rather than mirrors.** `tst_identicon.qml`'s
`_markBytes` and `_displayedBytes` probe the two components — vary one byte,
watch the output move — and a competing version that recomputed `abbreviate`'s
arithmetic was deleted for passing under a mutation to `abbreviate` itself. That
is the pattern the third probe has to join, not a pattern to redesign.

## Goals / Non-Goals

**Goals:**

- The name reads key bytes `18..23` directly, with nothing between the key and
  the draws.
- The three-channel allocation is enforced by something that fails when it is
  violated, in both languages, at the point where each channel is derived.
- The four falsified retraction sites describe the mechanism that now exists.

**Non-Goals:**

- **Deleting the author address.** `identity.rs::address()`, `feed.rs`'s `author`
  field and the specs naming it are the `key-identity-sweep` piece. This piece
  states the property; the sweep performs the removal. `Address` remains one type
  for Stoa and author addresses, and every `Address` left standing here is
  reachable from Stoa code.
- **Touching `OP_SIGNING_PREFIX`** or anything on the signing path.
- **Rendering a name in the UI.** `PostHeader` still receives one as a string.

## Decisions

### Why the name's six bytes are `18..23`: contiguous, and the gaps left empty

The allocation is the decision this piece exists to make, so it gets its own
entry rather than sharing one with the question of how the window is *expressed*
(below). The spec's table gives the mark `4..11`, the abbreviation `0..3`,
`14..17` and `29..31`, and the name `18..23` — 25 of 32 bytes, leaving seven
unallocated in two runs, `12..13` and `24..28`.

**Alternative considered: split the name across the gaps.** Six of the seven free
bytes — `12`, `13`, `24`, `25`, `26`, `27` — would have held the name entirely
inside what the abbreviation hides, and left a contiguous run free for whatever
comes next. That is a real option and it was not taken.

What ruled it out is that a split window stops being one fact. The derivation is
`key_bytes[name_key_bytes()]` — a single slice, so the window is *what the code
reads* rather than a figure a comment asserts, and the whole argument in the next
entry (one range cannot be half-moved) depends on there being one range. A split
window needs a list of ranges, a slot-to-range mapping, and `pin_name.rs`'s
`NAME_FIRST_BYTE + 2` arithmetic stops working; each of those is a place two
implementations can disagree about how far to read, which is the failure the
scheme exists to prevent. The gain — one contiguous free run instead of two — is
speculative, for a fourth channel nobody has specified.

**A contiguous window is also the half a human can check.** Bytes 18..23 are six
hex pairs a holder reads off consecutively; a name derived from bytes 12, 13, 24,
25, 26, 27 cannot be hand-verified without a diagram.

**Alternative considered: call the seven bytes reserved rather than unallocated.**
Rejected, and the wording is load-bearing rather than pedantic. "Reserved" claims
something is coming; nothing is. The spec's own warning is that a range read by
nothing gets taken as load-bearing by the next reader — and this is not
hypothetical here. `docs/IDENTICON.md` records that an earlier version of this
very document claimed bytes 0..11 were "reserved for the generated-name scheme"
and derived the two channels' independence from it, when no such mechanism
existed; two documents invented the same false reservation independently. The
word is what did that, so the four sites that mention the gaps
(`names.rs`'s module header, `names.rs`'s unallocated-byte test, `Identicon.qml`,
`IDENTICON.md`) each say **unallocated, not reserved**, and say that extending a
channel onto them is a spec change rather than a local edit. The gate
`the_name_reads_no_unallocated_byte` is the enforcement.

What this forecloses, recorded because it is the cost: no channel can be widened
without changing the spec's table, and a fourth channel must be specified into
the gaps rather than helping itself to them. That is the intended direction —
the gaps are free for a future spec to allocate, not free for a future commit to
take.

### The name reads `key.to_bytes()[18..23]`, and the type carries the window

`name_from_digest(&[u8; 32])` becomes `name_from_key_bytes(&[u8; 32])`, and the
six bytes it draws are taken as a slice at a named offset rather than from index
0. The rename is not cosmetic: the parameter stopped being a digest, and a
function still called `…_from_digest` invites a caller to hand it a digest, which
is exactly the mistake the change exists to remove.

The window is expressed as one range rather than as a start and a length, and the
derivation slices with it:

```rust
let drawn: &[u8; NAME_BYTE_COUNT] = key_bytes[name_key_bytes()]
    .try_into()
    .expect(...);
```

It began as a `const NAME_KEY_BYTES` and ended as a `const fn name_key_bytes()`;
the last paragraph of this entry is why, and the shipped code has no constant of
that name.

**Alternative considered: two constants, a start and a length.** Rejected because
two constants can disagree — a start moved without the length is a silent window
slide, and the spec's *A shifted byte window is visible* scenario is precisely
about that failure. One range cannot be half-moved.

**Alternative considered: keeping the offset at 0 and having the caller slice.**
Rejected because it puts the window at the call site, where a second call site
would acquire its own copy of it. The whole point of the allocation is that it is
one fact.

**It ended up a `const fn` rather than a `const`, and the code is what taught
this.** `const NAME_KEY_BYTES: Range<usize>` compiles, but a `Range` is not
`Copy`, so a `const` of one is re-materialised as a fresh temporary at every use
— and `rustc`'s `const_item_mutation` lint fires on `NAME_KEY_BYTES.any(..)`,
because the iteration advances a temporary that is then discarded. The available
fixes were a `.clone()` at each iterating call site or a function. The clone is
the trap: it is exactly the kind of noise a later reader deletes as redundant,
and deleting it reintroduces a call that iterates nothing while still compiling.
A function hands out a real range each time and has no such affordance.

### `NAME_PREFIX` and `name_digest()` are deleted rather than deprecated

Both go. `name_digest` was `pub` solely so a test could prove the name's digest
differs from the address — a claim the spec now removes as inverted. Leaving it
behind would leave a function whose doc comment argues for a withdrawn mechanism,
and a reader would take its existence as evidence the name still hashes.

The cost is recorded in the spec and not re-argued here: **no preimage means no
versioning seam**, so a wordlist change renames everyone at once with no way to
tell the schemes apart. The owner took this deliberately on issue #80; the name
is a pure local function and never published, so the divergence is a client-side
rendering difference.

**Alternative considered: keep a hash, but one per channel, each under its own
separator.** This is the shape that would have preserved the versioning seam —
`H(NAME_V1 || key)` for the name, `H(MARK_V1 || key)` for the mark — and it is
the obvious thing to reach for once the cost above is stated, so it is recorded
here rather than left for the next reader to re-derive.

It was ruled out by the owner's ruling on #80, which is that the name reads the
key's own bytes with no hash anywhere. What that ruling buys is that a name is
verifiable by hand: three 16-bit big-endian draws off bytes 18..23 of a key the
holder can read on screen, so a second implementation is a page of arithmetic
rather than a SHA-256 and a separator string that must match byte for byte.
Per-channel separators also put the independence of the three channels somewhere
invisible — a reader cannot tell by looking whether two separators differ in a
way that matters — where byte disjointness is a table anyone can check and a gate
can measure.

What it costs is exactly the one item recorded above and nothing further: the
seam. Independence is not among the costs, which is the thing worth being
explicit about — it moved from domain separation to the byte allocation rather
than being given up, and this change is what makes that allocation a stated
requirement with a gate on both sides of it instead of an accident.

### The disjointness gate is two gates, because the channels are in two languages

Neither language can check the property alone, and pretending otherwise is how a
gate ends up measuring one side of a two-sided claim.

- **Rust** owns the name's window against the *allocation table*. It cannot see
  the mark or the abbreviation at all, so what it can check is that the name's
  measured window is what the spec allocates, and that it overlaps neither of the
  two ranges the spec allocates to the other channels. Those two ranges are
  written down in the test as the spec's figures — this is the one place they are
  restated, and it is a restatement of the **spec**, not of another
  implementation.
- **QML** owns the real pairwise check, because it is where two of the three
  channels actually are. It measures all three by probing.

**What makes the QML side a measurement and not a restatement** is the third
channel's shape, below.

### The third probe measures a component, `DKeyNameWindow`, not a constant

The deleted computed probe failed because it restated production arithmetic; a
probe that read the name's window out of a `DTheme` constant would fail the same
way, one level further out — it would follow the constant wherever it went and
could never report an overlap.

So QML gets a component whose output *depends on the key bytes the name reads*,
and the probe varies bytes and watches it move, exactly as `_markBytes` does for
`Identicon`. `DKeyNameWindow.qml` computes the **three draw indices** — the same
three 16-bit big-endian reductions the Rust derivation performs, over the same
window — and exposes them. It deliberately does **not** carry the wordlists or
produce a name:

- The wordlists are 10,240 entries and are consensus-critical; a second copy in
  QML is a second thing to keep in step, and `CLAUDE.md`'s core/UI split puts
  derivation in core.
- The indices are sufficient for the property being gated. The question is
  *which key bytes reach the name*, and an index moves if and only if a byte it
  reads moves.

**Alternative considered: exposing the name's window through `Core` and probing
that.** Rejected: `Core` is the RPC bridge, the test runner has no module behind
it, and a probe that cannot run under `qmltestrunner` is not a gate.

**Alternative considered: no QML name channel, and asserting the Rust window
against QML's two in a script.** Rejected because a cross-language script
comparing two numbers is exactly the computed version that was deleted — it
restates both sides instead of measuring either.

The duplication this accepts is real and is the trade-off: the reduction
arithmetic now exists twice.

**What keeps it honest is a pin, and writing the gate showed that the window
alone was not enough.** The disjointness tests only ever ask *which bytes* a
channel reads, so a `DKeyNameWindow` that read the right bytes and reduced them
with the wrong modulus would satisfy every one of them while deriving different
names from core for the same key on the same build — a divergence nothing in
either suite could see. `test_the_name_window_agrees_with_cores_pinned_case`
closes it by pinning the three draw indices for one key, against the figures
`examples/pin_name.rs` produced. So the two implementations agree with a third
party rather than with each other, and a modulus drift fails that test alone —
measured, by changing `% 1024` to `% 1000` and watching exactly one test go red.

### The byte probes try every value of the byte, not a list of good ones

Both languages measure "which bytes does this channel read?" by varying one byte
and watching the output. The question is which values to try, and the answer
changed twice under measurement.

**One value is wrong, and shipped once.** `Identicon._weave()` is `_byte(11) % 3`,
and `0x00 % 3 == 0xff % 3`, so a probe that flipped a byte to `ff` concluded the
mark does not read byte 11 — a false all-clear on a real disjointness break.

**Seven spread values fixed that, and are still wrong.** They fix a *coincidence*
in an unconditional read, which was the failure in hand. They do nothing about a
**conditional** read: a derivation that touches an unallocated byte only when it
holds one particular value is invisible to any probe whose list omits that value,
and every disjointness and unallocated-byte assertion then reports clean. This was
measured, not reasoned about. Adding
`^ if key_bytes[12] == 0x42 { 1 } else { 0 }` to the adjective reduction left all
38 `names::` tests green — `the_name_reads_no_unallocated_byte` and
`the_name_reads_exactly_the_bytes_the_spec_allocates_to_it` among them; the same mutation in
`DKeyNameWindow.adjectiveIndex()` left all 17 `Identicon` tests green and the
whole QML run exiting 0.

**The fix is not a longer list, and that is the whole point.** Appending `0x42`
makes those two mutations fail and reproduces the defect at `0x43`. A probe that
enumerates values is always one value short of something — this repo's
`hand-maintained sweep lists go stale silently` trap, arriving as a literal array
rather than as a list of method names. The list is now the byte's entire domain,
`0..=255`, which is the one list that cannot be extended and cannot go stale.

**Alternative considered: a seeded random sweep, wide enough that a single-value
carve-out cannot hide.** Rejected because it trades a certainty for a probability
and buys nothing with it. To be confident of catching a one-value carve-out a
seeded sweep needs samples on the order of the domain anyway, and it adds a seed
to record on failure and a flake mode where the gate's verdict depends on which
seed ran. Exhausting 256 values costs the same and always answers the same.

**Alternative considered: making a byte-conditional read unexpressible instead of
detectable.** This is the better shape where it is reachable, and in Rust it is
partly reached already — `name_from_key_bytes` binds `key_bytes[name_key_bytes()]`
once and every draw reads that slice, so reaching byte 12 has to be written in.
But "has to be written in" is a reviewing argument, not a gate, and QML has no
equivalent at all. Detection is the reachable ceiling, so the detection is made
total over the thing it can be total over.

**What this does not cover, stated so it is not read as more.** A read gated on
**two** bytes at once (`key[12] == 0x42 && key[13] == 0x99`) is invisible to any
one-byte-at-a-time sweep from a fixed base, however many values it tries; closing
it would need 256² pairs per byte pair. The sweep's claim is exactly: for each
byte, holding every other byte at zero, the channel's output is constant over that
byte's whole domain. That is stated in `measured_name_bytes`'s doc comment rather
than left for a reader to infer, because the neighbouring
`the_derivation_reads_no_byte_outside_its_window` looks like it closes the residue
and does not — it is a two-value fixture and passes the `0x42` carve-out too.

**Cost, measured.** Rust: 32 × 256 = 8,192 derivations, and the `names::` suite
still runs in 0.06s. QML: `tst_identicon.qml` goes from 68ms to ~1.1s, which is
the right trade for the one gate standing behind this piece's central property.

One related defect was found while making this change rather than reported:
`test_a_byte_one_channel_reads_moves_only_that_channel` looped `&& !moved`, so it
stopped at the first value that moved the target channel — making its own comment
("the other two must be untouched for EVERY probe value") false for every value
after the first. It now runs to the end of the domain.

### The pinned cases have a precondition, and it is asserted rather than assumed

The spec requires pinned cases "chosen so that the bytes outside `18..23` differ
between them", because a window that slipped a byte would otherwise read the same
values in both and both pins would pass. That is a property of the *fixtures*,
and a fixture property nobody checks is a fixture property that decays: someone
re-derives a pin against a new key and the pins go quietly green about a window
that moved.

`the_pinned_cases_differ_outside_the_name_window` asserts it, and asserts it at
the four bytes a one-byte slip would actually reach — 16, 17, 24, 25 — rather
than as a general "the keys differ". The general form is satisfied by keys
differing only at byte 0, which would prove nothing about a slip.

### Pairwise, not "all three sets are disjoint"

The spec says pairwise, and the difference is not pedantry. A check written as
"the union of the three has 25 distinct entries" passes when a channel measures
empty, and the meta-test the existing gate already carries exists because a probe
that finds nothing reports a false all-clear. Pairwise with a non-emptiness
precondition on each set states what is actually required and names the
overlapping byte when it fails.

## Risks / Trade-offs

- **The reduction arithmetic exists in two languages** → `DKeyNameWindow` is
  pinned in the QML suite against the same written-down window and draw values
  that the Rust suite pins, so the two drifting apart fails on both sides rather
  than silently. The alternative — no third channel in QML — leaves the pairwise
  property ungated where two of three channels live, which is worse.
- **Every derived name changes** → No name is persisted or published (the spec's
  opening requirement), so nothing migrates. Every pinned expected name is
  re-derived through `examples/pin_name.rs`, which does not link the crate.
- **The colliding-key fixture stopped colliding** → Re-searched, not adjusted.
  `tests_support`'s doc comment says "if a test using these fails, the pair has
  stopped colliding … do not go and find a new pair to paper over it", and that
  instruction is right for every reason except this one: the derivation it was
  found against no longer exists, so the pair was never evidence about this
  scheme. The distinction is recorded in the fixture's own comment, because the
  next reader will meet the instruction before they meet the exception. Both new
  keys were verified through `pin_name.rs` to reach indices `(2628, 768, 971)`,
  so the collision is a fact about the scheme rather than about the search that
  found it.
- **No versioning seam for the wordlists** → Accepted on the owner's ruling and
  recorded in the spec's freezing requirement. Not mitigated, because the
  mitigation is the preimage that the ruling removes.
- **Seven bytes are unallocated and read by nothing** → They are *unallocated,
  not reserved*. The spec says so in those words, and the comments in this change
  say so too, because "reserved" is the word that created the fiction this change
  is correcting.

## Migration Plan

No data migrates: nothing derived here is stored or published. The build carries
the change wholesale — two peers on different builds render different names for
one key, which is the client-side rendering difference the spec's freezing
requirement records.

The author address survives this piece and is removed by `key-identity-sweep`.
Between the two pieces, `identity.rs::address()` still exists and the mark still
takes an `address` property name in QML while being fed a key; that is the sweep's
to finish, and the comments in this change say which half is which rather than
implying the sweep already happened.
