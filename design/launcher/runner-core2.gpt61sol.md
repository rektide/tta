---
type: ArchitectureCheckpoint
title: Target setup, framed probes and prepared SSH launch implemented
description: Runner-free setup and probe interfaces, bounded capture, shared CLI and embedded SSH orchestration, actual private-loopback evidence, and the remaining mosh and deferred-compiler choices
resource: /design/launcher/runner-core2.gpt61sol.md
status: implemented-checkpoint
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e80ce13ddffeiWXfWVMBW9qXv5
    title: Fresh rotated implementer builds and verifies the SSH milestone, integrates LC1/LC2, and checkpoints after crossing the requested context threshold
  - resource: opencode://ses_e80f5d05bffe77stycHGRJOH29
    title: Parent commission, target-setup/capture/SSH architecture constraints and inherited consumer ownership
  - resource: /design/launcher/runner-core0.gpt6a.md
    title: Accepted commission and milestone outcomes
  - resource: /design/launcher/runner-core1.gpt61sol.md
    title: Actual local/core predecessor and rotation handoff
  - resource: /verification/reviews/local-core0.gpt6a.md
    title: Independent local/core evidence and LC1/LC2 toolkit corrections
  - resource: /session/launcher/src/target.rs
    title: Pure target setup, facts, options and context/registry bootstrap
  - resource: /session/launcher/src/probe.rs
    title: Pure framed probe construction and decoding with shared tmax session schema
  - resource: /session/runner/src/capture.rs
    title: Bounded complete-EOF acquisition and owned process-group cleanup
  - resource: /session/runner/src/ssh.rs
    title: Shared read-only SSH planning and prepared terminal command
  - resource: /verification/driver/src/ssh.rs
    title: Reusable actual SSH fixture, independent assertions and structured reports
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Local attempt worker and embedded SSH handoff preserving consumer transport/proxy/retry ownership
---

# Outcome

The second [commissioned milestone](runner-core0.gpt6a.md) is implemented:
target-local setup/context/registry acquisition, versioned framed probes, bounded
capture, usable SSH CLI and embedding APIs, and term-preserver's **local** embedded
SSH worker. The actual target needs **no installed tta**, including cold creation.
This is not acceptance of a deferred whole-workflow shell compiler, nor a claim
that mosh became an exec/probe channel. Work anchor remains
**tta-launch-backends-remote-target**; the parent owns ticket coordination.

## Committed stages

| Repository | Commit | Outcome |
| --- | --- | --- |
| tta | `15019779` | Shared bounded complete-EOF capture; migrate CLI lookup and Git-default acquisition |
| tta | `21a740ed` | LC1 exact endpoint preflight and retained fallback cleanup in the PTY driver; correct predecessor's coverage overstatement |
| tta | `8229b1f7` | LC2 canonical-root assertions plus adversarial pure replay and reusable poison/syscall tooling |
| tta | `cf616e94` | Pure target setup/bootstrap and framed probes; SSH runner/prepared handoff; SSH CLI; coordinated 0.4.0 and exact internal pins |
| tta | `fe236cf3` | Stop setup immediately on failed explicit environment export/unset |
| tta | `e9d46c32` | Reusable actual SSH library/CLI exercise, guarded owned endpoints, real consumer retry and toolkit guide |
| tta | `3fe1850d` | Format the fixture endpoint guard |
| tta | `cfd99ba1` | Correct find's ENOENT/ENOTDIR symlink ambiguity with framed GNU stat; add process and actual SSH regression coverage |
| tta | `79150ee2` | Bound retained sshd logs and surface cleanup log-read failures |
| term-preserver | `105a4a90` | Shared embedded SSH planning in the local worker, one SSH option set, both scaffold adapters and receiving lock |
| term-preserver | `83d9b453` | Maintained SSH ownership, retry and mosh documentation |
| term-preserver | `780a6f2c` | GNU-stat and noninteractive authentication requirements |
| tmax | `6123ab36` | Library-side account of framed SSH reuse; no tmax code/API change |

Maintained tta accounts and this checkpoint follow these stages. No push/squash/edit
was performed. The single coordinated workspace marker and exact pins were updated
directly; no publication was requested or attempted.

## Actual interfaces and invariants

