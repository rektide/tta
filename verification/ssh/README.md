---
type: VerificationToolkit
title: Owned loopback SSH launch verification
description: Reusable actual OpenSSH exercise with private sshd keys and configuration, no target tta, independently observed tmux outcomes and structured cleanup evidence
resource: /verification/ssh/README.md
status: implemented
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e4e2b4bcbffePwraSmunm9Lmav
    title: Verify the independent SSH default, target-prefix environment and explicit override, and update expected mutation counts
  - resource: opencode://ses_e804f36ddffepWs5FSeXN6VDBv
    title: Correct the POSIX absence-status assertion and document encoded capture budgets and uncertified Fish/Nu execution
  - resource: opencode://ses_e80ce13ddffeiWXfWVMBW9qXv5
    title: Implement and run the actual loopback SSH toolkit, target setup/framing and embedded consumer handoff
  - resource: /design/launcher/runner-core0.gpt6a.md
    title: Commissioned verification outcomes and process-carrier versus actual SSH distinction
  - resource: /verification/reviews/local-core0.gpt6a.md
    title: LC1 exact endpoint proof and retained fallback-cleanup requirements
  - resource: /verification/driver/src/ssh.rs
    title: Actual reusable SSH library exercise and owned fixture lifecycle
  - resource: /session/runner/tests/probes.rs
    title: Separate process-carrier framing, hostile filename and error checks
---

# Owned loopback SSH verification

This is **actual `/usr/bin/ssh` → private `/usr/sbin/sshd`**, not a shell standing
in for SSH. It contacts only its exclusively owned high loopback port. Private
host/client keys, authorized keys, known_hosts and both configs live beneath a
unique fixture directory in the owned scratch root (`TTA_TEST_TMPDIR`, then
`/tmp/opencode` when writable, then the system temp directory). It does not
change system SSH config, use user host connections, or address the user's
default tmux server.

```sh
cargo build -p tta --all-features
cargo run -p tta-verify --bin tta-verify-ssh -- \
  --tta target/debug/tta --report /tmp/opencode/tta-ssh-report.json

# Also exercise the real term-preserver proxy and local attempt worker:
cargo build --manifest-path ../term-preserver/Cargo.toml --all-features --bin tper
cargo run -p tta-verify --bin tta-verify-ssh -- \
  --tta target/debug/tta --term-preserver ../term-preserver/target/debug/tper \
  --report /tmp/opencode/tta-ssh-consumer-report.json
```

Library hook: [`tta_verify::ssh::exercise`](../driver/src/ssh.rs) takes executable
paths and a per-phase timeout and returns a structured `Report`, including on
failure. The command exits nonzero for failed assertions or cleanup. Prerequisites
are OpenSSH client/server/key tools, GNU find/stat, tmux 3.4+, Git, and
`/usr/local/bin/jj` for the real native scaffold. Missing prerequisites fail; they
are not successful skips. This fixture currently targets a non-root Unix account
with noninteractive private-key authentication; it does not test an interactive
password/passphrase bootstrap. Probe capture has closed stdin and an owned process
group, so use keys/agent or an authenticated master for this acquisition path.
It uses the installed Linux tool locations. No Python/Perl/tta worker is required on
the target. The target remains the same machine/filesystem/UID behind genuine
SSH; this is not evidence for a different operating system or network topology.

## What is checked

- **Before mutation:** shared [`isolation::verify_absence`](../driver/src/isolation.rs)
  validates complete read-only `tmux -S OWNED_ABSOLUTE_PATH list-sessions` failure
  naming exactly the owned endpoint. Both cold and existing fixture setup obey it.
- **No target tta:** after selected-shell setup, target PATH contains only a private
  tool directory and `/usr/bin:/bin`. `command -v tta` must return **1 or 127**
  with empty command output after the recorded selected-shell startup; the
  installed `/usr/local/bin/tta` is unreachable. Only jj is deliberately admitted
  from that directory. The target-shell fixture records login/interactive argv,
  applies private startup/environment/cwd and emits intentional diagnostics; it
  is not a claim to test every real-world Bash/Zsh/Fish/Nu startup configuration.
  Fish and Nu are not installed: their carrier construction, **not execution**,
  is covered. The historical actual SSH run returned **127** on dash; accepting
  Bash's **1** does not revise that recorded observation.
