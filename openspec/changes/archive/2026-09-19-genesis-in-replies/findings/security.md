# Security review — `genesis-in-replies`

Scope: security dimension only, per dispatch. Reviewed against `piece/genesis-in-replies`
(merge-base `2549468`, tip `ce8ab44`), i.e. the actual diff introduced by this piece:
`dialectica/rust-lib/dialectica-core/src/wire.rs`, `dialectica-ui/src/qml/DStoaListScreen.qml`,
`dialectica-ui/tests/tst_stoa_screens.qml`, and the spec/proposal/tasks docs.

## What I checked, and what held up

**1. Does widening the reply weaken the verification story?** No. `rememberGenesis` in
`DStoaListScreen.qml` is called from exactly two places — `reload()` (the `list_stoas`
reply) and `create()` (the `create_stoa` reply) — both direct core replies. Nothing in
this piece routes a peer-supplied or pasted genesis into `genesisByStoa`; the paste/join
flow (`DStoaReference.qml`, untouched by this piece) deliberately does *not* self-verify
and instead relies on `join_stoa` → `genesis_for` → `Membership::verified` to do the one
hash check. The map this piece fills is populated only from core's own authenticated
replies, never from anything the view invents or receives from a peer.

**2. The relation, not just the shape.** Enforced on the read path, not only in a test.
`MembershipStore::decode_row` (pre-existing, in `membership.rs`, not touched by this
piece) re-derives `genesis.matches(&stoa)` on every `get()` and `list()` row and refuses
a mismatched pair as `CorruptEntry` before a `Membership` value can even be constructed.
This piece's `wire.rs` additions (`stoa_reply`, `membership_page_json`) sit downstream of
that already-verified `Membership`/`Genesis` pair and just re-encode it.

I mutation-tested both of this piece's own encode sites directly (not the pre-existing
`decode_row`, which is out of scope for this diff):
- Tampering `stoa_reply`'s encoded bytes (title changed after the address was already
  computed) was caught: `a_creation_reports_the_record_its_address_is_the_hash_of` failed
  with `left: "27941f89…" right: "80329cf0…"` — the relation check in the `genesis_of`
  test helper does compare hashes, not literals.
- The identical mutation against `membership_page_json`'s per-item encoding was caught by
  `a_listed_stoa_carries_the_record_its_address_is_the_hash_of` the same way.

Both mutations reverted; `cargo test -p dialectica-core wire::` is back to 247 passed, 0
failed, and `git status`/`git diff` are clean in this worktree.

**3. Round-tripping attacker-controlled bytes.** The inbound path (`genesis_for` in
`wire.rs`, used by `join_stoa`/`read_feed`/`read_thread`, unchanged by this piece) already
bounds hex length against `MAX_CANONICAL_BYTES * 2` *before* `hex::decode`, refuses
non-hex, decodes strictly (`Genesis::decode` rejects unknown version, truncation, trailing
bytes, oversized title, bad UTF-8), and then re-verifies the address relation via
`Membership::verified`. This piece does not touch that path; it only adds an **outbound**
encode of bytes the store already validated on the way in (`decode_row`) and, in the
create case, bytes this same call just built and hashed itself (`create_stoa` calls
`genesis.address()`, which calls `canonical_bytes()`, before `stoa_reply` does so a second
time). Nothing here reads unvalidated store bytes straight onto the wire.

