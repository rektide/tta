---
type: ImplementationReport
title: Production cutover — execution effects, real CLI and consumer adoption
description: Machine-owned Execute delivery with requested/in-flight effects and honest mutation knowledge, the creation-root witness, the actual CLI and term-preserver call sites driving the retained actor, and the exact remaining deletion/version acceptance
resource: /design/entry/cutover0.glm53m.md
status: implemented-partial
author: model:zai-coding-plan/glm-5.3#max
date: 2026-11-10
sources:
  - resource: opencode://ses_e3d4bcd7dfferOAqdJlG648aqj
    title: Implement execute/effects and the creation-root witness, cut the CLI and term-preserver over to the retained actor, and verify real-terminal and loopback SSH adoption
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commissions the cutover stage under the approved redesign and retains acceptance
  - resource: /design/entry/redesign0.gpt6a.md
    title: Approved destination, stage-four acceptance and effect-lifecycle obligations
  - resource: /design/entry/hosts0.gpt61sol.md
    title: Exact G handoff items followed here, including the missing creation-root binding obligation
  - resource: /design/entry/kernel0.gpt61sol.md
    title: Deletion-obligation table that remains the cutover contract
  - resource: /session/launcher/src/entry/entry.machine.json
    title: Extended executable chart owning bind evidence, dispatch and requested/in-flight run states
  - resource: /session/launcher/src/entry/protocol.rs
    title: Effect vocabulary, receipts, mutation knowledge, delegation and cancellation outcomes
  - resource: /session/launcher/src/entry/bind.rs
    title: Pure execution-shape choice and the collected creation-root/destination evidence
  - resource: /session/launcher/src/entry/action.rs
    title: Native creation invocation, native terminal tail and the bound shape
  - resource: /session/runner/src/execute/mod.rs
    title: One thin effect driver with the shared diagnostic trace contract
  - resource: /session/runner/src/execute/local.rs
    title: Generate/directory/native-create/terminal execution with typed failures
  - resource: /session/runner/src/execute/ssh.rs
    title: Frozen-setup terminal effects and the print/execute SSH drive
  - resource: /app/cli/src/main.rs
    title: Production CLI on semantic attempts, catalog completion and SSH handoff
  - resource: /session/runner/tests/entry_execute.rs
    title: Fake-backed effect-ordering, failure-knowledge and no-fallback witnesses
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Ported embedded_tta/embedded_ssh call sites with explicit grouping and fresh retries
  - resource: /verification/entry/README.md
    title: Focused command coverage extended with the effect suite
---

# Outcome and standing

**The production CLI and the actual consumer now drive the retained entry
attempt through delivery and effects.** Execute delivery chooses an immutable
bound execution shape purely from the fresh binding inventory before any
mutation; every effect walks requested → in-flight with started/finished
correlation; failed native creation reports possible mutation with earlier
receipts retained and never falls back to a CLI block; an outstanding
terminal request can be delegated once, ending tta's ownership honestly; and
cancellation preserves prior receipts and in-flight uncertainty. The missing
creation-root canonical-identity witness is implemented and verified to stop
changed roots before the destination lstat is even requested. The temporary
`entry-internal` gates are gone: entry, its observers and the effect driver
are the public surface.

Real-terminal evidence this session: the full CLI acceptance matrix (direct
and shell, native and CLI backends, inside and outside, existing and new
work, scaffold, completion/zsh, entry options) passed against real tmux, and
authenticated worker-free loopback SSH planning produced the complete frozen
target-side block through the new actor. The consumer's ported call sites
compile and pass its launcher suite; three pre-existing `launch.rs`
ssh-config environment probes fail identically before and after the port.

Work anchor: `tta-entry-attempt-cutover`, still open — the coordinated old-
runtime deletion, the 0.8.0 version/pins/receiving-lock landing, the
verification-driver SSH port and the real-terminal consumer run remain, with
the exact list below.

## What landed (explicit owned-path commits)

| Commit | Content |
| --- | --- |
| `67b45138` | Execute/effects in the retained chart: shape choice, requested/in-flight states, receipts, delegation, cancellation knowledge, creation-root witness; launcher tests and regenerated graph |
| `51f81969` | Runner `execute` module: local effect interpretations, one thin drive loop, fake-backed ordering/failure/no-fallback witnesses |
| `a5caf2be` | Production CLI cutover: local/SSH run/print, catalog completion, environment.rs reduced to CLI-value collection, entry made public |
| `2de3b967` (term-preserver) | Ported `embedded_tta`/`embedded_ssh` onto acquisition + drive with explicit grouping; worker/proxy/PTY ownership unchanged |

