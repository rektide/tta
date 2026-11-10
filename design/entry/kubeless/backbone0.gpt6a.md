---
type: ArchitectureProposal
title: Kubeless beneath EntryAttempt — useful execution seams and the cost of a second backbone
description: Independent source-grounded comparison of Kubeless adoption levels for the completed tta 0.8 entry architecture, with native ownership, correlation, terminal handoff and fresh-binding constraints.
resource: /design/entry/kubeless/backbone0.gpt6a.md
status: proposed
author: model:openai/gpt-6-astra#high
date: 2026-11-10
sources:
  - resource: opencode://ses_e3ce67451ffeUSPwnGvQP2ckgU
    title: Independent Astra high source inquiry into Kubeless as a backbone for the completed EntryAttempt rewrite
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commission after acceptance of the completed rewrite; independent comparison and adoption decisions remain with the parent and human
  - resource: /doc/entry/README.md
    title: Implemented owned-attempt, fresh-binding and separate catalog contract
  - resource: /design/entry/redesign0.gpt6a.md
    title: Approved destination, read as design history rather than a description of current missing work
  - resource: /design/entry/cutover1.gpt61sol.md
    title: Attributed deletion, runner-free and actual CLI/SSH/consumer acceptance evidence
  - resource: /design/entry/consolidation1.glm53m.md
    title: Completed architecture and documentation consolidation
  - resource: /session/launcher/src/entry/mod.rs
    title: Actual retained owner, protocol ingress and effect accounting
  - resource: /session/launcher/src/entry/protocol.rs
    title: Native request, result, failure and mutation-knowledge boundaries
  - resource: /session/launcher/src/entry/machine.rs
    title: Actual xmachines control adapter and emission tests
  - resource: /session/launcher/src/entry/bind.rs
    title: Fresh domain evidence and execution-shape choice
  - resource: /session/launcher/src/entry/catalog.rs
    title: Mutation-incapable catalog actor and partial-source report
  - resource: /session/runner/src/execute/mod.rs
    title: Thin synchronous effect driver and interpreter contract
  - resource: /session/runner/src/execute/ssh.rs
    title: Frozen SSH observer reuse and process-replacing terminal delivery
  - resource: /session/runner/src/execute/local.rs
    title: Native acknowledged creation, output-forwarding failure and aggregate terminal execution
  - resource: /session/runner/src/observe/mod.rs
    title: Existing shared read driver, native error wrapper and target guard
  - resource: /doc/consumer/term-preserver.md
    title: Maintained receiving obligations and actual retry acceptance
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Actual local and SSH worker call sites using exec and fresh attempts
  - resource: file:///home/rektide/src/kubeless/AGENTS.md
    title: Provider instructions and pre-alpha revision policy
  - resource: file:///home/rektide/src/kubeless/doc/architecture/README.md
    title: Current modular capability and complete-product direction, including connected R3 acceptance
  - resource: file:///home/rektide/src/kubeless/README.md
    title: Capability entrances and dependency boundaries
  - resource: file:///home/rektide/src/kubeless/work/ready/src/lib.rs
    title: Exact admission and terminality laws
  - resource: file:///home/rektide/src/kubeless/work/execute/src/inline.rs
    title: Actual owner-held synchronous admission, invocation and intake
  - resource: file:///home/rektide/src/kubeless/work/execute/README.md
    title: Executor, borrowed Run, pause and cancellation contracts
  - resource: file:///home/rektide/src/kubeless/work/custody/src/model.rs
    title: Native type family and independent owner, target, offer, run and acceptance identities
  - resource: file:///home/rektide/src/kubeless/work/custody/src/owner.rs
    title: Actual retained responsibility owner and native result inspection
  - resource: file:///home/rektide/src/kubeless/work/custody/README.md
    title: Claims, one-shot input, recovery, historical transfer and limits
  - resource: file:///home/rektide/src/kubeless/control/attempt/src/lib.rs
    title: Single resolved invocation, distinct from a whole entry attempt
  - resource: file:///home/rektide/src/kubeless/control/returns/src/lib.rs
    title: Synchronous native-return ownership before advisory reporting
  - resource: file:///home/rektide/src/kubeless/control/block-execution/src/lib.rs
    title: Whole prepared-block handoff, correlation and native output types
  - resource: file:///home/rektide/src/kubeless/control/retained-actions/README.md
    title: Finite preparation and custody composition
  - resource: file:///home/rektide/src/kubeless/control/plan/src/advance.rs
    title: Checked native replacement and saved acceptance receipt, not a transition interpreter
  - resource: file:///home/rektide/src/kubeless/control/retained-actions/src/advance.rs
    title: Native replacement joined to custody accounting through prevalidated turns
  - resource: file:///home/rektide/src/kubeless/control/retained-actions/src/service.rs
    title: Actual maintained Engine, native State and Recipe contracts, attachment and handoff
  - resource: file:///home/rektide/src/kubeless/control/retained-actions/source-cuts.md
    title: Narrow explicit immutable acquisition-sharing contract and its reference-backend limits
  - resource: file:///home/rektide/src/kubeless/catalog/src/lib.rs
    title: Provider capability snapshot, distinct from tta candidate discovery
  - resource: file:///home/rektide/src/kubeless/observation/assessment/README.md
    title: Non-advancing scalar and collection comparison capabilities
  - resource: file:///home/rektide/src/kubeless/observation/assessment/src/lib.rs
    title: Exact scalar projection/baseline distinctions; equality supplies no freshness
  - resource: file:///home/rektide/src/kubeless/work/execute/Cargo.toml
    title: Actual smaller execution dependency boundary
  - resource: file:///home/rektide/src/kubeless/control/retained-actions/Cargo.toml
    title: Finite, host and timed feature-selected dependency boundaries
  - resource: file:///home/rektide/src/kubeless/operation/README.md
    title: Conditional conflict recomputation and native outcome limitations
  - resource: file:///home/rektide/src/kubeless/doc/consumer/diff-dirs-intent.md
    title: Prior step-machine ownership and original-preserving refusal inquiry, qualified by later provider delivery
  - resource: file:///home/rektide/src/kubeless/doc/consumer/walcap3.gpt6a.md
    title: Attributed actual Inline adoption and the separate physical-host, observation and maintenance opportunities
