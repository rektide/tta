---
type: ImplementationReport
title: Machine-owned discovery and creation — entry attempt second stage
description: Chart-owned root scope and read loop, guarded physical ranking, creation ladder with validated-generator requirements and single materialization, semantic StartProject/ScaffoldAndStart, fresh route/destination binding and complete short-circuit export
resource: /design/entry/discovery0.glm53m.md
status: implemented-internal
author: model:zai-coding-plan/glm-5.3#max
date: 2026-11-09
sources:
  - resource: opencode://ses_e3d8804a3ffez4iETRd6llQTnL
    title: Implement and verify the machine-owned discovery/creation stage, record the deletion delta and hand the hosts stage to the next Sol session
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commissions this G stage under the approved destination and controls rotation and architecture review
  - resource: /design/entry/redesign0.gpt6a.md
    title: Approved destination, invariants and landing sequence this stage implements
  - resource: /design/entry/kernel0.gpt61sol.md
    title: Retained kernel, protocol invariants and the exact discovery handoff followed here
  - resource: /session/launcher/src/entry/entry.machine.json
    title: Extended executable chart owning scope, the root read loop, ranking and the creation ladder
  - resource: /session/launcher/src/entry/discover.rs
    title: Chart-owned discovery task, guarded ranking and creation preparation
  - resource: /session/launcher/src/entry/protocol.rs
    title: New correlated request kinds, observations, purposes and typed errors
  - resource: /session/launcher/src/entry/action.rs
    title: StartProject/ScaffoldAndStart decisions, routes, creation sites and semantic lowering
  - resource: /session/launcher/src/entry/prepare.rs
    title: New-work plans and exactly-once generator materialization with actual facts
  - resource: /session/launcher/src/entry/bind.rs
    title: Fresh name, route-identity and destination-absence binding checks
  - resource: /session/launcher/src/entry/mod.rs
    title: Extended ingress shape gate and exhaustive native directive handlers
  - resource: /session/launcher/src/entry/machine.rs
    title: Discovery emission, reentry, guard and vocabulary witnesses on the retained actor
  - resource: /session/launcher/src/entry/tests.rs
    title: Discovery, creation, invalid-ingress and new-work binding/lowering journeys
  - resource: /session/launcher/src/scaffold.rs
    title: Validated generator specification with adapter-declared destination contracts
  - resource: /scaffold/rekon/src/lib.rs
    title: External adapter declaring its own destination selection and zero default needs
  - resource: /scaffold/rekon-native/src/lib.rs
    title: Native adapter validation split with author/license requirements
  - resource: /doc/entry/project-roots.md
    title: Ordered-root, physical-identity and safe-creation contract preserved
  - resource: /doc/entry/attempt-graph.md
    title: Regenerated structural projection of the extended chart
  - resource: /verification/entry/README.md
    title: Focused commands and journey coverage for this stage
---

# Outcome and authority

**The second rewrite stage is implemented internally.** The same retained
`EntryAttempt` actor that owned existing-work entry now also owns project
discovery and creation. After `sessions_rank.empty`, the executable chart
parses the root scope, walks the required roots through an explicit
self-reentering read state, ranks globally over complete evidence with a
physical-identity guard, and then either prepares a `StartProject` plan or
walks the creation ladder — policy, destination absence, adapter validation,
required-default observation, one materialization — to a `ScaffoldAndStart`
plan. Binding rechecks fresh session names plus the chosen route's physical
identity or the creation destination's continued absence. The staged
`DiscoveryNotImplemented` boundary is gone. Nothing in `entry/` calls the old
workflow, core resolver, scaffold resolver wrapper, runner loops, `Selection`
or `NativeCreation`.

Tracking: epic `tta-entry-attempt`; this stage `tta-entry-attempt-discovery`;
next `tta-entry-attempt-hosts`. The parent retains architecture review and
agent rotation. No agent was spawned by this session.

## Implemented versus future

| Implemented in this checkpoint | Not implemented or claimed here |
| --- | --- |
| Chart-owned scope, per-root read loop, guarded physical rank | Host-side root/destination/defaults acquisition (hosts stage) |
| Typed Projects/Destination/GeneratorDefaults/ProjectIdentity requests with exact echo | Registry path walk, target/environment/context acquisition |
| Exhaustive kind/observation shape gate before `pending.take()` | Batched observations (protocol permits them later) |
| StartProject/ScaffoldAndStart with routes, site, program and session policy | Execution/effect lifecycle, run states, dispatch (cutover) |
| Validated spec → requirements → materialize-once generator API | Generator-spec evolution, new adapters |
| External rekon multi-root refusal via declared destination contract | Tightening its single-root divergence (separate review) |
| Fresh route/destination binding without reranking | Anchor rescue, retries, cleanup, alternate routes |
| Short-circuit `cd`/create/switch lowering with literal startup argv | Native creation path, execution shape choice (cutover) |
| Catalog/completion remains untouched; old public path unchanged | CLI/SSH/consumer adoption of this actor |

