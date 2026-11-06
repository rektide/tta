---
type: InvestigationReport
title: XState 6 faithful port — target and runtime decision gate
description: Verified JavaScript and Rust XState 6 candidates, current native boundaries and the explicit choice needed before implementing the faithful port
resource: /design/entry/xstate6-target0.gpt61sol.md
status: blocked
author: model:openai/gpt-6.1-sol#high
date: 2026-11-06
sources:
  - resource: opencode://ses_e4de0f8b5ffeW5ODAtJx9rWv3u
    title: Fresh first-stage commission, registry and local-source investigation, Rust boundary review and code-aligned session-resolution diagram
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commission to verify the exact XState 6 technology before a faithful port and leave redesign for a different fresh Sol
  - resource: https://registry.npmjs.org/xstate
    title: Live distribution tags and version publication timestamps observed 2026-11-06
  - resource: https://registry.npmjs.org/xstate/6.0.0-beta.1
    title: Exact published Stately JavaScript package metadata and exports
  - resource: https://crates.io/api/v1/crates/xmachines-runtime/1.1.0
    title: Exact Rust crate version, publication date, checksum and minimum Rust version
  - resource: https://docs.rs/crate/xmachines-runtime/1.1.0
    title: Independent Rust port README, XState alpha.71 target and host-driven runtime contract
  - resource: https://docs.rs/xmachines-runtime/1.1.0/xmachines_runtime/struct.Machine.html
    title: MachineJSON compilation and Rust implementation registration API
  - resource: https://docs.rs/xmachines-runtime/1.1.0/xmachines_runtime/struct.Actor.html
    title: Actor event API, context bounds, error behavior and trait surface
  - resource: /doc/entry/README.md
    title: Implemented entry contract and overall and session-resolution diagrams
  - resource: /session/launcher/src/workflow.rs
    title: Current pure Workflow next/supply interface, validation and fresh inventory behavior
  - resource: /session/launcher/src/resolve.rs
    title: Existing Result guards before optional session and project continuation
  - resource: /session/launcher/src/workspace.rs
    title: Session candidate ranking, actual-group equivalence and satellite naming
  - resource: /session/launcher/src/scaffold.rs
    title: Registry validation, generator selection and pure scaffold expression construction
  - resource: /session/launcher/src/target.rs
    title: Target-local bootstrap and explicit Rust workflow input construction
  - resource: /session/runner/src/driver.rs
    title: Rust observer loop shared by CLI and embedded callers
  - resource: /verification/views/README.md
    title: Prior independent-view verification account, not rerun by this investigation
---

# Outcome: target choice required, not an unavailable-version claim

**XState 6 exists, but “xstate6” does not uniquely identify a native dependency.**
The official package is JavaScript/TypeScript; an independent Rust runtime also
exists, targeting an earlier XState 6 prerelease. Selecting the latter would be
an explicit semantic-port choice, not installing an official Rust XState package.
Selecting the former introduces a JavaScript execution boundary in today's Rust
core and consumers. Neither choice is implied merely by the name.

