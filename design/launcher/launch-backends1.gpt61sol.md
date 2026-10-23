---
type: ArchitectureCheckpoint
title: Independent workspace views need an explicit client-origin contract
description: Two-real-terminal findings, the validated session-group mechanism and the unresolved choice needed before robust native switching
resource: /design/launcher/launch-backends1.gpt61sol.md
status: decision-needed
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-23
sources:
  - resource: opencode://ses_e9a2dad06ffeFURNHXCj1ln0ea
    title: Implement initial native creation, then investigate the clarified independent-view requirement and reproduce wrong-client switching with two real terminal clients
  - resource: opencode://ses_e9a6011b0ffeBJ4GW5UhFqoHb5
    title: Human requests more tmax switching and attachment work and clarifies independent current-window selection as the main outcome; return at a consequential unresolved choice
  - resource: /design/launcher/launch-backends0.gpt61sol.md
    title: Prior native creation slice and retained grouped/terminal boundary
  - resource: /verification/trials/independent-views/README.md
    title: Version-asserted two-PTY trial, preserved reproduction and both backend reports
  - resource: /session/launcher/src/plan.rs
    title: Current exact satellite queue and switch without an explicit client selector
  - resource: /session/launcher/src/socket.rs
    title: Current inherited TMUX endpoint/client-mode policy, not a terminal-client identity
  - resource: https://github.com/tmux/tmux/blob/3.7c/cmd-find.c
    title: Read from the local 3.7c archive extraction; pane lookup and activity-based best-session/best-client fallback
  - resource: https://github.com/tmux/tmux/blob/3.7c/cmd-if-shell.c
    title: Read from the local tag extraction; -F evaluates a format and inserts parsed branch commands into the same queue without launching a shell
  - resource: file:///home/rektide/src/tmux-interface-control/src/query/clients.rs
    title: Attached-client name, tty, session and PID facts do not identify which client supplied shared-pane input
  - resource: file:///home/rektide/src/tmux-interface-control/src/operation/invocation.rs
    title: Single-invocation argv/control encoders; no public native command-list execution API
---

# The user clarified the outcome

On October 23 the human clarified that separate tta terminals should each choose
which window they show, while sharing the project work. Grouping is a possible
mechanism, not the desired feature in its own right. Work anchor:
**tta-launch-backends-independent-views**, child of **tta-launch-backends**.

The two-terminal [preserved trial](/verification/trials/independent-views/README.md)
establishes that tmux session groups do provide this exact window-selection
behavior. Plain tta currently joins the same session and therefore couples the
current window. Explicit `--group` gives distinct sessions over the same windows
and panes, with independent current-window selection. That is not evidence that
grouping was misbuilt; the main outcome is currently opt-in.

## A second, consequential defect

Even grouped terminals can show the **same shared pane**. The shell and every
subprocess in that pane have one environment, one tty and one process ancestry.
Commands entered through terminals A and B see the same `TMUX`, `TMUX_PANE` and
pane tty. The session component in `TMUX` comes from pane creation; it does not
change according to which grouped client supplied the next input byte.

This is not merely theoretical. In the trial, A enters a delayed `tta other`,
B becomes active during the delay, and tta switches **B** to `other`. A remains
in `work`. This reproduces on both native and CLI backend selections on pinned
tmux **3.7c**, with actual terminal clients rather than control-mode stand-ins.

The current switch is still CLI-owned, so the defect already exists; native
switching must not silently preserve it while claiming reliable caller identity.
tmux's `cmd_find_current_client` selects a best session containing the input pane
and then the most recently active attached client. Its tty lookup only identifies
the shared **pane**, not the physical terminal client. A native connection's empty
tty can recover the pane from `TMUX_PANE`, but cannot recover the missing origin.

Consequences:

- Querying `client_name` first merely freezes the heuristic's answer.
- Parsing the inherited `TMUX` session ID is not authoritative after grouping.
- A list of attached client PIDs/ttys establishes candidates, not the source of
  input to a shared shell. PID ancestry and Linux pane tty inspection cannot
  distinguish the same subprocess launched through either terminal.
- Recent activity is a useful convenience heuristic, not a robust origin token.

# Decision needed before a reliable native switch

**Recommended contract:** make grouped independent views the CLI default, keep
`--group=false` as the intentional shared-selection escape hatch, and introduce
authoritative client context at a boundary that actually knows the client—such
as a tmux key binding or integration expanding `#{client_name}`. Switch by an
explicit `-c` selector. Automatically resolve only genuinely unique candidates;
refuse unresolved ambiguity rather than moving an unrelated terminal.

That adds an integration/context contract and changes ambiguous inside behavior.
It is not a harmless implementation detail, so it has **not** been silently chosen.
Concrete alternatives for the parent/human:

1. **Strict origin contract (recommended):** add a client-context input/integration,
   validate it against the selected server, and native-switch that explicit client.
   Ambiguous ordinary shared-pane calls fail with actionable guidance.
2. **Compatibility heuristic:** deliberately keep tmux's best-client heuristic and
   allow native switching with that same weaker guarantee. It can move B when A
   invoked tta, as reproduced. This does not satisfy the stronger reliable-origin
   requirement without explicitly relaxing it.
3. **Separate the deliverables:** change outside entry to independent views first,
   retaining current CLI inside switching until integration is designed. This
   meets the outside multi-terminal window goal, but does not fix the demonstrated
   inside wrong-client defect. It should not be described as robust switching.

No production launch code or defaults were changed at this checkpoint. Keeping
the existing behavior is not an endorsement; it avoids inventing the requested
new context/ambiguity contract without agreement.

## Native attachment preparation and lifecycle

tmax still has no interactive terminal UI. The final attached terminal remains
a real tmux client exec; control-mode attachment is not a substitute.

The previous detached satellite blocker should not be treated as fundamental.
An explicit client makes native grouped inside work plausible as one server
queue: create → switch `-c CLIENT` → set satellite `keep-last`. Outside, a possible
bridge is one native queue creating the satellite and setting **only its local**
`destroy-unattached=off`, followed by CLI attach → set `keep-last`. This could
avoid inherited destruction before attachment without changing group-wide state;
failed attachment would deliberately leave residue, not trigger auto-pruning.

However, tmax exposes single `Invocation` execution, not a structured native
queue API. tmux's supported `if-shell -F 1 COMMANDS` inserts parsed branch commands
without launching a shell, and `Invocation::control_line()` can encode literal
commands. Whether to use that explicit composition or add a small command-list
API needs focused native error, queue-ordering and lifecycle tests. No safe native
queue composition was implemented or claimed verified in this checkpoint.

After the origin choice, the next coherent slice should also resolve:

- Same-project inside entry should reuse the terminal's current independent view,
  not create a satellite on every invocation. This needs actual client identity.
- Project/group identity must survive a missing base session; matching satellite
  substrings and recursively deriving names is not a canonical workspace model.
- Simultaneous entrants need deliberate collision semantics. Current smallest-
  counter inventory naming can race; no blind mutation retry should be added.
- Two-client tests must cover cold first entry, later entry, independent window
  changes both ways, shared panes, explicit native target, ambiguity, same-terminal
  reuse/switch, detach and last-view survival, both backends and shell emission.

These are scoped acceptance work, not justification for a full terminal client
or unsolicited reconciliation. Library-side findings are linked in the
[tta consumer account](file:///home/rektide/src/tmux-interface-control/doc/consumer/tta.md).
