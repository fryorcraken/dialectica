# The moderation resolver: deciding, on read, whether a target is hidden

## Why

`op.rs` can tell you a moderation op is **authentic** — it really is from the key
it names. It says, in as many words, that it cannot tell you whether that key had
any business moderating:

> **This answers authenticity only.** It does not ask whether the signer is a
> moderator (§6) [...] Reading a `true` here as "this op is valid" is precisely
> the conflation §6.2 measured in the nearest kin project, which "checks
> moderator authority only on the send path and never on the read path".

Two tests in `op.rs` and one in `log.rs` pin that split deliberately: a `Moderate`
op signed by a random peer verifies, and the log stores it. **Nothing in the crate
currently refuses it.** Until something does, a peer that rendered the log's
moderation ops at face value would be Appendix A's defect exactly — "any peer can
forge a moderation, or forge the removal of one", in shipped code.

This change is the missing half: the first code in the project that can decide
authority, because it is the first that holds both a Stoa's genesis record (which
names the creator, §6's sole moderator) and the op log.

PLAN.md §5.7 states the rule the resolver implements:

> Among *moderation* ops on the same target, last-write-wins by Lamport order,
> valid only if the signer was a moderator at that time (§6).

And §6.1 sets the ceiling it must not overclaim: moderation "can only change what
conforming peers *render*". The resolver answers what a conforming peer renders.

## What Changes

- A `moderation` module answering **is this target hidden, and by which op?** over
  an `OpLog`, for one Stoa.
- A `Moderators` type carrying a Stoa's address and the set of keys authorised to
  moderate it, constructed **only** from a genesis record. The creator is the sole
  member (§6), and the type makes "I do not know the moderator set" unrepresentable
  rather than a value the resolver has to branch on.
- Three checks on every candidate op, every read, by every peer: the signature
  verifies (authenticity), the signer is in the moderator set (authority), and the
  op's own Stoa is the Stoa that set governs (scope).
- A `Moderation` result naming the deciding op, not merely a boolean — so a
  moderator acting on a post is acting on a judgement they can name, the way §5.7
  gives an author's revision a version a moderator can name.

## Capabilities

**New Capabilities**

- `moderation-resolution` — what makes a moderation op binding, how competing
  moderations on one target are ordered, and what a reader is told about a target
  the peer holds no valid moderation for.

**Modified Capabilities**

None. `op-log` describes what the store holds and refuses to decide; this describes
what a reader decides from it. No requirement in that spec changes.

## Impact

- New `dialectica/rust-lib/dialectica-core/src/moderation.rs`, and one line in
  `lib.rs`. No existing file's behaviour changes: `op.rs`'s authenticity/authority
  split is relied on, not altered, and the two tests pinning it stay as they are.
- No wire-contract change. Nothing crosses the module boundary yet; §2.5's surface
  is untouched.
- Forked from `phase2/op-log`, which supplies `OpLog::iter_target`. The revision
  resolver is being built in parallel against the same trait and adds its own
  module to `lib.rs`; the two touch no common line but that one.
