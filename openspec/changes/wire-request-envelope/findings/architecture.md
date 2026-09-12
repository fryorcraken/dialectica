# Architecture findings — `wire-request-envelope`

**Reconstructed from the commit record** — see the note at the top of
`correctness.md` for why that is itself a finding about how this piece was run.

---

## A1. `Request`'s guarantee was false for exactly the population it named — **Fixed**

For: `dev-writer`

This is the piece's central finding, and it is a language fact rather than a
style judgement.

`Request` is a tuple struct with a private field, and its doc claimed that a
handler holding one **provably went through the envelope check**. But a tuple
struct's private field is private to its **defining module**, not to its defining
type. `Request` lived in `wire.rs` beside every handler, so this compiled and ran
from inside that file's `mod tests`:

```rust
let bypass = Request(serde_json::Map::new());
let inner_read = bypass.0.len();               // compiled, ran, returned 0
```

The guarantee was therefore false for precisely the set of code it was making a
guarantee about — every handler. And the residual `design.md` conceded against it
("a future handler could call `serde_json::from_str` itself", mitigated by that
being visible in review as an anomaly) covered the **wrong hole**: neither line
above contains a `from_str`, and `Request(map)` would have looked like nothing at
all in review.

**Fixed** in `3bc2cbc` — structurally, not by rewording the doc. `Request` and
`REQUEST_NOT_AN_OBJECT` moved to `wire::request`, a file containing no handler and
no second constructor. The bypass now fails to compile from `wire.rs` with
`E0423`, verified verbatim.

The proof is split, deliberately, because half of it is not assertable at runtime:

- `wire::request::tests::request_is_constructible_here_because_this_module_defines_it`
  compiles the bypass **inside** the defining module, where it is legal. That is
  what makes the compile failure in `wire.rs` attributable to the boundary rather
  than to a typo.
- `wire::tests::the_bypass_this_module_boundary_closes` records the verified
  compiler error verbatim and says plainly that it is a note, not a test. A
  compile-fail assertion would need a `compile_fail` doctest compiled as an
  external consumer — which proves the field is private to *other crates*, a
  weaker statement than the one at issue.

The boundary is only as good as the rule that this file holds no handler, which
is why that rule is stated in the module doc rather than left to be inferred.

## A2. What the module boundary does **not** buy — **Fixed (by building it)**

For: `dev-writer`

Asked whether the module move makes every handler checked: **it does not**, and
this was established by construction rather than by argument.

A sixth method was built — a handler parsing `Value` directly with all-optional
fields — and it served `[]` as a request that named nothing, with the whole suite
green. It was **rebuilt after the module move** and still compiles and still
serves `[]`: nothing obliges a handler to hold a `Request` at all. The boundary
closes one hole (constructing a `Request` without `parse`); it cannot close the
hole of skipping the type.

**Fixed** in `66de130` by keeping the handler as a test —
`the_sixth_method_the_boundary_does_not_stop` — so the scope of the fix is
demonstrated rather than asserted. `design.md` §1 now states the guarantee in the
narrow form: "a handler that reads fields **through** `Request` went through the
check", never "every handler is checked".

This is the finding that generated the measurement in A3.

## A3. The recorded mitigation was measured not to work — **Fixed (`4606f8d`)**

For: `design-reviewer` → `dev-writer`

Follows directly from A2. With the compiler unable to force the sweep, the
residual `design.md` recorded was "the sweep in `every_request_taking_method` and
review. Both are human, and that is the residual."

The reviewer then established **why the obvious mechanisation is impossible
here**, and this is the finding that the archive would otherwise actively
mislead on. A trait-driven sweep — enumerate the wire surface from the dispatch
trait and assert every request-taking method is in the list — cannot be written,
for two independent reasons, both verified against the code:

1. **The dependency points the wrong way.** The trait (`DialecticaModule`) lives
   in the `dialectica` crate, which *depends on* `dialectica-core`
   (`rust-lib/Cargo.toml:32`). `dialectica-core` cannot see it. A sweep asserting
   over the trait would have to live in `dialectica`.
2. **And `dialectica` cannot run it.** The whole trait and its impl are behind
   `#[cfg(logos_scaffold)]` (`rust-lib/src/lib.rs:157`, `183`, `193`), a cfg
   `build.rs` sets only when the builder has staged `generated/provider_gen.rs`.
   A plain `cargo test` never sets it, and per the comment at
   `rust-lib/src/lib.rs:130` "no arrangement makes it able to" — committing a copy
   of the generated file would recreate the contract/code drift the generator
   exists to prevent.

So a trait-driven sweep would live in the one crate that cannot run it. That is
worth as much as the review that produced it, and unwritten the next agent spends
the same afternoon on it.

**Fixed** in `4606f8d`: recorded in `design.md` as a rejected alternative with its
reason, beside the `include_str!` alternative it sits next to. Also records that
the sixth method was **built and measured** — 487 tests passing with it in place —
because "the mitigation is reviewer attention" reads very differently once you
know reviewer attention was the thing measured to fail.

## A4. An `include_str!` sweep test — **Rejected, recorded**

For: `dev-writer`

The other obvious mechanisation of the A2 residual: a test that `include_str!`s
`wire.rs` and looks for a `serde_json::from_str`, or a `pub fn` taking a request,
that `every_request_taking_method` does not account for.

**Rejected**, and the argument is about false positives rather than effort: it
would fail for three reasons other than the one it names — a doc comment
mentioning `from_str` (this file has several, deliberately, because the two-step
parse is worth explaining), a test helper legitimately parsing a fixture (several
of those too), and a reply *decoder*, which is the opposite direction entirely. A
test that goes red for three unrelated reasons is a test the next author learns to
delete or `#[ignore]`, and the real signal goes with it.

**Recorded** in `design.md`'s Rejected alternatives, so the next author who
reaches for it finds the objection stated rather than re-litigating it. The
obligation lives instead where an author adding a method has to be:
`every_request_taking_method`'s doc says **ADD YOUR METHOD HERE**, states that
nothing checks it, and names the measured sixth-method result as the reason.

## A5. The check belongs in one place, not per handler — **Fixed (as designed)**

For: `dev-writer`

Recorded here because it is the decision the rest of the piece rests on, and a
reviewer confirmed it rather than proposed it.

An `if !parsed.is_object()` per handler was the smaller diff. Rejected: a branch
has to be got right at every call site, "is it checked everywhere?" becomes a
question you answer by reading every handler, and a new method that forgets it
compiles, passes clippy and silently reintroduces the defect — **which is how this
defect arrived**. The type puts the invariant where the data is shaped, so a
method written next month inherits it without its author knowing this change
happened.

Same argument applied one step further along for the size cap (S1 in
`security.md`) and once more for the double parse (C1 in `correctness.md`, where
`&Request` in a private signature makes the second parse unspellable). The
consistency is the point: three findings, one shape of fix.
