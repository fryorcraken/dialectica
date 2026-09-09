# Phase 0 findings

Phase 0 existed to retire one risk before anything depended on it (PLAN.md §9,
§13): **does the `dependency_overrides` LIDL bridge to `delivery_module`
work?** — plus two cheap questions from §2.3, about the SDK revision the
builder's pin delivers and about what a panic in a handler actually does.

This records what was *found*, and what the finding cost. What builds today,
what version anything is at, and how many tests pass are questions a command
answers; they are named here rather than written down.

---

## 1. The `dependency_overrides` bridge: **no, not as PLAN.md §3.2 wrote it — and yes, with one conversion step**

### What was tried, verbatim

PLAN.md §3.2 proposed pointing the override at delivery's impl header:

```json
"dependency_overrides": {
  "delivery_module": {
    "file": "src/delivery_module_plugin.h",
    "input": "delivery_module",
    "impl_class": "DeliveryModuleImpl"
  }
}
```

That evaluates, resolves, and then fails inside the generator:

```
logos-dialectica-rust-scaffold> Failed to parse dep
  /nix/store/knmz1w5jp4b1y35xjw8wvcsdg9dcfdvd-source/src/delivery_module_plugin.h:
  parse error at line 1 column 1: Unexpected character '#'
```

The `#` is `#pragma once`.

### Why, and why the error names nothing useful

**The Rust and C++ generators do not have the same frontends, and
`dependency_overrides` was built for the one that does.**

The builder resolves every concrete dependency to a `{name, path, impl_class}`
triple and then feeds it to *both* backends — but they consume it differently:

- The **C++** backend gets `staticDeps` including `impl_class`, and reaches a
  `logos-cpp-generator --header-to-lidl` step. It can turn a header into a
  contract.
- The **Rust** path builds `--dep <name>=<path>` flags and hands them to
  `logos-lidl-gen`, whose `--dep` handler calls the **LIDL parser** on whatever
  path it is given. There is no `--header-to-lidl` on that side, and
  `impl_class` is dropped: nothing on the Rust path ever reads it.

So `impl_class` is accepted by the metadata parser — it is even *required* when
`file` ends in `.h` — and then silently has no effect. **That is the trap**: the
field's presence reads as "headers are supported here", and the validation that
demands it reinforces the reading. It is supported, for C++ consumers only.

### What works instead

Do the header→LIDL conversion **once, outside the build**, and commit the
result. The C++ generator will do it on demand:

```
logos-cpp-generator --header-to-lidl <delivery>/src/delivery_module_plugin.h \
  --metadata <delivery>/metadata.json --impl-class DeliveryModuleImpl \
  -o contracts/delivery_module.lidl
```
```
Generated LIDL: .../delivery_module.lidl (15 methods, 10 events)
```

The override then points at a real `.lidl` and drops `impl_class`, which a
`.lidl` does not need:

```json
"dependency_overrides": {
  "delivery_module": { "file": "contracts/delivery_module.lidl" }
}
```

`dialectica/contracts/regenerate-delivery-lidl.sh` is that command, kept so
"is our copy current?" stays a command (`git diff` after running it) rather
than a memory.

### The bridge genuinely works — evidence, not assertion

A build succeeding proves less than it looks like: a generator can emit a
dependency client nobody calls. So the trait calls one, and the *compiler* is
the witness. An early version got the argument type wrong, and the error is the
proof:

```
error[E0308]: mismatched types
   --> src/lib.rs:164:60
164 |  match modules().delivery_module.channel_exists(channel_id) {
    |                                  -------------- ^^^^^^^^^^ expected `&str`, found `String`
note: method defined here
   --> /build/logos-dialectica-rust-src/rust-lib/generated/provider_gen.rs:997:16
997 |  pub fn channel_exists(&self, channel_id: &str) -> Result<serde_json::Value, LogosError> {
```

A *type mismatch on a generated method* is much stronger evidence than a green
build: it can only happen if the builder generated a typed Rust client from
delivery's contract, with the right name (`channelExists` → `channel_exists`)
and a real signature. That was the whole question.

