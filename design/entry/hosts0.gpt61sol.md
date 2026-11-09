---
type: ImplementationReport
title: Shared target observation and read-only catalogs — entry attempt hosts stage
description: Machine-owned raw context policy and registry acquisition, frozen local and worker-free SSH read interpreters, native diagnostics and separate typed catalog querying, with the exact execution/cutover handoff
resource: /design/entry/hosts0.gpt61sol.md
status: implemented-internal
author: model:openai/gpt-6.1-sol#high
date: 2026-11-09
sources:
  - resource: opencode://ses_e3d6e664fffewNf1Zskfd1w5gU
    title: Implement and verify the approved hosts stage, preserve native context/error boundaries, and hand execution plus actual caller cutover to G
  - resource: opencode://ses_e4e2be9c6ffe1NOXcKTwKGp6fj
    title: Parent commissions the Sol hosts stage under the human-approved substantial redesign and retains architectural acceptance and rotation
  - resource: /design/entry/redesign0.gpt6a.md
    title: Approved destination, stage-three acceptance and execution/cutover obligations
  - resource: /design/entry/discovery0.glm53m.md
    title: Complete preceding discovery/creation stage and exact host acquisition handoff
  - resource: /design/entry/kernel0.gpt61sol.md
    title: Retained kernel, native protocol and competing-owner deletion obligations
  - resource: /session/launcher/src/entry/context.rs
    title: Shared pure policy over caller-native supplied facts, active whole layers and frozen target setup
  - resource: /session/launcher/src/entry/mod.rs
    title: Same retained actor acquiring context/registry before selection and validating exact read ingress
  - resource: /session/launcher/src/entry/catalog.rs
    title: Separate read-only actor, composed binding subchart, partial sources and typed candidate projections
  - resource: /session/launcher/src/entry/registry.rs
    title: Bounded one-fact registry acquisition task with classified component inspection
  - resource: /session/launcher/src/entry/probe.rs
    title: New request-bound worker-free codecs, including physical configured-route observations
  - resource: /session/launcher/src/probe.rs
    title: Shared nonce framing, complete-EOF gates, native session facts and classified filesystem mechanics
  - resource: /session/runner/src/observe/mod.rs
    title: One read driver, frozen host guard and native-error transport boundary
  - resource: /session/runner/src/observe/local.rs
    title: Raw local acquisition, explicit session capabilities, filesystem/default observers and bounded native capture failures
  - resource: /session/runner/src/observe/ssh.rs
    title: Frozen connection/setup interpreter with runtime-owned nonces, complete capture and retained receipts
  - resource: /session/runner/tests/entry_hosts.rs
    title: Local/probe agreement, same-driver journeys, target mismatch rejection, raw context and bounded diagnostic witnesses
  - resource: /session/launcher/tests/entry_purity.rs
    title: Runner-free supplied-facts journeys with poisoned commands and invoking-host environment
  - resource: /doc/entry/project-roots.md
    title: Physical identity, route provenance, complete-root selection and partial completion constraints
  - resource: /doc/consumer/term-preserver.md
    title: Actual consumer grouping, worker/retry ownership and non-library completion obligations
  - resource: file:///home/rektide/src/term-preserver/src/launcher.rs
    title: Inspected embedded_tta and embedded_ssh call sites and the consumer's independent roots policy
  - resource: /app/cli/src/main.rs
    title: Still-production preparation/completion paths and duplicate operation classification to replace at cutover
  - resource: /session/runner/src/execution.rs
    title: Still-production post-plan native eligibility and mutation sequence to move into the attempt
---

# Outcome and standing

**Stage three is implemented internally and ready for parent acceptance.** The
same retained `EntryAttempt` now owns context acquisition, pure policy and registry
acquisition before selection. Local and SSH hosts interpret its correlated reads;
one small read driver also drives a separate mutation-incapable `CatalogQuery`.
No new entry path delegates to the old bootstrap/workflow/resolver programs.
Source scans of both new `entry/` and runner `observe/` find no references to
`workflow`, `Bootstrap`, `NeedProjects`, `NeedDestination`, `NativeCreation`,
legacy `Selection` or resolver/scaffold orchestration calls. The local interpreter
deliberately reuses only `resolve::project_inventory` and `destination_state`
filesystem mechanics; those functions do not schedule entry.

