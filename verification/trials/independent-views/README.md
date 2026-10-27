---
type: VerificationTrial
title: Two terminals, independent windows and ambiguous command origin
description: Preserved real-PTY reproduction separating correct tmux group behavior from an existing wrong-client inside switch
resource: /verification/trials/independent-views/README.md
status: defect-reproduced
author: model:openai/gpt-6.1-sol#xhigh
date: 2026-10-23
sources:
  - resource: opencode://ses_e9a6011b0ffeBJ4GW5UhFqoHb5
    title: Connect preserved wrong-client reproduction to the human-selected visibility guard and next implementation brief
  - resource: opencode://ses_e9a2dad06ffeFURNHXCj1ln0ea
    title: Reproduce current default coupled windows, correct grouped independent views and wrong-client inside switching with two real terminal clients on both backends
  - resource: opencode://ses_e9a6011b0ffeBJ4GW5UhFqoHb5
    title: Human clarifies independent window selection as the primary goal and asks for robust native invoking-client targeting
  - resource: /app/cli/src/main.rs
    title: Current CLI group flag, backend selection and terminal handoff
  - resource: /session/launcher/src/plan.rs
    title: Current explicit grouped satellite queue and implicit switch-client targeting
---

# A reproduction, not an acceptance of the defect

Work: **tta-launch-backends-independent-views**, child of **tta-launch-backends**.
Decision checkpoint: [independent views and client origin](/design/launcher/launch-backends1.gpt61sol.md).

The October 27 [implementation brief](/design/launcher/launch-backends2.gpt6a.md)
selects independent outside views and guarded native switching. This trial remains
historical defect evidence; future acceptance must assert rejection of ambiguous
inside calls, not success in reproducing the wrong-client switch.

This trial runs **two real PTYs**, each executing the actual tta binary, against
fresh exclusively owned sockets under `/tmp/opencode`. It uses Python's standard
library for exploratory terminal driving; it is deliberately not a new Cargo
acceptance gate. The initial scratch version was promoted because the result
changes the targeting design, not because the current behavior is correct.

## Run

Build tta with both backends. Put a version-asserted tmux fixture first on PATH,
then run each backend separately. These runs used the local archive's **3.7c**
tag, built in `.test-agent/launch-backends/tmux37c/`.

```sh
cargo build -p tta --all-features
PATH="$PWD/.test-agent/launch-backends/tmux37c:$PATH" \
  python3 verification/trials/independent-views/proof.py \
  --tta target/debug/tta --backend native
PATH="$PWD/.test-agent/launch-backends/tmux37c:$PATH" \
  python3 verification/trials/independent-views/proof.py \
  --tta target/debug/tta --backend cli
```

Success means **the existing defect reproduced**; do not wire this expected-wrong
assertion into a future acceptance suite. Each run reports the actual client
PIDs/names, sessions, windows, panes and selected target, and kills only its owned
server. Recorded observations: [native](native0.json), [CLI](cli0.json).

## What it distinguishes

1. Cold first entry creates/attaches `work`. Plain second `tta work` attaches to
   the same session. Changing its current window also moves the first terminal.
2. With explicit `--group`, second entry creates `_work-1` with the same pane.
   Selecting its second window leaves the first terminal's current window alone.
3. Put both clients back on the same shared pane. Commands entered through A
   and B observe identical `TMUX`, `TMUX_PANE` and `tty` values.
4. A enters `sleep 1; tta other`. During the delay, B issues a tmux command from
   its own command prompt. **B**, not A, is then switched to `other`, on both
   backend selections. Native creation still hands switching to the CLI today.

The last result is an existing targeting defect, not a native protocol bug.
A native read of `client_name` would select by the same available heuristics;
freezing that value would not establish the actual origin of A's command.

This trial does not establish default policy, repeated-entry reuse, concurrent
name allocation, detach/last-view behavior or a native queue implementation.
Those remain implementation acceptance work after the targeting contract is
decided. No production defaults or launch code were changed for this checkpoint.

Both preserved-script runs exited successfully, reproducing all four cases
(plain/grouped × native/CLI). The existing Cargo `acceptance` and `launch` suites
also passed **24/24** on the pinned binary; their success does not cover this
newly reproduced two-client origin ambiguity. The current combined-feature CLI
build and JSON parsing passed. Markdown links, structure and session provenance
were checked with `rkt`. No Rust production source changed, so no new API version
or claim of a newly implemented native-switch capability is attached to this trial.
