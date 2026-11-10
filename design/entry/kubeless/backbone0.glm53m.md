---
type: ArchitectureAssessment
title: Kubeless as the entry-attempt backbone — independent G assessment
description: Source-grounded assessment of ~/src/kubeless as a possible backbone for the completed 0.8.0 EntryAttempt, mapping verified kubeless capabilities to every protocol seam, comparing four adoption levels with costs and deletion gains, and recommending retention of the delivered architecture
resource: /design/entry/kubeless/backbone0.glm53m.md
status: draft
author: model:zai-coding-plan/glm-5.3#max
date: 2026-11-10
sources:
  - resource: opencode://ses_e3ce67445ffexbMCISOQHM4LT9
    title: Independent G inquiry - kubeless source/guide/consumer inspection, tta entry code review, leaf-crate path-dependency spike and this assessment
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commission defining shared evidence, assigned output, independence constraints and the no-adoption boundary
  - resource: /doc/entry/README.md
    title: The completed 0.8.0 entry architecture this assessment must treat as the incumbent backbone
  - resource: /design/entry/consolidation1.glm53m.md
    title: Consolidation receipt - accepted-architecture table, deletion review and exact usable 0.8 APIs
  - resource: /design/entry/redesign0.gpt6a.md
    title: Human-approved destination including the explicit keep-xmachines engine-fit ruling
  - resource: /design/entry/xmachines-port0.gpt61sol.md
    title: Human selection of xmachines-runtime 1.1.0 with Statig as the named fallback, not kubeless
  - resource: /doc/consumer/term-preserver.md
    title: Actual consumer outcomes and constraints this assessment must preserve
  - resource: /session/launcher/src/entry/protocol.rs
    title: Caller-surface protocol - correlation, observation/effect vocabulary, outcome and error types
  - resource: /session/launcher/src/entry/mod.rs
    title: EntryAttempt retained actor - data slots, effect state, advance ingress
  - resource: /session/launcher/src/entry/machine.rs
    title: Chart adapter - serde-discriminant boundary and closed Directive enum
  - resource: file:///home/rektide/src/kubeless/README.md
    title: Kubeless entrance catalog and composition/boundary claims at revision cf37a4cde764
  - resource: file:///home/rektide/src/kubeless/kubeless.md
    title: Per-crate substrate matrix, control-family boundaries, semantics-that-must-not-collapse and current limitations
  - resource: file:///home/rektide/src/kubeless/AGENTS.md
    title: Pre-alpha status, lockstep versioning and minor-bump-per-API-change policy
  - resource: file:///home/rektide/src/kubeless/control/retained-actions/README.md
    title: The largest control composition - custody-based retained actions, transfer laws and limits
  - resource: file:///home/rektide/src/kubeless/work/custody/README.md
    title: Duty/offer/run ownership and exact historical transfer laws
  - resource: file:///home/rektide/src/kubeless/doc/consumer/index.md
    title: Consumer adoption patterns - all reconciler/watcher/capture-shaped, none bounded one-shot attempts
  - resource: file:///home/rektide/src/kubeless/doc/consumer/diff-dirs.md
    title: Closest exploratory consumer account - coordinated tree work and native control, still maintenance-shaped
---

# Kubeless as the entry-attempt backbone — independent G assessment

Commissioned by the parent
(`opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj`, ticket `tta-entry-attempt-kubeless`):
after the approved rewrite completed and consolidated at 0.8.0, assess
`~/src/kubeless` as a possible **new backbone for the completed EntryAttempt**.
This is research only — no adoption, no implementation authorization, no
runtime change. Same-wave peer documents
(`backbone0.gpt6a.md`, `backbone0.gpt61t.md`) were **not** read; the parent
compares the three returns.

## Method and inspected evidence

Read in full or in decisive part, at these revisions:

- **tta** `32710f0e6cd5` (parent `557da330`): [`/doc/entry/README.md`](/doc/entry/README.md)
  (the 0.8.0 contract), [`/design/entry/consolidation1.glm53m.md`](/design/entry/consolidation1.glm53m.md),
  [`/design/entry/redesign0.gpt6a.md`](/design/entry/redesign0.gpt6a.md)
  (engine-fit ruling, lines 449–520), [`/design/entry/xmachines-port0.gpt61sol.md`](/design/entry/xmachines-port0.gpt61sol.md)
  (opening and sources), [`/doc/consumer/term-preserver.md`](/doc/consumer/term-preserver.md),
  [`protocol.rs`](/session/launcher/src/entry/protocol.rs) (646 lines, entire),
  [`mod.rs`](/session/launcher/src/entry/mod.rs) (structure and ingress),
  [`machine.rs`](/session/launcher/src/entry/machine.rs) (boundary and Directive enum).
- **kubeless** `cf37a4cde764`, workspace `0.23.0` (pre-alpha, unpublished):
  [`README.md`](file:///home/rektide/src/kubeless/README.md),
  [`AGENTS.md`](file:///home/rektide/src/kubeless/AGENTS.md),
  [`kubeless.md`](file:///home/rektide/src/kubeless/kubeless.md) (families,
  substrate matrix, "semantics that must not collapse", current limitations,
  stopping points 5/5b), control crates `attempt`/`pass`/`returns`/`plan`
  READMEs and key `lib.rs` signatures, [`retained-actions`](file:///home/rektide/src/kubeless/control/retained-actions/README.md)
  (entire), [`custody`](file:///home/rektide/src/kubeless/work/custody/README.md)
  (laws section), `work/ready`, `work/keyed`, `catalog` READMEs, the
  [consumer index](file:///home/rektide/src/kubeless/doc/consumer/index.md)
  and the [diff-dirs account](file:///home/rektide/src/kubeless/doc/consumer/diff-dirs.md)
  (opening and verification block).
- **Spike**: a foreign consumer crate under the project's gitignored
  `.test-agent/kubeless-g/` (its `README.md` records the setup and output)
  depending on four kubeless leaves by path, exercising their advertised
  contracts with tta-flavored native types. Results summarized below.

Claims below distinguish **verified source capability** (path-cited) from
**wish** (what adoption would still have to build).

## What the completed EntryAttempt demands of a backbone

The 0.8.0 architecture is not a replay planner; it is a retained actor whose
backbone duties, each with its current owner, are:

| Backbone duty | Current owner (verified) |
| --- | --- |
| Orchestration: one persistent chart, compound states, ordered guards, state-entry directives, finals | [`entry.machine.json`](/session/launcher/src/entry/entry.machine.json) executed by xmachines-runtime 1.1.0 (crates.io); 28-directive closed enum in [`machine.rs`](/session/launcher/src/entry/machine.rs) |
| Correlation: exact-echo reads (ID, target, purpose, endpoint, program, path, slot), one pending read, stale/duplicate/wrong-shape rejection that leaves waiting intact | `RequestId`/`check_completion` in [`protocol.rs`](/session/launcher/src/entry/protocol.rs) and [`mod.rs`](/session/launcher/src/entry/mod.rs) |
| Effect lifecycle: `EffectStarted` before dispatch, no reissue/redispatch, dispatch-order receipts, `MutationKnowledge`, `DelegateTerminal`, `Cancel` | `EffectState`/`EffectKnowledge` and `AttemptOutcome` in [`protocol.rs`](/session/launcher/src/entry/protocol.rs) |
| Native payload/error boundary: paths, argv, inventories, typed `EntryError` chains never cross serde; only discriminants/counters/summaries cross the chart | `AttemptData` slots in [`mod.rs`](/session/launcher/src/entry/mod.rs); machine.rs doc header: "Only control discriminants cross serde" |
| Guarded selection (ambiguity before unique/empty), project ranking, creation ladder | [`select.rs`](/session/launcher/src/entry/select.rs), `discover.rs`, `prepare.rs` as chart directives |
| Fresh binding: fresh evidence for the chosen decision, fail-closed `AnchorChanged`/`RouteChanged`/`DestinationChanged`/`CreationRootChanged`/`NameCollision`, never rerank | [`bind.rs`](/session/launcher/src/entry/bind.rs) |
| Read-only catalog: second retained actor sharing the `bind_target` subchart, mutation-incapable | [`catalog.rs`](/session/launcher/src/entry/catalog.rs) |
| Replaceable interpreters: observers answer reads; executors perform exactly the dispatched effect; SSH swaps interpreters, not policy | `session/runner/src/observe/*`, `execute/*` |

Any "backbone" proposal must either supply these duties or host them without
becoming a second progression truth — the consolidation explicitly verified
"no second scheduling representation"
([consolidation1](/design/entry/consolidation1.glm53m.md)).

## What kubeless actually supplies, seam by seam

**Invocation distinction.** `kubeless-control-attempt` keeps resolution
`Rejected(R)` distinct from native `Completed(O)` where the invocation factory
was never called ([attempt README](file:///home/rektide/src/kubeless/control/attempt/README.md)).
Verified in the spike: a rejected resolution never runs the factory; a native
`Err` stays `Completed`. tta already realizes this law natively and more
specifically: observer/guard failures end in `Stopped(EntryError)` at defined
chart boundaries, while effect failures end in `Stopped(EntryError::Effect)` —
purpose-typed, not generic. Nothing is gained.

**No-I/O native payload/error boundary.** This is kubeless's strongest
philosophical match. `control-pass` carries an owned native context through
resolution/invocation/completion with **no Clone/Send/Sync/'static bounds and
no boxing** ([pass README](file:///home/rektide/src/kubeless/control/pass/README.md));
`control-returns` is std-only with zero normal dependencies (verified by
`cargo tree`). Spike-verified: a `!Clone` `OsString`-argv context survives to
the finish closure beside a native `Err` outcome. tta's equivalent law is the
serde-discriminant wall at the chart; the two mechanisms answer different
problems — pass guards *one future's* ownership transfer, tta guards *a
multi-state actor's* event vocabulary. pass does not host a chart.

**Effect/request correlation.** kubeless nearest supplies:
[`work-ready`](file:///home/rektide/src/kubeless/work/ready/README.md)
(producer owns work until a consumer *accepts*; permits as reservations;
immediate-return law), [`work-keyed`](file:///home/rektide/src/kubeless/work/keyed/README.md)
(at most one active future per key — spike-verified occupancy), and custody's
exact tickets/acceptance IDs. **None of them implement tta's exact-echo reply
matching**: verifying a reply answers *the exact pending request* — same ID,
target, purpose (`SessionReadPurpose::{Catalog,Selection,BindAction}`,
`DestinationPurpose::{Preparation,Binding}`), endpoint, program, path, root
slot — while rejecting stale, duplicate and wrong-shape replies as
`ProtocolError`s that leave the waiting state intact
([protocol.rs](/session/launcher/src/entry/protocol.rs)). That envelope
discipline is bespoke and load-bearing; kubeless would not supply it.

**Cancellation, delegation, partial mutation.** The kubeless laws genuinely
rhyme with tta's honesty posture, verified in source: custody runs own "sole
input until take, partial native returns, actual O, independent binding
settlements" with exact historical ownership transfer
(`TransferredHistory`/`SettledHistory`; "transfer never implies that a native
result did not exist" — [custody README](file:///home/rektide/src/kubeless/work/custody/README.md),
[retained-actions README](file:///home/rektide/src/kubeless/control/retained-actions/README.md));
`control-returns`: "dropping pending native work invokes neither continuation
and fabricates no result"; `control-settlement` preserves an acknowledgment
through explicit target observation; `control-drain` two-stage shutdown. The
mapping is real but the granularity is inverted: custody is a retained
**multi-run ledger** with admission laws (duties, offers, cohorts, HistoryIds,
latest-capability invalidation) built for reconciliation workloads; tta's
`EffectState` is three fields (requested / in-flight / finished) inside one
bounded attempt, with `DelegateTerminal` transferring exactly one outstanding
terminal request. Hosting tta's semantics on custody means adopting the
ledger to model a lane.

**Retry semantics.** kubeless explicitly refuses what tta also refuses:
`operation` retries **only on native conflict evidence** with bounded
recomputation; `requeue` "retries nothing" — it decides when a completed key
may return; retained-actions' discovery intake is acceptance-only retry; the
guide's "semantics that must not collapse" section states admission is not
delivery and local generation is not source revision
([kubeless.md](file:///home/rektide/src/kubeless/kubeless.md)). tta's law —
no automatic retry, no rollback, no cleanup, user retry builds a **new**
attempt — is aligned in spirit but stricter in posture: tta's fresh binding
**fails closed** where kubeless `operation` **recomputes** on conflict. That
difference is deliberate tta policy (stop, never substitute), not a gap.

**Fresh binding.** No kubeless crate supplies tta's "fresh evidence for the
*already-chosen* decision, fail-closed on change, never rerank" as a reusable
capability. The nearest verified analogues carry different postures: custody
`Advance` admission "checks the exact expected required level as well as
owner/duty birth" (expected-level checking ≈ anchor fingerprints, but for
admitting *new* work, not revalidating a chosen plan); retained-actions
`LateConditional` preparations explicitly "do not authenticate freshness";
`operation`'s fresh-read preparation recompute-on-conflict contradicts tta's
fail-closed stop. tta's binding (`bind.rs`, 198 lines, pure) stays tta's own
under any adoption level.

**Target/observation catalog.** Name collision, not overlap:
[`kubeless-catalog`](file:///home/rektide/src/kubeless/catalog/README.md) is
capability descriptors and declared-vs-discovered reconciliation — the
provider-neutral half of a descriptor-to-operation bridge for Kubernetes-shaped
capability spaces. tta's [`CatalogQuery`](/session/launcher/src/entry/catalog.rs)
is a read-only session/project inventory collector sharing the entry
`bind_target` subchart, structurally unable to emit a mutation request. The
observation family (`observation-session`, `assessment`, `reflection`,
`reflector`) is continuous-source machinery — snapshot acquisition and
*resumable* changes — while tta reads are bounded, purpose-tagged and
one-at-a-time by design. Only a hypothetical future continuous feature
(watching session inventories, drift) would want them.

**Orchestration itself — the decisive finding.** **kubeless contains no
statechart interpreter.** Verified: zero source hits for a
statechart/state-machine engine outside design history; the control family's
orchestration styles are `control-driver` (concrete FIFO/custom-producer
executor), `control-controller` (post-observation control loops) and the
retained-actions host (work/decision/acceptance driving over a custody owner)
— all reconciliation-loop shaped. "Kubeless as backbone" therefore cannot be
an engine swap for xmachines; it is a rewrite of tta's orchestration *style*
from a declarative chart (executable JSON, generated graphs, directive and
guard-order conformance tests) into imperative composition over custody and
drivers.

## Concrete API/type integration sketch (for the record, not a proposal)

If any level beyond retention were ever chosen, the only technically coherent
integration is at the **effect lane**, because that is where kubeless's
ownership laws live:

```rust,ignore
// tta-native types stay tta's own; kubeless would type the lane, not the actor.
// EffectRequest/EffectKind/EffectReceipt/MutationKnowledge remain in protocol.rs.
// Candidate seam (L2): custody Owner<S> with S::Key = TargetBindingId-slot,
// S::Input = EffectRequest, S::Output = Result<EffectReceipt, EffectFailure>;
// DelegateTerminal maps to custody's exact history transfer.
```

What does **not** integrate: the 28 directives, guard order, the `select`
compound's per-root reentry, the creation ladder and the fresh-binding
subchart have no kubeless types to receive them; they would remain native
functions called by whatever orchestrates — exactly their status today under
xmachines, minus the chart.

## Retained xmachines relationship vs replacement alternatives

xmachines-runtime 1.1.0 was **human-selected**
([xmachines-port0](/design/entry/xmachines-port0.gpt61sol.md): "Human selects
xmachines-runtime, retains Statig as a possible fallback"), and the approved
redesign ruled "Keep the human-selected `xmachines-runtime =1.1.0`"
([redesign0](/design/entry/redesign0.gpt6a.md), engine-fit section). The
serde cost is bounded and visible: only control discriminants cross
(`machine.rs`); the adapter is a closed enum plus an ingress function. The
named fallback is Statig (Rust-first, no serde), not kubeless. kubeless is
not an alternative *engine* at all — it is an alternative *architecture*
(imperative reconciliation composition). Any move from xmachines to kubeless
abandons: the executable JSON structural truth, generated
`attempt-graph`/`catalog-graph` with `--check` freshness, directive-exhaustion
and guard-order conformance tests, and a published crates.io dependency
(`=1.1.0`) in exchange for an unpublished pre-alpha path dependency.

## Adoption levels compared

| Level | Content | What tta deletes | What tta adds | Net for user outcomes |
| --- | --- | --- | --- | --- |
| **L0 — retain 0.8.0** (recommended) | Nothing changes; kubeless vocabulary stays a reference oracle | — | — | None needed; consumer needs all met |
| **L1 — leaf seams** | Path-dep on std-light leaves (`control-returns`, `control-attempt`/`pass`, `work-keyed`) at runner/driver seams only | A few lines of hand-rolled ownership code already tested | Unpublished pre-alpha path dep; revision-pin upkeep (kubeless bumps minor on every API change, lockstep, per its AGENTS.md); foreign abstractions at seams tta owns | Zero user-visible; marginal code-law uniformity |
| **L2 — effect lane on custody** | `EffectState`/`EffectKnowledge` become a custody `Owner` | ~30 lines of lane state | The custody model (duties/offers/admissions/HistoryIds), a mapping layer, a second authority for effect progression beside the chart (the consolidation explicitly verified "no second scheduling representation"), re-verification of the effect suite and consumer SSH evidence | None; risk of semantic drift around delegation and in-flight uncertainty |
| **L3 — attempt as retained-actions/maintenance** | EntryAttempt becomes asks/work/preparations under the retained-actions/custody engine | The chart, protocol core, generated graphs, verification suites (~7.3k lines in `entry/` incl. tests, most rewritten not deleted) | The whole reconciliation model whose semantics — continuing intent, reinterpretation of settled originals, successor work, disposable driving — are the ones tta's accepted contract explicitly rejects: "no production replay, resume token or second progression truth exists", "a user retry builds a **new** attempt" | Negative: changes the product's meaning; every verified outcome witness re-authored |

**Costs common to L1–L3:** dependency on an experimental soft fork that is
"pre-alpha … workspace-internal and not published" pinning reviewed revisions
(kubeless AGENTS.md); loss of the crates.io-only published-dependency property
tta holds today (xmachines 1.1.0 vs path dep); re-verification expense against
a body of evidence the cutover just banked (19 checks, 13 terminal runs, 16
creations, three real consumer cases, 245/220/241 consumer suites —
[term-preserver account](/doc/consumer/term-preserver.md)).

**Deletion gains:** none at any level. The real deletion (0.7 replay
orchestration) was already collected by the cutover; consolidation re-verified
the scans. L2/L3 trade small owned state for larger imported models.

## Recommendation

**Retain the delivered 0.8.0 architecture (L0). Do not adopt kubeless as the
entry backbone.** Reasons, in weight order:

1. **No supplied backbone.** kubeless has no chart engine; its orchestration
   compositions are reconciliation-shaped, while tta entry is a bounded
   one-shot attempt. Every kubeless consumer studied
   ([index](file:///home/rektide/src/kubeless/doc/consumer/index.md): walcap,
   cyclrs, kopacetic/Akri, twigd, kinefold; diff-dirs exploratory) is
   watcher/capture/maintenance-shaped — none is a bounded closed-stdin CLI
   attempt. The closest philosophical match (honest settlement, ownership
   transfer, admission≠delivery) is law *vocabulary* tta already implements
   natively with stricter, purpose-typed forms.
2. **Semantic mismatch at the model level.** tta's accepted contract rejects
   retained/maintained/reinterpretable work; kubeless's control core exists
   to serve it. Adopting the model to obtain its leaves inverts the value.
3. **Cost without user outcome.** No consumer need — term-preserver's bounded
   acquisition, consumer-owned transport/master/retry, honest `Stopped`/
   `Delegated`, embedded completion, retained non-library build — is better
   served by any level. The verification investment stays banked.

**Keep as standing reference:** kubeless's verbalized laws (custody transfer,
settlement, ownership-taking returns, admission-is-not-delivery) are an
excellent oracle when tta's protocol evolves; cite them without depending on
them. **Revisit triggers:** (a) tta gains a continuous-observation feature
(session watching/drift) → assess `observation-session`/`reflector`; (b)
concurrent per-target effect lanes → assess `work-keyed`/`control-driver`;
(c) kubeless publishes and stabilizes → re-cost L1.

## Unknowns (honest)

- Whether tta remaining publishable with crates.io-only deps is an operator
  goal; if it is, any kubeless adoption (unpublished) blocks it.
- kubeless velocity: 0.23.0 lockstep with minor-bump-per-API-change means a
  tta pin would move on every upstream API revision; maintenance coupling is
  unbudgeted.
- Whether a future consumer will request continuous maintenance features —
  the only realistic pull toward the observation/reflection family.
- I did not run kubeless's own test suites or read its full retained-actions/
  custody sources line-by-line; crate-level claims rest on READMEs, guide,
  signatures and the spike. The L2/L3 integration sketches are untested by
  construction (no adoption authorized).

## Smallest useful next commission

Not a port prototype. If the parent wants one more evidentiary step, the
smallest useful commission is a **law-mapping review of tta's protocol against
kubeless's custody/settlement/returns laws** (documentation-level, zero
dependency): for each tta seam (exact-echo ingress, EffectStarted ordering,
delegation transfer, mutation knowledge), check whether kubeless has already
verbalized a law tta's docs state weakly, and adopt the *wording* where so.
The leaf spike in `.test-agent/kubeless-g/` already supplies the only code
evidence L1 would need (contracts hold from a foreign consumer; dependency
pull measured: `returns` std-only, `attempt`/`pass` pull `pin-project`,
`keyed` pulls `hashbrown`). Anything larger should wait for a revisit trigger.

## Spike record

`.test-agent/kubeless-g/spike/` (gitignored scratch; its `README.md` records
setup and measurements): foreign consumer crate, path deps into
`~/src/kubeless` at `cf37a4cde764`, edition 2021, `cargo run` output 2026-11-10:

```text
attempt: rejected=Rejected("ambiguous selector") completed_is_native_err=true
pass: context argv len = 2, native outcome was err = true
returns: committed receipt = 2
keyed: drained outputs = [70, 90]
```

All four advertised leaf contracts held as documented; `cargo tree -e normal`
matched the substrate matrix's dependency claims. Honest caveats: this
verifies leaves only — it says nothing about custody/retained-actions
ergonomics at L2/L3, which remain unprototyped by design.

## Standing

This document is one of three independent same-wave assessments; the parent
(`opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj`) compares them and owns any
adoption decision. Nothing here authorizes implementation, dependency
changes, kubeless-side edits or ticket mutation.
