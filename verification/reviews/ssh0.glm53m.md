---
type: VerificationReview
title: Independent review of the SSH remote-target milestone
description: Revision-bound review of target setup, framed probes, bounded capture, SSH CLI/embedding and the consumer handoff, plus a new runner-free probe round-trip witness
resource: /verification/reviews/ssh0.glm53m.md
status: reviewed-with-followups
author: model:zai-coding-plan/glm-5.3#max
date: 2026-10-27
sources:
  - resource: opencode://ses_e8088432bffeXANRjlyh66v273
    title: Fresh independent reviewer verifies the committed SSH milestone and adds a runner-free probe round-trip witness
  - resource: opencode://ses_e80f5d05bffe77stycHGRJOH29
    title: Parent commission and architectural review priorities for this verification
  - resource: /design/launcher/runner-core0.gpt6a.md
    title: Accepted commission, verification outcomes and declared target tool profile
  - resource: /design/launcher/runner-core2.gpt61sol.md
    title: Implemented SSH checkpoint, commit ledger and declared limits
  - resource: /verification/reviews/local-core0.gpt6a.md
    title: Predecessor local/core review carrying LC1/LC2 followups
  - resource: /verification/ssh/README.md
    title: Checked-in owned loopback SSH toolkit and its honest limits
  - resource: /session/launcher/src/target.rs
    title: Reviewed target setup, bootstrap registry walk and facts-to-input precedence
  - resource: /session/launcher/src/probe.rs
    title: Reviewed framed probe construction, GNU find/stat contracts and decoding
  - resource: /session/runner/src/capture.rs
    title: Reviewed bounded complete-EOF capture and owned process-group cleanup
  - resource: /session/runner/src/ssh.rs
    title: Reviewed read-only SSH planning, frozen cwd and prepared handoff
  - resource: /app/cli/src/main.rs
    title: Reviewed CLI SSH policy, environment precedence and diagnostics
  - resource: /verification/driver/src/ssh.rs
    title: Reviewed the actual-loopback fixture, assertions and cleanup evidence
  - resource: /verification/reviews/ssh-regressions/runner-free/src/main.rs
    title: New runner-free probe round-trip witness added by this review
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Reviewed embedded consumer worker, one option set, handoff and retry ownership
---

# Outcome

**The committed SSH milestone satisfies the commissioned architecture. No
correctness blocker was found.** Observation/error distinctions, the GNU
find/stat errno contracts, target-versus-local precedence, framed
completeness/diagnostics, no-mutation-on-error, the single grouped tmux
queue, consumer option sharing and fresh user-authorized retries are all
implemented as claimed and honestly documented. The public core/runner APIs
genuinely support a runner-free request → probe → decode → resume loop —
this review adds a committed witness proving it, which nothing checked in
proved before (existing runner-free evidence covered construction only).

One P3 diagnostics-retention deficiency and one P3 configurability gap were
found in the consumer/CLI front ends, plus three small notes. The declared
GNU find/stat + keys/agent target profile, the mosh deferral and the
deferred-workflow boundary are accepted as declared, not counted as gaps.

## Exact scope

| Repository | Reviewed revisions |
| --- | --- |
| tta | Implementation `15019779`, `21a740ed`, `8229b1f7`, `cf616e94`, `fe236cf3`, `e9d46c32`, `3fe1850d`, `cfd99ba1`, `79150ee2`; checkpoint docs `96a16de8`, `f496fbea` (ledger verified against `jj log`) |
| term-preserver | `105a4a90`, `83d9b453`, `780a6f2c` (clean tree at `780a6f2c`) |
| tmax | `6123ab36` — confirmed doc-only (`doc/consumer/tta.md`, 7 lines, no code/API change) |

Source attention: `session/launcher/src/target.rs`, `probe.rs`, `workflow.rs`,
`socket.rs`, `plan.rs`; `session/runner/src/capture.rs`, `ssh.rs`,
`driver.rs`; `app/cli/src/main.rs` + `environment.rs`;
`verification/driver/src/ssh.rs`, `isolation.rs`, `lib.rs`;
`verification/core/src/lib.rs` (LC2 assertions); term-preserver
`src/launcher.rs`, `src/launch.rs`, `doc/launch/README.md`. Broad reported
suites (147/20/44 tests, 14-check real SSH run) were trusted, not repeated.

