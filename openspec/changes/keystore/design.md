# Design

## Context

See proposal.md — Why. `identity.rs` supplies `SecretKey`, `derive_stoa_key`
and the address construction, and says in its own module doc that a keystore is
missing and that the memory hygiene of secret copies is "deferred, not solved"
and belongs here. This change is that file, plus the probe a view gates on.

PLAN.md §5.6 names radicle as the model, so the model was read rather than
assumed. Two of its properties turn out to be **gaps** rather than choices, and
this design closes both — see the Decisions on permissions and on atomic writes.

## Goals / Non-Goals

**Goals**

- A root secret that survives a restart and is not readable from the file alone.
- A probe truthful enough that a view can gate a compose box on it.
- Every failure on this path a returned error, never a prompt and never a panic.

**Non-Goals**

- **No agent.** §5.6's third unlock path. See the Decision below.
- **No rotation, no key change in place.** §5.3 defers rotation; changing a
  passphrase is `write_to` and nothing else.
- **No path discovery.** The crate is handed the file. Where it lives is the
  module crate's decision, from the host's `instance_persistence_path`.
- **No signing through the keystore.** It hands back a `SecretKey`; `op.rs` owns
  what is signed.
- **No multi-identity.** One root secret, one file. §5.2's per-Stoa identities
  are derived, not stored.

## Decisions

### XChaCha20-Poly1305, not an OpenSSH-format key

The literal reading of §5.6 — "copy radicle's proven model" — is to use
`ssh-key` and write an OpenSSH-format private key, which is what heartwood does:
`aes256-ctr` under a `bcrypt-pbkdf` KDF at 16 rounds, all of it the `ssh-key`
crate's defaults rather than anything heartwood chose.

Rejected, for three reasons in ascending order of weight:

- **It imports a key-file format to store 32 bytes.** PEM armouring, a public
  half written as a second file, a legacy cipher suite, and a parser for all of
  it. `dialectica-core` decodes hostile input for a living and the cost of a
  format is its parser.
- **The interoperability it buys is the deferred half.** An OpenSSH file is
  worth having because `ssh-agent` and `ssh-keygen` can act on it — and the
  agent path is out of scope here. Paying the format cost for a benefit this
  change does not take is the wrong order.
- **`aes256-ctr` is unauthenticated.** heartwood's file has no MAC over the
  ciphertext, so a wrong passphrase decrypts to *something*; the check is a
  magic-value comparison inside the decrypted blob. That is the property this
  design is least willing to give up — see the next Decision.

**bcrypt_pbkdf** was rejected with PBKDF2 for the same reason, under
"Argon2id" below.

So: an AEAD over a small binary format of our own, with the same version
discriminant and strict-decoding discipline `stoa.rs` and `op.rs` already carry.

**XChaCha20-Poly1305 over ChaCha20-Poly1305 and AES-256-GCM:**

- Over **AES-GCM**: dialectica ships pure Rust everywhere, and AES without
  hardware support is a table lookup whose timing depends on the key. ChaCha is
  constant-time by construction.
- Over the **96-bit-nonce ChaCha20-Poly1305**: at 96 bits a random nonce needs a
  birthday-bound argument, which means the format would have to carry a counter
  and every future writer would have to respect it. At XChaCha's 192 bits a
  fresh random nonce per write is safe outright, so nonce reuse stops being
  something a later contributor can get wrong. A keystore is rewritten rarely,
  but "rarely" is not a property a format should rest on.

### Authentication is the load-bearing half, not the confidentiality

The reason an AEAD rather than a cipher plus a passphrase check: **without
authentication, anyone with write access to the keystore can substitute the
identity the user posts under.** They cannot read the old secret, but they can
replace it with one they know, and the user then posts — signed, verifiably,
under a key the attacker holds. Nothing downstream can detect it, because every
op is genuinely well-formed.

