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
