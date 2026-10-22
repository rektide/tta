---
type: ArchitectureDecision
title: Native detached creation without replacing tmux terminal ownership
description: The implemented native-default launch slice, explicit CLI boundaries, lifecycle rejection of a satellite split, and remaining opportunities
resource: /design/launcher/launch-backends0.gpt61sol.md
status: implemented-partial
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-22
sources:
  - resource: opencode://ses_e9a2dad06ffeFURNHXCj1ln0ea
    title: Inspect tta and tmax, reject unsafe split satellite creation, implement native inside creation and client-directory support, and test execution contracts
  - resource: opencode://ses_e9a6011b0ffeBJ4GW5UhFqoHb5
    title: Parent commissions a practical native-default launch improvement, preserving CLI alternatives and terminal semantics
  - resource: /session/launcher/src/plan.rs
    title: Shared invocation construction, existing command queue ordering, startup escaping and project cwd
  - resource: /session/launcher/src/execution.rs
    title: Prepared launch execution, live-server preflight, prefix and cwd checks, native mutation and CLI switch
  - resource: /app/cli/tests/launch.rs
    title: Real-server backend, literal argv, same-client, grouped lifecycle, cold-config and failure checks
  - resource: /design/launcher/research0.gpt6a.md
    title: Prior language and terminal boundaries and warning against coupling generic expressions to tmux
  - resource: file:///home/rektide/src/tmux-interface-control/src/endpoint/mod.rs
    title: Native run/query, byte-preserving client context and the added client-directory override
  - resource: file:///home/rektide/src/tmux-interface-control/src/protocol/native/context.rs
    title: Validated native identity context and raw Unix cwd/environment preservation
  - resource: file:///home/rektide/src/tmux-interface-control/src/protocol/native/command.rs
    title: Descriptor-free direct command identity, empty tty name, output limits and no server startup
  - resource: file:///home/rektide/archive/tmux/tmux/cmd-find.c
    title: Current-client selection prefers the invoking tty and otherwise TMUX_PANE; a native client is not an attached terminal client
---

# Decision

Work anchor: **tta-launch-backends**. The user's direction is to use more native
tmax where there is a straight shot, preferably by default—not to implement a
terminal client or shared-server supervisor.

`prepare_launch()` retains the complete CLI expression plus an eligible native
detached-create operation. The CLI's existing `native`/`cli` features and
`--backend` / `TTA_BACKEND` selection govern lookup and execution together.
Native remains the default. Existing `launch()` callers still receive the same
generic `Expression`; no tmux operation enters `tta-command`.

Native eligibility is deliberately narrow:

1. A project or scaffold selection, inside the selected tmux server.
2. Startup operands are all UTF-8 (including the empty startup vector).
3. A fresh read confirms a live server with sessions before generation begins.

For eligible launches: read → generator prefix, if any → validate/canonicalize
the selected directory → native `new-session -d` → CLI `switch-client` by exact
name. Canonical IDs still target existing-session selections. The creation and
switch were already separate CLI invocations, so this adds no new mutation/
handoff split. The prefix is run only for create selections, never project ones.

The native invocation uses the same format-escaped name and directory, startup
words and lone-executable wrapper as the CLI plan. tmax's argv encoder owns the
trailing-semicolon boundary. Non-UTF-8 startup operands deliberately use the
whole existing CLI plan rather than being rejected or converted lossily.

## Cwd and the small tmax addition

tmax previously identified every direct client with the Rust process's cwd.
tta's generic expression runner instead gives subprocesses the selected physical
cwd without changing process-global state. A `-c` operand controls tmux's new
session/pane directory but is not itself the native client's identified cwd.