Confidentiality protects against a stolen disk. Authentication protects against
a hostile local process, which is the more likely of the two on a desktop
sharing a machine with other Logos modules. Both matter; only one of them is
missing from the model §5.6 points at.

The **header is the associated data** — version, protection scheme, salt, nonce
— so a downgraded protection byte or a swapped salt fails the tag. The KDF
parameters are deliberately *not* in the AAD: they are inputs to the key, so
editing them produces a different key and the tag fails anyway.

### Argon2id, with the parameters recorded in the file

Over PBKDF2 and bcrypt_pbkdf. The threat is an offline attack on a stolen file,
where the attacker's budget is hardware: both alternatives parallelise cheaply
on a GPU, and Argon2id is memory-hard, which is what makes that hardware cost
money. It is also the only one of the three with a standards recommendation
behind it (RFC 9106) rather than incumbency.

RFC 9106's **second** recommended option (64 MiB, t=3, p=4), not its first
(2 GiB). A forum module shares a machine with a desktop session and with other
Logos modules, and a 2 GiB allocation on unlock is an availability problem of
its own.

**The parameters are written into the file**, not assumed. Assuming them means
that changing the cost turns every existing keystore into a "wrong passphrase" —
the single most misleading thing this code could tell a user, because the fix it
implies (retype the passphrase) cannot work.

### A ceiling on the recorded cost, which a test found

Recording the parameters creates a hole: they are values anyone with write
access to the file chooses, and `argon2::Params::new` accepts an `m_cost` up to
`u32::MAX` — a request to allocate four terabytes. The blanket hostile-input
sweep flipped one byte of a recorded cost and the **OOM killer took the test
binary with SIGKILL**.

In a module process that is the same death PHASE0-FINDINGS §3 measured, and it
is worse than a panic: reached with no unwinding anywhere, so `guarded` has
nothing to catch. The ceiling is checked *before* `Params::new`, because the
allocation happens in `hash_password_into` and a check after the fact runs too
late for nothing.

**This is the one finding here that was not predicted.** The general lesson:
any field this crate reads from untrusted bytes and then *sizes an allocation
on* needs a ceiling, and the op decoder already learned the same thing about its
length prefixes.

### Bound the work, not each knob — the first fix was the wrong shape

