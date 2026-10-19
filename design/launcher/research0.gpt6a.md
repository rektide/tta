---
type: ArchitectureProposal
title: A small launcher with explicit language and terminal boundaries
description: Independent source-grounded recommendations for session-first resolution, reusable command expressions, native versus CLI lookup, and real Rust runtime acceptance
resource: /design/launcher/research0.gpt6a.md
status: research-proposal
author: model:openai/gpt-6-astra#medium
date: 2026-10-19
sources:
  - resource: opencode://ses_eaa0a7a21ffeDLXegLkqkjuq88
    title: Independent Astra medium inspection of in-progress launcher, tmax, versioned tmux source and tracing precedents; architecture and acceptance recommendations
  - resource: opencode://ses_eaa17ad2effeEEkcV1BqiWcrUp
    title: Parent commissions additive research while Sol independently implements the launcher
  - resource: /session/launcher/src/
    title: Read backend, plan, resolve and socket implementations as mutable work in progress
  - resource: /shell/command/src/lib.rs
    title: Read bon-built command, assignment, cwd, conjunction and handoff semantics
  - resource: /app/cli/src/main.rs
    title: Read in-progress CLI and tracing handoff boundary
  - resource: /session/launcher/tests/private_tmux.rs
    title: Read private-server tests; did not execute them or claim their results
  - resource: file:///home/rektide/src/tmux-interface-control/src/endpoint/mod.rs
    title: Native endpoint execution, limits, socket context and failure behavior
  - resource: file:///home/rektide/src/tmux-interface-control/src/operation/invocation.rs
    title: Distinct argv and control-line tmux command encoders
  - resource: file:///home/rektide/src/tmux-interface-control/src/lib.rs
    title: Actual feature and supported-platform boundaries, cross-checked with Cargo.toml and query module documentation
  - resource: https://github.com/tmux/tmux/blob/3.7c/cmd-new-session.c
    title: Read complete tagged source from local archive; create, attach, format expansion, duplicate and cwd behavior
  - resource: https://github.com/tmux/tmux/blob/3.7c/tmux.c
    title: Read tagged socket resolution and label-directory setup from local archive
  - resource: https://github.com/tmux/tmux/blob/f0a2ed30/cmd-attach-session.c
    title: Local archived source inspection of terminal attach and existing session cwd mutation
  - resource: https://github.com/tmux/tmux/blob/f0a2ed30/cmd-switch-client.c
    title: Local archived source inspection of client switching; newer than installed 3.7c
  - resource: file:///home/rektide/src/pipewire-native-rs-node/.design/verification/tracing0.gpt6a.md
    title: Read prior effort's complementary capture and tracing-test/mocking comparison, especially sections 2 and 6
  - resource: file:///home/rektide/src/pipewire-native-rs-node/verification/trace.md
    title: Read maintained saved-trace interface and evidence boundaries through fixed-rule checker introduction
  - resource: file:///home/rektide/src/pipewire-native-rs-node/pipewire/tests/trace_messages.rs
    title: Read isolated same-line tracing-test diagnostic test and exact 0.2.6 no-env-filter dependency declaration
  - resource: https://docs.rs/tracing-mock/0.1.0-beta.3/tracing_mock/
    title: Versioned documentation confirms scoped ordered subscriber expectations, assert_finished and exact beta pin recommendation
---

# Recommendation and read-time ground

Keep the original deliverable central: **session-name match first; otherwise an
immediate directory under `~/src`; create a session rooted there and enter it.**
The current `shell/command`, `session/launcher`, `app/cli` split already expresses
the useful reuse boundaries. Add runtime verification alongside it rather than
turning the launcher into a generic terminal orchestration framework.

At entry, working-copy parent was `56aeee8a` and implementation changes were
already owned by Sol. This document describes source inspected during that work,
not a final review of a frozen build. No other same-wave proposal was read. No
implementation, index, or README was changed. No live server was queried or
mutated. The only installed-tmux command executed was `tmux -V`, reporting
**3.7c**; no behavioral tests were run by this researcher.

The archive HEAD is `3.7b-1138-gf0a2ed30`, not the installed release. Socket and
creation claims below were therefore cross-checked against its **3.7c tag**.
The newer `make_label` additionally validates/canonicalizes socket bases, which
must not silently be projected onto 3.7c behavior.

# 1. Useful seams, without a framework