verification:
  method: source-and-contract-review
  scope: Current local source inspection and attributed existing verification; no integration prototype, runtime test, dependency installation or production change in this inquiry
---

# Recommendation

**Keep the completed `EntryAttempt` and xmachines chart as tta's backbone.
Kubeless is a credible optional execution substrate beneath it; the inspected
source does not justify replacing it with the maintained semantic Engine now.**
The best next experiment, if further adoption evidence is wanted, is a small
`ReadyWork`/`poll_inline` runner adapter compared against the current thin driver.
It should have to demonstrate useful composition or deleted coordination, not
merely run the same effects through another vocabulary.

This is the independent Astra-high inquiry for `tta-entry-attempt-kubeless`.
It proposes choices and a possible next commission, not adoption authorization.
No same-wave peer document was read. The earlier redesign is already built:
replay, parallel launch representations and competing resolver loops are not
available deletion gains for this proposal.

## Evidence cut and confidence

| Checkout | Inspected committed baseline | Local state at inspection |
| --- | --- | --- |
| `/home/rektide/src/tta` | `870b2174`, completed consolidation; packages 0.8.0 | Only unrelated `.beads/issues.jsonl` modified before this document |
| `/home/rektide/src/kubeless` | `e422837a`, accepted connected R3; workspace 0.23.0 | Clean; empty working-copy commit `cf37a4cd` |
| `/home/rektide/src/term-preserver` | `1de07f12`, adopted 0.8 wording | Clean; empty working-copy commit `06025688` |

Paths below are navigation to those local checkouts; the commits bind this
account's source claims. Existing test results are attributed to their receipts,
not rerun here. Type mappings are proposed and uncompiled. In particular, an
API's documented native flexibility is not proof of a compiled tta integration.

