# Readability review — `thread-read`

Scope: can someone modifying this code in six months tell which lines are
load-bearing for the security property and which are convenience? Not
correctness, security, architecture, spec-test or design-record — those are
reviewed in the five other files beside this one.

**The prose in `thread.rs` is the best in the crate and I am not going to pad
this list.** Every question the task set came back positive on its own terms:

- **`thread_of`'s prohibition is legible where it matters.** The module header
  (`thread.rs:3-28`) names the attack, names the test that was watched failing,
  and closes with the strongest available formulation — "[`thread_of`] does not
  mention it, which is the strongest form of 'never trusted' available". The
  function's own doc repeats it as "**THE security property of this
  capability**" (`thread.rs:283-285`), and `read_thread`'s placement step
  carries it a third time at the one line that could be edited wrong
  (`thread.rs:516-517`). Three statements, each at a point a reader arrives.
- **The three refusals read as three next actions, not three variants.**
  `NotAThread`'s doc (`thread.rs:204-215`) leads with "three different mistakes
  call for three different responses", each variant's own doc says which, and
  `the_three_refusals_each_name_the_next_action_they_imply`
  (`thread.rs:2418-2486`) asserts the mapping as a relation rather than as
  pinned literals. A reader who only skims the enum still gets the mapping.
- **`CyclicLog` earns its place before a reader can call it dead weight.** Its
  doc (`thread.rs:1095-1112`) opens with why a real log *cannot* mint a cycle
  (an op id is a hash over the `parent` field), then why the fake is
  nonetheless the honest fixture (a peer's store is a file), and closes with
  the detail that decides whether someone deletes it — "without the visited set
  the tests below do not fail, they **hang**".
- **The `clamp_per_page` duplication is defended at the function itself.**
  `thread.rs:259-273` is headed "This is a COPY of [`crate::feed::clamp_per_page`],
  and the copy is on purpose", gives the constants-follow argument, and names
  the third divergent answer (`MembershipStore::list`) so a reader changing the
  rule knows there are three sites. That is the note that reaches someone
  editing the function, which a `design.md` entry alone would not.
- **The five gates are individually legible.** Each of the five refusal sites in
  `read_thread` (`thread.rs:446-500`) carries its own reason, and the two added
  by security review carry the longest ones.

Two findings below. Both are **stale cross-references** — prose that was correct
when written and that a later correction on this same branch left behind. Both
are the failure this repo has a named rule against (a claim that cannot fail
loudly), and neither is a defect in behaviour.

- [x] **`dev-writer`** — `thread.rs:426-430` — `read_thread`'s doc defers the
      non-zero page-size type on a trigger that has already fired, and points at
      a `design.md` section that no longer says what it is cited for.
      **Scenario:** the doc comment reads "The alternative — a page-size type
      that cannot be zero — [...] was not taken here because it changes
      [`crate::feed::list_threads`]'s signature too, and `feed.rs` is not this
      change's to touch. Recorded in `design.md` as the shape to reach for **if a
      third read is added**." `design.md` §12 was rewritten during design review
      to say the opposite: "This section used to defer it until 'a third
      paginated read is added'. Design review pointed out that **this is the
      third**" (`design.md:280-282`), and the deferral now rests on a different
      argument — that the type must live somewhere neutral, so introducing it
      edits `feed.rs`, `membership.rs`, `wire.rs` and this module at once
      (`design.md:307-313`). A reader of the code today is told the condition is
      unmet when it is met, is given a two-file reason where the real one is
      four, and is sent to a `design.md` section that contradicts the sentence
      pointing at it. **Measured:** `grep -n "third read is added\|third
      paginated read"` over both files returns exactly one hit each —
      `thread.rs:430` carrying the superseded framing and `design.md:280`
      explicitly retiring it. Severity: **medium** — the code comment is the
      copy a reader editing the function meets, and it is the one that is wrong;
      `design.md` is archived with the change while `thread.rs` is not.
      The fix is to restate the honest count (three) and the real four-file
      reason, or simply to point at §12 without re-deriving it.

      **Fixed**, taking both halves of your suggestion. The comment now states
      the count (three reads, two different zero answers, with `MembershipStore`
      named as the third), states the four-file cost, and then **stops** — it
      closes "**Do not re-derive the argument from this comment**; it is a
      pointer, and §12 is the record." Two copies of an argument is how one goes
      stale, which is this box.

      Your severity reasoning is what shaped that ending: `design.md` is archived
      with the change and `thread.rs` is not, so the code comment has to be the
      one that survives — and the way to keep a surviving copy true is to make it
      carry the facts a reader needs and defer the reasoning rather than mirror
      it.

      **Also folded in the observation you left in prose rather than a box**
      (`read_thread`'s "refused three ways" beside §4's five-gates table).
      `thread.rs:375` now reads "five gates producing three messages" and points
      at §4 for the mapping. You were right that it did not need its own box, but
      it was the same class of drift and the edit was one line.

- [x] **`dev-writer`** — `revision.rs:240-244` — `current_version`'s doc still
      enumerates **four** conditions and omits the Stoa, which this change added
      as a fifth. **Scenario:** the public function's "What is dropped, and in
      what order" section reads "A candidate revision must be all four of: a
      [`OpKind::Revise`], naming this post, **authentic**, and **by this post's
      author**. The log's `iter_target` supplies the second; **the two checks
      here** supply the rest". `is_valid_revision` now makes **three** checks,
      not two — kind, Stoa, verify, author — and its own doc was updated to say
      "All **four** must hold" (`revision.rs:289`). The Stoa condition is
      documented only on the private helper, so the doc a caller reads
      (`feed.rs:238` and `thread.rs:576` both call `current_version`, and nothing
      calls `is_valid_revision` from outside the module) states a rule that is
      now incomplete in the direction that matters: a reader auditing "what makes
      a revision bind?" from the public entry point is told authorship and
      authenticity and is not told the Stoa, which is the very condition this
      change discovered was missing. **Measured:** `grep -n "current_version("`
      over `dialectica-core/src/` shows the only non-test callers are
      `feed.rs:238` and `thread.rs:576`; both reach the four-condition doc and
      neither reaches the five-condition one. Severity: **medium** — this is the
      same shape as the overclaiming comment this change correctly fixed one
      screen below it (`revision.rs:706-721`), left in the doc directly above the
      fix. One sentence naming the Stoa as a condition and correcting "the two
      checks here" to three.

      **Fixed** — "all four" is now "all **five**", the Stoa is named in the
      enumeration between "naming this post" and "authentic", and "the two checks
      here" is "the three checks in `is_valid_revision`".

      **Your framing is the part I want to record, because it is sharper than
      "update the count".** The omitted condition was *the one this change
      discovered was missing*, in the doc a caller reads, while the corrected
      version sits on a helper no caller outside the module reaches. A reader
      auditing "what makes a revision bind?" from the public entry point was told
      authorship and authenticity and not the Stoa — which is exactly the audit
      that would have caught the defect in the first place. So the doc now says
      why the condition exists and that its absence was a real defect found by
      security review, rather than silently listing a fifth item.

      I checked your caller claim before editing rather than after: `grep -n
      "current_version("` over `dialectica-core/src/` confirms `feed.rs:238` and
      `thread.rs:576` are the only non-test callers, and nothing outside the
      module calls `is_valid_revision`.

      Two intra-doc links from this public doc to the private
      `is_valid_revision` produce `cargo doc` warnings. Left as they are: the
      crate has the same pattern in at least four places (`moderation`,
      `contains`, `resolve` and `sanitise` all link to private items), `cargo
      doc` is not a CI gate here, and pointing a reader at the helper that holds
      the reasoning is worth more than a clean docs build this repo does not
      produce anyway.

## Checked and clean

Recorded in prose so the next reader does not re-derive it, and so the list
above is not padded with observations nobody needs to act on.

- **No comment in the new code argues from a premise the code disproves.** I
  checked every numeric and behavioural claim in `thread.rs` and the new
  `wire.rs` section against a command. `the_page_caps_agree_until_someone_decides_otherwise`
  really does hardcode 100 and 20 rather than compare two drifting constants
  (`thread.rs:2988-2997`); `an_enormous_page_index_does_not_overflow`'s claim
  that `(1 << 63, 2)` is the discriminating row is arithmetically right
  (`(2^63).wrapping_mul(2) == 0`, serving page zero); the `CyclicLog` tests'
  claim that they hang rather than fail without the visited set follows from the
  loop having no other exit. The one overclaiming comment on this branch —
  `a_revision_lifted_into_another_stoa_is_dropped`'s "relies entirely on
  `verify()`" — is corrected in place at its true width (`revision.rs:706-721`),
  and I found no second instance.
- **The suite is green and the figures in the task are right.** 879 core + 28
  integration, 0 failed, run against this worktree. CI's `ran == declared` gate
  will also hold: counting `^\s*#\[test\]\s*$` exactly as the gate does gives
  879 across `dialectica-core/src/` and 28 in `tests/end_to_end.rs`, matching
  both targets exactly.
- **The bare `§` citations are house convention, not a gap.** `thread.rs` cites
  §3.3, §11.1, §5.2 and so on without naming `PLAN.md`; `feed.rs`, `moderation.rs`
  and `revision.rs` carry 37, 42 and 32 of the same. Changing it here alone would
  make this module the odd one out.
- **`dialectica/rust-lib/src/lib.rs` reads correctly by eye.** The gate cannot
  see it, so I read the 43-line diff rather than testing it. The `read_thread`
  trait method's doc states the parent-chain rule, the flat-items rule and the
  hidden-root asymmetry in the same order the core does; the impl
  (`lib.rs:699-710`) is byte-for-byte the shape of `list_threads` above it, with
  the store-per-call reasoning stated by reference rather than copied. Nothing in
  it makes a decision core does not already make.
- **`read_thread`'s "refused three ways" (`thread.rs:375`) is not wrong**, since
  three is the message count and the piece's whole framing is three refusals —
  but it sits beside `design.md` §4's "five gates, three messages" table with no
  pointer to it. Folding a pointer into the first finding's edit would close it;
  it does not need a box of its own, because each of the five gates carries its
  own reason inline and a reader is never left guessing at one.
