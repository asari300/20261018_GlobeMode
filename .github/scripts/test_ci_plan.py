#!/usr/bin/env python3
"""ci_plan.py の判定の単体テスト（CI の plan ジョブで最初に実行する）。"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ci_plan import LARGE_CHANGE, plan  # noqa: E402


def pkgs(p):
    return set(p["rust_packages"].replace("-p ", "").split())


class Plan(unittest.TestCase):
    def test_leaf_crate_only_tests_itself(self):
        p = plan(["src/Rust/mod/globe-mode-cli/src/main.rs"])
        self.assertFalse(p["full"])
        self.assertEqual(pkgs(p), {"globe-mode-cli"})
        self.assertFalse(p["wasm"])
        self.assertFalse(p["deny"])

    def test_change_propagates_to_dependents(self):
        p = plan(["src/Rust/mod/globe-mode-ir/src/lib.rs"])
        self.assertEqual(
            pkgs(p),
            {"globe-mode-ir", "globe-mode-frontend", "globe-mode-graph", "globe-mode-db", "globe-mode-cli", "xtask"},
        )
        self.assertTrue(p["wasm"])

    def test_core_change_tests_everything_but_is_not_full(self):
        p = plan(["src/Rust/mod/globe-mode-core/src/lib.rs"])
        self.assertFalse(p["full"])
        self.assertEqual(
            pkgs(p),
            {"globe-mode-core", "globe-mode-ir", "globe-mode-frontend", "globe-mode-graph", "globe-mode-db",
             "globe-mode-cli", "xtask"},
        )
        self.assertFalse(p["msl"])

    def test_graph_change_reaches_db_and_cli(self):
        p = plan(["src/Rust/mod/globe-mode-graph/src/lib.rs"])
        self.assertEqual(pkgs(p), {"globe-mode-graph", "globe-mode-db", "globe-mode-cli"})
        self.assertTrue(p["wasm"])

    def test_xtask_only(self):
        p = plan(["src/Rust/xtask/src/main.rs"])
        self.assertEqual(pkgs(p), {"xtask"})
        self.assertFalse(p["wasm"])

    def test_msl_model_list_affects_xtask_only(self):
        self.assertEqual(pkgs(plan(["src/Modelica/msl-compare.txt"])), {"xtask"})

    def test_full_runs_msl_compare(self):
        self.assertTrue(plan(["Cargo.lock"])["msl"])

    def test_modelica_samples_affect_frontend_tests(self):
        p = plan(["src/Modelica/GlobeModeExamples/package.mo"])
        # db のテストも src/Modelica を読む。xtask は frontend に依存するので、依存元として入る
        self.assertEqual(pkgs(p), {"globe-mode-frontend", "globe-mode-cli", "globe-mode-db", "xtask"})

    def test_crate_manifest_needs_deny_but_not_full(self):
        p = plan(["src/Rust/mod/globe-mode-cli/Cargo.toml"])
        self.assertFalse(p["full"])
        self.assertTrue(p["deny"])

    def test_docs_only_skips_rust(self):
        p = plan(["doc/roadmap.md"])
        self.assertFalse(p["rust"])
        self.assertTrue(p["docs"])
        self.assertFalse(p["slides"])

    def test_slides_trigger_browser_check(self):
        self.assertTrue(plan(["doc/sessions/S002-workspace-skeleton/slides.html"])["slides"])

    def test_python(self):
        p = plan(["uv.lock"])
        self.assertTrue(p["python"])
        self.assertFalse(p["rust"])

    def test_full_triggers(self):
        for path in ["Cargo.lock", "Cargo.toml", "mise.toml", "mise.lock", "deny.toml", ".github/workflows/ci.yml"]:
            with self.subTest(path=path):
                self.assertTrue(plan([path])["full"])

    def test_large_change_is_full(self):
        many = [f"doc/x{i}.md" for i in range(LARGE_CHANGE + 1)]
        self.assertTrue(plan(many)["full"])
        self.assertFalse(plan(many[:LARGE_CHANGE])["full"])

    def test_full_ci_marker_and_missing_base(self):
        self.assertTrue(plan(["doc/a.md"], message="fix\n\n[full-ci]")["full"])
        self.assertTrue(plan(None)["full"])
        self.assertTrue(plan([], force_full=True, force_reason="schedule")["full"])

    def test_nothing_relevant(self):
        p = plan(["data/.gitkeep"])
        self.assertFalse(any(p[k] for k in ["full", "rust", "deny", "python", "docs", "slides", "msl"]))


if __name__ == "__main__":
    unittest.main(verbosity=1)
