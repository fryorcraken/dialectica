# Where the source material is

Migrated from `docs/PLAN.md` §12. This project's design was written from
several investigations of local checkouts on the development machine. When a
claim about the surrounding ecosystem needs re-checking, these are where it
came from — all absolute paths on the development machine, none of them
vendored into this repo.

| What | Where |
|---|---|
| **The structural template** — monorepo, core+UI, Rust FFI, CI, `CLAUDE.md` conventions | `/home/fryorcraken/src/rad/radicle-logos-module` |
| Docs source for docs.logos.co (module building, delivery API, LEZ, LGX format) | `/home/fryorcraken/src/logos-co/logos-docs/docs/` |
| Delivery module — **read at tag `v0.2.1`**, not the working tree | `/home/fryorcraken/src/logos-co/logos-delivery-module` |
| Waku/Nim internals — sharding, retention, message size, RLN config | `/home/fryorcraken/src/logos-messaging/logos-delivery` |
| SDS spec (LIP-109) and LIP-23 content topics | `/home/fryorcraken/src/logos-co/logos-lips/docs/` |
| LEZ private accounts — nullifiers, ML-KEM, commitment set | `/home/fryorcraken/src/logos-blockchain/logos-execution-zone` |
| λAccount / VLAD identity roadmap and FURPS | `/home/fryorcraken/src/logos-co/roadmap/content/anoncomms/` |
| A real delivery consumer (older API, still instructive) | `/home/fryorcraken/src/logos-co/logos-delivery-demo` |
| `logos-scaffold` source, docs and bundled skills | `/home/fryorcraken/src/logos-co/logos-scaffold` |
| OpChan — the nearest kin forum, read for the relevance-design appendix that lived in PLAN.md. **No local checkout**; clone from GitHub | `logos-messaging/OpChan` |
| λ-Prize LP-0005 / LP-0016 / LP-0017 — **submission write-ups only, not code.** The solution repos are not cloned here, so every claim about them is the builders' self-assessment | `/home/fryorcraken/src/logos-co/lambda-prize/` |
| The JS SDK reliable-channels tutorial — informal prose, and the clearest statement of what a channel id is | `/home/fryorcraken/src/logos-messaging/docs.waku.org/` |
| Spasm — a signer-agnostic social protocol, read for the never-verified-credential precedent behind the `identity`/claims design. **The code is the spec**: versioning lives in format strings (`spasmid01`, `SpasmEventV2`) and the normative reference for its hashing rule is its README, so read `src.ts/`, not the docs site | `/home/fryorcraken/src/spasm-network/spasm.js` |

Two of these are **stale working trees** and will mislead if read directly:
`logos-delivery-module` sits on a pre-channels branch, and
`logos-messaging/logos-delivery` predates reliable channels entirely. Use
`git show v0.2.1:<path>` for the delivery API.

**The one exception, and reach for it first: the delivery module's interface is
vendored here.** `dialectica/contracts/delivery_module.lidl` is the converted
v0.2.1 contract the build actually compiles against — every method, every
event, every docstring. For "does delivery expose X?" that file is both the
nearest and the most authoritative answer. (`docs/PHASE0-FINDINGS.md` §1
records a correction that followed from answering that question from a report
instead of from this file.)

`logos-module-builder` and `logos-rust-sdk` are not local checkouts — they are
flake inputs. The Rust module examples and doctests live inside the
`logos-rust-sdk` source, under `tests/` and `doctests/`.

**Clone the SDK from GitHub rather than reading it out of `/nix/store`.** The
store copy is a bare source snapshot with no git history, and it is whatever
revision the builder pinned — which has lagged HEAD substantially. Reading the
store tells you what you will *link against*; reading GitHub tells you what
the documentation describes. Both are worth knowing, and they are not the same
thing (see `openspec/changes/archive/2026-09-11-op-log/design.md`'s "What the
SDK does not provide" for why the gap matters).
