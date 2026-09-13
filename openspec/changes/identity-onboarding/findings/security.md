# identity-onboarding — security review

**Dimension reviewed: SECURITY ONLY.** Correctness in general, readability and
architecture are other reviewers'. Findings below are limited to: peer- or
disk-supplied data reaching a length/index/allocation, a check skippable by
another call path, reachable panics, secret material escaping, non-constant-time
comparison of secrets, and error messages leaking what a caller should not learn.

Worked in `.claude/worktrees/rev-id-security` at `2f3fccf`. Every scenario below
was reproduced by a temporary test in that worktree; all such edits were reverted
and `git status` is clean.

Baseline before and after: **553 passing** (`cargo test -p dialectica -p
dialectica-core`).

---

- [x] **S1 — A hand-edited `path` column derives a *working* identity that is not the user's, and nothing refuses it (HIGH)**

**For: `dev-writer`** (and `spec-writer` for the range requirement it needs).

`identity_store.rs:423-428` (`path_from_row`) refuses a stored path outside
`u32`, and the three `NO SPEC:` markers argue at length that refusing beats
coercing *because a coerced path derives a valid key*. That argument is right and
the guard it justifies is **too narrow**: it bounds the path to `u32`, when the
value this build can actually write is bounded to `u32 & 0x7fff_ffff` by
`onboarding.rs:167`. Every path in `[2^31, 2^32)` is inside `u32`, outside what
any slate can offer, and accepted silently.

The masking asked about in the brief **is applied at exactly one call site** —
`derive_path` in `onboarding.rs:167`. It is applied nowhere on the read-back
path. `identity_store.rs:340-354` (`path_for`) and `:371-408` (`all_paths`) hand
an unmasked `u32` to `wire.rs:700`, which calls
`keystore.stoa_public_key_at_path(stoa, path)` →
`identity.rs:424` `derive_stoa_key_at_path`. That function has no range
precondition, so the hardened-range value derives a perfectly good Ed25519 key.

Worse, the guard is narrower still than "paths a slate offers": **any** in-range
path is accepted, including ones no nonce could produce. The suite itself relies
on this — `wire.rs:2470-2471` records paths `1` and `2` by hand, values
`derive_path` produces with probability ~2⁻³¹ each. So there is no notion
anywhere of "a path this build could have written", which is the property the
`NO SPEC:` markers' own reasoning requires.

### Reproduction (measured)

Keep candidate 0 normally, then `UPDATE chosen_paths SET path = ?` and ask
`whoami_for`:

| tampered `path` | `whoAmI` result | address reported |
|---|---|---|
| `0x8000_0001` (2147483649) | `hasIdentity: true` | `68d12ac4…367c0d` |
| `1` | `hasIdentity: true` | `4f5d657a…c2bc23` |

Against the kept identity `path 88981780`, address
`ff890771…169591`. In both rows the reported address **differs from the one the
user chose** and no error is surfaced anywhere. Both tampered paths derive keys
that sign and verify, so the user posts under an identity they never picked —
the outcome the spec calls unrecoverable, "because the choice cannot be
recomputed".

Threat model note: this is not only the hand-edit case the markers anticipate. A
restored-from-backup, a file-synced, or a same-uid-process-written
`identity.sqlite` reaches the same state, and unlike the keystore there is **no
permission or directory check on this file at all** (contrast
`keystore.rs:1104-1145` `read_checked`, which checks the file mode on the handle
*and* the containing directory). `IdentityStore::open` is
`Connection::open` plus a version/column check and nothing else.

### What is needed

A `path` read back from disk must be refused unless it is a value this build's
derivation can produce — at minimum `path < 0x8000_0000`, applied in
`path_from_row` so both read sites inherit it (the "a guard is a job" shape that
function already exists to provide). The spec needs a requirement stating the
recorded path's admissible range; today it states none, which is why
`path_from_row`'s bound was free to be the wrong one.

**Fixed.** The finding's diagnosis is exactly right, including that the reason
the bound was wrong is that the mask and the guard were two rules. So the fix is
not "change `u32::MAX` to `0x8000_0000`" — that would leave two rules that
happen to agree today. `onboarding::PATH_LIMIT` is now the single constant:
`derive_path` masks with `PATH_LIMIT - 1` and `path_from_row` refuses
`>= PATH_LIMIT`, so widening one widens the other because there is one thing to
change.

