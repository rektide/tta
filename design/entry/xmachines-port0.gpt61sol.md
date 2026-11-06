---
type: ImplementationReport
title: Faithful XState 6 Rust entry port — implementation and architecture handoff
description: Native supplied-facts workflow port through xmachines-runtime 1.1.0, preserved API and behavior, verification and concrete friction for a different fresh architecture session
resource: /design/entry/xmachines-port0.gpt61sol.md
status: implemented
author: model:openai/gpt-6.1-sol#high
date: 2026-11-06
sources:
  - resource: opencode://ses_e4de0f8b5ffeW5ODAtJx9rWv3u
    title: Establish the target, receive Rust-runtime authorization, implement the faithful graph-driven port, compare pre-port replay and verify native boundaries
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Human selects xmachines-runtime, retains Statig as a possible fallback and reserves architecture steering for another fresh Sol
  - resource: /design/entry/xstate6-target0.gpt61sol.md
    title: Initial package/version evidence and consequential-boundary decision gate
  - resource: /session/launcher/src/workflow/entry.machine.json
    title: Executable eight-state entry graph with ordered defaults and satellite guards
  - resource: /session/launcher/src/workflow/machine.rs
    title: Shared compiled graph, thin event protocol and graph/diagram regression tests
  - resource: /session/launcher/src/workflow.rs
    title: Immutable pull adapter, native handlers, supplied facts and typed failure boundary
  - resource: /session/launcher/src/workflow/tests.rs
    title: Native data isolation, thread traits, clone independence, error ordering and policy tests
  - resource: /scaffold/rekon-native/src/lib.rs
    title: Stable package key emission despite preserve_order feature unification
  - resource: /verification/core/src/lib.rs
    title: Existing runner-free four-scenario replay and native observation sequencing checks
  - resource: /verification/core/poison.py
    title: Poisoned-environment, canary, dependency-isolation and syscall-trace witness executed after the port
  - resource: /verification/views/README.md
    title: Maintained eight-case real-terminal matrix exercised by the all-feature integration suite
  - resource: /verification/ssh/README.md
    title: Actual owned loopback SSH toolkit rerun successfully after the port
  - resource: /verification/probes/README.md
    title: Package-only emitted-probe round-trip toolkit rerun after the port
  - resource: file:///home/rektide/src/term-preserver/tests/launcher.rs
    title: Receiving embedded/CLI integration suite inspected and exercised with the updated dependency lock
  - resource: /doc/entry/README.md
    title: Updated overall graph and separate unchanged native session-resolution algorithm
  - resource: https://docs.rs/crate/xmachines-runtime/1.1.0
    title: Selected independent runtime's alpha.71 target, API and pure host-driven contract
---

# Outcome

The first-stage **mostly faithful port is implemented**, without blending the
later architecture redesign into it. The target is exactly
**`xmachines-runtime = "=1.1.0"`**, whose upstream semantic target is
**`xstate@6.0.0-alpha.71`**. This is the human-selected independent Rust runtime,
not official Stately beta execution and not a silent Statig substitution.

The embedded MachineJSON graph now determines entry-stage transitions and
ordered policy guards in production. Rust dispatches the active state, performs
its pure domain computation, and sends outcome events. There is no legacy
conditional progression running beside an ornamental machine. Session matching,
scaffold resolution and launch lowering remain native functions called by the
appropriate stage; their internals were deliberately not redesigned.

## Boundary and interface

`Workflow` still owns its `Input` and five optional observation slots. A call to
`next(&self)` creates an actor over a **once-compiled shared machine**, replays
the already supplied facts until the first unavailable observation, and returns
that request or the ready plan. It never executes an observer. `supply()` still
checks the current request through `next()` before accepting a correctly typed
reply. It does not eagerly resolve newly supplied facts; errors remain visible
at the same public pull boundary.

The replay traverses the real graph, including observation → resolve return
edges. Native local references reveal project/destination/default facts only
when their graph stage requests them. An actor is **not** retained in the public
workflow: no locks, interior mutability, persistent actor snapshots or public
change to `next` is needed. This preserves **`Clone + Send + Sync`**, including
independent cloned facts and simultaneous immutable pulls from two threads.

Only a private event discriminator and the `needs_defaults` / `needs_satellite`
booleans cross the runtime's serde event boundary. Its context is unit. There is
no serialization of native `Input`, `OsString`, paths, sessions, registry/generator
function pointers, plans or typed errors. Tests drive non-UTF-8 native program
and startup values through the workflow successfully; shell rendering continues
to reject such data explicitly rather than lossy-convert it.