`delivery_channel_exists` in the trait is deliberately the cheapest delivery
call with no side effect and no node required, so a failure there is a bridge
failure rather than a network one.

### What this changes, and what it does not

**The architecture stands.** PLAN.md §3.2 called the bridge "the single
unproven link" and said the architecture changes if it does not work. It works
— it just needs a conversion step that §3.2 did not anticipate. Consuming
delivery from Rust stays viable, so nothing in §4 (SDS transport) or §3 needs
revisiting.

What it costs is a **checked-in copy of somebody else's contract**, which can
go stale silently. Two consequences to carry forward:

- Re-run the regeneration script whenever the delivery pin moves, and read the
  diff. A removed method is a call site that stops compiling; a *changed
  signature* is one that might not.
- The right fix is upstream, and it is small: teach the Rust `--dep` path to
  run the C++ frontend when the path ends in `.h`, exactly as the C++ path
  already does. Worth raising alongside the panic guard (§3), since both are
  contributions to the same repos and both are ~30-line changes.

---

## 2. The SDK revision the builder's pin delivers

**The headline is not the gap — it is that the gap depends entirely on the
builder pin, and the pin moved during Phase 0 for an unrelated reason.**

**Which rev** is a question for a command, and it changes:

```
grep -A 30 '"logos-rust-sdk": {' dialectica/flake.lock   # what we actually link
gh api repos/logos-co/logos-rust-sdk/commits/master      # what the README describes
```

### The measurement, and why it was taken twice

Measured against the source the builder actually stages — `nix build
<builder>#rust-sdk-src`, i.e. the bytes that get compiled, not GitHub's HEAD.
The first measurement was against builder tag **0.2.6**; the pin then had to
move to a master commit to get a Rust module building at all (§4's crates.io
trap), and every answer changed:

| §2.3 asked | Builder 0.2.6 | Our pin (a master commit) |
|---|---|---|
| subscription status / restart | **absent** | **present** (`on_subscription_status`) |
| per-call timeouts | **absent** | **present** (`<method>_with_timeout` twins) |
| argument type checking | **absent** | **present** |
| panic guard (`catch_unwind`) | absent | **still absent** |

0.2.6 delivered an SDK roughly two dozen commits behind master. The pin we ended
up on delivers one a single commit behind it.

**So the §2.3 worry was real and is, for now, retired — by accident.** Nothing
about dialectica caused this; the pin moved to fix a crate download, and the SDK
came along with it. That is precisely why §2.3 says to check rather than assume:
the answer is a property of the builder pin, and it can move backwards just as
easily if someone pins a tag for a good-looking reason.

### What is present, concretely

- **`recv()` no longer blocks forever on a dead provider.** The subscription
  loop uses `recv_timeout` on a death-poll interval and there is an
  `on_subscription_status` callback, so a listener thread learns its provider
  went away. This is the one that mattered most: for dialectica the listener is
  the `channelMessageReceived` ingest thread, and its failure mode was "the Stoa
  silently stops receiving" with nothing logged anywhere.
- **Per-call timeouts exist as parallel entry points** — `add_with_timeout`
  beside `add` — rather than as a parameter, because Rust has no default
  arguments and adding one would break every call site. The generator's own
  comment calls the twinning a deliberate stopgap.
- **The default call timeout is 20 seconds** (protocol default, `timeout_ms <= 0`).
  That number is not academic: it is exactly the delay §3's unguarded panic
  imposed before the caller heard anything.

### What is still absent, and is the one that matters

**No `catch_unwind`, at our pin or at HEAD** — `grep -rn catch_unwind` over both
is empty. §3 is what that costs.

### Consequence for the design

Do not design against upstream's README, and do not assume a pin bump is
capability-neutral in either direction. Two standing rules:

- **Re-run this measurement whenever the builder pin moves**, especially if it
  moves *back* to a tag. Everything in the table above is a pin-dependent fact.
- Where a needed guarantee is missing at the pin, bound it ourselves (as §2.3
  says for the event queue) or move the pin deliberately and say why in the
  flake — which is what `dialectica/flake.nix` now does at length.

---

## 3. What a panic in a dispatch handler actually does

**Answered experimentally**, as §9 asked — a real `logoscore` daemon, a real
module process, an unguarded handler called over IPC. Not inference.

