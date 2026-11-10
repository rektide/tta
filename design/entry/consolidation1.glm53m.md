---
type: ImplementationReport
title: EntryAttempt consolidation — accepted architecture published at 0.8.0
description: Final consistency review of the approved rewrite's deletion and delivery; promotion of the maintained architecture accounts, generated graphs, guides, consumer accounts and verification toolkits onto the actual 0.8 interfaces, with the evidence index, current limitations and exact usable APIs
resource: /design/entry/consolidation1.glm53m.md
status: implemented
author: model:zai-coding-plan/glm-5.3#max
date: 2026-11-10
sources:
  - resource: opencode://ses_e3cf482d5ffeyD5hefdVDZGmbe
    title: Claim and complete the consolidation - architecture review, documentation promotion, lint resolution, bidirectional consumer accounts and this report
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent retains architectural review, commissions this consolidation and owns the epic close and any later kubeless wave
  - resource: /design/entry/redesign0.gpt6a.md
    title: Human-approved destination whose stage-five acceptance this consolidation completes
  - resource: /design/entry/cutover1.gpt61sol.md
    title: Deletion/API landing receipt trusted here rather than rerun
  - resource: /doc/entry/README.md
    title: Promoted maintained entry architecture account with guarded-resolution prose
  - resource: /doc/entry/attempt-graph.md
    title: Generated structural entry chart promoted to production standing
  - resource: /README.md
    title: Guide with reusable 0.8 API examples and matching diagram
  - resource: /doc/consumer/term-preserver.md
    title: Consumer account rewritten on the actual EntryAttempt call sites
  - resource: file:///home/rektide/src/term-preserver/doc/launch/README.md
    title: Consumer-side launch guide adopted to the same wording
  - resource: /verification/core/README.md
    title: Runner-free witness account on fresh attempts and protocol rejection
  - resource: /verification/probes/README.md
    title: Emitted-probe round-trip account including registry acquisition and fresh binding
  - resource: /verification/ssh/README.md
    title: Actual-SSH guide updated to observation-only export evaluation and the three-case consumer matrix
---

# Outcome and standing

**The consolidation is complete.** The approved destination
([redesign0](redesign0.gpt6a.md), shape B) is the published, documented
architecture at 0.8.0: the deletion receipt
([cutover1](cutover1.gpt61sol.md)) is trusted rather than rerun, and this pass
reviewed it against the actual sources, promoted every maintained account onto
the delivered interfaces, and corrected the small direct gaps it found. The
`tta-entry-attempt-consolidate` work item is closed by this report; the epic
`tta-entry-attempt` remains for the parent's review. No kubeless inquiry,
nested agent, push, install, daemon, cleanup policy, automatic retry, backend
fallback or compatibility machinery was introduced.

## Accepted architecture (as verified against code)

Each approved destination property, with its actual owner:

| Approved property | Delivered owner |
| --- | --- |
| One orchestration owner; the machine persists for the attempt | `session/launcher/src/entry/entry.machine.json` + `entry/machine.rs`; `EntryAttempt` (`entry/mod.rs`) advances once per accepted event, one correlated pending request, `waiting()`/`snapshot()` inert |
| Native Rust errors/data retained | `entry/protocol.rs` keeps paths, argv, inventories and typed `EntryError` chains outside the runtime's serde context; only discriminants, counters and validated summaries cross the chart |
| Shared pure policy; whole-layer context selection before decoding | `entry/context.rs` (`Scope`/`Origin`/`Facts`/`Overrides`/`Acquisition`); adapters only collect values |
| Distinct work/view/project values; semantic decisions | `entry/select.rs` (`WorkspaceMatch`, `MatchReason`, `WorkspaceKey`) and `entry/action.rs` (`EntryDecision`: EnterView/OpenView/StartProject/ScaffoldAndStart with `DecisionBasis` provenance) |
| Fresh binding of the chosen action; no reranking | `entry/bind.rs` — fresh inventory plus route/destination evidence, fail-closed anchor change, satellite allocation at binding, pure `ExecutionShape` chosen before mutation |
| Typed catalog/read/effect protocol | `entry/catalog.rs` (read-only `CatalogQuery`, mutation-incapable chart), `protocol.rs` request/event/effect vocabulary, `runner/observe/*` read interpreters, `runner/execute/*` thin drivers interpreting exactly the dispatched effect |
| Dispatch/partial/delegation outcomes honest | `AttemptOutcome::{Exported, Completed, Delegated, Stopped, Cancelled}` with `EffectKnowledge`; `execute/mod.rs` owns the single semantic trace-label contract, emitted before dispatch |
| Replaceable hosts, retained actor ownership | CLI (`app/cli/src/main.rs`), term-preserver's two call sites and the verification driver all drive the same protocol; SSH changes the interpreters, not the policy |