**4. The error path.** Confirmed cannot emit an empty or partial `genesis`. Both
`stoa_reply` and `membership_page_json` `return error_json(...)` immediately on an
encoding failure — `membership_page_json`'s `return` exits the whole function, discarding
any already-built `items`, so a mid-page failure cannot surface as a partial listing.
`error_json` always produces `{"error": message}`, never a success shape with an empty
field, and the messages (`GenesisError`'s `Display` impl) describe format problems only —
no secret material. Note (not a finding, a scope note): the only failure mode of
`canonical_bytes()` is `TitleTooLong`, and by construction every `Genesis` that reaches
either encode site here has already passed that same check once (via `decode()` on the
way out of storage, or via `create_stoa`'s own `genesis.address()` call before
`stoa_reply` runs) — so this error arm is presently unreachable from either call. That is
consistent with the doc comment's own claim and does not weaken the guarantee; it is a
defensive arm rather than a live one.

**5. The unhidden share affordance.** `DStoaReference.shareText` (pre-existing, untouched)
carries both `stoa` and `genesis` in full hex, no truncation or abbreviation, and
`DStoaReference.parse` deliberately does not re-verify the hash relation on the receiving
end — correctly, since verification belongs to `join_stoa` alone (per its own comment: "a
view that re-derived an address would be a second implementation of the one check this
entire design rests on"). This piece only supplies the previously-missing `genesis` value
that makes the pre-existing share code path reachable; it does not change what is shared
or weaken the join-side check.

**6. Hex encoding.** The `genesis` field is only ever *produced* by this piece via
`hex::encode`, which is total and always emits valid even-length lowercase hex — no
length-assumption or truncation risk on the encode side. The `hex` crate is a pre-existing
dependency (`hex = "0.4"` in `dialectica-core/Cargo.toml`), not a new one. Decoding
(`hex::decode` in `genesis_for`) is pre-existing and out of this piece's diff; it already
refuses non-hex and odd-length input via `hex::decode`'s own `Err`, bounded by a length
check before the call.

## Finding

- [x] **`dev-writer`** — `dialectica-ui/tests/tst_stoa_screens.qml:626-649`
      (`test_an_item_short_of_its_record_is_not_recorded_as_an_empty_one`, tasks.md 4.4,
      "the guard direction") — the test intended to pin `rememberGenesis`'s
      empty/non-string guard cannot actually distinguish "the guard skipped the write"
      from "the guard was removed and `""`/`undefined` was written into
      `genesisByStoa` anyway."
      **Scenario:** Delete the guard in `DStoaListScreen.qml`'s `rememberGenesis`
      (`if (typeof genesis !== "string" || genesis === "") return`) so it unconditionally
      writes `next[stoa] = genesis`, including writing the literal empty string for the
      `genesis:""` fixture row. `test_an_item_short_of_its_record_is_not_recorded_as_an_empty_one`
      still passes, because every assertion in it reads the value back through
      `genesisFor`, whose own line (`typeof g === "string" ? g : ""`) coerces both "key
      absent" and "key present with value `\"\"`" to the same `""`. `canShare` is
      `genesisFor(stoa) !== ""`, so it is unaffected either way. The test can only ever
      observe "the effective read is falsy," never "the map itself holds nothing for this
      key" — so it cannot tell a correctly-guarded write from the exact regression this
      guard exists to prevent (re-admitting `""` as a "recorded" genesis, which is
      verbatim the input that produced the owner's original "genesis record ended
      mid-field" bug were it ever sent back to `read_feed`).
      **What does catch a broken guard:** three *other*, pre-existing tests in the same
      file fail under this mutation (`test_a_share_is_offered_only_where_the_view_holds_the_record`,
      `test_a_share_string_reaches_the_clipboard_sink_verbatim`,
      `test_no_share_button_is_on_screen_for_a_row_whose_record_is_not_held`) — but only
      incidentally, because they happen to seed `genesisByStoa` directly and then call
      `reload()`, which overwrites the seeded value with the mutated write. None of them
      was written to pin this guard, and a change to their fixtures could stop catching it
      without anyone noticing, since the guard-direction test that reads as the
      purpose-built regression test for this does not itself fail.
      **Measured:** 81/81 QML tests in `tst_stoa_screens.qml` pass at baseline; under the
      guard-removal mutation, 78/81 pass and the 3 failures above are the ones that catch
      it — the named 4.4 test is not among them, in either the full-removal or the
      narrower "coerce to `\"\"` and still write" variant of the mutation. Mutation
      reverted; file is back to the committed state (`git status` clean).
      **Fix direction (not prescribing the exact assertion, leaving that to dev-writer):**
      assert on `screen.genesisByStoa` directly for at least the negative case — e.g.
      `verify(!screen.genesisByStoa.hasOwnProperty(missing))` and the same for `empty` —
      so the test observes the map's actual contents rather than only the value
      `genesisFor` computes from them.

      **Fixed, implemented exactly as your fix direction prescribes.** The test
      now asserts `!screen.genesisByStoa.hasOwnProperty(missing)` and the same
      for `empty`, placed ahead of the existing `genesisFor`/`canShare`
      assertions, which are kept. No production change was needed —
      `genesisByStoa` is already a public property.

      **Proved**: under the guard-removal mutation the suite goes to **80
      passed, 1 failed**, and the one failure is this test, on *"an empty field
      must leave no key either"*. At baseline it is 81/81. Mutation reverted.

      The part of your finding I want to record as the durable lesson is the
      second half, because it is the part that would otherwise be lost: the
      three pre-existing tests that *did* catch the mutation caught it
      **incidentally**, because they seed `genesisByStoa` and then `reload()`
      overwrites the seed. None was written to pin this guard, so a fixture
      change in any of them could have removed even that accidental coverage
      with nothing reporting it. That is why the fix had to be in the
      purpose-built test rather than resting on the three. It is in `design.md`
      under "The guard against an empty record is asserted against the map's
      keys", along with the general form: ask what the null implementation
      would produce, and a lossy accessor is the usual way an assertion that
      cannot fail gets written by accident.

      Your scope framing is carried over too — this was a test-strength gap,
      not a live vulnerability: the guard's code was correct and present
      throughout, and remains unchanged.

## What I did not find

No defect in the core (`wire.rs`) verification story, the round-trip of stored bytes, the
error-path shape, or the share affordance's contents. The one finding above is a
test-strength gap on the QML side, not a live vulnerability today — the guard's code is
correct and present; what's missing is a test that would notice if it were later removed
or weakened.
