---
type: ImplementationCheckpoint
title: Ordered project roots — establishing core and integration handoff
description: Declared policy, implemented pure multi-root workflow and physical provenance, observed local/probe agreement, actual consumer port, and bounded remaining CLI/completion/SSH acceptance
resource: /design/entry/project-roots0.gpt61sol.md
status: implemented-partial
author: model:openai/gpt-6.1-sol#high
date: 2026-11-09
sources:
  - resource: opencode://ses_e3e3170d4ffeij7U00lUlwaZg7
    title: Declare and implement the establishing/core slice of commissioned tta-project-roots, verify observations and hand integration back to the parent for review
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commissions rotating Sol/G implementation after accepting the capability-ticket pitch
  - resource: /doc/entry/project-roots.md
    title: Pre-code roots/configuration/identity/selectors/destination contract
  - resource: /session/launcher/src/projects.rs
    title: Pure configuration layers, selector scope, inventory binding, physical matching and completion helper
  - resource: /session/launcher/src/resolve.rs
    title: Session guard, required-root iteration, actionable ambiguity and safe destination selection
  - resource: /session/launcher/src/workflow.rs
    title: Native inventory transcript replay through the unchanged eight-stage graph
  - resource: /session/launcher/src/target.rs
    title: Target-only plural environment and explicit root-list lowering
  - resource: /session/launcher/src/probe.rs
    title: Worker-free physical-directory acquisition and fail-closed decoding
  - resource: /session/runner/tests/project_roots.rs
    title: Actual local/probe identity agreement, multi-root runner use and no-mutation checks
  - resource: /session/launcher/tests/project_roots.rs
    title: Pure configuration, matching, provenance, selector and destination tests
  - resource: /doc/consumer/term-preserver.md
    title: Actual receiving API/lock adoption and intentionally pending consumer surface work
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Reviewed local and SSH caller policy and minimal coherent 0.7.0 port
---

# Outcome and standing

**The establishing/core slice is implemented and committed. The ticket remains
open for G's direct integration turn and final acceptance.** The declaration was
committed **before code** as `3247ce5e`; core/API implementation is `89c70249`.
This is not a claim that repeated CLI roots, local `TTA_ROOTS`, Zsh completion or
actual multi-root SSH end-to-end acceptance are complete.
The actual term-preserver API/receiving-lock checkpoint is commit `71d13db4`
in that repository; its two calls preserve explicit caller policy.

## Decisions now embodied in core

- Ordered roots are not first-match priority. Use the existing shortest literal
  basename rule globally, then collapse tied winners by observed canonical
  physical directory. Preserve all supplied routes to the winner. Different
  physical directories remain ambiguous, with actionable `@N/name` selectors.
- `@N/fragment` is one-based configured-slot qualification. Session resolution
  still sees the original literal input first. A qualified project miss observes
  only its selected root; an unqualified miss needs all roots before selection.
  There is no arbitrary absolute-path/traversal grammar.
- A clean scaffold miss must have an explicit qualified slot or only one physical
  root across the supplied inventories. Root order cannot pick a destination.
  Name/collision guards precede the exact absence request. Present/dangling paths
  and failed inspection never authorize creation.
- The declared layer order is repeated CLI roots > JSON-array `TTA_ROOTS` >
  literal `TTA_ROOT` > HOME-relative `PROJECTS_DIR` > HOME/src. One active list
  replaces lower layers. Target lowering implements this now using only target
  facts; the pure helper is available for the next local CLI integration.
- No runtime/graph redesign: eight states and all transitions remain unchanged.
  Projects facts now replay one required root per projects-stage visit. Core
  retains native paths/argv/errors, immutable pulls and no observation I/O.

## Interfaces and boundaries

| Interface | Implemented shape |
| --- | --- |
| `workflow::Input` | `roots: Vec<PathBuf>`; empty supports rootless session hits; supplied roots must be absolute target-local paths |
| `target::Options` | `roots: Option<Vec<PathBuf>>`; explicit list replaces target defaults; `TTA_ROOTS` added to observed variables |
| Core/scaffold/local-runner resolver builders and `tta_runner::prepare` | `.roots(&[PathBuf])`, default empty; no singular compatibility shim |
| `ProjectInventory` | `requested_root`, physical `root`, `entries: Vec<ProjectEntry>` |
| `ProjectEntry` | Entry `name` and observed canonical physical `directory` |
| `Selection::Project` / `Create` | Existing directory/name fields plus `sources: Vec<ProjectSource>` |
| `ProjectSource` | One-based `root_index`, requested root, physical root, basename and physical directory; `selector()` and `entry_path()` |
| `projects::configured_roots` | Pure whole-layer list precedence and target-relative freezing; no environment reads |
| `projects::completion_candidates` | Validated complete inventories → qualified selectors plus physically unambiguous bare names; not yet wired into CLI/Zsh |
| `ObservationRequest::Projects` | Existing single-root request and basename filter, unchanged shape |

Inventory request binding is checked both by workflow supply and native resolver
validation. Empty filtered inventories still identify their physical root for
safe creation. The local observer canonicalizes matching entries. The emitted
GNU-find probe adds a NUL-delimited physical-path field and the decoder requires
it for each matching directory; a failed/racing `cd -P`/`pwd -P` cannot erase a
candidate. Canonical identity is path equality, not inferred inode/bind-mount
equivalence. Acquisition is still observation, not an atomic snapshot or lock.

