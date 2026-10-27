---
type: ImplementationBrief
title: Runner-free planning and remote execution
description: Commissioned crate boundaries, implementation sequence, and reusable verification outcomes for observation-driven tta and its term-preserver consumer
resource: /design/launcher/runner-core0.gpt6a.md
status: implemented
author: model:openai/gpt-6-astra#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e80f5d05bffe77stycHGRJOH29
    title: Accepted commission and architectural stewardship; parent reviews local and SSH interfaces, rotates agents, selects and checks finishing corrections, and closes the delivered pure-core/SSH scope
  - resource: opencode://ses_e80e6e9a9ffe2N24sOOfQnrL3D
    title: Sol xhigh implements the pure core and optional local runner, migrates the consumer and returns for rotation above 200k context
  - resource: opencode://ses_e80ce13ddffeiWXfWVMBW9qXv5
    title: Rotated Sol xhigh implements target setup, framed probes, bounded capture, SSH, embedded remote handoff and actual loopback SSH toolkit; returns at roughly 343k context
  - resource: opencode://ses_e8088432bffeXANRjlyh66v273
    title: GLM 5.3 max independently verifies the SSH milestone, identifies bounded followups and contributes a runner-free full probe round-trip witness
  - resource: opencode://ses_e804f36ddffepWs5FSeXN6VDBv
    title: Fresh Sol xhigh completes shared diagnostics, configurable limits, guide/fixture corrections and deliberate promotion into verification/probes
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

