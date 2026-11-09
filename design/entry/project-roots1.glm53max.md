---
type: ImplementationCheckpoint
title: Ordered project roots — surface integration and acceptance
description: Repeated CLI roots, whole-layer environment, identity-aware Zsh completion, actual multi-root SSH verification, the external-rekon fail-closed boundary, consumer plural adoption and configurable fixture scratch, with the executed verification matrix
resource: /design/entry/project-roots1.glm53max.md
status: implemented
author: model:zai-coding-plan/glm-5.3#max
date: 2026-11-09
sources:
  - resource: opencode://ses_e3e1e7340ffeqbKYojX8Zvxx4L
    title: Implement the commissioned multi-root integration slice end to end and verify acceptance
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commissions rotating Sol/G implementation and decides the external-rekon fail-closed boundary
  - resource: /doc/entry/project-roots.md
    title: Delivered ordered-roots contract and acceptance scope
  - resource: /design/entry/project-roots0.gpt61sol.md
    title: Establishing checkpoint whose handoff items this slice implements
  - resource: /app/cli/src/main.rs
    title: Repeated --root, whole-layer environment and identity-aware completion inventory
  - resource: /shell/zsh/functions/_tta
    title: Repeatable root forwarding collected from command words
  - resource: /session/launcher/src/scaffold.rs
    title: External-rekon multi-root creation guard
  - resource: /verification/driver/src/ssh.rs
    title: Actual loopback SSH multi-root layering, override, qualified creation checks
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Consumer plural local roots helper and embedded launch wiring
---

# Outcome and standing

**The commissioned multi-root feature is implemented and verified across every
accepted surface. The ticket's acceptance criteria are met; closing remains the
owner's decision.** This slice implements the establishing checkpoint's handoff
items 1–5 in explicit-path jj commits:

| Commit | Surface |
| --- | --- |
| `6016cf41` | Configurable fixture scratch roots (`TTA_TEST_TMPDIR` > `/tmp/opencode` when usable > system temp), restoring the previously environment-blocked fixtures |
| `b2533ec0` | Repeated CLI `--root`, whole-layer environment through the pure `configured_roots` helper, identity-aware completion inventory |
| `105adfd4` | Zsh repeatable-root forwarding and qualified-selector completion in a real PTY |
| `1083d60e` | External-rekon fail-closed multi-root creation guard |
| `17d54fc2` | Actual multi-root SSH verification and real local-tmux repeated-root launch |
| `6cf870ed` | Structural runner-free witness counts and the Python witness's scratch selection |
| `b492e7c0` | Delivered-contract documentation and this checkpoint |
| `d26064c1` | Workspace formatting reflow of the integration changes |

Consumer: `0f97bfe3` (`vnpwlnws`), "Adopt plural local project roots and
identity-aware completion".

## What was implemented

- **CLI and environment.** `--root` is repeatable and ordered. One active layer
  replaces the whole list: repeated `--root` > `TTA_ROOTS` (JSON array, UTF-8) >
  `TTA_ROOT` > HOME-relative `PROJECTS_DIR` > `$HOME/src`. Explicit roots
  suppress lower layers without parsing them; an invalid active layer fails
  before lookup. The singular layers keep raw non-UTF-8 path bytes
  (`configured_roots` now takes `OsStr` for them). SSH keeps explicit roots
  only; its defaults layer from target facts exactly as before.
