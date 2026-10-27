---
type: DesignHandoff
title: Targeting a remote tmux server with the embedded launcher
description: Situation, constraints, and directions of inquiry for driving remote sessions from local tta, with state-machine and trait seams as candidate structuring rather than a chosen path
resource: /design/launcher/remote0.glm53max.md
status: draft
author: model:zai-coding-plan/glm-5.3#max
date: 2026-10-27
sources:
  - resource: opencode://ses_e818d4b00ffe1ewolZWtBj7BsP
    title: Commission the remote-targeting exploration after a stale remote tta CLI rejected the launcher's scaffold flag
  - resource: /session/launcher/src/backend.rs
    title: SessionLookup trait and the Native/Cli lookup implementations
  - resource: /session/launcher/src/execution.rs
    title: LaunchBackend execution policy and the may-already-have-taken-effect error surface
  - resource: /session/launcher/src/socket.rs
    title: Socket selection, environment context, and client-mode resolution
  - resource: /session/launcher/src/plan.rs
    title: Launch plan, ClientMode, and the grouped satellite queue
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Consumer remote carrier, backend selection, and the embedded/local-only boundary
  - resource: file:///home/rektide/src/term-preserver/doc/launch/README.md
    title: Consumer backend availability policy and remote selection rules
  - resource: /design/launcher/launch-backends1.gpt61sol.md
    title: Independent-view checkpoint and the native/CLI execution boundary this exploration extends
  - resource: file:///home/rektide/src/tmux-interface-control/README.md
    title: "Native control-mode client constraints: existing server, no implicit start"
---

# The situation that raised the question

term-preserver embeds this launcher (`tta-lib`) and, as of 2026-10-27, compiles
it by default: local launches resolve, scaffold, create, and hand off entirely
in-process. Remote transports cannot use any of that. Embedded execution needs
a local tmux server socket, so term-preserver's `--ssh`/`--mosh` paths select
the **tta CLI on the remote host** when present, else fall back to classic
tmux. The consumer wraps a POSIX carrier:

```sh
if command -v tta >/dev/null 2>&1; then
  exec tta --scaffold="${TTA_SCAFFOLD:-default}" ...
else <classic tmux fallback>; fi
```

The coupling in that carrier is flag-surface coupling. This week it failed in
the wild: a host carrying a stale `tta 0.1.0` (no `--scaffold`, no `--group`,
no `--dir`) rejected the launcher's arguments, and every remote launch from
the embedded consumer errored until the remote binary was upgraded. The
consumer's embedded backend cannot regress that way locally — remotely it has
no presence at all.

The ask: explore how tta-the-library could target a remote tmux server, so a
local embedded launcher (or local CLI) drives remote sessions without
requiring a current tta binary on the target host. The human suggests a more
explicit **state machine** and **traits for implementations** as promising
structuring. Both are developed below as candidate lenses; neither is a
decision. Nothing here prescribes a path.

# What the current seams already say

- [`SessionLookup`](/session/launcher/src/backend.rs) is the one trait: it
  acquires session facts, with `Native` (tmax control-mode connection) and
  `Cli` (spawn a tmux client) implementations. Lookup is deliberately
  fact-only.
- [`execution`](/session/launcher/src/execution.rs) is an enum policy, not a
  trait: native owns ordinary detached creation inside a live server; the CLI
  path owns terminal operations, grouped queues, cold creation, and non-UTF-8
  startup argv. Its `ExecuteError` already names the deeper issue: a failed
  mutation *"may already have taken effect"*. Steps are not idempotent and not
  transactional.
- [`plan`](/session/launcher/src/plan.rs) and
  [`socket`](/session/launcher/src/socket.rs) resolve client mode (inside-tmux
  switch vs fresh attach) and the endpoint from the environment — all
  local-process-local-terminal assumptions.
- tmax's native client connects to an **existing** server's socket and never
  starts one; the terminal client (interactive attach) is a separate process
  concern.

Constraints any direction must respect or explicitly revisit:

1. **Terminal locality.** Attaching means a tmux client process reading a
   terminal. Under ssh/mosh the terminal is local and the server is remote;
   *something* must bridge that (remote client over the transport's PTY, or
   forwarded terminal streams). No library trait removes this; it only decides
   where the bridge lives.
2. **mosh has no per-attempt exec channel.** mosh runs one remote command at
   session start and then carries a UDP terminal. Re-probing or re-execing on
   retry needs a fresh ssh side channel. term-preserver's carrier runs once
   per attempt inside the transport; a design that assumes "run this argv on
   the host, now" needs an answer for mosh beyond "use ssh".
3. **The tmux protocol has no authentication.** Whoever reaches the server
   socket owns the server. Any direction that moves or forwards that socket
   across hosts is expanding the trust boundary and must say so loudly.
4. **Cold start.** A control-mode connection presupposes a running server;
   creating the first session is inherently a process spawn on the target
   host. Remote targeting still needs one remote exec for cold start, or it
   declines cold start.
5. **Scaffolding is filesystem work.** Generators create directories, run
   git/jj, author files. A remote clean miss means that work belongs on the
   remote filesystem — "where does the generator run" is part of the design,
   not an afterthought.

# Directions of inquiry

Four directions, not mutually exclusive. They differ in where the planner
runs, what crosses the network, and what version-skew surface remains.

