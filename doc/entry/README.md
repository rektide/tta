---
type: ArchitectureContract
title: Project entry — input states, workspace identity and independent views
description: Declared input progression and mutation boundary for default independent tmux workspace entry
resource: /doc/entry/README.md
status: implemented
author: model:openai/gpt-6.1-sol#high
date: 2026-11-06
updated: {by: "model:openai/gpt-6.1-sol#high", at: "2026-11-09"}
sources:
  - resource: opencode://ses_e3d9d71bfffe3f2KznncOc1djO
    title: Distinguish the approved internal rewrite and retained kernel from this still-public replay contract
  - resource: opencode://ses_e3db9cabaffeSpjYjLIjvqdbPO
    title: Link the newly commissioned architectural redesign while preserving this implemented contract
  - resource: opencode://ses_e3e3170d4ffeij7U00lUlwaZg7
    title: Declare multi-root selection and safe creation before implementation; retain the eight-stage observation graph
  - resource: opencode://ses_e4dbbb07bffeqm6Khh2fp29jNR
    title: Link the post-completion capability-oriented ticket proposal from its entry domain
  - resource: opencode://ses_e4dca921dffexww2JZDdY3Zyne
    title: Review the port independently, retain immutable replay and the eight-stage graph, type native dispatch and enforce exact ordered diagram conformance
  - resource: opencode://ses_e4de0f8b5ffeW5ODAtJx9rWv3u
    title: Verify the XState 6 target, port pure progression to xmachines-runtime 1.1.0, preserve native data and errors, and align both diagrams with verified code
  - resource: opencode://ses_e4e2b4bcbffePwraSmunm9Lmav
    title: Declare and align the input state machine before implementation, then implement and verify independent outside views, prefix-aware aliases and actual-group matching
  - resource: /session/launcher/src/workflow.rs
    title: Pure supplied-facts handlers dispatched by the executable control graph
  - resource: /session/launcher/src/workflow/entry.machine.json
    title: Executable entry stages and ordered policy transitions
  - resource: /session/launcher/src/workflow/machine.rs
    title: Pinned XState 6 Rust runtime adapter, native guards and thin control events
  - resource: /session/launcher/src/resolve.rs
    title: Session-first resolution and project/scaffold fallback
  - resource: /session/launcher/src/workspace.rs
    title: Literal exact preference, alias ranking and actual-group tie collapse
  - resource: /session/launcher/src/plan.rs
    title: Existing grouped create, client handoff and keep-last queue
  - resource: /design/launcher/launch-backends1.gpt61sol.md
    title: Independent-window outcome, orphan identity and shared-pane origin limitations
  - resource: /design/launcher/launch-backends2.gpt6a.md
    title: Accepted outside-default policy and separate inside-switching commission
  - resource: /verification/trials/independent-views/README.md
    title: Preserved real-terminal evidence and historical wrong-client reproduction
---

# Project entry contract

Declared before implementation in commit `285e3113`, then aligned with the
human's guard-before-continuation correction in `2cb8dc6f`, this is the implemented
contract. `Workflow` now drives the executable entry graph through
**`xmachines-runtime =1.1.0`**, the independent Rust implementation targeting
XState **`6.0.0-alpha.71`** semantics, selected by the human after the
[target investigation](/design/entry/xstate6-target0.gpt61sol.md).
Observers provide facts; resolution and planning perform no I/O; execution alone
crosses the mutation boundary. The literal input fragment is never rewritten
into a guessed project name.

## Input state machine

