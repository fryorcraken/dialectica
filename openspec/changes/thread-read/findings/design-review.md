# Design review — `thread-read`

Scope: does the code take the decisions `design.md` records, and were the
decisions worth recording recorded? Not correctness, security or test coverage —
those are reviewed separately.

**The substance is in good shape.** Every decision I could check against the code
holds, including the three the task flagged as most load-bearing: membership is
derived from the parent chain and the `thread` field is genuinely never read
(`thread_of` does not mention it, and neither does `read_thread`'s placement
step at `thread.rs:502`); the cross-Stoa fix went into `revision.rs` as one
condition at `revision.rs:327` and `feed.rs:238` really does call
`current_version`, so the free repair of `feed.rs` is real rather than asserted;
and the hidden-root/hidden-reply asymmetry is structural — `body:
Option<Sanitised>` with `None` reachable only through `withhold` at
`thread.rs:579`, so withheld and author-cleared are different values rather than
one value plus a flag. The `CyclicLog` justification is written on the fake's own
doc comment (`thread.rs:1082-1096`), which is exactly where a reader meeting it
looks. The suite is green: 876 + 26 tests, 0 failed.

The findings below are one stale section, two claims that do not check out
against a command, and two unrecorded decisions.

- [ ] **`dev-writer`** — `design.md:269-289` tells the reader the spec
      contradicts itself, and it no longer does. The section "The spec names two
      different refusals for a revision's op id" describes the scenario "Reading
      by a revision's op id is not reading the thread" as saying the refusal "is
      the one for a thread this peer does not hold", and closes with "**The
      spec-writer should pick one.**" Commit `c6ce3c7` on this same branch
      resolved it: `spec.md:46-47` now reads "the refusal is the one for an op
      the peer holds that is not a post, a revision being exactly that" / "and it
      is not the refusal for an op the peer does not hold, which would be false
      about an op it has", matching `spec.md:393`'s rule over kinds. **Verified:**
      read both spec lines and `git show --stat c6ce3c7`, whose own message says
      "No code change: the implementation already does all four. The spec now
      describes it." A reader of `design.md` today is sent to fix a contradiction
      that is already fixed, and is left believing a shipped spec is
      self-inconsistent. Rewrite the section as the record of a resolved
      question, or move it to "What this change found" — the argument for the
      resolution is still worth keeping, the instruction to the spec-writer is
      not.

- [ ] **`dev-writer`** — `design.md:255-257` cites the wrong document for the
      index debt. The Risks entry says "the projection with an index by parent is
      where this is fixed, and **`proposal.md` records that the index is owed to
      whoever builds it**". `proposal.md` records no such thing: grepping it for
      `index`, `projection`, `sqlite`, `cost`, `quadratic` and `scale` returns
      nothing at all. **Verified:** two greps over
      `openspec/changes/thread-read/proposal.md`, both empty. The debt *is*
      recorded, and in a better place — `docs/PLAN.md:3686-3693`, the
      resolver-gap bullet, corrected in this change to say the projection must
      index by **parent** rather than by the untrustworthy `thread` field. That
      is where whoever builds `sqlite-projection` will actually look. Point the
      citation there. This matters beyond tidiness: `design.md` and `proposal.md`
      are archived with the change, and a future reader chasing the owed index
      through the named document finds an empty file and may conclude the debt
      was never filed.

- [ ] **`dev-writer`** — `design.md:246-247`'s deferral of the non-zero page-size
      type is not honest about the count, and the moment it names has already
      passed. The entry says the type "changes `feed::list_threads`'s signature
      too" and that "**If a third paginated read is added, that is the moment to
      introduce the type** rather than write a third clamp." There are already
      three paginated reads in the crate, and this change makes the third clamp
      the one it says it is deferring. **Verified:** `feed::list_threads`
      (`feed.rs:206`), `MembershipStore::list` (`membership.rs:568`) and now
      `thread::read_thread` (`thread.rs:415`). `membership.rs:598` carries its own
      independent zero guard — `if per_page == 0 { return
      Ok(MembershipPage::empty_last_page(page)) }` — written for the same defect
      and with a comment at `membership.rs:589-597` arguing at length that the
      guard must be unconditional because a caller one layer up cannot be relied
      on. That is the same argument `design.md` §12 makes, arrived at
      independently in a second module, which is precisely CLAUDE.md's "fourth
      slightly-different copy of a guard" signal that `design.md` §10 invokes
      correctly for the Stoa check but not here. Either state the real count and
      say why three is still not enough, or take the shape. Note the two are not
      even the same answer — `feed`/`thread` clamp zero **up** to the default,
      `membership` returns an empty last page — so the three sites already
      disagree about what a zero means.

