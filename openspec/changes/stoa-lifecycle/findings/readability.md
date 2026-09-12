# Readability findings — stoa-lifecycle

Reviewer: `code-reviewer`, **readability dimension only**. Correctness, security
and architecture were not reviewed; other instances hold those.

Scope read in full: `dialectica-core/src/membership.rs` (all 1636 lines,
including tests), the `create_stoa` / `join_stoa` / `list_stoas` / `stoa_reply` /
`policy_name` / `with_membership_store` / `parse_stoa` / `membership_page_json` /
`membership_path_in` handlers in `dialectica-core/src/wire.rs`, the
identity-key methods in `dialectica-core/src/keystore.rs`, and the adapter
handlers in `dialectica/rust-lib/src/lib.rs`.

**False-premise comments found and verified: 4** (entries 1, 2, 3, 4). Each was
checked against the code it describes, not inferred from its wording; entry 1 was
verified by running the code.

Tree was clean when I started, and is clean now. One temporary probe test was
added to `wire.rs` and removed; `git status --short` is empty and the 550-test
core suite passes.

---

## 1. `create_stoa`'s doc credits `join`'s encode-before-write ordering with a refusal `join` never sees — FALSE PREMISE, verified by running it

**For:** `dev-writer`

The doc heading "The title is refused before anything is recorded"
(`dialectica/rust-lib/dialectica-core/src/wire.rs:587-592`) names its mechanism
explicitly:

> "`Genesis::canonical_bytes` is fallible for a title over the genesis cap, and
> `MembershipStore::join` encodes before it writes — so an over-long title
> returns before any statement runs. That ordering is what makes 'a failed
> creation leaves nothing behind' structural rather than a rule to remember."

**The claim:** `MembershipStore::join`'s encode-first ordering is what refuses an
over-long title in `create_stoa`.

**What contradicts it:** `create_stoa` never reaches `join` for an over-long
title. `dialectica/rust-lib/dialectica-core/src/wire.rs:642-645` calls
`genesis.address()` and returns `error_json(&format!("title: {e}"))` on failure,
and `join` is only called at line 651, after that early return.
`Genesis::address` is itself `Ok(stoa_address(&self.canonical_bytes()?))`
(`dialectica-core/src/stoa.rs:323-325`), so the cap is enforced one call earlier
and in a different module from the one the comment credits.

**Measurement.** I added a temporary probe test in the `wire.rs` test module that
panicked with the reply for a 1025-byte title, and removed it afterwards. The
reply was:

```
{"error":"title: title is 1025 bytes, the maximum is 1024"}
```

That is `create_stoa`'s own `"title: {e}"` prefix
(`dialectica-core/src/wire.rs:644`). `join`'s refusal renders as
`"that genesis record cannot be encoded, so it names no Stoa: …"`
(`dialectica-core/src/membership.rs:200-205`) and never appeared. So the two
paths are distinguishable by their message, and the one the comment names is not
the one taken.

This is the same shape as the version-independence premise already fixed on this
piece: a real guarantee credited to the wrong mechanism. The behaviour is
correct; the explanation sends a reader editing `join` to preserve a property
that `create_stoa` actually holds on its own. It also makes
`MembershipError::UnencodableRecord` look wire-reachable when it is not — see
entry 5.

**Outcome:**

---

## 2. `with_membership_store` says all three handlers take `&mut`; `list_stoas` takes `&` — FALSE PREMISE

**For:** `dev-writer`

`dialectica/rust-lib/dialectica-core/src/wire.rs:757-760`:

> "The three handlers take `&mut MembershipStore` because that is the shape worth
> testing"

**What contradicts it:** `list_stoas` is declared
`pub fn list_stoas(request: &str, store: &crate::membership::MembershipStore)`
at `dialectica/rust-lib/dialectica-core/src/wire.rs:727` — a shared reference.
Only `create_stoa` (`wire.rs:609`) and `join_stoa` (`wire.rs:680`) take `&mut`.
The call compiles because `with_membership_store` hands the closure a
`&mut MembershipStore` and `&mut T` coerces to `&T`, so nothing fails; the
sentence is simply not true of three of three.

Worth fixing rather than ignoring because the sentence is doing real work: it is
the stated reason the handlers do not take an opener closure, and a reader
checking that reason against the signatures finds it wrong at the first one they
look at — which discredits the surrounding argument, which is sound.

**Measurement:** not applicable; this is a signature a reader compares against a
sentence. `grep -n "pub fn list_stoas" wire.rs` is the whole check.

**Outcome:**

---

## 3. `parse_stoa`'s justification is contradicted three times in its own file — FALSE PREMISE

**For:** `dev-writer`

`dialectica/rust-lib/dialectica-core/src/wire.rs:806-811`:

> "Factored out because create does not take one and both of the other two do,
> and because the three-way distinction — absent, wrong-typed, not an address —
> is the one `module-wire-contract` requires be reported by name. **A second copy
> would eventually disagree with the first** about which of the three it was."

**What contradicts it:** there are already four copies of that three-way parse in
this one file, and `parse_stoa` is the only factored one. The inline copies are
at `dialectica-core/src/wire.rs:210-217` (`get_capabilities`),
`:345-352` (`list_threads_inner`) and `:423-430`
(`list_threads_from_request`). `grep -n "missing field: stoa" wire.rs` returns
lines 216, 351, 429 and 818.

The three inline copies predate this change — `git show
origin/main:dialectica/rust-lib/dialectica-core/src/wire.rs | grep -c "missing
field: stoa"` returns `3` — so the duplication is inherited, not introduced. What
this change introduced is a comment asserting that the factoring prevents
divergence, in a file where divergence is already four-way possible. A reader who
believes the comment will not think to check the other three; a reader who checks
loses confidence in the comment.

This is also exactly CLAUDE.md's stated signal — "when you find yourself writing
the fourth slightly-different copy of a guard, that is the signal to reshape
rather than to add a fourth test." This change wrote the fourth copy as a helper
and left the other three, which is half the reshape.

**Measurement:** 4 copies of one three-way parse in one file; 3 of them inline
and unreachable from the new helper. `grep -c` above is the count.

**Outcome:**

---

## 4. The adapter's "every method forwards straight into `core`, one line or it belongs in `core`" is false of six of its own methods — FALSE PREMISE (stale)

**For:** `dev-writer`

`dialectica/rust-lib/src/lib.rs:278-281`:

> "A thin adapter and nothing more. Every method forwards straight into `core`,
> which is where the guard and the decisions live. **If a body here ever grows
> past one line, that logic belongs in `core`** — otherwise it is logic no test
> can reach."

**What contradicts it:** six of the ten trait methods in the `impl` below it have
multi-line bodies — `delivery_channel_exists` (`lib.rs:296-325`),
`get_capabilities` (`:327-348`), `list_threads` (`:350-364`), `create_stoa`
(`:366-390`), `join_stoa` (`:392-400`) and `list_stoas` (`:402-410`). Three of
those six are this change's own additions.