`lib.rs` keeps `entry` private; the staged `allow(dead_code)` remains until
cutover. Public 0.7 APIs are unchanged, so there is still no version bump.

## Chart and native ownership

- `select` gains `project_scope` (emit `scope_projects`), `projects_read`
  (emit `read_projects`, self-reentering on `observed_projects` while the
  `more_roots` guard holds) and `projects_rank` (emit `rank_projects`).
  The remaining-root cursor lives in the native discovery task; the event
  carries only the validated `more_roots` summary and no matcher ever returns
  a request or successor state.
- `prepare_start` and `prepare_create` mirror the approved sketch:
  `create_policy → destination_read → generator_spec →(needs_defaults)
  defaults_read → creation`, reusing `observed`/`prepared` where the kernel
  already established them. Destination `Present` stops at `destination_read`
  itself; adapter options validate only after absence was established.
- `bind_action` gains `binding_evidence` between the fresh session read and
  the shared check/lower states, routed by the `needs_binding_evidence` guard
  on `BindingObserved { evidence_needed }` — a payload summary computed by
  the ingress from the plan's decision kind, exactly like `ViewPolicy`.
- Guards registered: `independent`, `more_roots`, `needs_defaults`,
  `needs_binding_evidence`. The renderer needed no new structural features;
  the generated graph and the exhaustive-directive/`references` test were
  extended instead.

### Ingress order

A reply must echo the exact pending request (attempt, sequence, target
binding, and now slot/root/filter or path/purpose) and then pass an
exhaustive request-kind ↔ observation-kind compatibility check — both before
`pending.take()`, so wrong-shape, wrong-slot, stale and duplicate replies
change no actor, data or journal state (`ProtocolError::ReadMismatch` /
`ObservationShape` / `NoPendingRead`). Accepted observation failures stop
natively at the reading boundary with the original source chain; they never
become absence. Malformed complete root inventories stop through the pure
`projects::sources` validation at acceptance, never at ranking.

### Discovery and creation semantics

- `@N/name` observes only slot N; unqualified selectors observe every
  configured root in order through the reentering loop, all of them complete
  before ranking. A session hit — including a literal project-looking name —
  never reaches `project_scope` (kernel guard `b92e00c1` kept, now with roots
  configured in the witness).
- Ranking is the reused pure primitive: literal shortest-Unicode basename
  across all observed roots, collapse by canonical physical directory.
  Distinct physical directories are a typed ambiguity offering `@N/name`
  selectors; equivalent routes (symlinked/aliased roots) share one identity,
  retain every observed route in the plan, and choose the deterministic route
  by configured-root then basename order.
- Creation policy, before any absence read: generator availability (with the
  bare-scaffold empty-registry guidance preserved), safe single-component
  fragment, explicit destination choice — unqualified creation only when all
  observed slots refer to one physical root, offering `@N/name` otherwise —
  and session name validation/collision against the selection inventory.
- Destination absence is exact-path `Destination` evidence; only `Absent`
  proceeds, `Present` (including dangling links) stops. Adapter validation
  then checks options and requirements, and applies the **declared
  destination contract**: an adapter whose program selects its own
  destination (external rekon, declared by its crate) is refused when more
  than one root is configured, with the same guidance as today; the
  single-root divergence remains documented behavior. No adapter-name string
  comparison lives in generic resolution code.
- Required defaults are observed once (`GeneratorDefaults { cwd, needs }`,
  explicitly-absent values are completed facts) and the generator program is
  materialized exactly once with those actual facts. No expression is ever
  built with fake empty defaults and re-resolved.

### Plans, binding and lowering

`StartProject` carries the chosen physical identity, selected route, every
equivalent route and a once-settled `NewSessionSpec` (validated name,
constructor-checked pane directory, raw startup argv, window policy).
`ScaffoldAndStart` adds the fixed `CreationSite` and the materialized
`GeneratorProgram`. `DecisionBasis` is now typed per branch (`Workspace` /
`Project` / `Creation`) and retains root-read, destination and defaults
request provenance. Startup/window/directory fields cannot reach
existing-work decisions.

Binding for new work reads fresh sessions (name availability) plus one
evidence read — `ProjectIdentity` canonicalizing the chosen route's entry
path (must still equal the observed physical directory) or a fresh
`Destination` absence check — then fails closed on change without reranking
or anchor rescue. Lowering emits the complete short-circuit sequences:
`cd -P` + create (+attach) for outside starts, detached create then exact
`switch-client` for inside ones, with the generator program prefixed for
scaffolds, startup argv as literal `--` operands (lone executables wrapped
in `/bin/sh -c exec "$0"`, trailing semicolons argv-escaped), tmux format
escaping and socket-label/config policy unchanged from the kernel.

## Deletion delta