## Deletion review and residual scans

The cutover's explicit reference scan (no `workflow::`, `NeedProjects`,
`NeedDestination`, `PreparedRemote`, `LaunchPlan`, `NativeCreation`,
`prepare_launch`, `ExecuteLaunch`, `target::Options`, `target::Bootstrap`,
`runner::prepare` in Rust sources) was re-verified by fresh scans here; the
only remaining mentions in the repository are deliberate historical labels in
`design/` evidence, `verification/reviews/`, and the version-history notes in
the maintained README/entry contract that name the removed surface as removed.
Findings corrected in this pass:

- Two unchanged adapter constant-assertion clippy warnings
  (`assertions_on_constants` in `scaffold/rekon`/`scaffold/rekon-native`)
  resolved narrowly as compile-time `const { assert!(..) }` blocks — the
  declared destination-contract constants are now checked at compile time
  instead of being test-time lints. Adapter tests (9) and affected-package
  clippy/fmt pass; the broad-workspace warnings that remain are the unchanged
  dependency-owned tmax lints.
- One pre-existing formatting drift in `scaffold/rekon-native` tests (a long
  `assert_eq!` the cutover's affected-package pass missed) reformatted.
- Empty leftover directories `session/launcher/src/workflow/` and
  `verification/core/examples/` (untracked filesystem residue of deleted
  files) removed; no source referenced them.
- No stale internal feature gates (`entry-internal` is gone from manifests and
  code), no second scheduling representation (`entry.machine.json` is the sole
  entry chart; `catalog.machine.json` is its read-only sibling by static
  composition), and no duplicate execution representation (one `BoundAction`,
  one `ExecutionShape` choice, one driver per host).
- User-outcome witnesses, not old shape tests: the retired eight-state/
  immutable-pull/clone-share requirements were already migrated by the
  cutover (its semantic-coverage table); no old-shape test was reintroduced
  and no meaningful outcome witness was dropped.

## Documentation promotion

Maintained accounts now describe the delivered 0.8.0 surface; independent
design reports and reviews are preserved as historical evidence, with
deleted-path historical claims bound to their recorded revisions rather than
rewritten:

| Document | Delta |
| --- | --- |
| `doc/entry/README.md` | Full promotion: owned-attempt protocol, chart, fresh binding, effect lifecycle, promoted guarded-resolution prose bound to `entry::select::resolve_session`, catalog, verification obligations. The 0.7 replay contract stays readable at revision `0349b34e`. |
| `doc/entry/attempt-graph.md`, `doc/entry/catalog-graph.md` | `implemented-internal` → `implemented`; prose promotes the generated charts from staged replacement to production architecture; graphs regenerate-check clean against their executable JSON. |
| `README.md` | Retired 0.7 API examples replaced with actual `EntryAttempt`/`CatalogQuery`/`observe`/`execute` usage (acquisition, driving, export, SSH, catalog), matching diagram retargeted at `entry::select`, crate table and capability map updated, deletion recorded as version history. |
| `doc/entry/project-roots.md` | Progression section rebound to the chart's `project_scope`/`projects_read` reentry/`projects_rank` states instead of the retired eight-stage resolver route; root semantics unchanged. |
| `doc/consumer/term-preserver.md` | Rewritten on the actual call sites: `EntryAttempt` with explicit grouping (`Independent`/`Shared`, never Auto), local native/CLI observers, frozen SSH observer + one terminal effect, `Stopped` reporting with effect knowledge, `CatalogQuery` completion with the retained non-library implementation, target-relative overrides, cutover commits and the three real consumer retry cases. |
| term-preserver `doc/launch/README.md` | Consumer side of the same account: 0.8.0 wording, `EntryAttempt`/interpreter description, deleted `plan.rs` citation repaired to `entry/action.rs`, diagnostic/effect language rebound. |
| `verification/core/README.md` | Fresh-attempt scenarios through actual fresh binding/export, protocol-rejection witnesses, removed replay-cost probe bound to its historical record, hooks section on the public protocol. |
| `verification/probes/README.md` | Attempt-owned registry acquisition (symlink/interior `..`), one-shell-per-probe, actual defaults, observation-only export, decode-cause retention. |
| `verification/ssh/README.md` | Library planning described as observation-only export + explicit PTY evaluation; three-case consumer matrix (local native, local CLI, embedded SSH); final run numbers (19 checks, 204 records, 13 terminals, 16 creations, one deliberate loss). |
| `design/entry/README.md` | Index reworked around the delivered architecture with the cutover receipt and maintained-account section. |

All promoted documents pass the owned Markdown/OKF check (structure,
destinations, anchors, provenance including native session sources); the
former failing deleted-path citations in `doc/entry/README.md` and
`verification/core/README.md` are repaired. Both graph freshness checks pass.

## Evidence index

- Final workspace matrix, feature subsets, consumer suites, parallel lib run,
  clippy/fmt/graph/docs, poisoned-environment/syscall witness, runner-free
  scripts, rustdoc, and the authenticated SSH + actual consumer retries:
  [cutover1](cutover1.gpt61sol.md) (commit `8118168c` tta, `c771ad8b` +
  `d6eece86` term-preserver; raw logs in ignored `.test-agent/entry-cutover/`).
- Reproduction recipes: [entry verification guide](/verification/entry/README.md),
  [SSH toolkit](/verification/ssh/README.md), [core](/verification/core/README.md)
  and [probes](/verification/probes/README.md) witnesses.
- Staged delivery history: [kernel](kernel0.gpt61sol.md),
  [discovery](discovery0.glm53m.md), [hosts](hosts0.gpt61sol.md),
  [partial cutover](cutover0.glm53m.md); engine/architecture history:
  [index](README.md).
- This pass's targeted checks: adapter tests/clippy/fmt, graph `--check` (both),
  rkt `check --okf` on every touched document, fresh deletion/gate/duplicate
  scans. No code beyond the two adapter assertions, the missed reflow and the
  empty directories changed, so the cutover's full matrix was not rerun.

## Current limitations (not new obligations)

- Managed ControlMaster expiry/recovery, interactive password/passphrase
  authentication, network/OS diversity beyond this Linux loopback target, and
  strong tmux-server incarnation identity are **not** verified; the owned SSH
  fixture deliberately disables the managed master.
- Fish/Nu selected-shell carrier **construction** is covered; their execution
  is not certified by the installed-shell fixtures.
- Inside-client origin remains heuristic; guarded inside switching and
  input-origin research stay open under `tta-launch-backends`.
- Mosh remains on its remote CLI carrier; a distinct observation channel is a
  declared future choice. Deferred whole-workflow shell compilation is not
  implemented; `--print-shell`/export is a frozen snapshot.
- One outstanding read/effect at a time is the deliberate first-implementation
  bound; batching is a later optimization within the protocol.
- The consumer adoption is internal (exact 0.8.0 pins, its own version 0.2.0);
  no new consumer multi-root UI, and its non-library completion remains a
  real feature obligation, not a debt.
- Verification evidence lives in this checkout's ignored scratch paths and is
  not portable fixture credentials.

## Exact usable APIs (0.8.0)

- `tta_launcher::entry::{EntryAttempt, catalog}` — `EntryAttempt::{acquire,
  start, start_at}`; `waiting() -> Waiting<{Read, Planned, Effect, InFlight,
  Finished}>`; `advance(Event::{Observed, Deliver, EffectStarted,
  EffectFinished, DelegateTerminal, Cancel})`; `snapshot()`.
- `tta_launcher::entry::{context, protocol}` — `Acquisition`/`Scope`/`Origin`
  (Local/Ssh with `TargetSetup`)/`Overrides`/`EntryIntent` (+`selector`
  constructor), `ReadRequest`/`ReadCompletion`/`Observation`, `Delivery`,
  `EffectRequest`/`EffectKind`/`EffectReceipt`/`EffectKnowledge`,
  `AttemptOutcome`, `ProtocolError`, `EntryError`.
- `tta_launcher::entry::{action, select, probe}` — `EntryDecision`,
  `EntryPlan`, `BoundAction`/`BoundOperation`/`ExecutionShape`,
  `ExportedAction`; `WorkspaceMatch`/`ViewRef`/`WorkspaceKey`/`MatchReason`;
  request-bound `Probe::{new, script, decode}`.
- `tta_launcher::entry::catalog::{CatalogQuery, Input, Report, Candidate,
  Outcome}`; `tta_launcher::{scaffold, socket, target, validation}`.
- `tta_runner::observe::{drive_reads, local::{Local, SessionBackend},
  ssh::{Ssh, Record, write_diagnostics}}`; `tta_runner::execute::{drive, run,
  operation_label, local::Local, ssh::drive}`; `tta_runner::{ssh, capture}`.

## Declaration

Within the scope actually verified and recorded — local Linux real terminals,
authenticated loopback SSH without a target tta, the three real consumer
cases, and the runner-free/poison witnesses — **the whole approved
destination is code-complete and documented**: old orchestration deleted with
no facade, generated graphs conforming to the executable chart, outcome
witnesses retained, maintained guides/consumer accounts on the actual 0.8
interfaces. This declaration does not extend to platform diversity, Fish/Nu
execution, managed-master recovery or the separately queued kubeless inquiry,
which remains the parent's commission after the epic review.