## Findings

### S1 — P3: the consumer discards failed-prepare observation evidence

term-preserver `src/launcher.rs`, `embedded_ssh` line 411:

```rust
let prepared = tta_runner::ssh::prepare(ssh, setup, options, tta_runner::capture::Limits::default())?;
```

On failure the `?` propagates only the anyhow source chain
("SSH planning stopped before mutation: …"); `error.observations`
(`Vec<ssh::Record>`: raw stdout/stderr, carrier status, completeness/stop,
cleanup diagnostics, and every earlier probe's decoded
startup/trailing/transport/command diagnostics) is dropped. The CLI does
this correctly — `app/cli/src/main.rs` `run_ssh` calls `remote_diagnostics`
on both the Ok and Err branches. On a real remote failure (connection
refused, shell-startup errors, a tmux diagnostic), the consumer's PTY log
loses exactly the evidence the CLI would print.

**Correction:** in `embedded_ssh`, match on the prepare result and write the
records' diagnostics (and failed-record summaries) to stderr before
propagating, mirroring the CLI's `remote_diagnostics`. No behavior change
beyond diagnostics retention.

### S2 — P3: probe acquisition limits are not configurable in either front end

`app/cli/src/main.rs` line 383 and term-preserver `src/launcher.rs` line 411
both hardcode `capture::Limits::default()` (10 s deadline, 256 KiB/stream).
The plumbing exists — `ssh::prepare` takes `limits` — but no CLI flag or
consumer option reaches it. On a slow link, or a target with slow login
startup (the CLI default runs login startup for **every** probe, and without
caller `--ssh-option` ControlMaster args each probe is a fresh connection),
every probe fails closed after 10 s with "incomplete probe capture".

**Correction:** surface an SSH probe deadline (minimum: `--ssh-probe-timeout`
CLI option and a consumer launch setting), or state the fixed bound in the
SSH guide's SSH section; it is currently only recorded in the checkpoint.

### S3 — P4 (documented-bound mismatch): effective remote command-stderr bound is ~87 KiB, not 256 KiB

`probe.rs` `script()` (line 88) hex-encodes operation stderr at ~3
characters per byte (`od -An -v -tx1`), so the 256 KiB channel-stdout
retention cap (`capture.rs` `Limits::default`, line 19) is exceeded once
operation stderr passes roughly 87 KiB — long before the probe's own
786 432-character bound. Both layers fail closed (OutputCap → incomplete →
rejected), so this is a documentation-precision note, not a defect: the
checkpoint's "per-stream retention 256 KiB" describes the channel, not the
effective per-probe stderr budget. One sentence in the SSH guide would
prevent misreading. Operation stdout >256 KiB (gigantic project listings)
fails the same way.

### S4 — P4 (portability): the fixture's no-target-tta assertion is dash-specific

`verification/driver/src/ssh.rs` lines 507–518 assert
`command-v-tta-status=127`. Dash returns 127 for a failed `command -v`
(verified on this system: `/bin/sh → dash`); bash returns 1. On a
non-dash `/bin/sh` the check fails loudly (fail-closed, never silently
passes), but the expected value embeds one shell's behavior.
**Correction:** accept 127 *or* 1 with empty output, or compute the
expected status from the fixture shell itself.

### S5 — P4 (coverage limit): Fish/Nu selected-shell carriers have no execution witness

`target.rs` `shell_command` (lines 63–105) constructs fish-escaped and
nu-escaped carriers; neither fish nor nu is installed in this environment,
the real fixture's selected shell is a POSIX script, and no unit test
executes those branches. Construction is verified; execution is not. The
README's "selected-shell startup supports POSIX, Fish and Nu" is a
construction claim — fine, but the toolkit's limits paragraph should say
execution is uncertified for those families.

## Verified architecture priorities

