---
type: VerificationToolkit
title: Runner-free supplied-facts entry attempts
description: No-I/O witness driving fresh EntryAttempt instances through actual fresh binding and observation-only export, with protocol rejection and package-isolated dependency checks
resource: /verification/core/README.md
status: implemented
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
updated: { at: 2026-11-10, by: "model:zai-coding-plan/glm-5.3#max" }
sources:
  - resource: opencode://ses_e3cf482d5ffeyD5hefdVDZGmbe
    title: Rewrite the toolkit account around the retained actor after the 0.8.0 cutover deleted the replay API and its cost probe
  - resource: opencode://ses_e3d19979bffe2Eu6JnXnb11i7H
    title: Migrate this witness to fresh attempts through actual fresh binding/export with protocol rejection checks
  - resource: opencode://ses_e804f36ddffepWs5FSeXN6VDBv
    title: Link the promoted process-carrier round-trip sibling while preserving this toolkit's no-I/O supplied-facts and poison contract
  - resource: opencode://ses_e80ce13ddffeiWXfWVMBW9qXv5
    title: Promote canonical-root and adversarial replay assertions plus a reusable poisoned-environment and optional syscall witness after independent LC2 review
  - resource: opencode://ses_e80e6e9a9ffe2N24sOOfQnrL3D
    title: Extract the pure core and optional runner and implement supplied-facts exercises, semantic checks and a package-only verification hook
  - resource: /verification/core/src/lib.rs
    title: Four fresh-attempt scenarios, typed checks and protocol rejection witnesses
  - resource: /verification/reviews/local-core0.gpt6a.md
    title: Independent checks and toolkit coverage findings used for deliberate promotion
---

# Runner-free supplied-facts entry attempts

This toolkit compiles and exercises `tta-launcher` and `tta-command` **without
`tta-runner`, native tmax endpoints, or tmux execution**:

```sh
./verification/core/check.sh --report /tmp/opencode/tta-core-report.json
cargo run -p tta-core-witness --no-default-features -- --scenario create
python3 verification/core/poison.py --trace --report /tmp/opencode/tta-pure-poison.json
```

The script checks package-only feature isolation, runs the witness tests, and
emits JSON. No host is contacted; `/target/...` paths in these fixtures need
not exist. `execution_attempted: false` is intentional, not a simulated
successful mutation.

Each scenario constructs a **fresh `EntryAttempt`** from a supplied frozen
context, answers every correlated read with typed native facts, delivers
`Delivery::Export` at `Waiting::Planned`, answers the fresh binding read, and
inspects the frozen `ExportedAction`. The four scenarios cover canonical-ID
session entry (`Existing`), a supplied canonical project root (`Project`),
clean creation with explicit target author/license defaults (`Create`), and a
fresh-satellite grouped inventory (`Grouped`). Checks inspect semantic decision
domains, canonical-ID retention, the single grouped create → attach →
`keep-last` queue, the selected physical route surviving into `ChangeDir`, one
generator materialization with the actual supplied defaults, and that export
never issued an effect. Truncated, wrong-shape, stale and duplicate replies
fail; string snapshots alone are not the oracle. Reports also construct each
required versioned probe through an explicit target shell/cwd setup without a
runner. These scripts are emitted, **not executed**.

The witness's protocol-rejection tests pin the interaction contract directly:
repeated `waiting()` is inert (no replay or reissue), a wrong observation kind,
a stale request ID and a duplicate reply are each rejected without consuming
the pending request, and changed fresh anchor facts stop both `OpenView` and
plain `EnterView` binding rather than authorizing mutation. The former
immutable-replay `replay_cost` benchmark had no destination role in the owned
architecture and was removed with the replay API itself; the historical
measurements remain recorded in the
[architecture review](/design/entry/architecture0.gpt61sol.md#replay-cost-evidence-rather-than-an-assumed-optimization)
at its recorded revision.

[`poison.py`](poison.py) runs all four scenarios in two disagreeing environments
with failing executable canaries, compares byte-identical reports, and checks the
package-only feature graph. It needs Python **locally for verification only**,
not on a launch target. `--trace` additionally checks target access, network I/O
and child execution when `strace` is available; availability and the actual trace
are recorded, never inferred from a missing tool. Build the witness first via
`check.sh` or `cargo build -p tta-core-witness --no-default-features`.

## Library hooks

[`exercise(scenario)`](src/lib.rs) returns a serializable `Report` on success
or failure. Reports contain readable request/reply representations; those
strings are diagnostics, **not a wire protocol or a JSON transcript-import
API**. Callers wanting their own facts drive the public
[`EntryAttempt`](../../session/launcher/src/entry/mod.rs) protocol directly:
`waiting()` yields one correlated `ReadRequest`, `advance(Event::Observed(..))`
installs the typed result, and `Event::Deliver` selects export or execution.
Errors from an observer stop at the observer boundary; there is no
"unknown means empty" reply. Local I/O is implemented separately by
[`tta-runner::observe`](../../session/runner/src/observe/mod.rs) and its
shared read driver.

## Scope and independent runtime evidence

This establishes supplied-facts attempts, protocol rejection and pure probe
construction, not live discovery or a complete deferred shell workflow.
`entry::probe::Probe` emits and decodes the framed request-bound protocol.
Actual transport evidence lives in the [SSH toolkit](../ssh/README.md). ⇄ The
separately maintained [emitted-probe round-trip](../probes/README.md) executes
the public emit/execute/decode loop through stdlib processes without a runner,
including registry and generator-default acquisition. Its intentional
filesystem/process I/O is kept out of this supplied-facts package; this
package's poison and dependency witness are unchanged.

Real process/shell parity lives in
[`runner/tests/expressions.rs`](../../session/runner/tests/expressions.rs), and real
tmux grouped/argv/cwd behavior in
[`runner/tests/satellites.rs`](../../session/runner/tests/satellites.rs).
[`cargo verify`](../driver/src/main.rs) retains the existing real-PTY outcome
toolkit, and [entry attempt checks](../entry/README.md) own the focused
machine/host suites of the 0.8.0 surface.
