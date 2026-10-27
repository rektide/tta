---
type: ImplementationBrief
title: Independent outside views and guarded native inside switching
description: Agreed behavior, current implementation boundaries, and instructions for the next launch slice; input-origin assistance remains a separate investigation.
resource: /design/launcher/launch-backends2.gpt6a.md
status: design-ready-not-implemented
author: model:openai/gpt-6-astra#xhigh
date: 2026-10-27
sources:
  - resource: opencode://ses_e9a6011b0ffeBJ4GW5UhFqoHb5
    title: Human selects independent outside views, native switching guarded by unique window visibility, separate exploratory input-origin tracking, then explicitly pauses implementation for design and instructions
  - resource: /design/launcher/launch-backends1.gpt61sol.md
    title: Earlier targeting checkpoint; its request for a client-integration decision is superseded by the human's guard choice
  - resource: /verification/trials/independent-views/README.md
    title: Preserved two-terminal evidence of window independence and wrong-client switching
  - resource: /design/launcher/runner-core1.gpt61sol.md
    title: Implemented pure-core and optional-runner boundaries that the next slice must preserve
  - resource: /session/launcher/src/workflow.rs
    title: Explicit inputs, observation requests/replies, satellite naming and pure progression
  - resource: /session/launcher/src/prepared.rs
    title: Current expression and semantic native-creation plan
  - resource: /session/launcher/src/plan.rs
    title: Current attach/switch commands, creation ordering and grouped lifecycle queue
  - resource: /session/runner/src/driver.rs
    title: Shared local preparation and observation adapter used by CLI and embedded consumers
  - resource: /session/runner/src/execution.rs
    title: Native detached creation followed by the current CLI handoff
  - resource: /app/cli/src/main.rs
    title: Current boolean group option, local execution and SSH outside-entry composition
  - resource: /session/launcher/src/target.rs
    title: SSH target setup clears inherited tmux context and prepares outside attachment
  - resource: file:///home/rektide/src/tmux-interface-control/src/query/clients.rs
    title: Typed attached clients include name, PID, creation time, session and control-mode flag, but not current window
  - resource: file:///home/rektide/src/tmux-interface-control/src/query/panes/list.rs
    title: Typed pane-to-window identity and duplicate links across sessions
  - resource: file:///home/rektide/src/tmux-interface-control/src/query/windows/list.rs
    title: Typed per-session window links and active-window flags
  - resource: file:///home/rektide/src/tmux-interface-control/src/operation/invocation.rs
    title: One-command argv/control-line encoding, not a native command-list abstraction
---

# Agreed direction and present scope

**This is a design handoff, not an implementation report.** The human first
requested the guarded implementation, then explicitly asked to finish the design
and leave good instructions while adjacent work is still in progress. No new
production behavior, native switching, default, or test result is claimed here.

The October 27 direction settles the earlier choice:

1. Independent views are the default for ordinary **outside-tmux entry**.
2. Support **inside-tmux switching** through tmax, with a default guard: proceed
   only when exactly one attached client is showing the invoking window.
3. Reject ambiguous switching rather than select the most recently active client.
4. Investigate input-origin assistance separately. Potential hints are proposals,
   not authoritative origin evidence or prerequisites for the guarded version.

Work anchors, all under **tta-launch-backends**:

| Ticket | Outcome |
| --- | --- |
| `tta-launch-backends-independent-views` | Default outside entry gives independent window selection |
| `tta-launch-backends-switching` | Regular native switching with the unique-visible-window guard |
| `tta-launch-backends-input-origin` | Explore ways to assist ambiguous inside calls without the guard; no chosen mechanism |

The first two can be delivered independently. Neither depends on resolving the
third. The [previous checkpoint](launch-backends1.gpt61sol.md) remains evidence,
but requiring a new client-binding integration before any switching is no longer
the selected direction.

## Plain-language behavior

- **Outside:** `tta project` creates the first workspace or joins its windows
  through a separate grouped session. Each terminal can select a different window;
  programs, panes and window creation/deletion remain shared. The attached tmux
  client still owns terminal display and input.
- **Inside:** `tta other` moves the existing terminal connection to `other`, avoiding
  nested tmux. Native switching means sending `switch-client -c CLIENT -t TARGET`
  through tmax instead of spawning tmux to send that instruction. It does not mean
  implementing terminal display in tta.
- **Two clients showing the invoking window:** inside entry rejects, with an
  explanation that it cannot select between those terminals. Outside entry is
  unaffected. Moving one view to another window or invoking from an outside shell
  permits the corresponding ordinary flow.

# Guard contract

## What is counted

The source is the invocation's **pane identity**, normally `TMUX_PANE`, resolved
on the already-selected server to its canonical window ID. Do not derive caller
identity from the session component of inherited `TMUX`: grouped sessions can
share a pane whose environment predates the current attachment.

