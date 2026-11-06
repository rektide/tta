---
type: ArchitectureContract
title: Project entry — input states, workspace identity and independent views
description: Declared input progression and mutation boundary for default independent tmux workspace entry
resource: /doc/entry/README.md
status: implemented
author: model:openai/gpt-6.1-sol#high
date: 2026-11-06
sources:
  - resource: opencode://ses_e4de0f8b5ffeW5ODAtJx9rWv3u
    title: Inspect the XState 6 target boundary and document existing session resolution as a separate code-aligned diagram without changing runtime behavior
  - resource: opencode://ses_e4e2b4bcbffePwraSmunm9Lmav
    title: Declare and align the input state machine before implementation, then implement and verify independent outside views, prefix-aware aliases and actual-group matching
  - resource: /session/launcher/src/workflow.rs
    title: Existing pure observation-driven launch progression
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
contract. The existing pure `Workflow` remains
the backbone: observers provide facts; resolution and planning perform no I/O;
execution alone crosses the mutation boundary. The literal input fragment is
never rewritten into a guessed project name.

## Input state machine

```mermaid
flowchart TD
    input["Parse target-local input<br/>CLI overrides environment, then defaults"] --> common["Validate active option/environment values<br/>and satellite prefix"]
    common --> mode{"Read-only completion?"}
    mode -->|Yes| completionFacts["Observe sessions and projects independently"]
    completionFacts --> completion["Expose all literal names + satellite aliases<br/>No winner selection; never create or attach"]
    mode -->|No| validate["Validate fragment, registry/choice,<br/>endpoint and same-server client policy"]
    validate --> sessions["Observe session inventory<br/>Keep canonical IDs and actual group facts"]
    sessions --> aliases["Derive searchable literal names + logical aliases"]
    aliases --> rank["resolve_unique(sessions, fragment, prefix)?<br/>Exact raw → exact alias → shortest substring<br/>Ambiguity/error exits here before continuation"]
    rank --> outcome{"Selection returned?"}
    outcome -->|Some existing| entryMode{"Independent view?<br/>Default outside; explicit --group inside"}
    entryMode -->|No| shared["Prepare direct attach/switch<br/>to selected canonical session ID"]
    entryMode -->|Yes| refresh["Observe fresh inventory<br/>Revalidate selected ID, name and group"]
    refresh --> allocate["Choose group logical base<br/>Allocate prefix + base + smallest free numeric suffix"]
    allocate --> grouped["Prepare ONE tmux queue:<br/>detached grouped create → handoff → keep-last"]
    outcome -->|None| projects["Observe immediate project directories"]
    projects --> projectRank["matching::select(projects, fragment)?<br/>Exact/shortest; ambiguity/error exits before continuation"]
    projectRank --> projectOutcome{"Project returned?"}
    projectOutcome -->|Some directory| base["Prepare ordinary unprefixed base session<br/>Startup argv and initial window apply only here"]
    projectOutcome -->|None| scaffold{"Configured scaffold enabled?"}
    scaffold -->|No| missing["Report missing project; stop"]
    scaffold -->|Yes| destination["Validate safe name, session collision,<br/>absent destination and generator defaults"]
    destination --> generate["Prepare generator → enter directory → base creation<br/>Short-circuit on failure"]
    shared --> prepared["Prepared pure plan"]
    grouped --> prepared
    base --> prepared
    generate --> prepared
    prepared --> emission{"Output mode?"}
    emission -->|print-shell| printed["Render complete CLI expression only<br/>No mutation"]
    emission -->|execute / later evaluate| mutation["MUTATION BOUNDARY<br/>Run prepared action once; no automatic retry/prune"]
    mutation --> handoff["Outside: real terminal attach<br/>Inside same server: client switch"]
    sessions -.->|Observation error| error["Fail closed; unknown is never absence"]
    projects -.->|Observation error| error
    refresh -.->|Error or selected facts changed| error
    destination -.->|Error/present destination| error
    rank -.->|Ambiguous/error| error
    projectRank -.->|Ambiguous/error| error
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

`Workflow::next()` asks for missing observations, calls the resolver through
`scaffold::resolve`, requests fresh sessions for grouped allocation, and lowers
the ready selection with `prepare_launch`. `Workflow::supply()` answers only the
pending request. The chart's boxes name those real stages; it does not introduce
a second scheduler or an unimplemented parallel state machine. Completion takes
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

The [XState 6 target investigation](/design/entry/xstate6-target0.gpt61sol.md)
records the unresolved package/runtime choice for a future faithful port. These
charts describe the current Rust implementation, not a completed XState port.

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
