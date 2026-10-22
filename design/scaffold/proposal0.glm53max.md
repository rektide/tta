---
type: Proposal
title: Scaffold mode for missing projects
description: Commissioned design and decisions for generating a missing project before creating its tmux session — adapter crates, TOML registry, posture, and failure semantics
resource: /design/scaffold/proposal0.glm53max.md
status: implemented
author: model:zai-coding-plan/glm-5.3#max
date: 2026-10-22
sources:
  - resource: opencode://ses_e9b671d21ffeJXXTDi6GZnjm60
    title: Commissioning conversation and v1 implementation — adapter crate, launcher create-mode, TOML registry, CLI posture, tests, and this proposal
  - resource: opencode://ses_e9ae39decffe87hP4j4wpy51NC
    title: flash-max filesystem search locating the winter assembly-kit reference (start-it-up) for the survey ticket
  - resource: file:///home/rektide/.local/bin/rekon
    title: The rekon scaffold script wrapped by the first adapter; its ~/src hardcode and npm/jj workflow shape the caveats
  - resource: /README.md
    title: Maintained guide now carrying the scaffold section, tables, and feature documentation
  - resource: file:///home/rektide/src/start-it-up/ARCHITECTURE.md
    title: Winter 2026 project-scaffolding CLI design — prior art for future adapters, via the survey
tags: [scaffold, design, decisions]
---

# Scaffold mode for missing projects

Commission: "a mode where we can run one (maybe more?) skeleton generator tools
if the project doesn't exist." tta resolves session → directory → (previously)
error. This proposal records the decisions taken in
the commissioning session and the v1 shape implemented the same day.
The maintained guide for users is the
[scaffold section](/README.md#scaffolding-missing-projects); this document owns
the reasoning and the open ends.

## Situation

Before this work, a clean miss — no session match, no immediate directory under
the root, no candidates — ended resolution with "no matching session or project
directory" and created nothing. The user's actual workflow when starting a
project runs the [`rekon` script](file:///home/rektide/.local/bin/rekon)
("a godforsaken terrible shell script I suffer with that just requires the
project name"): mkdir under `~/src`, git/jj init, remotes, `npm init` with jq
surgery, `pnpm i`, initial commit. The miss error and the script are one
workflow split across two tools.

## Decisions

Recorded from the commissioning session, with the fork that shaped each:

1. **Named registry, not a bare flag** (user selection). Generators are chosen
   by name at invocation or by a configured default — never guessed.
2. **Each scaffolding system is its own crate** behind an `app/cli` cargo
   feature. v1 ships `scaffold/rekon`, which *wraps* the external script as
   `rekon <name>`; a native Rust port ("and not so crappy") is deliberately
   deferred. Consequence recorded as a caveat: the script hardcodes `~/src`, so
   a non-default root diverges until the port.
3. **The registry lives in a config file** (`TTA_CONFIG`, else the XDG/HOME
   default), not compile-time names. Entries map user-chosen names to compiled
   adapters; `[scaffold] default` picks bare-mode's generator. An absent file
   is an empty registry and misses stay inert — writing the file is the
   deliberate act that arms generation.
4. **Default create-on-miss**, with the `scaffold-by-default` feature flipping
   the compiled default to opt-in. Per-invocation: `--scaffold[=NAME]` /
   `--scaffold=false` and `TTA_SCAFFOLD`, layered CLI > environment > compiled
   default like the existing trace/backend surfaces.
5. **Failure semantics stay tta-shaped**: ambiguity stops (never resolved by
   generating); existing sessions and directories win over scaffolding; the
   fragment must be a safe single path component; name validation and collision
   checks run *before* the generator; an existing path under the target
   (including a dangling symlink) is refused.

## Architecture

- `session/launcher` owns the decision: `resolve` gains a `create` flag, and a
  clean miss becomes `Selection::Create { directory, session_name }` after the
  fragment-as-name validations. The launcher knows generators only as an
  opaque plan prefix; `launch` prepends it before `cd` + `new-session`.
- Adapter crates depend only on `tta-command`: a free function returning the
  generator expression, plus a registry name. `app/cli` composes the registry
  table behind cfg features; unknown adapters in config fail at load with the
  compiled list. A shared contract crate is deliberately deferred until a
  second adapter shows real duplication.
- The plan is `generator && cd -P <root>/<name> && tmux …` in both direct and
  `--print-shell` modes: direct mode runs the generator in the foreground
  (interactive scaffolds work) and the `cd` verifies the directory appeared;
  printed plans perform the generation when evaluated, preserving snapshot
  semantics. Generator programs are bare PATH names resolved at run time.

## Open ends

- **Native rekon port** honoring the configured root and dropping the script
  dependency; the wrap is the starting point, not the destination.
- **More adapters** from the survey: the board ticket `tta-scaffold-survey`
  carries the search for other homegrown scaffolding and the winter
  "assembly kit" reference, identified as
  [start-it-up](file:///home/rektide/src/start-it-up/ARCHITECTURE.md) — a
  plugin-per-behavior scaffolding CLI whose design (2026-01-04) predates this
  work. Literal "assembly kit" phrase matches elsewhere (`fgunn`, `yee`) are
  dated summer 2026 and about composition systems, not project scaffolding.
- **Per-adapter configuration** (template sources, arguments) extends the
  registry entry table when an adapter needs parameters.
- **Runtime-verification scenario** exercising a real generator end to end
  (directory created, session entered, cleanup) in `verification/driver`; the
  CLI-level tests cover planning and selection only.

Work anchor: `tta-scaffold` (epic) and `tta-scaffold-survey` (child) on the
project's Beads board.
