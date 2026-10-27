---
type: ImplementationBrief
title: Runner-free planning and remote execution
description: Commissioned crate boundaries, implementation sequence, and reusable verification outcomes for observation-driven tta and its term-preserver consumer
resource: /design/launcher/runner-core0.gpt6a.md
status: commissioned
author: model:openai/gpt-6-astra#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e80f5d05bffe77stycHGRJOH29
    title: Source inspection including the former launcher execution.rs; human accepts and commissions the architecture; parent directs Sol xhigh, integrates G scout findings and defines toolkit outcomes
  - resource: opencode://ses_e80e6a338ffeMZsVeoLYisi7h8
    title: Independent read-only verification scout identifies input-error and capture gaps, reusable fixtures, and installed prerequisites for owned loopback SSH
  - resource: /design/launcher/remote0.glm53max.md
    title: Remote targeting situation, transport alternatives, and later local socket-relay evidence
  - resource: /design/launcher/launch-backends1.gpt61sol.md
    title: Independent-view lifecycle and unresolved invoking-client identity
  - resource: /session/launcher/src/resolve.rs
    title: Current session-first matching mixed with filesystem observation
  - resource: /session/launcher/src/plan.rs
    title: Pure command planning and grouped queue ordering
  - resource: /session/launcher/src/scaffold.rs
    title: Registry loading, validation, adapter selection and generator prefixes
  - resource: /session/launcher/src/socket.rs
    title: Endpoint context, local observation and attach/switch policy
  - resource: /shell/command/src/lib.rs
    title: Command expressions currently combine pure rendering and direct execution
  - resource: /scaffold/rekon-native/src/lib.rs
    title: Generator planning currently reads environment and invokes Git for author defaults
  - resource: /verification/driver/src/main.rs
    title: Existing reusable PTY acceptance command and structured reports
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Consumer-owned attempts, PTY handoff, remote carriers and duplicated embedded orchestration
  - resource: file:///home/rektide/src/term-preserver/doc/launch/README.md
    title: Consumer launch options, default remote configuration, availability and retry policy
  - resource: file:///home/rektide/src/tmux-interface-control/Cargo.toml
    title: Typed operations and queries currently share the client feature with native execution
---

# Commission

Work anchor: **tta-launch-backends-remote-target**. The human accepted the
structure below and commissioned implementation, not merely another proposal.
Sol xhigh implements; the parent owns architectural review and integration;
G supplies independent verification. The prior [remote exploration](remote0.glm53max.md)
remains evidence of alternatives rather than the implementation decision.

The useful outcome is a tta library that describes required observations and
intended operations as data, works with no runner, and can be driven locally or
over SSH without an installed tta on the target. term-preserver is the first
embedded remote consumer. Reusable checked-in verification capabilities are
part of delivery, not just a final collection of ephemeral test results.

## Crate structure

| Home / crate | Responsibility |
| --- | --- |
| `session/launcher` / `tta-launcher` | Pure requests, observations, matching, workflow progression, semantic plans, probe construction/decoding, command lowering, registry parsing |
| `session/runner` / `tta-runner` (new) | Environment/filesystem/config acquisition, driver loop, process/native/SSH adapters, execution outcomes and prepared terminal handoff |
| `shell/command` / `tta-command` | Pure literal argv/environment/cwd/file-write/sequence expressions and POSIX rendering; execution moves to the runner |
| `scaffold/rekon`, `scaffold/rekon-native` | Pure generator planning with explicit options and supplied defaults |
| `app/cli` / `tta` | CLI/environment precedence and composition of core plus optional runner |
| `verification/driver` / `tta-verify` | Reusable exercise/observation library and command, extended for the new boundaries |

Runner modules can hold local process, native tmax and SSH implementations;
individual backends need not each become a crate. The core does not depend on
the runner. The runner depends on the core and command language. Pure emission
must be possible without runner features or a runner instance. Keep the generic
command language unaware of tmux and SSH.

Preserve tmax's command/query vocabulary and decoders. A focused feature split
in the sibling tmax checkout may expose pure operation/query construction and
decoding independently of native endpoint execution. Coordinate this narrowly:
that checkout also has unrelated ongoing API-coherence work. Native transport,
parser retirement and broad API redesign are not this task's ownership.

## Observations and decisions

The workflow has multiple input boundaries, not one boolean existence query:

