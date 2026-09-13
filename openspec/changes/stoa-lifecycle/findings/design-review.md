# Design-review findings — stoa-lifecycle

## Provenance, and why this file is a reconstruction

**These findings were relayed to the fixer through the runner's brief rather than
written to a file, because this piece's review ran before
`.claude/agents/README.md` gained the findings-file rule.** They are recorded here
after the fact so they survive the thread they arrived in.

That is the exact failure the rule exists to prevent. The measurements below are
the reviewer's, restated by the fixer; the reviewer's own wording and its full
evidence are **not recoverable**. Where the fixer re-derived something
first-hand it says so — **everything else in this file is second-hand.**

Six findings. Entry 1 is the one the piece turned on.

**Every `file:line` below is the position when the finding was made**, i.e. before
the fixes. The code and `design.md` have moved since. Lines outside this piece's
own files (`log/sqlite.rs:74`, `moderation.rs:141`/`:174`/`:191`) were re-verified
against the current tree and still hold; the `membership.rs`, `wire.rs`,
`design.md` and `UI-BRIEF.md` numbers are historical and should be located by
symbol or heading, not by line.

---

- [x] **1. The creator key is a third identity, and it breaks moderation**

**For:** `dev-writer`

`creator_public_key()` was `derive_stoa_key(root, CREATOR_KEY_DOMAIN)` with
`CREATOR_KEY_DOMAIN = [0u8; 32]`. The peer's **posting** identity in that same
Stoa was `derive_stoa_key(root, real_address)` — what `get_capabilities` reports.

**These are different keys.** `Moderators::of` names `genesis.creator` as the sole
moderator (`moderation.rs:171-174`, `:191`). So a peer that creates a Stoa is its
sole moderator under a key it will **never sign an op with**, and
`Moderators::contains(posting_key)` is **false for its own creator**.

**Why it could not wait for moderation to be in scope:** the genesis record is
immutable once published and the address is its hash, so every Stoa created before
a fix carries a creator key that cannot moderate it, **permanently**. Deferring
means shipping records that are wrong forever.

**And a PLAN contradiction bearing on the fix.** `docs/PLAN.md` §5.2's MVP
subsection records the owner's decision that for the MVP **a user has one identity
across every Stoa**, and that this means **not calling `derive_stoa_key`** —
signing with the root key directly. The creator key called it with a synthetic
context, a **fourth derivation position that no document argues for**.

The reviewer noted the circularity that produced the design is real and verified:
the address is `SHA-256(canonical_bytes)` over a record whose `creator` field is
the key in question, so no per-Stoa key is derivable before the record exists. A
synthetic domain was a reasonable answer to that — just not one compatible with
one-identity-per-user.

`keystore.rs` (the old `creator_public_key` and `CREATOR_KEY_DOMAIN`),
`rust-lib/src/lib.rs` (both adapter closures), `moderation.rs:171-174`, `:191`.

**Outcome: FIXED** in `4313cf6`.

`Keystore::identity_key` is the root secret used directly as an Ed25519 seed, and
both the genesis record's `creator` and the capability probe's answer come from
it. **One derivation position where there were three.** `CREATOR_KEY_DOMAIN` is
deleted, not retargeted.

**Reconciliation with PLAN §5.2, which the reviewer asked be argued explicitly if
it departed: it does not depart — the old code did.** §5.2's MVP subsection says
verbatim that one identity per user means *not calling* `derive_stoa_key` and
signing with the root key directly, and §9.2 lists per-Stoa identity as out of the
MVP, saying `derive_stoa_key` "is built and simply is not called". **That claim was
false before this fix** (the adapter called it twice) and is true now. The root key
is knowable before any record exists, so under the MVP rule **the circularity
dissolves rather than needing a workaround.** `derive_stoa_key` and
`Keystore::stoa_key` stay built and tested; the live `identity` spec requires the
primitive's properties, not that a handler call it, and `posting-capability` asks
only that the reported identity be the key that would actually sign.