```mermaid
flowchart TD
    input["Parse target-local input<br/>CLI overrides environment, then defaults"] --> common["Validate active option/environment values<br/>and satellite prefix"]
    common --> mode{"Read-only completion?"}
    mode -->|Yes| completionFacts["Observe sessions and projects independently"]
    completionFacts --> completion["Expose session aliases and project selectors<br/>No launch selection, machine, creation or attach"]
    mode -->|No| validate["Validate endpoint/client policy<br/>Workflow::new: registry/choice, fragment, prefix, absolute cwd"]
    validate --> sessions

    sessions["sessions: initial inventory<br/>Wait if missing; retain canonical IDs and actual groups"]
    resolve["resolve: scaffold::resolve(...) ?<br/>Session guard before Some/None; only None tries projects<br/>Project guard before Some/None; only missing may scaffold"]
    projects["projects: next required root's immediate directory facts<br/>Wait if missing; retain requested + physical root and physical entries"]
    destination["destination: requested safe creation path<br/>Wait if missing; present/error fails in resolver"]
    defaults["defaults: requested generator author/license facts<br/>Wait if missing; native values only"]
    fresh_sessions["fresh_sessions: second inventory for independent existing entry<br/>Wait if missing; guard unchanged ID/name/group<br/>Allocate prefix + logical base + smallest free suffix"]
    prepare["prepare: prepare_launch(...) ?<br/>Existing direct, ONE grouped queue, project base,<br/>or generator → enter directory → base creation"]
    ready["ready: return PreparedLaunch<br/>No target mutation"]

    sessions -->|observed| resolve
    resolve -->|projects_needed| projects
    projects -->|observed| resolve
    resolve -->|destination_needed| destination
    destination -->|observed| resolve
    resolve -->|resolved, needs_defaults| defaults
    defaults -->|observed| resolve
    resolve -->|resolved, needs_satellite| fresh_sessions
    fresh_sessions -->|observed| prepare
    resolve -->|resolved, otherwise| prepare
    prepare -->|prepared| ready

    ready --> emission{"Output mode?"}
    emission -->|print-shell| printed["Render complete CLI expression only<br/>No mutation"]
    emission -->|execute / later evaluate| mutation["MUTATION BOUNDARY<br/>Run prepared action once; no automatic retry/prune"]
    mutation --> handoff["Outside: real terminal attach<br/>Inside same server: client switch"]
    sessions -.->|Observer error| error["Return error; stop before selected/missing continuation<br/>Unknown is never absence"]
    projects -.->|Observer error| error
    destination -.->|Observer error| error
    defaults -.->|Observer error| error
    fresh_sessions -.->|Observer error or selected facts changed| error
    resolve -.->|Ambiguous, invalid, present destination or missing| error
    prepare -.->|Plan error| error
```

Classified absent servers and successful empty inventories are empty session
facts. Permission, protocol, decoding, incomplete capture and other acquisition
failures are errors. Completion may suppress diagnostics for Tab, but may not
interpret a failed lookup as permission to mutate.

### Code and chart are the same progression

`resolve_unique(sessions, fragment, prefix) -> Result<Option<Selection>, ResolveError>`
is the session-resolution stage in `resolve.rs`. It derives/ranks workspace
candidates and maps ambiguity to an error before returning. `resolve()` uses
`if let Some(selection) = resolve_unique(...)? { return Selected(selection); }`;
only `None` continues to `NeedProjects`. Project matching similarly applies its
error guard before the `Some`/`None` continuation. These errors are not main-flow
states, selections or opportunities to create a project.

The eight lower-case nodes are exactly the states in
[`entry.machine.json`](/session/launcher/src/workflow/entry.machine.json); their
labeled arrows are its transitions. A regression test checks exact diagram
routes and per-event guard order against the parsed executable graph, rejecting
extra/missing routes as well as event-name drift. Preprocessing/completion and
emission/execution stay outside the graph. The `resolved` transition tests
`needs_defaults`, then `needs_satellite`, then the unguarded preparation path.
Domain errors leave through native `?` **before** a successful control event;
they are not ambiguous/failed main-flow machine states.

`Workflow::next(&self)` starts a local actor over the shared compiled definition
and replays already supplied observations along those arrows. At the first
missing fact it returns `Observe`; it performs no observation itself.
`Workflow::supply()` answers only that pending request, storing the native fact;
the next pull replays again. This preserves immutable pulls, independent clones
and `Send + Sync` without retaining or locking a mutable actor. Rust handlers
exhaustively dispatch on the actor's typed `Stage` and send outcome events; they
do **not** choose a second independent next state. Only event kinds and two policy booleans
are serialized; paths, argv, config, generator functions, plans and typed errors
remain native. The graph has no actions, timers, invocations or I/O.

The independent architecture review retains this progression rather than
redesigning target bootstrap, resolver, matching or execution. The
[port report](/design/entry/xmachines-port0.gpt61sol.md) records its baseline and
friction; the [architecture review](/design/entry/architecture0.gpt61sol.md) records
the retention decisions, hardening and measured replay costs. Completion takes
`session_names()` and uses its shared `satellite_alias()` derivation without
calling the winner-selection stage.

### Session-resolution detail

This expands the overall chart's `resolve_unique(...) ?` stage, not the project
fallback or satellite allocation. It follows `workspace::select` and the
`resolve_unique` error guard in their actual order:

```mermaid
flowchart TD
    resolveUnique["resolve_unique(sessions, argument, prefix)"] --> validateSessionInput["Guard: nonempty fragment and valid prefix"]
    validateSessionInput --> rawExact{"Literal session name equals fragment?"}
    rawExact -->|Yes| exactSession["Return that canonical session from workspace::select"]
    rawExact -->|No| aliasCandidates["For each session: literal name + one-step satellite alias<br/>Keep literal substring matches; score by minimum Unicode length"]
    aliasCandidates --> shortestCandidates["Retain globally shortest scores<br/>Sort by literal name, then canonical ID"]
    shortestCandidates --> workspaceCandidates["Collapse equal IDs or equal nonempty actual groups<br/>Keep first sorted representative; ungrouped sessions stay distinct"]
    workspaceCandidates --> uniqueGuard["Guard: at most one workspace candidate<br/>workspace::select error maps to ResolveError::Sessions via ?"]
    uniqueGuard --> uniqueOption["Return Ok(None) or Ok(Some(session))"]
    exactSession --> existingSelection["resolve_unique maps Some(session)<br/>to Some(Selection::Existing(session))"]
    uniqueOption --> mapSelection["resolve_unique maps optional session<br/>to optional existing selection"]
    mapSelection --> resolveContinuation["resolve(): resolve_unique(...) ?"]
    existingSelection --> resolveContinuation
    resolveContinuation --> selectionReturned{"Selection returned?"}
    selectionReturned -->|Some| selectedSession["Return Resolution::Selected<br/>Do not inspect project directories"]
    selectionReturned -->|None| projectContinuation["Continue to project facts<br/>NeedProjects if not supplied"]
    validateSessionInput -.->|Invalid input| resolutionError["Return error; no selected/missing continuation"]
    uniqueGuard -.->|Ambiguous| resolutionError
```

Exact aliases are not a separate search pass in the code: their score equals
the fragment's length and therefore wins the same minimum-length comparison.
Actual-group collapse happens **after** shortest ranking; group membership does
not admit longer candidates. Ambiguity leaves through `?` before the final
`Some`/`None` decision. Completion does not execute this selection procedure.

This remains the native resolver's algorithm inside the machine's `resolve`
stage. Porting progression does not turn each candidate-loop operation into an
actor or replace actual group facts with alias-based identity.

## Ordered project roots

The [multi-root declaration](/doc/entry/project-roots.md) specifies configuration
precedence, qualified selectors, physical-directory provenance and safe scaffold
destinations. The graph remains eight stages: resolve may request another root
through its existing projects route before selecting a winner. Surface integration
and final acceptance remain tracked by `tta-project-roots`.
The [establishing checkpoint](/design/entry/project-roots0.gpt61sol.md) records
the implemented core, local/remote observation agreement and bounded integration
handoff. The executable graph itself is unchanged.

## Names, aliases and workspace winners

- Default prefix: `_`. Override with `--satellite-prefix` or target-local
  `TTA_SATELLITE_PREFIX`; explicit CLI wins. Rust APIs accept the same explicit
  prefix. SSH uses target environment, never the caller's local prefix variable.
- A prefix must be nonempty and tmux-session-name-safe: no `.`, `:` or control
  characters. Explicit empty prefix is an error. Empty environment values are
  unset, consistently with existing environment policy.
- A satellite alias removes **exactly one** configured leading prefix and
  **exactly one** terminal `-DIGITS`, with a nonempty base and one or more ASCII
  decimal digits. Examples: `_project-42` → `project`, `_project-7-42` →
  `project-7`; `_project-x` and `project-42` remain literal with prefix `_`.
  Zero and leading zeros are recognized as numeric aliases, although allocation
  emits ordinary positive counters starting at `1`. No recursive stripping.
- Exact literal selectors take priority. Otherwise exact aliases win, followed
  by shortest Unicode-character matching raw/alias names. As exact aliases have
  the fragment's length, this is also minimum-length ranking. Substrings remain
  literal and case-sensitive; no wildcard interpretation in launch resolution.
- Equally ranked sessions in the **same actual tmux group** are one workspace
  candidate. Choose its representative deterministically by literal name then
  canonical ID. Ungrouped sessions are separate candidates. Names alone never
  establish group membership. Distinct groups with identical aliases remain
  ambiguous; the error offers literal session selectors to disambiguate.
- An exact raw `project` selects that session even if an unrelated `_project-42`
  offers alias `project`. If only `_project-42` survives, `tta project` selects
  that existing session, even when no project directory exists. Its canonical ID
  remains the creation target; no new processes are started for existing work.
