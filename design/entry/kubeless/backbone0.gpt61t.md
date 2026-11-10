---
type: ArchitectureAssessment
title: Kubeless and the completed EntryAttempt — backbone choices
description: Source-grounded independent assessment of native execution, custody and observation capabilities against tta's retained 0.8 entry chart and actual consumer boundaries
resource: /design/entry/kubeless/backbone0.gpt61t.md
status: proposed
author: model:openai/gpt-6.1-terra#xhigh
date: 2026-11-10
sources:
  - resource: opencode://ses_e3ce6744effe4joNWIMGf6HADO
    title: Independent Terra inquiry into the completed EntryAttempt and concrete Kubeless integration boundaries
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commission for independent post-rewrite backbone assessments, not adoption authorization
  - resource: /doc/entry/README.md
    title: Maintained implemented 0.8 attempt, binding, effects and separate catalog contract
  - resource: /design/entry/redesign0.gpt6a.md
    title: Approved owned-attempt destination and native-data/runtime boundary
  - resource: /design/entry/cutover1.gpt61sol.md
    title: Actual runtime deletion, real caller cutover and bounded verification receipt
  - resource: /design/entry/consolidation1.glm53m.md
    title: Consolidated architecture, actual APIs and accepted limitations
  - resource: /session/launcher/src/entry/mod.rs
    title: Retained owner, ingress gates, effect lifecycle and native directive pump
  - resource: /session/launcher/src/entry/protocol.rs
    title: Exact requests, native completion, effect knowledge and outcome types
  - resource: /session/launcher/src/entry/machine.rs
    title: Current xmachines actor, JSON-only control events and emitted native directives
  - resource: /session/launcher/src/entry/catalog.rs
    title: Separate mutation-incapable catalog actor and partial-source report
  - resource: /session/launcher/src/entry/bind.rs
    title: Fresh evidence checks and native versus terminal execution choice
  - resource: /session/runner/src/observe/mod.rs
    title: Shared read task, host observer and frozen-target guard
  - resource: /session/runner/src/execute/mod.rs
    title: Current short synchronous effect interpreter and exact start-finish ingress
  - resource: /session/runner/src/execute/local.rs
    title: Real local effect and native partial-mutation classification
  - resource: /session/runner/src/execute/ssh.rs
    title: Frozen SSH interpreter and aggregate terminal effect
  - resource: /doc/consumer/term-preserver.md
    title: Actual local and SSH embedding, completion, retry and independent consumer policy
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Actual embedded attempts, explicit grouping and consumer-owned terminal worker
  - resource: file:///home/rektide/src/kubeless/doc/architecture/README.md
    title: Kubeless direction, implemented product status and independent-use obligation
  - resource: file:///home/rektide/src/kubeless/README.md
    title: Public capability entrances, pre-alpha dependency and in-memory boundaries
  - resource: file:///home/rektide/src/kubeless/control/attempt/src/lib.rs
    title: Exact one-invocation resolution and immediate factory semantics
  - resource: file:///home/rektide/src/kubeless/control/pass/src/lib.rs
    title: Owned context and invocation/completion future with drop semantics
  - resource: file:///home/rektide/src/kubeless/control/block-execution/src/lib.rs
    title: Checked whole-group handoff, native callback and observation boundaries
  - resource: file:///home/rektide/src/kubeless/control/returns/README.md
    title: Ownership-taking return cut and its retention qualifications
  - resource: file:///home/rektide/src/kubeless/work/ready/src/lib.rs
    title: Exact ready-work transfer and pending versus terminal contract
  - resource: file:///home/rektide/src/kubeless/work/execute/src/inline.rs
    title: Owner-held synchronous permit, run and intake ordering
  - resource: file:///home/rektide/src/kubeless/work/execute/README.md
    title: Native modes, executor cancellation and bounded borrowed driving
  - resource: file:///home/rektide/src/kubeless/work/custody/README.md
    title: Bound offers, one-shot claims, retained output and explicit recovery
  - resource: file:///home/rektide/src/kubeless/work/custody/src/owner.rs
    title: Actual native return retention as shared Arc and refusal ownership
  - resource: file:///home/rektide/src/kubeless/control/plan/README.md
    title: Native work correspondence, whole groups and structural versus operational authority
  - resource: file:///home/rektide/src/kubeless/control/retained-actions/finite.md
    title: Implemented finite Walk and exact selected scope, not a generic entry task
  - resource: file:///home/rektide/src/kubeless/observation/model/src/lib.rs
    title: Native scoped facts, explicit missing and unsupported evidence
  - resource: file:///home/rektide/src/kubeless/observation/assessment/README.md
    title: Non-advancing assessment and optional collection coverage
  - resource: file:///home/rektide/src/kubeless/catalog/README.md
    title: Capability-descriptor catalog and its limited snapshot semantics
  - resource: file:///home/rektide/src/kubeless/doc/consumer/diff-dirs-intent.md
    title: Prospective native step-machine integration and ownership limits
  - resource: file:///home/rektide/src/kubeless/doc/consumer/walcap1.gpt6a.md
    title: Owner-held execution and actual worker tenure are distinct adoption questions