The acceptance asked for proof that `resolve → Need* → resolve` scheduling
did not survive under new names. Source inspection of `session/launcher/src/entry/`
finds **zero** references to `Resolution`, `NeedProjects`, `NeedDestination`,
the `resolve()`/`resolve_with_config` builders, `workflow`, the `Selection`
type or `NativeCreation`. The only cross-module uses are the sanctioned pure
mechanics — `projects::{scope, sources, select, component}`,
`resolve::validate_name`, `workspace::select`/`satellite_name`,
`scaffold::{Config, Choice, Entry, Validated, DestinationContract, Defaults}`
— each called once per directive from native handlers that return only
control events. The chart owns every observation transition. (`validate_name`
still lives in `resolve.rs`; relocating it belongs to the cutover deletion of
that file, per the kernel's obligation table.)

## Checkpoints and verification

Explicit owned-path commits (`.beads/issues.jsonl` excluded from each):

1. **`c1c58768`** — split generator adapters into validate / requirements /
   declared destination contract (adapter crates + registry vtable).
2. **`870c3925`** — the machine-owned discovery/creation stage: chart,
   protocol, discover/prepare/bind/action, ingress shape gate, tests, graph.

| Check executed in this session | Result |
| --- | --- |
| Entry nextest, launcher all features | **50/50 passed**, including 21 new discovery/creation journeys |
| Entry nextest, launcher no default features | **40/40 passed** (adapter-dependent cases feature-gated) |
| Entry nextest, `scaffold-rekon` only / `scaffold-rekon-native` only | **47/47** / **43/43** passed |
| Full launcher + adapter crates, all features | **114/114 passed** (existing suites unaffected) |
| Full launcher, no default features | **85/85 passed** |
| Launcher all-target/all-feature clippy, no deps, `-D warnings` | Passed; dependency-owned tmax warnings unchanged |
| Launcher formatting check | Passed |
| Structural chart `--check` after regeneration | Passed against the executable JSON |
| Python renderer regression tests | **4/4 passed** |
| `rkt check --okf` on owned Markdown documents | Passed (see final commit) |

New witnesses cover: wrong-kind/wrong-slot/stale/duplicate replies rejected
before pending consumption; multi-root loop order and completeness before
ranking; distinct-physical ambiguity vs equivalent-route identity collapse
with retained provenance; missing and malformed root evidence stopping
natively; typed missing and scaffold guidance; destination-absence-before-
adapter-work; required-defaults observation with single materialization
(observed author baked into the program); the declared-contract multi-root
refusal and preserved single-root divergence; fresh name/route/destination
binding failures without reranking; startup/window/directory lowering
exactness including the sh wrapper and semicolon escaping; and session-first
literal priority with roots configured.

These exercise the new actor with supplied observations only. They are not
CLI/SSH/real-terminal/consumer cutover evidence; prior runtime evidence keeps
its original attribution.

## Exact handoff to Sol high: `tta-entry-attempt-hosts`

Read the approved [redesign](redesign0.gpt6a.md) stage 3, this report, the
kernel report, tickets, and the current `runner/` + `target.rs` + CLI code.
Claim the existing ticket; preserve append history. Sequential owned-path jj
commits; keep `entry` private; no push, install or consumer edits.

1. **Move pure context policy into its shared owner** (`context/{mod,policy}`
   per the redesign's ownership table or an equivalent entry-owned module):
   whole-layer precedence with caller-native environment values selected
   before decoding (the redesign's `environment.rs:51–55` divergence class),
   supplied-context constructor matching `SuppliedContext`, and target
   binding reuse. The entry attempt must stay suppliable-facts-first.
2. **Adapt local observation** to the new request vocabulary: sessions
   (existing observer), `projects::project_inventory`-style root reads
   (canonical roots, argument-filtered entries), exact `destination_state`
   lstat classification, `observe_defaults` for `GeneratorDefaults`, and a
   bounded canonicalize for `ProjectIdentity`. Keep them as observation
   implementations behind one interpreter that consumes `Waiting::Read` and
   replies with exact `ReadRequest` echoes; the interpreter must not choose
   work, generators, names or successor states.
3. **Registry walk into acquisition**: `bind_target.registry`'s path walk
   becomes a bounded acquisition task answering one semantic registry
   request (the kernel already validates supplied registries first).
4. **Add the read-only `CatalogQuery`** sharing target-context construction,
   request types, observation adapters and candidate constructors with a
   bind→collect→report graph; it must be incapable of emitting a mutation.
   Reuse `completion_candidates`, `session_names` and the identity/provenance
   types from `select`/`discover` rather than rebuilding them. Completion
   keeps per-source failure visible; missing root evidence never certifies a
   bare name.
5. **Worker-free SSH observation**: adapt the probe codecs/framing/capture
   limits to the same request vocabulary on the frozen target binding;
   preserve nonce binding, complete-EOF checks and diagnostic receipts.
   One driver shared by local and remote, not a second program.
6. **Tests**: runner-free supplied-facts and poison witnesses, local/probe
   agreement, context/request mismatch rejection, catalog physical-uniqueness
   under complete/partial inventories, and query-incapable-of-mutation
   witnesses. Update the verification guide and graph docs as structure
   changes. Return a bounded checkpoint; execution/cutover remains
   `tta-entry-attempt-cutover`.

The kernel's deletion-obligation table remains the cutover contract; nothing
in it was weakened by this stage.
