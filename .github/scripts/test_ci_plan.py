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
        self.assertFalse(p["wasm_node"])
        self.assertFalse(p["deny"])

    def test_change_propagates_to_dependents(self):
        p = plan(["src/Rust/mod/globe-mode-ir/src/lib.rs"])
        self.assertEqual(
            pkgs(p),
            {"globe-mode-ir", "globe-mode-frontend", "globe-mode-graph", "globe-mode-analysis", "globe-mode-db",
             "globe-mode-cli", "globe-mode-lsp", "globe-mode-exec", "globe-mode-sim", "globe-mode-protocol", "globe-mode", "globe-mode-wasm",
             "xtask"},
        )
        self.assertTrue(p["wasm"])
        self.assertTrue(p["wasm_node"])

    def test_core_change_tests_everything_but_is_not_full(self):
        p = plan(["src/Rust/mod/globe-mode-core/src/lib.rs"])
        self.assertFalse(p["full"])
        self.assertEqual(
            pkgs(p),
            {"globe-mode-core", "globe-mode-ir", "globe-mode-frontend", "globe-mode-graph", "globe-mode-analysis",
             "globe-mode-db", "globe-mode-cli", "globe-mode-lsp", "globe-mode-exec", "globe-mode-sim", "globe-mode-protocol", "globe-mode",
             "globe-mode-wasm", "xtask"},
        )
        self.assertFalse(p["msl"])

    def test_graph_change_reaches_db_and_cli(self):
        p = plan(["src/Rust/mod/globe-mode-graph/src/lib.rs"])
        # S005 から xtask（msl-funnel）も db を使う。S035 から cli と wasm は facade globe-mode 経由
        self.assertEqual(
            pkgs(p), {"globe-mode-graph", "globe-mode-db", "globe-mode", "globe-mode-cli", "globe-mode-lsp", "globe-mode-wasm", "xtask"}
        )
        self.assertTrue(p["wasm"])

    def test_analysis_change_reaches_db_and_cli(self):
        p = plan(["src/Rust/mod/globe-mode-analysis/src/lib.rs"])
        self.assertEqual(
            pkgs(p),
            {"globe-mode-analysis", "globe-mode-db", "globe-mode-protocol", "globe-mode", "globe-mode-cli", "globe-mode-lsp", "globe-mode-wasm",
             "xtask"},
        )
        self.assertTrue(p["wasm"])

    def test_known_unsupported_list_affects_frontend(self):
        p = plan(["src/Modelica/msl-known-unsupported.txt"])
        # S011 から exec も frontend に依存する。S012 から wasm の束ねも。S035 から facade も
        self.assertEqual(
            pkgs(p),
            {"globe-mode-frontend", "globe-mode-cli", "globe-mode-lsp", "globe-mode-db", "globe-mode-exec", "globe-mode", "globe-mode-wasm",
             "xtask"},
        )

    def test_wasm_node_runs_for_exec_sim_and_wasm(self):
        # S012: 実行系・sim・db・frontend・wasm の束ねを変えたら、release の wasm を Node で回す
        for path in ["src/Rust/mod/globe-mode-exec/src/lib.rs", "src/Rust/mod/globe-mode-sim/src/lib.rs",
                     "src/Rust/mod/globe-mode-db/src/lib.rs", "src/Rust/mod/globe-mode-frontend/src/lib.rs",
                     "src/Rust/mod/globe-mode-wasm/src/lib.rs", "src/Rust/mod/globe-mode-wasm/node/simulate.mjs"]:
            with self.subTest(path=path):
                p = plan([path])
                self.assertFalse(p["full"])
                self.assertTrue(p["wasm"])
                self.assertTrue(p["wasm_node"])
                self.assertIn("globe-mode-wasm", pkgs(p))

    def test_wasm_crate_only_tests_itself(self):
        p = plan(["src/Rust/mod/globe-mode-wasm/src/lib.rs"])
        self.assertEqual(pkgs(p), {"globe-mode-wasm"})

    def test_facade_and_protocol_reach_cli_and_wasm(self):
        # S035: facade を変えたら cli と wasm、protocol を変えたら facade も
        p = plan(["src/Rust/mod/globe-mode/src/lib.rs"])
        self.assertEqual(pkgs(p), {"globe-mode", "globe-mode-cli", "globe-mode-lsp", "globe-mode-wasm"})
        self.assertTrue(p["wasm"])
        self.assertTrue(p["wasm_node"])
        p = plan(["src/Rust/mod/globe-mode-protocol/src/lib.rs"])
        self.assertEqual(pkgs(p), {"globe-mode-protocol", "globe-mode", "globe-mode-cli", "globe-mode-lsp", "globe-mode-wasm"})

    def test_xtask_only(self):
        p = plan(["src/Rust/xtask/src/main.rs"])
        self.assertEqual(pkgs(p), {"xtask"})
        self.assertFalse(p["wasm"])
        self.assertFalse(p["wasm_node"])

    def test_msl_model_list_affects_xtask_only(self):
        self.assertEqual(pkgs(plan(["src/Modelica/msl-compare.txt"])), {"xtask"})

    def test_full_runs_msl_compare(self):
        self.assertTrue(plan(["Cargo.lock"])["msl"])
        self.assertTrue(plan(["Cargo.lock"])["wasm_node"])

    def test_modelica_samples_affect_frontend_tests(self):
        p = plan(["src/Modelica/GlobeModeExamples/package.mo"])
        # db・exec・wasm のテストも src/Modelica を読む。xtask は frontend に依存するので、依存元として入る
        self.assertEqual(
            pkgs(p),
            {"globe-mode-frontend", "globe-mode-cli", "globe-mode-lsp", "globe-mode-db", "globe-mode-exec", "globe-mode", "globe-mode-wasm",
             "xtask"},
        )
        self.assertTrue(p["wasm_node"])

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
        self.assertFalse(any(p[k] for k in ["full", "rust", "wasm", "wasm_node", "deny", "python", "docs", "slides", "msl"]))


if __name__ == "__main__":
    unittest.main(verbosity=1)