- [ ] **`dev-writer`** — `thread::clamp_per_page` is a byte-identical copy of
      `feed::clamp_per_page` and `design.md` does not record the duplication as a
      choice. §12 discusses at length *where* the clamp is called, and never that
      the function and both its constants were copied rather than reused.
      **Verified:** `thread.rs:258-263` and `feed.rs:175-180` have the same body
      (`None | Some(0) => DEFAULT_PER_PAGE, Some(n) => n.min(MAX_PER_PAGE)`) over
      constants with the same two values, 100 and 20. The constant duplication
      *is* defended, in the code and well — `thread.rs:82-94` argues the two caps
      answer different questions and names
      `the_page_caps_agree_until_someone_decides_otherwise` as observing rather
      than enforcing the agreement — but the *function* is not, and the code
      comment's argument is about the numbers, not the clamping rule. A reader
      changing the zero-clamps-up rule has two copies to find and no note saying
      the second exists. One sentence in §12 either way: reused-by-copy because
      the caps may diverge, or an oversight to fix.

- [ ] **`dev-writer`** — the Stoa check on the root is a decision with a real
      alternative and `design.md` does not record it. `thread.rs:471-473` refuses
      a root belonging to another Stoa as `NotHeld` rather than letting the
      `iter_stoa` loop below simply not find it, and the code comment gives the
      reason — "a root in another Stoa would otherwise produce an empty page,
      which is the answer reserved for a thread with no replies". That is a
      genuine choice between two behaviours a reader could plausibly have made
      differently, it maps onto `spec.md:373`'s scoping of the word "holds", and
      it is the fourth of four root refusals whose ordering decides which message
      a caller gets. `design.md` §4 enumerates three refusals settled before
      anything is read and does not mention the Stoa one; §11 records the
      mis-keyed-row guard beside it in the same function but not this. By the
      test the task sets — would someone changing this code later need the
      reason, and would they find it — the reason exists only in a code comment,
      where a reader reshaping the refusal order will meet it only if they happen
      to be in that function. It belongs in §4 alongside the other three.

## Checked and sound

Recorded here so the next reviewer does not re-derive them.

- **Membership everywhere, including paths added later.** `thread_of` reads
  `parent` only; `read_thread` places by `thread_of(log, &id)? != Some(*root)`
  (`thread.rs:502`) and by nothing else. The mis-keyed-row guard added by
  security review (`thread.rs:330`, `thread.rs:456`) is applied at both the root
  and every chain link, so the later-added path took the rule too.
- **`feed.rs` genuinely benefits from the `revision.rs` fix.** `feed.rs:238`
  calls `current_version`, which routes through `is_valid_revision`. `design.md`
  §10's further claim that `feed.rs`'s own suite has no test for it also checks
  out — `only_this_stoas_threads_are_returned` (`feed.rs:575`) tests the
  `iter_stoa` filter, not revision standing.
- **The corrected overclaiming comment.** `revision.rs:706-721` now states the
  replay test's true width and names
  `a_revision_freshly_signed_for_another_stoa_is_dropped` as covering what the
  old comment wrongly implied. I found no other comment in the new code arguing
  from a premise the code disproves.
- **The three `NO SPEC` markers are genuine.** The spec fixes no strings for the
  moderation state (`spec.md:250` requires three distinguishable values and the
  scenarios speak relationally), names no field for the public key
  (`spec.md:161`, `spec.md:193` constrain the set of author fields, not their
  names), and says nothing about a chain crossing a Stoa boundary mid-walk —
  `spec.md:122` scopes the Stoa rule to what is **returned**, and the membership
  requirement's three failure modes (`spec.md:57-61`) do not include a foreign
  intermediate link. `thread.rs:1481-1496` marks it and asserts both what the
  spec does require and which way this chose, which is the right handling.
- **A fourth gap the spec does not carry**, worth a sentence to the spec-writer
  rather than a box here: the mis-keyed store row has no requirement behind it.
  `spec.md:371`'s "the op it holds under that id" presumes the row is that op,
  and `spec.md:593`'s adversarial-contents list does not include it. The code
  closes it anyway and `design.md` §11 records the decision properly, so nothing
  blocks — but nothing pins it either.
