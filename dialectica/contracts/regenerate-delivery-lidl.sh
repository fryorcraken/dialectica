#!/usr/bin/env bash
# Regenerate contracts/delivery_module.lidl from delivery_module's impl header.
#
# WHY THIS FILE EXISTS — the Phase 0 finding, in executable form.
#
# delivery_module v0.2.1 publishes no `lidl` flake output, so PLAN.md §3.2
# planned to point a `dependency_overrides` entry straight at its impl header:
#
#     "delivery_module": { "file": "src/delivery_module_plugin.h",
#                          "input": "delivery_module",
#                          "impl_class": "DeliveryModuleImpl" }
#
# That does NOT work from a Rust module. The builder hands the override's path
# to the RUST generator as `--dep delivery_module=<path>`, and lidl-gen parses
# it as LIDL:
#
#     Failed to parse dep .../src/delivery_module_plugin.h:
#     parse error at line 1 column 1: Unexpected character '#'
#
# The '#' is `#pragma once`. `impl_class` is accepted by the metadata parser
# and then never reaches the Rust path — only the C++ backend consumes it, and
# only the C++ generator has a `--header-to-lidl` frontend.
#
# So the header→LIDL conversion the C++ path does inside the build is done HERE
# instead, once, and its output is committed. The override then points at a
# real .lidl, which the Rust generator accepts.
#
# The cost is a checked-in file that can go stale against upstream. That is why
# this script exists rather than a note: run it, `git diff`, and the answer to
# "is our copy current?" is a command rather than a memory. Re-run it whenever
# the delivery pin moves.
#
# See docs/PHASE0-FINDINGS.md for the full evidence.

set -euo pipefail

DELIVERY_TAG="${1:-v0.2.1}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

echo "==> fetching delivery_module ${DELIVERY_TAG}"
git clone --quiet --depth 1 --branch "${DELIVERY_TAG}" \
    https://github.com/logos-co/logos-delivery-module "${WORK}/delivery"

echo "==> building the C++ generator (it owns --header-to-lidl; the Rust one does not)"
nix build github:logos-co/logos-cpp-sdk#cpp-generator -o "${WORK}/generator" --no-write-lock-file

echo "==> converting the impl header to LIDL"
"${WORK}/generator/bin/logos-cpp-generator" \
    --header-to-lidl "${WORK}/delivery/src/delivery_module_plugin.h" \
    --metadata "${WORK}/delivery/metadata.json" \
    --impl-class DeliveryModuleImpl \
    -o "${HERE}/delivery_module.lidl"

echo
echo "==> done. Now check what moved:"
echo "      git diff ${HERE}/delivery_module.lidl"
echo
echo "    A non-empty diff means delivery's API changed under us. Read it"
echo "    before accepting it — a removed method is a call site that will stop"
echo "    compiling, and a changed signature is one that might not."