verification:
  method: source-and-contract-reading
  scope: Independent analysis only; no dependency integration, Rust experiment, new behavioral test or production change
---

# Recommendation: keep the chart; test a native execution seam before adopting

**Do not replace the completed `EntryAttempt` backbone with Kubeless now.** The
current application already owns an inspectable chart, one correlated outstanding
request, native artifacts and error chains, and the effect lifecycle through
completion or delegation. Kubeless has substantial *complementary* native work
and return-ownership machinery, not a general interpreter for this entry chart.
The best first question is whether its **direct owner-held `ReadyWork`/`Inline`
and return cut** can eliminate useful recurring *host* coordination without
duplicating the attempt's progression authority. The current synchronous host
loop is already short, so even that is an experiment, not a promised saving.

For a genuinely richer backbone, `work-custody` is the serious alternative: it
can retain one-shot prepared input, claims and actual native outputs across
disposable drivers, with explicit accounting and recovery. That is valuable if
tta acquires a real requirement for **an owner surviving driver replacement and
late results**. It is not simply a better implementation of today's single
process-local attempt; its additional identities, output ownership and recovery
policy introduce substantive work. A maintained semantic service or the
reference Walk would be a much larger and presently unjustified product choice.

This document is an independent proposal, not authorization to change tta,
Kubeless, a receiving consumer, or any dependency. No peer same-wave documents
were read. The work item is `tta-entry-attempt-kubeless`.

## Evidence and baseline

I inspected tta's working tree with `@-` at
`870b21745104a249582ce2b58a9c8602a2afc115`; its existing
`.beads/issues.jsonl` working-copy edits were outside this inquiry. Kubeless was
clean at working-copy commit `cf37a4cde764098c40a646a8ed146ffdf3f30443`
(parent `e422837a8743b056fbd999a21b2743e53ae58ef5`), workspace 0.23.0.
term-preserver was clean at working-copy commit
`0602568822fd066255e8557e174dfb9479aff296`. These are local source coordinates,
not a tested dependency pair. They identify what I read, not an immutable remote
release or proof of behavior at another revision.

The [cutover](/design/entry/cutover1.gpt61sol.md) records deletion of the old
`Workflow`, replay, resolver scheduling, and portable/native sidecars, plus
real local, SSH and term-preserver checks. I rely on that receipt rather than
rerunning its suite. [Consolidation](/design/entry/consolidation1.glm53m.md)
reports the accepted 0.8 shape. Reintroducing claims that Kubeless will remove
the old replay loops would count a benefit already realized.

Current control is concrete, not merely an enum in a document:
[`machine.rs`](/session/launcher/src/entry/machine.rs) retains
`Actor<(), Event>`, sends serializable control summaries, and drains one-time
directives from the embedded executable chart; [`EntryAttempt`](/session/launcher/src/entry/mod.rs)
holds the real native data, rejects incorrect completions *before* changing
pending state, and pumps domain functions. [`CatalogQuery`](/session/launcher/src/entry/catalog.rs)
is a separate, read-only actor sharing the binding chart and read protocol.
Neither needs an external scheduler to make a one-shot invocation meaningful.

