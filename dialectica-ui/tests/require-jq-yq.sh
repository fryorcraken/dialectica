# Sourced, not run: defines `require_jq_yq`, which the UI suite's YAML readers
# call before their first `yq`.
#
# `yq` NAMES TWO UNRELATED TOOLS, and these scripts need a particular one.
#
#   * kislyuk/yq, a jq WRAPPER. It turns YAML into JSON and hands that to jq,
#     so its filters are jq filters and its default output is JSON. Ubuntu's
#     `yq` package is this one, and it is also what ships `tomlq`, which the
#     scaffold.toml guard in ui-tests.yml already needs.
#   * mikefarah/yq, written in Go. GitHub's runner image preinstalls it at
#     /usr/bin/yq (actions/runner-images, install-yq.sh). Its expression
#     language only resembles jq's, and its default output is YAML.
#
# The apt package installs to the same /usr/bin/yq, and dpkg replaces a file
# no package owns, so after the install step the jq wrapper is the one on PATH.
# That is inferred from the two install paths, not observed, and a runner image
# that moved the Go binary to /usr/local/bin would reverse it: /usr/local/bin
# comes first on PATH. A jq filter handed to the Go yq can fail, or it can
# return something that looks right, so this check runs first and names the
# problem.
#
# It PROBES the one property the callers rely on, YAML in and JSON out, rather
# than parsing a version string that neither tool promises to keep stable.
#
# design.md D12 says why these scripts read YAML with `yq` at all, and what
# breaks without this check, measured.
require_jq_yq() {
    probe=$(printf 'a: [1, 2]\n' | yq -c . 2>/dev/null) || probe=""
    if [ "$probe" = '{"a":[1,2]}' ]; then
        return 0
    fi
    echo "::error::the yq on PATH ($(command -v yq || echo 'none')) is not the jq-wrapper yq (kislyuk/yq, Ubuntu's 'yq' package)"
    echo "::error::these scripts pass it jq filters, and a different yq reads them differently. Install the 'yq' package"
    return 1
}
