---
type: ArchitectureCheckpoint
title: Pure core and optional local runner implemented
description: Committed interfaces, local and runner-free verification, consumer migration, and the remaining target-setup and SSH observation slice
resource: /design/launcher/runner-core1.gpt61sol.md
status: implemented-checkpoint
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e80e6e9a9ffe2N24sOOfQnrL3D
    title: Implement the first commissioned milestone, extract optional runtime adapters, migrate the local embedded consumer and add runner-free replay verification; return for rotation after crossing 200k context
  - resource: opencode://ses_e80f5d05bffe77stycHGRJOH29
    title: Parent commission and independent G scout findings on filesystem errors, capture completeness, target shell setup and real SSH fixture feasibility
  - resource: /design/launcher/runner-core0.gpt6a.md
    title: Accepted implementation brief and checkpoint sequence
  - resource: /design/launcher/remote0.glm53max.md
    title: Remote alternatives and terminal/transport constraints
  - resource: /design/launcher/launch-backends1.gpt61sol.md
    title: Preserved invoking-client identity uncertainty
  - resource: /session/launcher/src/workflow.rs
    title: Actual pure input, observation and progression interfaces
  - resource: /session/runner/src/driver.rs
    title: Actual optional local driver and shared prepare API
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Embedded consumer migrated to the shared driver and prepared launch execution
---

# Checkpoint outcome

The first milestone of [the commission](runner-core0.gpt6a.md) is implemented,
not just proposed. Work anchor: **tta-launch-backends-remote-target**; ticket
coordination remains with the parent. The SSH/probe milestone and complete
deferred shell workflow are **not** implemented or claimed here.

## Commits and ownership

| Repository | Commit | Outcome |
| --- | --- | --- |
| tmax | `7104042c` | `operations` feature, implied by `client`; version 0.6.0; no parser retirement |
| tmax | `898668c2` | Existing pure query-decoder tests run without `client` |
| tmax | `04ec6ea1` | Library-side tta consumer account updated |
| tmax | `a2d1c0e0` | Scoped formatting of the new pure query-test helper |
| tta | `a3ed1450` | Pure core/command/generators, optional local runner, shared CLI driver and runner-free toolkit; coordinated version 0.3.0 |
| term-preserver | `6317dcdc` | Shared local tta driver, completion migration and receiving lock update |

Term-preserver's preexisting lock refresh is naturally included in its receiving
lock update. Concurrent **ControlMaster** work appeared during this session in
`src/launch.rs`, `src/config.rs`, `src/launcher.rs`, tests and launch docs. Only the
runner-migration hunks were committed after a full-ID `jj-hunk` dry-run; the
ControlMaster work remains its owner's. The initial tmax Beads export was neither
changed nor committed by this work.

A workspace formatting invocation touched unrelated term-preserver files. The
formatter-only token deltas were identified and restored on explicit paths;
`tests/launcher.rs` had acquired concurrent semantic edits and was preserved,
including remaining formatting. Do not bulk-restore that shared file.

## Actual interfaces

- [`tta-launcher::workflow`](../../session/launcher/src/workflow.rs): `Input`
  owns explicit **target-local** cwd, endpoint/client mode, root, tmux executable,
  tmux config, parsed scaffold registry/choice and entry options. `Workflow::new`
  validates registry choice before session observation. `next()` yields
  `Progress::Observe(ObservationRequest)` or `Ready(Box<PreparedLaunch>)`;
  `supply(ObservationReply)` rejects a reply of the wrong kind without advancing.
- Requests cover session inventory, canonical root/immediate project inventory,
  exact destination state, required generator defaults, and a fresh session
  inventory for satellite naming. `SessionInventory::{Absent, Present}` keeps
  classified absence distinct from a live empty inventory. Observer errors stop;
  they cannot be supplied as empty or absent facts.
- [`resolve`](../../session/launcher/src/resolve.rs) is now pure and yields a
  selection or the next missing project/destination fact. Existing session entry
  needs no usable project root. Registry parsing is `scaffold::Config::parse`;
  `None` text means explicitly observed missing configuration, never a read error.
- [`LaunchPlan`](../../session/launcher/src/prepared.rs) exposes the full CLI
  expression, socket and semantic `NativeCreation` candidate independently of
  runner features. The runner never parses shell source to recover a mutation.
- [`tta-command`](../../shell/command/src/lib.rs) contains only literal command
  data/rendering. `Expression::steps()` exposes ordered leaves. Runtime
  `execute`/`process`, and optional `Execute`/`ProcessCommand` extension traits,
  live in [`tta-runner::command`](../../session/runner/src/command.rs).
- Native generator planning takes explicit `Defaults`. Local default acquisition
  is selective: only clean creation and only missing author/license inputs.
  Git failure other than a clean unset identity is an observation error, not an
  absent author. Invalid UTF-8 default environment values fail explicitly.
