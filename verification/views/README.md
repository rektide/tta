---
type: VerificationToolkit
title: Default independent views — two real terminals and orphan rendezvous
description: Positive runtime regression for independent current-window selection over shared tmux processes, custom prefix and orphan lookup/completion
resource: /verification/views/README.md
status: implemented
author: model:openai/gpt-6.1-sol#high
date: 2026-11-06
sources:
  - resource: opencode://ses_e4e2b4bcbffePwraSmunm9Lmav
    title: Implement and exercise real-terminal regression across both backends, both execution modes, default/custom prefix and orphan survival
  - resource: /verification/driver/src/views.rs
    title: Reusable owned-server and real-PTY positive acceptance driver
  - resource: /app/cli/tests/independent_views.rs
    title: Cargo integration matrix, native/CLI by direct/shell by default/custom prefix
  - resource: /app/cli/tests/completion.rs
    title: Actual inventory, group-versus-name identity and prefix override checks
  - resource: /shell/zsh/tests/completion.zsh
    title: Real Zsh completion widget tests for orphan aliases and custom prefixes
  - resource: /doc/entry/README.md
    title: Input-state declaration and precise identity/ranking/allocation contract
---

# Positive independent-view acceptance

Unlike the [historical wrong-client reproduction](/verification/trials/independent-views/README.md),
success here means the desired **outside** behavior holds. No assertion treats
moving the wrong inside client as success. The separate inside-origin limitation
remains documented in the [entry contract](/doc/entry/README.md#explicit-boundary-inside-client-origin).

## Run

```sh
cargo build -p tta --all-features
cargo verify --backend native --independent-views --report /tmp/opencode/tta-views-native.json
cargo verify --backend cli --independent-views --report /tmp/opencode/tta-views-cli.json
cargo verify --backend native --independent-views --satellite-prefix view-
cargo verify --backend cli --independent-views --satellite-prefix view-
cargo nextest run -p tta --all-features --test independent_views --test completion
```

The driver uses real terminal clients in private PTYs, not control-mode client
substitutes. Each run preflights exact endpoint absence, owns a unique socket and
project fixture under `/tmp/opencode`, bounds external commands and terminal
phases, and stops only its owned clients/server. JSON retains actual tmux version,
session/client IDs, window/pane/process IDs, checks, completion text, transcripts,
failure and cleanup evidence. `tta_verify::views::exercise` is the reusable hook.

The integration tests run eight combinations: native/CLI × direct/evaluated-shell
× default `_`/custom `view-`. CLI-only builds run their four applicable cases.
Missing tmux or failed cleanup fails the regression rather than becoming a skip.

## What each case proves

1. Plain cold entry creates `verify-project`; plain second entry creates a
   different session over **identical window IDs, pane IDs and running PIDs**.
   Both attached clients are real terminal clients. Selection in either view
   leaves the other view's current window unchanged.
2. A third entry with `--group=false` shares the first session's current window;
   the independent satellite remains unaffected. Detach removes that client.
3. Rename the satellite to the configured prefix plus `verify-project-42`, remove
   the base session and detach the last client. The orphan and all original work
   survive. Read-only completion includes `verify-project` and the raw selector.
4. Plain `tta verify-project`, with a deliberately unusable project root, resumes
   those processes through a new independent view. The name is
   `<prefix>verify-project-1`, never recursively decorated. Final detach leaves
   one session preserving the work.

Complementary tests establish that multiple aliases in one **actual group** are
one match, unrelated groups with the same alias remain ambiguous, explicit raw
selectors disambiguate, and fresh selected facts changing during preparation
fail closed. Zsh's actual completion widget inserts `orphan` from `_orphan-42`,
respects `--satellite-prefix view-` and still completes explicit decorated input.
The [SSH toolkit](/verification/ssh/README.md) exercises the same outside default,
target-prefix environment and explicit override through genuine loopback SSH
without target tta. Existing lifecycle, argv, scaffold and runner-free checks
remain separate evidence rather than inferred from the two-terminal result.

## Scope

This proves independent **session/window selection**, not independent active
panes within the same window or copies of processes. It does not implement
inside-client origin inference, guarded native switching, atomic allocation,
automatic pruning, multi-root discovery or a new terminal client implementation.
Snapshot races remain deliberate failures, not permission to create new work.

## Recorded verification — 2026-11-06

On **tmux 3.8**, all eight real-terminal combinations passed, including orphan
resumption and explicit shared attach. Genuine private loopback SSH passed all
**15** assertions, including default independent entry, target-prefix inheritance,
explicit override, shell emission, scaffold failure and transport-loss boundaries.

One native/direct witness: clients **3770054** (`$0`) and **3770086** (`$1`)
shared `@0 / %0 / PID 3770063` and `@1 / %1 / PID 3770074`. Independent
`list-clients` observations, not launcher traces, recorded:

| Selection action | Client A window | Client B window |
| --- | --- | --- |
| B selects second window | `@0` | `@1` |
| A selects second, B returns to first | `@1` | `@0` |

The surviving `_verify-project-42` inventory offered `verify-project`; resumption
retained those original window/pane/PID tuples. Owned-server cleanup succeeded.

Workspace checks passed: **163/163** all-feature nextest tests, **147/147** default
native tests, **130/130** CLI-only tests, all-feature doctests, all-feature and
CLI-only clippy (`--no-deps -- -D warnings`), explicit-package formatting, Zsh
syntax, and both runner-free verification scripts. The term-preserver receiving
consumer passed all-target/all-feature checking and **9/9** launcher tests; its
explicit grouping policy is unchanged. Dependency-owned tmax warnings remain.

Full workspace nextest runs used `-j 1`: an earlier parallel probe run failed
closed when an unrelated sibling temporary directory disappeared during parent
enumeration. This is retained as a fixture/acquisition race, not reclassified as
absence or suppressed in production. The final complete serial runs passed.

JSON reports were retained under `/tmp/opencode/tta-views-{native,cli}.json`,
`tta-views-custom-{native,cli}.json`, `tta-independent-ssh.json`,
`tta-independent-core.json` and `tta-independent-probes.json`. These are ephemeral
run artifacts, not stable source paths; the session source and rerun toolkit
above carry the durable account.
