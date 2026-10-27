#!/usr/bin/env bash
# Package-only builds prevent workspace feature unification hiding a runtime.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
cargo check -p tta-launcher --no-default-features >&2
tree=$(cargo tree -p tta-core-witness --no-default-features -e normal --prefix none)
if grep -E '^(tta-runner|nix|rustix|bytes) v' <<< "$tree" >&2; then
    echo 'runner-free witness unexpectedly links a runtime dependency' >&2
    exit 1
fi
cargo test -p tta-core-witness --no-default-features --lib >&2
exec cargo run --quiet -p tta-core-witness --no-default-features -- "$@"
