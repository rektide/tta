---
type: ArchitectureContract
title: Project entry — input states, workspace identity and independent views
description: Declared input progression and mutation boundary for default independent tmux workspace entry
resource: /doc/entry/README.md
status: implementation-contract
author: model:openai/gpt-6.1-sol#high
date: 2026-11-06
sources:
  - resource: opencode://ses_e4e2b4bcbffePwraSmunm9Lmav
    title: Declare the input state machine before continuing implementation of default independent views and satellite aliases
  - resource: /session/launcher/src/workflow.rs
    title: Existing pure observation-driven launch progression
  - resource: /session/launcher/src/resolve.rs
    title: Session-first resolution and project/scaffold fallback
  - resource: /session/launcher/src/plan.rs
    title: Existing grouped create, client handoff and keep-last queue
  - resource: /design/launcher/launch-backends1.gpt61sol.md
    title: Independent-window outcome, orphan identity and shared-pane origin limitations
  - resource: /verification/trials/independent-views/README.md
    title: Preserved real-terminal evidence and historical wrong-client reproduction
---

# Project entry contract

This declares the implementation destination, not a retrospective claim that
every transition is already implemented. The existing pure `Workflow` remains
the backbone: observers provide facts; resolution and planning perform no I/O;
execution alone crosses the mutation boundary. The literal input fragment is
never rewritten into a guessed project name.

## Input state machine

```mermaid
flowchart TD
    input["Parse target-local input<br/>CLI overrides environment, then defaults"] --> validate["Validate fragment, registry/choice,<br/>prefix and endpoint/client policy"]
    validate --> mode{"Read-only completion?"}
    mode -->|Yes| completionFacts["Observe sessions and projects independently"]
    completionFacts --> completion["Expose all literal names + satellite aliases<br/>No winner selection; never create or attach"]
    mode -->|No| sessions["Observe session inventory<br/>Keep canonical IDs and actual group facts"]
    sessions --> aliases["Derive searchable literal names + logical aliases"]
    aliases --> rank["Rank: exact raw name → exact alias<br/>→ shortest matching raw/alias name"]
    rank --> outcome{"Distinct workspace winners?"}
    outcome -->|Tied groups/sessions| ambiguous["Report ambiguity with literal selectors; stop"]
    outcome -->|One| entryMode{"Independent view?<br/>Default true; --group=false opts out"}
    entryMode -->|No| shared["Prepare direct attach/switch<br/>to selected canonical session ID"]
    entryMode -->|Yes| refresh["Observe fresh inventory<br/>Revalidate selected ID, name and group"]
    refresh --> allocate["Choose group logical base<br/>Allocate prefix + base + smallest free numeric suffix"]
    allocate --> grouped["Prepare ONE tmux queue:<br/>detached grouped create → handoff → keep-last"]
    outcome -->|None| projects["Observe immediate project directories"]
    projects --> projectRank["Exact name, otherwise shortest substring<br/>Tied shortest names remain ambiguous"]
    projectRank --> projectOutcome{"Project result?"}
    projectOutcome -->|Ambiguous| ambiguous
    projectOutcome -->|Existing directory| base["Prepare ordinary unprefixed base session<br/>Startup argv and initial window apply only here"]
    projectOutcome -->|Clean miss| scaffold{"Configured scaffold enabled?"}
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
```

Classified absent servers and successful empty inventories are empty session
facts. Permission, protocol, decoding, incomplete capture and other acquisition
failures are errors. Completion may suppress diagnostics for Tab, but may not
interpret a failed lookup as permission to mutate.

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
  raw names remain available. Completion and launch use one alias function.

## Independent view allocation and lifecycle

The first project entry creates an ordinary session named after the project
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
window selection instead. Low-level lowering APIs also default to grouped entry
and require a supplied satellite name for existing selections; the higher-level
workflow performs allocation. No startup command runs merely for joining a group.

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
preserves the defect evidence this contract must not erase.