The first version of that ceiling capped `m_cost`, `t_cost` and `p_cost
*individually*, at values chosen for portability headroom: 16x this build's
memory, 10x its iterations, 4x its lanes.

**They multiply.** The worst set those caps accepted — m=1 GiB, t=32, p=16 —
measured at **302 seconds** of CPU-bound, uninterruptible work plus a 1 GiB
allocation, from editing twelve bytes of a file. PHASE0-FINDINGS §2 puts the
caller's timeout at 20 seconds, so that overruns by 15x: the view is told
`timeout`, the module is wedged with no cancellation, and nothing panics. It is
the same class of damage as the OOM finding and reached the same way — without
anything for the guard to catch.

It was also, exactly, the availability problem that ruled out RFC 9106's 2 GiB
option two decisions above — permitted anyway, 16x over, through the file.

So `MAX_WORK_FACTOR` bounds the *product* `m * t` against what this build itself
writes, and the per-knob caps remain only to stop a single absurd value before
the multiplication. The factor is 2, chosen by measurement rather than taste:

| work factor | worst accepted case (debug build) |
|---|---|
| 16 | 27.7s — past the caller's own 20s timeout |
| 4 | 9.7s — inside it, with little margin |
| 2 | ~5s — shipped |

**The methodological lesson is the part worth keeping**: the boundary was tested
and the *product of boundaries* was not. `a_cost_at_the_ceiling_is_still_attempted`
varied one knob and said so honestly in a comment — which is exactly why the
multiplied corner was never executed. A test that moves one parameter to its
limit cannot see a corner that needs three.

**The test asserts the refusal is instant, not that the acceptance is
tolerable.** An intermediate version derived at the hardest accepted parameters
and asserted a wall-clock budget: ~5 seconds on every run, to assert only that
the machine was fast enough that day. The bound's actual job is to reject before
`hash_password_into` allocates or iterates, which is checkable in microseconds
and is a property of the code rather than of the hardware. The measurements
above live in `MAX_WORK_FACTOR`'s doc comment instead of in a test that re-runs
them.

### A predictable staging path was a root-secret disclosure

**This was found by review, with a working proof of concept, after the first
version shipped to the branch.** It is recorded at length because the reasoning
that produced it was *locally correct* and still wrong, which is the kind of
mistake worth being able to recognise again.

The write staged through `identity.tmp` — a predictable sibling — opened with
`.write(true).create(true).truncate(true)` and `.mode(0o600)`. The mode-at-open
reasoning was sound: it really does close the chmod race, and that argument is
still in the code. What it missed is that **`OpenOptions::mode` applies only
when the file is actually created.** An existing path is opened with its own
mode intact, and an existing *symlink* is followed to its target.

So an attacker needing only write access to the **containing directory** — a
strictly weaker capability than writing to the keystore, and precisely the
"hostile local process on the same machine" the permission check exists for —
pre-places a symlink and waits. Measured against the vulnerable code: `create`
returned `Ok`, 35 bytes at mode 0666, the 32-byte root seed verbatim at the
attacker's path. The rename then completed, so the user saw a normal 0600
keystore and nothing indicated anything had happened. Against an encrypted
keystore it leaks ciphertext, salt and nonce instead — everything an offline
attack needs.

Two fixes, **either of which closes it alone**, kept together because they fail
in different directions:

- **`create_new(true)`** — the open fails `EEXIST` on anything already at that
  name. Verified by reverting the randomness and re-running the regression test.
- **A random staging name** — there is no name to pre-place anything at.
  Verified by reverting `create_new` and re-running.

**`O_NOFOLLOW` was written and removed.** `custom_flags` takes a raw `i32` whose
value differs by platform, so it means either a direct `libc` dependency or a
hand-maintained per-target constant. It would have been a third layer over two
that each suffice, and a dependency on a security-critical path is itself attack
surface. The residual gap it would have covered, stated rather than left
implicit: on a filesystem where `create_new`'s existence check and the open are
not one atomic operation, an attacker who has *also* guessed 64 bits of
randomness could win the race. That is not a threat model this design owes an
answer to.

**What let it through**: `a_written_keystore_is_owner_only_from_the_moment_it_exists`
passes against the vulnerable code, because it checks the *destination* after a
successful rename. The staging file was never examined by any test. The spec has
gained a requirement about intermediate paths so that this is a stated
obligation rather than an implementation detail nobody was looking at.

### Two unlock paths now; the agent deferred, and the shape grows one

§5.6 names three. This ships two:

- **Unencrypted** — a real configuration on a machine whose disk is already
  encrypted. Recorded in the file rather than inferred, so that "no passphrase
  was ever set" and "your passphrase is wrong" stay distinguishable. The empty
  passphrase is refused outright, because "encrypted under a key anyone can
  derive" reads as protection and is none.
- **Passphrase by environment variable** (`DIALECTICA_PASSPHRASE`, named after
  `RAD_PASSPHRASE`).

**The agent is deferred**, and the reason is proportion rather than difficulty:
an agent is a long-lived process holding decrypted material and answering a
socket — a second security boundary to design and a daemon to supervise — and
it buys nothing until a human is repeatedly typing a passphrase, which nobody
is yet. It is also the only one of the three that cannot be done at all from
inside a sandboxed module without a transport this project has not chosen.

**Adding it later is not a breaking change**, and that is by construction. The
extension point is `Unlock`, an enum on the *input* side: `Unlock::Agent { .. }`
is a new variant, every existing caller keeps compiling, no reply shape moves,
and the file format is untouched — an agent changes who holds the passphrase,
not how the file is encrypted. Had the API been
`open(path, Option<&Passphrase>)`, adding an agent would have meant changing
that signature at every call site.

### The passphrase environment variable, and its documented limitation

A process's environment is readable by the same user through
`/proc/<pid>/environ`, is inherited by every child, and is commonly captured
whole by crash reporters and supervisors. So this path protects against an
attacker who has the *file* and not the running machine — the stolen-disk case,
which is what the encryption is for — and not against one already running code
as this user.

That is stated in the code rather than mitigated, because it cannot be mitigated
from here; the agent path is what closes it. radicle documents the same thing
more bluntly in its man page: *"this is not secure and is equivalent to having
an unencrypted secret key."* That is slightly too strong — it still defeats the
stolen-disk case — but it is the right direction to err in.

### The file is asked before the environment

`unlock_from_env` reads the file's protection byte first and consults the
environment second. The reverse order — "a passphrase is set, so assume
encryption" — reports an unencrypted keystore as a protection mismatch, and
reports a missing passphrase against an unencrypted keystore as a locked one.
Neither is true, and both send the user to fix something that is not broken.

An **empty variable counts as unset**, matching `ssh-keygen` and radicle. The
alternative would attempt an unlock with a key anyone can derive and then report
`WrongPassphrase`, blaming the user for a value they never set.

### Refuse a too-open keystore; do not warn

**radicle does not do this, and that is a gap rather than a decision.**
heartwood never stats the key file; the `0600` on creation comes from `ssh-key`,
whose reader carries an unimplemented `// TODO(tarcieri): verify file
permissions match UNIX_FILE_PERMISSIONS`. So radicle loads a world-readable key
without complaint, unlike OpenSSH itself.

Dialectica refuses, and refuses rather than warning, for a reason specific to
this architecture: **a warning here is a message nobody sees.** The module has
no terminal, and its only caller is a view that would have to choose to render
it — which is the same reasoning that makes the probe a gate rather than a hint.

The error names the *whole* fix, which is two things and not one: restrict the
mode, **and replace the key**, because a secret that has been readable by every
local process is a secret to replace rather than to keep using.

The check runs **before any content is used**. Checking afterwards would have
already loaded a file the function is about to declare unsafe to use. It is on
the permission bits only, not on ownership: a file owned by someone else is
unreadable anyway, and the OS reports that.

### One open, not two resolutions of one name

The check and the read were originally `fs::metadata(path)` followed by
`fs::read(path)`, which resolves the name **twice**. Two problems, and the
second is what made the split wrong rather than merely untidy:

- **TOCTOU.** Between the `stat` and the `open`, the thing at that path can be
  replaced — by the same attacker the check exists to stop.
- **`fs::metadata` follows symlinks.** The mode checked was the *target's*, so
  a symlink pointing at something world-readable passed a check about a file
  nobody read.

`read_checked` opens once and calls `File::metadata()` on the **handle**. There
is no version of this where the check and the read can disagree, because there
is only one file. Plain std; nothing new was needed.

`open_from_env` inherited the same shape at a larger scale: `is_encrypted` did a
full check-read-parse and `Keystore::open` then repeated all of it. That was
*safe*, but only because `from_file_bytes` re-derives the protection from the
bytes it decodes — safety by coincidence, over a file an attacker may be editing
between the two reads. It now reads once and decides both from the same bytes.

The single open also bounds what is read. `MAX_KEYSTORE_LEN` is checked against
the handle's size, and the read itself goes through `take`, because
`metadata().len()` is 0 for a FIFO — so the size check alone does not bound a
`read_to_end` on an attacker-supplied path.

### Check the containing directory too, on write bits only

The directory is what makes the symlink attack possible, and a 0600 keystore
inside a 0777 directory is not protected by its own mode: anyone can delete it
and put their own there. The permission check is the one place whose job is
refusing an unsafe filesystem state, so it should be refusing this one.

**Write bits only**, unlike the file's check, and the asymmetry is deliberate: a
readable directory discloses that a keystore exists, which is not a secret — the
path is a documented convention. A writable one is a different claim entirely.

Reported as its own error, because the fix is a chmod on a different path.
Saying "the keystore's permissions are too open" about a correctly-permissioned
keystore sends the reader to the wrong file.

### Atomic writes, which radicle also does not do

`ssh-key`'s writer is `create + truncate + write_all` — no temp file, no
rename, no fsync. An interrupted write truncates the key in place, and the root
secret exists nowhere else, so that is every identity the user has. heartwood's
mitigation is a refuse-to-clobber guard, which is a different property and does
nothing for a crash mid-write.

Here: write to a **sibling** temporary at `0600`, `sync_all`, then `rename` over
the destination. A sibling rather than `/tmp` because a rename across
filesystems is a copy and loses the atomicity. The mode is set at open time
rather than chmod'ed afterwards, because between a create at 0644 and a chmod
there is a window in which the secret is world-readable, and a window is all a
local attacker polling the directory needs.

`create` additionally refuses to overwrite, keeping heartwood's guard as well —
the two answer different questions and both are worth having.

### The probe is an enum, so the illegal states cannot be written

§5.6's shape is `{"canPost":bool, "identity":"…" | "reason":"…"}` and the bar is
exclusive. A `{ can_post, identity: Option, reason: Option }` can express three
states the contract does not have — both, neither, and `canPost:true` with no
identity — and each would then have to be prevented at every construction site.

`Capability` is an enum with one payload per variant, so serialisation is total
and the handler has no branch to get wrong. This is CLAUDE.md's standing
instruction applied: a data shape is right everywhere at once, where a guard has
to be got right at each call site.

### A keystore failure is an answer, not an error reply

The probe returns `canPost:false` with a reason for **every** keystore state,
including ones that are plainly errors — unreadable file, malformed file, wrong
passphrase. §2.5's `{"error":"..."}` is reserved for the one failure that is not
about capability: a request this function could not parse.

The reason is what a caller would otherwise have to do. A view handling both
"you cannot post, because X" and "I could not determine whether you can post"
has two negative branches, and the second has no sensible rendering. A malformed
*request* is a caller bug; a malformed *keystore* is a user state.

### The reason is the keystore error's own message

Not a rewording of it. `KeystoreError::Display` already names the fix for every
variant — a documented obligation on it, with a test — so paraphrasing at the
probe would mean maintaining the same guidance in two places and watching the
two drift. The consequence is that "the reason names the fix" is enforced where
the strings live, on every variant at once, including ones added later.

### The probe takes a Stoa, because there is no "the" identity

§5.2 gives a user one identity *per Stoa*. "Who would post" has no answer until
a Stoa is named, so the probe takes one. A probe that ignored it would report
one Stoa's pseudonym while the user posted under another's.

### Nothing here discovers a path or reads the environment at init

`Keystore` is given the file to work on. A fixed path baked into a pure crate is
untestable and would mean this crate reading the environment at a moment its
caller does not control; the module crate has the host-stamped
`instance_persistence_path` from `on_context_ready` and passes it in.
`default_path_in(dir)` is the naming convention applied to a directory the
caller supplies — the *name* is fixed, the *directory* is not.

### Secret material is zeroized, closing a gap identity.rs names

`identity.rs` states plainly that its `to_bytes` hands out a copy it no longer
controls and that "those copies are where the keystore (§5.6) takes over, and
zeroizing them is its job". So: the root is `Zeroizing<[u8; 32]>`, the derived
cipher key and the decrypted plaintext likewise, the `Passphrase` wraps
`Zeroizing<Vec<u8>>`, and `chacha20poly1305`'s `zeroize` feature wipes the
cipher's own expanded key state.

**The wipe is structural, because a wipe is not testable.** `generate` used to
copy `sk.to_bytes()` into a local, wrap it, then `bytes.zeroize()` the local —
and review found that **deleting that line left all 184 tests green**. It could
never have been pinned: a stack local after its function returns is not
observable. So the copy is not made at all; `to_bytes()` moves straight into the
`Zeroizing`, and there is no second binding to forget. `Zeroize` is no longer
imported by the library, so a `use zeroize::Zeroize` reappearing is itself the
signal that someone has reintroduced a hand-rolled wipe.

What *is* tested: that `Zeroizing` clears on drop (observed through a raw
pointer post-drop, with a control proving the technique can see the difference),
and that every secret-bearing field is wrapped in it (a compile-time bound, so
removing a wrapper stops the build rather than failing at runtime).

**The absence of `Debug` on `Keystore`, `Passphrase` and `Unlock` is enforced by
a test**, not by a doc comment. It was previously enforced by absence, which is
not enforcement: the natural way to acquire one is a moment's convenience while
debugging, or an `unwrap_err()` that will not compile without it. A `Debug` here
puts the root secret one `{:?}` away from a log line, a panic payload, or —
through `guarded`'s message — the wire. The test uses autoref specialisation and
carries a control case, so it cannot pass vacuously.

**What that does not cover, said plainly so the omission is not mistaken for
coverage:** Rust can move a value before it is dropped, and a `Vec` that
reallocates leaves its old buffer unwiped. Argon2's internal memory blocks are
the crate's to manage, not ours. And nothing here prevents the pages being
swapped to disk — `mlock` is not attempted, because it needs a privilege the
module may not have and failing it silently would be worse than not claiming it.
Zeroization here raises the cost of a memory disclosure; it does not eliminate
one.

### No timing side channel is introduced

The passphrase check is the AEAD's own tag verification, which is constant-time,
and there is **no separate stored verifier** to compare against. That is the
shape an implementation reaches for by reflex — store a hash of the passphrase,
compare on unlock — and an ordinary `==` on one short-circuits, leaking how much
of a guess was right and turning an offline attack into a faster one.

The other comparisons in the file are on public values: the magic byte, the
version, the protection discriminant, the permission mode. None is secret, so
none needs constant time.

### Errors carry no key material

Every variant of `KeystoreError` carries either nothing, a public number (a
permission mode, a version discriminant), or an OS message that names a path and
an errno. None carries ciphertext, a passphrase, or a secret — checked by a test
that feeds distinctive markers through and greps every message and `Debug` form,
because the sort of thing that breaks this is a later
`format!("... {ciphertext:?}")` added while debugging.

`WrongPassphrase` deliberately does not distinguish "wrong passphrase" from
"someone edited the ciphertext". It cannot — both are one tag failure — and
guessing between them would be telling the user something unverified.

## Risks / Trade-offs

- **The environment-variable path is visible to other local processes.** →
  Documented rather than mitigated, because it cannot be mitigated from here.
  The agent path closes it and is deferred; the shape grows one without a
  breaking change.
- **An unencrypted keystore is supported at all.** → It is a real configuration
  on an encrypted disk, and refusing it pushes users to worse workarounds. The
  mitigation is that it is *recorded*, so no caller can mistake it for
  protection and the probe can say which state a user is in.
- **A too-open keystore is refused, which can lock a user out of their own
  identity.** → Deliberate, and the error says to chmod it. The alternative —
  loading it — is what radicle does, and it means posting under a key every
  process on the machine has had a chance to copy.
- **Refusing to overwrite means "I forgot my passphrase" has no recovery.** →
  Correct, and inherent: §5.3 has no rotation, so a lost root secret is a lost
  identity. The error says to remove the file deliberately, and says that the
  existing key cannot be recovered afterwards.
- **A 64 MiB allocation on every unlock.** → The tradeoff Argon2id is for. It
  is once per process, not per operation.
- **The cost ceiling could reject a genuinely-harder future file.** → Only past
  twice this build's `m * t`, which still admits every parameter set RFC 9106
  recommends at or below 128 MiB-equivalent work. Raising `MAX_WORK_FACTOR` is a
  one-line change, and the refusal test hardcodes the ceilings so that raising
  one has to be deliberate in two places. The measurements that chose the value
  are in its doc comment, so the next person changing it is changing it against
  numbers rather than against a guess.
- **`O_NOFOLLOW` is absent.** → Two independently-sufficient defences are
  present instead, and the residual race is documented above. Adding a `libc`
  dependency for a third layer would have widened the attack surface of a
  security-critical path to narrow it.
- **Windows has no permission check.** → Stated in the code rather than silently
  skipped. Windows ACLs are a different mechanism needing their own
  implementation; dialectica targets Linux (PLAN.md's Basecamp traps are
  Linux-specific), and the arm exists to keep the crate portable rather than to
  serve a supported platform.

### Two requirements were restated because they could not be observed

A spec requirement nothing can check is worse than no requirement: it reads as
coverage and drifts silently. Review found two of mine, both added in good faith
during the security fixes.

- **"A keystore is read at most once per operation."** Not observable from
  outside — a caller cannot count the implementation's reads. Restated as what
  the single read is *for*: **content that passed no permission check is never
  used**, and **a symlink's target does not inherit the check**. Both are about
  outcomes rather than mechanism. The read-once implementation is an obligation
  on the code, recorded above and in `read_checked`'s doc comment.
- **"Correctness is decided by the authentication tag."** Nothing would notice a
  stored verifier being *added* alongside the AEAD. Restated as a property of
  the file: **nothing is stored that can verify a passphrase independently of
  decrypting**. That is checkable three ways — the layout has no room, no byte
  past the header repeats across two writes of the same secret, and a wrong
  passphrase is indistinguishable from a tampered file. Adding a four-byte
  verifier now kills nine tests.

A third, in `posting-capability`, was **unsatisfiable through the API it
described**: "the probe is callable repeatedly", against a `lookup: impl
FnOnce`. The signature is now `Fn`.

Worth recording because the obvious explanation is wrong: `FnOnce` is a
supertrait of `Fn`, so `&F` satisfies it and the scenario *was* testable by
passing a reference — verified by reverting the signature and watching the test
still pass. `Fn` stays because it is the honest constraint, not because it is
enforceable: nothing consumes the lookup, and a signature that overstates what
it takes is one callers work around. **Reverting it is not mutation-detectable,
and the table says so.**

### The "names a fix" check was passing for the wrong reason

`every_error_message_names_a_fix` matched `msg.contains(verb)`. Review replaced
`Truncated`'s message with *"truncated: the restore operation that wrote this
file did not finish"* — a pure fault statement, no action, "restore" as a **noun**
— and the test stayed green. Substring matching has no word boundary and no
part-of-speech sense: "check" matches "checksum", "create" matches "created".

Now the verb must appear **at a word boundary** in the **guidance clause** —
everything after the first `;` or em dash — and a message with no such clause
fails with a message saying so. Every existing message already satisfied both,
so the fix cost nothing. The demonstrated bypass now dies.

The `NO SPEC:` marker stays and is narrowed to what the check actually pins.
What it still cannot see: a grammatically imperative sentence telling the user
to do something useless. That needs a reader.

## Open Questions

- **Where the module crate puts the keystore.** This change fixes the file
  *name* and leaves the directory to the caller. The adapter will use the
  host-stamped `instance_persistence_path`, which is per-instance — so two
  Basecamp profiles get two identities. That is probably right and is not
  decided here, because deciding it needs the adapter, which is out of scope.
- **Whether a passphrase change should re-encrypt without re-minting.**
  `write_to` already does it; nothing has asked for a verb with that name yet.
