---
type: VerificationReview
title: Independent review of the local pure-core and runner checkpoint
description: Revision-bound source review, poisoned-environment runner-free evidence, eight adversarial checks, and remaining reusable-toolkit gaps
resource: /verification/reviews/local-core0.gpt6a.md
status: reviewed-with-followups
author: model:openai/gpt-6-astra#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e80e6a338ffeMZsVeoLYisi7h8
    title: Independently scout verification seams, review committed local milestone, run poisoned-context and adversarial witnesses, and identify toolkit followups
  - resource: opencode://ses_e80f5d05bffe77stycHGRJOH29
    title: Parent commissions independent review of the local milestone while the next implementer proceeds with SSH
  - resource: /design/launcher/runner-core0.gpt6a.md
    title: Commissioned pure-core, runner, remote-context and reusable-verification requirements
  - resource: /design/launcher/runner-core1.gpt61sol.md
    title: Implementer's local milestone, reported checks and explicit remote-work boundary
  - resource: /verification/core/src/lib.rs
    title: Supplied-facts fixtures, exact transcript replay and semantic report assertions
  - resource: /session/launcher/src/workflow.rs
    title: Observation ordering, supplied replies and ready launch data
  - resource: /session/runner/src/driver.rs
    title: Local observer, shared driver and standard-error boundary
  - resource: /session/runner/src/resolve.rs
    title: Filesystem acquisition and destination error classification
  - resource: /verification/driver/src/lib.rs
    title: Existing PTY driver fixture and capture boundaries
  - resource: file:///home/rektide/src/tmux-interface-control/Cargo.toml
    title: Operations feature separation inspected at the recorded tmax revision
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Committed embedded-consumer migration inspected independently of concurrent ControlMaster work
---

# Outcome

**The inspected local/core interfaces satisfy the architectural split, and the
new concern-driven checks passed.** No production correctness blocker was found
in this bounded review. Two reusable-toolkit followups remain below; the PTY
driver's isolation gap should be fixed before another live driver exercise.
Parent acceptance remains the parent's decision.

This reviews the [local checkpoint](/design/launcher/runner-core1.gpt61sol.md),
not the concurrently developing SSH slice. No SSH connection or tmux server
operation was performed during this review. Reported broad checks were trusted
and not repeated.

## Exact scope

| Repository | Reviewed revisions |
| --- | --- |
| tta | Implementation `a3ed1450`; checkpoint documentation `4a534f7d` |
| tmax | Feature split `7104042c`, pure decoder tests `898668c2`, consumer account `04ec6ea1`, formatting `a2d1c0e0` |
| term-preserver | Local integration `6317dcdc` |

The implementation review used committed source snapshots under
`.test-agent/verification-review/snapshot/`, with executable/document symlinks
omitted. Builds used `.test-agent/verification-review/target/`; the shared CLI
executables were not overwritten. The tmax jj workspace's backing Git store is
in the archived repository; snapshots were exported from that store, not its
unrelated colocated `.git`. Concurrent SSH and ControlMaster edits were excluded.

Source attention covered core workflow/resolution/scaffolding, command rendering,
semantic native plans, local observation/execution, CLI composition, the tmax
feature delta and consumer migration. This was not a whole-repository audit.

## Findings

### LC1 — P2: the reusable PTY driver still lacks its required isolation proof

At `4a534f7d`, [driver run setup](/verification/driver/src/lib.rs) lines 239–250
only checks `!socket.exists()` before an existing-session fixture immediately
creates its server. Cold scenarios likewise lack a read-only tmux preflight.
The new fixture still falls back to the generic temporary directory
(lines 493–498), and its fallback `Drop` discards `kill-server` diagnostics
(lines 590–596).

These are retained scout findings, not a newly observed misdirected command.
They conflict with the commission's explicit socket-proof/diagnostic requirements.
The [checkpoint](/design/launcher/runner-core1.gpt61sol.md) lines 130–133 overstate
coverage when saying real fixture operations use read-only absence proof:
[CLI launch fixtures](/app/cli/tests/launch.rs) and
[runner private-server fixtures](/session/runner/tests/private_tmux.rs) were
improved, but the shared driver was missed.

**Requested correction:** establish and record a read-only failure for the exact
owned absolute `-S` socket before either scenario can mutate; require the approved
scratch parent; retain fallback-cleanup diagnostics. Reuse this capability in the
future SSH fixture. Keep this separate from the already acknowledged capture/EOF
work, which is also required before reusing capture for remote observations.

### LC2 — P3: canonical-target-path coverage is claimed more strongly than asserted

[Core exercise](/verification/core/src/lib.rs) lines 149–163 unconditionally
records `target-local paths from supplied facts`, but checks only the tmux
executable there. Its Project case checks command count; its Create case checks
package author/license. Neither checks that the selected directory and emitted
`ChangeDir` use the supplied canonical `/target/physical/projects` root rather
than the requested `/target/projects` root.

The independent `observed_canonical_root_drives_selection_and_emitted_directory`
test **passed for both Project and Create**, so this is a regression-coverage gap,
not evidence that the current implementation selects the wrong path.

**Requested correction:** add those explicit selection/command-data assertions to
the checked-in witness before advertising that check in its reports. This is a
small, non-runtime promotion from the scratch tests described below.

## Independent evidence

