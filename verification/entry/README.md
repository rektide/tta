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
stop/cancel exits. A separate synthetic control-only actor checks parent-before-child
entry emission and explicit self-reentry, needed for the next stage's root loop.

The native tests use supplied observations and deliberately nonexistent cwd/socket
paths. They never invoke tmux or any filesystem observer. The first journey is
orphan-only work → semantic OpenView → delayed export request → fresh binding →
one create/enter/keep-last tmux queue. Other cases cover Shared and policy overrides,
literal/alias/group/Unicode ranking, complete-name collisions, changed anchors,
incomplete/malformed observations, cancellation, inert inspection and exact reply
correlation. Errors retain their native type, raw path and nested I/O cause.
Non-UTF-8 program/socket/config/cwd survive progression; shell-only rejection does
not destroy the native bound action. Startup argv cannot enter existing-work
operation types.

These are **not** production CLI, real-terminal, SSH, or consumer cutover tests.
Those hosts still use the old public path; their earlier evidence is not claimed
as a run of this new kernel. See the [first-stage report](../../design/entry/kernel0.gpt61sol.md)
for exact outcomes and the next commission.