Work anchor: `tta-entry-attempt-hosts`, under `tta-entry-attempt`. The ticket is
claimed; acceptance/closure remains with the parent. The epic stays active.
Production execution and actual CLI/consumer adoption remain
`tta-entry-attempt-cutover`. No nested agent, push, installation, consumer edit,
unrelated switching/carrier feature or kubeless inquiry occurred. Mixed shared
`.beads/issues.jsonl` changes are excluded from all commits.

## Internal integration, not a compatibility facade

The existing crate split requires cross-crate visibility to compile real runner
interpreters. `entry-internal` therefore opts into a documentation-hidden
`tta_launcher::entry` and `tta_runner::observe`. Without that feature, entry stays
private. These are the actual new domain/protocol implementations, not public
wrappers forwarding to 0.7. Production callers still use the old path.

No package was published and versions/exact pins/locks remain **0.7.0**. The
coordinated public minor/API/receiving-lock change belongs to cutover; remove this
temporary feature gate rather than presenting it as a second permanent API.

## Owners and changed interfaces

| Owner | Responsibility now established |
| --- | --- |
| `entry/context.rs` | `Facts` with raw `OsString` environment, explicit `Overrides`, target `Origin`/`Scope`, and one pure active-layer policy |
| `EntryAttempt::{acquire,start_at}` | Acquired context and supplied frozen context enter the same chart; no placeholder bound target before context exists |
| `bind_target` compound | context read → policy → optional local endpoint-identity read → semantic registry read → registry/choice validation → selection |
| `ReadRequest` | Exact attempt/sequence/binding plus frozen host/setup/cwd and kind-specific endpoint/program/purpose/slot/path/filter |
| `entry/registry.rs::Task` | Bounded acquisition of one optional registry text fact; no entry/generator/view decisions |
| `entry/probe.rs` | New protocol codecs using shared framing/classification mechanics, not old Setup/Workflow envelope conversion |
| `runner/observe` | One `ReadTask`/`Observer` driver; local and SSH interpreters cannot advance entry policy themselves |
| `CatalogQuery` | Own read-only actor and event/waiting types, shared bind subchart, collect-all-roots loop, report with native per-source failures |

`AttemptSnapshot.target` is now `Option<BoundTarget>`: acquisition has an honest
unbound state. Rich scopes and physical route requests are boxed inside read
envelopes to keep native event/journal variants small; they never cross serde.
Existing semantic decisions, lowerers and fresh-binding inventory ownership remain.

### Context before decoding

One explicit roots list replaces the entire environment layer **before even
decoding** `TTA_ROOTS`. Active JSON/text layers validate; singular path/program/
socket/config/HOME values preserve native bytes. Explicit socket, generator and
prefix overrides similarly suppress replaced values and conflicts. Relative paths
freeze against the observed invocation cwd. Target shell context decoding retains
raw environment bytes too, so an inactive non-UTF-8 value cannot poison an SSH
override before policy runs.

SSH policy uses only supplied target facts and explicit overrides. Local HOME,
roots, tmux program and TMUX are not imported. A remote terminal always attaches;
`TargetSetup` clears TMUX/TMUX_PANE even after explicit environment overrides.
Observed physical cwd replaces the initial setup cwd once and accompanies every
subsequent read and eventual command construction. SSH options are privately owned
by the observer and reused unchanged for terminal construction.

Local entry with differently spelled inherited/chosen sockets requests physical
endpoint identities before authorizing a switch, preserving the existing supplied
canonical-identity policy. Read-only catalog use is explicit in the same context
policy: it can inventory another server without authorizing or demanding a client
switch. That distinction was checked against the actual CLI completion path.

### Acquisition and diagnostics

Registry acquisition is one semantic `Registry` read. Both interpreters use the
same pure component task: at most **256 components**, at most **1 MiB** of registry
text, successful parent enumeration and classified symlink referents, then one
bounded content read. Missing/dangling paths yield explicitly absent text;
ENOTDIR/EACCES/ELOOP, invalid text and incomplete capture never become absence.
SSH may perform several probes to answer that one fact; it cannot select another
entry state. Local filesystem operations remain synchronous OS reads, not promises
of wall-clock bounded execution on an unresponsive filesystem.

Session CLI decoding is shared with framed probes, including strict final-row,
selected-endpoint and exact C-locale absence checks. Local native inventory keeps
the original tmax source type. Defaults are observed only when requested; Git
reads are bounded and explicitly-unset defaults are completed facts. Failed local
command reads retain their raw bounded `capture::Output` in `CaptureFailure`.

