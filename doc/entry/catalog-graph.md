---
type: ArchitectureDiagram
title: Read-only catalog — executable structural graph
description: A separate bind-collect-report actor sharing the entry binding subchart and native discovery identities, with no entry decisions or mutation events
resource: /doc/entry/catalog-graph.md
status: implemented
author: model:openai/gpt-6.1-sol#high
date: 2026-11-09
updated: {by: "model:zai-coding-plan/glm-5.3#max", at: 2026-11-10}
sources:
  - resource: opencode://ses_e3cf482d5ffeyD5hefdVDZGmbe
    title: Promote the read-only catalog graph alongside the production entry architecture after the 0.8.0 cutover
  - resource: opencode://ses_e3d6e664fffewNf1Zskfd1w5gU
    title: Implement separate typed catalog querying, shared target policy and local/SSH read interpretation; project the actual composed chart
  - resource: /session/launcher/src/entry/catalog.machine.json
    title: Catalog-only collect/report states, without any delivery or mutation route
  - resource: /session/launcher/src/entry/entry.machine.json
    title: Shared bind_target subchart composed into the catalog with a collection exit
  - resource: /session/launcher/src/entry/catalog.rs
    title: Static chart composition, native query ingress, partial sources and typed projections
  - resource: /verification/entry/graph.py
    title: Structural renderer and the equivalent static subchart composition
---

# Read-only catalog: structural graph

This is the production completion/inventory path of the 0.8.0 architecture.
`CatalogQuery` is a separate retained actor with only `Observed` and `Cancel`
ingress. Its waiting type cannot yield a plan or effect. It shares the actual
`bind_target` subchart of the [entry graph](attempt-graph.md), wiring that
subchart's one validated exit to collection. `catalog.rs::definition` and the
renderer perform this static composition; no native directive assigns a state.

Each root is collected once, including after a preceding root failure. Successful
sources retain configured slots, physical roots and physical project routes.
The report contains native per-source failures; only complete root evidence may
certify bare project names. Partial results can still offer qualified selectors
and successful session aliases. The query does not select a launch winner,
generator, view policy or delivery mode.

```sh
python3 verification/entry/graph.py --catalog --write
python3 verification/entry/graph.py --catalog --check
```

<!-- entry-machine:start -->

```mermaid
flowchart TD
    catalog_query["catalog_query<br/>compound"]
    catalog_active["catalog_active<br/>compound"]
    catalog_sessions["catalog_sessions<br/>catalog_active.catalog_sessions<br/>emit read_catalog_sessions"]
    catalog_projects["catalog_projects<br/>catalog_active.catalog_projects<br/>emit read_catalog_projects"]
    bind_target["bind_target<br/>catalog_active.bind_target<br/>compound"]
    context_read["context_read<br/>catalog_active.bind_target.context_read<br/>emit read_context"]
    context_policy["context_policy<br/>catalog_active.bind_target.context_policy<br/>emit apply_context_policy"]
    endpoint_read["endpoint_read<br/>catalog_active.bind_target.endpoint_read<br/>emit read_endpoint_identity"]
    registry_read["registry_read<br/>catalog_active.bind_target.registry_read<br/>emit read_registry"]
    bind_registry["bind_registry<br/>catalog_active.bind_target.bind_registry<br/>emit validate_registry"]
    catalog_report["catalog_report<br/>emit report_catalog"]
    catalog_done["catalog_done<br/>final"]
    catalog_cancelled["catalog_cancelled<br/>final"]
    catalog_query -.->|initial| catalog_active
    catalog_active -.->|initial| bind_target
    catalog_active -->|stop| catalog_report
    catalog_active -->|cancel| catalog_cancelled
    catalog_sessions -->|catalog_sessions_observed, 1: has_roots| catalog_projects
    catalog_sessions -->|catalog_sessions_observed, 2: otherwise| catalog_report
    catalog_projects -->|catalog_root_observed, 1: more_catalog_roots, reenter| catalog_projects
    catalog_projects -->|catalog_root_observed, 2: otherwise| catalog_report
    bind_target -.->|initial| context_read
    context_read -->|observed| context_policy
    context_read -->|supplied| bind_registry
    context_policy -->|context_policy, 1: needs_endpoint_identity| endpoint_read
    context_policy -->|context_policy, 2: otherwise| registry_read
    endpoint_read -->|observed| registry_read
    registry_read -->|observed| bind_registry
    bind_registry -->|validated| catalog_sessions
    catalog_report -->|reported| catalog_done
```

<!-- entry-machine:end -->
