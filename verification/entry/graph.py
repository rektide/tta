#!/usr/bin/env python3
"""Project the executable entry chart into a checked Markdown Mermaid region.

This is a structural renderer, not another transition table or a claim about
the native matching algorithm. Event arrays are emitted in their declared order.
"""
import argparse
import copy
import html
import json
from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[2]
DEFINITION = ROOT / "session/launcher/src/entry/entry.machine.json"
DOCUMENT = ROOT / "doc/entry/attempt-graph.md"
START = "<!-- entry-machine:start -->"
END = "<!-- entry-machine:end -->"


def catalog_definition(entry, catalog):
    """Same static binding-subchart composition as catalog.rs::definition."""
    catalog = copy.deepcopy(catalog)
    binding = copy.deepcopy(entry["states"]["active"]["states"]["bind_target"])
    binding["states"]["bind_registry"]["on"]["validated"]["target"] = "#catalog_sessions"
    catalog["states"]["catalog_active"]["states"]["bind_target"] = binding
    return catalog


def render(definition):
    nodes = []
    by_path = {}
    by_id = {}

    def walk(node, path):
        identity = node["id"]
        if not re.fullmatch(r"[a-z][a-z0-9_]*", identity) or identity == "end":
            raise ValueError(f"not a supported Mermaid state ID: {identity!r}")
        if identity in by_id:
            raise ValueError(f"duplicate state ID: {identity!r}")
        nodes.append((node, path))
        by_path[path] = identity
        by_id[identity] = node
        for name, child in node.get("states", {}).items():
            walk(child, path + (name,))

    walk(definition, ())
    lines = ["flowchart TD"]
    for node, path in nodes:
        labels = [node["id"]]
        if path and ".".join(path) != node["id"]:
            labels.append(".".join(path))
        if node.get("type") == "final":
            labels.append("final")
        elif "states" in node:
            labels.append("compound")
        for entry in node.get("entry", []):
            if entry["type"] != "@xstate.emit":
                raise ValueError("teach the structural renderer about new entry action types")
            labels.append("emit " + entry["event"]["type"])
        label = "<br/>".join(html.escape(label, quote=True) for label in labels)
        lines.append(f'    {node["id"]}["{label}"]')

    for node, path in nodes:
        source = node["id"]
        if "initial" in node:
            target = by_path[path + (node["initial"],)]
            lines.append(f"    {source} -.->|initial| {target}")
        for event, transitions in node.get("on", {}).items():
            routes = transitions if isinstance(transitions, list) else [transitions]
            for index, transition in enumerate(routes):
                raw_target = transition["target"]
                if raw_target.startswith("#"):
                    target = raw_target[1:]
                    if target not in by_id:
                        raise ValueError(f"unknown absolute target {raw_target}")
                elif raw_target.startswith("."):
                    target = by_path[path + tuple(raw_target[1:].split("."))]
                else:
                    target = by_path[path[:-1] + tuple(raw_target.split("."))]
                label = event
                if len(routes) > 1:
                    label += f", {index + 1}: "
                elif "guard" in transition:
                    label += ", "
                if "guard" in transition:
                    label += transition["guard"]["type"]
                elif len(routes) > 1:
                    label += "otherwise"
                if transition.get("reenter"):
                    label += ", reenter"
                label = html.escape(label, quote=True).replace("|", "&#124;")
                lines.append(f"    {source} -->|{label}| {target}")
    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--catalog", action="store_true", help="compose/project the separate read-only catalog chart")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--check", action="store_true", help="fail on stale generated region, without writing")
    mode.add_argument("--write", action="store_true", help="replace only the generated region, preserving prose/provenance")
    args = parser.parse_args()
    definition = json.loads(DEFINITION.read_text())
    document_path = DOCUMENT
    if args.catalog:
        definition = catalog_definition(definition, json.loads((DEFINITION.parent / "catalog.machine.json").read_text()))
        document_path = ROOT / "doc/entry/catalog-graph.md"
    graph = render(definition)
    if not args.check and not args.write:
        sys.stdout.write(graph)
        return
    document = document_path.read_text()
    if document.count(START) != 1 or document.count(END) != 1:
        raise ValueError("expected exactly one generated region")
    prefix, rest = document.split(START)
    _, suffix = rest.split(END)
    expected = prefix + START + "\n\n```mermaid\n" + graph + "```\n\n" + END + suffix
    if args.check:
        if document != expected:
            raise SystemExit(f"stale chart: run python3 {Path(__file__).relative_to(ROOT)} {'--catalog ' if args.catalog else ''}--write")
        print(f"{'catalog' if args.catalog else 'entry'} chart matches executable definition")
    else:
        document_path.write_text(expected)


if __name__ == "__main__":
    main()
