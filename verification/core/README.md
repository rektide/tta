---
type: VerificationToolkit
title: Runner-free supplied-facts replay
description: Runnable pure-core witness, package-isolated runtime dependency check and typed transcript replay API
resource: /verification/core/README.md
status: implemented
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
updated: { at: 2026-11-06, by: "model:openai/gpt-6.1-sol#high" }
sources:
  - resource: opencode://ses_e4dca921dffexww2JZDdY3Zyne
    title: Independently assess immutable replay costs and add a runner-free informational cost probe with explicit workload sizes
  - resource: /verification/core/examples/replay_cost.rs
    title: Median batch costs for ready next() and complete transcript replay, without target I/O or timing thresholds
  - resource: opencode://ses_e804f36ddffepWs5FSeXN6VDBv
    title: Link the promoted process-carrier round-trip sibling while preserving this toolkit's no-I/O supplied-facts and poison contract
  - resource: opencode://ses_e80ce13ddffeiWXfWVMBW9qXv5
    title: Promote canonical-root and adversarial replay assertions plus a reusable poisoned-environment and optional syscall witness after independent LC2 review
  - resource: /verification/reviews/local-core0.gpt6a.md
    title: Independent checks and toolkit coverage findings used for deliberate promotion
  - resource: opencode://ses_e80e6e9a9ffe2N24sOOfQnrL3D
    title: Extract the pure core and optional runner and implement supplied-facts replay, semantic checks and a package-only verification hook
  - resource: opencode://ses_e80f5d05bffe77stycHGRJOH29
    title: Parent commissions runner-free planning, optional runner and reusable verification toolkits
  - resource: /design/launcher/runner-core0.gpt6a.md
    title: Accepted architecture and checkpoint sequence
  - resource: /session/launcher/src/workflow.rs
    title: Explicit inputs, observation requests and replies, progression and ready launch plans
---

# Runner-free supplied-facts replay

This toolkit compiles and exercises `tta-launcher` and `tta-command` **without
`tta-runner`, native tmax endpoints, or tmux execution**:

```sh
./verification/core/check.sh --report /tmp/opencode/tta-core-report.json
cargo run -p tta-core-witness --no-default-features -- --scenario create
python3 verification/core/poison.py --trace --report /tmp/opencode/tta-pure-poison.json
```

The script checks package-only feature isolation, tests replay, and emits JSON.
No host is contacted; `/target/...` paths in these fixtures need not exist.
`execution_attempted: false` is intentional, not a simulated successful mutation.

The four scenarios cover canonical-ID session entry, supplied canonical project
roots, clean creation with explicit target author/license defaults, and fresh
satellite inventory. Checks inspect semantic command data, literal package JSON,
and the single grouped create → attach → `keep-last` queue. Truncated, extra and
mismatched replies fail; string snapshots alone are not the oracle.
Reports also construct each required versioned probe through an explicit target
shell/cwd setup without a runner. These scripts are emitted, **not executed**.
Project and Create explicitly check both selection and `ChangeDir` against the
supplied physical root. Adversarial tests cover every pending wrong reply kind,
every truncated prefix, wrong endpoint identity, fresh grouped suffixes, rootless
session entry, and explicit generator defaults suppressing default acquisition.

[`poison.py`](poison.py) runs all four scenarios in two disagreeing environments
with failing executable canaries, compares byte-identical reports, and checks the
package-only feature graph. It needs Python **locally for verification only**,
not on a launch target. `--trace` additionally checks target access, network I/O
and child execution when `strace` is available; availability and the actual trace
are recorded, never inferred from a missing tool. Build the witness first via
`check.sh` or `cargo build -p tta-core-witness --no-default-features`.

## Library hooks

[`fixture`](src/lib.rs) returns an `Input` and typed `Vec<Exchange>`;
`replay(input, transcript)` checks each exact request and feeds its reply back
into `Workflow`. `exercise(scenario)` adds semantic checks and a serializable
report. Reports contain readable request/reply representations; those strings
are diagnostics, **not a wire protocol or a JSON transcript-import API**.

The core's `Workflow::next()` yields `Observe(ObservationRequest)` or
`Ready(Box<PreparedLaunch>)`; `supply(ObservationReply)` validates the pending
reply type. Errors from an observer stop at the observer boundary. There is no
"unknown means empty" reply. Local I/O is implemented separately by
[`tta-runner::driver`](../../session/runner/src/driver.rs).

## Informational planning cost probe

```sh
cargo run --release -p tta-core-witness --example replay_cost -- --iterations 10000
cargo run --release -p tta-core-witness --example replay_cost -- --extra-sessions 1000 --extra-projects 1000
```

[`replay_cost`](examples/replay_cost.rs) emits CSV for the four supplied-facts
fixtures. It compares a repeated immutable `next()` on an already supplied
workflow with a complete `replay(input.clone(), transcript)`, including input and
reply cloning and `supply()`'s pending-request checks. Each operation drops its
plan. Fixture construction, warmup and shared-graph compilation are outside the
timed batches; CSV values are median per-operation batch means, not percentile
latencies. `--iterations` and `--samples` must be positive (defaults: 1000 and 5).
Extra inventories contain nonmatching names; they are workload dimensions, not
simulated target discovery. Exact session hits intentionally retain their
short-circuit path.

This executable reads its arguments and a monotonic clock, but neither acquires
target facts nor runs a plan. It has no timing assertions and establishes no
latency budget, platform guarantee, or isolated runtime-versus-domain cost. Use
it to inform a replay/caching decision, not to infer live tmux/SSH performance.

## Scope and independent runtime evidence

This establishes supplied-facts planning, replay and pure probe construction, not
live discovery or a complete deferred shell workflow. The core's separately
driveable `target::Bootstrap` now describes context/registry requests and replies;
`probe::Probe` emits and decodes the framed protocol. Actual transport evidence
lives in the [SSH toolkit](../ssh/README.md). ⇄ The separately maintained
[emitted-probe round-trip](../probes/README.md) executes the public emit/decode/
resume loop through stdlib processes without a runner, including registry and
generator-default acquisition. Its intentional filesystem/process I/O is kept
out of this supplied-facts package; this package's poison and dependency witness
are unchanged. Branches/captures for a deferred
whole-workflow shell compiler remain later work.

Real process/shell parity lives in
[`runner/tests/expressions.rs`](../../session/runner/tests/expressions.rs), and real
tmux grouped/argv/cwd behavior in
[`runner/tests/satellites.rs`](../../session/runner/tests/satellites.rs).
[`cargo verify`](../driver/src/main.rs) retains the existing real-PTY outcome
toolkit. Its non-PTY capture now uses the runner's shared bounded complete-EOF
implementation, and reports exact read-only owned-endpoint preflight evidence.