## A. Remote-exec transport for library steps

Keep the planner local; generalize "execute this tmux invocation" from
process-spawn to a trait (`ExecutionTransport`?) with local-native, local-CLI,
and ssh-exec implementations. Probe, create, join-group, and switch become
transport calls; attach still runs a remote client over the transport PTY.

- *Gains:* one planner, one version (the local embedded one); the remote
  binary requirement disappears except for cold start; per-step errors stay
  structured.
- *Costs:* every step is a network round trip (or a persistent session —
  ssh ControlMaster multiplexing becomes load-bearing); mosh constraint #2
  applies to *every* step; idempotency matters immediately (a create that
  "may have taken effect" retried over a flaky link).

## B. Forwarded control socket

Forward the remote tmux server socket over ssh (relay/socat-style) so tmax's
existing native client connects to a remote server as if local. The tmux
control protocol is server-local; forwarding is a known technique.

- *Gains:* maximal reuse — lookup, query, and detached creation work
  unchanged against the remote server; tta learns almost nothing new.
- *Costs:* constraint #3 becomes the headline — the forwarded socket is full
  server control crossing hosts; constraint #4 — cold start still needs a
  remote exec; terminal attach still needs its own answer; per-command
  latency rides one connection (this may be fine: control mode is a
  persistent connection anyway).

## C. Remote worker speaking a tta protocol

A small worker binary (or the CLI in a worker mode) on the target host,
driven over ssh stdio with a versioned, structured protocol instead of
argv/flag coupling. The local side sends intents; the worker owns local
server interaction and reports structured results.

- *Gains:* fixes the actual failure class — skew becomes protocol
  negotiation, not clap errors; the worker can be shipped on demand over ssh
  (stream the binary, exec from stdin/temp) making the remote binary
  requirement transient; scaffold filesystem work runs naturally where the
  filesystem is.
- *Costs:* a protocol to design, version, and test; the largest new surface
  in tta; still one binary's worth of deployment story even if self-propelled.

## D. Structured carrier as minimal hardening

Keep the current architecture (exec a remote program) but make the contract
structured and versioned — e.g. the CLI grows a `--worker` mode reading a
request from stdin — so stale binaries degrade gracefully ("worker protocol
too old: falling back to classic") instead of erroring mid-carrier.

- *Gains:* smallest change that addresses this week's failure; no new
  trust boundary; composes with A/B/C later.
- *Costs:* does not remove the remote binary requirement; no native-query or
  scaffold parity; the remote still needs *some* current-enough tta.

# Candidate structuring threads (suggested, not prescribed)

**An explicit state machine** for the launch lifecycle, with each state
annotated by *where it must run*: server-socket host, terminal host, or
filesystem host. A sketch to react to:

```
Probe(server)            [socket host]      server exists? inventory?
Resolve(target)          [socket host]      exact / unique / project / miss
Scaffold?(miss)          [fs host]          generator creates the project
Create|JoinGroup|Switch  [socket host]      cold Create also needs [process host]
Handoff(attach|switch)   [terminal host]    client where the terminal is
```

The annotation is the analytical payoff: "remote targeting" stops being a
boolean backend property and becomes a per-state placement decision. Today's
enum policy (native for detached creation, CLI for terminal/grouped/cold) is
a placement policy stated implicitly; the state machine would state it
explicitly, and `ExecuteError`'s "may already have taken effect" becomes a
declared transition uncertainty (which states are idempotent, which are not,
what re-entry means after transport loss).

**Trait seams** to consider beyond the existing lookup trait — each is a
question about whether the seam is real or premature:

- lookup (exists) — could gain a remote implementation under B unchanged;
- step execution / mutation — A's `ExecutionTransport`, or a finer
  `ServerConnection` abstraction;
- terminal handoff — attach/switch as a seam, since placement differs from
  every other state;
- scaffold filesystem — generators currently assume the local fs; is the
  right seam "run generator remotely" (C) or "generator as data, applied
  remotely"?

# Open questions

- Is per-state placement (mixing, say, B for queries with a remote client for
  attach) desirable, or does the current all-or-nothing backend selection
  model serve better and just need a remote member?
- What is the security stance on forwarding a tmux control socket (B) across
  an ssh boundary — acceptable over ssh-only, never, or opt-in per host?
- Does grouped satellite creation genuinely need the CLI path remotely, or
  can native detached creation cover it once switching stabilizes (see
  [launch-backends1](/design/launcher/launch-backends1.gpt61sol.md))?
- Retry semantics over a transport that can fail between "took effect" and
  "reported success": does the state machine need journaling/probing
  re-entry, and is that worth it for a launcher?
- Where does scaffold configuration resolve — local `TTA_CONFIG` pushed with
  the request, or remote-respected, both?
- Should the consumer's remote carrier (currently owned by term-preserver)
  move into tta as part of any of these, so the protocol lives with one
  implementation?

# Related

- [launch-backends1](/design/launcher/launch-backends1.gpt61sol.md): the
  native/CLI boundary and independent-view work this extends.
- term-preserver's
  [launch backend guide](file:///home/rektide/src/term-preserver/doc/launch/README.md):
  availability policy and today's remote carrier.
- [tmax](file:///home/rektide/src/tmux-interface-control/README.md): the
  native client whose connection model directions B leans on.
