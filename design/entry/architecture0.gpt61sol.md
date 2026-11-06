---
type: ArchitectureReview
title: Entry architecture after the faithful xmachines port
description: Independent second-session review retaining the pure graph-driven backbone, tightening typed dispatch and diagram conformance, and measuring immutable replay before considering a redesign
resource: /design/entry/architecture0.gpt61sol.md
status: implemented
author: model:openai/gpt-6.1-sol#high
date: 2026-11-06
sources:
  - resource: opencode://ses_e4dca921dffexww2JZDdY3Zyne
    title: Fresh second Sol reviews the faithful port, implements bounded architectural hardening, measures replay, preserves the restart checkpoint and reports code completion
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commissions the separate architecture stage after the faithful port and reserves the later capability-oriented ticket pitch
  - resource: /design/entry/xmachines-port0.gpt61sol.md
    title: Explicitly supplied previous-stage implementation, preservation evidence, friction and architectural questions
  - resource: /doc/entry/README.md
    title: Maintained overall progression, separate guarded session-resolution diagram and independent-view lifecycle contract
  - resource: /session/launcher/src/workflow.rs
    title: Native supplied facts, immutable pull replay, deferred error boundary and exhaustive stage handlers
  - resource: /session/launcher/src/workflow/machine.rs
    title: Shared compiled graph, typed Stage boundary, ordered guard routes and strengthened diagram conformance tests
  - resource: /session/launcher/src/workflow/entry.machine.json
    title: Unchanged eight-state executable control graph
  - resource: /session/launcher/src/workflow/tests.rs
    title: Native data and thread/clone behavior plus new generator-validation and explicitly absent-default tests
  - resource: /session/launcher/src/resolve.rs
    title: Native session-first and project fallback resolution with ambiguity guarded before continuation
  - resource: /session/launcher/src/workspace.rs
    title: Literal preference, one-step aliases, shortest ranking, actual-group identity and satellite allocation
  - resource: /session/launcher/src/scaffold.rs
    title: Choice/registry validation and creation-prefix planning, including the pre-default adapter validation seam
  - resource: /session/launcher/src/target.rs
    title: Separate target-local context/registry bootstrap and configuration precedence
  - resource: /session/launcher/src/plan.rs
    title: Pure lowering and the indivisible grouped create/handoff/keep-last queue
  - resource: /session/runner/src/driver.rs
    title: Observer failure propagation outside the graph and shared local orchestration
  - resource: /verification/core/README.md
    title: Existing semantic replay witness and newly maintained informational cost-probe instructions
  - resource: /verification/core/examples/replay_cost.rs
    title: Reproducible release-mode timing of immutable ready pulls and complete supplied-facts transcripts
  - resource: /design/launcher/runner-core0.gpt6a.md
    title: Earlier pure-core, optional-runner and coherent-mutation boundaries retained by this later explicitly commissioned statechart
  - resource: /doc/consumer/term-preserver.md
    title: Consumer ownership of explicit grouping, terminal transport, attempts and retry policy
---

# Outcome

**Architecture steering and its bounded implementation are code-complete.** Keep
the selected **`xmachines-runtime =1.1.0`** and its eight-state control graph. Keep
`Workflow::next(&self)`, `supply()`, native facts/errors, and `Clone + Send + Sync`.
The correct backbone is a **pure supplied-facts evaluator**, not a long-lived
tmux actor or an execution scheduler. No broad rewrite is warranted by the
reviewed behavior or measured costs.

This is the deliberately fresh second Sol session, following the
[faithful port report](xmachines-port0.gpt61sol.md), not the port author silently
folding a redesign into that baseline. Improvements are typed native stage
dispatch, stricter ordered diagram checks, two boundary regressions, and a
maintained cost probe. Public APIs, dependencies, feature policy and package
versions are unchanged. No fallback to Statig was attempted.

## The actual backbone and its useful seams

| Layer | What it owns | What it must not absorb |
| --- | --- | --- |
| `target::Bootstrap` / target input lowering | Target context, registry acquisition, path provenance and option precedence | Caller-host path/environment guesses or target mutation |
| `Workflow` + executable graph | Which fact is needed next and when pure resolution/preparation may run | Observation I/O, client identity invention or execution/retry policy |
| Native resolver / workspace matching / scaffold adapter | Guarded selection, alias/group rules and generator expressions over supplied facts | Treating ambiguity or acquisition failure as missing work |
| Plan and optional runner | Literal command lowering, coherent mutation block and actual observation/execution | Parsing shell text to recover semantics or replaying uncertain mutation |