SSH records retain the semantic request, acquisition step, bounded streams,
status, EOF/omissions/cleanup and decoded diagnostics (or decode failure). Runtime
hosts generate each nonce; codecs and machine directives do no I/O. `HostFailure`
keeps anyhow ownership and exposes its actual native root as a source. A focused
test caught the otherwise-opaque `anyhow → Box<dyn Error>` conversion; the final
driver preserves the original `Utf8Error` through the stopped attempt instead of
flattening it to diagnostic text. Human-readable receipt summaries are secondary.

`ProjectIdentity` observes the **configured requested-root/name route**, not merely
the already-canonical root/name spelling. Changing that root symlink therefore
produces changed physical evidence, while action lowering still uses the chosen
physical route. Both local and probe interpreters exercise this distinction.

### Catalog and completion

The catalog has only `Observed`/`Cancel` events and `Read`/`Finished` waiting
boundaries. Its closed directive vocabulary cannot construct an entry plan,
delivery or mutation effect. Static chart composition reuses the entry chart's
actual `bind_target` subtree and wires its validated exit to catalog collection;
there is no second native transition table.

Reports retain literal `ViewRef`s, actual workspace group/singleton identities,
one-step aliases, physical project groups and every successful configured route.
Typed `ProjectSelector::{Bare,Qualified}` preserves root slots. Complete inventories
use the existing `completion_candidates` policy; any missing/malformed required
root forbids bare project names but still permits successful qualified routes.
Registry/session/root failures remain attached to their sources. Registry names
come from its single successful parse, without selecting or validating a generator.
The thin line-oriented projection filters controls and deduplicates text without
discarding the report's identity or errors; Zsh escaping remains the frontend's job.

## Checkpoints and focused evidence

1. **`0344f72a`** — context/registry acquisition and shared pure policy in the retained
   entry chart, exact setup ingress and raw-byte witnesses, regenerated entry graph.
2. **`f2712d4d`** — internal cross-crate host backbone, shared codecs/acquisition,
   separate catalog, local/SSH same-driver witnesses and generated catalog graph.
3. Documentation/verification checkpoint — this report, navigation and exact next
   handoff; code-stage receipts above remain separate from acceptance.

| Check executed | Result |
| --- | --- |
| Focused launcher+runner all-feature suite: entry, poison and host tests | **71/71 passed** |
| Same focused suite, no default features, internal entry only | **60/60 passed** |
| Existing runner probe tests after shared framing refactor | **6/6 passed** |
| Internal runner with CLI-only / native-only features | Both compiled |
| Launcher+runner all-target/all-feature clippy, `--no-deps -D warnings` | Passed; pre-existing tmax warnings remain dependency-owned |
| Formatting and entry/catalog graph freshness | Passed |
| Python graph projection regressions | **5/5 passed**, including composed read-only graph |
| `rkt check --okf` on six owned knowledge documents | Passed: 53 links and 64 sources checked |

The poison witness runs pure entry/export and catalog journeys in a subprocess
with poisoned tmux/Git/tta/SSH/system-command PATH and invoking-host environment.
Host fixtures compare local and executed worker-free scripts, include actual
bounded capture, and exercise the SSH interpreter through a literal-argv shell
carrier. **This is not authenticated loopback SSH, real tmux/terminal execution,
CLI cutover or consumer adoption evidence.** Those belong to the next stage.

The preferred `/tmp/opencode` is not writable here. Tests support
`TTA_TEST_TMPDIR`; this pass used `.test-agent/entry-hosts/` with its scratch README,
without changing permissions or user-owned directories. Reproduction commands live
in the [verification guide](/verification/entry/README.md).

## Exact handoff to G: `tta-entry-attempt-cutover`

Read the approved redesign, this report, both preceding reports, the current
chart/action/bind code, and the actual CLI plus term-preserver sources. Claim the
existing cutover ticket only after parent acceptance of this hosts checkpoint.
Keep coherent explicit-path commits, no push/install, no nested agents, and leave
mixed beads changes excluded. The kernel's [deletion table](kernel0.gpt61sol.md#actual-deletion-obligations-at-coordinated-cutover)
still governs removal; the hosts stage did not weaken it.

