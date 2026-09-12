# Design-review findings — `wire-request-envelope`

From `design-reviewer`: **did the code take the decisions that were recorded, and
were the decisions worth recording recorded?** **Reconstructed from the commit
record** — see the note at the top of `correctness.md` for why that is itself a
finding about how this piece was run.

---

## D1. Decision 1 claimed a guarantee the code did not provide — **Fixed**

For: `dev-writer`

The sharpest kind of finding this role can make: not a missing decision, but a
recorded one that was **false**.

`design.md` Decision 1 said "the inner map is private and `parse` is the only
constructor, so a handler holding a `Request` provably went through the check".
Private-to-module, not private-to-type; every handler was in that module. The
mechanism is recorded in full as A1 in `architecture.md`.

**Fixed** in `509df91`. Decision 1 now records the bypass, the module move that
closes it, the verified `E0423`, and why the proof is split between a test inside
the defining module and a note in `wire.rs`.

## D2. The residual named was not the residual that was open — **Fixed**

For: `dev-writer`

Decision 1 conceded "a future handler could call `serde_json::from_str` itself",
mitigated by that being "visible in review as an anomaly".

That mitigation is real, and it covered the **wrong hole**. The one actually open
was `Request(map)` — which contains no `from_str` and would have looked like
nothing at all in review. A recorded residual that names the wrong risk is worse
than none: it tells a reader the hole was considered.

**Fixed** in `509df91`. The Risks section now names the real residual (a handler
that never holds a `Request` at all), keeps the old wording as a recorded
correction so the mistake is visible rather than quietly replaced, and says
plainly: **do not read Decision 1 as "every handler is checked"**.

## D3. Four decisions existed only in the code — **Fixed**

For: `dev-writer`

Each was a real choice with an alternative, made in the code, with nothing in the
archive saying why.

- **§6, the 4 MiB cap.** With the measurement that made it necessary, the
  derivation of the number from `op.rs`'s measured 768,076-byte `Post`, and — the
  part that matters most — **the cap and the unknown-field leniency recorded as
  ONE decision**. Ignoring unknown fields is what made the padded request valid;
  dropping the cap also costs the leniency, so a future reader prices both.
- **§7, the explicit-null reading.** The rejected alternative is named
  (`.filter(|v| !v.is_null())` in `get`) and ruled out **on measurement**: four of
  seven readers observe the difference and `ping` flips from success to error.
  Records that the 486/487 survival was a coverage gap and **not** evidence that
  nothing observed it — the conclusion first drawn from it, and wrong. Also states
  that PLAN §9.1's "Absent, not null-and-present" governs the reply half and does
  not bind requests in either direction, rather than claiming a departure.
- **§8, duplicate keys, accepted last-wins.** See S4 in `security.md`.
- **§9, serde's recursion bound is being relied on.** See S5 in `security.md`.

**Fixed** in `509df91`.

## D4. A rejected alternative had gone unrecorded — **Fixed**

For: `dev-writer`

The `include_str!` sweep test. Recorded as A4 in `architecture.md`.

**Fixed** in `509df91`. The reason to record a *rejection* is the point: without
it the obligation lives only in `every_request_taking_method`'s doc comment, which
is where it belongs but is not where someone deciding to build the test would
look.

## D5. `tasks.md` recorded present state in a form that cannot fail loudly — **Fixed**

For: `dev-writer`

Two instances, and they fail differently:

- **5.1's "485 pass"** — a count, which is exactly what `CLAUDE.md`'s
  "keeping this file true" section says to replace with the command. It goes
  quietly wrong as parallel branches land.
- **5.2's "`cargo fmt --check` — exit 0"** — **true and vacuous.** The workspace
  manifest has no `members`, so the gate never reaches `dialectica-core`, where
  nearly all the logic lives. An exit 0 from a gate that measured nothing is worse
  than no gate, and reporting it as passed is the failure the flow README names.

**Fixed** in `509df91`. Both now name the command and say what it cannot see. The
replacement for the fmt gate is per-file `rustfmt --check` with
`skip_children=true`.

## D6. `docs/UI-BRIEF.md` — **Rejected (no change needed), and recorded as a judgement**

For: `dev-writer`

The brief is a live document designed against by someone who cannot read the code,
and `CLAUDE.md` requires a change that invalidates it to fix it in the same
change. The last Risk in `design.md` says the refusal message is contract surface
a view may render — which is the kind of claim the brief carries.

**Assessed and rejected as needing no change**, recorded rather than left as
silence: the brief states what a view must show, hide or refuse to claim and does
not enumerate core's error strings, and all four refusals this change adds or
reworks arrive through the single `{"error":"..."}` shape the brief already tells a
designer to render as one branch. **A view renders the string; it is not asked to
recognise it.**

The note also says what would change that — a view given reason to *branch* on
which refusal came back — so the next person to add a refusal has the test rather
than the conclusion.

## D7. The recorded mitigation was the one measured not to work — **Fixed (`4606f8d`)**

For: `dev-writer`

The last open design finding, and the one place the archive would actively
mislead. Recorded in full as **A3 in `architecture.md`**: the reviewer established
that a trait-driven sweep is impossible here (the trait lives in the crate that
depends on `dialectica-core`, not the reverse, and sits behind a `cfg` `cargo test`
never sets, so the sweep would live in the one crate that cannot run it), and
**built the sixth method, measuring 487 tests passing with it in place**.

Before this pass, `design.md` told a reader the residual was "the sweep and
review. Both are human." That is true and it omits the two things worth knowing:
that the mechanical alternative was investigated and is structurally unavailable,
and that human attention was the thing **measured** to fail. A reader would
reasonably conclude nobody had tried.

**Fixed** in `4606f8d`, as a rejected alternative in `design.md` beside the
`include_str!` entry, carrying both the impossibility argument and the
measurement.