The guard is applied on the **write** side too (`record_path`), which the finding
did not ask for. Read-only would leave this build able to write a row it then
refuses to read, and with no migration path by design that is a store bricked by
its own writer.

Three tests, each watched failing first:

- `a_stored_path_this_build_could_not_have_written_is_refused` — `PATH_LIMIT`,
  `PATH_LIMIT + 1` and `u32::MAX` through both `path_for` and `all_paths`, plus
  `PATH_LIMIT - 1` still reading back so a guard refusing everything fails too.
  Measured failing before the fix: *"path 2147483648 is above the writable range
  and was not refused by path_for"*.
- `recording_a_path_outside_the_writable_range_is_refused_and_stores_nothing` —
  the write half, asserting the record is still empty afterwards rather than only
  that an error came back.
- `every_path_a_slate_can_offer_is_inside_the_range_the_record_accepts` — 400
  paths across four nonces asserted against `path_from_row` directly. This is the
  one that pins the two rules *to each other* rather than each to a literal, which
  is the property that was missing. 400 rather than one slate's five because a
  digest's top bit is set about half the time.

**Not claimed, and said so in `path_from_row`'s doc comment:** this bounds a path
to the range a slate *can* offer, not to the five a particular nonce *did*. Those
five are not knowable at this layer — the nonce is deliberately not stored — and
the finding's own observation that the suite records paths `1` and `2` by hand is
why. "A value this build's derivation could have produced" is the strongest
property available here.

**For `spec-writer`:** the finding's last sentence is a live spec gap — there is
no requirement stating the recorded path's admissible range, and the bound above
is therefore a dev choice. It is not marked `NO SPEC:` because it is not a silence
the code chose to fill arbitrarily; it is the mask's range, which the spec does
constrain indirectly. Recorded in `design.md` under Decisions.

---

- [x] **S2 — `getCapabilities` reports a *different* author address than `whoAmI` and than the identity the user kept (HIGH)**

**For: `spec-writer` first, then `dev-writer`.**

`dialectica/rust-lib/src/lib.rs:382`:

```rust
core::keystore::open_from_env(&path).map(|ks| ks.stoa_address(stoa).to_hex())
```

`Keystore::stoa_address` (`keystore.rs:712`) is the **pathless** two-input
derivation, salt `/dialectica/1/Identity/Stoa` (`identity.rs:67`). Meanwhile
`whoami_for` (`wire.rs:700`) and `keep_selection` (`wire.rs:544-553`) report the
**path-taking** derivation, salt `/dialectica/2/Identity/Stoa`
(`identity.rs:87`).

The salt bump is deliberate and correct — `identity.rs:69-87` argues for it
precisely so "one scheme's identities cannot be silently reproduced by the
other". The consequence nobody appears to have followed through is that the two
schemes now yield **two different identities for one Stoa**, and two shipped
methods report one each.

`proposal.md:30` records the decision as "`getCapabilities` gains no new shape
and **its derivation is unchanged**; it begins answering `canPost: true` because
a keystore now exists", and `proposal.md:58` lists `posting-capability` as "not
modified, deliberately — the probe's shape, its reasons and its derivation are
untouched". Leaving the derivation untouched is exactly what creates the split.

### Reproduction (measured)

Keep candidate 0, then reproduce `lib.rs`'s closure against the written keystore:

```
keep / whoAmI address :  6de4ee4bf902810e22e8437cc596997b8c9ff44e9faf6a29c97354caee947a33
getCapabilities identity: 656c6003a040b37bda69ac1c34d35de81d6c193e8f571dedffb8845d2355e153
```

### Why this is a security finding and not a cosmetic one

- The modified `identity` spec (`specs/identity/spec.md:27-29`) requires "A user
  SHALL have exactly one identity within a Stoa". Two methods reporting two
  addresses is that requirement violated on the wire.
- `getCapabilities` is described in `lib.rs:99-102` as the gate every posting
  affordance in the view hangs on, and its `identity` field is what a view would
  label the author with. A user shown one address by the onboarding flow and
  another by the posting gate cannot tell which one an op will be attributed to.