`endpoint::Tmux::client_directory(absolute_path)` now overrides that identity
field without a global chdir, changing socket resolution or assuming ownership
of a server. Existence is the consumer's responsibility; relative paths fail
before connection. tta canonicalizes and checks the directory first, matching
the existing `ChangeDir` contract. See the library-side
[consumer account](file:///home/rektide/src/tmux-interface-control/doc/consumer/tta.md).

Public additions mark tta's inherited workspace version as **0.2.0** and tmax as
**0.5.0**. The path dependency and receiving lock are updated together; version
equality is not an immutable revision pin.
The verified library addition is tmax commit
`f7fb3acdf6aba8df72d5f4f2a56eb1816bcf7b69`; the sibling path remains moving.

## Boundaries retained on the CLI

- **Outside new-session + attach:** one real client owns the terminal and cold
  server startup. tmax endpoints cannot start the shared server; `OwnedServer`
  is an unrelated explicit private-server lifecycle, not a replacement here.
- **Cold/sessionless inside context:** a read chooses the full CLI expression
  before any generator or mutation. This preserves the old stale-context
  behavior, including startup `-f` and a subsequent switch failure if no client
  exists. Protocol, permission and other read errors are not treated as absence.
- **All switches:** CLI current-client selection uses tty context and then
  `TMUX_PANE`. Native direct clients identify with an empty tty name. Do not
  silently switch whichever attached client a new native connection chooses.
  Exact explicit-client resolution remains a separate possible improvement.
- **Grouped launches:** detached create → attach/switch → `keep-last` remains
  one CLI command queue. An early checkpoint explored native satellite creation;
  it was rejected because inherited `destroy-unattached` may remove the new
  satellite before the next client connection can attach. Setting `keep-last`
  before attachment has the same problem. The initial checkpoint is evidence,
  not the final implementation.
- **Shell output:** always the full CLI expression. Printing never executes a
  generator, native preflight mutation or partial launch. Both backend selections
  emit the same commands for the same resolved input and client executable.

`-L`/`TMUX_TMPDIR` and `-S` are frozen by the existing socket resolver; native
operations use that exact path. All client commands retain the original label
and startup-config envelope. `-f` only matters for server startup, which remains
CLI-owned. Native errors never trigger a mutation retry through the CLI.

## Failure semantics and remaining work

Prefix failure stops before creation; invalid cwd stops before native mutation;
native rejection or uncertain completion stops before switching. Native
timeouts/output failure can follow an actual mutation, so errors explicitly do
not promise that the session was never created. A subsequent CLI failure may
leave the detached session behind. There is no rollback, prune or reconciliation.

The native direct command path retains tmax's text-output and deadline limits;
successful output/diagnostics are forwarded before handoff. It does not provide
arbitrary byte-stream output equivalence or a hard rollback deadline.

The ticket remains open for a broader backend implementation: safe explicit
invoking-client identity, queue-preserving grouped mutations, and native byte
argv would expand eligibility. Those are distinct library/design work, not
hidden fallbacks in this slice. See the maintained
[operation matrix](/README.md#which-operations-use-tmax) and
[runtime verification](/README.md#runtime-verification).

## Verification record

The system binary resolved to tmux **3.8** during this session, rather than the
3.7c recorded by earlier work. A separate fixture was built from the local archive's
`3.7c` tag, its `tmux -V` asserted, and its binary put first on PATH. All live
sockets remain exclusively owned fixtures under `/tmp/opencode`.

Final workspace nextest results on pinned **3.7c**:

| Build | Result |
| --- | --- |
| Default native + scaffold features | 110 passed |
| CLI-only (`--no-default-features --features cli`) | 84 passed |
| Combined (`--all-features`) | 122 passed |

System 3.8 runs also passed these configurations, plus a 24-test CLI-only pass
with scaffold features explicitly restored. Clippy with deny-warnings passed for
combined and CLI-only builds; workspace formatting and doctests passed. Markdown
structure, links and native session provenance were checked with `rkt`.

The library's six focused endpoint tests passed under its isolated test wrapper.
Its streams experiment compiled with the refreshed locked dependencies. The
separate `materialize` package is blocked by its existing unmatched
`PublicationReport::Enqueued` variant from the moving Kubeless checkout, not by
the client-directory addition; no materialize implementation was changed.
