#!/usr/bin/env python3
"""Focused tests for the chart projection (not native algorithm semantics)."""
import json
import unittest

from graph import DEFINITION, render


class ChartProjection(unittest.TestCase):
    def test_actual_graph_contains_directives_hierarchy_and_ordered_guards(self):
        graph = render(json.loads(DEFINITION.read_text()))
        self.assertIn("active.prepare_view.view_policy", graph)
        self.assertIn("emit rank_sessions", graph)
        self.assertIn("active -.->|initial| bind_registry", graph)
        self.assertIn("active -->|stop| stopped", graph)
        self.assertLess(graph.index("view_policy, 1: independent"), graph.index("view_policy, 2: otherwise"))

    def test_reordered_guard_array_changes_projection(self):
        definition = json.loads(DEFINITION.read_text())
        events = definition["states"]["active"]["states"]["prepare_view"]["states"]["view_policy"]["on"]
        events["view_policy"].reverse()
        graph = render(definition)
        self.assertIn("view_policy, 1: otherwise", graph)
        self.assertIn("view_policy, 2: independent", graph)

    def test_self_reentry_remains_visible(self):
        graph = render({
            "id": "test", "initial": "read", "states": {
                "read": {"id": "read", "on": {"observed": {"target": "read", "reenter": True}}}
            }
        })
        self.assertIn("read -->|observed, reenter| read", graph)

    def test_unknown_actions_and_missing_targets_fail_loudly(self):
        for state in [
            {"id": "test", "entry": [{"type": "surprise"}]},
            {"id": "test", "on": {"go": {"target": "#missing"}}},
        ]:
            with self.assertRaises(ValueError):
                render(state)


if __name__ == "__main__":
    unittest.main()
