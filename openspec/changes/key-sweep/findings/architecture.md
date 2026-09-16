# Architecture review — key-sweep

Dimension: **architecture** only. Correctness, security and readability are held
by other instances; spec/test correspondence and design-decision correspondence
are held by the spec-test and design reviewers.

Verified in a worktree at `a5f83ba`. `cargo test -p dialectica-core`: **987
green** (957 unit + 30 integration), matching `tasks.md`' claim exactly.

---

- [x] **`spec-writer`** — `openspec/changes/key-sweep/specs/op-format/spec.md`
      and the absent `specs/stoa-metadata/` — the proposal promises two bare
      "The address is the identity" sentences are scoped to say *Stoa*, and
      neither delta makes the edit
      **Scenario:** `proposal.md:227–231` states: *"`stoa-metadata` and
      `op-format` each close a requirement with the bare sentence **"The address
      is the identity."** … Both are scoped to say *Stoa*."* Neither is.
      `openspec/specs/op-format/spec.md:444` still ends *"The address is the
      identity; a name never is"*, inside `### Requirement: Valid text is never
      normalised or otherwise transformed` — a requirement the `op-format` delta
      does not list among its three MODIFIED entries (`An op is the unit that
      crosses the wire`, `An op is named by a content-derived id`, `An op carries
      no ordering field and no per-peer state`). `openspec/specs/stoa-metadata/
      spec.md:119–120` still ends *"The address is the identity."*, and
      `key-sweep/specs/` contains **no `stoa-metadata` directory at all**, so
      nothing in this change can reach it.
      **Measured:** `grep -rn "The address is the identity" openspec/specs`
      returns 2 hits, both untouched by `git diff origin/main...piece/key-sweep`.
      **Severity: medium.** Both sentences are Stoa-scoped by their immediately
      preceding clause (*"show the Stoa address alongside any name"* /
      *"never resolve or match a Stoa by title"*), so neither requirement is
      *wrong* — which is why this is medium and not high. But the proposal
      identified these two sentences by name, gave the reason (*"the unscoped
      sentence is the one a reader will cite afterwards as authority for the
      thing this change removes"*), and then did not act. The code half of the
      same reasoning **was** done — `stoa.rs:190–197` scopes its copy explicitly
      and says why — so the spec half is an omission against a stated intent
      rather than a decision. Either make the two edits, or amend the proposal to
      record that the preceding clause was judged sufficient scoping.
      **Fixed — the edits are made, which was the better of your two options**
      given the code half was already done and the proposal's reasoning was
      sound. Confirmed your measurement first: `grep -rn "The address is the
      identity" openspec/specs` returns exactly two hits, and
      `key-sweep/specs/` had no `stoa-metadata` directory.
      `op-format`: the `Valid text is never normalised or otherwise transformed`
      requirement is now carried in the delta as MODIFIED, with the closing
      sentence reading *"The **Stoa** address is the identity; a name never is."*
      `stoa-metadata`: a delta now exists, carrying `A displayed title is never
      an identifier` with *"The **Stoa** address is the identity."*
      Both deltas state explicitly that **only that sentence changes** and that
      the requirement's meaning is unaltered, since the preceding clause was
      always Stoa-scoped — so a reader diffing them is not left hunting for a
      substantive change that is not there. The reason is recorded in each: the
      bare sentence is what a later reader would cite out of context as authority
      for an author address.
      The proposal's past-tense promise is also corrected to name where the edits
      live, with a parenthetical recording that it had claimed the work as done
      before it was.
      `openspec validate key-sweep --strict` passes with the new capability.

