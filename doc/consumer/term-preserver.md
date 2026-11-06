---
type: ConsumerAccount
title: term-preserver — local attempts and prepared SSH handoff
description: Implemented shared-runner adoption, target setup and SSH option ownership, verified no-target-tta behavior, and the remaining mosh observation choice
resource: /doc/consumer/term-preserver.md
status: implemented-partial
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e4e2b4bcbffePwraSmunm9Lmav
    title: Check the 0.6.0 independent-view API against the receiving consumer, preserve explicit grouping policy and refresh its lock
  - resource: opencode://ses_e80ce13ddffeiWXfWVMBW9qXv5
    title: Integrate and exercise the runner's SSH planning through the consumer's local attempt worker and real proxy/PTY
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Actual availability policy, worker request, target setup and prepared handoff
  - resource: file:///home/rektide/src/term-preserver/doc/launch/README.md
    title: Consumer-owned launch, transport and retry contract
  - resource: /session/runner/src/ssh.rs
    title: Shared target acquisition and prepared command API
  - resource: /verification/driver/src/ssh.rs
    title: Reusable actual SSH and optional consumer exercise
---

# Implemented adoption

[term-preserver's launch guide](file:///home/rektide/src/term-preserver/doc/launch/README.md)
owns consumer behavior. Its `tta-lib` feature now embeds `tta-launcher` 0.6.0,
`tta-runner` and `tta-command`; both scaffold adapters are compiled. Local attempts
reuse `tta_runner::prepare` and semantic native/CLI execution. SSH attempts keep
the worker **local**, use `ssh::prepare` for context/registry and framed read-only
observations, then pass `PreparedRemote::command()` to the consumer's transport.
The parent proxy, PTY and user retry prompt remain consumer-owned.

The 0.6.0 library default gives omitted outside grouping an independent view,
but this consumer passes `.grouped(launch.tmux_group)` explicitly in both local
and SSH preparation. Its intentional caller policy is preserved, not replaced
by tta's CLI default. `cargo check --all-targets --all-features` and the nine
`launcher::` tests pass with the updated receiving lock. This checks compatibility
of actual adoption; it does not claim new consumer prefix UI or inside-origin
inference support. The receiving lock also adopts the already-required tmax 0.7.0.

Auto chooses embedding for SSH without probing for a target tta executable.
Matching/read/generator/execution failures never fall through to classic creation.
The configured target shell's login/interactive startup runs for every probe and
handoff. Target cwd/HOME/UID/root/config/Git facts supply defaults; explicit
consumer options win, and remote paths are not resolved locally. The consumer's
`tmux_program`, including default `tmux`, is an explicit executable choice.

The consumer computes managed ControlMaster options once per prepared worker
request and gives the runner the same options for probes and handoff. It must not
derive a second SSH policy after discovery. Remote terminal entry attaches;
inherited local TMUX does not authorize a switch. The runner keeps grouped
creation/attachment/keep-last in one queue and stops after uncertain mutation,
without automatic replay, rollback, pruning or backend fallback. A user pressing
retry invokes a fresh worker and new discovery.
Probe authentication must already be available through keys/agent or an
authenticated master; a separate interactive password/passphrase bootstrap is
not implemented in this bounded, closed-stdin acquisition path.

# Evidence and remaining choices

The [actual loopback SSH toolkit](/verification/ssh/README.md) can run the real
consumer proxy and its nested PTYs against a target PATH where `command -v tta`
returns 127. It observes cold creation, detach and user-authorized fresh grouped
retry independently through SSH/tmux. The fixture explicitly disables the managed
master for private lifecycle isolation; option identity/construction has unit
coverage, not an inferred master-recovery guarantee.

Mosh still uses the existing remote CLI carrier. Embedding needs a separately
declared SSH observation host/options and the same target setup before mosh
terminal handoff. Reusing `LaunchConfig.ssh` is not currently possible because it
selects the transport and conflicts with mosh. A distinct observation-channel
configuration is the concrete remaining choice, not a promise that mosh executes
probes. Complete deferred shell workflow compilation is also later work; resolved
plan emission is not that compiler.

Refresh this account as library or consumer integration changes; wishes do not commit the
library or consumer to implementing them.