**Test written first and watched fail**, as asked:
`keystore.rs::the_creator_of_a_stoa_this_keystore_made_can_moderate_it`. Its first
form used the two methods the two adapter closures used —
`creator_public_key()` for the record and `stoa_public_key(&stoa)` for the posting
key — and failed on exactly *"the creator named in the genesis record must be the
key this peer presents in that Stoa"*. **`Moderators::of(genesis).contains(posting_key)`
is now true.**

`design.md` records the alternatives and what ruled each out — the synthetic
domain, derive-from-title, a fresh random key, take-it-from-the-caller, and a
two-pass derivation that does not converge — plus the security note (see
`correctness.md` entry 5) and why the deleted constant's preimage argument does not
survive.

---

- [x] **2. `design.md` credits the wrong mechanism for the file boundary**

**For:** `dev-writer`

Three places claimed **version independence** holds the boundary between the op
store and the membership store: `membership.rs:67-71` (doc comment),
`membership.rs:677` (test comment), `design.md:215-216`.

**Both constants are `1`** (`log/sqlite.rs:74`, `membership.rs:77`). So the version
check cannot separate the stores at all: hand `MembershipStore::open` an op-log
file and `found == 1 == MEMBERSHIP_LAYOUT_VERSION`, so it takes the `else` branch
at `membership.rs:257` and the refusal comes entirely from **`check_layout`**
failing to prepare `SELECT stoa, genesis_bytes FROM stoas`.

**`check_layout` naming columns is the only thing holding the boundary.**

The reviewer was explicit that the **recovery-asymmetry claim** (`design.md:218-223`)
is correct and genuinely distinct and must be **kept**: a membership store from a
future build costs the user their Stoa list and not their ops, which is a property
of the **separate file** and survives both constants being `1`. It also noted the
test at `membership.rs:675-704` already says this honestly in its own comment; it
is the framing sentences above it that overclaim.

**Outcome: FIXED** in `e019f3d`, all three places, with the recovery claim kept and
now explicitly labelled as a distinct claim rather than a restatement.

Independently corroborated: the **tester** reached the same conclusion from the
other direction, by pointing `membership_path_in` at `ops.sqlite` and observing the
failure was `LayoutDoesNotMatchItsVersion { why: "no such table: stoas" }` — the
column check, not the version check. That mutation is recorded in PR #45's body.

The fixer also tightened what "independent" means where it is still true: neither
refusal *reads* the other's constant, which is what the independence test asserts,
and comparing two constants that are both `1` would pass an implementation that
read one from the other.

---

- [x] **3. "Structural" is the wrong word for the statement choice, measured**

**For:** `dev-writer`

`design.md:96-102` said `INSERT OR IGNORE` makes non-destructiveness *"a property
of the statement rather than a branch"*.

**Reviewer's measurement:** change `membership.rs:386` to `INSERT OR REPLACE` and
delete the one `assert_eq!(…, Joined::AlreadyIn)` at `membership.rs:1014-1018` →
**544 tests pass, zero failures** (baseline also 544).

"Structural" is doing real work for the **shape** — `OR IGNORE` cannot write a
second row, and `join`'s pre-verification makes a mismatched pair unreachable, so
REPLACE could only overwrite byte-identical bytes. But the **statement choice** has
exactly one witness in the suite. `membership.rs:993-1006` states this in the
test's own comment; `design.md` did not carry it at all.

Asked for: record where the single witness is, and that **the wire deliberately
hides the distinction** (`wire.rs:647` and `:699` discard `Joined` with `Ok(_)`,
argued at `wire.rs:670-675`) — the consequence being that nothing observable at the
module surface tells `OR IGNORE` from `OR REPLACE`, so the spec's
non-destructiveness requirement is **unfalsifiable from outside core**. A
defensible choice whose cost was unrecorded.

**Outcome: FIXED** in `e019f3d`, as scoped — and deliberately **not** as a rewrite
of the "structural" claim.

