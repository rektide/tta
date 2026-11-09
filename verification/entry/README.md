---
type: VerificationGuide
title: Retained entry kernel verification
description: Focused retained-actor, request-correlation, native-data and existing-work witnesses plus structural chart generation
resource: /verification/entry/README.md
status: maintained
author: model:openai/gpt-6.1-sol#high
date: 2026-11-09
sources:
  - resource: opencode://ses_e3d9d71bfffe3f2KznncOc1djO
    title: Implement and exercise the internal existing-work kernel without replay or legacy resolver calls
  - resource: opencode://ses_e3d8804a3ffez4iETRd6llQTnL
    title: Extend the focused suite with machine-owned discovery, creation and new-work binding journeys
  - resource: /session/launcher/src/entry/tests.rs
    title: Supplied-facts journeys, typed-error/byte boundaries and protocol guards
  - resource: /session/launcher/src/entry/machine.rs
    title: Actual runtime emission, hierarchy, ordered guard and self-reentry witnesses
  - resource: /verification/entry/graph.py
    title: Executable-chart projection and read-only freshness check
---

# Retained entry kernel checks

Run from the tta checkout:

```sh
cargo nextest run -p tta-launcher --all-features -E 'test(entry::)'
cargo nextest run -p tta-launcher --no-default-features -E 'test(entry::)'
cargo clippy -p tta-launcher --all-targets --all-features --no-deps -- -D warnings
cargo fmt -p tta-launcher --check
python3 verification/entry/graph.py --check
python3 -B -m unittest discover -s verification/entry -p 'test_*.py'
```

The [internal chart](../../doc/entry/attempt-graph.md) is a projection of the JSON;
regenerate it with `python3 verification/entry/graph.py --write`. This changes only
its marked diagram region, preserving maintained explanation and provenance.
Tests deserialize every directive and exhaustively dispatch its Rust enum. They
also run the actual retained actor through guard alternatives and inherited
stop/cancel exits, and through the machine-owned discovery course: session miss,
scope, per-root read reentry on the remaining-root summary, the creation ladder
and the new-work binding evidence read. A separate synthetic control-only actor
checks parent-before-child entry emission and explicit self-reentry.

The native tests use supplied observations and deliberately nonexistent cwd/socket
paths. They never invoke tmux or any filesystem observer. The existing-work
journey is orphan-only work → semantic OpenView → delayed export request → fresh
binding → one create/enter/keep-last tmux queue. The discovery journeys cover
qualified `@N` single-root observation, the multi-root read loop before global
ranking, distinct-physical ambiguity, equivalent-route identity collapse with
retained provenance, missing/malformed root evidence stopping natively, wrong
observation-kind/slot/path replies rejected before the pending request is
consumed, typed missing and scaffold-registry guidance, destination absence
preceding adapter work, required-default observation with one materialization,
the external rekon multi-root refusal through its declared destination contract,
fresh name/route/destination binding checks, and new-work lowering with literal
startup argv, window policy and directory overrides. Non-UTF-8 bytes and native
error chains survive as in the kernel stage. Errors retain their native type,
raw paths and nested I/O causes.

These are **not** production CLI, real-terminal, SSH, or consumer cutover tests.
Those hosts still use the old public path; their earlier evidence is not claimed
as a run of this new kernel. See the [kernel](../../design/entry/kernel0.gpt61sol.md)
and [discovery](../../design/entry/discovery0.glm53m.md) reports for exact
outcomes and the next commission.