- **Cold/project:** embedded runner preparation, target HOME/root/config/cwd versus
  contradictory local values, literal quoted/empty startup arguments and relative
  `--dir`, observed independently through target artifacts and tmux queries.
- **Multi-root:** target-local `TTA_ROOTS` layering versus contradictory caller
  values, an explicit repeated root list replacing the whole target layer,
  duplicate-basename ambiguity reporting, a qualified `@2` selection attaching
  through a real terminal, an unqualified clean miss refusing to choose a
  destination, and a qualified creation landing in the selected root with
  target Git/license defaults.
- **Existing/grouped by default:** fresh suffix selection, one create → attach → `keep-last`
  command queue, shared window IDs, actual option value and detach lifecycle.
  Target `TTA_SATELLITE_PREFIX` and an explicit prefix override are both exercised
  over genuine SSH. Contradictory local prefix variables do not leak remotely.
- **Scaffold:** real Git/jj native generation with target registry/Git-author/license
  defaults; injected generator failure stops before tmux/handoff and runs once.
- **Read failures:** malformed typed session output and nonzero command diagnostics
  over real SSH stop planning without mutation or backend fallback.
  A symlink whose referent has a non-directory ancestor is also rejected before
  scaffolding: GNU find's ambiguous `%Y=N` cannot manufacture a clean miss.
- **CLI:** read-only `--ssh --print-shell` emits an evaluable **local SSH invocation**
  with no stdout startup chatter; direct CLI cold creation uses target defaults.
- **Transport loss:** stop an owned SSH carrier after independently observing its
  created session. The session persists; no automatic replay/rollback occurs.
- **Consumer (optional):** real term-preserver proxy/PTY → local embedded attempt
  worker → SSH, detach, explicit user retry with fresh grouped discovery, detach
  and quit. Its managed master is explicitly disabled for this fixture's private
  lifecycle; shared option identity and default ControlMaster construction have
  separate unit coverage. This does not certify master expiry/reconnection.

Reports retain versions, fixture/endpoint, observations with framing/status and
diagnostics, PTY transcripts, actual inventories/artifacts, target tmux argv counts,
sshd log, endpoint cleanup and daemon termination. Only the proven owned server is
stopped; a second read-only exact-absence check records cleanup. Fallback cleanup
prints its evidence instead of discarding diagnostics. Private files are removed
afterward; reports retain paths as evidence, not durable usable credentials.

## Complementary checks and limits

Front-end acquisition defaults are 10000 milliseconds per probe and 262144
retained bytes per channel stream; both are configurable as documented in the
[SSH guide](../../README.md#ssh-targets). The byte cap is on **encoded** output:
operation stderr travels in stdout as hex at roughly 3× its decoded size, so
about 87 KiB alone can fill the default stdout budget (less with other output).
The independent remote metadata ceiling is 786432 characters. Incomplete/capped
captures fail closed; retention is not a promised decoded-diagnostic allowance.

```sh
cargo nextest run -p tta-runner --all-features \
  -E 'test(capture::tests) | binary(probes) | test(ssh::tests)'
./verification/core/check.sh --report /tmp/opencode/tta-core-report.json
python3 verification/core/poison.py --trace --report /tmp/opencode/tta-pure-report.json
```

The first command is **process-carrier**, not SSH evidence: complete EOF,
pipe-holding descendants, output caps, retained nonzero status, unrelated-process
survival, startup chatter, malformed/truncated/wrong-token envelopes, hostile
filenames, dangling-link collisions and exact endpoint classification. The
symlink tests distinguish missing targets from non-directory ancestors, loops and
permission-denied parents, including quoted/newline names and registry traversal.
The GNU-stat classifier requires its single-operand C-locale diagnostic, status 1
and empty mode output; unknown errors fail closed rather than becoming absence.
The [supplied-facts toolkit](../core/README.md) constructs probes without executing
them. ⇄ The [runner-free process toolkit](../probes/README.md) executes the full
public emit/decode/resume loop without a runner, including registry and Git
defaults, while remaining distinct from this actual SSH evidence.
Neither resolved-plan emission nor this harness implements a complete deferred
shell workflow. Mosh still needs a separately selected SSH observation channel;
the existing mosh `tta-cli` path still requires target tta, while its classic
availability fallback remains as before.
