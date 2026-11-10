---
type: ConsumerAccount
title: term-preserver — owned entry attempts and delegated terminal effects
description: Implemented 0.8.0 adoption through EntryAttempt, observe and execute interpreters with CatalogQuery completion, explicit grouping and consumer-owned transport/master/retry, verified by actual local/SSH consumer retries
resource: /doc/consumer/term-preserver.md
status: implemented
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
updated: {by: "model:zai-coding-plan/glm-5.3#max", at: 2026-11-10}
sources:
  - resource: opencode://ses_e3cf482d5ffeyD5hefdVDZGmbe
    title: Promote the consumer account to the 0.8.0 entry-attempt call sites, catalog completion and actual retry evidence
  - resource: opencode://ses_e3d19979bffe2Eu6JnXnb11i7H
    title: Coordinate the consumer onto exact 0.8.0 pins with missed completion callers on the catalog and target-relative SSH paths
  - resource: opencode://ses_e3e1e7340ffeqbKYojX8Zvxx4L
    title: Adopt the plural local roots environment and identity-aware completion at the consumer's real call sites
  - resource: opencode://ses_e3e3170d4ffeij7U00lUlwaZg7
    title: Port actual local/SSH call sites to ordered-root APIs and refresh the receiving lock
  - resource: opencode://ses_e4e2b4bcbffePwraSmunm9Lmav
    title: Check the independent-view API against the receiving consumer, preserving explicit grouping policy
  - resource: opencode://ses_e80ce13ddffeiWXfWVMBW9qXv5
    title: Integrate and exercise SSH entry through the consumer's local attempt worker and real proxy/PTY
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Actual EntryAttempt acquisition, explicit grouping mapping and interpreter driving at both call sites
  - resource: file:///home/rektide/src/term-preserver/src/completion.rs
    title: CatalogQuery-based embedded completion beside the retained non-library implementation
  - resource: file:///home/rektide/src/term-preserver/doc/launch/README.md
    title: Consumer-owned launch, transport and retry contract
  - resource: /design/entry/cutover1.gpt61sol.md
    title: Coordinated 0.8.0 consumer adoption, actual SSH retry evidence and receiving-lock receipts
  - resource: /verification/driver/src/ssh.rs
    title: Reusable actual SSH and optional consumer exercise
---

# Implemented adoption

[term-preserver's launch guide](file:///home/rektide/src/term-preserver/doc/launch/README.md)
owns consumer behavior. Under its `tta-lib` feature the consumer now embeds
the **0.8.0** tta packages (exact internal pins; the consumer's own public
version stays 0.2.0). Both embedded call sites — local
(`launcher::embedded_tta`) and SSH (`launcher::embedded_ssh`) — acquire one
`tta_launcher::entry::EntryAttempt` with explicit, presence-based
`context::Overrides` and an `EntryIntent` whose view maps its explicit
`tmux_group` to `ViewPreference::Independent`/`Shared`, never `Auto`. Local
attempts drive `observe::local::Local` (native or CLI session backend) and
`execute::local::Local` through `execute::drive` with
`Delivery::Execute { native }`; SSH attempts freeze one
`observe::ssh::Ssh` observer over the consumer's SSH options and target
`TargetSetup`, then dispatch the bound terminal effect through
`execute::ssh::drive` — one terminal invocation sharing the exact connection
and setup that answered every read. The parent proxy, PTY and user retry
prompt remain consumer-owned; a user retry builds a **fresh attempt** with
fresh observations. `Stopped` outcomes surface the boundary and retained
effect knowledge (including a possibly executed mutation) rather than asking
anyone to replay; `Delegated`/terminal receipts exit with the client's status.

Completion under `tta-lib` now queries the read-only
`tta_launcher::entry::catalog::CatalogQuery` through
`observe::drive_reads`, reusing the same ordered-roots policy helper
(`launcher::project_roots`) as embedded launches. The consumer's
**non-library build keeps its own standalone completion and roots policy**;
that is a retained real feature obligation of the classic/no-`tta-lib`
configuration, not a debt to be replaced by mandatory embedding.

## SSH specifics corrected with the consumer

- Explicit SSH `--session-root`, `--tmux-socket` and target directory
  overrides remain **native relative values** until shared target context
  policy freezes them on the target; the cutover fixed an earlier port that
  joined them against the local cwd.
- The consumer computes its managed ControlMaster options once per worker
  request; probes and the terminal action share that one option set and the
  same OpenSSH config. It must not derive a second SSH policy after
  discovery. Remote terminal entry attaches; inherited local TMUX does not
  authorize a client switch. Grouped create → attach → `keep-last` stays one
  tmux queue; after action start, transport loss leaves target mutation
  uncertain with no library replay, rollback, prune or backend fallback.
- Probe authentication must already be available through keys/agent or an
  authenticated master; a separate interactive password/passphrase bootstrap
  is not implemented in this bounded, closed-stdin acquisition path.
  `observe::ssh::write_diagnostics` reports startup/trailing/transport/
  operation evidence on stderr before the original failure propagates;
  stdout stays clean.

## Actual acceptance

The [cutover receipt](/design/entry/cutover1.gpt61sol.md) records the final
authenticated run: all **three** real consumer cases use term-preserver's own
proxy, worker and PTYs — local native observer, local CLI observer, and
embedded SSH. Each creates work, detaches, accepts an explicit user retry,
observes the newly selected independent view sharing the primary window with
`keep-last`, detaches and quits; the SSH case also passes explicit relative
target root and directory under a different local cwd. Consumer commits
`c771ad8b` (0.8.0 pins/receiving lock, catalog completion callers,
target-relative SSH paths, honest effect-stop reporting) and `d6eece86`
(concurrent SSH fixture isolation) land this adoption; the full 245/220/241
consumer feature suites and the 180-test ordinary parallel lib run passed.

## Ordered roots and completion (0.7-era adoption, retained)

The consumer's local configuration and completion mirror tta's declared plural
precedence through its own call sites: `--session-root`, then `TTA_ROOTS`
(a JSON array of nonempty paths), then `TTA_ROOT`, then home-relative
`PROJECTS_DIR`, then `~/src` — one active layer replacing the whole list,
relative entries frozen against the invocation cwd, invalid active layers
erroring rather than falling through. `launcher::project_roots` is one shared
helper for embedded local launches and completion, deliberately implemented
locally so completion keeps working in builds without the `tta-lib` feature;
it tracks the tta [project-roots contract](/doc/entry/project-roots.md).
Project completion is identity-aware across the whole list: a bare basename is
offered only when exactly one physical directory of that name exists across
all roots; ambiguous basenames stay reachable through `@N/name` selectors.
The `--session-root` option remains a single explicit override (the consumer's
established input contract); repeated-root input is not new consumer UI.

## Remaining mosh choice

Mosh still uses the existing remote CLI carrier. Embedding needs a separately
declared SSH observation host/options and the same target setup before mosh
terminal handoff. Reusing `LaunchConfig.ssh` is not currently possible because
it selects the transport and conflicts with mosh. A distinct observation-channel
configuration is the concrete remaining choice, not a promise that mosh executes
probes. Complete deferred shell workflow compilation is also later work.

Refresh this account as library or consumer integration changes; wishes do
not commit the library or consumer to implementing them.