### The result is worse than PLAN.md §2.3 predicted

§2.3 expected a poisoned mutex bricking the module for the process lifetime.
What actually happens is more abrupt: **the module process aborts.**

The apparatus was a throwaway build of this module carrying two handlers — the
normal guarded `panic_probe`, and a `panic_unguarded` that panics with no guard
at all — so the two paths differ in exactly one thing.

**Guarded** — the panic is caught, the wire contract holds, and the module
keeps serving:

```
$ logoscore call dialectica panic_probe '{}'
{"method":"panic_probe","module":"dialectica",
 "result":"{\"error\":\"panic in panic_probe: panic_probe was asked to panic with: {}\"}",
 "status":"ok"}

$ logoscore call dialectica version          # still alive
{"method":"version","module":"dialectica","result":"{\"version\":\"0.1.0\"}","status":"ok"}
```

**Unguarded** — the call does not fail, it *hangs*, and then the module is gone:

```
$ logoscore call dialectica panic_unguarded '{}'
{"code":"METHOD_FAILED","error":{"code":"timeout",
 "message":"call to 'dialectica.panic_unguarded' timed out after 20000ms",
 "origin":"dialectica"},"status":"error"}

$ logoscore call dialectica version          # the module is no longer there
{"code":"MODULE_NOT_LOADED","message":"Module 'dialectica' is not loaded. ...","status":"error"}
```

The daemon log says why, and this is the line that matters:

```
[dialectica] thread '<unnamed>' panicked at src/lib.rs:137:9:
[dialectica] unguarded panic with: {}
[dialectica] fatal runtime error: failed to initiate panic, error 5, aborting
[logos] [dialectica] FATAL: module 'dialectica' crashed (signal 6). Backtrace ...
[logos] Module process crashed: dialectica
```

**"failed to initiate panic, error 5"** is the unwinder refusing to cross the
`extern "C"` frame. Rust does not poison a mutex and carry on — it aborts
(signal 6, SIGABRT). So the §2.3 mechanism is real but it is not reached: the
process dies before any later dispatch can meet the poisoned lock.

Three details worth carrying forward, none of them predictable from the source:

- **The caller waits 20 seconds before learning anything**, and then is told
  `timeout` — a message that points at a slow provider, not a dead one. The
  20s is the protocol default call timeout (§2), so this is the *floor*: a
  handler using a `_with_timeout` twin would hear back sooner, and still hear
  the wrong thing.
- **The failure changes shape between the first call and the next.** The call
  that killed the module reports a timeout; the *next* call reports
  `MODULE_NOT_LOADED`. Neither says "panic", and only the daemon log does.
- **Nothing restarts it.** The module stays unloaded until something loads it
  again.

### Two things the same run proved in passing

Both are §2.2/§2.3 claims that had not actually been exercised:

- **A `codegen.rust` module serves real IPC.** `logoscore call dialectica
  version` returns `{"version":"0.1.0"}` from the Rust handler, through the
  generated dispatch, over the daemon's socket. The whole path works.
- **`on_context_ready` fires, and the context is populated.** The module logs
  `dialectica ready: instance ad3b9de07272 (persistence: .../data/dialectica/
  ad3b9de07272)` at load. So the host-stamped instance id and per-instance
  persistence path are both real, which is what Phase 1's store will hang off.

### Why the underlying mechanism still matters

Both §2.3 facts hold at the rev we link, and each was checked rather than
assumed:

1. **There is no `catch_unwind` anywhere in the SDK** — not at the pinned rev,
   not at HEAD. `grep -rn catch_unwind` over both is empty.
2. **The generated dispatch holds the instance lock across the call and
   unwraps it** — `let mut guard = INSTANCE.0.lock().unwrap();` — so a panic
   that *did* unwind would poison it.

The abort simply gets there first. That matters for the upstream fix: adding
poison-tolerant locks alone would not have saved this module, because the
process was already gone. **The `catch_unwind` is the load-bearing half**; the
poison tolerance is the belt to its braces.

### How defensive the guard must be, which is what §9 wanted to know