- The pathless identity's path is **not recorded anywhere**, so if anything is
  ever signed under it, `identity-onboarding`'s whole recorded-path recovery
  story does not cover it.

### Why the suite cannot see it

`get_capabilities` takes the lookup as a closure (`wire.rs:201-204`) and every
test supplies a stub returning a literal (e.g. `wire.rs:3512` `|_| Ok("abcd")`).
The real derivation is only chosen in `lib.rs`, which `cargo test` cannot compile
at all (it is behind `#[cfg(logos_scaffold)]` — `lib.rs:183-211` explains why).
So no test in the project compares the probe's identity to `whoAmI`'s, and none
can as currently shaped. A test asserting the two agree has to live in `core`,
which means the choice of derivation has to move out of `lib.rs`.

**Fixed.** The last paragraph is the fix, and it was followed exactly: the choice of
derivation moved out of `lib.rs` into `core::wire::posting_identity`, and the test
asserting the two agree now lives in `core`.

`getCapabilities` consults the path record and derives at the recorded path.
`get_capabilities_from_stores` is the shape the adapter forwards to — two openers in,
the derivation decided inside — and the adapter keeps only the part that cannot move,
which is where the directory is.

Two consequences taken on purpose, both recorded in `design.md`:

- **A master key with no recorded choice for this Stoa is now `canPost: false`**,
  with a reason naming the missing choice. It was `true`, with a pathless address —
  which, as the finding says, asserted posting ability for an identity whose path is
  recorded nowhere. `whoAmI` reports the same state through the same
  `NO_CHOICE_FOR_THIS_STOA` constant, so the two methods cannot describe one
  situation in two vocabularies.
- **The lookup's error widened to `String`.** Adding an `Other(String)` arm to
  `KeystoreError` so it could keep that type was the obvious alternative and is
  rejected: those arms are documented as distinguishable *so a reason can name a
  fix*, and a catch-all carrying another module's failure is the collapse that
  doctrine exists to prevent.

Two tests, both asserting between methods rather than against a constant:

- `the_probe_and_whoami_report_the_same_identity_for_one_user_and_stoa` — and its
  strongest assertion is not the comparison. It signs an op with the key at the
  recorded path and requires it to verify against the address the probe reported,
  which is `posting-capability`'s own wording ("derived from the key that would
  actually sign it") rather than two derivations that could both be wrong.
- `the_probe_and_whoami_give_one_reason_when_no_choice_is_recorded_for_this_stoa`.

Measured: returning `keystore.stoa_address(stoa)` — the old pathless call — fails the
first test and **nothing else in 563**, which confirms this finding's "why the suite
cannot see it" section exactly.

**For `spec-writer`, and not closed by this fix.** The finding is addressed to
`spec-writer` *first*, and rightly. `proposal.md:30` and `:58` still say
`posting-capability` is "not modified, deliberately — the probe's shape, its reasons
and its derivation are untouched". Its derivation is now modified, and the paragraph
that declares otherwise is how this went unnoticed: the distinction it exists to draw
is the one it got backwards. That is a proposal correction I have not made, because
the proposal is `spec-writer`'s file.

---

- [x] **S3 — On a fresh install the candidate the user picks is not the identity they get (HIGH)**

**For: `dev-writer`.**

`lib.rs:306-317` (`Dialectica::master_key`) returns
`Keystore::generate()` whenever no keystore exists. Both
`generate_identity_slate` (`lib.rs:404-424`) and `keep_identity`
(`lib.rs:434-437`) call it. On a fresh install — the only install onboarding
exists for — the slate call mints master key *A* and the keep call mints master
key *B*.

`lib.rs:294-301` names this and argues it is harmless:

> *"two `generateIdentitySlate` calls on a fresh install offer candidates of two
> different master keys … What makes it harmless is that the keep mints its own
> key too and writes THAT one, so the identity kept is always one of the
> candidates of the key that was stored — never a candidate of a key that was
> discarded."*

The premise is true and the conclusion does not follow. "A candidate of the key
that was stored" is not "the candidate the user looked at". The nonce reproduces
the same five *paths* (paths depend on the nonce alone —
`onboarding.rs:159-168`, asserted at `onboarding.rs:516-518`), so index 0 still
resolves, but to a **different key, public key and address** under master key
*B*. The `live_slate` nonce check cannot catch it, as the comment itself says.

