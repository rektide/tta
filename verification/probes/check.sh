#!/usr/bin/env bash
# Package-only graph: workspace all-features builds can conceal a native runtime.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
tree=$(cargo tree -p tta-probe-witness --no-default-features -e normal,features --prefix none)
if grep -E '^(tta-runner|nix|bytes) v|^tmax feature "(client|control_mode)"' <<< "$tree" >&2; then
    echo 'runner-free probe witness unexpectedly links a runner/native runtime' >&2
    exit 1
fi
cargo nextest run -p tta-probe-witness --no-default-features --lib >&2
exec cargo run --quiet -p tta-probe-witness --no-default-features -- "$@"