| Boundary | Responsibility | Keep out |
| --- | --- | --- |
| Lookup | Return session facts or a typed failure for one selected endpoint | Creation, fuzzy selection policy, interactive attachment |
| Resolution | Exact-first, then unique literal substring; sessions precede directories | Terminal handling, shell quoting |
| Launch plan | Convert selection plus endpoint/client context into operations | Re-querying or silently choosing another project |
| Expression | Typed argv, assignments, cwd, short-circuit sequence; render or execute | tmux format expansion and target syntax |
| CLI | Inputs, diagnostics, subscriber, stdout contract, terminal handoff | Own tmux protocol implementation |
| Verification | Own fixtures, launch the actual binary, observe outcome | Substitute implementation-only mocks for terminal acceptance |

Bon builders fit the meaningful constructors here. A trait for every command or
every expression node would add little; the small enum plus immutable operands
is enough. Keeping tmax out of the generic shell crate is particularly valuable.

`SessionLookup` can remain one method returning `Vec<Session>`. Introduce richer
inventory metadata only for a demonstrated consumer. Empty means an observed
absence/no sessions, not “any error occurred.” Permission, malformed reply,
protocol failure and timeout must fail before directory creation is considered.

## Native and CLI are execution choices, not different launchers

Verified tmax capabilities:

- `endpoint::Tmux::query(query::Sessions::new())` uses native command execution,
  not an implicitly spawned tmux subprocess or server.
- The endpoint has timeout and retained-output limits. Documentation qualifies
  them: they are not universal end-to-end guarantees for every native profile.
- `operation::Invocation` owns unquoted command words and separate `argv()` and
  `control_line()` encodings. The query module owns reversible text encoding and
  rejects malformed rows rather than returning a partial inventory.
- Native direct execution is little-endian Linux. Persistent `control_mode`
  brings substantially more lifecycle machinery and is unnecessary for one lookup.
- Currently `operation` and `query` themselves are behind tmax's **`client`**
  feature. Thus a CLI-only tta build can still compile tmax native dependencies
  while using only its query/decoder. Describe the distinction honestly: CLI-only
  *execution* is not currently native-code-free *compilation*. An upstream pure
  query feature could improve that later, but need not block this launcher.

Sharing tmax's query format/decoder across both backends is preferable to inventing
a second tab-split schema. The in-progress CLI adapter's `-u` and C-locale handling
are deliberate, important details. Its `Command::output` currently has neither a
deadline nor an output cap; a future shared bounded-process helper is more useful
than pretending parity with the native endpoint's limits. Native failure should
not automatically trigger CLI fallback: an explicit backend selection preserves
diagnostic meaning and prevents different failure policies from being invisible.

# 2. There are three parsing boundaries, plus tmux semantics

1. **POSIX shell syntax:** quote each operand, including an executable named like a
   shell keyword; empty words and apostrophes must survive.
2. **Process argv:** direct execution passes raw values, not prequoted shell text.
3. **tmux command parsing:** even already separated argv can have a trailing `;`
   interpreted as a command separator. tmax's `Invocation::argv()` adds precisely
   the escaping that this parser consumes. Do not bypass or double-apply it.
4. **Command-specific formats:** 3.7c `new-session -s` and `-c` call
   `format_single`. Shell and argv quoting cannot make `#{...}` or `#(...)`
   literal. The current plan's hash encoding belongs here, not in generic `Word`.

Hostile-name tests should exercise all layers together, including trailing
backslash/semicolon, quotes, spaces, `#[...]`, `#{...}`, and `#(...)`. Assertions
must inspect actual stored name and pane cwd, not just a rendered string. Current
private-server tests are a useful start; read-time coverage did not establish a
real interactive attach or switch.

## Execution equivalence needs a stated scope

For `ChangeDir(A) && Command(B)`, direct execution should give B the selected cwd
without process-global `set_current_dir`. This permits concurrent library use.
Rendered `cd -P ... && B` under **eval** intentionally changes the evaluating
shell; direct tta cannot change its parent shell. These are equivalent command
inputs and failure ordering, not identical parent-shell side effects.

Prefer the current short-circuit conjunction over a general shell AST. A failed
cd must suppress creation; failed creation must suppress switch. A failed final
exec must preserve its OS error. Relative path operands, symlink traversal, cd
permissions and shell functions can make broad “identical to any shell” promises
false; target ordinary POSIX shell execution with explicit physical cwd behavior.

Direct mode should exec the real tmux client for normal terminal ownership and
signals. Shell output should omit exec by default, with explicit opt-in replacing
the evaluating shell. Preserve stdout exclusively for complete executable output;
diagnostics and trace events belong on stderr. Render fully before printing.

**A concrete documentation correction:** `eval "$(tta --print-shell project)"`
can conceal a failed tta invocation because eval of empty output succeeds. Prefer:

```sh
tta_plan=$(tta --print-shell project) && eval "$tta_plan"
```

This also makes it clear that lookup occurs during emission. The emitted plan is
a snapshot, not a delayed fresh lookup or a promise that the server still exists.

