# Tasks

## 1. Core: the regression tests, written first

- [x] 1.1 `genesis_of`, a helper that decodes a reply's record and asserts its
  address equals the address the SAME reply names. The relation rather than a
  pinned literal: a reply carrying a well-formed record of some other Stoa
  satisfies "the field decodes" and is useless to the caller.
- [x] 1.2 A creation reports the record its address is the hash of. **Proved to
  fail first** — `the reply must carry a 'genesis' record…: {"foundingTitle":
  "Agora","policy":"open","stoa":"80329cf0…"}`.
- [x] 1.3 A listed Stoa carries it too. Proved to fail first, same message
  against the listing's item shape.
- [x] 1.4 The record a listing reports round-trips through the call that takes
  one. This is what pins both sides to ONE encoding — a reply carrying base64
  would pass 1.2 and 1.3 and still be refused by the handler that reads it.
  Proved to fail first (`Option::unwrap()` on a `None`).

## 2. Core: the widening

- [x] 2.1 `GENESIS` constant, documented with why an address cannot substitute
  for a record and why the encoding must match what `join_stoa` reads.
- [x] 2.2 `stoa_reply` carries it — this serves BOTH create and join, which is
  why the constant's docstring states the reason is a property of neither call.
- [x] 2.3 `membership_page_json` carries it per item. Rewritten from a `map`
  closure to a loop so an encode failure can return the error shape; a closure
  could only have swallowed it or panicked.
- [x] 2.4 An encode failure is the error shape, never an empty field. An empty
  record is exactly the input that fails to decode as "ended mid-field", so
  defaulting to one would reintroduce the defect as a success reply.
- [x] 2.5 The two handler docstrings' reply shapes updated to match.
- [x] 2.6 Full suite green: 1051 tests, 0 failed. `cargo fmt --check` clean on
  both crates — with `-p dialectica -p dialectica-core`, which is what reaches
  `dialectica-core` at all.

## 3. The view

- [x] 3.1 `rememberGenesis(stoa, genesis)`, which records a non-empty string and
  otherwise leaves the map untouched. Writing `""` in would make `canShare` true
  for a Stoa whose share text cannot be built and would send that same `""` back
  to `read_feed`.
- [x] 3.2 The map is REASSIGNED rather than mutated in place. A QML `var`
  property does not notify on an in-place key write, so bindings on `canShare`
  would not re-evaluate and a share button would stay hidden.
- [x] 3.3 `reload()` records every item's record, additively, before the rows
  render — so paging away from a Stoa does not drop its record.
- [x] 3.4 `create()` records the creation reply's own record rather than leaving
  it to the reload: a new Stoa need not land on page 0.
- [x] 3.5 The stale comment naming this as unfixable in the view, and the one
  saying a creation reply carries no record, replaced with what is now true.

## 4. The view's tests

- [x] 4.1 A listed Stoa is openable from the reply alone. **Nothing is seeded** —
  the older tests in this file write `genesisByStoa` directly, which is exactly
  why the defect survived them: seeding the map assumes the missing thing.
- [x] 4.2 A created Stoa is openable and shareable from the creation reply alone.
- [x] 4.3 A record survives paging away from the Stoa that carried it.
- [x] 4.4 An item short of its record is not recorded as an empty one, and the
  listing is still a successful read. The guard direction.
- [x] 4.5 **4.1, 4.2 and 4.3 proved to fail** with the view change set aside —
  each on the empty string, e.g. `Actual (): ` vs `Expected (): 01cccc…74657374`.
  4.4 holds in both directions by design; it pins that nothing writes `""`.
- [x] 4.6 Full QML suite green: 411 passed, 0 failed across 20 spec files. The
  three static gates green — names, members, reachability.

## 5. Proof by launching

- [ ] 5.1 `lgs basecamp build --variant lgx`, `install`, `launch alice`.
- [ ] 5.2 Create a Stoa, press **Open**, and confirm the feed reads rather than
  reporting "genesis record ended mid-field". A green suite is not proof here — a
  broken bridge has shipped under a green suite in this repo before.
- [ ] 5.3 Confirm the share affordance is now offered, which the same lookup
  gated.
- [ ] 5.4 `git diff scaffold.toml` after every `lgs basecamp` verb; restore if
  the file was rewritten.