- [x] **`spec-writer`** — `wire.rs:184`, `:734`, `:781`, `:1059`, `:1568`,
      `:1827` — one value, an Ed25519 public key's hex, is now reported under
      three different field names across six replies, and no capability owns the
      inconsistency
      **Scenario:** after this change every one of these carries the same 64-hex
      public key: thread item `author` (`wire.rs:1827`), feed row `author`
      (`:1568`), slate candidate `publicKey` (`:734`), kept identity `publicKey`
      (`:781`), whoami `publicKey` (`:1059`), capability probe `identity`
      (`:184`). A view holding a thread item's `author` and wanting its name must
      re-spell it as `publicKey` to call `display_name`, whose contract is
      `{"publicKey":"<64 hex chars>"}` (`wire.rs:2812`). That is a branch on
      where the data came from, which CLAUDE.md's *"JSON shapes are
      source-independent, so a view renders without branching on where the data
      came from"* exists to prevent.
      **Measured:** the shipped view already branches. `DOnboardingScreen.qml`
      reads `row.modelData.publicKey` (`:441`, `:461`) and
      `screen.keptIdentity.publicKey` (`:655`, `:665`); `FeedScreen.qml:602`
      reads `row.modelData.author` — and **both feed the same
      `Identicon.address` property with the same value**. Two spellings, one
      concept, one sink.
      **This change created the condition, and did not create the names.** On
      `origin/main` the three spellings named three genuinely different values:
      `identity`/`address`/`author` were author addresses and `publicKey` was the
      key, so distinct names were correct. Collapsing the two values into one
      made the names redundant, and `design.md` §4 reasons about the spelling
      only *within* the thread/feed family (*"`author` was kept because it is how
      every other reply here spells the same job"* — `wire.rs:1824–1825`), never
      across the six sites.
      **Severity: medium**, and this is a **defect of the change's scope rather
      than of its code** — nothing is incorrect, and unifying six reply fields is
      plainly its own piece. What is missing is that the condition is *recorded*.
      The proposal records the feed-row spec gap at length and the thread item's
      spelling choice twice in `// NO SPEC:` markers; this larger and now-live
      inconsistency is recorded nowhere. Ask for a paragraph in `proposal.md`
      naming it and filing it, on the same standard the proposal sets for itself:
      *"what this change owes is that the gap is stated rather than discovered
      later from a diff."*
      **Fixed as asked — filed, not fixed in code.** `proposal.md` gains a
      section, *One value, three field names — created here, filed rather than
      fixed*, which names all six sites, states that every one now carries the
      same 64-hex public key, and reproduces your two strongest points: that the
      three spellings named three genuinely different values before this change
      (so the names were correct and this change is what made them redundant),
      and that the inconsistency is **already live** in the shipped view, with
      `DOnboardingScreen.qml` reading `publicKey` and `FeedScreen.qml` reading
      `author` while both feed the same `Identicon.address`. The `display_name`
      re-spelling is named as the concrete cost, against CLAUDE.md's
      source-independence rule.
      The deferral is argued on the same ground the proposal uses for the
      feed-row contract — unifying six reply fields across four capabilities is a
      breaking wire change of its own size, and bundling it into a deletion would
      make neither half reviewable. Your point that `design.md` §4 reasons about
      the spelling only *within* the thread/feed family and never across the six
      sites is recorded as precisely the gap the section closes.

- [x] **`dev-writer`** — `Identicon.qml:102`, `AddressLabel.qml:48` — one
      `address` property is now fed two different kinds of value by two groups of
      callers, and nothing at the property says which it is holding
      **Scenario:** `Identicon.address` and `AddressLabel.address` are each fed a
      **public key** by `PostHeader.qml:31`/`:45` (via `identityKey`) and by
      `DOnboardingScreen.qml:441`, `:461`, `:655`, `:665` (via `publicKey`) — and
      a **Stoa address** by `FeedScreen.qml:306`/`:325`, `DJoinScreen.qml:244`/
      `:265`/`:498`/`:513`, and `DStoaListScreen.qml:347`/`:375`/`:529`.
      **Measured:** `grep -rn "address:" dialectica-ui/src/qml` — 15 call sites,
      **6 passing a key and 9 passing a Stoa address**.
      **Severity: low, and this is a readability/naming observation rather than a
      defect.** The *decision to keep the property generic is correct and I am
      not asking for it to change* — `tasks.md` 4.3 makes the call explicitly,
      the nine Stoa callers are real and live, and narrowing the property to
      `key` would break every one of them. `Identicon.qml:88–97` and
      `PostHeader.qml:11–19` both already explain the split accurately, so the
      reasoning is recorded. What is worth a line is that **the property's own
      declaration carries none of it**: a reader arriving at
      `Identicon.qml:102`'s bare `property string address: ""` from one of the
      six key call sites sees a name that denies what it holds, and must scroll
      to the file header to learn the name is only half right. A one-line comment
      at each of the two declarations — *"a Stoa address or an author's public
      key; 32 bytes of hex either way"* — closes it. Flag if the reviewer holding
      **readability** has already raised this; it sits on that boundary and I do
      not want it double-counted.
      **Not double-counted** — you asked, so: the readability reviewer examined
      both components and passed them, explicitly calling `Identicon.qml`'s
      retained `address` property "explained correctly and for the right reason"
      and `AddressLabel.qml` as correctly keeping `address` "and says why". They
      raised no box. So this is yours alone.
      **Fixed at one of the two declarations, and here is why not both.** Reading
      them showed the situation is asymmetric: `Identicon.qml:87-101` already
      carries a fourteen-line comment **at the declaration itself**, stating that
      for an author it holds the public key and for a Stoa the Stoa address, that
      the name is "now only half right", and why renaming was rejected. Adding
      your one-liner above that would restate it.
      `AddressLabel.qml:48` was the bare one — its explanation sits ~20 lines up
      in the file header, which is exactly the scroll you describe. It now
      carries the line, close to your wording: *"A Stoa address or an author's
      public key; 32 bytes of hex either way, and the name is only half right"*,
      pointing at the header for the rest and recording that nine call sites
      still pass a Stoa address.
      Your accompanying judgement — keep the property generic — is untouched, and
      your measurement of 6 key callers against 9 Stoa callers is what the new
      comment cites.

---

## What is clean, in prose

**The four things I was asked to confirm all hold.** `OP_SIGNING_PREFIX` is
untouched (`identity.rs:76–80`) and correctly argued as out of scope — it
separates the signing digest and never participated in identity derivation.
Stoa addresses are untouched: `STOA_ADDRESS_PREFIX` keeps its value,
`identity::stoa_address` and `Genesis::address` survive, and the Stoa-address
known-answer pin is verbatim. `Identicon` and `AddressLabel` are left generic
rather than half-renamed, and that is **measured to be right rather than merely
argued**: nine of their fifteen call sites still pass a Stoa address
(`FeedScreen.qml:306`/`:325`, `DJoinScreen.qml:244`/`:265`/`:498`/`:513`,
`DStoaListScreen.qml:347`/`:375`/`:529`), so renaming the property to `key` would
have narrowed a component with a live majority of Stoa callers. `tasks.md` 4.3
called this correctly. And the sweep is **complete**: `grep -rn "fn address"` over
`dialectica-core/src` returns exactly one production definition,
`Genesis::address` at `stoa.rs:355`, which is the Stoa derivation.

**Question 1 — `Address` left unenforced is defensible, and I would not block on
it.** `design.md` §2 rejects the `StoaAddress` newtype on two grounds and both
survive checking. The blast-radius argument is real (`Address` appears in
`op.rs`'s `Op.stoa`, `stoa.rs`, `feed.rs`, `thread.rs`, `membership.rs`,
`moderation.rs`, `transport.rs`, `wire.rs` and the adapter), and CLAUDE.md's
*make the change easy, then make the easy change* makes a rename a separate
behaviour-free commit rather than a rider on a deletion. The substantive
argument is the stronger one and I verified it: **after this change there is no
other 32-byte identity value for an `Address` to be confused with.** Every
non-test `Address::from_bytes` call site is a Stoa address (`op.rs:670` decoding
`Op.stoa`, `membership.rs:732`, `identity_store.rs:500`, `transport.rs:1757`),
and the one value that would be confusable, `OpId`, is already its own type. The
residual risk is therefore a *future* author-shaped 32-byte value, and the
mitigation is proportionate: `identity.rs:113–124` states the narrowing
positively at the first place a reader looks, names `derive_stoa_key`'s
parameter as an input rather than a leftover, and
`no_derivation_turns_a_public_key_into_an_address` leaves a runtime witness that
an author derivation was retired on purpose. That last one is the part that
makes the deferral safe rather than hopeful — it pins the *absence* with the
retired derivation's own measured output, so a reintroduced author address under
the old prefix trips it.

**Question 3 — `verify_authored_op` still earns its name and its existence.**
The name describes it: it verifies an op against its author, and the author is
now the key. It is not a pass-through to `verify_op_bytes`, and the distinction
is load-bearing — it takes `&[u8]` where `verify_op_bytes` takes parsed types,
so it owns the three parse-failure refusals on the attacker-controlled path
(`identity.rs:266–279`), which is exactly the *"a guard is a job"* separation
CLAUDE.md asks for. `verification_takes_no_author_identifier_beside_the_key`
pins the arity with a typed `fn(&[u8], &[u8], &[u8]) -> bool` binding, so
reintroducing a fourth parameter is a **compile** error — a structural gate no
runtime assertion could give. The rename was considered and rejected in
`design.md` §3 with a reason I agree with (`op-transport` still calls this the
verification step). And the deleted guard genuinely was vacuous: `SignedOp::
verify` computed the claimed author by calling `.address()` on the op's own key,
so `k.address() == k.address()` could not fail for any input.

**Question 4 — the feed-row spec deferral is right, and `// NO SPEC:` is the
correct instrument.** Writing a feed-reply contract means specifying every
field, the pagination shape and the moderation flags; bundling that into a
deletion would make neither half reviewable, which is CLAUDE.md's *do not
refactor speculatively* applied to specs. Making the edit anyway is also right —
leaving the row naming an author by a derivation that no longer exists would be
worse than an unspecified-but-correct field. The marker is placed at both the
emission site (`feed.rs:657–664`) and the test that pins it
(`feed.rs`, `wire.rs:1819–1826` and `:6697–6701`), names what was chosen, and
points at `design.md` §5 — so it is greppable and it carries its reasoning,
which is what distinguishes a recorded gap from an undocumented one.

**Question 5 — the six capabilities are coherent, with the one exception filed
above.** I checked each merged spec site that still mentions an author address
against the deltas: `identity/spec.md:208–239` (the record-hashing requirement)
is wholly REMOVED with a migration note; `:241–255` (the four-way pin) is
MODIFIED to three pins plus an absence scenario; `:272–296` and `:298–319` are
MODIFIED and RENAMED to say Stoa and to state what verification actually
establishes; `thread-read:161` is RENAMED to the single-field contract;
`identity-onboarding:115` and `view-identity-onboarding` are both MODIFIED. The
`generated-names` hits are all already-correct prose about the deletion.
`content-authoring` being audited-and-unchanged is the right call and is
recorded, so *"considered"* and *"missed"* are distinguishable from the delta —
which is the standard this repo sets.

**Question 2 — the narrowing is otherwise shaped right.** Three of the four
sub-judgements are good. Deleting a field rather than repointing it, where a
`publicKey` already sat beside it (`design.md` §4's second table), is correct:
carrying both would put a derived value on the wire beside the material it
derives from, which is the same argument `generated-names` uses to keep names
off replies. Collapsing `ThreadItem`'s two author fields into one is correct for
the same reason, and choosing `author` over `authorKey` gives the loud failure
(a caller reading `authorKey` gets a missing field, not a wrong value). Keeping
the probe's `identity` field name is right — `posting-capability` is written
over *"the identity that would post"* and never over an address, so the contract
genuinely does not move. And `creator_and_poster_in` collapsing to
`creator_key_in` is a real architectural improvement: a pair whose second element
was `creator.address()` was never a pair, and the invariant it bought is now
carried by there being one value. That is CLAUDE.md's *complexity in the data
structure, not the logic*, applied correctly.

**The doc-comment correction pass is the strongest part of the change.** Six
comments across `op.rs`, `identity.rs`, `transport.rs` and `revision.rs`
credited an address re-derivation for a refusal the **signature** check
produces — a claim that was false *before* this change too, since the only
caller derived the claimed author from the op's own key. Each is corrected, and
the mechanism is measured rather than asserted (stubbing `verify_op_bytes` to
return `true` fails the tests that name it). Two forgery tests also gained the
control clause their own spec scenario names — that the rejected signature is
genuinely valid under its own key — without which a junk-signature fixture would
have passed identically. That is this repo's one recurring test defect family
(two explanations, one answer) being closed rather than propagated.