**Maximally, and uniformly.** The experiment settles this rather than leaving it
to judgement: the damage is to the *process*, not to the call, so there is no
such thing as a handler too trivial to guard. A panic in `version()` takes
`ping()` down with it, along with every other method and any background thread
the module owned.

So `core::guarded` wraps *every* handler body, and the adapter in `lib.rs` holds
no unguarded code at all — including the cross-module call in
`delivery_channel_exists`, since a panic while decoding a peer's reply is a
panic in a dispatch handler like any other.

This is also why the guard cannot be "added later where it matters". There is no
subset where it matters less.

The guard's own details each pay for themselves:

- **Both panic payload types are handled.** `panic!("literal")` yields `&str`,
  `panic!("{}", x)` yields `String`. Handling one loses the message for every
  panic of the other kind — which is most of the interesting ones.
- **The error message is JSON-escaped.** A panic payload is attacker-influenced
  the moment a handler formats untrusted input into it, and naive concatenation
  would emit malformed JSON — turning a diagnosable failure into a parse error
  at the view.
- **`AssertUnwindSafe` is an assertion we must actually keep**: handler bodies
  should not leave observable state half-written before panicking. Note the
  alternative is not "safe state" — it is a poisoned lock and a dead module.

### The guard is proven, not asserted

A guard with nothing exercising it is a claim no gate can see. So:

- `panic_probe` is a contract method that panics on purpose. It is Phase 0
  apparatus and should be removed once this question stops being live.
- The guard tests were **watched failing before they passed**, per CLAUDE.md.
  Stubbing `guarded` out to `f()` turns 6 of the suite's tests red; restoring it
  turns them green. Note the failure mode: without the guard they do not fail,
  they *abort the test binary* — which is the same "one panic takes everything
  down" shape, in miniature.

### Upstreaming

A `catch_unwind` in the dispatch emitter remains the highest-value contribution
available (§2.3), and the experiment sharpens the argument for it: **today every
Rust module in the ecosystem is one panic away from a dead process**, and the
operator-visible symptom is a 20-second timeout followed by `MODULE_NOT_LOADED`,
with the word "panic" appearing only in a daemon log.

The ordering correction is worth carrying upstream too. §2.3 framed poison
tolerance and the guard as two halves of one fix; the abort shows the guard is
the one that does the work, and poison tolerance only becomes reachable once
panics stop aborting.

---

## 4. New traps found

In PLAN.md §11's style — structural, and each one cost a debugging cycle here.

- **`dependency_overrides` pointing at a `.h` does not work from a Rust
  module**, though `impl_class` is required and validated for exactly that case.
  Only the C++ generator has a header frontend. Convert the header to `.lidl`
  out of band and point the override at that. (§1)

- **`lgs basecamp install` alone does not install a module's declared
  `dependencies`.** It builds the `[modules.*]` project sources and nothing
  else; the `dependencies` array in `metadata.json` is never consulted.
  `lgs basecamp modules` is the verb that captures project sources *plus*
  runtime dependencies — run it first, then `install`, which then reports
  "installing 3 module(s) (deps first)".

  **The failure is silent in every place you would look for it.** The build
  stays green, because `dependency_overrides` is a build-time concern and works
  regardless. `lgs basecamp modules --show` lists the dependency correctly even
  when it has never been installed, because capture and installation are
  separate steps. The symptom appears only at load, as a launcher tile that
  does nothing when clicked:

  ```
  Module not found in known modules: delivery_module
  Missing dependencies detected: delivery_module
  Cannot resolve dependencies for: dialectica
  Failed to load core dependency "dialectica" for "dialectica_ui"
  ```

  This is PLAN.md §8.1's indistinguishability in its most expensive form: the
  same dead tile is produced by a missing icon, a wrongly-sized icon, a QML
  parse error, an unclicked plugin, and this. Read the profile log before
  forming any hypothesis about which.

  **The trap inside the trap:** a UI module failing this way proves *less* than
  it appears to. Resolution happens before any QML is instantiated, so four
  failed clicks say the tile existed and the icon resolved — they say nothing
  about whether `Main.qml` parses. Do not read "the launcher responded" as "the
  view works".

