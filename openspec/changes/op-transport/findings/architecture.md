# Findings — architecture review, `op-transport`

Read: `dialectica/rust-lib/dialectica-core/src/transport.rs` (2,506 lines,
non-test body lines 1–602), `design.md`, `tasks.md`, the spec delta, and — because
the piece merged `origin/main` twice — `authoring.rs` and
`openspec/specs/content-authoring/spec.md`, which is where the sharpest finding
came from.

Baseline confirmed: **626 tests passing**, matching the brief.
`cargo mutants --file dialectica-core/src/transport.rs`: **32 mutants, 26 caught,
5 unviable, 1 missed** (`OpenChannels::is_empty -> true`) in 7m. The one miss is a
coverage question and belongs to `tester`, not to this dimension; it is noted here
only because it names the accessor finding 3 is about.

Worktree used for the mutation run and removed afterwards.

## Findings

- [ ] **`dev-writer`** — `transport.rs:573` and `authoring.rs:187` — **two
      functions named `publish` in one crate, and the one a caller actually reaches
      never touches this capability.** `authoring::publish` signs and appends with
      `Arrival::unordered()` and then returns; `transport::publish` appends with
      `Arrival::unordered()` and returns bytes. Neither calls the other, and
      `authoring.rs` does not import `transport` at all
      (`grep -n "^use " authoring.rs` returns four lines, none of them
      `crate::transport`).
      **Measured:** `grep -rn "transport::" dialectica/rust-lib/` returns **zero
      non-test call sites**. `transport::publish`'s only callers are its own 14
      tests. Meanwhile `authoring::post`, `reply` and `vote` — the live publish
      path, merged as #51 at 12:20 today and merged into this branch in `89f7f55` —
      reach `authoring::publish` instead.
      **What future change becomes hard:** `openspec/specs/content-authoring/
      spec.md:29-32` requires each publish operation to "sign an op …, append it to
      the local op log, **and hand it to delivery**". Wiring that means making
      `authoring::post` reach `transport::publish` — at which point the op is
      appended twice unless one of the two appends is removed. Removing
      `authoring`'s append moves a requirement its own suite pins; removing
      `transport`'s makes `transport::publish` a pure derivation that no longer
      earns the "store first" ordering argument its doc comment is built on. Either
      way the next change is the reshape this one deferred, and it lands in two
      files with two test suites asserting the same ordering from different sides.
      **Severity: high** — this is the "make the change easy, then make the easy
      change" question the brief asks, answered in the negative: the easy change is
      not available, because the seam was placed beside an existing path rather than
      under it.

- [ ] **`dev-writer`** — `transport.rs:312` and `authoring.rs:84` — **two public
      `Refusal` enums in one crate**, `dialectica_core::transport::Refusal` and
      `dialectica_core::authoring::Refusal`, with disjoint variant sets and no
      relation between them.
      **What a future change becomes:** the core API method that has to report both
      — `content-authoring`'s handler, once it hands off to delivery — must import
      both under aliases and write a conversion, and a reader of an
      `{"error":"..."}` string cannot tell which enum produced it. `stoa.rs` and
      `op.rs` each name their error type for what it is (`OpError`,
      `GenesisError`); this file and `authoring.rs` both took the generic word.
      **Severity: medium** — a naming collision rather than a defect, but it is the
      kind that gets resolved by whoever needs it first, under time pressure, in the
      call site rather than in the types. Separate from finding 1: even if the two
      publish paths converge, the two error enums are still two types.
      *(Judgement, not defect: which one renames is the author's call. `transport`'s
      is arguably `InboundRefusal`, since every variant is about a payload that
      arrived.)*

- [ ] **`dev-writer`** — `transport.rs:171` — `ChannelIdentity::content_topic()`
      **has no caller outside this file's own tests**, and neither does the adapter
      that would need it. `grep -rn "content_topic()" dialectica/rust-lib/` returns
      8 hits, **all of them in `transport.rs`'s `mod tests`**, and
      `grep -rn "channelCreate\|channel_create" dialectica/rust-lib/src/` returns
      nothing — the adapter `design.md:318` describes as "three lines" is not
      written.
      **Why this is architecture and not dead code to delete:** the content topic is
      half of what makes `ChannelIdentity` one type rather than two functions
      (`design.md:65`), and that argument is sound. But it means the type's stated
      justification — "two functions can each be called with a *different* address"
      — is today protecting a value nothing consumes, so the first real consumer
      arrives with no test showing the topic reaching `channelCreate` and nothing to
      catch a topic/channel-id mix-up at the one call site where it matters.
      **Severity: low** — the shape is right and the gap is the adapter's. Worth a
      box so it is a tracked consequence of splitting at `cfg(logos_scaffold)` rather
      than something a later reader discovers while wiring the send.