### Reproduction (measured)

Slate under one fresh `Keystore::generate()`, keep index 0 under another, same
nonce:

```
view showed candidate 0 as : 3790ac03e8e89a33dd4f9b1ffbf5532e5d5b19d64f486822d4eaab2310ca2e4b
keep stored               : a2fa76492307461c1a55ef6651bb9e9ec718ae12004f3265e8fa77c869816169  (path 948714228)
```

`kept: true` is returned with the address the user never saw.

### Why this belongs in the security dimension

`spec.md:363-366` puts the rule as: coercing a selection "would store an identity
the user did not choose — which is unrecoverable, because the choice cannot be
recomputed". This is that outcome reached without any coercion: the index is
honoured, the nonce matches, the refusal machinery all fires correctly, and the
identity stored is still not the chosen one. Every guard in
`keep_selection` (`wire.rs:494-553`) is about the *selection*; none is about the
master key the selection was made against.

The minimal shape that closes it: the slate reply must commit to the master key
it was derived under (the nonce alone is not sufficient), and the keep must
refuse when the key it is about to write is not that one — or the fresh key must
be minted once and held for the lifetime of the live slate, which is the state
`live_slate` already spans.

**Fixed** — the second of the two shapes this finding offers, and the finding's own
framing is why. It ends with both options as though they were alternatives of equal
standing; they are not, and choosing between them is the one substantive judgement
in this fix.

**Committing to the key detects the divergence without repairing it.** The keep
would refuse, correctly, and the user would be unable to do anything about it: the
key that produced their slate was dropped when that handler returned. A refusal the
user cannot act on is a worse outcome than the one it replaces only in the sense
that it is honest — it still does not give them the identity they chose.

So the key is minted once and held, in `core::wire::OnboardingSession`, which holds
it alongside the nonce precisely because — as this finding says — *"`live_slate`
already spans"* that lifetime. The two are one value: a slate is a
`(master key, nonce)` pair, and holding half of it was the defect.

**What holding costs, stated rather than glossed:** a root secret in memory for the
module's lifetime instead of one call. That is the same lifetime a *kept* keystore's
root already has, so the window widens only on the fresh-install path and only until
the user keeps or the module stops. Nothing is written — the mint writes no file,
which the spec requires of a slate — and `Keystore`'s root is `Zeroizing`.

Regression test:
`on_a_fresh_install_the_identity_kept_is_the_candidate_the_slate_showed`, which
supplies **no** master key (its opener returns `NotFound`, so minting happens) and
asserts a relationship between the two replies. It also re-opens the written keystore
and checks it derives the shown address at the recorded path, so a session that
reported its held key while writing another is caught too. Measured failing before
the fix with this finding's signature: same path, different addresses, `kept: true`.

Restoring the per-call mint fails that test plus two others and **nothing else in
563**.

---

- [ ] **S4 — `check_layout` does not prove the PRIMARY KEY, so "one path per Stoa" is not structural on read-back (MEDIUM)**

**For: `dev-writer`, with a scenario for `tester`.**