- **`lgs basecamp setup`, `modules` and `install` each strip every comment from
  `scaffold.toml`.** PLAN.md §11 names only `setup`, which is why this was
  rediscovered: the comments came back after a restore and vanished again on
  the next unrelated verb. Treat *any* `lgs basecamp` verb as comment-
  destroying, run `git diff scaffold.toml` after each, and restore in one pass
  at the end rather than after every command.

- **The newest builder *tag* cannot build a Rust module on a cold cache.**
  `importCargoLock` fetches crates through a nixpkgs `fetchurl` that sends no
  User-Agent, and crates.io answers those with 403:

  ```
  curl: (22) The requested URL returned error: 403
  error: cannot download crate-syn-3.0.5.tar.gz from any mirror
  ```

  It reads as a dead mirror rather than a policy. Tell them apart in one
  command — `curl -o /dev/null -w '%{http_code}'
  https://crates.io/api/v1/crates/syn/3.0.5/download` gives 403, and the same
  with `-A anything` gives 200.

  **The radicle workaround does not transfer.** That module fixes it in its own
  flake with a second nixpkgs feeding `fetchCargoVendor`; on the `codegen.rust`
  path the *builder* owns the vendoring, so a module's flake has no override
  point. Both module flakes therefore pin a **commit past the newest tag**, with
  the reason and the move-me-to-a-tag instruction written at the pin.

- **`cargo test` cannot compile a `codegen.rust` crate at all**, and the error
  says nothing about why:

  ```
  error: couldn't read .../generated/provider_gen.rs: No such file or directory
  ```

  The scaffold is generated into the builder's *own staged copy* of the crate,
  so it never exists in a checkout — and committing one would recreate exactly
  the contract/code drift `codegen.rust.trait` exists to prevent. Two things
  are needed together, and each is useless alone:

  - `crate-type = ["staticlib", "rlib"]`. A bare staticlib has no test harness.
  - A `build.rs` setting a cfg when the scaffold file is present, gating the
    module surface on it.

  **The trap inside the trap:** gating on `#[cfg(not(test))]` looks equivalent
  and is not — a plain `cargo build` still tries the include and still fails.

  The consequence is architectural rather than annoying, and it is the same
  force PLAN.md §2.3 describes: **whatever is not in the SDK-free module is
  untestable.** `src/core.rs` holds every decision and `src/lib.rs` holds none,
  because that is the only arrangement in which any of it can be tested. Phase 1
  inherits this shape rather than migrating to it.

- **The contract trait must stay at the top level of `src/lib.rs`, ungated.**
  The generator reads that file as *text* (`syn`), so a trait moved into a
  submodule or behind a `cfg` is a trait the contract loses — silently, with a
  smaller `.lidl` and a build that still succeeds. Where the trait mentions a
  scaffold-defined type (`RustModuleContext`), a `#[cfg(not(logos_scaffold))]`
  stand-in keeps the ungated build compiling.

- **A Rust-first module's published `.lidl` says `depends []` even when
  `dependencies` names modules.** The `--from-rust` derivation reads the trait,
  which has no way to express a dependency, so the field comes back empty:
  `nix build .#lidl` then `grep depends`. A consumer generating bindings from
  dialectica's published contract therefore does not learn that dialectica needs
  delivery. Harmless while nothing consumes us; worth knowing before something
  does.

- **An unguarded panic aborts the module process, and the caller is told
  `timeout`.** Not "the call failed", and not the poisoned-mutex behaviour
  §2.3 predicted. The first call blocks for 20s and reports a timeout; the next
  reports `MODULE_NOT_LOADED`; the word "panic" appears only in the daemon log
  (§3). Anyone debugging from the client side alone will chase a slow provider.

- **`lgs` refuses a scaffold.toml without `[repos.lez]`** — "invalid
  scaffold.toml: missing [repos.lez]" — even for a module-only project that
  never touches the execution zone. The validator is shared with the LEZ project
  shape. The `[repos.lez]` and `[repos.spel]` entries in this repo exist to
  satisfy that check and are fetched by no verb this project runs; they are not
  a dependency on LEZ.

---

## 5. One deliberate deviation from PLAN.md

