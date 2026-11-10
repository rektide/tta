---
type: ImplementationReport
title: EntryAttempt 0.8.0 — runtime deletion and actual caller acceptance
description: Coordinated removal of the replay and sidecar APIs, migrated semantic and runner-free witnesses, exact 0.8.0 pins and receiving lock, and authenticated worker-free SSH plus local/SSH consumer retry evidence
resource: /design/entry/cutover1.gpt61sol.md
status: implemented
author: model:openai/gpt-6.1-sol#high
date: 2026-11-10
sources:
  - resource: opencode://ses_e3d19979bffe2Eu6JnXnb11i7H
    title: Finish G's partial cutover with runtime deletion, migrated witnesses and verification drivers, coordinated 0.8.0 consumer adoption, and actual local/SSH terminal retries
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commissions this fresh Sol continuation under the human-approved redesign and retains architectural review
  - resource: /design/entry/redesign0.gpt6a.md
    title: Approved destination and stage-four acceptance, not the earlier preservation recommendation
  - resource: /design/entry/cutover0.glm53m.md
    title: G's implemented-partial checkpoint and exact remaining acceptance followed here
  - resource: /design/entry/kernel0.gpt61sol.md
    title: Coordinated-cutover deletion obligations
  - resource: /design/entry/hosts0.gpt61sol.md
    title: Shared context, observation and actual-consumer handoff obligations
  - resource: /session/launcher/src/entry/mod.rs
    title: Sole retained entry owner after removal of its predecessor
  - resource: /session/launcher/src/entry/action.rs
    title: Semantic plans and bound actions without portable/native sidecars
  - resource: /session/launcher/src/validation.rs
    title: Relocated name and discovery errors, with no resolver scheduler
  - resource: /session/launcher/src/probe.rs
    title: Retained framing/classification mechanics after deleting Setup/Workflow envelopes
  - resource: /session/runner/src/observe/filesystem.rs
    title: Retained local filesystem mechanics, not another entry loop
  - resource: /session/runner/src/execute/mod.rs
    title: Thin interpreter and the single shared semantic trace-label contract
  - resource: /session/runner/tests/entry_execute.rs
    title: Effect ordering, generator failure, native uncertainty and no-fallback witnesses
  - resource: /app/cli/tests/launch.rs
    title: Real native acknowledged creation followed by failed handoff, and a native creation race after binding
  - resource: /verification/driver/src/ssh.rs
    title: Ported authenticated SSH matrix, actual consumer PTYs, target-relative overrides and fresh retry witnesses
  - resource: /verification/core/src/lib.rs
    title: New runner-free supplied-facts actor scenarios and protocol rejection witnesses
  - resource: /verification/probes/src/lib.rs
    title: New runner-free emitted-probe loop including registry, defaults and fresh binding
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Actual embedded callers, target-relative SSH paths, effect knowledge and unchanged worker ownership
  - resource: file:///home/rektide/src/term-preserver/src/completion.rs
    title: Remaining embedded completion calls moved onto CatalogQuery; non-library implementation retained
  - resource: file:///home/rektide/src/term-preserver/src/launch.rs
    title: Reproduced shared-environment SSH fixture races isolated without changing transport/master policy
  - resource: /Cargo.toml
    title: Coordinated workspace 0.8.0 and exact internal pins; tmax remains separately versioned 0.7.0
  - resource: file:///home/rektide/src/term-preserver/Cargo.lock
    title: Actual receiving lock on the 0.8.0 packages
---

# Outcome and standing

