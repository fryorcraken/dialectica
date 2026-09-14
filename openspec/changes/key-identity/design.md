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

### The name reads `key.to_bytes()[18..23]`, and the type carries the window

`name_from_digest(&[u8; 32])` becomes `name_from_key_bytes(&[u8; 32])`, and the
six bytes it draws are taken as a slice at a named offset rather than from index
0. The rename is not cosmetic: the parameter stopped being a digest, and a
function still called `…_from_digest` invites a caller to hand it a digest, which
is exactly the mistake the change exists to remove.

The window is expressed as one `Range<usize>` constant, `NAME_KEY_BYTES`, and the
derivation slices with it:

```rust
let drawn: &[u8; NAME_BYTE_COUNT] = key_bytes[NAME_KEY_BYTES]
    .try_into()
    .expect(...);
```

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