The [cutover receipt](/design/entry/cutover1.gpt61sol.md) supplies actual
runner-free, terminal and SSH evidence, including three real consumer retries.
The [consolidation](/design/entry/consolidation1.glm53m.md) puts those results on
the implemented destination. Provider guides sometimes retain earlier
“awaiting review” language; the maintained
[architecture home](file:///home/rektide/src/kubeless/doc/architecture/README.md)
records connected R3 accepted at 0.23.0. That acceptance is not tta adoption.

## What the comparison must preserve

1. One chart owns target acquisition, selection, preparation, delivery-time
   binding and effect order. Native handlers cannot choose successor states.
2. `waiting()` is inert. Every accepted reply matches its exact request; wrong
   IDs, target, purpose, paths, slots or result shapes leave progression unchanged.
3. Raw native paths/argv, generator programs and typed errors stay outside serde.
   Supplied-facts progression requires no host I/O or mandatory execution runtime.
4. Binding refreshes the chosen anchor/route/root/destination and inventory;
   it never reranks. An earlier preparation or catalog snapshot is not fresh proof.
5. Independent-view create → enter → keep-last remains one tmux queue. One
   aggregate terminal status is not a series of individual effect receipts.
6. Started, acknowledged, uncertain, cancelled, exported and delegated are
   different facts. A user retry creates a fresh attempt, never replays a spent
   effect or silently selects another backend.
7. `CatalogQuery` remains read-only by its protocol. Partial source discovery
   cannot certify missing work or unqualified project uniqueness.
8. CLI/consumer terminal ownership remains meaningful: successful `exec` replaces
   the worker, while the consumer's parent owns PTYs, transport policy and retry.

These are current properties and obligations, not new benefits attributable to
Kubeless. Their source owners are the
[protocol](/session/launcher/src/entry/protocol.rs),
[actor](/session/launcher/src/entry/mod.rs),
[binding kernel](/session/launcher/src/entry/bind.rs) and
[actual consumer](file:///home/rektide/src/term-preserver/src/launcher.rs).

## 1. What Kubeless actually supplies

The source exposes several different sizes of useful mechanism. They are not
aliases for a workflow interpreter.

| Existing entrance | Verified capability | Consequence for tta |
| --- | --- | --- |
| `control_attempt::attempt(Result<I,R>, factory)` | On `Ok`, calls the factory immediately; polling returns native `Outcome::Completed(O)`. `Rejected(R)` means the factory was not called. | Its “attempt” is one resolved invocation, not `EntryAttempt`'s acquisition/decision/effect protocol. Wrapping entry in it deletes no lifecycle logic. |
| `control_returns::{finish,map}` | Move one native result through synchronous intake before advisory reporting, with no required Clone/serde/runtime. | A small reusable ordering rule. Storage is supplied by the caller; it cannot keep evidence alive across successful process replacement. |
| `work_ready::ReadyWork` and `work_execute::{poll_inline,Inline,Executor,run}` | Exact admitted work/permit, synchronous owner-held turns, keyed future capacity, retained-output callback and disposable borrowed driving. | The strongest smaller fit: host the existing actor without owning its transitions. A blocking native operation still blocks its polling thread. |
| `control_block_execution::{Block,handoff,Handoff}` | Check occurrence/preparation, then invoke a whole native input synchronously or asynchronously; keep actual output and optional correlated hooks. | Preserves a coherent terminal block, but adds preparation/execution coordinates without replacing exact tta request validation or its chart. |
| `work_custody::Owner<S>` | Bound/latest obligations, prepared offers, exact claims, one take, native returns, accounting/recovery and deliberate history transfer. | Could genuinely replace a richer effect ledger. Today's single requested/in-flight slot and receipt vector are much smaller than this product's responsibilities. |
| `control_retained_actions` finite composition | Work/ask/preparation links, checked installation/take, actual-return custody and independent accounting. | A plausible backbone for independently owed work, not a requirement for one entry invocation. Native success and fresh target evidence remain tta code. |
| `control_plan::advance` and `retained_actions::advance::{account,recover}` | Check exact native head and retained candidate; replace native cursor and commit custody settlement together; replay returns the saved acceptance receipt. | Useful when delayed native computations compete to advance a head. Does not determine the next entry state or interpret JSON guards. Receipt replay is not operation replay. |
| `retained_actions::service::Engine` | Continuing review, typed computations, refusal retention, continuation deduplication, custody/execution, selected finish and checked attachment/handoff. | Substantive larger product, but tta would have to supply native Model/State/Recipe/discovery and terminal ownership contracts. It is not a drop-in statechart. |
| `catalog::Catalog` | Pure snapshot resolution of provider capability descriptors and modes. | Not a session/project inventory, root-discovery driver or completion engine. |
| Observation assessment/reflection/publication | Non-advancing comparisons; separately, maintained current pictures and publication products. | Optional observation services if a real continuing reader appears. A cached picture cannot replace binding-time acquisition by itself. |

Source anchors: [ReadyWork](file:///home/rektide/src/kubeless/work/ready/src/lib.rs#L77),
[Inline](file:///home/rektide/src/kubeless/work/execute/src/inline.rs#L29),
[block handoff](file:///home/rektide/src/kubeless/control/block-execution/src/lib.rs#L78),
[custody owner](file:///home/rektide/src/kubeless/work/custody/src/owner.rs#L19),
[native advance](file:///home/rektide/src/kubeless/control/plan/src/advance.rs#L87),
[maintained State/Recipe](file:///home/rektide/src/kubeless/control/retained-actions/src/service.rs#L98).
The line fragments are source-reading coordinates, not claims of generated
Markdown headings.

The provider's complete products matter: this is not a recommendation to recreate
its custody or retained-host algorithms privately. It is a recommendation to
select the product whose recurring coordination tta actually needs. Current
`execute::run` has a small dispatch loop; `observe::drive_reads` already serves
both entry and catalog. Replacing both with a large generic host is not an
automatic simplification.

## 2. Meaningful adoption levels

| Level | Concrete destination | Gain and deletion | Cost / decision |
| --- | --- | --- | --- |
| **0. Current architecture** | Retained xmachines actor, native core, current observers/executors | Already has one progression owner, exact requests and honest outcomes | **Recommended production baseline.** No new dependency; retains today's small host-loop duplication. |
| **1. Shared native execution** | Optional runner adapter implementing `ReadyWork`, driven by `poll_inline`/`Inline`; chart and public protocol remain | Common admission→invoke→intake ordering; composable caller-polled driving; could replace local and SSH dispatch-loop mechanics | Best bounded comparison. Needs a host adapter, stop/registration law and terminal special handling. With one worker and no capacity arbitration, gains may be smaller than added code. |
| **2. Retained execution ownership** | Same chart, actual effects stored in one custody Owner; optional finite retained-action/block composition | Replace authoritative effect slots/results with one-shot inputs, retained originals, owned refusals and explicit handoff/recovery | Real new capability for delayed/manual execution or driver replacement. Significant protocol/result-ownership changes; do not maintain custody plus an independent `EffectState`. |
| **3. Maintained entry service** | Enduring owner with Pursuit/Engine, native entry progression as discovery, each invocation a finite request | Could share intake, continuing observation, multiple concurrent requests, explanation and historical result handling | Requires an actual multi-request service use case and lifetime beyond terminal workers. Larger responsibility and retention policy, new API surfaces, no demonstrated need in present callers. |
| **4. Native-state replacement** | Delete xmachines; native cursor/reducer supplies successors, optionally checked by `control_plan::advance` and hosted by Kubeless | Remove serde control adapter/JSON runtime, use native state directly | A distinct engine-selection rewrite. Kubeless supplies checked replacement, not tta's transition logic or generated graph. Must re-establish the executable structural source; currently weaker payoff than keeping the selected chart. |

Levels 1–3 do not require removing xmachines. Level 4 can be combined with a
small execution layer; it does not logically require the large semantic service.
Conversely, wrapping the existing actor as one opaque Engine operation is legal
but mostly hides its useful stages and leaves all its coordination in place.

### What would make a larger level worth selecting?

- **Level 1:** a real caller needs composable direct driving, native permits or
  independently progressing operations, and the adapter replaces host coordination.
- **Level 2:** completed effects must survive a disposable driver, asynchronous
  acceptance or a manual executor must return original-preserving refusals, or
  one actual operation serves independently accounted commitments.
- **Level 3:** a longer-lived parent intentionally owns many entries, selected
  observation subscriptions or continuing desired work. Its requests must remain
  separate even when they name the same project: two independent terminal entries
  mean two views, not one latest desired value to coalesce away.
- **Level 4:** a measured engine limitation or a human-selected native-state
  direction justifies replacing a now-working executable chart. No such failure
  was established by this inquiry.

These are payoff conditions, not implied new scope.

## 3. Concrete smaller integration: native host, retained chart

### Ownership allocation

```text
EntryAttempt / CatalogQuery              tta core, no I/O
  xmachines transitions + native facts
  exact requests, plan, fresh binding
             │ requested work / typed completion
             ▼
EntryHost implementing ReadyWork         proposed runner adapter
  one borrowed attempt + host resources
  exact admission and in-flight transport bookkeeping
             │
             ▼
poll_inline / Inline                    existing Kubeless mechanism
  reserve → native run → native intake → release → advisory receipt
             │
             ▼
local / frozen SSH interpreters          existing tta host semantics
```

The direct capability entrance is `poll_inline`; the ready-to-use tta convenience
would own its adapter and expose a finite drive over the same turn. Kubeless's
`Inline`/borrowed `run` already provide the continuing convenience. No new
framework-wide “entry registry” is necessary. Start in the runner, not the pure
launcher: `EntryAttempt::{start,acquire,waiting,advance}` remains usable without
selecting this host.

Proposed native types, not compiled APIs:

```rust,ignore
enum HostWork {
    Read(ReadRequest),
    Effect { request: EffectRequest, target: BoundTarget },
}
enum HostReturn {
    Read(ReadCompletion),
    Effect(EffectCompletion),
}
struct EntryHost<'a, H> {
    attempt: &'a mut EntryAttempt,
    host: H, // local resources OR the one frozen SSH observer/connection
    // Only transport/admission state absent from EntryAttempt's public protocol.
    read_in_flight: Option<RequestId>,
    // Any external wake registration is attachment-owned, not a second phase.
}
// ReadyWork::Work = HostWork
// Reserve: FnMut(&HostWork, &Waker) -> Option<NativePermit>
// Run:     FnMut(Pin<&mut EntryHost<...>>, HostWork, &NativePermit) -> HostReturn
// Retain:  FnMut(Pin<&mut EntryHost<...>>, HostReturn, &NativePermit) -> Receipt
```

The concrete existing signature is
`poll_inline(owner, cx, &mut reserve, &mut run, &mut retain)` with those three
callback shapes. `Inline::new` wraps the same operation; it does not demand
`Send`, `Clone`, a new action schema or an allocated owner. Native pinning and
borrow splitting still have to compile in the prototype. An async Executor
cannot simply return a future borrowing its transient mutable producer: it
needs owned captures or separately retained native resources. That is a real
additional design choice, not something solved by making a method `async`.

### Admission and correlation

1. `poll_ready` inspects the actor; it performs no observation or effect.
   At `Planned`, delivery remains a caller decision supplied through the existing
   event. Temporary waiting is `Poll::Pending`, never EOF.
2. Construct a candidate from the current native request. Ask the reservation
   callback once; on rejection leave the request untouched.
3. On accepted **effect** admission, call `EffectStarted(id)` before invoking
   native code, then immediately return that exact work and permit. In a correctly
   exclusively borrowed adapter the observed pending effect cannot change between
   selection and this event; protocol failure is an invariant violation, not a
   reason to take another candidate.
4. Accepted **read** admission needs host dispatch bookkeeping: today's read
   request remains pending until `Observed`. A generic asynchronous producer must
   not offer it twice. `read_in_flight` is transport state, cleared on intake; it
   neither chooses the next read nor becomes a second domain cursor. A synchronous
   effect-only prototype can leave reads with the existing driver instead.
5. Native execution answers the complete request. Intake sends the exact
   `ReadCompletion` or `EffectCompletion` back before advisory reporting and
   before releasing the permit. The chart still decides the next operation.
6. Preserve the full envelope check. A generic execution key/permit does not
   establish the correct endpoint, project slot, source purpose or return shape.

The current `RequestId`/`EffectId` need not become Kubeless `ExecutionId`s at
level 1. `poll_inline` does not require a key. A later keyed Executor must choose
its exclusion key deliberately: attempt identity serializes one invocation,
while target identity could unnecessarily serialize independent terminals and
still provides no cross-process tmux lock. Current IDs also do not all implement
`Hash`; do not silently substitute a formatted name as semantic identity.

### Actual caller and terminal fit

In term-preserver, `embedded_tta` and `embedded_ssh` create an attempt inside a
worker and end with `Handoff::Exec`. The inspected SSH driver even explicitly
selects Exec for execution. A successful exec never returns to Inline intake,
block reporting or custody. None of those mechanisms preserves in-process
history through image replacement.

For today's worker, record dispatch before exec and accept its terminal nature.
Do not require permit cleanup or a future's return after successful exec. A
shared long-lived executor containing unrelated work **must not exec itself**:
it needs an explicit still-undispatched `DelegateTerminal(id)` and a caller-owned
worker, or a separately chosen child-process host. Delegation ends tta's local
ownership; it is not `Completed`. The existing actual consumer uses exec, not
an already-delivered durable terminal-delegation service.

SSH also needs one host object, not independently constructed read and execution
clients. The existing driver borrows its frozen observer once per effect precisely
to preserve setup, cwd, endpoint and connection policy. A proposed native host
can dispatch through that same object sequentially; adopting Kubeless must not
reintroduce a second transport-policy construction.

## 4. If custody really becomes the backbone

This is the strongest meaningful larger design before adopting the entire
semantic service. Use one custody owner **inside the attempt's native owner**;
keep xmachines for domain progression. Make pending/running/returned effect
inspection derive from custody instead of writing an additional lifecycle map.

A concrete proposed `kubeless_work_custody::Types` family:

| Associated type | tta mapping |
| --- | --- |
| `Key` | Per-owner `u64` effect sequence; one owner per attempt. If owners span attempts, introduce a correctly scoped hashable native key. |
| `Target` | Native effect obligation containing original `EffectId`, target binding and required receipt kind. This is an owed obligation, not the host `BoundTarget`. |
| `Body` | Immutable description linking effect identity, binding evidence and execution shape; no duplicate executable expression. |
| `Input` | Owned `EffectRequest` paired with the required frozen target/host binding. |
| `Output` | Actual native completion, preserving `Result<EffectReceipt, EffectFailure>` and its original error. |
| `Unit` | `()` for an opaque terminal block; do not invent substep receipts. |
| `Decision` | Native continuation/stop/delegation evidence and mutation interpretation. |
| `Acceptance` | `Infallible` or an explicitly selected native acceptance result; do not invent an async acceptance operation where none exists. |

Existing operations supply the useful sequence:
`admit_bound` → `install(OfferInput)` → `begin` → `take` → native invocation →
`return_output` → native result interpretation → `account(Batch)`.
The chart authorizes one offer at a time. `admit_latest` is the wrong default:
new demand must not retarget the immutable selected entry or erase an independently
requested view. Whole-block execution can retain one actual result for several
explicit obligations, but equivalence and completion criteria stay native.

**Do not equate `begin`, `take`, and backend mutation.** `begin` attaches the
claim/input; `take` removes the executable input once; `EffectStarted` denotes
dispatch intent; the native receipt may establish more. Before-take delivery
loss can recover the same claim through `claim_for_input`. After take, missing
output is identified uncertainty, not a new copy of executable work.

There are two important implementation costs hidden by a simple type table:

1. **Two authoritative effect ledgers would be wrong.** Replacing `EffectState`
   requires deriving requested/in-flight/finished evidence from offers/runs and
   translating the chart's expected effect boundary to the exact claim. The chart
   still orders effects; custody owns their input/result status. The current
   `Waiting`/snapshot API must deliberately expose projections, not preserve an
   independently updated vector merely to minimize the diff.
2. **The existing ingress consumes the native original.** Custody retains output
   in an `Arc`; current `advance(Event::EffectFinished(completion))` consumes a
   non-Clone failure into `EntryError::Effect`. One cannot both move that failure
   there and retain the same original in custody. A real adoption must introduce
   shared-result ingress/projection (for example an `Arc` to the canonical native
   completion, with source traversal through it), or choose the attempt as sole
   result owner and forgo custody retention. Formatting or cloning a diagnostic
   string is not an acceptable bridge.

Similarly, current protocol rejection preserves **attempt state**, but the
consumed event signature does not return the original rejected completion.
That is adequate for the checked synchronous driver; a retained/manual adapter
needs a preflight-and-accept split or `RejectedEvent { event, reason }` so foreign,
stale or duplicate native originals have an explicit owner. This is a proposed
API improvement, not a claim that present ordinary callers lose valid results.

The larger retained-actions composition adds `Ask`, `WorkSpec`, `WorkBatch`,
`PreparedGroup` and `Preparation::checked` links. Use it when explanation of
several asks and shared work earns those records. A `Block` may directly carry
the complete terminal expression as `PreparedCall::input`, native binding as its
context/binding, and the original effect ID in its description. Its
`handoff(Execution, Block)` checks preparation/occurrence correspondence; tta
still checks effect ID, target and receipt shape. A `RunId` is neither a tmux
session ID nor permission to reuse a former `BoundAction`.

## 5. Retaining or replacing xmachines

The present machine uses `Actor<(), Event>` with small serde discriminants;
`Control::take_directives` decodes the closed directive enum. Native payloads
already avoid JSON. A Kubeless-native data model cannot claim that saving as a
new benefit unless it actually deletes this adapter without adding another one.

### Keep the chart: preferred

- xmachines owns state-entry emission, ordered guards, compound-state common
  stop/cancel and successor selection.
- tta owns exact protocol ingress and pure domain kernels.
- Optional Kubeless execution/custody owns admission/intake or effect storage,
  at the selected level; neither chooses an alternate domain successor.
- Generated entry/catalog graphs continue to describe the executable definitions.

The inspected Kubeless `advance::accept` takes **the caller's proposed whole
replacement cursor** and a read-only native check. Its `Next` value is a
continuation hint, not `sessions_rank → prepare_view`. Its retained-actions
variant composes local native replacement with custody settlement. That mechanism
can support a native state machine, but it does not implement tta's machine.

### Replace the chart: a real alternative, with an honest bill

A native `EntryCursor` could contain typed phase artifacts and a reducer
`transition(cursor, correlated_completion) -> Advance<EntryFamily>` could supply
the next cursor. `Issuer`/`Header`/`Proposal`/`Fields` would add exact-head checks,
original-preserving refusal and saved acceptance receipts; optional custody
settlement would admit the successor effect in the same native turn.

This removes JSON/serde control and may improve phase-local type expression.
It also requires implementing hierarchical stop/cancel, root iteration, guard
order, entry emission and a structural graph source in native code. Keeping
the JSON as a manually synchronized diagram would lose an existing property.
Retaining both a mutable native phase and xmachines while claiming replacement
would instead add a second progression truth.

There is a further fit constraint: the current retained Actor advances in place;
the checked native edit expects a precomputed replacement cursor and an
infallible local commit. Calling arbitrary `EntryAttempt::advance` as a custody
`commit_with` callback is not a demonstrated atomic composition: it can reject
protocol input, pump native directives and allocate. A strong integration needs
prevalidated application or an explicit native reducer, not “call the actor in
the commit closure and unwrap.” No such adapter was compiled here.

## 6. Target, observation catalog and fresh binding

Three similarly named things must remain separate:

- tta's **BoundTarget** freezes host/setup/cwd/tmux endpoint for an invocation.
- tta's **CatalogQuery** collects candidate workspace/project identities and
  per-source failures, without action or mutation authority.
- Kubeless **catalog** describes provider capabilities; custody **Target**
  describes an obligation; neither supplies either tta concept above.

The useful shared observation seam is the existing
[`ReadTask` + `Observer`](/session/runner/src/observe/mod.rs), used by both actors.
It could be hosted with the same read-only ReadyWork adapter. Keep catalog work
typed separately from effects: a generic enum containing mutation variants
should not weaken the catalog's public mutation-incapable contract.

`Catalog::resolve` offers descriptor/version resolution, not raw-name priority,
one-step aliases, shortest Unicode ranking, actual-group collapse or physical
root-route uniqueness. Converting tta candidates into descriptors would retain
all those algorithms and add conversion. Nor does the capability catalog carry
the root-completeness rule needed to certify bare project names. A future backend
capability directory might use it; today's handful of compile-time executors and
explicit `native` capability do not require that directory.

Observation assessment compares projected fingerprints under an equal basis;
known absence belongs inside that native fingerprint (for example `Option`),
distinct from `ProjectionUnavailable` and `BaselineUnknown`. Current
`anchor_check`, fresh name checks and route/root comparisons are
already small domain kernels. Their admission policy still has to distinguish
failed/incomplete reads from absence. A reflector becomes interesting for a
continuing completion UI or shared observer, not as a reason to make each command
maintain a cache.

### Fresh binding is an acquisition boundary, not an ID property

Keep `Deliver` → fresh inventory → chosen route/root/destination witnesses →
checks → lower before effects. For scaffold, current code verifies root identity
before requesting destination lstat; inspecting only the destination is weaker.
The already-selected generator and chosen work remain fixed. `PreparationKind::Fixed`,
`LateConditional`, exact custody tickets or checked semantic heads authenticate
none of these external facts by themselves.

A generic host introduces possible queue delay after binding. It must not claim
a prepared action remains fresh merely because it is unclaimed. The smallest
adapter should avoid inserting a new durable/long waiting queue between binding
and invocation. If a future capacity gate intentionally pauses there, select a
new revalidation protocol or a new attempt before mutation; do not silently rerun
selection inside an executor. Even immediate execution retains the existing
TOCTOU race: observations are not filesystem/tmux locks or server incarnation.

The provider's 0.23 source-cut backend does support one acquisition shared by
explicitly compatible origins. Its native contract requires an **immutable
names-and-bytes cut with source incarnation and coverage meaning**. tta's
selection snapshot and later binding snapshot intentionally have different
temporal meaning. Equal socket/host strings, equal inventory values or adjacent
request sequences do not justify sharing them. A future shared target sampler
would need its own epoch/completeness and freshness contract; this reference
Walk backend is not already a tmux observer.

## 7. Cancellation, partial mutation, delegation and retries

| Boundary | Current tta truth | Required Kubeless interpretation |
| --- | --- | --- |
| Cancel before dispatch | Clear requested work; no new effect starts | Withdraw unclaimed input or end the native pending obligation deliberately; EOF/drop is not the cancellation event. |
| Dispatch intent recorded, no result | `InFlight` and possible mutation; cancel retains that knowledge | Taken input stays spent. Dropping a future/driver proves no physical termination and authorizes no retry. |
| Generator failed | May have left files; no subsequent create | Retain native failure; generic `Completed(Err(...))` means invocation returned, not entry succeeded. |
| Directory check failed after generation | That check did not mutate; earlier generator receipt still matters | Preserve the whole prior effect history, not just the latest effect's `NotStarted`. |
| Native create acknowledged, forwarding/handoff fails | Cause can state `Acknowledged`; earlier creation is not undone | Keep original result evidence. Accounting discharge does not imply cleanup or undo. |
| Terminal block returns failure / SSH loss | Aggregate result, target may have mutated | No fabricated per-command receipts, retry, alternate backend or rollback. |
| Terminal request delegated | Still-undispatched work transfers; caller owns later outcome | Explicit transfer policy, not successful effect return or a generic Engine handoff. |
| Successful exec | No local return; process image replaced | No manufactured completion/custody receipt. Long-lived ownership must reside elsewhere if required. |
| Human retries | New attempt, fresh context/selection/binding; may now choose an existing workspace | New native invocation identity. Never invalidate/rearm a previous effect and run its old bytes. |

Current `Cancel` clears in-flight storage after copying uncertainty into the
terminal outcome. Later completions are rejected; the actor does not currently
offer a cancellation-drain protocol that revises its terminal knowledge.
Custody could retain late originals separately, which is a legitimate level-2
gain. Making those results update a cancelled attempt or restart its chart would
be a new policy requiring explicit selection.

The distinction among **borrowed waiter**, **owning engine**, **physical job** and
**terminal parent** is essential. Kubeless Run drop detaches borrowed driving;
owning Executor/Engine drop can drop its futures; neither guarantees a child or
remote tmux operation stopped. A borrowed Inline attachment leaves its owner
available, but cannot preempt a synchronous native call. Existing term-preserver
worker/process ownership does not disappear when a future adapter is introduced.

`kubeless-operation`'s bounded conflict retry is not suitable default entry retry.
It requires native conflict evidence and fresh read/reprepare on every conflict.
tmux name collision, transport error or unknown native result does not become a
generic conditional-write conflict. A later observation-only investigation of
uncertain creation is plausible, but server-local IDs/name reuse and missing
incarnation proof limit what it can establish. It is a new commission, not
permission to repeat the mutation.

## 8. Larger service alternative and consumer lessons

A serious level-3 design would put the enduring owner in a parent process that
does not exec. Native State would hold the real entry actors/resources alongside
`Pursuit` and `Continuations`. An invocation could be a finite intent/request;
native discovery would expose its next correlated read/effect, retained originals
would feed the actor, and a terminal worker would own the transferred terminal.
Engine's review keys would include exact native progression/basis, not a global
revision that dirties itself on every report. Multiple terminal requests naming
the same workspace would keep independent completion and view-allocation duties.

This is feasible as a design direction over real `service::State`, `Recipe`,
`try_maintain` and optional discovery APIs. Native adapters remain substantial.
The Engine owns continuing review, capacity and retained outputs; tta still owns
entry semantics and the read/effect boundary. A success criterion must come from
actual tta outcome meaning, not an empty obligation set or `Frontier::Exhausted`.
`stop_new`/`try_handoff` support drained whole-Engine replacement; they do not
transfer an arbitrary live terminal operation or stop a physical job.

The most useful possible product outcome would be a multi-request launcher with
readable retained outcomes and shared, qualified observations. Neither current
CLI execution nor the inspected consumer commissions that product. A service
must not start automatically recreating terminated sessions merely because a
purpose remains declared. It also needs deliberate history lifetime: provider
output capacities do not bound all Account/custody history.

Two provider-side consumer accounts inform the judgment:

- [Walcap after Inline](file:///home/rektide/src/kubeless/doc/consumer/walcap3.gpt6a.md)
  reports actual adoption: native admission, capture/install turns, return intake
  and borrowed continuing driving share the same mechanism. Its work survives
  attachment replacement, making the ownership split useful. It explicitly keeps
  worker construction, join, resource tenure and native publication meaning out
  of the generic promise. This supports level 1's feasibility and warns against
  claiming it replaces term-preserver's worker.
- [diff-dirs intent](file:///home/rektide/src/kubeless/doc/consumer/diff-dirs-intent.md)
  distinguishes generic hosting/representation from native frontier transitions,
  and identifies original-preserving acknowledgment refusal. Later connected R3
  supplies a real maintained Walk, so that older account's prospective standing
  cannot be used to claim there is still no complete provider task. However,
  the supplied task's tree/ordered-report semantics are not tta entry semantics;
  its actual shared-native-cut conditions sharpen rather than remove this boundary.

Neither account demonstrates tta migration. Their valuable test is the one
Kubeless's architectural direction asks for: what recurring coordination and
ownership work does the consumer stop implementing?

## 9. Costs, deletion gains and unresolved questions

### Honest deletion budget

At level 1, candidate deletion is the repeated started/perform/finished host
loop in local/general and SSH driving. The shared read loop is already small
and reused. A no-dependency native host helper could also consolidate those
loops; the prototype must compare it, not credit all possible cleanup to
Kubeless. The chart, pure kernels, context policy, probe codecs, lowerers,
observers and physical executors remain necessary.

At level 2, `EffectState` and its lifecycle plumbing can genuinely move into
custody, but not the semantic chain or correlation checks. New machinery includes
targets/offers/claims/tickets, accounting policy, snapshot projections and native
result ownership changes. Generic atomic successor admission pays off when there
are real competing or independently owed commitments; one chart immediately
issuing one successor has less to gain.

At level 4, xmachines and control serialization can be removed only after a native
transition source replaces them. “Delete JSON” is not itself “delete orchestration.”
The old replay machinery is already gone; counting its removed lines would
misrepresent this inquiry's incremental benefit.

### Dependency and maintenance cost

The inspected provider is unpublished and pre-alpha. Any adoption needs a reviewed
revision, consistent package versions and receiving lock; a sibling path alone
does not pin contents. `work-execute` directly pulls ReadyWork, keyed execution,
budget and returns plus futures/pin-project/thiserror; Inline is not a separate
zero-dependency crate. The retained-actions host adds more families. These paths
need no Kubernetes or mandatory Tokio runtime; `timed` explicitly adds timing.
No dependency-tree/build-size/startup benchmark was run here, so those costs are
qualitative, not measured regressions.

### Questions still requiring evidence or owner choice

1. Is there an actual consumer needing driver replacement or native permits for
   entry, rather than a sequential worker which immediately execs?
2. Can a single local/SSH native host adapter compile cleanly without generic
   type noise or duplicated transport ownership, and is its code smaller or
   materially more composable than a direct helper?
3. If retained originals are desired, should the public ingress return rejected
   events and accept shared result handles? That is an API choice, not a trait
   adapter detail.
4. Which process would own evidence after successful terminal exec? Persisting
   or sending it to a parent requires actual transport/retention semantics.
5. Are cancellation-time late results a user need? If so, define whether they
   enrich a stopped record or establish a distinct follow-up observation.
6. Is future shared observation intended to be immutable-cut reuse, a sampled
   current view or a live source? Each needs different freshness/coverage rules.

No source-grounded answer here establishes strong tmux-server incarnation,
invoking-client identity, managed-master recovery, arbitrary remote cancellation,
durable exactly-once mutation or improved launch latency.

## 10. Smallest useful next commission

**Commission an isolated level-1 comparison, only if the human wants further
adoption evidence.** Keep production dependencies unchanged while evaluating:

1. A direct native-host helper over the current protocol.
2. The same host using existing `ReadyWork` and `poll_inline`/`Inline`.

Start after actual `EntryAttempt` binding with an effect-only adapter; leave reads
on the existing `drive_reads` until a concrete benefit justifies expanding it.
Use real public request/completion types and the actual local/SSH interpreter
boundaries, not a standalone counter relabeled “entry.” A disposable scratch
workspace can pin the reviewed provider revision. This inquiry did not create
that workspace or execute a prototype.

The comparison should answer these discriminating cases:

| Encounter | Evidence required |
| --- | --- |
| Reject a permit, inspect repeatedly, then accept | Same pending effect and ID; no native call until accepted; one call and one accepted result. |
| Native create acknowledged, terminal handoff fails | Creation evidence retained; no CLI fallback or duplicate create. |
| Wrong/stale/duplicate completion | No chart movement; any retained original/refusal has an explicit owner. |
| Drop an attachment between effects and attach again | Same attempt survives; completed effect never replays; distinction from dropping an owning physical job remains explicit. |
| Cancel after start without result | Uncertainty retained; adapter does not claim native termination or reissue. |
| Terminal block / successful exec / delegation | One coherent queue; no fabricated substep success; exercise exec in an owned subprocess and distinguish delegation before dispatch. |
| Raw non-UTF-8 path/argv and typed native failure | No serde/string round-trip, preserved error source/downcast behavior. |
| Delayed delivery changes anchor/root/destination | Existing fresh-binding refusal remains; no mutation or reranking. |

For the smallest source/ownership proof, supplied facts plus typed fake effect
returns are sufficient, but they do not establish tmux/SSH parity. Before any
production selection, run the existing actual isolated terminal/SSH and consumer
journeys against the candidate, including fresh user retry. Keep consumer-specific
checks downstream, consistent with Kubeless's architecture instructions.

Report adapter lines and owners added/removed, receiving dependency graph,
compile constraints, semantics and whether a real caller gained composition.
Do not require a microbenchmark to decide a pure ownership question; measure
runtime cost only if a credible performance claim is made. **If the only result
is another wrapper around the same two small loops, stop at level 0.** If shared
admission/intake removes real coordination, select level 1 on that evidence.
Consider custody only when the experiment or a new consumer demonstrates the
retained-result/driver-lifetime need; test its original-result ingress separately.

## Cross-references and verification standing

- [Approved redesign](/design/entry/redesign0.gpt6a.md): architectural precedent
  for one owner and native data; its replay-removal gains are already realized.
- [Maintained contract](/doc/entry/README.md): present acceptance baseline,
  especially fresh binding, independent views and catalog separation.
- [Actual consumer account](/doc/consumer/term-preserver.md): constrains terminal
  parent/worker ownership and makes fresh retry a user outcome, not a scheduler
  policy.
- [Provider architecture](file:///home/rektide/src/kubeless/doc/architecture/README.md):
  supports both direct mechanisms and complete products; useful sharing is judged
  by removed coordination and preserved native meaning.
- [Native advance composition](file:///home/rektide/src/kubeless/control/retained-actions/advance.md):
  a real possible future atomic acceptance seam, with a different state contract
  than today's in-place actor.
- [Source-cut backend](file:///home/rektide/src/kubeless/control/retained-actions/source-cuts.md):
  current sharing precedent with stronger compatibility requirements than equal
  tta target names.

This document is source-grounded design, not executed integration evidence.
The only new verification is the explicit document structure/link/provenance
check. Runtime claims above either follow inspected source or are attributed to
existing implementation receipts; proposed adapters and benefits remain unverified.

Document verification command, run from `/home/rektide/src/tta`:

```sh
cargo run --quiet --manifest-path /home/rektide/src/rekon/tool/rust/Cargo.toml -- check --okf design/entry/kubeless/backbone0.gpt6a.md
```

It passes structure, local destinations, anchors and session provenance. No
runtime tests were added or run for this research-only change. Only this assigned
document is committed; ticket state and shared navigation remain parent-owned.