### Effect protocol and chart

- `Delivery::Execute { native }` states the host's native capability at
  `planned`; eligibility is decided purely at `bind_action.lower` from the
  decision (new work only), local origin, inside-switch client, live
  **nonempty** binding inventory and UTF-8-representable startup argv. Cold,
  sessionless, outside, raw-argv and CLI-only hosts get the complete terminal
  block by chosen policy — never as a post-failure fallback.
- `run.dispatch → generate? → directory → native → terminal`, each effect a
  `requested → in_flight` pair in the executable chart with one
  `EffectStarted` before any target contact and exactly one
  `EffectFinished`. Wrong/stale/duplicate starts, wrong receipts and
  delegation of non-terminal or started effects are protocol rejections that
  change no state; `Waiting::InFlight` cannot redispatch.
- Outcomes carry `EffectKnowledge` (finished receipts in dispatch order,
  dispatched-but-unfinished IDs): `Completed`, `Delegated { terminal }`,
  `Stopped` and `Cancelled` all preserve it. A failed effect keeps its typed
  native source and `MutationKnowledge::{NotStarted, MayHaveOccurred,
  Acknowledged}` — acknowledged native creation survives output-forwarding
  failure as a fact, not a lost launch.
- `Generate` failure prevents directory/native/terminal effects (journal
  witness: no later effect issued). `CreateNative` failure against a dead
  socket stops with `MayHaveOccurred`, earlier `Directory` receipt retained,
  and no CLI block is ever dispatched afterwards. A returned terminal status
  is an aggregate: success becomes a `Terminal { code }` receipt, failure
  becomes an honest `MayHaveOccurred` stop.
- Export always lowers the complete portable block even for a
  native-eligible binding (witnessed); the native tail is `cd` + exact
  `switch-client`, never a second creation.

### Creation-root witness (the missing binding obligation)

`ScaffoldAndStart` binding now issues `ReadKind::CreationRootIdentity { root,
expected }` first; the ingress verifies the canonicalized configured root
against the preparation-time physical root and stops with
`EntryError::CreationRootChanged` **before** the chart requests the binding
destination lstat (journal witness: no binding `Destination` read after a
changed root). The collected evidence (`CreationRoot` then `Destination`) is
ordered, and both witnesses ride the bound action. StartProject keeps its
single route-identity read; existing-work bindings read no extra evidence.

### Production CLI

One `EntryAttempt::acquire` per invocation with explicit CLI values as
overrides — whole-layer environment precedence, conflict detection and
defaults belong solely to the shared context policy (`environment.rs` no
longer pre-expands any environment layer). Local run/print drive the local
observer and executor with `Handoff::Exec` replacing the process on success;
labels (`resolution`, `plan`, `emission`/`handoff`, `launch.native`) derive
from the semantic decision via the runner's shared trace contract. SSH run
and print drive the frozen observer; print wraps the exported target-side
block in the resolved local SSH invocation; execution dispatches the terminal
effect through the same connection/setup. Remote overrides carry only
explicit target-side choices; local defaults never leak. Completion is the
read-only catalog: aliases-only for `session-aliases`, names+aliases for
`sessions`, identity-aware project selectors with the bare-name filter, and
registry names for `scaffolds`; policy/registry failures are fatal while
per-root source failures stay diagnostic with partial candidates — the
documented partial-evidence contract, which two 0.7 tests asserted against
and were updated accordingly.

### Consumer port

`embedded_tta`/`embedded_ssh` acquire one attempt each and drive it through
the shared interpreters. `tmux_group` maps to `Independent`/`Shared`, never
`Auto`; the consumer's independent ordered-roots policy supplies explicit
overrides; target-local defaults come from target facts; `TTA_BACKEND`
selects the observer and native capability. A user retry re-runs the worker,
naturally building a fresh attempt with fresh observations. Worker/proxy/PTY/
master ownership and the non-library completion contract are untouched.

## Checks executed this session

