---
type: ConsumerAccount
title: term-preserver — local attempts and prepared SSH handoff
description: Implemented shared-runner adoption, target setup and SSH option ownership, verified no-target-tta behavior, the local plural-roots/completion adoption, and the remaining mosh observation choice
resource: /doc/consumer/term-preserver.md
status: implemented-partial
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e3e1e7340ffeqbKYojX8Zvxx4L
    title: Adopt the plural local roots environment and identity-aware completion at the consumer's real call sites; make its fixtures runnable under configurable scratch
  - resource: opencode://ses_e3e3170d4ffeij7U00lUlwaZg7
    title: Port actual local/SSH call sites to 0.7.0 ordered-root APIs and refresh the receiving lock; distinguish the existing one-root consumer UI from pending multi-root integration
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

## Ordered-root API checkpoint (0.7.0)

The consumer's two actual call sites now pass ordered lists: its explicit
`session_root` becomes a one-element SSH override, and its existing local
`project_root()` result becomes the runner's `roots` input. The receiving lock
adopts the coordinated 0.7.0 packages. Explicit grouping, terminal transport and
retry ownership are unchanged. This is an API adoption checkpoint, **not** a new
consumer multi-root UI or local `TTA_ROOTS`/completion implementation. Target-local
SSH defaults do use the shared new roots policy when no explicit root is supplied.

## Local plural-roots adoption

The consumer's local configuration and completion now mirror tta's declared
plural precedence through its own call sites: `--session-root`, then `TTA_ROOTS`
(a JSON array of nonempty paths), then `TTA_ROOT`, then home-relative
`PROJECTS_DIR`, then `~/src` — one active layer replacing the whole list, relative
entries frozen against the invocation cwd, and invalid active layers erroring
rather than falling through. `launcher::project_roots` is one shared helper for
embedded local launches and completion, deliberately implemented locally so
completion keeps working in builds without the `tta-lib` feature; it tracks the
tta contract
([project-roots](file:///home/rektide/src/tta/doc/entry/project-roots.md)).

Project completion is identity-aware across the whole list: a bare basename is
offered only when exactly one physical directory of that exact name exists
across all roots; ambiguous basenames stay reachable through one-based
`@N/name` selectors, and launch-equivalent qualified duplicates are dropped so
widget menus stay minimal. The `--session-root` option remains a single explicit
override (the consumer's established input contract); repeated-root input is not
new consumer UI. SSH attempts keep their explicit-only root override and layer
defaults from target facts, unchanged. Its Zsh widget needed no change: it already
forwards `--session-root`, and the environment reaches the binary.

Verified with `cargo check --all-targets --all-features` and the full
245-test suite (including the Zsh PTY widget and the formerly
environment-blocked remote-classic fixture, restored by the consumer's
`TTA_TEST_TMPDIR` scratch selection). Consumer commit `vnpwlnws`
(`0f97bfe3`), "Adopt plural local project roots and identity-aware completion".

`cargo check --all-targets --all-features --offline` passed. Of nine selected
`launcher::` tests, eight passed; the existing remote-classic fixture failed to
create its directory because `/tmp/opencode` was owned mode 0700 by `zimtest2`.
That test did not exercise the changed API. The tta
[establishing checkpoint](/design/entry/project-roots0.gpt61sol.md) gives the next
integration handoff: review consumer local configuration/completion deliberately,
and rerun that fixture after the scratch-access problem is resolved.

## Earlier SSH acceptance and remaining mosh choice

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