`delivery_channel_exists` is explicitly excused at `lib.rs:297-301` ("the one
place in this file with more than a forwarding line"), which was true when
written and is now false: this change added three more multi-line bodies and did
not update either comment. So the file now carries two comments that disagree
with the file and with each other.

The rule itself is worth keeping — the multi-line bodies are all `storage_dir()`
plumbing that genuinely cannot live in `core` — but the sentence needs to say
what the exception is (deriving a host path and passing it in) rather than
claiming there is one exception when there are six.

**Measurement:** 6 of 10 methods have bodies past one line; 3 added by this
change. The "one place in this file" claim at `lib.rs:297` was accurate against
`origin/main` and is not accurate now.

**Outcome:**

---

## 5. `genesis_for`'s doc says `joinStoa` "does not exist" — stale, `join_stoa` is 380 lines below it and is a caller

**For:** `dev-writer`

`dialectica/rust-lib/dialectica-core/src/wire.rs:294-297`:

> "[`Moderators::of`] needs a genesis record and there is nowhere else to get
> one: §9.1 Stage D is where `joinStoa` records what a peer has joined, **and it
> does not exist.** Until it does, the record travels with the request."

**What contradicts it:** `join_stoa` exists in this same file at
`dialectica-core/src/wire.rs:680`, and it *calls* `genesis_for` at
`wire.rs:694`. Membership is recorded, retained and re-read — that is this whole
change. `grep -rn "Stage D" dialectica-core/src/` returns only this one line, so
nothing else tracks the claim.

The consequence is not cosmetic. The sentence is the recorded justification for
why `list_threads` takes the record as a request parameter rather than looking it
up. With `joinStoa` landed and the record retained per Stoa
(`MembershipStore::get`), that justification has changed from "there is nowhere
else to get one" to "there is now somewhere else, and this capability
deliberately does not read it yet". Those license different follow-up work: a
reader acting on the current text would conclude the lookup is impossible rather
than out of scope.

**Measurement:** 1 stale sentence; the contradicting caller is 386 lines below it
in the same file.

**Outcome:**

---

## 6. `MembershipError::UnencodableRecord`'s doc describes a caller experience no caller can have

**For:** `dev-writer`

`dialectica/rust-lib/dialectica-core/src/membership.rs:145-163` explains the
`UndecodableRecord` → `UnencodableRecord` rename with a concrete scenario:

> "A caller passing a 2000-byte title was told its record 'could not be read:
> title is 2000 bytes'."
>
> "There is no decode-side sibling here, and that is not an omission: a
> membership is joined from a `Genesis` this crate already decoded — `wire.rs`
> owns that decode and reports it"

Both halves read as being about a wire caller, and no wire caller can reach this
variant. `grep -rn "UnencodableRecord" rust-lib/` shows the only construction
site is `membership.rs:401`, and both wire paths refuse an over-long title
earlier: `create_stoa` at `wire.rs:642-645` via `genesis.address()` (proved in
entry 1), and `join_stoa` via `genesis_for`'s `Genesis::decode`
(`wire.rs:319-322`), which enforces the same cap at `stoa.rs:296`. The only
reachable caller is the crate's own test at `membership.rs:1057`.

The variant should stay — it is the honest error for a crate-level caller
building a `Genesis` by hand, and `membership.rs` is a library module whose API is
not only the wire. But the doc should say that is who reaches it. As written, a
reader looking for the user-facing message this text describes will look on the
wire and not find it, and the "wire.rs owns that decode and reports it" clause
reads as an explanation of the decode side when it is in fact the reason the
encode side is unreachable too.

**Measurement:** 1 construction site; 0 wire paths reach it; 1 test does.

**Outcome:**

---

## 7. One user-facing sentence is hardcoded twice in two modules, and only one copy is reachable

**For:** `dev-writer`

`"the genesis record does not hash to the Stoa address it was given with"` is a
literal in two places:

- `dialectica/rust-lib/dialectica-core/src/wire.rs:328` — `genesis_for`'s refusal
- `dialectica/rust-lib/dialectica-core/src/membership.rs:198` —
  `MembershipError::RecordDoesNotMatchAddress`'s `Display`

`join_stoa` calls `genesis_for` (`wire.rs:694`) before `store.join`
(`wire.rs:703`), so a mismatched pair is always refused by the `wire.rs` copy and
the `membership.rs` copy never reaches a wire caller. The test at
`wire.rs:3354-3357` pins the `wire.rs` wording as a hardcoded literal; nothing
pins the two against each other.

The double *check* is justified at `wire.rs:699-702` and I agree with that
argument. What is not written down anywhere is that the two checks also carry two
independently-maintained copies of one sentence. Editing the reachable one leaves
the other silently diverged, and no gate sees it, because the unreachable copy is
only exercised by `membership.rs`'s own tests — which assert on the variant, not
on this literal, everywhere except
`every_error_renders_without_leaking_rust_syntax`.

Either the duplication earns a one-line note at both sites saying which is
reachable from the wire, or the two converge on one constant. I have no
preference between those; what is a defect is that neither comment mentions the
other copy exists.

**Measurement:** 2 hardcoded copies of one sentence, in 2 modules; 1 reachable
from the wire; 0 tests compare them.

**Outcome:**

---

## 8. `membership.rs::list` shadows `offset` with a different type mid-function

**For:** `dev-writer`

`dialectica/rust-lib/dialectica-core/src/membership.rs:521` binds
`let offset = page.saturating_mul(per_page);` (a `usize`), and `:549` rebinds
`let offset = match i64::try_from(offset) { … }` (an `i64`). Between them sits a
32-line comment block and a `prepare` call.

This is a genuine readability cost rather than a preference, because the
surrounding comment (`:532-547`) is specifically *about* the difference between
the two representations — "SQLite's parameters are `i64`, so both have to cross
that boundary" — and the code makes the two sides of that boundary share one
name. A reader checking "which `offset` does the `try_from` consume?" has to
count backwards past the comment to find out. The sibling `limit` at `:548` does
the conversion in one binding and reads cleanly; `offset` could too, or the
`usize` one could be named `row_offset`.

Note this is the only naming issue I found in either file worth raising, and it
is small. Flagging it because the file is otherwise unusually careful about
exactly this kind of thing.

**Measurement:** 28 lines and one statement between the two bindings of one name.

**Outcome:**

---

## Checked and clean

Premises I verified and found sound. Recorded so the next agent does not
re-derive them.

- **`MEMBERSHIP_LAYOUT_VERSION`'s doc** (`membership.rs:63-86`) — the
  already-fixed false premise is genuinely fixed. It now states that both
  constants are `1`, that the version check therefore separates nothing, and
  that `check_layout` naming `stoa` and `genesis_bytes` is "the whole of the
  boundary". Verified: `MEMBERSHIP_LAYOUT_VERSION = 1` (`membership.rs:86`),
  `log::sqlite::LAYOUT_VERSION = 1`, `check_layout` selects both column names
  (`membership.rs:304`). The matching test comment (`:748-759`) says the same
  thing and asserts the right property. No further work here.
- **"Nothing here panics"** (`membership.rs:50-55`) — verified by reading every
  non-test line. No `unwrap`, no `expect`, no indexing; `split_off` is guarded by
  `.min(items.len())` (`:581`), `i64::try_from` handles the offset (`:549`),
  `saturating_mul`/`saturating_add` handle the arithmetic (`:521`, `:548`),
  `usize::try_from` handles the count (`:599`). (A reachable-panic hunt proper is
  the correctness/security instances' job; I checked only that the comment is
  true.)
- **"`PRAGMA user_version` IS LAST, AND MUST STAY LAST"** (`membership.rs:323-329`)
  — true of `create_schema`: the pragma is the second-to-last statement, followed
  only by `COMMIT` (`:353-355`). The stated consequence (a crash leaves version 0,
  which takes the create branch) matches `from_connection:276-277`.
- **`stoa_reply`'s claim that the spec fixes the list-item shape**
  (`wire.rs:534-537`) — sound. `specs/stoa-membership/spec.md:223`: "each item
  carrying the Stoa's address and its founding title", and `:247-251` is the
  scenario. So `policy` being on the create/join reply and not on a list item is
  a spec-backed choice, not an omission.
- **`FOUNDING_TITLE` / `foundingTitle` never `title`** (`wire.rs:514-528`) —
  sound and well argued. `spec.md:253-263` requires the reply distinguish a
  founding title from a resolved current one, and the test at `wire.rs:2460-2487`
  asserts the absence of a bare `title` as well as the presence of
  `foundingTitle`, which is the half that actually pins it.
- **`policy_name`'s "exhaustive with no wildcard arm"** (`wire.rs:547-554`) —
  true. `Policy` has one variant (`stoa.rs:116-119`), the match has no `_` arm
  (`wire.rs:556-562`), and the cited `Policy::from_byte` does refuse an unknown
  discriminant rather than defaulting to `Open` (`stoa.rs:142-151`). The `NO SPEC`
  marker on the `"open"` literal is correct — `grep` of the spec finds no wire
  spelling for it.
- **`clamp_per_page` turns 0 into the default** (`membership.rs:498-500` and
  `:1530-1532`) — true. `feed.rs:175-180`: `None | Some(0) => DEFAULT_PER_PAGE`.
  So both `NO SPEC` markers on the `per_page == 0` behaviour are honest about the
  wire never producing it.
- **`identity_key`'s "Infallible … the `expect` is unreachable"**
  (`keystore.rs:676-682`) — true. `SecretKey::from_bytes` fails only on a length
  other than 32 (`identity.rs:295-298` — `try_into` then an infallible
  `SigningKey::from_bytes`), and `root` is `[u8; ROOT_SECRET_LEN]` with
  `ROOT_SECRET_LEN = 32` (`keystore.rs:96`, `:585`).
- **`stoa_key`'s "Built and, in the MVP, not called by any handler"**
  (`keystore.rs:641-645`) — true. `grep` of `wire.rs` and the adapter finds
  `stoa_key`/`stoa_public_key` only in tests; the adapter's `get_capabilities`
  and `create_stoa` both use `identity_*` (`lib.rs:346`, `:385`), and the comment
  at `lib.rs:337-341` explaining why is accurate.
- **`membership.rs`'s "nothing in this change edits `SqliteOpLog`"**
  (`:44-48`) — true. `git diff --stat origin/main...HEAD` lists no file under
  `dialectica-core/src/log/`.
- **The `list_stoas` `Err` arm comment** (`wire.rs:744-747`) — the obligation it
  states ("a storage failure is the error shape and NEVER an empty listing") is
  the one the code implements, and the regression test at
  `membership.rs:1476-1511` cites this exact sentence as the obligation the old
  conversion bug broke. The cross-reference resolves.
- **`membership_path_in`'s separate-file argument** (`wire.rs:789-801`) — the
  load-bearing half is checkable and holds: `SqliteOpLog`'s schema creation is
  gated on a never-stamped file, so a table added there would not reach an
  existing store. The test at `wire.rs:2781-2797` hardcodes both sides of the
  filename.
- **Error-message actionability across `MembershipError`** — every variant's
  `Display` (`membership.rs:178-213`) names what failed and what to do, carries
  the underlying reason where one exists, and is distinguishable from its
  siblings; `every_error_renders_without_leaking_rust_syntax` (`:935-988`)
  asserts the distinguishability as a set rather than per-variant, which is the
  assertion that actually catches a collapse. No unactionable message found.
- **The `guarded` nesting** — `with_membership_store` wraps `guarded(method, …)`
  and the handler inside it calls `guarded` again under the same name. Not a
  readability defect: the doc at `wire.rs:773-775` states why the outer guard
  exists, the inner one is each handler's own contract, and a panic reported
  twice under one name is still one reply. Raising it only so the next reviewer
  does not spend time on it.
