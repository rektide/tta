---
type: Guide
title: tta — enter a matching tmux workspace
description: Rust session-first tmux launcher, reusable command expressions, and instrumented private-server runtime verification
resource: /README.md
status: implemented
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-19
sources:
  - resource: opencode://ses_eaa17ad2effeEEkcV1BqiWcrUp
    title: Original launcher commission and additive Rust runtime verification and tracing requirements
  - resource: opencode://ses_eaa172275ffe5q1qIrtAKpAG3q
    title: Implement and exercise the Rust launcher, command model, interchangeable lookup, PTY verification driver, and tracing contracts
  - resource: /design/launcher/research0.gpt6a.md
    title: Independent architecture assessment and integrated relative-path and guarded-eval recommendations
  - resource: file:///home/rektide/src/tmux-interface-control/README.md
    title: Native tmax client capability, feature, platform and development-dependency contracts
  - resource: file:///home/rektide/src/tmux-interface-control/src/endpoint/mod.rs
    title: Existing-server lookup, socket resolution and classified connection errors
  - resource: file:///home/rektide/src/pipewire-native-rs-node/verification/trace.md
    title: Isolated tracing-test diagnostic precedent and global-buffer caveats
---

# tta

Attach to a matching tmux session. If none matches, find a matching directory
under `~/src`, create a session named after that directory, and enter it.

```sh
tta project
tta --root ~/work project
tta --socket-name work project
tta --session-name project-name project.name
```

Outside tmux, `tta` hands its process and terminal to the real tmux client.
Inside the same server, it switches the existing client instead of nesting an
attachment. New inside sessions are created detached, then selected on success.

## Build and install

Requires Rust, tmux 3.2 or newer, and a Unix terminal. The default native lookup
requires **little-endian Linux** and a tmux protocol supported by the sibling
[tmax checkout](file:///home/rektide/src/tmux-interface-control/README.md).
Live tests here use tmux **3.7c**; they do not establish every version's behavior.

This development workspace depends on `../tmux-interface-control`, package
`tmax` 0.4.0, with only its `client` feature. Keep that sibling checkout available;
this is not a claim that its current API is published on crates.io.

```sh
cargo install --path app/cli --locked
# Include the optional scripted lookup backend:
cargo install --path app/cli --locked --features cli
# CLI-only runtime lookup, e.g. for other Unix platforms:
cargo install --path app/cli --locked --no-default-features --features cli
```

`native` is the default feature/backend. With both compiled, select
`--backend native` or `--backend cli`. There is no hidden fallback after a native
error. CLI-only execution still uses tmax's shared typed schema/decoder, whose
current `client` compilation feature also includes native plumbing.

## Matching and failure behavior

1. Match existing session names first: **exact name**, otherwise a **unique,
   case-sensitive literal substring**.
2. Only if no session matches, apply that policy to immediate directories under
   `--root` (default `$HOME/src`). Directory symlinks count, retaining their entry
   basename. Regular files and deeper descendants do not.
3. Report sorted ambiguity instead of selecting an arbitrary first match. Session
   ambiguity never falls through to project matching.

Arguments are not globs or regular expressions. Quote shell metacharacters; use
`--` before a fragment beginning with `-`. Project matching requires UTF-8 names;
non-UTF-8 directory entries are skipped. New tmux start-directory paths must also
be UTF-8. No project root or HOME is needed when an existing session matches.

New names containing `.`, `:`, or control characters are rejected rather than
normalized into potentially colliding names. Use `--session-name` to choose a
distinct safe name. This option affects creation only, not session matching.
An override colliding with another existing session is an error.

Native discovery does not spawn a process or implicitly start a server. Missing
or refused server sockets and a server with no sessions are empty inventories;
permission, protocol, decode, and other errors are failures. CLI lookup uses the
same strict typed decoder, `-u` for UTF-8 framing, and narrowly matched C-locale
absence diagnostics. Native discovery inherits tmax's bounded connection/output
policy; the optional CLI lookup currently has no equivalent timeout/output cap.

Existing sessions are targeted by canonical server-local IDs, not tmux's fuzzy
name lookup. A duplicate-creation race fails visibly: rerun to resolve again.
There is deliberately no `new-session -A` fallback that might change an existing
session's cwd. Failed switching does not automatically remove a newly created
detached session. Server restarts, rename/replacement between create and switch,
and changes after shell emission remain races, not hidden reconciliation duties.

## Shell output

No shell is needed for normal execution. To change the calling shell's directory
as well when entering a new project, evaluate the emitted POSIX shell expression:

```sh
tta_plan=$(tta --print-shell project) && eval "$tta_plan"
```

The guard matters: `eval "$(tta ...)"` alone can conceal a failed lookup.
Printing only observes state and emits a plan; it does not create or attach.
Stdout contains only the complete command expression. Errors and optional tracing
use stderr. Ordinary output never replaces the evaluating shell; explicitly add
`--exec-shell` if replacement is wanted.

New-project output models `cd -P ... && tmux ...`; direct execution supplies the
same physical cwd to child processes without changing the parent shell's cwd.
Shell output is a snapshot, not a fresh lookup at evaluation time. Evaluate it in
the same terminal/context rather than saving it as a durable session identity.

## Endpoint and client options

- `--socket-name LABEL` selects tmux's `-L` label; tmux still creates/checks its
  private socket directory on cold startup. Relative `TMUX_TMPDIR` is frozen
  against the invocation cwd.
- `--socket-path PATH` selects `-S`; relative paths are frozen before project cd.
  As with tmux itself, an explicit socket path needs an existing parent directory.
- Inside tmux, selecting another server fails explicitly: clients cannot switch
  between servers. Run outside tmux, or deliberately unset `TMUX` and `TMUX_PANE`
  before creating a separate client.
- `--tmux PROGRAM` chooses the client executable (also used by CLI lookup).
  Explicit relative paths such as `./bin/tmux` are frozen before cd. Bare names
  retain normal PATH lookup: relative PATH entries can resolve differently after
  entering a project; use an absolute executable path when that matters.
- `--tmux-config PATH` passes `-f` to the final client; relative configuration
  paths also belong to the invocation cwd. `/dev/null` is useful for isolation.
- `--trace` emits structured JSON events on stderr: lookup, match decision,
  operations, emission, handoff attempt, and failure. A handoff event means an
  **attempt**, not proof that tmux attached after replacing the Rust process.

## Reusable pieces

| Home | Responsibility |
| --- | --- |
| [shell/command](shell/command/src/lib.rs) | Bon-built argv commands, validated environment assignments, physical cd, short-circuit conjunction; direct run/Unix exec or POSIX shell rendering |
| [session/launcher](session/launcher/src/lib.rs) | `SessionLookup`, native/CLI acquisition, deterministic resolution, endpoint policy and launch planning |
| [app/cli](app/cli/src/main.rs) | Arguments, structured diagnostics and interactive handoff |
| [verification/driver](verification/driver/src/lib.rs) | Shared Rust runtime exercise/observation library plus `tta-verify` command |

The generic command model knows no tmux syntax. The launcher separately handles
tmux's argv trailing-semicolon rule, exact targets, and format operands (`-s` and
`-c`). Shell quoting alone does not make tmux formats literal.