A stronger framing was put to the fixer (that "structural" was simply wrong) and
the **correctness reviewer disagreed with it**, judging the single `assert_eq!`
adequate for `OR REPLACE` specifically because the **primary key** — not the
statement verb — is what makes the property structural, and `membership.rs`'s own
comment says so accurately. `design.md` now keeps the structural claim, names the
primary key as its load-bearing half, records the single witness and the
measurement, and records that the wire hides the distinction. The disagreement is
itself recorded in `correctness.md` because it changed what was done.

Independently corroborated by the **tester**, which reached the same measurement
and added that it extends to the wire-level coverage too (PR #45 body, finding 3).

---

- [x] **4. `joinStoa` contradicts PLAN §9.1 twice, unargued**

**For:** `dev-writer`

PLAN §9.1 #5 and §9.1 #6 Stage D both specify `joinStoa({address})`. The
implementation takes `{"stoa","genesis"}` (`wire.rs:654`, `:676`).

**The implementation is almost certainly right** — PLAN's own sentence is
incoherent (*"take an address, verify the genesis record"* — which record?), and a
hash verifies but cannot reconstruct. But `design.md:111-119` defended the **wrong
question**: it argued for including the address against a record-only signature,
and never noticed PLAN specifies the opposite omission.

Asked for: argue the departure in `design.md`, **and** update PLAN §9.1 — strike
the stale signature and the reasoning this change implemented down to a line
saying create/join/list exist, moving the rest to `design.md` where most of it
already is. PLAN duplicates the verification reasoning and **its copy is the stale
one**. That this branch did not touch `docs/PLAN.md` at all is itself the finding.

**Outcome: FIXED** in `e019f3d`, both halves.

`design.md` now argues the departure directly, including why the address is still a
parameter given the record determines it (the spec requires a mismatched record
leave the peer in *neither* Stoa, which a record-only call could not express).

`docs/PLAN.md` §9.1: the stale `joinStoa({address})` struck in **both** places, the
duplicated verification reasoning struck down to "built, see the capability", and
`createStoa` / `joinStoa` / `listStoas` marked BUILT with a pointer to the
capability as the authority for their actual shape.

Two further staleness items the fixer found while there, first-hand, not in the
finding: §9.1's *"nothing implements that pagination shape yet"* (the feed and the
Stoa listing both do), and Stage D's summary line not saying it was built.

---

- [x] **5. `docs/UI-BRIEF.md:230` contradicts its own fix**

**For:** `dev-writer`

It still said *"you join by pasting an address someone gave you, or by following a
link in a post"* — in the **Stoa list** section, surviving the edit verbatim.
Thirty lines later the brief says pasting an address alone is **not** enough, and
the summary at the top retracts the paste framing as the whole reason for the
edit.

So the document **asserts and denies the same thing**, and the surviving assertion
is in the section a designer reads **first**. UI-BRIEF is designed against by
someone who cannot read the code, so a contradiction there is worse than the
original error.

Also: *"Gate the affordance on that probe"* asks the view to call
`getCapabilities({stoa})`, which needs a Stoa address — and at **creation** time
there is none. **That obligation is not actionable at that screen.**

**Outcome: FIXED** in `307bf13`.

The Stoa list section now says how a Stoa reaches the user and points at *Joining a
Stoa* as the screen that argues the constraint — one statement, one place, rather
than a restated mechanism that can drift from it.

The un-actionable gate is replaced with what the screen can actually do: always
offer the affordance, show the core's own reason when it fails, route the user to
what fixes it. Gating on a build flag is still called wrong.

The fixer also added, first-hand, what the creator key now is, because a later
"you moderate this Stoa" badge answers the same question: the key recorded as
creator is the key the user posts under, and a Stoa's creator is its sole
moderator — so a designer should not assume the two could be different people.
Entry 1 makes constraint 2 of the brief true rather than aspirational.

---

- [x] **6. Three things live only in code comments, and one checkbox overclaims**