Count attached clients whose **currently displayed window ID** equals that window,
across the entire server, including other sessions linking the same window.
The rule concerns the window, not just its active pane: clients showing different
panes in the same window still make the call ambiguous. Clients merely belonging
to a session group, but showing another window, do not count. Do not substitute
`session_attached` or group membership for this observation.

For the first implementation, count control-mode clients conservatively too:
they appear in attached-client inventory and have selected session/window state.
Do not silently ignore a second client because it lacks a terminal tty. This is
an implementation policy proposed here; real terminal clients remain mandatory
for acceptance of the human-facing outcome.

| Observation | Decision |
| --- | --- |
| Exactly one client showing the source window | Retain its exact client name, PID and available creation identity; explicitly target it |
| More than one | Reject before any generator, creation, group mutation or switch |
| None | Reject: there is no uniquely visible caller to move |
| Missing/invalid source pane, stale pane, failed or incomplete query | Report the failed precondition; never infer a client from activity |

Error example: `cannot switch: 2 clients show window @7 containing pane %4;
run tta from an outside terminal or leave this window visible in only one client`.
Include candidate identifiers where helpful; never label one the proven input
origin. An explicit `--group=false` changes view behavior, not the switching guard.

## Visibility is not historical input provenance

The human selected a **current-visibility policy**. A unique visible client is
not proof of which terminal typed an earlier command. For example, after a delay,
A could leave and B could become the sole viewer before tta starts. Record that
limit plainly rather than reopening origin research as a prerequisite.

Observe during preparation, then revalidate before execution-side effects. Pin
the selected client rather than retargeting a different unique client later.
After a potentially slow generator/create step, recheck before the switch; if the
window/client situation changed, stop and leave any already-created work intact.
Compare available client instance identity as well as its name to reduce tty/name
reuse mistakes. Server errors and uncertain mutations are not reasons to retry
through another backend.

Separate queries and a subsequent mutation are not atomic. The implementation
must state the check-to-command race and exercise it where controllable. A
server-side conditional/queue could narrow it, but is not already verified or a
promise of input-origin certainty. Do not build an always-subscribed monitor for
this slice or claim that a final preflight is a transactional guard.

# Instructions against the current architecture

## Preserve the pure core

The code moved since the earlier launch discussion. Runtime execution now lives
in **`session/runner`**, while **`session/launcher`** is runner-free. Follow the
[core checkpoint](runner-core1.gpt61sol.md), not the obsolete
`session/launcher/src/execution.rs` path in historical notes.

1. Extend `workflow::Input` with explicit source-pane/context data needed for
   inside entry. Local acquisition belongs in the runner; pure core callers
   supply it. Add an inspectable observation request/reply for switching facts
   and a pure guard decision. Avoid doing environment reads in `plan.rs`.
2. Represent the resolved switching target and its precondition as semantic plan
   data in `prepared.rs`. Extend `prepare_launch` and relevant forwarding APIs.
   Both native execution and CLI lowering consume that same decision. Do not
   recover semantic commands by parsing the rendered shell expression.
3. Put acquisition and revalidation in runner adapters. Native reads use tmax;
   CLI reads use the same operation/decoder contracts through bounded capture.
   Unknown, truncated and failed observations must not become an empty client set.
4. In `plan.rs`, supply explicit `-c CLIENT` for every actual switch and exact
   session targeting as today. Preserve literal argv and tmux format boundaries.
   An inside plan lacking required source/guard context must fail rather than
   silently generate today's heuristic switch. Update low-level callers/tests
   deliberately, including the embedded consumer.

### Available tmax facts

Current `Clients::all()` returns client name, PID, optional creation timestamp,
session ID and control-mode flag. It does **not** include the current window ID.
`Windows::all()` returns session/window links and `active`; `Panes` supplies
pane/window IDs, possibly repeated across linked sessions.

A starting acquisition can join client session IDs to active window links and
resolve the source pane's window, rejecting inconsistent observations. Check that
join against actual `list-clients` window values in the real-client tests. An
alternative focused typed client-view query can collect current window and client
identity together if the join's consistency/complexity warrants it. Reuse tmax's
strict field codec; do not introduce naive tab parsing of arbitrary client names.
Neither approach proves historical input origin. A tmax API addition is a scoped
implementation choice, not an assumption that the field already exists.

## Native switching delivery

Start with an existing destination and no grouped creation: after a successful
guard, run an explicit `Invocation::new("switch-client")` with `-c` and `-t`
through the selected tmax endpoint. Return its outcome normally; there is no new
terminal client to exec. CLI mode sends the equivalent explicit command and
enforces the same guard. Prove native success with a tmux executable recorder
showing that no `switch-client` subprocess was used.

Then compose this with existing detached project/scaffold creation. Keep generator
short-circuiting, physical cwd, startup argv and failure residue contracts. UTF-8
eligible creation stays native; raw Unix startup argv may still require CLI
creation, followed by guarded native switching if the plan models that split.
Do not make unsupported byte argv a reason to bypass the guard.