| Check | Result |
| --- | --- |
| Launcher entry suite (all features / no default / rekon only / rekon-native only) | **118 / 95 / 111 / 102 passed** |
| Runner full suite incl. `entry_hosts` + new `entry_execute` (all features) | **72 passed** |
| CLI suites: acceptance (16), launch (9), completion (9), scaffold, ssh-policy, independent-views, entry-options | **64 passed** against real tmux |
| Full workspace `cargo test --all-features` | **273 passed, 0 failed** |
| clippy `-D warnings` (launcher/runner/cli all-targets all-features) | Passed (dependency-owned tmax warnings unchanged) |
| Structural entry + catalog graph `--check` after regeneration | Passed; Python renderer **5/5** |
| Authenticated worker-free loopback SSH (private sshd, key-only): `--ssh --print-shell` planning | Complete frozen target-side block emitted, exit 0 |
| Loopback SSH failure probe (wrong option spelling) | Typed `EntryError::Observation` with retained bounded capture record and full source chain |
| term-preserver `cargo test --features tta-lib` | **177 passed, 3 failed** — the three are pre-existing `launch.rs` ssh-config environment probes, untouched by the port |

Scratch: `TTA_TEST_TMPDIR=.test-agent/entry-cutover` (its README explains);
`/tmp/opencode` is not writable here. Direct SSH exec without a local PTY
blocks in `ssh -tt` exactly as the old CLI did — the real-terminal exec
matrix belongs to the PTY driver listed below, and planning-side loopback is
covered above.

## Exact remaining acceptance (ticket stays open)

1. **Coordinated deletion** per the kernel table, now unblocked: launcher
   `workflow.rs`+`workflow/`, `resolve.rs` scheduler (`Resolution`,
   `NeedProjects/NeedDestination`, `Selection` surface; relocate
   `validate_name`), `scaffold.rs` resolver wrapper, `plan.rs` +
   `prepared.rs` sidecars, `target::Bootstrap`/`Options`, old probe
   envelopes; runner `driver.rs`, `resolve.rs::resolve`, `scaffold.rs`
   resolver loop, `ssh.rs::prepare`/`PreparedRemote`, `execution.rs`,
   `backend.rs` (superseded by `observe::local::SessionBackend`), and the
   duplicate local socket/context helpers once caller-free. The old
   runner-side registry/config layering goes with them; the shared context
   policy, registry acquisition task and local default observer are the
   owners.
2. **Test migration** for that deletion: `resolution.rs`, `probes.rs`,
   `project_roots.rs` (runner), `trace_contract.rs`, `trace_messages.rs`,
   `satellites.rs`, `scaffold.rs`, `private_tmux.rs`, launcher
   `project_roots.rs`, and the `launch.rs` native section in the CLI —
   semantic equivalents exist (entry tests, `entry_execute.rs`, catalog
   tests, the acceptance matrix); rewrite the genuinely-load-bearing ones
   against the attempt/executor surface and retire pure replay assertions
   (including the old trace events, superseded by the runner trace
   contract).
3. **0.8.0 coordinated landing**: workspace minor bump, exact internal pins,
   receiving consumer lock refresh, `rkt`-checked docs — one landing with
   the deletion so 0.8.0 is the clean surface (pre-1.0 minor-per-change
   policy makes splitting it cost an extra minor).
4. **Verification-driver SSH port**: `verification/driver/src/ssh.rs` still
   drives `runner::prepare`; port it to acquisition+`execute::ssh::drive`,
   then run the commissioned actual-SSH exercise (planning + terminal) and
   the real-terminal consumer (`--consumer`) matrix.
5. **Real-terminal consumer run**: the ported `embedded_tta`/`embedded_ssh`
   through term-preserver's own worker/PTY harness, including a retry
   building a fresh attempt.
6. Docs promotion (README/doc entry accounts, consumer account adoption
   status) rides the consolidation ticket as commissioned; this report and
   the verification guide are the current evidence.

## Connections

- [Entry graph](/doc/entry/attempt-graph.md) now shows the bind-evidence
  split and the run states; [catalog graph](/doc/entry/catalog-graph.md) is
  unchanged in shape.
- [Kernel report](kernel0.gpt61sol.md#actual-deletion-obligations-at-coordinated-cutover)
  remains the deletion contract; nothing in this stage weakened it.
- [Hosts report](hosts0.gpt61sol.md) handoff items 1–7 are each addressed
  above or explicitly listed as remaining.