`identity_store.rs:251-270` documents `stoa BLOB PRIMARY KEY` as what makes "one
chosen path per Stoa" *"a property of the schema rather than of a guard at each
write"*, and `record_path`'s doc (`:301-313`) rests the second-keep refusal on
it. That holds for writes this build makes. It does not survive the file being
replaced, because `check_layout` (`:238-249`) runs
`SELECT stoa, path FROM chosen_paths LIMIT 0` — which proves the two column
*names* exist and nothing about keys or constraints. Its own doc comment
anticipates the analogous gap one level up ("`SELECT 1` proves a name exists and
nothing about its shape") and then stops one step short.

### Reproduction (measured)

```
second INSERT on the intact table -> Err(ConstraintViolation, "UNIQUE constraint failed: chosen_paths.stoa")   [correct]

then: DROP TABLE chosen_paths;
      CREATE TABLE chosen_paths (stoa BLOB NOT NULL, path INTEGER NOT NULL);
      INSERT ... VALUES (stoa,100),(stoa,200);

IdentityStore::open  -> Ok                      (accepted)
path_for(stoa)       -> Ok(Some(100))           (silently picks one of two)
all_paths()          -> two ChosenPath rows for the SAME Stoa
```

`path_for` uses `query_row(...).optional()`, which takes whichever row SQLite
returns first and discards the rest without comment — so the identity reported
depends on physical row order, not on anything the user chose. `all_paths` is
documented as the export path (`:356-370`); an export carrying two contradictory
paths for one Stoa is an export that cannot be restored unambiguously.

Note this is the same class as S1 (disk content trusted further than the module's
own invariants) and the same remediation site: the refusal belongs in the read
path, not only in the schema.

`LAYOUT_VERSION` is pinned by a hardcoded assertion at `:807-814` with the right
reasoning (`cargo mutants` does not mutate `const`s) — that part is correct and
worth keeping as the model for whatever guard S1 and S4 add.

**OPEN — not fixed, and this box stays empty deliberately.** The finding is correct
in every particular; I re-read `check_layout` and `path_for` and confirmed the
mechanism. `LIMIT 0` proves two column *names* and nothing about keys, so a replaced
file whose `chosen_paths` lacks the `PRIMARY KEY` opens `Ok`, and
`query_row(...).optional()` then returns whichever row SQLite hands back first — an
identity selected by physical row order.

**Why it is not fixed here rather than why it does not matter:**

- It is a **different fix from S1's**, despite being the same disk-content family.
  S1's is a value range, checkable with a comparison. This one needs a constraint
  check at open — reading `sqlite_master`'s DDL or `PRAGMA index_list`, deciding what
  counts as "the right constraint", and deciding what a legitimate older file may
  look like. That is a schema-verification design, not a guard.
- The commit that closes it should also decide about `all_paths` returning two rows
  for one Stoa, which the finding notes makes an export unrestorable. That is a
  second behaviour question in the same area.
- This change's blast radius is already large. A schema check bolted on at the end,
  in the commit that also reshapes the session and moves two derivations, is the
  diff nobody can review for either.

**Where it now lives, so it is deferred rather than dropped:** `design.md`'s
Risks/Trade-offs carries it, in the entry that replaces the old (and false)
"`identity.sqlite` has no `check_layout` equivalent". That entry states what
`check_layout` does prove, what it does not, and the measured consequence — so the
next reader knows the one-path-per-Stoa invariant holds for files *this build* wrote
and not for every file it will open.

It is left unticked because that is a record, not a fix, and a ticked box here would
tell a reviewer the read path refuses a constraint-less table. It does not.

---

- [x] **S5 — `onboarding.rs:266-270`'s zeroize comment claims coverage the code does not have (MEDIUM)**

**For: `dev-writer`.**

The comment above the `Zeroizing` in `Slate::from_nonce` says:

> *"`SecretKey` itself has no `to_bytes` call here — there is no plain copy of
> the seed to wipe, because none is made."*

The next line is `Zeroizing::new(candidate_key(master_key, stoa, path).to_bytes())`
— `onboarding.rs:271-272`. `to_bytes()` **is** called, and it returns
`[u8; 32]` by value (`identity.rs:319-321`). The temporary is moved straight into
the wrapper, which is the correct shape and is exactly the one
`keystore.rs:629-664` argues for at length; the comment describes a *different*
correct shape and so reads as a guarantee about something else. A later reader
who trusts the sentence and binds a local first would be told by this comment
that no local exists.

The substantive half is one layer down, and it is not covered by anything:
`identity.rs:424-433` `derive_stoa_key_at_path` builds

```rust
let mut info = [0u8; 36];
let mut seed = [0u8; 32];
hk.expand(&info, &mut seed)...
```

`seed` is the derived per-Stoa secret seed, on the stack, **never zeroized** —
`identity.rs` has no `zeroize` import at all (grepped). `identity.rs:266-282`
documents this omission deliberately and names the mitigation: *"those copies are
`crate::keystore`'s to own, and it does — it moves what `SecretKey::to_bytes`
returns straight into a `Zeroizing`"*. That mitigation covers the **root**, not
this `seed`, and it was written when `derive_stoa_key` ran at keystore setup. It
now runs **five times per `generateIdentitySlate` call**, on a method a caller
invokes repeatedly and without limit (`onboarding.rs:542-556` pins that
regeneration is deliberately unbounded — 200 rounds in the test, so 1000 unwiped
seeds).

This is a residual-memory finding, not a reachable leak: nothing reads those
bytes back, and `identity.rs` is honest that memory lifetime is owned one layer
up. What is wrong is that the layer up no longer owns it for this path, and the
`onboarding.rs` comment asserts it does. Either `derive_stoa_key_at_path` should
wipe its own `seed`, or the two comments should stop claiming the obligation is
discharged.

**Fixed — the second of the two remedies, and the first is deferred with a place to
live.** The finding offers them as alternatives; they close different halves, and
only one of them is this change's business.

**The comment is fixed**, which is the half the finding is actually about. Its own
framing makes the case: the sentence describes *"a different correct shape"*, so *"a
later reader who trusts the sentence and binds a local first would be told by this
comment that no local exists."* It now says `to_bytes()` **is** called and that what
makes it safe is the absence of an intermediate binding — the temporary is consumed
by the `Zeroizing`, which is `Keystore::generate`'s shape and argued at length there
after review found that deleting an explicit wipe left the whole suite green. The
comment also now names what it does **not** cover, which is the other half.

**`derive_stoa_key_at_path`'s `seed` is still unwiped**, and that is recorded in
`design.md`'s Risks rather than fixed. The reason is scope: `zeroize` would have to
reach into `identity.rs`, which has no such import and whose stated posture is that
it holds no memory obligations — *"those copies are `crate::keystore`'s to own"*.
Changing that is a change to `identity`'s contract about who owns secret lifetime,
and it belongs in a change that says so. The finding's own assessment is that this is
residual memory and not a reachable leak, which is why deferring the code half and
fixing the comment half is the right split rather than a convenient one.

The `design.md` entry records the part the finding makes that is easy to lose: the
deferral to `keystore` was written when `derive_stoa_key` ran **once at setup**, and
it now runs five times per slate on a method whose unbounded regeneration is itself a
pinned requirement — so 200 rounds in a test is 1000 unwiped seeds.

**No test.** A stack local after its function returns is not observable from a test;
`keystore.rs`'s own review established that, and it is the same reason task 3.5 is
deliberately untested. I am not writing one that pretends otherwise.

---

- [x] **S6 — `parse_index`'s `v as usize` truncates on a 32-bit target (LOW)**

**For: `dev-writer`.**

`wire.rs:918-933`, line 926: `Some(v) => Ok(Some(v as usize))` where `v: u64`.
On a 64-bit target this is lossless and the finding is theoretical. On a 32-bit
target `as usize` truncates silently, so `{"index": 4294967296}` becomes `0` and
**keeps candidate 0** — precisely the coercion `spec.md:363-369` forbids
("SHALL be refused, and SHALL NOT be satisfied by any other candidate"). The
existing test at `wire.rs:2289-2308` covers `-1`, `1.5` and `"two"` but no value
above `usize::MAX`-on-32-bit.

A `usize::try_from(v)` with an explicit refusal costs nothing and makes the
property hold on every target rather than on the one currently shipped.
`dialectica` targets Linux x86-64, which is why this is LOW and not higher.

**Fixed** — `usize::try_from` with an explicit refusal, exactly as recommended.
Correctness review raised the same thing independently as its closing note, which is
part of why it was worth taking now rather than leaving as a comment for a future
port.

The sentence that decided it is *"makes the property hold on every target rather than
on the one currently shipped"*. The refusal is unreachable on any target CI builds,
so its value is not that it catches something — it is that the guarantee stops being a
consequence of `usize` happening to be 64 bits and becomes a decision in the source.

**No test, and none is possible on a 64-bit host**: `usize::try_from(u64)` is
infallible there, so the refusal branch cannot be reached. Not ticking this as
"fixed with a test" — the existing `parse_index` test still covers `-1`, `1.5` and
`"two"`, and `no_onboarding_handler_panics_whatever_it_is_given_or_whatever_fails`
carries `18446744073709551616` (which exceeds `u64` and is refused earlier, by
`as_u64`). Cross-compiling to a 32-bit target to reach it is a CI change, not a test.

While in that function I also fixed readability's R3, which is about the same lines:
the doc comment named two callers when there are three, and argued the refusal
entirely in pagination terms — so a reader arriving from `keep_identity` was told
about serving the wrong page of a feed rather than about storing an identity nobody
chose.

---

## Areas examined and found clean

Stated plainly rather than padded into findings.

- **No build-time passphrase constant exists.** Grepped the whole tree for
  `DIALECTICA_PASSPHRASE`: it appears only in `keystore.rs` (the `const` naming
  the env var, plus `Display` text), `wire.rs` tests, and three spec/design
  documents. Nothing in `metadata.json`, `scaffold.toml`, `flake.nix`, the UI, or
  any `.rs` supplies a fallback. `protection_from_env` (`keystore.rs:365-370`) has
  no default arm — absent or empty means `Unlock::Unencrypted`, matching
  `unlock_from_env`'s treatment (`:317-330`) and `ssh-keygen`/radicle. The
  spec's prohibition (`spec.md:333-339`) is met by construction.

- **The reported protection cannot disagree with the stored one.**
  `keep_selection` computes `encrypted` from the `Unlock` it actually wrote under
  (`wire.rs:548-553`), not by re-reading the file, and the comment gives the right
  reason (re-reading reports whatever is at the path *now*). `to_file_bytes`
  (`keystore.rs:822-875`) writes the matching `Protection` byte in the same call
  with the same value. `wire.rs:2227-2254` checks both directions *and*
  cross-checks against `Keystore::is_encrypted` on the written file.

- **No secret reaches a slate reply.** `slate_json` (`wire.rs:340-359`) emits only
  `index`, `path`, `address`, `publicKey`. `Candidate` (`onboarding.rs:184-196`)
  has no secret field. The two tests at `onboarding.rs:663-773` are the strong
  form: a byte-window search for the master key and each candidate secret across
  everything exposed, *and* a check that every 32-byte value in the reply, taken
  as key material, signs as no candidate — with working positive controls on both,
  and a master key deliberately distinct from the nonce so a hit cannot be
  misattributed. `SlateNonce` is 32 bytes derived from the OS CSPRNG and
  independent of the master key; `Slate` is not `Clone`, `SecretKey` is not
  `Clone`/`Debug`/`Serialize`.

- **`SLATE_SIZE = 5` leaks nothing.** `derive_path` (`onboarding.rs:159-168`)
  takes only the nonce and the index, so the five paths — and the index-walk's
  skip behaviour, which is the only data-dependent control flow in
  `from_nonce` — are functions of public randomness alone and independent of the
  master key. `onboarding.rs:516-518` asserts exactly this (identical paths across
  three different master keys and Stoas). The count is not caller-supplied: there
  is no field for it (`wire.rs:266-296`), which is stronger than validating one.
  The user's eventual choice is not leaked by the slate because the slate is
  generated before any choice exists and nothing about it is written
  (`generate_identity_slate` takes no store).

- **No reachable panic in the onboarding path.** `onboarding.rs` non-test code has
  no `unwrap`, `expect`, `panic!`, indexing or arithmetic that can overflow;
  `SlateNonce::generate` returns a `Result` for the randomness failure
  (`:100-104`) rather than `SecretKey::generate`'s `expect`, and the reasoning for
  the split is correct. `MAX_PATH_WALK` (`:78`) bounds the only loop, so the
  unbounded-iteration DoS is closed. `identity_store.rs` non-test code has no
  `unwrap`/`expect`/indexing; every `rusqlite` failure becomes an error.
  `NoSuchCandidate`'s `Display` uses `saturating_sub` (`:388`) so a zero-length
  slate cannot underflow in a formatter. `from_hex` on both `SlateNonce` and
  `Address` is strict on hex validity and exact length, with the one-too-long case
  covered (`onboarding.rs:655-659`).

- **Truncated / arbitrary / hostile store content does not panic.**
  `identity_store.rs:643-696` sweeps empty, 1-byte, 0xff-filled, fake-header and
  all-256-bytes content plus truncation at five offsets, and — the load-bearing
  part — calls `path_for` *and* `all_paths` on any file that opens, because SQLite
  treats a zero-length file as a fresh database. The stored Stoa blob's length is
  `try_into`'d with a refusal rather than assumed (`:394-400`). Verified by
  running the suite, not by reading.

- **No non-constant-time comparison of secret material.** The keystore's only
  secret-dependent check is the AEAD tag (`keystore.rs:916-924`), which is
  constant-time, and `:909-915` records why no separate stored verifier exists.
  `SlateNonce`'s derived `PartialEq` compares 32 bytes of *public* randomness, so
  its non-constant-time comparison is not a finding: a timing oracle on the live
  nonce reveals a value the module hands out in plaintext in the slate reply.

- **KDF parameters read from the file are bounded before use.**
  `derive_key_with` (`keystore.rs:1001-1033`) checks each knob and then the
  `m_cost × t_cost` product against `MAX_WORK_FACTOR`, *before* `Params::new`, with
  the right reason recorded (the allocation happens in `hash_password_into`, and
  an OOM kill is a module death with no panic for `guarded` to catch). The
  ceiling-before-construction ordering and the separate product check are both
  correct.

- **No error message leaks what a caller should not learn.** `KeystoreError`
  (`keystore.rs:439-510`) carries no key material, ciphertext or passphrase; modes
  are public `stat` data. `IdentityStoreError::PathOutOfRange` carries the Stoa
  hex and the `i64` found — both public per the spec's "reveals nothing that a
  published identity does not already reveal". `Storage(String)` carries SQLite's
  message, which names no derivation-path content.

- **The UI consumes none of this yet**, so there is no view-side leak to review:
  grepping `dialectica-ui/src/qml/` for `slate`/`keepIdentity`/`whoAmI` returns
  only false positives (`ctx.translate` in `Identicon.qml`).

- **Panic guard coverage.** All four onboarding-relevant handlers are inside
  `guarded` (`wire.rs:302, 455, 628`, plus `205`). The adapter code *outside* the
  guard in `lib.rs` (`storage_dir`, the `Cell`, `master_key`,
  `IdentityStore::open`, `protection_from_env`) is panic-free on inspection, but
  note it is structurally unguarded — `delivery_channel_exists` puts its
  cross-module call *inside* `core::guarded` for exactly this reason
  (`lib.rs:348`), and `keep_identity`/`who_am_i` do not follow that pattern. Not
  filed as a finding because no panic is reachable there today; flagged so the
  asymmetry is a choice rather than an oversight.

---

## Measurements and tooling notes

- **Suite:** 553 passing before and after, via
  `cargo test --manifest-path …/dialectica/rust-lib/Cargo.toml -p dialectica -p dialectica-core`.
- **`cargo mutants`: abandoned, no result.** Two filter forms found 0 mutants
  (`--file dialectica-core/src/onboarding.rs` against the workspace manifest, and
  `--file src/onboarding.rs` against the crate manifest); the form that works is
  the crate manifest with the **workspace-relative** path
  (`--file dialectica-core/src/onboarding.rs`), which found 36 mutants. It cleared
  the baseline (61s build + 86s test) and auto-set a 433s per-mutant timeout, so
  36 mutants projected well past the couple of minutes this review budgets for it,
  and it was stopped with no mutant results. **Nothing in this report rests on a
  mutation score.** Recording the working invocation so the next reviewer does not
  re-derive it:
  `cargo mutants --manifest-path <worktree>/dialectica/rust-lib/dialectica-core/Cargo.toml --file dialectica-core/src/onboarding.rs`
  Also worth noting for whatever guard S1 needs: `cargo mutants` mutates functions
  and not `const`s, so a changed mask or bound is invisible to it — the hardcoded
  assertions at `identity_store.rs:807-814` and `onboarding.rs:412-440` are the
  right pattern and a new bound needs one too.
- **No new dependencies** are introduced by this change's identity files:
  `zeroize`, `hkdf`, `sha2`, `ed25519-dalek`, `rusqlite`, `getrandom`, `hex` and
  `argon2` are all pre-existing. Nothing to assess for licence compatibility.
- **Tree state:** every probe edit was to `wire.rs`'s test module and was reverted
  with `git checkout --`; `git status --short` is empty. `cargo mutants` operates
  on its own copy under `target/mutants.out` and left the worktree untouched
  (verified after stopping it).