Grouped inside create → switch → `keep-last` remains one lifecycle-sensitive
queue. For this delivery it can remain CLI-owned **with the same guard and explicit
client target**. Do not split that queue just to report more native commands.
Native grouped queue work remains under the broader parent; it is not required
to implement regular native switching. Actual outside terminal attachment also
remains a tmux client exec.

## Independent outside views

Make CLI `--group` accept an optional boolean, including `--group=false` as the
shared-selection opt-out. Resolve the omitted value to true for outside attach
and retain inside's existing explicit grouping policy for this slice. SSH entry
is also outside entry: `target.rs` intentionally clears `TMUX`/`TMUX_PANE`.
Pass the resolved choice explicitly through the workflow; do not unexpectedly
change every pure-library builder's default or the embedded consumer's policy.

Cold first entry still creates an ordinary project session. An existing match
uses the tested grouped satellite queue and independent current-window selection.
Keep current deliberate last-view/detach semantics and collision failures; no
cleanup daemon, silent prune, or blind create retry is introduced. Broader group
identity after base-session loss, canonical satellite matching and inside view
reuse remain follow-up scope, not hidden claims of this default change.

## Printed plans and delayed execution

Printing must remain observation-only. Inside printed plans must carry the
explicit client target and a deferred guard/revalidation before mutation, so a
plan printed with one viewer cannot silently run after a second viewer arrives.
Design that lowering at the tmux-specific layer; keep the generic command model
tmux-free. It must not require an installed tta on an SSH target.

This deferred inside precondition is a real implementation item, not something
the current `Expression` already supports. If its faithful lowering is not ready,
explicitly reject inside `--print-shell` for the guarded slice with an actionable
diagnostic; do not emit an unguarded or stale-target plan and claim equivalence.
Outside shell output remains supported. Embedded prepared plans likewise need a
clear revalidation boundary; a preparation result is not perpetual permission to
switch its recorded client.

# Implementation and verification sequence

1. Land pure guard decision/data plus supplied-observation tests; wire local
   native/CLI acquisition and explicit-target ordinary switching. Test ambiguity
   before side effects, revalidation and failure propagation.
2. Land the independently deliverable outside default and boolean opt-out, with
   local/SSH composition and completion/help updates. Existing tests intentionally
   asserting shared-session attachment should request `--group=false` explicitly.
3. Integrate guarded project/scaffold execution and delayed-plan behavior. Update
   consumer-facing APIs/docs and receiving checks, preserving runner-free builds.

Use the existing runtime toolkit and isolated servers under `/tmp/opencode`:

- Two real terminals enter the same project with ordinary `tta`; verify shared
  windows/panes and independent selections in both directions. Include first
  cold creation, later entry, evaluated outside shell output, detach and last view.
- Two clients display the same window: inside `tta other` rejects on both
  backends, leaves both clients unchanged, creates no session and runs no scaffold.
  Repeat with distinct active panes in the same window and separate linked sessions.
- Move B to a different window: an inside invocation in A's window switches only A
  to the explicit destination. Also test zero viewers, missing pane, query failure,
  target disappearance, and client identity changing after preparation.
- Record native calls/client observations independently of trace claims. Cover
  both existing and newly created destinations, failed create, failed switch,
  and changes during a generator. Preserve residue rather than auto-removing it.
- Do not reuse the old trial's **expected wrong-client success** as acceptance;
  preserve that [historical proof](../../verification/trials/independent-views/README.md)
  and add positive outcome assertions in the maintained toolkit/tests.

Run relevant default-native, CLI-only, and combined checks, including meaningful
scaffold-enabled configurations; formatting, clippy, pure-core replay, doctests
and affected consumer checks. Test against an identified tmux version rather than
assuming PATH still holds 3.7c. Public interface changes follow the coordinated
workspace version/pin policy and update affected receiving locks at implementation
time. This design-only change requires neither version bumps nor runtime retesting.

# Input-origin investigation: leads, not commitments

`tta-launch-backends-input-origin` asks whether additional context can enable useful
inside calls when several clients show the source window. Possible leads include
tmux-owned bindings/command prompts that know their client, explicit per-invocation
client hints, and embedding applications that already own a terminal connection.
An inherited pane environment is shared and may be stale; activity timestamps are
heuristics, not proof. Explore validation, expiry/reuse, delayed commands and how
assisted invocation would feel before selecting any interface or trust level.

An always-running subscription is not the assumed solution. Assess whether any
proposed mechanism genuinely links an invocation to input, merely nominates a
target, or cannot distinguish them. Return evidence and tradeoffs. The useful
guarded version and independent views proceed without this investigation being
complete. Library-side context: [tta's tmax consumer account](file:///home/rektide/src/tmux-interface-control/doc/consumer/tta.md).