- [ ] **`dev-writer`** — `transport.rs:521-542` — **`Publishable` makes forgetting
      the send silent, and the seam is where the three owed things must attach.**
      The brief asks whether that is acceptable; measured, it is not yet, for a
      reason narrower than "a caller might forget". `Publishable` derives
      `Debug, Clone, PartialEq, Eq` and carries **no `#[must_use]`** — and
      `grep -rn "must_use" dialectica-core/src/` returns **zero hits across the
      whole crate**, so there is no house precedent either way.
      **What future change becomes hard:** `design.md:280-288` names `Publishable`'s
      `id` + `channel_id` as "exactly the pair a tracker needs to key an outcome back
      to an op". A tracker built on that premise must observe *every* `Publishable`
      that was sent; a `Publishable` dropped on a path that forgot to send is
      indistinguishable, to the tracker, from one that was sent and never propagated
      — which is precisely the third owed thing ("what it records for one that never
      propagated"). So the missing `#[must_use]` is not a lint preference here; it is
      the one compiler-visible signal that would separate the two states the tracker
      has to tell apart. `a_send_failure_does_not_lose_the_op:2180` deliberately
      `drop`s one to witness that the log survives, so the drop is a legitimate
      operation — which is the argument for `#[must_use]` plus an explicit
      `let _ = ` at that one site, not against it.
      **Severity: medium.**

## Areas that are clean

Stated in prose, since none needs action.

**The guard ordering in `receive` (`transport.rs:444-483`) is load-bearing and
correctly ordered, and the brief's worry does not land here.** Each guard is
strictly cheaper than the one after it: a `HashMap` lookup, a `len() >`
comparison, a decode, an Ed25519 verify, a 32-byte compare, then the single write.
The specific masking the brief describes — a hostile sweep never reaching a later
parser because an earlier guard refuses every fixture — is structurally impossible
for the two orderings that matter, because both are pinned by a fixture that
*passes* the earlier guard: `the_size_check_runs_before_the_decode:1763` uses a
payload that is both over-long and undecodable and asserts `TooLong`, and each of
the four decode/verify/mismatch tests carries a guard assertion
(`assert!(SignedOp::from_bytes(&payload).is_ok(), "the fixture must decode, or
this is the decode test")`) proving the fixture reaches the check it names. That
guard-assertion habit is the right generalisable answer to the defect family and
this file should be the precedent the next capability copies.

**The single trailing `log.append` is the right shape for "a refusal cannot
partially apply".** It is not a rollback path and it is not a discipline a reviewer
has to re-verify per call site: every guard operates on borrowed data and the only
mutation is the last statement. `OpenChannels` being taken as `&` in `receive` and
in `publish` makes "SHALL NOT open a channel as a side effect" unreachable by type
rather than by test, which is the CLAUDE.md "complexity in the data structure"
principle applied correctly twice.

**`OpenChannels` as `HashMap<String, Address>` earns its shape, and it leaves room
for the owed things.** The map answers both questions the receive boundary asks,
`open()` taking a `ChannelIdentity` makes the id→Stoa invariant hold by
construction, and adding a `requestId`→`OpId` map beside it — which is what the
tracker needs — is additive rather than a reshape, because nothing in `receive` or
`publish` reads the map's value type. I checked this specifically against the
brief's question 4 and the answer is yes for two of the three owed things (what a
peer records in flight; what it records for one that never propagated). The third,
the bound, needs a clock, and `design.md:297` already says plainly that this crate
has never held one and that where it lives is the tracker's decision — which is the
right place to leave it.

**`ChannelIdentity::of` taking one argument is as structural as a Rust signature
can make it, and the brief's question 5 has no better answer available.** The
spec-test reviewer's mutation 4b (appending `std::process::id()`) is real, but it is
not a hole the *type* could close: a function body may reach for ambient state
whatever its signature says, and no Rust type prevents that. What the type does
achieve is that a *caller* cannot supply a per-peer value, which is the half a type
can hold, and the private fields close the hand-assembly route. The remaining guard
is necessarily the hardcoded pin, and
`the_derivation_is_pinned_to_a_known_answer:871` is that pin done properly —
expectation derived outside the crate via `sha256sum`, with the command in the
comment. Recorded as clean rather than as a finding because the alternative the
brief hints at does not exist.

**The `cfg(logos_scaffold)` split is the right call and is stated honestly.** The
module docs (lines 10–21) and `tasks.md` §7 both say plainly that a property holding
only in the adapter is untested by definition, rather than covering it with a test
that does not reach it. That is the discipline `.claude/agents/README.md` asks for.