**The outstanding production cutover is implemented and verified.** The public
surface is now 0.8.0 with one retained `EntryAttempt`, separate `CatalogQuery`,
and local/SSH read and effect interpreters. No old entry runtime or public
compatibility facade remains. CLI and term-preserver call those owners directly.
This completes the remaining list in [G's partial checkpoint](cutover0.glm53m.md#exact-remaining-acceptance-ticket-stays-open),
not merely a successful test run while the old runtime remains present.

Work anchor: `tta-entry-attempt-cutover`. The separately commissioned
`tta-entry-attempt-consolidate` and epic remain open: **this report does not
claim promotion or architectural acceptance of the maintained documentation**.
No nested agents, installation, push, kubeless inquiry, daemon, cleanup policy,
automatic retry, backend fallback or compatibility machinery was introduced.
Mixed `.beads/issues.jsonl` changes remain excluded from commits.

## Source and receiving checkpoints

| Repository / commit | Coherent landing |
| --- | --- |
| tta `8118168c` | Delete superseded runtime/public APIs and old probe envelopes; relocate reusable mechanics; migrate semantic tests and both runner-free verification crates; port the actual SSH driver; centralize trace classification; coordinate 0.8.0 and exact internal pins |
| term-preserver `c771ad8b` | Exact 0.8.0 pins and receiving lock; missed embedded completion callers on the catalog; target-relative SSH overrides; honest effect-stop reporting |
| term-preserver `d6eece86` | Isolate the four concurrent SSH environment fixtures; correct unavailable-backend assertions in minimal builds |

The receiving lock contains all five tta library/adapter packages at **0.8.0**.
The consumer's own public version remains 0.2.0; this is internal adoption, not
a new consumer CLI. `tmax` stays at its independently owned exact 0.7.0 pin.
Source comments/help, verification recipes and this receipt form the follow-up
acceptance checkpoint; they are not a second API version change.

## Actual deletion delta

| Removed competing owner | Retained mechanics / sole replacement |
| --- | --- |
| Launcher `workflow.rs`, `workflow/` actor/JSON/tests | `entry/entry.machine.json` and the retained actor; no `next/supply` reconstruction, replay fact slots, clone/Sync or eight-action-free-state requirements |
| Launcher `resolve.rs`, `Resolution`, `Selection`, public `resolve`/`resolve_unique` | `validation.rs` for domain errors/name validation; pure workspace/project ranking remains; entry chart owns selection and read scheduling |
| Scaffold `resolve`, `resolve_with_config`, `Progress`, `Resolved`, `defaults_required` | Registry parsing/choice and validated adapter requirements/materialization; entry preparation owns generator selection and actual default requests |
| Launcher `plan.rs`, `prepared.rs`, `LaunchPlan`, `NativeCreation`, `ClientMode`/`WindowName` legacy surfaces | Semantic `EntryDecision`, `BoundAction` and native protocol values; literal tmux/shell mechanics remain in action lowering |
| `target::Bootstrap`, `Options`, old `Facts::input` and request/reply/progress vocabulary | Entry context's raw facts and active-layer policy, bounded registry acquisition task, pure `TargetSetup` shell carrier and classified filesystem entry facts |
| Old probe `Setup`/`Workflow` envelopes and `Probe` | Shared framing/strict session/errno mechanics in `probe.rs`; request-bound codecs in `entry/probe.rs` |
| Runner `driver.rs`, resolver/scaffold loops and config/default compatibility builders | Shared `observe::drive_reads`; `observe::filesystem` only enumerates or lstats; registry/default observation stays in the new interpreters |
| Runner `backend.rs`, `execution.rs`, `ExecuteLaunch`, native preflight and fallback-shaped sidecars | `observe::local::SessionBackend`; actor-owned eligibility and effect dispatch; executor interprets exactly one requested effect |
| Runner `ssh::prepare`, `PreparedRemote`, bootstrap/workflow loops, duplicate diagnostics | `ssh::Ssh` only constructs literal transport argv; `observe::ssh` freezes connection/setup and retains records; `execute::ssh` drives the same actor |
| Runner `socket.rs` and core's old `client_mode*` helpers | Shared context policy requests endpoint identity before authorizing a switch; socket parsing/label startup mechanics remain pure |
| CLI duplicate semantic-operation matrix and post-return trace reconstruction | Runner's one label contract, emitted before dispatch; SSH stops report boundary/effect knowledge rather than universally claiming no mutation |
| `verification/core/examples/replay_cost.rs` | The old immutable-pull benchmark has no destination role; owned-actor inspection/progression witnesses remain |

An explicit Rust-source reference scan found **no** `workflow::`,
`NeedProjects`, `NeedDestination`, `PreparedRemote`, `LaunchPlan`,
`NativeCreation`, `prepare_launch`, `ExecuteLaunch`, `target::Options`,
`target::Bootstrap` or `runner::prepare` references after cutover. Historical
Markdown is not a production caller and is not counted as one.

## Semantic coverage migrated, not silently dropped

The test count is lower because old planner/replay implementations and their
duplicate tests are gone. The following witnesses still exercise the outcomes:

| Former test/API dependency | Current meaningful witness |
| --- | --- |
| Runner `resolution.rs` | Real local acquisition into semantic plans/stops: session-first/rootless/shortest/ambiguity, immediate/literal project rules, safe overrides/collisions, clean creation/unsafe/occupied paths and lstat errors |
| Runner `probes.rs` | Six new-request codec cases retain startup channels, hostile names, physical aliases, dangling collision, malformed/truncated/wrong-nonce/nonzero/incomplete capture, exact endpoint absence, symlink ENOTDIR/loop and denied-parent distinctions |
| Both project-root suites | Pure domain unit tests plus retained-actor discovery tests and actual local route tests retain global ranking, qualification, complete required roots, physical aliases/provenance, malformed facts and overlap; `entry_hosts` keeps local/probe agreement |
| `trace_contract` / `trace_messages` | One semantic resolution/plan/emission sequence; structured ambiguity retains candidates and has no plan/effect, rather than requiring the retired resolver's log text |
| `satellites` / `private_tmux` | Real explicit-socket tmux witnesses keep independent window selection, same-client switching limitation, keep-last, hostile formats, raw argv, singleton executable, permissions/protocol errors, creation races and label-carrier behavior |
| Scaffold prefix tests | Actor registry/choice/destination stops plus `entry_execute` ordering/failure/uncertainty, CLI/consumer scaffold tests and actual SSH generator failure; no second prefix-plan builder |
| Launcher entry options / CLI native section | New-work-only options, raw bytes and rendering boundary, relative-path preparation rejection, real acknowledged native creation surviving failed handoff, and native race failure issuing no terminal fallback |
| Runner-free verification packages | Four supplied-facts scenarios through actual fresh binding/export; protocol wrong-shape/stale/duplicate rejection and inert inspection; emitted-probe registry walk through symlink/interior `..`, actual defaults, native decode causes and observation-only export without a runner dependency |

The CLI's local direct/shell, native/CLI and inside/outside acceptance remains
**64 passed** in the final workspace run. Session/project alias, raw-argv,
format escaping, selected-target binding, root identity and whole-active-layer
precedence witnesses remain in the retained actor/host suites.

## Actual SSH and actual consumer evidence

`verification/driver/src/ssh.rs` no longer calls the deleted preparation API.
Its library planning cases acquire an attempt and call `execute::ssh::drive`
through **observation-only export and fresh binding**, then the harness explicitly
evaluates that frozen artifact in a real PTY. That is not misreported as an
effect receipt. Direct CLI and consumer cases drive machine-owned terminal
effects/exec. Reports retain independent observations and loss/cleanup evidence.

Final authenticated run: **19 checks, 204 retained probe records, 13 terminal
runs, success=true**. Twelve terminal runs exited 0; one deliberately killed
SSH carrier exited 255 after its target mutation was independently observed.
Exactly **16** logged `new-session` invocations match the authorized attempts.
The exclusively owned endpoint's initial absence, final absence and sshd
termination are verified; no default or user server is contacted.

Coverage includes target-local context/defaults versus contradictory local values,
literal startup argv and relative cwd, multi-root ambiguity/qualification and
creation, prefix environment/override, coherent grouped lifecycle, actual Git/jj
scaffolding, generator failure once with no tmux/handoff, malformed/failed/ENOTDIR
reads stopping before mutation, evaluable local SSH emission, direct CLI SSH and
transport loss without replay/rollback. Target `tta` absence is proven after
selected-shell setup and a restricted PATH.

All three real consumer cases use term-preserver's own proxy, worker and PTYs:
local native observer, local CLI observer, and embedded SSH. Each creates work,
detaches, accepts an explicit user retry, observes the newly selected independent
view sharing the primary window with `keep-last`, detaches and quits. Reusing
the first StartProject action would fail or recreate rather than yield that view.
SSH also passes explicit **relative target root and directory** under a different
local cwd. The cutover port had incorrectly joined those overrides against local
cwd; they now remain native relative values until shared target policy freezes
them. Explicit grouping stays Shared/Independent, never Auto.

Managed ControlMaster expiry/recovery, interactive authentication, Fish/Nu
execution, network/OS diversity, strong tmux-server incarnation and stronger
inside-client identity are **not** claimed. Managed master is deliberately
disabled for the owned SSH fixture; its construction/policy remains consumer-owned.

## Final checks and failures reconciled

| Check | Exact result |
| --- | --- |
| Workspace nextest, all features | **239 passed, 0 failed, 0 skipped**: CLI 64, launcher 101, runner 58, core witness 2, probe witness 1, driver 4, adapters 9 |
| Launcher + runner, no default features | **122 passed, 0 failed, 0 skipped** |
| Launcher + runner, CLI + external rekon | **150 passed, 0 failed, 0 skipped** |
| Launcher + runner, native + native adapter | **136 passed, 0 failed, 0 skipped** |
| Consumer nextest, all features | **245 passed, 0 failed, 0 skipped** |
| Consumer nextest, no defaults + tta-lib only | **220 passed, 0 failed, 0 skipped** |
| Consumer nextest, no tta-lib; CLI/classic-CLI/tmux/regex | **241 passed, 0 failed, 0 skipped**, including independent completion and Zsh widget |
| Consumer ordinary parallel `cargo test --features tta-lib --lib` | **180 passed, 0 failed**, after fixing fixture isolation |
| Affected six tta packages, all-target/all-feature clippy, no deps, `-D warnings` | Passed |
| Launcher/runner all-target no-default clippy, no deps, `-D warnings` | Passed |
| Affected-package formatting; entry/catalog graph freshness | Passed |
| Python structural renderer | **5 passed** |
| Package-only runner-free core and probe scripts | Passed; dependency graphs exclude runner/native execution features |
| Runner-free poisoned-environment/syscall witness | **4 scenarios in 2 contexts**, identical outputs, no target file/network/child execution; strace available and used |
| Workspace Rustdoc | **2 passed, 0 failed, 0 ignored** |
| Owned Markdown/OKF check | Passed: README, entry verification guide and this receipt, including native session provenance |

G's three consumer failures were **real test-isolation defects**, not three SSH
failures to waive. Ordinary parallel cargo test reproduced 177 pass / 3 fail:
`ssh_user_mux_absent_without_path_or_master`,
`ssh_control_options_creates_managed_directory_and_options`, and
`ssh_control_options_tags_along_on_user_mux`. Four SSH fixtures concurrently
overwrote process `PATH`/`XDG_RUNTIME_DIR` without the existing environment-test
lock. All 180 passed serially and all those tests passed in isolated nextest
processes. Adding the existing shared serial annotation to those four writers
restored the ordinary parallel 180/180 run without changing production master
or transport policy. A minimal-feature test also incorrectly assumed an
uncompiled classic backend could be selected; its assertion now respects that
explicit capability boundary.

An initial broad workspace clippy run additionally found **two unchanged
adapter unit assertions on compile-time constants**, outside the edited runtime
and witness packages. They remain a separately identified lint limitation, not
failed cutover behavior; affected-package clippy passes. Dependency-owned tmax
warnings remain unchanged. Early migrated-fixture mistakes (format-value trace
matching, raw-byte export versus native execution shape, and obsolete stop-text
expectations) were corrected rather than weakened into successful skips.

Reproduction commands are in [the verification guide](/verification/entry/README.md).
Raw logs/JSON live in ignored `.test-agent/entry-cutover/`, notably
`final-workspace.log`, `ssh-final.json`, `consumer-final-all.log`,
`consumer-lib-only.log`, `consumer-no-lib.log` and `consumer-parallel-fixed.log`.
These paths retain this checkout's evidence, not portable fixture credentials;
the private daemon/key directories were removed by their owners.

## Consolidation handoff — still separate work

1. Review this deletion/API landing against the approved destination. The actor,
   not a replacement wrapper, owns lifecycle and fresh binding. Acceptance of
   that architecture remains with the parent/human, not this report's timestamp.
2. Replace the README's explicitly marked historical 0.7 Rust/API narrative,
   `doc/entry/README.md`, its old state/algorithm account, and consumer accounts
   with the complete destination account and actual source/API examples. Preserve
   independent design reports as historical evidence; repair deleted-path source
   references deliberately, preferably binding exact historical claims to their
   recorded revisions rather than pretending old reports described new code.
3. Update `verification/core/README.md`, `verification/probes/README.md` and the
   SSH guide to the new package APIs and 19-check consumer matrix. The entry guide
   is current; generated entry/catalog diagrams already match their executable
   sources, and the separate guarded-resolution explanation still needs promotion.
4. Carry this session's consumer completion adoption and SSH path correction into
   both sides of the consumer account. Its standalone non-library completion and
   transport/master ownership are retained, not replaced by mandatory embedding.
5. Review remaining preservation-oriented prose/examples and source citations,
   not resurrect deleted public APIs to make them compile. No production cleanup
   loop, reconciliation or compatibility obligations are left by in-memory types.
6. Keep the epic/consolidation open until their docs/architecture review is done.
   Kubeless research still waits for that whole-rewrite milestone and the parent's
   explicitly commissioned independent wave; none was dispatched here.