- [`tta-runner::driver`](../../session/runner/src/driver.rs) supplies `Observer`,
  `LocalObserver`, `drive`, and the `prepare()` builder shared by CLI and consumer.
  [`execution::ExecuteLaunch`](../../session/runner/src/execution.rs) owns native
  preflight/process execution and Run/Exec handoff. A fresh absent/sessionless
  server chooses CLI before mutation; failed native mutation never triggers a
  retry, fallback or cleanup. Grouped create → attach/switch → `keep-last` stays
  in one real tmux queue.

No core runtime/environment/filesystem acquisition remains. Context and registry
acquisition are **still an explicit caller/runner prelude**, not yet observation
variants of `Workflow`. Local physical endpoint identity is observed by the
runner; core socket policy compares supplied identities. Client-origin ambiguity
from the prior checkpoint remains unchanged, not solved by this extraction.

## Verification and reusable recipes

```sh
./verification/core/check.sh --report /tmp/opencode/tta-core-report.json
cargo nextest run --workspace --all-features
cargo nextest run -p tta -p tta-runner --no-default-features --features cli
cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings
cargo test --workspace --all-features --doc
```

Results in this session:

- All-feature nextest: **128 passed**. CLI-only package-selected nextest:
  **62 passed**. This includes real tmux/PTY cold/existing/inside/grouped paths,
  independent windows/detach, literal argv/cwd, native uncertainty ordering,
  scaffold short-circuiting and generic direct-vs-shell process/file outcomes.
- Pure-core package tests and doctests passed; all workspace doctests passed.
  Owned tta package formatting and workspace deny-warnings clippy passed.
  tmax retains preexisting dependency warnings, outside this task's cleanup scope.
- [`verification/core`](../../verification/core/README.md) runs with no runtime
  dependency. Its script separately checks the no-default launcher, rejects
  runtime packages in the witness dependency tree, replays all four scenarios,
  and emits structured reports. Actual report: Existing 1 observation/4 checks;
  Project 2/4; Create 4/5; Grouped 2/5. All say `execution_attempted: false`.
- tmax operations-only witness and **20 existing query-decoder tests** passed.
- Consumer all-target/all-feature check passed; embedded-only all-target check
  passed. Its launcher integration tests passed **31 cases**, then the remaining
  **4 scaffold cases** after rebuilding the sibling CLI with adapters. The first
  scaffold run hit a deliberately slim CLI binary left by the parallel CLI-only
  build, not an embedded failure. The all-feature CLI was restored afterward.
- Rekon checks passed on maintained tta/toolkit/consumer Markdown. Real fixture
  operations use explicit owned sockets and read-only absence proof. The label
  policy test now uses an explicitly labelled carrier simulation which always
  gives the real tmux `-S`; it does **not** claim to verify native `-L` bootstrap.

## Next coherent slice: target setup, probes and SSH handoff

1. Add an explicit **target setup** contract: selected login/interactive shell,
   startup environment, invocation cwd and explicit overrides must be identical
   for context/config/session probes and final terminal action. Plain `sh`
   probes cannot stand in for term-preserver's configured shell setup. Never
   apply local HOME, cwd, UID, root, config or PATH defaults to a remote target.
2. Make the context/registry prelude inspectable, emit/decode observation probes,
   retain typed tmax schemas and bound framing. Startup noise, malformed/truncated
   payloads, inaccessible paths and nonempty failure diagnostics must fail closed.
   Do not parse `ls`; only `NotFound` authorizes a fresh destination. The local
   destination adapter now enforces this, including dangling symlinks.
3. Implement shared **bounded complete-EOF capture** before SSH observations.
   The old driver's 100ms post-exit read grace and the consumer's blocking drain
   are not suitable precedents: require complete EOF, retain status/diagnostics,
   and handle descendants holding pipes beyond deadlines explicitly. The migrated
   local CLI lookup still has its old unbounded `Command::output` limitation;
   no improved capture contract is claimed yet.
4. Produce one coherent resolved target command block and prepared terminal
   handoff for the consumer's attempt worker. Preserve queue ordering, explicit
   remote attach policy and post-mutation uncertainty. The consumer still owns
   proxy/PTY/retry; no blind replay of uncertain creation.
5. Extend the checked-in toolkit with transport fixtures and actual opt-in SSH
   evidence. The G scout found OpenSSH 10.5p1 client/server/key tools installed,
   `/run/sshd` present and `/tmp/opencode` writable. No sshd was started here.
   An owned rootless fixture appears feasible. Restrict PATH **after selected
   shell setup** to prove no target tta: `/usr/local/bin/tta` exists on this host.
   System tmux is 3.8; a pinned 3.7c fixture is available at
   `.test-agent/launch-backends/tmux37c/tmux` if needed.
6. Bring **mosh's separate SSH observation capability** to parent/human decision;
   do not pretend it is an exec channel. The subsequent complete deferred shell
   compiler still needs bindings/branches/captures and a separately reviewed
   contract. Supplied-facts or live-discovery emission is not that compiler.

The session context readout crossed **212k** while resolving the concurrent
consumer edits. This is the requested rotation checkpoint, not a reason to start
the transport implementation in this context.