- [`target::TargetSetup`](../../session/launcher/src/target.rs) owns selected
  target shell, login/interactive policy, post-startup literal environment
  overrides/unsets and invocation cwd. The same contract wraps every probe and
  action. Initial physical cwd is observed remotely and frozen thereafter.
  Startup runs anew; this is not a captured snapshot of the entire environment.
  Remote entry is **Attach** and clears TMUX/TMUX_PANE after overrides.
- `target::Facts` contains target cwd/UID and relevant environment. `Options`
  contains explicit caller choices, which win over target defaults. `Bootstrap`
  yields inspectable Context, parent Entry and Registry requests, then a workflow
  `Input`. Supplied facts can also call `Facts::input` directly. No core API
  reads local environment/filesystem or canonicalizes a target path locally.
- [`probe::Probe`](../../session/launcher/src/probe.rs) constructs scripts and
  decodes real output/status/completeness. Version-1 nonce-bound envelopes keep
  startup/trailing stdout, transport stderr, command stderr and command status
  distinct. Successful diagnostics remain available. Unknown/incomplete capture,
  nonzero carrier status, malformed/truncated/duplicate framing, typed row errors
  and command failure stop planning. Session construction/decoding reuses tmax's
  operations-only `Sessions`; exact C-locale endpoint absence is not a substring
  heuristic. Tokens separate noise, not hostile-host authentication.
- Filesystem profile: POSIX sh/Unix utilities plus **GNU find** and **GNU stat**.
  Successful NUL-delimited parent enumeration proves absence; no `ls`, false
  shell `test`, remote Python/Perl or worker binary is used. Dangling destinations
  are present; inaccessible/non-directory/looping paths fail. Registry path
  traversal is separately inspectable, including missing parents. Remote command
  operands/context/configuration require UTF-8; non-UTF-8 project basenames are
  skipped as in local matching. Supported literal newlines/quotes/content survive.
  A final edge check found that find `%Y=N` conflates ENOENT and ENOTDIR. The
  correction frames a separate single-operand GNU-stat status/mode/stderr for
  symlinks. Only status 1, no mode output and its single C-locale `cannot stat[x]`
  errno diagnostic ending `No such file or directory` classify a missing
  referent. ENOTDIR/EACCES/ELOOP and unknown messages fail closed; exact scaffold
  destination presence remains lstat-based even when its referent is broken.
- [`capture::run`](../../session/runner/src/capture.rs) retains bounded streams,
  real optional status, EOF flags, omitted counts, stop reason and cleanup
  diagnostics. Default deadline is 10 seconds, per-stream retention 256 KiB and
  cleanup grace 250ms. Child exit is not EOF. Deadlines/caps terminate only the
  acquisition's new local process group; detached/escaped remote work is not
  claimed cleaned up. No blocking reader join or fictitious complete result.
- [`ssh::prepare`](../../session/runner/src/ssh.rs) drives bootstrap and the existing
  pure workflow with read-only non-PTY SSH probes. `PreparedRemote` retains target
  facts/setup, ordinary `PreparedLaunch` including semantic `LaunchPlan`, and raw
  observation records. `Error::observations` retains failure evidence. No strings
  are parsed to rediscover mutation semantics. `command()` creates the single
  terminal/mutation handoff; no internal mutation retry/fallback/prune exists.
- CLI: `--ssh`, repeated literal `--ssh-option`, `--ssh-program`, target shell/
  login/interactive/cwd/env/unset options and explicit registry path. Remote
  launch defaults do not read local `TTA_*`/HOME/PATH/TMUX; tracing remains local.
  `--ssh --print-shell` observes read-only then emits a **resolved local SSH
  invocation**. `--exec-shell` governs replacement of the evaluating local shell.
  It does not re-run discovery when evaluated later.
- Consumer: term-preserver starts its worker locally for embedded SSH, hands it
  one serialized `ssh_control_options()` result, and uses the same caller SSH
  config/options for probes and terminal handoff. Target selected-shell setup
  is shared. The parent still owns proxy/PTY and user retry prompt. A retry is
  fresh discovery, not replay of an uncertain action or classic fallback.
  Concurrent ControlMaster work was already committed on entry and is preserved.

## Verification evidence and reusable recipes

The [SSH toolkit](../../verification/ssh/README.md) is maintained checked-in code,
not scratch-only evidence:

```sh
cargo build -p tta --all-features
cargo run -p tta-verify --bin tta-verify-ssh -- \
  --tta target/debug/tta --term-preserver ../term-preserver/target/debug/tper \
  --report /tmp/opencode/tta-ssh-report.json
./verification/core/check.sh --report /tmp/opencode/tta-core-report.json
python3 verification/core/poison.py --trace --report /tmp/opencode/tta-pure-report.json
```

Actual run report: `/tmp/opencode/tta-ssh-consumer-proud-mountain.json`.
**14 checks**, **66 directly recorded API probes**, **7 PTY scenarios** (the
consumer scenario contains two user-authorized attempts), and exactly **7 target
new-session invocations**. CLI/consumer-internal probes are additional to those
66, not falsely counted as parent-visible API records. OpenSSH client/server
**10.5p1**, system tmux **3.8**, GNU findutils **4.10.0**, GNU coreutils stat
**9.10**. Private high-port
loopback sshd, private host/client keys/config/known_hosts, exact read-only owned
absolute `-S` preflight before mutation, no system configuration changes or user
host connections. The target wrapper refuses missing/other endpoints. A second
exact absence proof and owned daemon termination are recorded at cleanup.

The post-startup target PATH excludes `/usr/local/bin`; `command -v tta` returned
**127**. Only jj is explicitly admitted from that directory. Evidence includes
cold/project and direct CLI creation, existing/grouped entry, shared window IDs
and actual keep-last option, literal/empty argv and target-relative cwd, actual
Git/jj scaffold and target Git/license defaults, failed scaffold stopping before
tmux with one attempt, malformed/failed typed probes and symlink ENOTDIR causing
no mutation, clean
print-shell stdout/evaluation, actual carrier termination after observed mutation
without automatic replay, and the real consumer proxy's detach → explicit retry
→ fresh satellite → detach → quit lifecycle.

The fixture uses a selected-shell script with recorded login/interactive argv and
controlled startup/env/cwd; it does not certify arbitrary real-world shell rc
configurations. It is genuine SSH on the same machine/filesystem/UID, not evidence
for another OS or network topology. Managed ControlMaster is explicitly disabled
in this private consumer fixture; default option construction/identity has unit
coverage, not a master-expiry/recovery claim.

Other checks: all-feature workspace nextest **147 passed**; final focused capture,
target/probe, SSH-option and pure-witness tests **20 passed**; consumer launcher unit/integration
selection **44 passed**. Deny-warnings workspace clippy, owned formatting,
workspace doctests and CLI-only all-target check passed. Consumer all-feature
all-target check, embedded-only and CLI-only all-target checks, and clippy-all
passed, retaining its preexisting compiler warning in `src/signals.rs`. The
package-only runner-free witness includes emitted probes. The reusable poison
tool passed four scenarios under two disagreeing environments with byte-identical
output, no executable canary, isolated dependencies and an actual available
strace witness. Independent local/core review evidence is trusted,
not re-labelled as a review of this new SSH implementation.

## Remaining decisions and rotation

1. **Mosh:** keep current CLI carrier usable. A concrete next design is a separate
   observation SSH configuration (destination/options/master policy) alongside
   mosh transport, sharing the same `TargetSetup`; prepare locally using SSH and
   hand only the resolved target block to mosh. `LaunchConfig.ssh` cannot simply
   be reused: it selects the transport and conflicts with mosh. Bind/validate the
   intended target identity and options explicitly before implementing this.
2. **Complete deferred shell workflow:** not implemented. It needs reviewed
   bindings, captures and branches, plus observation/error/uncertainty semantics;
   do not grow `And` into it accidentally or call resolved emission that compiler.
3. Further desired coverage can add a real managed-master lifecycle and more
   selected-shell startup profiles; current evidence states those limits honestly.
   Probe capture closes stdin and creates an owned process group; authentication
   must already work through keys/agent or an authenticated master. Interactive
   password/passphrase bootstrap is not implemented or certified here.

Context crossed the human's 200k rotation threshold during implementation and
measured about **325k** while finishing this checkpoint. The working SSH slice is
committed for parent review; rotate rather than starting another architectural
slice here. Consumer relationships:
[tta's account](../../doc/consumer/term-preserver.md) ⇄
[term-preserver's guide](file:///home/rektide/src/term-preserver/doc/launch/README.md),
and [tmax's tta account](file:///home/rektide/src/tmux-interface-control/doc/consumer/tta.md).