The [overall chart and separate resolution chart](/doc/entry/README.md#input-state-machine)
already express these seams. The main graph chooses stage transitions. The native
`resolve_unique(...)?` guard precedes selected/missing continuation; project
ambiguity likewise precedes destination/scaffold continuation. Errors escape
before a successful control event. Moving candidate loops into statechart actions
would add domain serialization or opaque callbacks without improving these
boundaries.

Completion is deliberately a different read-only use of inventory: it exposes
literal names and aliases without choosing a winner. Execution is also outside
the machine: a ready plan is not a successful launch. This preserves the earlier
[pure-core architecture](/design/launcher/runner-core0.gpt6a.md#observations-and-decisions).
Its warning against a general execution workflow engine remains useful; the
later human-selected graph controls pure entry progression, not a replacement
command executor.

## Decisions: what stays and why

1. **Retain immutable replay.** Each pull constructs an actor over the once-compiled
   shared graph and walks only supplied facts. The implementation is easy to
   clone, concurrently read, replay and fail at the existing pull boundary.
   Persisting an actor is not automatically better: it would need an ownership
   and synchronization policy compatible with `next(&self)` and native clones.
   A memoized result additionally needs ownership of non-Clone typed error chains,
   plan cloning and an explicit account of when validation occurs. These are
   solvable design questions, but no measured requirement justifies taking them
   on now. No persistent actor, mutex, interior cache or eager `supply()` resolution
   was added.
2. **Retain the eight stages and the native resolver.** They correspond to real
   fact boundaries plus resolution/preparation, rather than arbitrary lines of
   code. A session hit still needs no project root. Fresh sessions are a distinct
   observation stage despite sharing the public request variant with the initial
   inventory. The fixed private fact slots currently make that distinction sound;
   there is no need to expand the public observation protocol just to rename it.
3. **Do not deduplicate validation by weakening independently usable APIs.**
   Bootstrap, `Workflow::new`, scaffold resolution and low-level lowering have
   different entry points and error responsibilities. Repeated checks are real
   costs, but not evidence that a low-level caller may skip them. A future private
   prepared-policy object could reduce repetition while retaining each public
   boundary; this pass does not add another representation without a demonstrated
   need.
4. **Keep control-only serde and native domain payloads.** Paths, `OsString`,
   generator function pointers, selections, command expressions and typed errors
   stay in Rust. Unit context and a fixed actor timestamp remain appropriate.
   Embedded graph/protocol mistakes are developer invariants, not fabricated
   user-domain errors. The selected runtime's prerelease semantic target and
   feature-unification cost remain exactly those in the port report.
5. **Keep independent-view safety and lifecycle outside architectural fashion.**
   Fresh ID/name/actual-group agreement still fails closed; no reranking on changed
   facts. One-step orphan aliases still navigate existing work without inventing
   groups. Complete satellite names are checked globally. Grouped create →
   handoff → `keep-last` remains one tmux queue. Nothing in this pass changes
   outside defaults, inside-origin limitations, startup policy or cleanup/retry
   ownership.

No materially consequential fork was taken: no runtime substitution, mutation
boundary change, public progression redesign or new project-root policy required
an unrequested broad implementation.

## Implemented improvements

- **Exhaustive native stage dispatch** in
  [`machine.rs`](/session/launcher/src/workflow/machine.rs) and
  [`workflow.rs`](/session/launcher/src/workflow.rs): graph IDs enter through a
  single private `Stage` mapping. The handler match now covers enum variants
  exhaustively instead of ending in a string catch-all. This is a vocabulary
  boundary, **not a second transition table**. Sending an event also compares
  copied stages instead of allocating the previous state ID string.
- **Exact ordered graph/diagram conformance**, not just finding each expected
  edge somewhere in prose. The test compares all core-to-core routes, exact event
  and guard labels, and per-event transition priority. Negative witnesses reject
  extra labeled/unlabeled routes, missing routes, event-name drift and reversed
  guard order. This is a focused checker for the maintained Mermaid syntax, not
  a general Mermaid parser or proof of the separate native matching algorithm.
  Prelude, native error exits and post-ready execution remain deliberately outside
  its checked executable subgraph. The graph itself did not change.
- **Generator-boundary regressions** in
  [`workflow/tests.rs`](/session/launcher/src/workflow/tests.rs): invalid adapter
  options must fail before requesting generator defaults on creation, while an
  existing-session hit must not validate an unused generator's options. A supplied
  author/license absence is a completed observation, not a reason to request the
  same defaults again.
- **Maintained informational replay-cost probe** under
  [`verification/core/examples/replay_cost.rs`](/verification/core/examples/replay_cost.rs),
  documented in the [runner-free toolkit](/verification/core/README.md#informational-planning-cost-probe).
  No new dependency or production metric/clock read was introduced.

These improvements intentionally do not claim reduced architectural code size:
typed dispatch and stronger tests add explicit vocabulary and verification code.
The benefit is stricter maintenance of the human's code/chart and failure-order
contracts, not making the runtime adapter disappear.

### Commits

| Commit | Coherent stage |
| --- | --- |
| `29edacd9` | Type native graph dispatch and enforce ordered diagram conformance |
| `e84d7b47` | Add a supplied-facts replay cost probe for architecture decisions |
| `a6c265ef` | Protect generator validation and explicit absent-defaults ordering |

The report and maintained navigation form a subsequent documentation checkpoint.
Only explicit owned paths were committed; preexisting `.beads/issues.jsonl`
changes remain outside these commits. No ticket reorganization, install or push.

## Replay cost: evidence rather than an assumed optimization

Measured locally on Linux with `rustc 1.101.0-nightly (822182db4 2026-10-13)`,
release defaults, warmed shared graph, and five timed batches. Small fixtures
used 10,000 iterations per batch; expanded fixtures used 1,000. Values below are
**microseconds**, rounded to one decimal, of the median per-operation batch mean:

| Scenario | Ready `next()`, small | Full transcript, small | Ready `next()`, +1,000 sessions/projects | Full transcript, +1,000 sessions/projects |
| --- | ---: | ---: | ---: | ---: |
| Existing | 8.6 | 13.6 | 9.0 | 44.3 |
| Project | 14.0 | 35.5 | 366.8 | 1,452.9 |
| Create | 47.0 | 117.6 | 1,003.2 | 2,698.0 |
| Grouped | 16.1 | 31.5 | 16.5 | 96.3 |

The extra supplied names do not match the requested fragment. Existing/grouped
fixtures intentionally retain their exact literal hit; the project dimension is
unused on those branches. Full transcript timing includes workflow construction,
input/reply cloning, caller pulls, `supply()`'s own pending-request validation,
and plan destruction. Ready pulls still recreate the actor and plan. Fixture
construction, shared compilation and warmup are excluded. These are neither
tail latencies nor isolated runtime costs, a before/after speedup comparison,
large-inventory worst cases, nor live tmux/SSH measurements.

**Interpretation:** small fixtures do not reveal a reason to complicate state
ownership. Larger miss/create paths confirm meaningful inventory-sensitive cost;
they do not establish that actor persistence is the right first optimization.
Within one fully supplied creation pull, `resolve` is entered four times and
generator expressions are built first without and then with supplied defaults.
Repeated external pulls and `supply()` amplify that work. Trace event multiplicity
also remains; immutable pulls are not unique operational attempts.

Simply deferring all generator planning until after defaults would alter error
ordering: invalid adapter options currently fail before a target Git/environment
request. The new regression makes that obligation explicit. If measured workloads
later require improvement, separating **adapter option validation** from
**expression materialization**, or reusing a native selection within one replay,
is a narrower candidate than retaining a synchronized actor. Neither candidate
is implemented or presumed necessary here.

Reproduce the informational runs from the checkout:

```sh
CARGO_TARGET_DIR=/tmp/opencode/tta-lucky-sailor-target cargo run --offline --release -p tta-core-witness --example replay_cost -- --iterations 10000
/tmp/opencode/tta-lucky-sailor-target/release/examples/replay_cost --extra-sessions 1000 --extra-projects 1000 --iterations 1000
```

CSV receipts are `/tmp/opencode/tta-architecture-cost-small.csv` and
`/tmp/opencode/tta-architecture-cost-large.csv`. They are ephemeral receipts;
the checked-in probe, methodology and table are the durable account.

## Verification and restart recovery

The server interrupted this session with a full-disk database failure after the
source edits were saved. On resumption, the changes and completed test result
were recovered, not redone. No owned commit had yet been made. All subsequent
new Cargo output went to the separate tmpfs target
`/tmp/opencode/tta-lucky-sailor-target`; no unrelated artifacts were deleted and
the original target tree was not relocated. The interruption is not a code-test
failure or an unresolved implementation blocker.

| Check | Actual result |
| --- | --- |
| `cargo nextest run -p tta-launcher --all-features -j 1` before interruption | **43/43 passed**, including the typed dispatcher and stricter diagram tests |
| Two subsequently added generator-boundary tests, selected with `-E 'test(generator_validation) \| test(absent_generator_defaults)'` | **2/2 passed**, 43 existing launcher tests deliberately skipped |
| Package-only `cargo nextest run --offline -p tta-core-witness -j 1` | **7/7 passed**, including all semantic scenarios, changed fresh facts and wrong/truncated observations |
| All-target, all-feature release clippy on `tta-launcher` and `tta-core-witness`, `--no-deps -- -D warnings` | Passed; existing dependency-owned tmax warnings remain |
| Release launcher-test clippy after adding the two new tests | Passed with the same dependency warning boundary |
| `cargo fmt --all --check` after the final source edits | Passed |
| Cost probe: both workloads above | Passed; all four scenarios yielded ready plans |
| Cost probe: `--iterations 0` | Rejected by Clap with exit 2, as intended |
| Newly emitted four-scenario JSON report versus the retained faithful-port report | **`cmp` byte-identical**, including shell text, selections, observations and probes |
| `rkt check --okf` on the report, entry contract, replay toolkit and root guide | Passed: **4 documents, 87 links, 68 sources** |

The comparison used the witness's `--report` output on both sides, avoiding its
stdout-only extra newline. Current receipt:
`/tmp/opencode/tta-architecture-core.json`; retained baseline:
`/tmp/opencode/tta-xmachines-after.json`. Remaining check logs use
`/tmp/opencode/tta-architecture-*.log`. The Markdown check verifies structure,
local destinations, anchors and provenance, not remote content or semantic claims.

The faithful port's full workspace feature matrix, eight real-terminal cases,
15 SSH checks, syscall poison witness, emitted-probe round trips and receiving
term-preserver tests are [prior evidence](xmachines-port0.gpt61sol.md#verification),
not reruns claimed for this session. Repeating them was not warranted for an
internal dispatcher representation and tests with no graph, observer, lowering,
feature or public API changes. No new consumer lock/source change is required.

## Remaining recommendations and handoff

- **Proceed to the parent's Astra medium capability/surface-oriented ticket
  pitch.** This architecture pass has no pending code or decision gate. The pitch
  is a proposal before any ticket reorganization; it must retain existing history
  and distinguish user capabilities from implementation evidence. This session
  did not dispatch that stage or modify tickets.
- **Keep `tta-project-roots` separate.** Its real design questions are ordered
  roots, duplicate names/physical identity and unambiguous scaffold destination,
  not which actor library to use. Preserve the session-first error guard and
  rootless hits; do not flatten away root provenance in an anticipatory generic
  workflow rewrite. No multi-root behavior or abstraction was added here.
- **Use the cost probe if a performance obligation appears.** Decide the workload
  and acceptable cost first. Consider validated native policy/adapter separation
  before synchronized persistence, while preserving error chains, pull timing and
  changed-fact fail-closed rules. There is no performance ticket or commitment
  implied by this recommendation.
- **Keep execution, completion and inside-client origin separately scoped.** The
  [consumer account](/doc/consumer/term-preserver.md) already assigns transport,
  attempts and retry to the consumer. No new machine ownership of these concerns,
  auto-pruning, auto-retry or backend fallback is authorized by this review.

The separate session-resolution diagram was reviewed against the unchanged native
algorithm and remains honest. Its exact-literal fast path, alias/shortest ranking,
actual-group collapse and guard-before-`Some`/`None` structure were not replaced
with machine states merely to make the graph larger.