**A concrete implementation seam to inspect:** `--tmux ./bin/tmux` and
`--tmux-config ./config` are read before a plan's cwd change but used afterward.
The read-time `launch` passes these relative operands unchanged. CLI lookup can
therefore execute a different binary path than final creation, and cold startup
can read a different config. Freeze explicit path operands against entry cwd
(while leaving bare executable names as PATH searches), or explicitly document
project-relative semantics. A relative PATH entry is a further intentional limit
unless the executable itself is resolved once. This is a focused integration
test, not a reason to redesign the expression model.

# 3. Matching, identity, creation and terminal context

## Predictability beats accidental shell glob behavior

Recommended policy is case-sensitive exact match, otherwise unique literal
substring. Sort ambiguity diagnostics. Session ambiguity must stop resolution
rather than fall through to a convenient directory. Do not scan `~/src` when an
existing session already suffices; nonexistent HOME/root should not block attach.

Preserve directory spelling as the requested session name, including a symlink
entry's basename. Do not silently replace punctuation, since that creates name
collisions. The current explicit name override is a reasonable escape hatch for
tmux-invalid names. Declare the UTF-8 boundary: OS paths can be byte strings,
while the CLI fragment and tmax command model are UTF-8. Skipped non-UTF-8 entries
are a deliberate limit, not fully general Unix filename support.

An observed existing session should be targeted by `$id` on its **same endpoint**,
not passed back through tmux fuzzy matching. IDs are server-lifetime-local, not
global durable identities. Server restart between lookup and handoff is a residual
race even when the socket path and ID match. No extra identity service is warranted
for this small launcher, but printed plans must not imply timeless identity.

## Create/attach is not an unconditional `new-session -A`

3.7c source confirms: `new-session` starts a server when necessary, creates and
attaches unless detached, and checks duplicate names. `-A` redirects a duplicate
to attach, passing `-c` through; attach can change the existing session cwd. Thus
the convenience “always new-session -A -c project” loses the requested distinction
between an existing session and a new project session.

The current choice is defensible:

- Outside tmux, existing → attach by ID; new → one create-and-attach invocation.
- Inside the same server, existing → switch; new → detached create then switch
  only on success.
- If another creator wins the duplicate-name race, fail visibly. Do not silently
  accept that session as if it were the project just resolved.

There remains a small detached-create/name-switch race: exact `=name` avoids
fuzzy matching but cannot stop a rename or replacement between commands. Capturing
`new-session -P -F '#{session_id}'` would strengthen the target but adds output
binding to the expression model. Keep the simpler plan unless this is a demonstrated
requirement; do not smuggle a shell substitution node into the reusable AST solely
to eliminate one theoretical race.

## A socket path and startup policy are related but different

3.7c honors explicit `-S`/`-L` over TMUX; otherwise it takes the prefix of TMUX
before the first comma, then defaults to a label under the socket base. A label
also causes creation/checking of the private `tmux-uid` directory. Turning every
label into `-S /.../tmux-uid/name` loses that cold-start behavior.

The in-progress `Resolved { path, label }` therefore preserves real information:
use the frozen path for lookup, and the label/base policy for actual bootstrap.
Resolve relative explicit paths before any cd. Alternate-server selection while
inside tmux cannot switch a client across servers; an explicit error is preferable
to silently nesting a new attach. TMUX environment presence alone does not prove
a usable attached client: stale env, background processes and multiple clients
need real-terminal coverage. Let tmux's client selection semantics remain visible.

# 4. Additive runtime verification with real behavioral witnesses

The useful helper is a **Rust library plus thin runtime command** that launches
the actual built tta binary, owns a private socket and project tree, observes the
server/client, and reports evidence. `cargo fmt/clippy/test` orchestration alone
does not satisfy this request. The in-progress `verification/driver` home fits.

Share only the repeated mechanics: private server ownership, child environment,
bounded process wait/output, PTY lifecycle, and postcondition queries. Acceptance
tests and the helper should call those same Rust functions. A short typed scenario
list is enough; no configurable workflow engine or query DSL is needed.

Private fixtures must override/remove inherited TMUX and TMUX_PANE, use explicit
sockets, disable user config, and clean up only what they own. Model a cold endpoint
separately from an already-running seed server. Bound waits by observable readiness
and a deadline; drain PTY output to avoid introducing hangs. Drop can be best-effort
cleanup, but an explicit finish/report should expose cleanup failures.