- **Observation/error distinctions, find/stat contracts:** framing keeps
  startup stdout, trailing stdout, transport stderr, command stderr and
  command status separate; unique nonce-bound boundaries; incomplete
  capture, nonzero carrier, malformed/truncated/duplicate frames, wrong
  token and unknown rows all reject. The `%Y=N` conflation is genuinely
  fixed: only single-operand C-locale `stat` status 1 + empty mode + exact
  `cannot stat[x] …: No such file or directory` classifies ENOENT;
  ENOTDIR/EACCES/ELOOP/unknown fail closed; destination presence stays
  lstat-based. Verified against live `find`/`stat` behavior on this host
  (sentinel `/` with `%f=/`, both diagnostic forms).
- **Target-vs-local defaults, matching setup:** CLI `environment.rs`
  short-circuits all local `TTA_*`/HOME/PATH/TMUX for `--ssh` (trace only);
  the consumer skips `normalize_local`/socket-directory creation on the SSH
  path. `Facts::input` derives everything from target facts; explicit
  options win; socket conflicts bail; relative `TTA_TMUX` with `/` joins the
  frozen target cwd; `Context { tmux: None }` makes remote entry always
  Attach. Probe/handoff share one `TargetSetup`; the first observed physical
  cwd is frozen for later probes and the terminal action.
- **Bounded capture/ownership:** `capture::run` requires real EOF on both
  pipes plus exit status; child exit is not EOF; deadline/cap terminate only
  the acquisition's own process group; normal completion does not kill
  deliberately backgrounded work; stdin is null. Honest non-claims about
  remote descendants are documented.
- **No mutation on observation error; no blind replay:** all mutation lives
  behind `PreparedRemote::command()`; the workflow yields Ready only after
  successful observations; the fixture proves malformed/failed probes,
  symlink ENOTDIR and failed generation stop before tmux with inventory
  unchanged and exactly one generator attempt; carrier loss leaves the
  created session in place with no replay/rollback.
- **One tmux queue:** grouped create → attach → `keep-last` is one command
  with two `;` separators (plan tests, core replay checks, and the fixture's
  shared-window/`keep-last` independent queries agree).
- **Consumer integration:** one serialized `ssh_control_options()` result
  shared by probes and handoff; the worker stays local; a retry is a fresh
  worker with fresh discovery (fixture: detach → user retry → fresh
  satellite → detach → quit; exactly 7 recorded `new-session` invocations);
  the parent proxy survives; managed-master construction has unit coverage
  and the fixture honestly disables the master for its owned lifecycle.
- **Runner-free API usability:** proven by the new witness below — including
  the parts the shipped runner-free toolkit did not cover (Registry and
  GeneratorDefaults execution/decoding, and a registry walk through a
  symlink component and an interior `..` from `/`).
- **LC1/LC2:** both are correctly implemented — the PTY driver now records
  an exact `isolation::verify_absence` preflight before any mutation with
  approved-scratch enforcement and retained fallback-cleanup diagnostics;
  the core exercise asserts both selection and emitted `ChangeDir` against
  the supplied physical root for Project and Create.
- **Reports assert claims:** the SSH fixture records tool versions, the
  restricted target PATH with `command -v tta` proof, guarded owned
  endpoints (wrapper refuses other `-S`/`-L`), exact creation counts, and
  two absence proofs around cleanup; documentation (tta README SSH section,
  SSH toolkit README, consumer launch guide) matches the implemented
  behavior including its limits.

## Independent evidence: runner-free round-trip witness

New package [`verification/reviews/ssh-regressions/runner-free/`](ssh-regressions/runner-free/src/main.rs)
(see its [README](ssh-regressions/README.md)): depends only on
`tta-launcher` + `tta-command` + anyhow/serde_json/tempfile — `cargo tree -e
normal | grep -c tta-runner` prints **0** — and drives the complete public
loop with **stdlib** process spawning: `Bootstrap` context/registry walk →
`Probe::shell` → `/bin/sh` execution → `probe::Output` decode → supply →
`Workflow` to Ready → `TargetSetup::launch` rendering.

Result: **7 checks passed, exit 0**
(`/tmp/opencode/tta-runner-free-review.json`):

