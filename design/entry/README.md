---
type: Index
title: Entry architecture inquiry
description: Navigation through the commissioned redesign, prior engine and architecture stages, and delivered project-root work
resource: /design/entry/README.md
status: maintained
author: model:openai/gpt-6-astra#high
date: 2026-11-09
sources:
  - resource: opencode://ses_e3db9cabaffeSpjYjLIjvqdbPO
    title: Connect the newly commissioned destination redesign to previous stages without changing their delivery standing
  - resource: /design/entry/redesign0.gpt6a.md
    title: Proposed owned entry protocol and reviewable implementation sequence
---

# Entry architecture inquiry

## Destination under review

- [Entry attempts — state-machine-powered redesign](redesign0.gpt6a.md): proposed
  replacement backbone, domain model, diagrams, Rust interfaces, engine fit,
  concrete journeys and staged landing. Commissioned after the human clarified
  that the intended architecture pass was a substantial redesign.

## Earlier engine and architecture stages

- [Faithful xmachines port](xmachines-port0.gpt61sol.md): selected runtime,
  preserved behavior, verification and implementation friction.
- [Preservation-oriented architecture review](architecture0.gpt61sol.md): typed
  dispatch, diagram checks and replay measurements. Evidence for comparison with
  the later redesign, not a constraint requiring its destination to be retained.
- [XState target investigation](xstate6-target0.gpt61sol.md): package/semantic
  target investigation preceding the selected runtime port.

## Delivered capability work and navigation

- [Ordered-root establishing checkpoint](project-roots0.gpt61sol.md) and
  [integration checkpoint](project-roots1.glm53max.md): implementation history;
  [maintained root contract](/doc/entry/project-roots.md) owns current behavior.
- [Capability-oriented ticket pitch](ticket-surface0.gpt6a.md): accepted work-map
  proposal and historical delivery organization.
- [Maintained entry contract](/doc/entry/README.md): current implemented architecture
  and independent-view semantics; proposed replacements remain in this inquiry.