| Scenario | Genuine witness |
| --- | --- |
| Existing session beats matching directory | Actual tta attaches/switches to pre-existing ID; no new session |
| Cold start | No socket before lookup; actual tta creates one named session and its pane starts in project cwd |
| Label bootstrap | Base exists but `tmux-uid` does not; final launch creates the private directory correctly |
| Inside-server switch | Real attached client changes session; original pane/client survives; no nested client |
| Ambiguity/missing/backend error | Nonzero status, useful stderr, no unintended new session |
| Direct versus rendered mode | Actual created name/cwd agree; eval mode retains caller shell unless exec explicitly selected |
| Failure ordering | Missing cwd or duplicate create suppresses subsequent command; recorder proves no switch |
| Hostile operands | Actual name/cwd preserved and no extra command/job marker appears |
| Alternate socket | All observed mutations confined to the owned endpoint |

A no-PTY test can prove decoding, mutation and cwd, but cannot prove interactive
attachment. A PTY driver should launch tta with a controlling terminal, query the
private server's client/session relation, then detach that owned client and wait.
For switching, execute tta from an actual pane under an attached client. Merely
setting TMUX in a subprocess with no attached client is not equivalent.

Keep launch exit, server postconditions, trace assertions, and cleanup status
separate in the report. On failure retain bounded stdout/stderr/trace excerpts and
the scenario name. This is sufficient useful instrumentation without importing
the PipeWire effort's much larger campaign/provenance machinery.

# 5. Tracing: three different jobs

The recent library found in the other effort is **`tracing-test =0.2.6` with
`no-env-filter`**, verified in its manifest and actual `trace_messages.rs` test.
That is complementary to the user's explicit **tracing-mock** request.

1. **Application visibility:** standard tracing events and tracing-subscriber
   JSON on stderr, optionally selected by a strict EnvFilter. Stable fields such
   as event, backend, endpoint, decision, session ID, client mode and operation
   make the launcher legible. The read-time CLI already emits a useful small chain.
2. **Narrow contract tests:** exact-pin `tracing-mock = "=0.1.0-beta.3"` as a dev
   dependency. Use `with_default`, declared event/field expectations, and
   `handle.assert_finished()`. Test a deterministic branch or failure boundary,
   not every internal event of tmax and not a concurrent system-wide transcript.
3. **Runtime subprocess evidence:** collect and parse the actual child's JSON
   events in the Rust driver, while checking actual tmux state independently.
   A subscriber installed in the test process cannot capture the child's events.

The old tracing-test pilot is useful for a stable diagnostic on one line, not a
reason to adopt another global collector everywhere. It installs global state;
keep it in a separate test binary from scoped mock/capture tests. Its `logs_assert`
closure holds a buffer lock: join producers first; never log or wait inside it.
The user-requested mock is sufficient for the first focused tracing contract here;
adding tracing-test itself is optional after identifying a distinct benefit.

**Exec defines an observation boundary:** successful handoff destroys the Rust
process image. No Rust “attachment completed” event or RAII writer flush runs
afterward. The last application event should honestly mean *handoff attempted*.
The runtime driver proves attachment through tmux state, and eventually records
the client process's exit. Prefer the existing sparse synchronous output to an
asynchronous logger requiring a pre-exec drain protocol.

Tmux and application errors may share stderr with JSON. Treat non-JSON lines as
diagnostic evidence, not automatically malformed tracing. If a strict pure-JSON
artifact becomes necessary, add a dedicated trace destination with explicit fd
ownership rather than guessing that all client stderr is JSON. stdout must stay
clean in shell-emission mode, including when tracing is enabled.

# Integration and cross-references

- [Command expression implementation](/shell/command/src/lib.rs): apply the
  multi-language and relative-path checks at its consumers, preserving its small
  generic scope.
- [Launch planner](/session/launcher/src/plan.rs) and
  [socket selection](/session/launcher/src/socket.rs): existing architecture
  already supports the recommended create/attach and cold-label distinctions.
- [Private-server tests](/session/launcher/tests/private_tmux.rs): useful
  behavioral groundwork; reusable fixture extraction should serve the real
  binary/PTY cases, not replace them with plan-only assertions.
- [PipeWire trace guide](file:///home/rektide/src/pipewire-native-rs-node/verification/trace.md):
  precedent for typed saved-event inspection and honest evidence scope; its domain
  checker and full campaign architecture are unnecessary here.
- [tmux-tracker README](file:///home/rektide/src/tmux-tracker/README.md): sibling
  dependency/platform precedent; its warning that `cargo fmt --all` traverses
  sibling dependencies supports naming owned packages explicitly when formatting.

Suggested parent README/index link: **Launcher research: language boundaries,
native/CLI lookup and runtime acceptance** → `design/launcher/research0.gpt6a.md`.
Highest-value integration checks are relative executable/config paths, shell
emission failure propagation, and actual PTY create/attach/switch witnesses. These
are additive checks on the existing design, not a replacement implementation plan.