The existing filter policy is preserved: errors on matching names stop; unrelated
broken links need not poison a launch-specific scan. Completion uses an empty
filter to obtain the full inventory. Physical provenance does not claim discovery
of aliases outside supplied matching facts.

All workspace packages and exact internal pins are coordinated at **0.7.0**.
`cargo bump` 1.1.0 was attempted but rejects virtual workspaces; the coordinated
workspace version/pins were therefore updated together and Cargo refreshed the
lock. Minimal current CLI call sites pass a one-element list, keeping the build
coherent without silently adding a second policy. No install or push.

## Verification actually performed

Existing workspace target output was reused. Read-only logs are in ignored
`.test-agent/project-roots/`; no duplicate heavy target tree was created.

| Command / check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --all-features --offline -j 2` | Passed |
| `cargo nextest run --offline -p tta-launcher -p tta-core-witness --all-features -j 2` | **61/61 passed**, including 54 launcher tests and 7 runner-free witness tests |
| Runner resolution/probes/project_roots/scaffold suites, all features, excluding `generator_prefix_runs_before_creation` | **27/27 passed**, one deliberately skipped environmental fixture |
| `cargo clippy --workspace --all-targets --all-features --offline -j 2 --no-deps -- -D warnings` | Passed; existing dependency-owned tmax warnings remain |
| `cargo check -p tta-launcher --no-default-features --all-targets --offline -j 2` | Passed |
| Actual term-preserver `cargo check --all-targets --all-features --offline -j 2` | Passed; receiving lock updated |
| Actual term-preserver `launcher::` tests, all features | **8 passed, 1 environment failure** before exercising changed code |
| tta `cargo fmt --all --check`; receiving `rustfmt --edition 2021 --check src/launcher.rs` | Passed; consumer-wide formatting still reports preexisting drift in unrelated files, not repaired here |
| `rkt check --okf` on six owned knowledge documents | Passed: **96 links, 80 sources** |

The new executed-probe comparison covers real symlinked roots, child symlinks,
hostile literal/newline names, dangling paths, physical identity and missing-root
failure. Other tests cover global shortest matching, duplicate-name ambiguity,
qualified scope, repeated projects-stage replay, root-bound replies, physical
alias collapse/provenance, overlapping-root nonrecursion and safe creation with
no mutation. Machine/diagram exact-route and guard-order tests still pass.

**Environment limitation:** `/tmp/opencode` is mode 0700 owned by `zimtest2`, not
the current user. The existing runner-free probe round-trip test and receiving
consumer remote-classic fixture fail creating their scratch directories there.
The runner scaffold command-execution fixture was deliberately skipped for the
same hardcoded path. These are not successful checks, and no permissions or other
user's files were changed to bypass them. Receipts identify the exact failures.
The successful new probe tests use their own tempfile roots and do exercise the
changed protocol. Actual SSH/tmux terminal tests are left to final integration.

## Parent review and next G turn

1. **Wire CLI/environment once.** Make `Args.root` repeated ordered paths, freeze
   them before runner preparation, and use the pure configuration helper for the
   declared whole-layer precedence. Honor active-layer replacement (invalid lower
   `TTA_ROOTS` must be ignored by explicit roots). Preserve empty-env and one-root
   behavior, rootless session hits, non-UTF-8 path policy and explicit SSH options.
   The current `.roots(&args.root.iter()...)` / `.maybe_roots(...)` calls are only
   the minimum singular-input port, not a second intended public policy.
2. **Completion must adopt identity, not concatenate names.** Collect all root
   inventories with empty filter and pass them to the pure completion helper.
   Forward every repeated root from Zsh, retain configured slot numbers, make
   `@N/name` candidates discoverable via basename substring/wildcard filtering,
   and test duplicate-name insertion/launch agreement in a PTY. Existing sessions
   and aliases remain independent inventory; Tab is not launch authorization.
3. **Finish target/SSH acceptance.** Target facts/variables, physical probes and
   options already support lists. Add actual target-local `TTA_ROOTS` vs caller
   environment tests, explicit repeated-root override and plan/scaffold results.
   Restore accessible owned scratch before rerunning the existing probe witness
   and real terminal fixtures; do not silently drop or claim them.
4. **Resolve the legacy adapter safety boundary before claiming acceptance.**
   External `rekon` intentionally ignores root and calls its name-only script,
   which hardcodes HOME/src. Recommendation: fail closed for multi-root Create
   using that adapter unless its destination contract is changed. Native adapter
   planning honors the selected root. This is an actual retained limitation, not
   a reason to add unrelated adapters or silently choose another root.
5. **Review the real consumer, not a hypothetical shim.** Its current local helper
   still resolves one root, and completion has that established input contract.
   The API/lock port is coherent and explicit grouping survives. Decide/adopt its
   local plural environment/completion behavior alongside the CLI integration;
   do not mistake target-local SSH list support for complete consumer UI adoption.
6. **Finish maintained guide and acceptance.** Replace staged notes with verified
   final configuration/matching/completion examples, preserve this checkpoint as
   evidence, update the consumer accounts, and close `tta-project-roots` only when
   all accepted surfaces pass. Mixed `.beads/issues.jsonl` remains uncommitted.

No guarded inside switching, input-origin inference, optional remote carrier or
socket tunnel work was started. No agents were spawned from this establishing
turn; parent review and the next G dispatch remain with the parent.
