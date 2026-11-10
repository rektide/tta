---
type: VerificationToolkit
title: Runner-free emitted-probe round-trip
description: The public entry read protocol through a controlled stdlib process carrier - registry walk, actual defaults and fresh binding - without a runner, distinct from the no-I/O supplied-facts witness and real SSH evidence
resource: /verification/probes/README.md
status: implemented
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
updated: { at: 2026-11-10, by: "model:zai-coding-plan/glm-5.3#max" }
sources:
  - resource: opencode://ses_e3cf482d5ffeyD5hefdVDZGmbe
    title: Rewrite the toolkit account around the retained entry attempt after the 0.8.0 cutover
  - resource: opencode://ses_e3d19979bffe2Eu6JnXnb11i7H
    title: Migrate this witness onto fresh entry attempts, the bounded registry acquisition task and observation-only export
  - resource: opencode://ses_e804f36ddffepWs5FSeXN6VDBv
    title: Promote the independent witness without duplicating it, expose a library and CLI with retained observation reports
  - resource: opencode://ses_e8088432bffeXANJjlyh66v273
    title: GLM 5.3 max independently authored and ran the original seven-check runner-free round-trip witness
  - resource: /verification/reviews/ssh0.glm53m.md
    title: Independent findings, original results and deliberate promotion proposal
  - resource: /verification/reviews/ssh-regressions/README.md
    title: Historical reviewer-owned home with promotion note and revision binding
  - resource: /verification/core/README.md
    title: Separate no-I/O supplied-facts witness, poison and dependency-isolation evidence
  - resource: /verification/probes/src/lib.rs
    title: The actual seven-check fixture, bounded registry task and decode-cause assertions
---

# Runner-free emitted-probe round-trip

**Execute observations without linking `tta-runner`.** This maintained witness
drives the public entry protocol all the way from a frozen supplied scope →
emitted selected-shell probe → stdlib process execution → `entry::probe`
decode → the attempt's correlated reads → machine-owned registry acquisition
and selection → fresh binding → **observation-only export**. It does **not**
execute that exported action, connect to SSH, or start/query a real tmux
server.

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

- One retained attempt owns context, the registry walk through a symlink
  component **and interior `..`** (a bounded acquisition task answers the
  machine's registry request), selection and fresh binding — under
  contradictory local HOME/TTA/TMUX/author/license values.
- Every happy-path probe runs selected-shell `-l -i -c` exactly once: the
  registry walk's setup probes follow the fixture's path depth, the attempt's
  sessions/projects/destination/defaults reads each map to exactly one
  selected-shell invocation, and startup stdout and transport stderr stay
  distinct from the framed payload.
- Registry-selected native generation observes the **actual** target Git
  author/license defaults and materializes the package exactly once, with the
  literal package JSON, target program/socket and the correct final cwd
  retained in the bound action.
- Fresh binding runs a second inventory over the same frozen target and the
  export renders one selected-shell terminal handoff that is **never
  executed**; the fixture's project directory stays absent.
- A symlink with an `ENOTDIR` referent fails inside projects decoding and
  stops the actor before scaffold, alongside adversarial quoted/newline decoy
  names; invalid UTF-8 registry text fails inside the bounded registry
  acquisition **before session selection**, with the native decode cause
  retained. Neither becomes a clean miss or a ready action.

Prerequisites: a usable fixture scratch root — `TTA_TEST_TMPDIR` overrides,
then writable `/tmp/opencode`, then the system temp directory — plus POSIX
`/bin/sh`, GNU find/stat and Git at the installed Linux tool paths. Missing
tools are failures, not successful skips. The fake tmux returns exact
owned-endpoint absence; no real server is addressed. Fixture files are private
temporary directories removed on return. The stdlib carrier drains actual EOF
on both pipes but deliberately has **no general deadline/output-cap
framework**: its programs are finite controlled fixtures, not arbitrary or
untrusted target programs. Use the runner's bounded capture API for that
different job. Fish/Nu execution is not certified here.

### Promotion run

The finishing pass reran the maintained package-only hook successfully:
**7 checks, 22 captured observations**, including the expected ENOTDIR and
invalid-UTF-8 rejections, and no launch action. Report:
`/tmp/opencode/tta-probe-clever-otter.json`. The original pure witness was rebuilt
separately and its unchanged poison tool passed **4 scenarios under 2 disagreeing
environments**, with byte-identical output, no canary execution, isolated
dependencies and an available actual syscall trace:
`/tmp/opencode/tta-pure-clever-otter.json`. These are local verification results,
not a new actual SSH run. The 0.8.0 cutover reran this package's checks as part
of its final matrix; see the
[cutover receipt](../../design/entry/cutover1.gpt61sol.md) for the current
standing.

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
I/O** in its fresh-attempt exercise; its poison/syscall/dependency witness is
unchanged. This sibling intentionally performs local process/filesystem I/O,
so it does not weaken that contract. ⇄ [Actual SSH toolkit](../ssh/README.md)
owns real transport/PTY/mutation evidence. Neither toolkit implements a complete
deferred shell workflow or certifies arbitrary real-world shell startup files.