- Target environment and invocation context: cwd, HOME, UID, socket policy and
  relevant configuration inputs. Local and target paths/context are distinct.
- Scaffold registry contents and explicit/default generator choice. Preserve
  current configuration-validation ordering unless a deliberate change is argued.
- Session IDs and names, or a classified observation of server absence.
- Canonical project root and immediate directory/symlink candidates, needed
  after session matching misses. Session-only entry must not require a usable root.
- Exact scaffold destination state, including dangling symlinks and inspection
  errors. Unknown state is not absence.
- Generator defaults, including target author/license environment and Git identity.
- Execution outcomes and any fresh preflight facts required for native eligibility.

Use ordinary inspectable data for requests, replies, plans and workflow state.
The core yields an observation request or a ready plan; callers supply answers.
Do not represent printing as an executor returning fictitious success. Keep
matching policy shared across native, CLI and remote observation implementations.

The existing launch plan is mostly a short-circuit action sequence after
selection. Do not introduce a general workflow engine merely to represent it.
Native execution must not parse shell text to rediscover operation semantics.

## SSH and consumer integration

The selected starting path keeps decisions local and runs generated probes and
launch commands on the target:

1. Obtain target context/configuration/session facts through a non-PTY SSH probe.
2. Feed typed replies to the local workflow; acquire additional facts as needed.
3. Prepare a coherent target-side launch program and terminal handoff.
4. Let the consumer run that program through its terminal transport.

There is no remote tta dependency, including for cold creation. Target tools
remain tmux and the tools selected by the generator. Probe and action paths must
agree on target environment, cwd, endpoint and explicit overrides. Shell startup
noise cannot become valid observation data. Probe framing must faithfully carry
supported names/content and preserve status/diagnostic distinctions; do not parse
`ls` or convert connection/read/decode errors into empty inventories.

Batch independent observations where useful. Once decisions are made, keep a
coherent mutation block together. In particular, preserve grouped create →
attach/switch → `keep-last` in one tmux command queue. Failed creation prevents
handoff. Transport loss after mutation can leave an uncertain result; do not
blindly replay creation, retry through another backend, roll back, or auto-prune.

The target's environment/configuration supplies defaults; explicit caller
options override it. Target paths must not be canonicalized on the local host.
Being inside local tmux is not authority to switch a remote client. A fresh SSH
terminal normally attaches. Existing inside-client identity ambiguity is recorded
in [the independent-view checkpoint](launch-backends1.gpt61sol.md), not magically
resolved by native queries.

term-preserver owns its parent proxy, PTY and attempt/retry lifecycle. Supply a
prepared-handoff API usable by its attempt worker. Centralize tta orchestration
so this consumer no longer rebuilds the policy. Mosh needs a distinct SSH
observation capability before terminal handoff; decide its integration explicitly
at the transport checkpoint rather than silently pretending mosh is an exec API.

## Runner-free outputs

First-class foundation:

- Supplied facts produce a plan and shell commands with no runtime I/O.
- Missing facts produce explicit observation requests, with emit/decode support
  so an external caller can run the probes and feed results back.
- Existing print-after-discovery remains a useful wrapper, accurately described
  as observation-performing but mutation-free.

A complete deferred shell workflow (discovery and decisions execute only when
the generated script runs) is the next compiler target. It needs bindings and
branches beyond the current `And` expression. Return at the established core/SSH
checkpoint with the concrete remaining design/effort before growing that language.
Do not describe resolved-plan emission as complete deferred workflow compilation.

## Coherent implementation checkpoints

1. **Pure core and local runner:** establish explicit inputs, extract execution,
   migrate the CLI and local embedded consumer, and preserve existing behavior.
   Include a runnable runner-free witness and meaningful local parity checks.
2. **Remote execution and embedded handoff:** implement target probes/SSH,
   integrate term-preserver, and exercise a target without tta. Return for
   architectural review with actual interfaces, limitations and evidence.
3. **Deferred shell workflow:** following that review, extend the pure compiler
   if selected, with a clearly declared semantic/runtime contract.

Commit each coherent stage with explicit owned paths. Save a predecessor before
review-driven revision. Update receiving locks and coordinated versions according
to the projects' policy. Do not push or squash. Preserve unfamiliar changes;
initially term-preserver has a preexisting lock refresh and tmax has an unrelated
Beads export. Parent owns ticket coordination for this effort.

## Useful checked-in verification toolkit