### Runner-free build, dependency graph and poisoned environments

Built only `tta-core-witness` from the frozen snapshot, offline and with its
recorded lock. The package's normal feature graph contains tmax `operations` and
excludes `tta-runner`, tmax `client`/`control_mode`, `nix`, `rustix`, and `bytes`.
This check was separate from the adversarial package that intentionally imports
the runner.

Ran all four supplied-facts scenarios twice with different invocation directories,
missing HOME/config/root locations, invalid ambient backend/configuration,
contradictory author/license values, poisoned TMUX/socket settings and a PATH
containing only failing executable canaries. Results:

- **4 scenarios × 2 contexts; byte-identical JSON; no canary executed.**
- Target author/license and canonical paths continued to come from supplied facts.
- A `strace` observation of one poisoned run showed only the initial executable,
  dynamic-loader files, Rust runtime `/proc/self/maps` access and process exit:
  no target-path access, socket/connect syscall or child execution.

The source scan of launcher, command and generator production code found no
environment/filesystem/process acquisition. Error types retaining `std::io::Error`
are data, not I/O. The syscall witness supports the exercised paths; it is not a
universal proof about all dependency APIs or arbitrary tracing subscribers.

### Eight scoped adversarial checks: 8 passed, 0 skipped

| Check | Observable result |
| --- | --- |
| Wrong reply kind at every request in all four scenarios | Rejected; pending request unchanged; valid continuation still succeeds; replies after readiness rejected |
| Every truncated transcript prefix and wrong endpoint request identity | Replay fails rather than manufacturing missing observations |
| Supplied canonical project root | Both selection and final `ChangeDir` use the supplied physical root |
| Session hit without a root; invalid generator choice | Hit succeeds without filesystem facts; bad choice fails before first observation |
| Fresh grouped inventory containing suffix 1 | Chooses suffix 2, retains selected base ID, and emits one queue with two separators |
| Explicit generator author/license | No defaults observation needed; explicit values survive emission |
| Observer permission failure | Stops acquisition after two observations; contextual message and underlying typed I/O cause survive both driver and public error wrapper |
| Actual owned filesystem failures | Missing target is absent; dangling link is present; non-directory ancestor, symlink loop and denied traversal remain errors |

These tests used no real tmux, SSH or generator commands. Permission behavior was
observed as UID 1000. The scratch package used offline resolution and retained its
generated lock; it was not the broad workspace suite.

### Source-level compatibility and integration review

- The generic interpreter retains ordered short-circuit execution and process-local
  cwd handling in [runner command execution](/session/runner/src/command.rs).
- [Native execution](/session/runner/src/execution.rs) consumes semantic creation
  data; preflight still precedes generation/mutation, and native failure does not
  retry through CLI. Grouped creation remains outside that native shortcut.
- [Workflow](/session/launcher/src/workflow.rs) preserves session-first matching,
  explicit fresh satellite observation and registry-choice ordering. Context and
  registry acquisition remain the documented caller/runner prelude.
- The committed consumer uses shared `tta_runner::prepare`, selects matching
  lookup/execution backends, and executes inside its worker. Completion imports
  were migrated. No incoming integration omission was found in those hunks.
- The tmax split gates the operation/query vocabulary separately from native
  endpoint/protocol modules. Existing pure decoder tests were made reachable in
  the operations-only build; parser retirement was not folded into this change.

The implementer's 128 all-feature, 62 CLI-only, 20 operations-only and consumer
results remain attributed to the [checkpoint](/design/launcher/runner-core1.gpt61sol.md).
This review added independent evidence instead of repeating those checks.

## Reproduction and deliberate toolkit promotion

Checked-in starting point: [runner-free toolkit](/verification/core/README.md)
and [real-PTY driver](/verification/driver/src/main.rs).

The additional review witnesses are retained locally under
`.test-agent/verification-review/`:

```sh
R="$PWD/.test-agent/verification-review"
CARGO_TARGET_DIR="$R/target" cargo build --locked --offline \
  --manifest-path "$R/snapshot/tta/Cargo.toml" \
  -p tta-core-witness --no-default-features
python3 "$R/verify-pure.py"
CARGO_TARGET_DIR="$R/target" cargo nextest run --locked --offline \
  --manifest-path "$R/adversarial/Cargo.toml" --lib
```

Evidence files: `evidence/pure-summary.json`, `poison-a.json`, `poison-b.json`,
`dependency-features.txt`, and `pure-syscalls.log` beneath that scratch root.
The adversarial nextest run ID was `8ef4d906-14d6-4659-b9e8-841f43507ca2`.

Promote the canonical-path assertions and expanded reply/error cases into the
existing core/runner verification homes after coordinating ownership. A
checked-in poison-environment command would strengthen the existing package-only
hook; syscall tracing can remain optional with explicit availability reporting.
Scratch retention alone does not satisfy the commissioned reusable-toolkit goal.

## Remaining remote checkpoint

Keep target setup, framed probes/decode, bounded complete-EOF capture, actual
SSH/no-target-tta evidence, prepared remote terminal handoff and mosh observation
policy with the next commissioned slice. Preserve the distinction between a
process-carrier fixture and real SSH, and between supplied-facts emission and
complete deferred workflow compilation. The first milestone does not claim
those capabilities; this review does not certify their concurrent implementation.
