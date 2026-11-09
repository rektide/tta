---
type: ArchitectureDiagram
title: Retained entry attempt — executable structural graph
description: Generated states, directives, initial edges and ordered transitions of the staged internal replacement, not the still-public legacy workflow
resource: /doc/entry/attempt-graph.md
status: implemented-internal
author: model:openai/gpt-6.1-sol#high
date: 2026-11-09
sources:
  - resource: opencode://ses_e3d9d71bfffe3f2KznncOc1djO
    title: Implement the approved first-stage retained actor, verify emission and guards, and generate this structural projection
  - resource: /session/launcher/src/entry/entry.machine.json
    title: Sole executable owner of entry-attempt transitions
  - resource: /session/launcher/src/entry/machine.rs
    title: Closed native directive vocabulary and retained xmachines adapter
  - resource: /verification/entry/graph.py
    title: Structural renderer preserving event-array guard order
---

# Retained entry attempt: structural graph

This is the **internal replacement**, not the public `Workflow` contract in
[the current entry account](README.md). The [approved redesign and first-stage
report](/design/entry/kernel0.gpt61sol.md) explain scope, evidence and remaining
work. There are no effect requests yet: the complete existing-work journey ends
in an exported snapshot after fresh binding. `project_scope` deliberately stops
with a typed not-yet-implemented outcome until the discovery stage replaces it.

Generated from the actual JSON, not copied from the design sketch. Node labels
show IDs, hierarchical paths and state-entry directives. Dotted initial edges
enter compounds; solid edges show actual event routes. Numbers retain guard
priority. `active` owns common `stop`/`cancel` routes inherited by its descendants.
Native errors are stored before `stop`, never serialized into the actor.

```sh
python3 verification/entry/graph.py --write
python3 verification/entry/graph.py --check
```

Only the marked region is regenerated. The renderer intentionally fails on entry
action types it does not understand. It is not an execution engine or proof of
the guarded matching algorithm; [the kernel report](/design/entry/kernel0.gpt61sol.md#guarded-session-resolution)
and the native tests cover that separate contract.

<!-- entry-machine:start -->

```mermaid
flowchart TD
    entry_attempt["entry_attempt<br/>compound"]
    active["active<br/>compound"]
    bind_registry["bind_registry<br/>active.bind_registry<br/>emit validate_registry"]
    select["select<br/>active.select<br/>compound"]
    sessions_read["sessions_read<br/>active.select.sessions_read<br/>emit read_selection_sessions"]
    sessions_rank["sessions_rank<br/>active.select.sessions_rank<br/>emit rank_sessions"]
    project_scope["project_scope<br/>active.select.project_scope<br/>emit begin_projects"]
    prepare_view["prepare_view<br/>active.prepare_view<br/>compound"]
    view_policy["view_policy<br/>active.prepare_view.view_policy<br/>emit settle_view"]
    open_view["open_view<br/>active.prepare_view.open_view<br/>emit plan_open_view"]
    enter_view["enter_view<br/>active.prepare_view.enter_view<br/>emit plan_enter_view"]
    planned["planned<br/>active.planned"]
    bind_action["bind_action<br/>active.bind_action<br/>compound"]
    binding_read["binding_read<br/>active.bind_action.binding_read<br/>emit read_binding_sessions"]
    binding_check["binding_check<br/>active.bind_action.binding_check<br/>emit check_binding"]
    binding_lower["binding_lower<br/>active.bind_action.binding_lower<br/>emit bind_operation"]
    export_action["export_action<br/>active.export_action<br/>emit export_action"]
    exported["exported<br/>final"]
    stopped["stopped<br/>final"]
    entry_attempt -.->|initial| active
    active -.->|initial| bind_registry
    active -->|stop| stopped
    active -->|cancel| stopped
    bind_registry -->|validated| select
    select -.->|initial| sessions_read
    sessions_read -->|observed| sessions_rank
    sessions_rank -->|unique| prepare_view
    sessions_rank -->|empty| project_scope
    prepare_view -.->|initial| view_policy
    view_policy -->|view_policy, 1: independent| open_view
    view_policy -->|view_policy, 2: otherwise| enter_view
    open_view -->|prepared| planned
    enter_view -->|prepared| planned
    planned -->|export| bind_action
    bind_action -.->|initial| binding_read
    binding_read -->|observed| binding_check
    binding_check -->|checked| binding_lower
    binding_lower -->|bound| export_action
    export_action -->|exported| exported
```

<!-- entry-machine:end -->
