#!/usr/bin/env sh
# Install Ubuntu's jq-wrapper `yq` (which ships `tomlq`), and any further
# Ubuntu packages a job names, on a GitHub Actions Ubuntu runner.
#
#     dialectica-ui/tests/install-yq.sh [package...]
#
# CI ONLY. It moves apt sources aside with sudo, which is the runner's to lose
# and nobody's workstation's. Both UI workflows call it, ci.yml's `ui-specs`
# with no arguments and ui-tests.yml's `spec` with the graphics libraries
# Basecamp loads, so there is one copy of the procedure to keep right. The
# e2e-suite-review change's design.md D9.
#
# THE APT SOURCES. The runner image preconfigures third-party apt repositories
# these jobs do not use, and one of them failing takes `apt-get update` down
# with it: radicle-logos-module's identical step failed on a 403 from
# packages.microsoft.com. Every package installed here is Ubuntu's own, so
# every source but Ubuntu's is moved aside first, `update` is allowed to fail,
# and `install` is the real gate.
#
# THE yq. The runner's preinstalled `yq` is a different tool, the Go one, with
# no `tomlq` and not jq's language. This package installs over it at
# /usr/bin/yq. `require_jq_yq` then confirms the jq wrapper is the one on
# PATH, here, where a wrong one is an install problem, rather than first in
# whichever script calls `yq` next.
set -eu

here=$(cd "$(dirname "$0")" && pwd)
. "$here/require-jq-yq.sh"

sudo mkdir -p /etc/apt/disabled-sources.d
# `find -exec` rather than a glob: no match must not be an error.
sudo find /etc/apt/sources.list.d -maxdepth 1 \
    \( -name '*.list' -o -name '*.sources' \) \
    ! -name 'ubuntu.sources' ! -name 'ubuntu.list' \
    -exec mv -v -t /etc/apt/disabled-sources.d {} +
sudo apt-get update -qq ||
    echo "::warning::apt-get update reported errors; install is the gate, continuing"
sudo apt-get install -y --no-install-recommends "$@" yq

command -v yq
yq --version
tomlq --version
require_jq_yq
