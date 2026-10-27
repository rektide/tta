---
type: VerificationToolkit
title: Runner-free emitted-probe round-trip
description: Reusable public-core Bootstrap and Workflow loop through a controlled stdlib process carrier, distinct from the no-I/O supplied-facts witness and real SSH evidence
resource: /verification/probes/README.md
status: implemented
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e804f36ddffepWs5FSeXN6VDBv
    title: Promote the independent witness without duplicating it, expose a library and CLI with retained observation reports, and preserve the pure supplied-facts contract separately
  - resource: opencode://ses_e8088432bffeXANRjlyh66v273
    title: GLM 5.3 max independently authored and ran the original seven-check runner-free round-trip witness
  - resource: /verification/reviews/ssh0.glm53m.md
    title: Independent findings, original results and deliberate promotion proposal
  - resource: /verification/reviews/ssh-regressions/README.md
    title: Historical reviewer-owned home with promotion note and revision binding
  - resource: /verification/core/README.md
    title: Separate no-I/O supplied-facts replay, poison and dependency-isolation witness
---

# Runner-free emitted-probe round-trip

**Execute observations without linking `tta-runner`.** This maintained witness
drives the public core all the way from `target::Bootstrap` → emitted selected-shell
probe → stdlib process execution → `probe::Output` decode → supply/resume →
`workflow::Workflow::Ready` → rendered `TargetSetup::launch` commands. It does
**not** execute that final action, connect to SSH, or start/query a real tmux server.

```sh
# Package-only dependency isolation, the seven-check test, and a JSON report:
./verification/probes/check.sh --report /tmp/opencode/tta-probe-report.json

# Individual hooks (use the package-only graph, not workspace --all-features):
cargo run -p tta-probe-witness --no-default-features -- \
  --report /tmp/opencode/tta-probe-report.json
cargo tree -p tta-probe-witness --no-default-features -e normal,features
```

`tta_probe_witness::exercise()` returns a structured `Report` on success **or
failure**. The CLI writes it before returning a nonzero failure status. Reports
retain checks, tool versions, each request, actual carrier status/completeness,
decoded reply/diagnostic byte counts or error, and `action_executed: false`.
`check.sh` rejects `tta-runner`, native tmax client/control features and their
`nix`/`bytes` dependencies in the package-only graph. Unlike the pure witness,
this package uses `tempfile` (and its filesystem dependency `rustix`) for real
owned fixtures. There is still no runner-backed execution.

## What the seven checks establish

- Context and registry acquisition walk from `/` through a symlink component
  **and interior `..`**, with contradictory local HOME/TTA/TMUX/author/license.
- Every happy-path probe runs selected-shell `-l -i -c`: 12 probes (8 setup,
  4 workflow), with startup stdout and transport stderr kept distinct.
- Registry-selected native generation observes target Git author/license defaults
  and emits literal package JSON, target program/socket, and the correct final cwd.
- Ready launch commands render as one selected-shell terminal handoff.
- A symlink with an `ENOTDIR` referent fails inside Projects decoding, alongside
  adversarial quoted/newline decoy names; invalid UTF-8 registry text fails inside
  Registry decoding. Neither becomes a clean miss or a ready action.

Prerequisites: writable `/tmp/opencode`, POSIX `/bin/sh`, GNU find/stat and Git
at the installed Linux tool paths. Missing tools are failures, not successful
skips. The fake tmux returns exact owned-endpoint absence; no real server is
addressed. Fixture files are private temporary directories removed on return.
The stdlib carrier drains actual EOF on both pipes but deliberately has **no
general deadline/output-cap framework**: its programs are finite controlled
fixtures, not arbitrary or untrusted target programs. Use the runner's bounded
capture API for that different job. Fish/Nu execution is not certified here.

### Promotion run

The finishing pass reran the maintained package-only hook successfully:
**7 checks, 22 captured observations**, including the expected ENOTDIR and
invalid-UTF-8 rejections, and no launch action. Report:
`/tmp/opencode/tta-probe-clever-otter.json`. The original pure witness was rebuilt
separately and its unchanged poison tool passed **4 scenarios under 2 disagreeing
environments**, with byte-identical output, no canary execution, isolated
dependencies and an available actual syscall trace:
`/tmp/opencode/tta-pure-clever-otter.json`. These are local verification results,
not a new actual SSH run.

## Ownership and complementary evidence

Original author: **GLM 5.3 max**, session
`ses_e8088432bffeXANRjlyh66v273`, committed at **`a65eac3a`** under
`verification/reviews/ssh-regressions/runner-free/`. This is a deliberate source
move into the maintained verification domain, not a second implementation or a
rewrite of the independent findings. The obsolete standalone manifest/lock were
retired; their exact state remains at that commit. The
[historical home](../reviews/ssh-regressions/README.md) links here; the
[review](../reviews/ssh0.glm53m.md#independent-evidence-runner-free-round-trip-witness)
retains its original seven-check result and adds the promotion record.

⇄ [Supplied-facts core witness](../core/README.md) still performs **no runtime
I/O** in its planning/replay exercise; its poison/syscall/dependency witness is
unchanged. This sibling intentionally performs local process/filesystem I/O,
so it does not weaken that contract. ⇄ [Actual SSH toolkit](../ssh/README.md)
owns real transport/PTY/mutation evidence. Neither toolkit implements a complete
deferred shell workflow or certifies arbitrary real-world shell startup files.
