---
type: VerificationGuide
title: Entry attempt kernel and host verification
description: Focused retained-actor, native data, target correlation, runner-free poison, local/probe agreement and complete/partial catalog witnesses plus structural chart generation
resource: /verification/entry/README.md
status: maintained
author: model:openai/gpt-6.1-sol#high
date: 2026-11-09
sources:
  - resource: opencode://ses_e3d6e664fffewNf1Zskfd1w5gU
    title: Extend verification through machine-owned context, shared local/SSH interpreters, poison isolation and read-only catalogs
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
  - resource: /session/launcher/src/entry/catalog.rs
    title: Typed read-only catalog, partial source and complete-root uniqueness witnesses
  - resource: /session/launcher/tests/entry_purity.rs
    title: Runner-free supplied-facts journeys under poisoned commands and invoking-host environment
  - resource: /session/runner/tests/entry_hosts.rs
    title: Actual local/probe agreement, one read driver, frozen target guard and bounded capture/diagnostic evidence
---

# Entry attempt checks

Run from the tta checkout:

```sh
cargo nextest run -p tta-launcher --all-features -E 'test(entry::)'
cargo nextest run -p tta-launcher --no-default-features -E 'test(entry::)'
cargo clippy -p tta-launcher -p tta-runner --all-targets --all-features --no-deps -- -D warnings
cargo fmt -p tta-launcher -p tta-runner --check
python3 verification/entry/graph.py --check
python3 verification/entry/graph.py --catalog --check
python3 -B -m unittest discover -s verification/entry -p 'test_*.py'
```

The [entry chart](../../doc/entry/attempt-graph.md) is a projection of the JSON;
regenerate it with `python3 verification/entry/graph.py --write`. This changes only
its marked diagram region, preserving maintained explanation and provenance.
The separate [catalog chart](../../doc/entry/catalog-graph.md) composes the actual
shared binding subtree with its read-only collection/report course; regenerate
with `python3 verification/entry/graph.py --catalog --write`.
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

## Host and catalog checks

`entry-internal` exposes the real staged core across the existing crate boundary,
without a production compatibility facade or release-version bump. For the full
focused host suite, choose a writable scratch root (the preferred
`/tmp/opencode` is not writable in the implementation environment):

```sh
mkdir -p .test-agent/entry-hosts
TTA_TEST_TMPDIR="$PWD/.test-agent/entry-hosts" cargo nextest run \
  -p tta-launcher -p tta-runner --all-features \
  -E 'test(entry::) | binary(entry_purity) | binary(entry_hosts)'
TTA_TEST_TMPDIR="$PWD/.test-agent/entry-hosts" cargo nextest run \
  -p tta-launcher -p tta-runner --no-default-features \
  --features tta-launcher/entry-internal,tta-runner/entry-internal \
  -E 'test(entry::) | binary(entry_purity) | binary(entry_hosts)'
```

The pure subprocess witness supplies nonexistent target paths and raw facts under
a poisoned executable PATH and invoking-host environment. Catalog tests exercise
complete/partial physical uniqueness, qualified selectors, workspace aliases,
native source chains, malformed inventories, inert ingress rejection, cancellation
and a closed mutation-incapable chart. Read-only queries can inventory another
server without authorizing an inherited-client switch.

Host tests execute worker-free scripts and compare local results for canonical
roots/routes, symlinks, argument-filtered failures, exact destination absence,
required defaults and bounded registry acquisition. They drive both local supplied
contexts and frozen SSH-observed contexts through the same read loop, reject
host/setup/attempt/endpoint/program mismatches before I/O, preserve raw target
environment bytes before policy, and retain capped/incomplete captures and native
decode error types. The SSH interpreter is exercised through a literal-argv shell
carrier with startup noise; no target tta is needed. Existing `runner --test probes`
witnesses also pass after factoring shared framing mechanics.

These are **not** production CLI, real-terminal, authenticated loopback SSH, or consumer cutover tests.
Those hosts still use the old public path; their earlier evidence is not claimed
as a run of this new protocol. See the [kernel](../../design/entry/kernel0.gpt61sol.md),
[discovery](../../design/entry/discovery0.glm53m.md) and
[hosts](../../design/entry/hosts0.gpt61sol.md) reports for exact outcomes and the
next G execution/cutover commission.