**Delivered:** the pure core, optional process/native/SSH runner, CLI and embedded
SSH consumer, and maintained verification toolkits. Current use belongs to the
[tta guide](/README.md#ssh-targets) and
[consumer launch guide](file:///home/rektide/src/term-preserver/doc/launch/README.md).
The commission and checkpoint sequence below preserve how that result was reached;
the completion section records final review corrections and verification scope.

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

The human requests Sol xhigh and a general rollover once returned context exceeds
200k tokens: use a fresh session for the next substantial assignment. The human
clarifies that the current agent should finish its coherent slice comfortably;
crossing 200k is not an interruption threshold, and finishing headroom below 350k
is welcome. A constrained finishing correction may remain in the same session.
Return at coherent checkpoints with committed work and a durable handoff; check
context before a long continuation rather than stretching one session indefinitely.

## Local checkpoint review and next commission

The [first implementation checkpoint](runner-core1.gpt61sol.md) records the
committed pure core, optional local runner, tmax `operations` feature, CLI and
embedded local migration, and runner-free replay toolkit. The parent read the
actual workflow, runner driver, semantic plan, target/socket and consumer
interfaces and accepts that structure for the next slice. G independently
reviews the committed local milestone; its findings may require focused corrections.

Context/config acquisition still precedes `Workflow` as caller/runner setup;
probe emission, SSH and complete-EOF bounded capture are not delivered by that
checkpoint. These are explicit next work, not capabilities inferred from the
presence of an observation enum. A fresh Sol xhigh was commissioned after the
first returned at roughly 222k context tokens. Prior checks are reported evidence
to build on, not a reason to repeat unchanged suites.

The next commission includes an immediately usable SSH CLI/embedding path,
inspectable target setup, framed observation emission/decoding, shared bounded
capture, a prepared remote terminal handoff, and real owned loopback SSH toolkit
evidence with no target tta. Consumer ControlMaster changes made concurrently
must be respected; probes and handoff share the configured SSH options.

For this SSH milestone, retain the current mosh carrier if its distinct SSH
observation channel cannot be integrated coherently. Document that limitation
and return the remaining choice; do not degrade current mosh or present it as an
exec transport. Complete deferred shell-workflow compilation remains the later
compiler checkpoint. A new remote interpreter/worker requirement must be raised
explicitly rather than silently added to the accepted target tool profile.

## SSH checkpoint and independent review

The [second implementation checkpoint](runner-core2.gpt61sol.md) records the
working SSH milestone and exact commit ledger. The parent read the actual
`TargetSetup`, supplied target facts/bootstrap and `PreparedRemote` interfaces:
setup is now independently inspectable and pure; the runner drives setup and
workflow, retains observation evidence, and returns one explicit terminal command.
This fulfills the selected local-decisions/remote-commands structure without a
target tta or worker binary. The declared POSIX-shell plus GNU-find/stat profile
is a concrete Unix-tool contract rather than a claim of universal shell portability.

The implementer reports 147 workspace tests, 20 focused checks, 44 consumer
launcher cases, and an actual private-loopback SSH run with 14 checks, 66 recorded
API probes and 7 PTY scenarios. The target's post-startup `command -v tta`
returned 127. These are implementation evidence, not independent review. The
checked-in [SSH toolkit](/verification/ssh/README.md) owns the rerun recipes and
explicit limits. LC1/LC2 and meaningful reviewer scratch witnesses have been
integrated into maintained verification homes.

Sol returned at roughly 343k context after finishing the coherent slice. That
session was retired. A fresh GLM 5.3 max independently reviewed the committed remote
milestone with concern-driven checks, trusting existing reported runs rather than
duplicating broad suites. Its [review](/verification/reviews/ssh0.glm53m.md) found
no correctness blocker and contributed a full public-core probe round-trip witness.
Mosh's separate observation channel, complete deferred workflow compilation, and
additional real master/shell-profile coverage remain distinct followups.

## Completion and bounded finishing

A third Sol xhigh completed the selected review followups and returned at roughly
182k context. The parent checked the shared diagnostics API, front-end plumbing,
maintained witness separation and source-preserving promotion records.

- **S1:** `ssh::write_diagnostics` shares failure/success evidence reporting between
  tta and term-preserver. Failed preparation reports retained capture/framing and
  cleanup evidence while preserving the primary error; a worker regression proves
  no handoff. tta `ddd2764f`, consumer `f92ef101`.
- **S2:** positive per-probe timeout milliseconds and per-stream retained channel
  bytes now reach capture from both front ends, with explicit configuration
  precedence. Defaults remain 10000 ms and 262144 bytes. tta `7c71da11`, consumer
  `4148285a`; coordinated versions are tta 0.5.0 and term-preserver 0.2.0.
- **S3–S5:** guides distinguish encoded-channel retention from decoded diagnostic
  budget and state Fish/Nu execution limits; the no-target-tta fixture accepts the
  supported shells' absence statuses with empty output. tta `ebbb0972`, consumer
  `416045c1`; regression lint correction `72a373a4`.
- **Witness promotion:** tta `5f6bdfd1` moves the single reviewed implementation
  into [verification/probes](/verification/probes/README.md), with a reusable
  library, CLI, dependency-isolation hook and reports. The historical review home
  retains revision binding and provenance. Recipes updated in `461c9374`.

The finishing pass reports seven targeted regression tests passed, the promoted
witness passed **7 checks / 22 observations / no runner dependency**, and the
rebuilt pure poison witness passed **4 scenarios × 2 environments** with identical
output and an actual syscall trace. Relevant builds, clippy, formatting, doctest
commands and document checks passed. It did not repeat the earlier broad suites
or actual SSH run; their results remain attributed to the earlier checkpoints.

Maintained toolkit entrypoints:

```sh
./verification/core/check.sh --report /tmp/opencode/tta-core-report.json
./verification/probes/check.sh --report /tmp/opencode/tta-probe-report.json
cargo run -p tta-verify --bin tta-verify-ssh -- \
  --tta target/debug/tta --term-preserver ../term-preserver/target/debug/tper \
  --report /tmp/opencode/tta-ssh-report.json
```

The [core toolkit](/verification/core/README.md) proves supplied-facts planning
without runtime I/O, the [probe toolkit](/verification/probes/README.md) proves
caller-driven observation execution without linking the runner, and the
[SSH toolkit](/verification/ssh/README.md) owns real transport/PTY/action evidence.
This closes the commissioned pure-core and SSH-target implementation. It does
not promote resolved-plan emission into a whole-workflow compiler or claim the
separately recorded mosh/master/shell-profile followups are implemented.

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