The commission explicitly requires returning this consequential choice before
broad changes. No Rust code, dependency, lockfile, public API or runtime behavior
was changed. This is **not code-complete** and does not trigger the second
architecture session or the later ticket-surface pitch. The existing overall
input chart now has a [separate session-resolution detail](/doc/entry/README.md#session-resolution-detail)
grounded in current code, with ambiguity guarded before selected/missing flow.

## Verified technology, 2026-11-06

| Candidate | Exact observed version | Standing and boundary |
| --- | --- | --- |
| npm `xstate` | `6.0.0-beta.1` on `beta` | Official Stately JS/TS implementation; published 2026-11-06T15:26:20.919Z. No stable `6.0.0` was listed. |
| npm `xstate` | `6.0.0-alpha.73` on `alpha` | Earlier prerelease channel, not the beta selected automatically by its name. |
| npm `xstate` | `5.33.3` on `latest` | Bare installation selects v5, **not** the requested major. Do not substitute it. |
| crates.io `xmachines-runtime` | `1.1.0` | Independent, explicitly unaffiliated Rust port targeting **`xstate@6.0.0-alpha.71`**, according to its versioned README. Published 2026-11-04T12:00:51.489660Z; not yanked. |
| npm/crates.io `xstate6` | No package found | Both exact package endpoints returned HTTP 404. This says nothing against XState v6 under its actual package name. |

The Rust crate declares Rust **1.99**; this checkout has
`rustc 1.101.0-nightly (822182db4 2026-10-13)`, so its declared floor does not
itself block a local build. Registry checksum for `xmachines-runtime@1.1.0`:
`2baea1349d0ed2f9374552f70e5d0cbac426a6fb2a15b2c5b05f732cdf873591`.
No build or differential conformance test of that crate was performed here.

### Local discovery

Searched names under `~/src`, `~/archive` and the Cargo registry source cache for
`xstate`, `xstate6` and `xmachines`. Found the upstream checkout at
`~/archive/statelyai/xstate` (HEAD
`4519c24b3c7de84d4a4ee8aa821f09db28a8718d`, branch `v6`) and numerous v6 alpha
tags. Its inspected `packages/core/package.json` still says `5.32.0`; the live
registry, rather than that checkout label, establishes published versions.
The upstream README identifies XState as a JavaScript/TypeScript library.
No locally named `xstate6` or `xmachines` repository/cache source was found in
the bounded name search (maximum depth eight for `xstate6`, six for `xmachines`).
This is not proof against an unusually named, unindexed local implementation.

The crates.io `xstate` search found `xmachines-runtime` and its companion crates;
`rustate` describes itself as XState-inspired, not as the requested v6 runtime.
No alternative was silently adopted. No archived checkout was modified or fetched.

## Focused decision options

1. **Use `xmachines-runtime = "=1.1.0"` in the Rust core, explicitly accepting
   its `xstate@6.0.0-alpha.71` semantic target.** Recommended **if the objective
   is an executable declarative XState-6-style entry machine while retaining
   native Rust distribution and consumers**. This requires accepting a young,
   independent implementation, not official Stately beta conformance. First
   exercise its exact API and relevant guard/error/transition semantics; then
   port the existing progression without redesigning matching or execution.
2. **Use official `xstate@6.0.0-beta.1`**, with an explicitly chosen JS runtime
   boundary. This is the right identity if actual Stately execution is required,
   but needs another decision: embedded JS versus an external process, where it
   executes during local/SSH preparation, and how Rust library consumers drive
   it. Neither rewriting all consumers to TypeScript nor adding Node to target
   machines is authorized by the faithful-port request.
3. **Name a different intended package/revision or local path.** If “xstate6”
   refers to something else, establish that identity before evaluating it.

These are proposals, not accepted choices. An official-JS executable reference
model alongside unchanged Rust can aid differential testing, but by itself is
**not** a runtime port and must not be reported as one.

## Current boundary to preserve

The core package `tta-launcher` owns `workflow::Input`, `Workflow`, resolution,
scaffold expressions and pure launch plans. `Workflow::next(&self)` returns the
pending observation or a prepared plan; `supply(&mut self, reply)` answers only
that request. Missing observations are not I/O commands executed by the core.
`tta-runner::driver::drive` supplies observations through `Observer` and is
shared by CLI and embedded callers. SSH uses the same workflow with a different
observer. `target::Bootstrap` acquires target context and registry before
constructing the input. Completion is a distinct read-only inventory path.

A faithful port should replace the **progression authority** inside that
boundary, not merely add an ornamental graph beside the old conditional loop.
Keep the following existing behavior unchanged:

- Validate registry/choice, fragment, prefix and absolute cwd before the first
  workflow session observation; preserve the CLI/target prelude's own validation.
- Query sessions first. `resolve_unique(...)?` rejects ambiguity before optional
  selected/missing continuation; only a genuine `None` proceeds to projects.
- Keep literal exact precedence, one-step aliases, shortest Unicode ranking,
  actual-group tie collapse and deterministic representative selection.
- Request projects, destination and generator defaults only when required.
  Observation failure never becomes absence or authorization to create.
- Resolve omitted grouping to outside independent attachment, inside ordinary
  switch; honor explicit caller policy and configurable satellite prefix.
- For independent existing entry request a second session inventory; reject
  changed selected ID/name/group, allocate the same smallest free complete name,
  and retain create → handoff → keep-last as one prepared tmux queue.
- Keep prepare/print-shell separate from mutation; retain literal argv/path
  handling, no retries/pruning and existing inside-client-origin limitations.
- Mismatched replies must not advance, and ready workflows reject more replies.

## Architectural friction for the later fresh Sol

These are inspected friction points and verification obligations, **not** a
redesign commission or evidence that any public API change is unavoidable:

- Current progress is recomputed from five optional fact slots on every `next`;
  `supply` calls `next` again. A native actor port needs an intentional mapping
  between this public pull interface and internal event-driven transitions.
  Changing validation/error timing during eager transitions would not be faithful.
- `xmachines-runtime::Machine::from_json_str` compiles MachineJSON with a Rust
  `Registry`; the documented registry supports guards, expressions and actions.
  Its runtime is host-time-driven and claims no I/O, so a Rust integration can
  plausibly retain the pure core and existing observers without Node/JS.
  That is API-based feasibility, **not an implementation proof**.
- The documented `Actor<C, E>` requires serializable/deserializable, cloneable
  context and an `EventType`; its event operations take `&mut self`. `Actor` has
  no documented `Clone` implementation and is `!Sync`, while today's `Workflow`
  derives `Clone` and contains ordinary owned native inputs. Blindly embedding
  an actor changes the public trait surface. Keep native path/argv and generator
  function pointers outside a JSON-only context unless a lossless representation
  is justified. Do not lossy-stringify `OsString` or serialize all inputs merely
  to satisfy the runtime.
- Runtime guard failures are strings/system errors and can put actors in error
  status; current errors retain typed `ResolveError`/scaffold/plan source chains.
  Preserve those guards and errors at the adapter boundary rather than treating
  ambiguity as an ordinary machine state or erasing the source.
- Grouped entry reuses the same `Sessions` request/reply variant for initial and
  fresh inventory. Make their distinct internal stages explicit without inventing
  new externally observable requests or adding automatic recovery.
- Current matching is useful domain logic, not necessarily a state per loop
  iteration. The separate session diagram expands its real algorithm; a port's
  chart should name actual machine stages and Rust guard calls, not imply that
  every derived-candidate operation became an actor.

No API change is necessary for this investigation. Whether the selected port
can preserve `Workflow`'s complete interface and traits remains to be tested,
not waived in advance. Broader decomposition belongs after a faithful port and
its independent verification, with a **different fresh Sol** as commissioned.

## Verification and handoff standing

This pass read the two requested bead records (`tta-state-machine` and
`tta-launch-backends-independent-views`), the entry and real-terminal contracts,
current workflow/resolver/workspace/scaffold/target/driver code, upstream local
source and versioned registry/docs sources. It changed documentation only.
The diagram was traced manually against `workspace::select`, `resolve_unique`
and `resolve`. The report and revised entry contract passed `rkt check --okf`.
Rust and real-terminal suites were **not rerun**: their previous results remain
at [the independent-view verification home](/verification/views/README.md), not
new evidence for an unimplemented port.

After target acceptance, preserve the existing core/runner-free, CLI/native,
real-terminal and SSH verification surfaces. Add state-transition/repeated-next,
reply mismatch, error-ordering and selected/missing guard assertions for the
actual chosen runtime. Keep the overall and separate resolution diagrams aligned
with that code. Do not begin multi-root discovery (`tta-project-roots`), the
second-session redesign or ticket reorganization here.

The preexisting `.beads/issues.jsonl` modification was left untouched and excluded
from this stage's commit. No install, push, package substitution, JS process,
runtime embedding or new agent dispatch was performed.
