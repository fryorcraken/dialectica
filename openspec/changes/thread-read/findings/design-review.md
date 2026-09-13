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

- [x] **`dev-writer`** — `design.md:269-289` tells the reader the spec
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

      **Fixed**, taking the first option. The section is now headed "RESOLVED IN
      THE SPEC", opens by saying so, keeps the argument for *why* the resolution
      went that way — which is why `spec.md` reads as it does — and records that
      `c6ce3c7` stated the rule over **kinds** so a later kind needs no new
      decision. The closing instruction to the spec-writer is gone. The section
      heading above it also changed from "disagrees" to "disagreed", since only
      one of its two entries is still open.

      **A second copy of the same stale framing, which the finding did not name
      and I would not have found without it.**
      `reading_by_a_revisions_op_id_is_not_reading_the_thread`'s comment opened
      "SPEC CONFLICT [...] flagged because the two genuinely disagree" and closed
      "Reported to the spec-writer" — a test comment telling its reader the
      contract is self-inconsistent. Rewritten to state the rule, note that the
      spec did not always say it and resolved the same way with no code change,
      and point at `design.md` for the argument. Same defect, same fix, one file
      over.

- [x] **`dev-writer`** — `design.md:255-257` cites the wrong document for the
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

      **Fixed** — the Risks entry now cites `docs/PLAN.md`'s resolver-gap bullet
      and says what makes it the right pointer: it records the **parent**
      correction, and it outlives the change's archived folder, which
      `proposal.md` does not. I re-ran your check before editing rather than
      taking it on trust — grepping `proposal.md` for `index`, `projection`,
      `sqlite` and `quadratic` returns nothing, and `PLAN.md:3686-3693` carries
      the corrected bullet. The entry also names the old citation as wrong, so a
      reader who remembers it is told rather than left wondering.

      Your point about the archived folder is the part that decided the wording:
      the failure is not a reader finding the wrong file, it is a reader
      concluding the debt was never filed. The new text makes the debt's location
      the claim rather than an aside.

- [x] **`dev-writer`** — `design.md:246-247`'s deferral of the non-zero page-size
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

      **Deferred — still, but on a stated count and a real argument instead of a
      trigger that had already fired.** You are right that the old wording was the
      dishonest part: naming a condition that was already true reads as a plan and
      functions as an evasion. §12 now opens by saying the honest count is three,
      carries a table of all three sites and the different answer each gives, and
      quotes `membership.rs:589-597` reaching this change's argument
      independently — which is the fourth-guard signal, named as such.

      **Why still deferred**, stated plainly as you asked. The type has to live
      somewhere neutral: `membership.rs` has no caps of its own and borrows
      `feed`'s through the wire, so introducing it edits `feed.rs`,
      `membership.rs`, `wire.rs` and this module at once. That reshapes two other
      pieces' public surfaces in a findings-response commit on a piece whose
      reviewers were looking at a thread read — the same boundary that kept the
      feed's spec debt out of this change, applied to code instead of a spec.

      **One thing I checked that makes the divergence narrower than it looks, and
      it is worth having in the record:** `list_stoas` also clamps through
      `feed::clamp_per_page` (`wire.rs:2329`), so **all three wire paths answer a
      zero with the default** and `membership`'s empty-last-page is unreachable
      through the module surface. The disagreement is between the three `pub`
      functions' direct contracts, not between three observable behaviours. That
      does not dissolve it — a `pub` function on a `pub mod` is a contract, which
      is `membership.rs`'s own point — but it is why nothing is blocked.

      §12 names the owner as whoever next touches pagination in more than one of
      the three, and names the decision they must make first: which of the two
      zero answers becomes the single one. I did not pick it here, because
      picking it for `membership.rs` from inside this piece is the overreach the
      deferral is about.

- [x] **`dev-writer`** — `thread::clamp_per_page` is a byte-identical copy of
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

      **Fixed** — recorded as reused-by-copy, in both places rather than one.
      §12 gains a paragraph, and `thread::clamp_per_page`'s own doc comment gains
      a section saying it is a copy and why, so a reader changing the rule is told
      where the other one is at the moment they are looking at it. That second
      half is the part that actually answers your objection: a note only in
      `design.md` does not reach someone editing the function.

      The argument is the one you anticipated — it follows the constants. A shared
      clamping function over per-module caps would have to take the caps as
      arguments, which is a guard with two parameters a caller can pair wrongly,
      and that is the shape a guard exists to avoid; or it would re-export one
      module's numbers and quietly undo the divergence argument `MAX_PER_PAGE`
      makes. You are right that the code comment defended the *numbers* and not
      the *rule*, and both notes now say the rule lives in two places and that
      `MembershipStore::list` answers a zero a third way.

- [x] **`dev-writer`** — the Stoa check on the root is a decision with a real
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

      **Fixed, and it turned out worse than the finding says: §4 described a
      function with three gates and the function has five.** Writing the Stoa one
      in meant reading the code rather than my own section, which is how the other
      two surfaced — the mis-keyed-row guard (recorded in §11 but absent from §4)
      and the verification gate were both missing too. A decision record
      describing an earlier version of the function is worse than none, so §4 is
      now a table of all five gates against the three messages they collapse into,
      with a closing note that three of the five arrived after the section was
      first written.

      **One refinement, which strengthens the finding rather than weakening it.**
      *That* a foreign-Stoa root is refused rather than answered with an empty
      page is this change's decision, and §4 now records it with its alternative —
      letting the `iter_stoa` loop simply not find it, which costs nothing and is
      wrong because an empty page is the answer reserved for a held root with no
      replies. *Which* refusal it takes is not ours to defend: `spec.md:373`,
      which you cite, scopes "holds" to "holds a usable post in the named Stoa"
      and names both the unverifiable op and the foreign-Stoa op as taking the
      **not-held** refusal. Both gates were implemented before the spec said so
      and the spec then agreed. §4 cites that line rather than re-deriving it, and
      also cites the limit it sets — an in-Stoa op of the wrong kind is *usable*
      and takes `NotAPost` — because that is what stops this reasoning becoming a
      licence to report anything inconvenient as not held.

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