**`dialectica/rust-lib/build.rs` exists, and PLAN.md §2.2 says "no `build.rs`".**
Flagged here as a decision rather than left to be found as a slip.

§2.2's point is that the Rust path needs no build script *to build the module* —
the builder generates the scaffold and stages the SDK, so nothing is generated
crate-side. That remains true, and this `build.rs` generates nothing. It reads
one thing (is `generated/provider_gen.rs` present?) and sets one cfg.

It is there so `cargo test` can compile the crate at all — see §4's third trap.
Without it there is no test layer outside Nix, and the panic guard would be a
claim no gate could check. The header comment on the file spells out why it must
not grow into generating a stub scaffold: that would let tests pass against a
contract nothing derived.

If the builder ever ships a way to run a Rust module's unit tests itself, this
file should go.

---

## 6. End to end in Basecamp — observed, not inferred

Everything in this section was watched happening in a running Basecamp, with a
person clicking the buttons. Where something is inference it says so.

Reproduce with `lgs basecamp modules`, `lgs basecamp install`,
`lgs basecamp launch alice`, then click the four buttons in the Dialectica
plugin. The evidence is in `.scaffold/basecamp/profiles/alice/basecamp.log`.

### Both modules load, and the view renders

`Module loaded: dialectica`, then the core publishes itself for remote access —
`RemoteTransportHost: Published object: "dialectica"` on
`local:logos_dialectica_<id>` — with **no `sun_path` segfault**, confirming the
`runtime_dir` setting in `scaffold.toml` does its job (PLAN.md §11).

`Main.qml` then parses and paints: the log walks every import
(`QtQuick`, `QtQuick.Controls`, `QtQuick.Layouts`) and resolves every type it
uses, ending at `Successfully loaded UI module: "dialectica_ui"` and
`MainContainer: Added plugin dock to WorkspaceArea: "Dialectica"`. This is the
half that four earlier failed clicks could NOT establish, because dependency
resolution happens before any QML is instantiated — a launcher tile that
responds proves the icon resolved and nothing more.

**Process-per-module is real, and visible.** `pgrep -af logos` shows
`dialectica` and `delivery_module` each in their own `logos_host` process with
its own persistence path. PLAN.md §2.4's "every `modules().x` call is IPC" is
not a figure of speech.

### The panic guard holds, in the real host

The whole point of the two presses. Pressed at 12:23:16 and again at 12:23:41 —
25 seconds apart, **same module process, same instance id, no abort between
them.** Each press logs the panic:

```
thread '<unnamed>' panicked at src/core.rs:97:9:
panic_probe was asked to panic with: {}
```

and each returns the wire-contract error shape to the view:

```
{"error":"panic in panic_probe: panic_probe was asked to panic with: {}"}
```

Compare §3's unguarded behaviour — `failed to initiate panic, error 5`,
`signal 6`, then `MODULE_NOT_LOADED` for every later call. Same dispatch path,
opposite outcome. `catch_unwind` at our own boundary is the entire difference,
and it is now proven in the host that will actually run it rather than in a
test binary.

### The bridge carries live traffic

The §3.2 question, closed. `delivery channelExists` drives the full IPC chain:
`dialectica` requests `delivery_module`, `capability_module` mints and delivers
a token, and delivery's own C++ implementation executes the call —

```
DeliveryModuleImpl::channelExists called with channelId: dialectica-phase0
```

That is our LIDL-derived typed Rust client reaching a real provider in another
process. §1 proved the bridge compiles; this proves it carries traffic.

Its reply is an error, and the *right* error:

```
{"error":"Context not initialized","success":false,"value":null}
```

Nothing has called `createNode` (§11: exactly once per context), so delivery
declining is correct. **The error is delivery-side, not bridge-side** — which
is precisely the distinction this button was built to make.

### One real bug, which only a live call could find

Our core wrapped delivery's error as:

```
{"error":"delivery_module.channelExists returned an unrecognised value:
{\"error\":\"Context not initialized\",\"success\":false,\"value\":null}"}
```

