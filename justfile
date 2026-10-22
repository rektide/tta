# Install recipes exist because the workspace root is a virtual manifest:
# `cargo install --path .` fails; the binary package lives at app/cli.

# List available recipes
default:
    @just --list

# Install the tta CLI with the default native backend (extra args pass through, e.g. just install --offline)
install *args:
    cargo install --path app/cli --locked --force {{args}}

# Install with the optional scripted lookup backend compiled in as well (native + cli)
install-cli:
    cargo install --path app/cli --locked --force --features cli

# Install CLI-only runtime lookup, e.g. for other Unix platforms
install-cli-only:
    cargo install --path app/cli --locked --force --no-default-features --features cli