- **Completion.** Project completion gathers every configured root with an
  empty filter and feeds the pure `completion_candidates` helper: a bare
  basename is offered only when launch could select it unambiguously, ambiguous
  basenames appear as qualified `@N/name` selectors, and launch-equivalent
  qualified duplicates are dropped from the line protocol so the single-root
  menu shape is preserved. The old single-root `project_names` runner helper is
  removed. The Zsh `_tta' spec is repeatable (`*--root`), and values are
  collected from the command words because `opt_args` concatenates repeated
  values with `:` — a character legal paths may contain — stopping at a bare
  `--`. The PTY harness covers first/second-root forwarding, duplicate names
  completing only qualified, and substring/wildcard matches inside selectors.
- **External-rekon boundary (parent decision).** The external `rekon` script
  ignores the selected root, so with **more than one configured root** creation
  now fails closed with an actionable error before generation — including for
  qualified `@N` selectors, which the script never sees. Single-root use keeps
  its documented divergence; `rekon-native` honors the root and is the
  suggested generator. Name/collision and destination-ambiguity guards still
  fire first; pure, runner and CLI tests verify nothing is created in any root.
- **SSH and local-tmux acceptance.** The actual loopback SSH fixture pushes a
  target-local `TTA_ROOTS` list and verifies over genuine SSH: duplicate
  basenames report `@N/name` selectors; the second root's unique project is
  found from target facts while contradictory caller values (including a
  poisoned local `TTA_ROOTS`) stay local; an explicit root list replaces the
  whole target layer; a qualified selection attaches through a real terminal;
  an unqualified clean miss refuses to choose a destination; a qualified `@2`
  creation lands in the selected root with target Git/license defaults. A local
  launch-fixture test drives the real CLI and tmux client across repeated roots
  with global shortest-basename selection.
- **Consumer adoption.** term-preserver's local launches and completion share
  one plural helper mirroring the declared precedence (`--session-root` >
  `TTA_ROOTS` > `TTA_ROOT` > `PROJECTS_DIR` > `~/src`), deliberately local to
  the consumer so completion keeps working without the `tta-lib` feature. Its
  project completion is identity-aware with the same bare/qualified rule. Its
  `--session-root` stays a single explicit override (established input
  contract); no new consumer UI was invented.
- **Fixture scratch.** `TTA_TEST_TMPDIR` overrides the harness scratch root;
  `/tmp/opencode` remains preferred when the current user can create
  directories there, otherwise fixtures fall back to the system temp directory.
  Every fixture still creates a uniquely named private directory, and the
  driver's endpoint-isolation preflight validates against the selected root.
  Two calibration fixes keep verification honest at any depth: the sshd log is
  retained as a bounded, announced tail, and the runner-free witness counts
  probes structurally (fixed workflow probes; one selected-shell per probe)
  instead of an absolute number tied to the old path depth.

## Verification actually performed

All under `TTA_TEST_TMPDIR` pointing at owned project scratch; receipts in
ignored `.test-agent/project-roots/receipts/`.

| Command / check | Result |
| --- | --- |
| `cargo check --workspace --all-targets --all-features --offline` | Passed |
| `cargo nextest run --workspace --all-features` | **196/196 passed**, including the previously skipped runner scaffold fixture, the Zsh PTY suite and the new CLI/runner/launcher tests |
| `cargo clippy --workspace --all-targets --all-features --offline --no-deps -- -D warnings`; `cargo fmt --all --check`; `cargo check -p tta-launcher --no-default-features --all-targets` | Passed (existing dependency-owned tmax warnings remain) |
| `cargo run -p tta-verify --bin tta-verify-ssh -- --tta target/debug/tta --timeout 20` | **Actual loopback SSH passed: 16 checks, 202 probes**, both multi-root terminals attaching and detaching cleanly; report retained |
| `cargo verify --backend native` and `--backend cli --inside` | **8/8 runtime PTY runs passed** (direct/shell × new/existing) |
| `./verification/probes/check.sh` and the witness test | Passed with structural counts |
| `./verification/core/check.sh`; `python3 verification/core/poison.py --trace` | Passed; pure witness `success: true` |
| term-preserver `cargo check --all-targets --all-features`; full suite | Passed; **245/245 tests**, including the formerly environment-blocked remote-classic fixture and the new TTA_ROOTS layering/identity tests |

No skipped-required tests remain. The earlier `/tmp/opencode` ownership
limitation is fully bypassed without touching another user's files; when that
directory is usable again the fixtures prefer it exactly as before.

## Boundaries respected

The eight-stage graph, core no-I/O posture, typed errors, guard ordering
(name/collision/destination ambiguity before absence requests) and the
no-push/no-install rule are unchanged. The external rekon script was not
redesigned and no adapter was silently switched; the guard is the parent's
fail-closed decision. The mixed `.beads/issues.jsonl` stays uncommitted. No
agents were spawned. Work anchor `tta-project-roots` remains open pending the
owner's close decision.