**For:** `dev-writer`

*"The archive is where someone greps; a code comment is not the archive."*

- **The creator key's three rejected alternatives** (derive from the title / fresh
  random / take from the caller) lived only in `keystore.rs`'s
  `creator_public_key` doc comment (~line 700). `design.md` covered the cost three
  times and named none of them.
- **`CREATOR_KEY_DOMAIN`'s value and the preimage argument** that made reusing
  `derive_stoa_key` for a non-Stoa purpose safe. Changing the constant silently
  re-mints the creator identity of every Stoa a user has made — `keystore.rs`'s own
  comment says so.
- **The `"open"` wire token.** `policy_name` (`wire.rs:555-558`) freezes a
  lowercase literal a view branches on. There are **five** `NO SPEC` subjects in
  the code and four in `design.md`; the wire-name one is the missing fifth.

Plus: **`tasks.md` 3.2 claims `creation_without_a_usable_key_fails_and_records_nothing`
verifies "mints no key". It cannot** — the closure returns `Err`, no `Keystore` is
in scope, and a handler that called `Keystore::generate()` then returned the error
anyway would pass every assertion. The test's own comment is honest about this; the
checkbox is not. `design.md:151-153` has the honest form (*"no path from this
handler to `Keystore::generate()`"*). Relabel 3.2 as satisfied-by-construction.

Plus: add to entry 1's record what the file split **forecloses** — two stores means
no transaction can span membership and ops, so a future "join and backfill" is
foreclosed. `design.md` stated the cost but not this.

**Outcome: FIXED** in `e019f3d`.

`design.md` now carries: the creator-key alternatives as a table with what ruled
each out — **revisited under entry 1, as asked**, so it is the post-fix set
(including the synthetic domain itself, and a two-pass derivation that does not
converge); the deleted constant's value, its address-determining property, and why
its preimage argument does not survive while the pinning obligation does; the
`"open"` token as the fifth `NO SPEC` subject, with a marker added at
`policy_name` so the code and the archive agree; and the foreclosed cross-store
transaction, stated with the file decision and repeated under Risks because it is a
trade-off and not only a rationale.

`tasks.md` 3.2 is relabelled satisfied-by-construction, saying what the test can
and cannot establish. 6.1's hardcoded test count was also replaced with the command
per CLAUDE.md — a number in a checklist cannot fail loudly when it drifts.

**Note for the `spec-writer`:** the **tester** independently raised "no key was
created as a side effect" as a requirement that is **not testable at this
surface** and should be scoped to the adapter or dropped, since it currently reads
as a checkable obligation and is not one. That is the spec-side half of this same
observation. See PR #45's body, finding 1, and `spec-test.md`.

---

- [x] **7. For the `spec-writer` — two unobservable requirements, carried forward from PR #45 so they survive that thread**

Two requirements the tester established are unobservable at the surface the spec
constrains. Both are the `spec-writer`'s to decide on, and neither is a defect in
the code:

1. **"No key was created as a side effect"** (under *creation requires a usable
   signing key*). The creation call takes a `FnOnce() -> Result<PublicKey,
   KeystoreError>`, so the keystore is entirely behind the closure and no test can
   observe whether one was minted. The property holds structurally — verified: no
   path from the handler or the adapter to `Keystore::generate()` — but by the
   absence of a call in code this capability does not own. Scope it to the adapter
   or drop it.

2. **"MUST NOT disturb what was already retained"** (under *joining a Stoa the peer
   is already in*). Confirmed by mutation: `OR IGNORE` → `OR REPLACE` fails exactly
   one test in the suite, on its `Joined::AlreadyIn` assertion. `join_stoa` discards
   `Joined` and the reply carries no "was this new" flag **by design**, and no rowid
   or insertion order is exposed, so nothing at the wire can distinguish the two
   statements. The requirement is real and its only witness is one store-level
   return value. Worth deciding whether the spec wants an observable form.

**Outcome: OPEN — `spec-writer`'s, and the box stays unticked.** Given a number and a
box by `dev-writer` so the merge gate can see it; it was a bare `##` section, which
`grep -rn "^- \[ \]"` does not match. No text changed.

Both still stand, and item 2's measurement survives this change's reshape — I re-read
it rather than assuming. `join` still discards `Joined` at both wire handlers and the
reply still carries no "was this new" flag, so the only witness is still the
store-level return value. `design.md` states the consequence in its own words under the
canonical-bytes decision: nothing observable at the module surface tells `OR IGNORE`
from `OR REPLACE`, so the non-destructiveness requirement is unfalsifiable from outside
core.

One addition for item 2, from a mutation run during this pass and worth having before
the spec decides: the verification reshape makes the *reason* sharper. A mismatched pair
can no longer reach `join` at all — `Membership::verified` is the only constructor and
`join` takes nothing else — so the only record `REPLACE` could ever write over a row is
the byte-identical one. That is now a property of the type rather than of the call
order, which strengthens the case that the requirement's observable form, if the spec
wants one, has to come from somewhere other than the write verb.

**Outcome: FIXED — both decided, and the two went opposite ways. Neither is deleted, and
neither keeps an unobservable scenario.**

**Item 1, "no key was created as a side effect" — the clause is removed from the
scenario, and the prohibition is kept in the requirement's prose.** The entry offers
"scope it to the adapter or drop it"; this is a third answer and I think the right one,
so here is the argument. *Dropping* it loses a real constraint — "creation must not mint
a key for the occasion" is the whole point of the requirement, and the reason is already
stated and good (a Stoa created under a key the user does not hold cannot be moderated
and cannot be un-minted). *Scoping it to the adapter* moves a requirement into a layer
this change's spec does not contract. So: the **MUST NOT** stays as a prohibition on
this call, and the **scenario clause goes**, because a scenario is the part that claims
a test can check it.

The requirement now says why in its own text: creation is handed the means of obtaining
a key rather than reaching for a keystore, so no caller of this capability can observe
whether one was minted, and a scenario asserting none was would assert something
indistinguishable from the call simply succeeding. It points at `keystore` as the
capability that can answer whether a key exists. The scenario keeps what is checkable —
failure, the reason, and no Stoa — and I **added** an assertion while I was there: the
failure carries the reason the key is unusable, which the tests already check against
the keystore's own message and which no scenario had stated.

**Item 2, "MUST NOT disturb what was already retained" — kept, and given an observable
form, which is what the entry asks whether the spec wants.** It does. The requirement
now says explicitly that "not disturbed" is specified as *what a later read answers*:
the founding values, the retained record's verification, and the count of Stoas for that
address MUST all be what they were before the repeated join. It states that it
deliberately does **not** constrain how the write is performed, and that a scenario
asserting no write occurred would assert something this surface cannot distinguish.

**The entry's own measurement is what makes that safe, and I used it rather than
re-deriving it.** A mismatched pair cannot reach `join` at all now — `Membership::verified`
is the only constructor — so the only record a repeated join could write over a retained
one is byte-for-byte identical to it. The spec says this, because it is the reason
constraining the answers rather than the verb loses nothing: there is no observable
difference for the write verb to make. That is the entry's addition promoted from a note
to the requirement's stated reasoning.

The idempotence scenario gains one assertion to match: the retained record still verifies
against the address it is retained under, after the repeated join.

**One thing I added that the entry did not raise, from the same reasoning.** The
requirement now states that the reply **MUST NOT** be required to say whether the join
was new. The entry notes `join_stoa` discards `Joined` and the reply carries no
"was this new" flag *by design* — but that was a design fact with no contract behind it,
so the next person to want an observable form of item 2 would reach for exactly that flag.
It is the wrong fix: it would invite a view to treat a second join as a failure, which is
the behaviour this requirement exists to prevent. Better to forbid it in the contract than
to leave it as the obvious available move.