`channel_exists_reply` matches `true`/`false` and refuses to guess at anything
else — that refusal is deliberate and tested (coercing an unknown reply to
`false` would report "channel not open" for an answer delivery never gave).
The gap is narrower: it does not recognise delivery's **error envelope** as an
error, so a real delivery-side failure reaches the user wrapped in a parser
complaint, and the actual message — `Context not initialized` — survives only
as quoted text inside it.

Fix: check for an `error` key before attempting the boolean, and propagate that
message unchanged. Per PLAN.md §2.5 the caller should see delivery's error, not
our confusion about it.

**The general trap, worth more than the instance:** a typed client's happy path
and its error path are separate contracts. Every generated cross-module call
needs a test for what arrives when the callee *declines*, and no amount of
unit-testing the success shape produces it — the tests here covered `true`,
`false` and junk, and still missed the one reply a live provider actually sent.

---

## 7. What Phase 0 still did not answer

Stated so nothing above reads as more than it is:

- **No delivery *node* was ever created**, so no channel has been opened, no
  message sent, and the §4.3 channel-id-reuse trap remains entirely untested.
  `channelExists` reaching delivery and being declined is the bridge working,
  not the transport working.
- **Only the `alice` profile was launched.** Both profiles are installed, but
  nothing has exercised two peers, which is what §4 is ultimately about.
- **The `bob` profile, and two-instance p2p, are untouched.**
- **CI does not exist yet.** PLAN.md §10 describes the intended shape; nothing
  here has been run by anything but a person clicking buttons.
- **The panic guard is proven for a panic we caused on purpose.** A panic
  inside a *dependency* — inside the generated client, or in `serde` parsing a
  hostile payload — takes the same path by construction, but that is inference
  from the dispatch shape, not an observation.

---

## 8. Two pins `lgs basecamp doctor` warns about, and why they stand

`doctor` reports these on every run. Both are deliberate; neither is resolved.
Recorded here so the next person does not "fix" them back into a broken build.

- **The basecamp/lgpm pin pair is split**, and doctor is right that this is a
  real hazard: basecamp embeds the same package-manager library `lgpm` is built
  from, so a mismatched pair can install packages the app cannot then read.

  It stands anyway because the matched pair does not work here. lgpm at the
  scaffold default rejects our UI package outright — `Forbidden root entry:
  assets` — since the builder emits manifest 0.6.0, which relocates a `ui_qml`
  icon into a top-level `assets/`, and that lgpm predates `assets` in
  `logos-package`'s allowlist. So the choice is a split pair that installs and
  runs, or a matched pair that cannot install the UI at all.

  **What makes it tolerable is that the split is on the read side and we have
  exercised it**: this same pair installed all three modules and ran them (§6).
  Re-pair the pins the moment a basecamp release ships with an lgpm that
  accepts `assets`, and drop this note with it.

- **The `delivery_module` pin differs from the scaffold default.** Ours is the
  rev whose channel API §1 and §6 were proven against, end to end. The default
  is not known to be wrong — it is untested here, which is a different thing.
  Doctor's warning ("may not work against a basecamp release built with the
  scaffold default") is the honest framing, and moving to it means re-running
  §6's four buttons rather than assuming.

The general point: **`lgs basecamp doctor` is the check to run before believing
a green build**, and a WARN here is a decision to make, not noise to skip past.

---

## 9. How to tell whether a launched instance is still running

There is no `lgs` verb for it — `launch` starts one and nothing reports on it.
What exists is the PID, which `launch` writes to the profile's `launch.state`;
`lgs basecamp paths <profile>` prints that file's location along with the log
file and every XDG dir.

Read `launch.state` rather than scanning the process table. The process name is
not what you would guess — the launcher runs as `.LogosBasecamp.elf` under the
dynamic loader, and each module is a separate `.logos_host.elf` — so a
`pgrep basecamp` finds nothing and reads as "it exited" when it is running
perfectly well. That mistake was made here.

Two things fall out of that layout, both worth knowing:

- **Every module really is its own OS process**, each with its own persistence
  path (PLAN.md §2.4). The IPC in "every `modules().x` call is IPC" is literal.
- **The module process surviving a panic is checkable from outside** — same
  PID, same instance id, before and after. That is the §6 guard result stated
  in a form that does not depend on trusting the module's own report.