## What Kubeless actually contributes

| Concrete entrance | Implemented capability | What it does **not** supply for tta |
| --- | --- | --- |
| [`control-attempt::attempt`](file:///home/rektide/src/kubeless/control/attempt/src/lib.rs) | `Result<I, R>` separates `Outcome::Rejected(R)` from `Completed(O)` and calls the native factory once on `Ok`. The factory is called **immediately**, future polling is separate. | Not tta's `EntryAttempt`; it has no chart, pending request validation, staged binding, or one-dispatch authority. A constructed future is not a target dispatch receipt. |
| [`control-pass::from_resolution`](file:///home/rektide/src/kubeless/control/pass/src/lib.rs), [`control-returns::finish`](file:///home/rektide/src/kubeless/control/returns/README.md) | Carries native context/outcome through an invocation and completion; the return leaf moves output to a selected owner before an advisory callback. Native values need not be serde or Clone. | No automatic surviving store, scheduling, retry, durable journal or drop-time completion. `finish` only protects a value if the selected commit actually stores it. |
| [`ReadyWork::poll_ready`](file:///home/rektide/src/kubeless/work/ready/src/lib.rs), [`poll_inline`](file:///home/rektide/src/kubeless/work/execute/src/inline.rs) | Producer retains rejected work; accepted exact work and permit transfer together. Inline synchronously performs reserve → native run → ownership-taking intake on the same pinned owner, then releases permit. | A native adapter must define eligibility, wakeups, tta ingress and effects; it does not interpret JSON statecharts. Blocking work blocks its polling thread. `Pending` is not end and `None` is permanent end. |
| [`work-execute::Executor`](file:///home/rektide/src/kubeless/work/execute/README.md) and borrowed `Run` | Capacity, same-key exclusion, bounded admissions and native completion intake; no spawned tasks or mandatory async runtime. | No target lock, rollback, remote cancellation, physical-job supervision or fairness guarantee. It is useful for multiple independent work items, not automatically for one sequential CLI attempt. |
| [`work-custody::Owner<S>`](file:///home/rektide/src/kubeless/work/custody/README.md) | Bound duties, exact prepared offers, one-shot claim/take, original output/unit retention, independent accounting and explicit recovery/late intake. Native meanings are caller supplied. | Not a statechart, durable log, remote exactly-once protocol or implicit replay permission. Claim is not remote dispatch, and driver loss does not prove native job termination. |
| [`control-block-execution`](file:///home/rektide/src/kubeless/control/block-execution/src/lib.rs) | Checked `Execution`/`PreparedCall` whole-group handoff, native sync/async invocation, context and native output, borrowed progress hooks; `run_sync_returning` has a selected intake. | Its structural check authenticates neither a tta effect ID nor a claim and does not enforce freshness. Executor-entered/first-poll hooks are not backend dispatch. Prepared groups add model and identity machinery. |
| [`observation-model`](file:///home/rektide/src/kubeless/observation/model/src/lib.rs), [assessment](file:///home/rektide/src/kubeless/observation/assessment/README.md) | Typed native subjects, scope, evidence and non-advancing comparisons; optional collection assessments distinguish missing knowledge from known mismatch. | Do not observe tmux, validate tta probes, guarantee freshness or decide that absence authorizes creation. Existing tta domain types already encode those facts. |
| [`kubeless-catalog`](file:///home/rektide/src/kubeless/catalog/README.md) | Pure owned capability-descriptor snapshot and resolution; no I/O. | Not tta's workspace/project inventory or its partial-root and completion policy. Broader catalog work in Kubeless is a proposal, not an existing replacement. |

These leaves are runtime-neutral in the documented senses, but that does not
mean every API is synchronous: futures/streams are explicit and must be polled.
The workspace crates are unpublished, pre-alpha, and may break between reviewed
revisions. A dependency must pin an actual reviewed source revision and meet
tta's runner-free core/feature matrix; version equality alone proves nothing.

## Proposed ownership and concrete mapping

The following is a **proposed adapter**, not an existing API or compiled design.
The chart and attempt remain the sole authorities on course, pending identity,
native data and native terminal outcome. Kubeless enters at a host boundary.

```text
EntryAttempt + xmachines chart (tta, pure, one owner)
  Waiting::Read(ReadRequest) ──► host observation ──► Event::Observed(exact completion)
  Waiting::Planned(EntryPlan) ───► caller chooses Export or Execute
  Waiting::Effect(EffectRequest) ─► optional Kubeless native-work adapter
                                      ├─ reserve/accept exact request
                                      ├─ tta Event::EffectStarted(effect.id)
                                      ├─ native local/SSH interpreter
                                      └─ tta Event::EffectFinished(actual completion)
  Waiting::Finished(outcome) ◄──── tta owns typed outcome and prior knowledge
```

One possible concrete synchronous mapping is a native `AttemptHost` that owns
`EntryAttempt`, observer, executor and the frozen target. It implements
`ReadyWork<Work = HostWork>` where `HostWork` is a **tta-defined** enum holding
an exact cloned `ReadRequest` or `EffectRequest`. `poll_ready` examines the
attempt's current `waiting()` and only offers the current read/effect; it calls
`decide(&work)` first and must leave the attempt unchanged on `None`. On
`Some(permit)`, it immediately returns that exact work and permit, recording
whatever adapter-local reservation is necessary to prevent another offer.
`Waiting::Planned` is a caller decision, **not** producer EOF; `InFlight` and
external waits are pending, also not EOF. Only a finished attempt plus
explicit host policy can return `Ready(None)`. Register wakeups before checking
when another party can supply delivery, cancellation, or capacity; a simple
single synchronous driver may not need a long-lived wake path at all.

`poll_inline` takes `Pin<&mut AttemptHost>`, a `reserve` callback receiving
`(&HostWork, &Waker) -> Option<Permit>`, a `run` callback receiving the same
owner, exact work and borrowed permit, and a `retain` callback consuming the
native output. For a read, `run` calls the current `Observer::observe` with the
exact request and packages a `ReadCompletion`; `retain` feeds it to
`Event::Observed`. For an effect, `run` first advances `EffectStarted(id)` and
**only on success** calls the existing `Executor::perform`; `retain` feeds the
original `EffectCompletion` to the attempt. A protocol refusal or a panic at
either cut must not be silently reported as a successful effect. `retain` must
keep the complete native output/refusal somewhere owned if `advance` cannot
consume it; today's `advance` API consumes an invalid event and returns only a
`ProtocolError`, so that stronger refusal property is **not already provided**.
Current normal drivers avoid this by producing matching completions; a hostile
or late-result adapter needs a deliberately designed ownership-returning seam.

This sketch has an important tradeoff: `Inline` runs a synchronous observer or
executor while holding the native owner borrow. It does not enable an external
caller to cancel the attempt during a blocking native call, nor does it make a
successful `exec` return. If cancellable asynchronous drivers become a real
need, the `Executor`/borrowed-`Run` alternative transfers owned work into a
future and retains intake on its owner, but requires explicit owner/wake and
completion routing. Do not claim a synchronous mock proves that mode.

### The stronger custody experiment

A possible per-attempt schema for `Owner<S: Types>` uses a local, hashable
sequence as `Key` under one owner per tta attempt; `Target` holds the full
`EffectId`, `TargetBindingId` and binding witness, `Body` describes the
native effect/target, `Input` owns the actual `EffectRequest`, and `Output`
is the original `EffectCompletion` or an owned native result. `Unit = ()` and
zero slots are appropriate for an opaque terminal block. Since tta's
`EffectId` is `Eq` but not `Hash`, the mapping must not pretend it directly
meets the custody key bound. The native owner must maintain and validate the
association; a generated Kubeless `RunId` is a **different** process-local
occurrence, not a replacement for the tta ID or a tmux identity.

Using the actual custody entrances, `admit_bound` captures a fixed obligation;
`install(OfferInput { targets, body, input, slots: 0, references: vec![] })`
retains the one-shot payload; `begin(offer)` claims it; `take(&claim)` moves it
once; the host records tta `EffectStarted(id)` before real invocation;
`return_output(claim.run(), output)` retains the original; and a native
`account(Batch)` supplies the actual disposition. `install` and output refusal
return owned inputs. These are **additional proposed steps**, not a present
tta integration. An admitted or claimed item is not proof of a spawn, remote
delivery, successful tmux action or physical exclusion.

There is a nontrivial ownership collision: custody's `return_output` stores the
actual original under an `Arc` and returns a shared receipt, while
`Event::EffectFinished(EffectCompletion)` consumes the completion by value and
its failure contains a non-Clone boxed native error. One cannot simply retain
the same original in custody *and* move it into today's event. A design must
choose one authoritative owner and an explicit interface: e.g. tta retains
the original and custody records a deliberately smaller coordination receipt,
or tta changes ingress/outcome types to share/borrow the original under a
reviewed contract. Do not stringify/clone-away the error or label the small
receipt as the native effect result. Whether such a redesign saves more than it
adds is unknown; a public API change and receiving-consumer migration may be
required. `control-returns::finish` can establish an ordering cut, but does not
itself solve the competing ownership problem.

## Correlation, boundaries and non-negotiable behavior

**Reads.** `ReadCompletion` echoes the entire `ReadRequest` including
attempt/sequence, frozen target, scope, exact path/root slot, endpoint, program
and purpose; the ingress separately checks the observation variant. The host's
[`Guard`](/session/runner/src/observe/mod.rs) also freezes target setup and
later checks the same endpoint/program. A Kubeless work key, ID or native
`SubjectRef` can carry this envelope but cannot replace either validation.
Wrong-shape, stale, duplicate, wrong-target or wrong-purpose completions must
leave tta's pending state and journal unchanged. Partial or failed inventory
must never be converted to an empty successful inventory.

**Fresh binding.** Selection is a decision, not permission to reuse old facts.
After `Delivery`, the chart requests fresh session inventory; existing view
anchors are checked by ID/name/group. For new work it additionally checks the
chosen route, or verifies the creation root **before** another exact destination
lstat. `bind::check` fails closed without reranking; `bind::operation` allocates
the satellite from the fresh whole inventory and chooses execution shape before
mutation. These reads are not generic cache refreshes. Observation assessment
can compare values, but cannot supply the acquisition, request correlation,
ordered evidence or the tta-specific authorization. No current source token
proves a tmux server incarnation, and these checks do not lock tmux or the
filesystem. An exported shell artifact is a frozen snapshot, not an evergreen
authorization.

**No-I/O native core.** Do not place callbacks that execute host I/O inside
the launcher's chart pump, guard registry or Kubeless-backed core. `PathBuf`,
`OsString`, `tta_command::Expression`, tmax invocation, inventory and typed
failure chains remain native, including non-UTF-8 where the selected path
supports it. The host alone performs reads/effects. `control-pass` and
`work-custody` can hold native values; a generic `serde` conversion would
regress the implemented contract. Observe the full source chain, not only a
message. Runner-free and poisoned-environment tests must still work without a
runner, Tokio, implicit environment acquisition or a child process.

**Dispatch and correlation.** `EffectStarted(id)` is tta's explicit
pre-invocation boundary; it prevents repeat dispatch within the active attempt,
but does not prove remote receipt. Correct `EffectFinished` requires the exact
in-flight ID and appropriate receipt kind, checked before consuming state.
Forbid duplicate starts, wrong IDs, wrong receipt types and stale completions.
If a Kubeless claim and tta start both exist, define their ordering and failure
handling; a claim alone cannot become a dispatch and must not permit a second
physical invocation after the tta start. In particular, `attempt` and
`from_resolution` invoke their factory immediately on `Ok`; recording start
*after constructing* that future is too late if the factory has side effects.
`block-execution`'s executor-entry hook also runs before native callback and
may itself prevent invocation; it is weaker than a native dispatched receipt.

**Cancellation, delegation, partial mutation.** Before dispatch, cancel leaves
the effect undispatched. After `EffectStarted`, tta's `Cancelled` snapshot
retains the in-flight ID as unknown; earlier completed receipts remain. Cancel
does not kill a remote process or rollback a generator. A late completion after
terminal cancellation currently cannot advance that finished attempt: a future
retained-host design must explicitly route and preserve its original as late
evidence, without resurrecting the attempt or authorizing new work. On failure,
`EffectFailure` retains `NotStarted`, `MayHaveOccurred`, or `Acknowledged` and
its native source; earlier receipts remain. A native creation can succeed and
then fail while forwarding output. An aggregate SSH/terminal exit status is not
evidence for every substep. Grouped create → attach/switch → keep-last remains
one indivisible tmux queue, not a generic per-command work batch.

`DelegateTerminal(id)` is permitted only for the exact *undispatched terminal*
effect and ends tta's ownership; the receiving caller owns the later outcome.
This differs from successful `exec`, which replaces the process and never
returns to record a completion. Transfer or `Drop` of a Kubeless driver is not
automatically either operation. No automatic retry, rollback, cleanup, changed
backend or alternate selection follows uncertainty. A **user retry creates a
new attempt with fresh observations**; Kubeless's `Keep::Reconsider` or a
recovered available input must not become permission to replay tta mutation.

**Catalog and target.** Keep `CatalogQuery`'s separate `Waiting`/`Event`
surface incapable of delivering plans or effects. It continues after an
individual root read fails, retains per-source failures and qualified routes,
and offers a bare project name only with complete required roots. It shares
target binding and read interpretation with entry without selecting an entry
winner. `kubeless-catalog` describes advertised capabilities, not these
workspace views and project routes; a future retained shared observation view
would need explicit source completeness, observation lineage and new freshness
semantics before replacing anything here.

## Adoption levels and what could actually disappear

| Choice | Possible benefit / removals | Cost, loss or decision gate |
| --- | --- | --- |
| **0. Keep 0.8** | No dependency or behavioral churn; current ~small shared read and effect loops already implement the needed host behavior. | Does not gain shared multi-owner admission or retained returns across disposable drivers. |
| **1. Leaf host seam, retain xmachines** | `ReadyWork`/`Inline` or a return cut could replace some repeated future host-specific reserve/run/intake choreography if a real additional host needs it; read-only and effectful producers can share carefully selected mechanics. Existing observer and executor remain native. | In today's single synchronous worker it may add more adapter, pinning, wake and dependency code than it removes. `execute::run`, SSH's borrowing-specific loop, and `drive_reads` are deletion *candidates only after* a proven equivalent host; the chart pump, ID gates, native decisions and domain tests remain. |
| **2. Custody-backed owner + chart** | Real retained one-shot offers, original results and explicit driver replacement/late-evidence inventory could replace portions of `EffectState` and host bookkeeping, **if** ownership, cancellation and native output integration are redesigned coherently. | Extra duty/offer/run/ticket/tta ID correspondence, accounting, history and delivery machinery; Arc-versus-consumed completion issue; explicit transfer/retention policy and no durability. A parallel ledger without deletion would be worse. Requires evidence of a long-lived owner and consumer benefit. |
| **3. Kubeless semantic/retained product as organizing backbone** | Could become useful for an eventual multi-attempt service with continuing purposes, several independently owed consumers, explanation and lifecycle policies; its `Account`, custody, native execution and selected maintained products are substantial. | Current normalized [finite Walk](file:///home/rektide/src/kubeless/control/retained-actions/finite.md) implements a particular ordered tree/report task, not entry's hierarchy. Adapting entry would require a new native task/product or substantial tta machinery; retained history and semantic models have cost and no automatic bounded residence. No current consumer asks for this. |
| **4. Replace xmachines with native Kubeless-driven progression** | One might remove serde control summaries, chart adapter and xmachines dependency; native types could flow directly through a newly designed Rust progression model. | Kubeless `ReadyWork`, custody and semantic frontier do **not** provide an equivalent general hierarchical statechart or executable guarded graph. Tta would have to author and maintain its own transition authority, state-entry/reentry and chart/diagram proof, or separately choose another real state-machine engine. A host adapter plus a parallel Rust transition table is not replacement. No demonstrated reason to lose the selected chart. |

Choice 1 is my recommended **investigation**, not an adoption decision. Choice 2
is the strongest contingent alternative, not dismissed as merely a wrapper: it
addresses materially different owner/driver lifetimes. Choice 3 becomes
reasonable only with an actual maintained-service product and selected native
semantics, not because Kubeless contains a comprehensive model. Choice 4 is a
separate engine-selection decision, not implied by adopting Kubeless.

No option should claim it removes the selection, preparation, binding,
lowering, actual host adapters or tta's semantic fixtures: these embody tta's
user-visible behavior, not incidental generic orchestration. No line-count or
performance gain has been measured. The cost includes one more pre-alpha
source pin, MSRV/dependency/feature effects, existing exact internal tta pins,
and the burden of coordinating a real receiving lock when public interfaces
change. Conversely, successful reuse could improve shared native intake and
driver replacement across projects; that potential deserves evidence rather
than being ruled out by the small size of today's loop.

## Consumers and next useful evidence

The actual [term-preserver account](/doc/consumer/term-preserver.md) and
[call sites](file:///home/rektide/src/term-preserver/src/launcher.rs) own the
proxy, PTY, worker, SSH options/ControlMaster policy and retry prompt. Explicit
grouping maps to `Independent` or `Shared`, never an inferred `Auto`; remote
setup and relative target paths stay target-local. The embedded SSH route
currently calls `execute::ssh::drive(..., Handoff::Exec)` using the observer
that performed reads; it does **not** demonstrate production use of
`DelegateTerminal`, which is a supported protocol route tested separately.
Successful exec replaces the worker. Completion in `tta-lib` uses the separate
catalog; the non-library build keeps its own completion and roots policy.
Replacing the consumer worker or forcing the library into that build would be a
new product choice, not a prerequisite for experimentation. Mosh's distinct
observation-channel choice and inside-client origin remain open separately.

The relevant Kubeless [diff-dirs account](file:///home/rektide/src/kubeless/doc/consumer/diff-dirs-intent.md)
similarly distinguishes hosting a native step machine from implementing its
state machine or choosing a maintained model. The [Walcap account](file:///home/rektide/src/kubeless/doc/consumer/walcap1.gpt6a.md)
distinguishes owner-held turns from real worker tenure and joined physical jobs.
Both are useful analogies, not proofs that either integration exists for tta.

If the human wants a next commission, make it a **disposable, runner-free,
public-API encounter** on reviewed tta/Kubeless revisions, with no production
dependency first. Compare a minimal `ReadyWork`/`Inline` host around the real
0.8 `EntryAttempt` against the existing supplied-facts driver. Use real native
request/completion types and injectable fake observers/effects, not a second
toy state machine. Include one existing-view path with a changed anchor; one
multi-root partial catalog; and a native scaffold ladder with a success followed
by a typed failure. Prove exact request/target/shape rejection leaves state
unchanged, start-before-invoke, duplicate-start rejection, no effect after
cancel-before-start, cancellation after start with prior receipts and uncertain
in-flight ID, and a delegated undispatched terminal. Retain non-UTF-8 native
payloads and downcastable source errors. Verify the coherent terminal queue is
one work item and reject reuse after an uncertain mutation. Explicitly test a
planned pause as Pending, not EOF, and count what code actually disappears.
Exercise post-start cancellation by explicitly pausing a fake in-flight effect
at the protocol boundary; do not claim that synchronous `Inline` can interrupt
its own blocking `run` callback. An asynchronous host would need its own proof.

Separately spike the custody ownership clash only if a surviving host is an
actual target: return a real non-Clone effect failure into custody, attempt the
tta ingress design, replace a driver, and route a late original after cancel.
Show the exact owner of each original and every refusal, with no replay or
invented receipt; compare against simply retaining it in a tta-owned host.
This may reveal that a native ownership-returning ingress or shared error form
is needed. Test it rather than assuming that `Arc` extraction or identity
mapping works.

Report dependency tree, feature/MSRV impact, native code and state removed
versus added, runnable tests and known untested physical-job/SSH boundaries.
If the adapter cannot preserve one progression truth or cannot show a concrete
host benefit, retain 0.8. A favorable mock is still not authorization for
production adoption: the real-terminal, authenticated SSH, runner-free and
term-preserver retry matrices in the cutover remain acceptance obligations for
any later commissioned change.
