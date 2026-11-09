---
type: Proposal
title: Ticketing tta by the work people enter
description: A small capability-oriented navigation and ticket proposal preserving shipped outcomes, historical identities and separately commissioned future work
resource: /design/entry/ticket-surface0.gpt6a.md
status: accepted
author: model:openai/gpt-6-astra#medium
date: 2026-11-06
updated: { at: 2026-11-09, by: "model:zai-coding-plan/glm-5.3#max" }
sources:
  - resource: opencode://ses_e3e371b6affeGKQKZ8BQx0AGtt
    title: Record human acceptance; promote the compact surface map into the guide and reconcile ticket lifecycle with same-ID summaries
  - resource: opencode://ses_e4dbbb07bffeqm6Khh2fp29jNR
    title: Inspect all sixteen tickets and current capability contracts; propose surface navigation and lifecycle decisions without ticket mutations
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commissions the post-code-complete capability pitch with historical identity preserved
  - resource: /README.md
    title: Current user workflows, matching, configuration, remote entry, reusable APIs and verification
  - resource: /doc/entry/README.md
    title: Implemented pure progression, aliases, independent-view lifecycle and explicit inside-origin boundary
  - resource: /design/entry/architecture0.gpt61sol.md
    title: Independent architecture completion and retained eight-stage graph; separate multi-root follow-up
  - resource: /doc/consumer/term-preserver.md
    title: Actual library adoption, explicit caller policy and consumer-owned transport and retry
  - resource: /verification/views/README.md
    title: Recorded two-terminal acceptance and distinction from the historical wrong-client trial
---

# Pitch: make the board answer “what can I do with tta?”

**Organize navigation around finding work, entering views, creating projects and
entering remote work. Keep tickets as bounded undertakings under those surfaces.**
Show embedding and inspectable plans as first-class capabilities too, without
inventing unfinished implementation to populate them.

The payoff is concrete: “Can two terminals choose different windows?” should lead
to a shipped answer and its evidence; “Can I search two project directories?”
should lead to the existing next-work ticket. Neither question should require
decoding the history of `tta-launch-backends` or an actor-runtime migration.

This is a proposal, not an adopted hierarchy. No tickets were created, renamed,
closed, reparented or otherwise edited for this pitch.

