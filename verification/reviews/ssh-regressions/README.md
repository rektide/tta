---
type: VerificationToolkit
title: SSH review regression witnesses
description: Historical reviewer-owned runner-free witness and its deliberate promotion into the maintained probes toolkit
resource: /verification/reviews/ssh-regressions/README.md
status: review-witness-promoted
author: model:zai-coding-plan/glm-5.3#max
date: 2026-10-27
sources:
  - resource: opencode://ses_e804f36ddffepWs5FSeXN6VDBv
    title: Deliberately move the single round-trip implementation to verification/probes and preserve this reviewer-owned historical account
  - resource: opencode://ses_e8088432bffeXANRjlyh66v273
    title: Independent SSH milestone review adds a runner-free probe round-trip witness and records it for deliberate promotion
  - resource: /verification/reviews/ssh0.glm53m.md
    title: Owning review report, findings and promotion proposal
---

# SSH review regression witnesses

## Promotion — 2026-10-27

The original `runner-free/` source is now the maintained
[`verification/probes` library/CLI](../../probes/README.md), with one implementation,
retained reviewer attribution, package-only dependency isolation and richer
observation reports. Its standalone manifest/lock were deliberately retired.
Use `./verification/probes/check.sh --report /tmp/opencode/tta-probe-report.json`
from the repository root. The original package and result are preserved at
**`a65eac3a`**; the recipes below describe that revision, not today's paths.
The no-I/O supplied-facts witness remains separate and unchanged.

## Original review-owned account (at `a65eac3a`)

Witnesses added by [independent review](../ssh0.glm53m.md), kept deliberately
separate from the shipped `verification/core` and `verification/driver`
packages because their ownership is not yet agreed. Each is a standalone
cargo package (its own `[workspace]`), is not a member of the tta workspace,
and builds into an isolated target directory so shared executables are never
overwritten.

## `runner-free/` — public-API probe round-trip without tta-runner

Proves the milestone's runner-free contract end to end: a caller depending
only on `tta-launcher` + `tta-command` (never `tta-runner`) can construct
setup requests, emit the framed probe scripts through a selected shell,
execute them with **stdlib** process spawning, decode the framed replies
through the public `probe::Output` contract, and resume `target::Bootstrap`
and `workflow::Workflow` to a ready launch plan.

New coverage relative to the checked-in toolkit:

- the registry walk rooted at `/` through a **symlinked component and an
  interior `..`**, not just a plain parent directory;
- `Registry` and `GeneratorDefaults` probes, which had no process-carrier
  coverage (only the real SSH fixture exercised them);
- contradictory local environment values never becoming target facts;
- symlink-`ENOTDIR` and non-UTF-8 registry content failing closed inside the
  same loop.

```sh
CARGO_TARGET_DIR=/tmp/opencode/ssh-review-target cargo run --offline --quiet \
  --manifest-path verification/reviews/ssh-regressions/runner-free/Cargo.toml \
  -- /tmp/opencode/tta-runner-free-review.json
# 7 checks; nonzero exit on any failure. Dependency isolation:
CARGO_TARGET_DIR=/tmp/opencode/ssh-review-target cargo tree --offline \
  --manifest-path verification/reviews/ssh-regressions/runner-free/Cargo.toml \
  -e normal | grep -c tta-runner   # must print 0
```

It is process-carrier evidence (real `/bin/sh`, GNU find/stat, Git), **not**
SSH evidence; the [actual SSH toolkit](../../ssh/README.md) owns that. The
happy-path run performs no tmux mutation (the fake tmux classifies the owned
socket as absent). Promotion proposal and correction recommendations live in
the [review report](../ssh0.glm53m.md). Current reruns use the
[promoted toolkit](../../probes/README.md), which links back here.