[Launcher research: language boundaries, native/CLI lookup and runtime acceptance](design/launcher/research0.gpt6a.md)
provides the independent rationale, residual races, and focused review findings;
it is design evidence, not a replacement owner for this guide.

## Runtime verification

`cargo verify` runs the Rust **runtime driver**, not a formatter/test orchestrator.
It exercises the actual built `tta` executable with a PTY on exclusively owned
private tmux sockets:

```sh
cargo build -p tta --all-features
cargo verify --backend native
cargo verify --backend cli --inside --report report.json
# Narrow a reproduction:
cargo verify --tta target/debug/tta --backend native --mode shell --scenario new
```

By default it covers direct and evaluated-shell execution for both cold creation
and an existing session. `--inside` instead starts a real attached origin pane,
invokes tta there, and verifies switching the **same client**, preserving the
original pane/shell. The [integration tests](app/cli/tests/acceptance.rs) call that
same library for both contexts and both compiled backends.

The JSON report contains process lifecycle events, captured output/PTY transcript,
actual session/client/pane IDs and cwd before/after detach, parsed child tracing,
and cleanup results. Outside direct mode verifies exec preserves the client PID;
shell mode verifies the evaluator survives. Detach must preserve the same session
and pane. Traces are compared against those independent tmux observations rather
than treated as completion evidence themselves.

Queries/lifecycle phases have bounded timeouts (`--timeout`, default 10 seconds);
retained process/PTY output is bounded to 256 KiB per stream. Fixtures use
`/dev/null` tmux config and isolated homes. Only owned endpoints are cleaned up.
Reports retain diagnostics after fixture removal. Missing tmux, unsupported
backends, failed assertions and failed cleanup return nonzero, not skipped
acceptance reported as success.

## Development checks

Tests and recorders are Rust/Cargo artifacts; no Python harness is required.
Live tests need tmux on PATH. Native tests require its supported Linux platform.
Scoped `tracing-mock` expectations and the complementary `tracing-test` diagnostic
test live in separate test binaries; child JSON is captured by the runtime driver.

```sh
# Name owned packages: --all can traverse the sibling tmax dependency.
cargo fmt -p tta-command -p tta-launcher -p tta -p tta-verify --check
cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings
cargo nextest run --workspace --all-features
cargo nextest run --workspace --no-default-features --features cli
cargo test --workspace --all-features --doc
```

The development tmax dependency currently emits its own compiler warnings;
`--no-deps` keeps clippy's deny-warnings policy scoped to this workspace.