The graph has **no actions, invocations, timers or external effects**. Actor time
is a fixed zero, never a system-clock read. Domain failures propagate from Rust
through the original `?` boundaries, preserving typed source chains. Ambiguity
does not become a failed/ambiguous main-flow state or a missing outcome. Graph
initialization/protocol invariants use assertions/`expect`: a malformed embedded
program is a developer failure, not user configuration. Normal domain errors do
not enter that path. Graph compilation, every transition route and guard order
are exercised by regression tests.

**Public API changes: none.** Existing input, observation, progress, error and
driver types/signatures remain unchanged; workspace packages remain `0.6.0` for
this internal-only change. The new dependency requires **Rust 1.99+**, now stated
in the guide. No Node, JS evaluator or extra target-side executable is required.
The CLI/runner, target bootstrap, completion inventory, SSH probes and execution
queue APIs were not replaced.

## Preserved behavior and one real correction

The same validation precedes the initial session request. Session resolution
still guards ambiguity before `Some`/`None`, requires no project root for a hit,
and uses exact literal preference, one-step satellite aliases, shortest Unicode
ranking and actual-group tie collapse. Only genuine session absence proceeds to
projects; project ambiguity precedes missing/scaffold continuation.

Outside omitted grouping still requests an independent view; inside omitted
grouping still switches normally. Explicit grouping and custom prefixes are
honored. Independent existing entry still requires fresh selected ID/name/group
agreement and the same globally unused complete satellite name. Group creation,
terminal handoff and keep-last remain one tmux queue. Startup remains new-session
only. Read-only shell preparation, literal cwd/argv, classified absence, observer
errors and no automatic retry/cleanup retain their previous boundaries.

The before/after supplied-facts comparison caught a dependency-wide side effect:
xmachines enables `serde_json/preserve_order`. This initially reordered generated
`package.json` keys, changing shell bytes despite equivalent content. The native
scaffold adapter now explicitly sorts its package keys and has an exact-output
regression. After that correction, **all four complete pre-port and post-port
reports compare byte-for-byte**, including observation records, selection,
scaffold shell text and emitted probes. This comparison is stronger than simply
parsing the generated JSON and overlooking changed emission.

## Commits

- `8a1016b0`: establish target/runtime gate and separate native session diagram.
- `b5c8492a`: pin the selected Rust runtime and execute the entry graph in pure
  supplied-facts progression; graph route/guard tests.
- `0b08b704`: add immutable pull, clone/thread traits, typed error ordering,
  matching/policy and non-UTF-8 isolation regression tests.
- `30c68814`: correct feature-unification emission drift and assert exact sorted
  package output.
- The subsequent documentation checkpoint aligns the overall diagram with the
  executable transitions and adds a parsed-graph/diagram conformance test.
- Receiving repository `term-preserver` commit **`28e4a8d8`** adopts the new
  transitive runtime in `Cargo.lock`; no integration source change was needed.

Each stage committed explicit owned paths. Preexisting `.beads/issues.jsonl`
work is excluded; no install or push was performed.

## Verification

The final all-feature workspace run passed **173/173** nextest tests, including
all eight native/CLI × direct/shell × default/custom-prefix real-terminal cases
inside the two independent-view integration tests. Every executable graph stage,
transition and guard is represented in the maintained overall diagram and checked
against the parsed definition by a regression test. Explicit-package formatting
and `rkt check --okf` of all four changed knowledge documents also passed.

| Verification | Result |
| --- | --- |
| Workspace all features, `nextest -j 1` | **173/173** |
| Workspace native defaults, `nextest -j 1` | **157/157** |
| Workspace CLI-only, `nextest -j 1` | **140/140** |
| All-feature doctests | Passed (2 executable doctests) |
| All-target clippy, all features and CLI-only, `--no-deps -- -D warnings` | Both passed; dependency-owned tmax warnings remain |
| Real terminals, tmux **3.8** | Eight combinations passed through the native/CLI integration cases; orphan survival and independent window selection retained |
| Genuine private loopback SSH, OpenSSH **10.5p1** | **15/15** checks; cleanup succeeded; target needs no tta/JS executable |
| Package-only supplied-facts script | Passed, four scenarios |
| Poison witness with actual available `strace` | Four scenarios × two disagreeing environments, identical output, no canary execution, no target/network/child I/O |
| Package-only emitted-probe round-trip | **7/7** checks, expected invalid-UTF-8 and ENOTDIR rejection; no launch action |
| Receiving `term-preserver` | All-target/all-feature check and **35/35** all-feature launcher tests passed |

Representative reruns (from the tta checkout):