- Completion offers raw names **and** aliases without shortest-winner filtering.
  An ambiguous alias is still a valid navigation term, not proof of uniqueness;
  raw names remain available. Zsh first offers all logical workspace aliases,
  falling back to raw selectors when the typed fragment does not match an alias
  (for example `_proj`). This keeps generated names from hiding `project` on Tab.
  The full read-only inventory retains both; neither inventory selects a winner.
  Completion and launch use one alias function.

## Independent view allocation and lifecycle

The first outside project entry creates an ordinary session named after the project
(or explicit new-session override). Every subsequent independent entry creates
a distinct session sharing the selected session's actual group windows/panes.
Changing one session's current window does not change another's. Shared panes
still share running processes and pane-local state.

Before allocation, request a fresh inventory and require the selected session's
ID, name and group to agree with the selection snapshot. Stop on disappearance
or change; do not rerank, scaffold, retry or silently choose another workspace.
For the selected actual group, prefer a non-satellite member's name as base
(shortest name, then lexical order); with only satellites, use the selected
session's one-step alias. An ungrouped selection uses its own name/alias.
Allocation never produces `_` + an already decorated satellite + another suffix.

Choose the smallest positive decimal counter whose **complete generated name**
is unused among all sessions, including unrelated groups. Exact names, rather
than parsed integer values, determine collisions: `_project-01` does not occupy
`_project-1`. No lock is implied by the snapshot. Concurrent entrants can collide;
tmux failure stops the queue, with no blind retry or alternate-match fallback.

Create → attach/switch → set `destroy-unattached=keep-last` stays one CLI tmux
queue on both backend selections and in shell/SSH output. This avoids detached
satellite destruction between separate actions. Detaching removes a satellite
when another group session remains; the final session preserves the work.
Failures after creation can leave residue; no automatic cleanup is introduced.

`--group=false` and `.grouped(false)` deliberately join the selected session's
window selection instead. Omitted grouping resolves to true for outside attach
and false for inside switch, consistently across CLI, workflow, runner and
low-level lowering APIs. Explicit caller policy is never replaced. Grouped
low-level lowering requires a supplied satellite name for existing selections;
the higher-level workflow performs allocation. No startup command runs merely
for joining a group.

## Explicit boundary: inside-client origin

Outside terminals, including new SSH terminals, each own their real attached
tmux client. Inside entry retains the existing same-server switching boundary.
Two clients displaying the same shared pane have identical shell, environment
and pane tty; tmux's activity heuristic can choose the other client. This work
does **not** establish input-origin identity, guarded native switching, repeated
inside-entry view reuse, or an authoritative client-token contract. Those remain
separate work; independent outside-window selection must not be described as
fixing that ambiguity.

## Verification obligations

Prove plain entry with two **real terminal clients**, not control-mode substitutes:
same window IDs, pane IDs and process IDs; distinct sessions; selecting in either
client leaves the other selection unchanged. Cover native/CLI, direct/evaluated
shell entry, explicit shared attachment, custom prefix, orphan-only alias lookup
and completion, and last-view survival. Retain existing lifecycle, runner-free and
SSH boundary tests; verify SSH defaults/prefix precedence and generated action.
Unit tests must distinguish aliases from actual group identity, preserve shortest
matching and ambiguity, and reject changed fresh facts without authorizing mutation.

The [maintained user guide](/README.md#how-we-use-tmux) describes released behavior;
the [historical two-terminal trial](/verification/trials/independent-views/README.md)
preserves the defect evidence this contract must not erase. Current acceptance:
[two-real-terminal verification](/verification/views/README.md).

## Related work organization

The later [entry-attempt redesign](/design/entry/redesign0.gpt6a.md)
responds to the human's clarified request for a substantial state-machine-powered
rewrite. The human approved its new orchestration owner, domain model, stronger
binding and effect lifecycle. The [first retained kernel](/design/entry/kernel0.gpt61sol.md)
is implemented internally through existing-work export, with a
[generated structural graph](attempt-graph.md). Public CLI/consumer entry still
uses the contract above until coherent cutover; this is not a public facade over
the new actor. The
[inquiry index](/design/entry/README.md) connects the previous stages and current
implementation standing.

The [capability-oriented ticket pitch](/design/entry/ticket-surface0.gpt6a.md)
proposes navigation from user-visible entry, discovery, creation and remote work
to existing delivery and follow-up tickets. It preserves the distinction between
shipped outside views, open guarded inside switching and multi-root discovery;
the proposed ticket changes are not an adopted hierarchy or new runtime contract.