1. **Extend this same attempt through delivery and effects.** Add Execute and
   caller capabilities, immutable bound execution shape, correlated effect IDs,
   requested → in-flight substates, `EffectStarted` before I/O and exactly one
   `EffectFinished`. Inspection and in-flight waiting cannot redispatch. Native
   payloads/errors stay outside serde. `drive_reads` is the read half of the thin
   interpreter, not permission to build another post-plan application.
2. **Choose native eligibility purely before generation/mutation.** Use the one
   fresh binding inventory already retained in `AttemptData.binding`: local,
   nonempty live server, inside-switch, supported argv and host capability. Cold/
   sessionless/outside/raw-Unix cases use CLI by chosen policy; independent views
   remain one coherent CLI queue. Do not call old `LaunchPlan::native_creation`
   or `ExecuteLaunch` to decide after ready. Export always lowers a complete
   portable terminal block, never a native tail.
3. **Finish every approved binding obligation.** Existing work now checks exact
   anchor ID/name/group; projects compare the configured route's fresh physical
   directory; scaffolds recheck exact physical-root/name lstat absence. The earlier
   discovery slice has **no separate fresh creation-root canonical-identity read**:
   add the approved root witness/check before destination binding rather than
   claiming lstat alone proves the root has not changed. Do not rerank, rescue an
   anchor, choose another root or silently restart an attempt on changed evidence.
4. **Own execution knowledge.** Native scaffold sequence is Generate → check
   produced directory → CreateNative once → terminal entry. Generation/check
   failure prevents later effects. Native failure may already have applied and
   never falls back or retries. Preserve acknowledged creation across later output
   forwarding/handoff failure. Aggregate terminal blocks can only report aggregate
   status/possible mutation; coherent short-circuiting is not transactionality.
5. **Separate dispatch, process replacement and delegation.** A successful exec
   cannot acknowledge attachment back to Rust. `DelegateTerminal` transfers an
   outstanding, undispatched terminal request and ends tta's ownership honestly.
   Cancellation after dispatch preserves uncertainty and prior receipts. The SSH
   interpreter's `terminal_command(target, expression)` only constructs a command
   under its frozen connection/setup; call it at the effect/delegation boundary,
   not directly from `Planned` as an alternate orchestration route.
6. **Cut over the real CLI and actual consumer together.** Replace CLI local/SSH
   preparation, operation-label reconstruction and separate candidate discovery
   with semantic attempts/catalog reports. Do not feed pre-defaulted local CLI
   values into remote `Overrides`. Port term-preserver's inspected `embedded_tta`
   and `embedded_ssh`; its explicit `tmux_group` maps to Shared/Independent, retries
   create fresh attempts, and its worker/proxy/PTY/master options remain its own.
   Retain its non-`tta-lib` completion contract, with shared fixtures rather than
   an incidental crate split. Mosh/guarded switching/daemon work stays separate.
7. **Coordinate the API landing.** Remove old bootstrap/workflow/resolve/scaffold
   entry loops, sidecar plans and duplicate frontend classification when callers
   move; keep only named mechanics. Replace the temporary internal feature gate,
   bump the coordinated next minor from 0.7.0, exact internal pins and receiving
   consumer lock, and test actual call sites. No in-memory migration machinery is
   justified. Run the commissioned native/CLI direct/shell real-terminal matrix,
   worker-free authenticated loopback SSH, scaffold failure/uncertainty cases and
   consumer/non-library completion witnesses. Earlier runtime reports are not
   evidence that those callers have adopted this actor.

The old runner scaffold file's config-path layering and Git-default acquisition
are now competing old-path owners, not additional canonical policy to preserve.
At cutover keep the new shared context policy, registry acquisition task and local
default observer, removing their predecessor routes along with the old loops.

## Connections and boundaries

- [Entry structural graph](/doc/entry/attempt-graph.md) now shows context/registry
  ownership before selection; [catalog graph](/doc/entry/catalog-graph.md) shows
  the separate read-only course and common subchart.
- [Ordered-root contract](/doc/entry/project-roots.md) supplies identity and
  partial-evidence constraints; this stage projects them instead of inventing a
  flattened completion-specific identity model.
- [Consumer account](/doc/consumer/term-preserver.md) grounds the handoff boundary,
  explicit grouping and non-library obligations. It is intentionally not updated
  to claim adoption that has not happened.
- [Current entry account](/doc/entry/README.md) remains the production 0.7 contract.
  Promotion of the complete destination belongs to the separately queued
  consolidation stage, not this internal hosts checkpoint.