```sh
cargo nextest run --workspace --all-features -j 1
cargo nextest run --workspace -j 1
cargo nextest run --workspace --no-default-features --features tta/cli -j 1
cargo test --workspace --all-features --doc
cargo clippy --workspace --all-targets --all-features --no-deps -- -D warnings
cargo clippy --workspace --all-targets --no-default-features --features tta/cli --no-deps -- -D warnings
cargo build -p tta --all-features
cargo run --quiet -p tta-verify --bin tta-verify-ssh -- --tta target/debug/tta --report /tmp/opencode/tta-xmachines-ssh.json
./verification/core/check.sh --report /tmp/opencode/tta-xmachines-core.json
python3 verification/core/poison.py --trace --report /tmp/opencode/tta-xmachines-poison.json
./verification/probes/check.sh --report /tmp/opencode/tta-xmachines-probes.json
```

Logs are under `/tmp/opencode/tta-xmachines-*.log`; structured runtime reports
use the paths above. Workspace tests were deliberately serial, preserving the
existing fixture-acquisition caution rather than introducing a production retry.

The retained pre-port report is `/tmp/opencode/tta-xmachines-before.json`; the
post-port report is `/tmp/opencode/tta-xmachines-after.json`. `cmp` succeeds after
the sorted-emission correction. These paths are ephemeral run receipts, not
portable sources. The maintained tests/toolkits and session source carry the
durable evidence.
Both complete replay reports have SHA-256
`2e3b535f414eb5ee5a60cd6ebd2ca3e6590e33fc04b8a5a00c77cb902dded5ea`.

## Fit, friction and recommendations for the second fresh Sol

**My lean: keep xmachines for the next review, rather than fallback now.** It
really executes a small, inspectable graph, allowed the current pure/native
contract to survive without a JS boundary, and made guard priority explicit.
No unsupported runtime feature was needed. The concrete initial schema errors
also demonstrated a useful strict boundary: MachineJSON requires explicit
`{ "target": ... }` transitions and `{ "type": ... }` guards, not familiar
JavaScript shorthand. Fixing those definitions was ordinary integration, not a
reason to replace the runtime.

The cost is visible. This was not a code-size simplification: a small native
conditional workflow became a graph, private serde event layer, native state
dispatcher and integration invariants. The runtime interprets JSON events and
allocates actor state even though native resolution remains ordinary Rust.
The new `serde_json` feature coupling had a real emitted-output effect and had
to be neutralized. Pinning exact `1.1.0` is deliberate: this is a very young
independent runtime targeting a prerelease semantic version, not a claim of
Stately beta compatibility or broad ecosystem maturity.

The deliberate replay adapter preserves the requested API and meaning; it does
**not** optimize work. `next()` now revisits native resolution at several replay
stages, and `supply()` itself calls `next()`. Repeated pure computation and trace
event multiplicity can increase. No latency, binary-size or performance claim is
made. Before making a persistent actor or caching decisions, measure this cost
and decide which interface properties actually matter. Such a change must
preserve request ordering, native error chains and changed-fact fail-closed rules.

Concrete review questions for the next independently commissioned session:

1. Is immutable replay still the right semantic/public boundary, or would an
   explicitly mutating event API better express the intended lifecycle? Do not
   silently trade away `Clone`/`Sync` or add mutex-backed domain state merely to
   hold an actor.
2. Are eight progression states the useful granularity? Matching is still a
   native guarded algorithm, not an actor per candidate operation. Keep its
   separate diagram and main-flow error guard, whichever decomposition is chosen.
3. Should the native handler integration become a more typed adapter? Current
   state IDs and payload `Option`s are guarded embedded-program invariants;
   introducing another transition table would be regression, not improvement.
4. Where should validation and selection be owned across target bootstrap,
   workflow, scaffold wrapper and low-level resolver? This port retains their
   existing duplication intentionally; moving them now would have mixed redesign
   with the faithful baseline.
5. Keep completion and execution outside this machine unless a new requirement
   actually changes that boundary. An observation scheduler is not authorization
   to mutate, prune, retry or solve inside-client origin ambiguity.

**Statig remains a plausible fallback, not a tested alternative here.** The
implementation's native domain data and tiny control-only JSON layer identify
the relevant tradeoff: if an executable interoperable graph is not valuable
enough to justify runtime/serde adaptation, evaluate a Rust-first typed state
machine against these exact preservation tests. Do not assume it automatically
solves immutable pulls, clone semantics, documentation alignment or ownership.
No Statig dependency/API evaluation or competing implementation was commissioned
or performed, so this report does not claim a verified advantage for it.

Multi-root discovery, inside-origin switching, automatic pruning, ticket
reorganization and the second architecture redesign remain out of scope. The
parent owns the next **different fresh Sol** commission and any later
capability-oriented ticket pitch; this session did not dispatch either.