1. Bootstrap context + registry walk through a symlinked component and an
   interior `..` from `/`; contradictory local `TTA_*`/HOME/TMUX/AUTHOR/
   LICENSE never became facts (target Git author and license survived into
   the emitted `package.json`).
2. Every probe ran the selected shell with `-l -i -c`; exactly 12 probes
   (8 setup, 4 workflow).
3. Startup stdout (16 B) and startup stderr (19 B) stayed distinct
   diagnostic channels through the public decode.
4. Runner-free Create plan: target registry generator, target Git/license
   defaults, target socket/program, literal directory; one `package.json`
   write; last `ChangeDir` = target project directory.
5. `TargetSetup::launch` renders the prepared plan as one selected-shell
   handoff containing the tmux action.
6. Symlink ENOTDIR (`blocked → blocker/child`) fails the loop closed inside
   Projects decode (`cannot inspect symlink`); hostile decoy names with
   quotes/newlines parsed without poisoning the inventory.
7. Non-UTF-8 registry text fails the Registry decode inside the loop.

The witness is process-carrier evidence (real `/bin/sh`, GNU find/stat,
Git), not SSH evidence, and performs no tmux mutation (the fake tmux reports
the owned socket absent, exercising exact-endpoint classification).

## Coverage limits of this review

- No real SSH connection, sshd, or tmux server was started by this review;
  the implementer's 14-check private-loopback run was trusted. My witness
  exercises the same pure code paths through a local carrier.
- Fish/Nu carrier execution, managed-ControlMaster lifecycle/expiry, real
  Bash/Zsh startup files, interactive password bootstrap, non-UTF-8
  operands, and a different OS/network topology remain uncovered here —
  several are already declared limits of the implementer's evidence too.
- `Word::shell()` requires UTF-8, so non-UTF-8 ssh options/socket paths fail
  at probe construction with a clear error (verified in source; consistent
  with the declared UTF-8 boundaries).
- Source attention focused on the SSH milestone surface; the core runner
  split predecessor was already independently reviewed and was not re-audited.

## Corrections requested and promotion proposal

1. **S1** (consumer diagnostics): mirror the CLI's `remote_diagnostics`
   handling in `embedded_ssh` before propagating the prepare error.
2. **S2** (limits): expose the probe deadline (and ideally retention) in the
   CLI and consumer, or document the fixed bound in the SSH guide.
3. **S3/S4/S5**: one-sentence guide note on the ~87 KiB effective stderr
   budget; accept 1-or-127 in the fixture's no-target-tta check; state
   fish/nu execution as uncertified in the toolkit limits.
4. **Witness promotion:** `ssh-regressions/runner-free` is committed as a
   review-owned package. Once ownership is agreed, fold its scenarios into
   `verification/core` (which already has the right dependency shape:
   package-only, no runner) — ideally as a `probe round-trip` scenario in
   `tta-core-witness` reports, reusing its fake-shell/fake-tmux fixture
   pattern — and retire the standalone package. The symlink/`..` registry
   walk also deserves a case in `session/runner/tests/probes.rs`.

## Reproduction

```sh
# Runner-free witness (isolated target; never touches shared executables):
CARGO_TARGET_DIR=/tmp/opencode/ssh-review-target cargo run --offline --quiet \
  --manifest-path verification/reviews/ssh-regressions/runner-free/Cargo.toml \
  -- /tmp/opencode/tta-runner-free-review.json
CARGO_TARGET_DIR=/tmp/opencode/ssh-review-target cargo tree --offline \
  --manifest-path verification/reviews/ssh-regressions/runner-free/Cargo.toml \
  -e normal | grep -c tta-runner   # 0

# Empirical shell/tool contracts used during source review:
LC_ALL=C find -H / -maxdepth 1 -printf '%f\0%y\0%Y\0' | head -c 12   # "/\0d\0d\0"
/bin/sh -c 'PATH=/usr/bin:/bin; command -v missing-xyz; echo $?'     # 127 (dash)
LC_ALL=C stat -L --printf='%f' -- /nonexistent                      # cannot statx … ENOENT, rc 1
```

Reported checks trusted per commission; nothing in this review re-ran the
broad suites. Parent acceptance remains the parent's decision.