Extend the existing verification domain rather than relying on throwaway scripts.
Select reusable components that establish behavior, including:

- A runner-free fixture/replay or example interface exercising observation
  requests, supplied facts, emitted probes, decode, and final commands. It must
  work with no native runtime dependency and prove more than string snapshots.
- Comparable local runner and emitted-shell scenarios observed through independent
  tmux queries/real PTYs: cold creation, existing sessions, grouping and detach,
  startup argv/cwd, scaffold and failure short-circuiting.
- A reusable remote-target harness. Hermetic process-carrier fixtures can test
  protocol and argument boundaries, but must not be called actual SSH evidence.
  Provide an opt-in real SSH exercise or an exclusively owned local sshd fixture
  when available, with explicit target/endpoint configuration and no remote tta.
- Adversarial/error cases at consequential boundaries: nonempty diagnostics,
  malformed/truncated replies, inaccessible paths, hostile supported names,
  connection loss and uncertain mutation disposition, and local/target context
  disagreement. Preserve meaningful status and bounded capture behavior.
- Structured reports with tool versions, owned endpoint/target, selected backend,
  observation outcomes, process/PTY evidence, failure and cleanup results.
- User-facing recipes explaining how to rerun the toolkit and what each result
  actually establishes. Missing external prerequisites are explicit, not success.

Fixtures use explicit absolute owned tmux sockets under `/tmp/opencode` or
project scratch, verify isolation read-only, and never address or clean up the
user's default server. Follow term-preserver's `AGENTS.md` incident rules. Prefer
narrow cleanup; broad server shutdown is permitted only for verified owned fixtures.

Checks should cover relevant feature combinations, nextest, clippy, formatting,
doctests and consumer builds. Keep coverage proportional to actual changes. G
reviews independently and should use the checked-in toolkit to reproduce evidence;
the parent trusts reported checks and adds only checks for specific unresolved
questions. Maintain native session provenance in knowledge documents and run
Rekon document checks on explicit owned Markdown.

### Independent scout findings to carry into implementation

G's initial read-only scout identified these concrete boundaries; it did not
run builds/tests, start an SSH daemon, contact a host, or query a tmux inventory:

- `resolve.rs` currently uses `symlink_metadata(...).is_ok()` for a scaffold
  target. Extraction must correct the conflation of inspection errors with
  absence, not preserve it in a new observation type.
- Existing capture helpers can finish before EOF or block while descendants
  retain pipes. New reusable capture needs explicit completeness and bounded
  stdout/stderr, including the child-exited/pipe-still-open case.
- Probe and handoff must share the consumer's selected shell/environment setup;
  equal SSH destinations alone do not establish equal PATH/cwd/configuration.
- The installed environment contains OpenSSH client/server 10.5p1 and key tools,
  a writable `/tmp/opencode`, and `/run/sshd`. A read-only empty-config inspection
  stopped for lack of host keys. An owned loopback fixture is plausible, but
  startup and authentication remain to be verified. `/usr/local/bin/tta` exists,
  so the no-target-tta witness must inspect its restricted PATH after shell setup.
- Preserve pure query decoder coverage when extracting tmax features. A core-only
  dependency/build witness must be run outside an all-features graph that could
  conceal accidental native dependency activation.

The parent forwarded these findings to Sol as implementation input. Useful
existing witnesses are the satellite suite's independent views/lifecycle and
the launch suite's recording wrapper proving one grouped tmux invocation.

## Agent rotation

The human requests Sol xhigh, rotated after a returned context above 200k tokens
unless only a constrained follow-up remains, ideally kept below 350k. Return at
coherent checkpoints with committed work and a durable handoff; check context
before a long continuation rather than stretching one session indefinitely.

## Connections

- [tta's maintained guide](/README.md#reusable-pieces) owns the eventual current
  API account; this brief records the accepted direction and implementation scope.
- [Original launcher research](research0.gpt6a.md#1-useful-seams-without-a-framework)
  supplies the existing pure-plan and quoting boundaries this work extends.
- [term-preserver launch guide](file:///home/rektide/src/term-preserver/doc/launch/README.md)
  owns consumer behavior and should be updated with the implemented handoff.
- [tmax's tta consumer account](file:///home/rektide/src/tmux-interface-control/doc/consumer/tta.md)
  is the library-side home for the pure vocabulary/runtime feature requirement
  and evidence from this integration.