> **Accepted 2026-11-09.** The human approved the pitch ("fine go ahead").
> The compact surface map moved into [the guide](/README.md#what-tta-can-do),
> same-ID title/summary refreshes and lifecycle closures were recorded on the
> board by session `opencode://ses_e3e371b6affeGKQKZ8BQx0AGtt` (parent
> `opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj`), and `tta-project-roots` was
> marked as the commissioned next feature. This document remains the proposal
> evidence; the sentences above describe only what the pitching session did.

## Two things to keep distinct

1. **The durable capability map** describes what exists, limits, and relevant work.
   The [user guide](/README.md) already owns this account. Add a compact work map
   there if accepted; do not create a competing product specification.
2. **Work lifecycle** answers what someone has undertaken and whether it finished.
   A shipped feature remains visible through its closed ticket. A future change
   gets a new bounded ticket; it need not reopen the original delivery.

An epic is useful when coordinating actual related work, not as a permanently open
placeholder for a product feature. A closed epic can remain the historical delivery
anchor in the capability map. Do not read open-child counts as a capability score:
`tta-scaffold` currently shows 2/3 children complete even though the remaining
child explicitly preserves prior art, not an implementation commitment.

## The surface map I recommend

These are navigation headings, not six proposed mandatory epics. “Available” is
based on the maintained contracts and attributed verification, not new runtime
tests in this documentation session.

| Surface people recognize | Available now | Existing work and honest next edge |
| --- | --- | --- |
| **Find and return to work** | Session-first literal/alias/shortest matching; rootless session hits; one project root; read-only Zsh wildcard completion | Retain closed `tta-zsh-completion` and lookup audit `tta-lookup-backends`. Open `tta-project-roots` extends discovery and completion across roots. |
| **Open another view; switch work** | Outside entry gives independent window selection over shared processes; explicit shared selection; orphan aliases, custom prefix and last-view survival | Closed `tta-launch-backends-independent-views` is the delivery anchor. Open `tta-launch-backends-switching` is guarded inside switching; `tta-launch-backends-input-origin` is separate research. |
| **Create and start a project** | Configured scaffold-on-miss, named adapters, native generator options, literal startup argv/cwd, initial window naming | Retain `tta-scaffold`, its two closed implementation children and closed `tta-window-name`. Reconcile the scaffold epic's stale “remaining” list before deciding completion. |
| **Enter work on another host** | SSH target-local observation and real terminal handoff, no target tta required; shared local/embedded planning and diagnostic capture | Closed `tta-launch-backends-remote-target` anchors delivery. Remote-carrier and socket-tunnel siblings are older, distinct proposals requiring relevance review, not missing requirements for shipped SSH. |
| **Embed the same entry policy** | Pure supplied-facts launcher, optional runner, semantic plans; term-preserver adoption with explicit caller grouping respected | The remote-target delivery and [consumer account](/doc/consumer/term-preserver.md) supply history. `tta-state-machine` records the completed engine change, not the whole embedding capability. No additional implementation ticket is needed just for visibility. |
| **Inspect and execute a launch plan** | POSIX shell snapshots, literal command expressions, native/CLI execution boundaries, tracing and bounded diagnostics | Existing launch/remote tickets and [shell contract](/README.md#shell-output) cover delivery. Full deferred workflow compilation is a possible later commission, not an unfulfilled promise of `--print-shell`. |

Grouping, configuration, naming and matching are visible features within these
surfaces. They do not each need a new epic. In particular, “native” is an execution
choice, “SSH” is a target/transport surface, and “Rust library” is a caller surface;
these are not mutually exclusive buckets.

## Ticket changes to propose, in order

### 1. Preserve identity; refresh the current summary

Keep every existing ID, closed result, provenance entry and decision history.
Do not rename `tta-launch-backends-independent-views` to manufacture a cleaner
prefix, or replace `tta-project-roots` with a duplicate under a new namespace.
Readable titles and explained links do most of the work.

Recommended title proposals (same IDs, only after approval):

| Existing ID | Proposed current-facing title | Why |
| --- | --- | --- |
| `tta-launch-backends` | **Launch execution and client targeting — native/CLI delivery and remaining work** | Preserve the backend undertaking while separating delivered creation from the inside-client gap. |
| `tta-launch-backends-switching` | **Switch the sole visible client from inside tmux** | Lead with the user outcome; retain native default, explicit targeting and CLI parity in the contract. |
| `tta-launch-backends-input-origin` | **Identify or explicitly target the invoking terminal when views overlap — research** | Keep uncertainty visible: explicit targeting and proven input origin are different outcomes. |
| `tta-scaffold` | **Create a project on a clean miss with a configured generator** | Describe the feature already present rather than an indefinitely unfinished mode. |

The existing project-roots, completion, initial-window-name and independent-view
titles already describe useful surface. Keep them. Keep `tta-state-machine`'s
historical title: faithfully porting and independently reviewing the runtime was
the actual undertaking, not a misleading name to erase.

For `tta-launch-backends`, prepend a short current-state summary with links to the
[operation matrix](/README.md#which-operations-use-tmax) and the switching ticket.
Its old acceptance text still treats attach/switch as terminal handoff, whereas
the later human direction selects regular native switching. Preserve the old
record and explicitly reconcile the acceptance contract; do not let the older
wording silently overrule the later decision.

### 2. Make completion truthful without discarding future work

| Ticket | Recommended decision for the owner, not an action taken here |
| --- | --- |
| `tta-state-machine` | After the parent presents this pitch, record the faithful port, fresh architecture review and pitch as the completion receipt, then close if accepted. Its acceptance requires presenting a pitch, not implementing the proposed ticket organization. |
| `tta-launch-backends-independent-views` | Keep closed. Prefix/orphan behavior is delivered too; keep inside-origin limitations explicit rather than reopening this outside-entry result. |
| `tta-launch-backends` | Keep its remaining execution/client-targeting scope visible. Do not declare all-native execution complete; do not hold already shipped independent views hostage to it. Owner should resolve whether old grouped/native/raw-byte aspirations remain commitments or merely options. |
| `tta-scaffold` | Reconcile original remainder against current implementation: native adapter and per-adapter configuration are delivered. Confirm whether a dedicated runtime-driver scaffold scenario remains desired beyond current live/SSH tests. If no commissioned remainder exists, close the bounded delivery rather than waiting for hypothetical adapters. |
| `tta-start-it-up-ref` | Preserve the explicit “interesting prior art, not the path” decision. Recommend closing as reference captured, with its link retained under scaffolding. Do not turn it back into an adapter/survey obligation. |
| `tta-launch-backends-remote-carrier` | Review against actual remaining mosh/classic-carrier use. Current SSH bypasses target tta, but mosh still uses it. Retain history; defer if there is no active commission, or clarify the remaining consumer contract before implementation. It is not automatically obsolete or complete. |
| `tta-launch-backends-socket-tunnel` | Treat as an optional native-remote direction, not a prerequisite for usable SSH. Recommend deferral unless this remains selected work; preserve its lifecycle/cold-start/permission questions. |

“Close as reference captured” is not “implementation shipped.” A completion reason
must state the kind of undertaking completed. Likewise, deferral is an owner
choice, not a conclusion that a recorded idea lacks value.

### 3. Add only the containers that make active work easier to see

**My default is the surface map plus existing tickets first.** If the human wants
surface epics directly on the board, these are concrete candidate IDs and titles,
all proposals only:

- `tta-workspace-entry` — **Independent views and deliberate inside switching**.
  Show the closed independent-view delivery, open guarded-switching work and
  separately exploratory input-origin assistance. Acceptance for an active epic
  must name its bounded delivery (for example the guarded-switching contract),
  not promise every future terminal integration.
- `tta-project-discovery` — **Find and complete work across project locations**.
  Coordinate `tta-project-roots` if it grows multiple genuine implementation
  slices. Existing completion and lookup tickets are linked evidence, not reopened
  requirements. With only one active feature, this epic is unnecessary overhead.
- `tta-remote-entry` — **Remote project entry across terminal transports**.
  Useful only if the owner commissions another transport/carrier slice. Link the
  closed SSH delivery; choose a concrete next outcome before opening it as work.
  Do not silently accept both tunneling and carrier redesign as prerequisites.

Reuse `tta-scaffold` for creation rather than inventing `tta-project-creation`.
Do not add empty embedding, configuration, testing or state-machine epics merely
to fill a diagram. If a new embedding API or deferred compiler is commissioned,
name that actual outcome then.

Initially use explained related-work links instead of moving the old children
out of `tta-launch-backends`. If a surface epic becomes the clearer primary owner,
approve reparenting explicitly, preserve the old relationship in a note and keep
the same ID. A parent edge organizes ownership; a dependency edge asserts a real
prerequisite. Neither should be generated from this table mechanically.

## Next concrete work: multiple project roots

**Retain `tta-project-roots` as the next visible discovery feature.** Its recorded
independent-view dependency is satisfied; no new ticketing taxonomy is a blocker.
This pitch does not start its implementation or override the separate pause on
guarded inside switching.

Before coding, its own design should settle:

- Ordered roots and configuration syntax/precedence, including target-local SSH
  configuration and unchanged one-root use.
- Duplicate basenames versus duplicate physical directories: overlap and symlinks
  need provenance, not silent flattening. Decide whether order is selection policy
  or only presentation order; do not assume “ordered” means first match wins.
- How an explicit selector/path disambiguates, and how completion can expose that
  choice without pretending an ambiguous basename uniquely identifies a project.
- An explicit or otherwise unambiguous scaffold destination. A clean miss across
  roots does not authorize an arbitrary creation location.

Keep the existing session-first guard and rootless hits. Carry enough root identity
through supplied facts, local and SSH observations, completion and planning to
explain the selected directory. Revisit the graph only where observation contracts
actually change; [the architecture review](/design/entry/architecture0.gpt61sol.md#remaining-recommendations-and-handoff)
specifically keeps this work separate from runtime selection. One coherent feature
ticket is sufficient until real independently deliverable slices emerge.

## Crosscutting concerns and evidence

- **Inside switching is not outside independence.** The selected policy rejects
  zero/multiple viewers of the invoking window and pins the sole client. It is
  still open and its ticket retains an implementation pause. Historical input
  origin cannot be inferred from shared pane environment or recent activity.
  Research assistance blocks neither outside views nor the selected guard.
- **CLI, library, shell and SSH share policy, not every default source.** Embedded
  callers retain explicit grouping; SSH uses target facts; completion is read-only
  inventory, not winner selection. Put these affected surfaces in a feature's
  acceptance criteria rather than copying the same task four times.
- **Testing belongs with the outcome it proves.** Link two-real-terminal evidence
  for independent views, real client-identity checks for switching, core/probe
  witnesses for purity/transport, and consumer checks for API changes. A separate
  verification ticket is warranted for a reusable missing test capability, not
  automatically for every feature. Passing a graph-conformance check does not
  prove terminal identity or transport correctness.
- **Architecture is valuable enabling work.** Keep the completed eight-stage pure
  xmachines graph, native values/errors and diagram-conformance evidence attached
  to `tta-state-machine`; cross-link it from embedding and entry. Do not create
  eight product tickets for eight internal states. The cost probe establishes no
  new performance obligation or automatic optimization backlog.
- **Consumer ownership stays explicit.** tta prepares actions; term-preserver
  owns attempts, terminal transport and user retry. Mosh observation configuration
  needs a consumer decision. The library map should expose the limitation without
  claiming that tta alone owns its resolution.

## Evidence inspected and decisions requested

On 2026-11-06, `bd list --status all --limit 0` returned **16 tickets: 7 open,
2 in progress, 7 closed**. `bd show` was read for the state-machine, roots,
launch-backends and all six children, scaffold/reference, completion, lookup and
window-name tickets. The two closed scaffold implementation children were visible
in the listing/parent; their implementation standing was cross-checked against
the current guide, not claimed as a fresh code audit. Ticket descriptions, later
notes and closure reasons were read together: several descriptions retain earlier
design-only pauses that later notes explicitly supersede.

Current contracts and the independent architecture report were read, alongside
the consumer and two-terminal evidence homes. Runtime results here are attributed
to those documents; this pitch runs no builds or runtime tests. The preexisting
`.beads/issues.jsonl` changes are outside this documentation checkpoint.

Recommended owner decisions:

1. **Approve the surface map and same-ID summaries first**, with no mass rename or
   reparent. Would a board-visible `tta-workspace-entry` epic help enough to add now,
   or should the compact map carry that navigation?
2. **Accept completion of `tta-state-machine` after pitch presentation**; reconcile
   scaffold/reference lifecycle separately rather than closing by assumption.
3. **Keep multi-root discovery next**, with its duplicate/destination policy
   discussion before implementation. Confirm separately when guarded switching
   may resume; its accepted design does not itself lift the recorded pause.
4. **Review remote carrier/tunnel relevance with the consumer owner** before
   assigning them fresh work. Mosh is the concrete remaining carrier context;
   usable SSH is already delivered.

This yields a board where completed value remains visible, future changes have
honest scope, and implementation history remains recoverable.
