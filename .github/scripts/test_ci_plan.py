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

    def test_fmu_runtime_is_used_by_the_facade(self):
        # S095: globe-mode-fmu に依存元は無かった。S099 から exec と frontend に依存するので、exec の変更で回る。
        # S238: `globe fmu export --runtime p5b` が modelDescription.xml を作るのに使うので、依存元は globe-mode-cli だけ
        # S239: その本体が facade の optional な依存（feature fmu-p5b）に移ったので、facade とその依存元（lsp・wasm・py・c・nif・cli）も回る
        self.assertEqual(
            pkgs(plan(["src/Rust/mod/globe-mode-fmu/src/lib.rs"])),
            {"globe-mode-fmu", "globe-mode-fmu-cdylib", "globe-mode", "globe-mode-lsp", "globe-mode-wasm", "globe-mode-py", "globe-mode-c", "globe-mode-nif", "globe-mode-cli"},
        )
        self.assertIn("globe-mode-fmu", pkgs(plan(["src/Rust/mod/globe-mode-exec/src/lib.rs"])))
        # S117: fmu は fmi（既定の feature だけ）の書く側を使うので、fmi の変更で回る（S118 から facade も fmi に依存するので、その依存元も）
        self.assertIn("globe-mode-fmu", pkgs(plan(["src/Rust/mod/globe-mode-fmi/src/lib.rs"])))
        # S204: globe-mode-fmi-import は globe-mode-fmi に依存し、globe-mode-cli から使われる
        self.assertIn("globe-mode-fmi-import", pkgs(plan(["src/Rust/mod/globe-mode-fmi/src/lib.rs"])))
        self.assertIn("globe-mode-cli", pkgs(plan(["src/Rust/mod/globe-mode-fmi-import/src/lib.rs"])))

    def test_change_propagates_to_dependents(self):
        p = plan(["src/Rust/mod/globe-mode-ir/src/lib.rs"])
        self.assertEqual(
            pkgs(p),
            {"globe-mode-ir", "globe-mode-frontend", "globe-mode-fmi", "globe-mode-fmi-import", "globe-mode-graph", "globe-mode-analysis", "globe-mode-db",
             "globe-mode-cli", "globe-mode-lsp", "globe-mode-py", "globe-mode-c", "globe-mode-nif", "globe-mode-exec", "globe-mode-fmu", "globe-mode-fmu-cdylib", "globe-mode-sim", "globe-mode-protocol", "globe-mode", "globe-mode-wasm",
             "xtask"},
        )
        self.assertTrue(p["wasm"])
        self.assertTrue(p["wasm_node"])

    def test_core_change_tests_everything_but_is_not_full(self):
        p = plan(["src/Rust/mod/globe-mode-core/src/lib.rs"])
        self.assertFalse(p["full"])
        self.assertEqual(
            pkgs(p),
            {"globe-mode-core", "globe-mode-ir", "globe-mode-frontend", "globe-mode-fmi", "globe-mode-fmi-import", "globe-mode-graph", "globe-mode-analysis",
             "globe-mode-db", "globe-mode-cli", "globe-mode-lsp", "globe-mode-py", "globe-mode-c", "globe-mode-nif", "globe-mode-exec", "globe-mode-fmu", "globe-mode-fmu-cdylib", "globe-mode-sim", "globe-mode-protocol", "globe-mode",
             "globe-mode-wasm", "xtask"},
        )
        self.assertFalse(p["msl"])

    def test_graph_change_reaches_db_and_cli(self):
        p = plan(["src/Rust/mod/globe-mode-graph/src/lib.rs"])
        # S005 から xtask（msl-funnel）も db を使う。S035 から cli と wasm は facade globe-mode 経由
        self.assertEqual(
            pkgs(p), {"globe-mode-graph", "globe-mode-db", "globe-mode", "globe-mode-cli", "globe-mode-lsp", "globe-mode-py", "globe-mode-c", "globe-mode-nif", "globe-mode-wasm", "xtask"}
        )
        self.assertTrue(p["wasm"])

    def test_analysis_change_reaches_db_and_cli(self):
        p = plan(["src/Rust/mod/globe-mode-analysis/src/lib.rs"])
        self.assertEqual(
            pkgs(p),
            {"globe-mode-analysis", "globe-mode-db", "globe-mode-protocol", "globe-mode", "globe-mode-cli", "globe-mode-lsp", "globe-mode-py", "globe-mode-c", "globe-mode-nif", "globe-mode-wasm",
             "xtask"},
        )
        self.assertTrue(p["wasm"])

    def test_known_unsupported_list_affects_frontend(self):
        p = plan(["src/Modelica/msl-known-unsupported.txt"])
        # S011 から exec も frontend に依存する。S094 から fmi（feature `export`）も。S012 から wasm の束ねも。S035 から facade も
        self.assertEqual(
            pkgs(p),
            {"globe-mode-frontend", "globe-mode-fmi", "globe-mode-fmi-import", "globe-mode-cli", "globe-mode-lsp", "globe-mode-py", "globe-mode-c", "globe-mode-nif", "globe-mode-db", "globe-mode-exec", "globe-mode-fmu", "globe-mode-fmu-cdylib", "globe-mode", "globe-mode-wasm",
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

    def test_ui_runs_for_svelte_and_wasm(self):
        # S041: ブラウザ UI は src/Svelte の変更か wasm の束ねの影響で回す。無関係な crate では回さない
        for path in ["src/Svelte/src/App.svelte", "src/Svelte/pnpm-lock.yaml", "src/Rust/mod/globe-mode-wasm/web/client.js",
                     "src/Rust/mod/globe-mode-exec/src/lib.rs"]:
            with self.subTest(path=path):
                p = plan([path])
                self.assertFalse(p["full"])
                self.assertTrue(p["ui"])
        self.assertFalse(plan(["src/Rust/mod/globe-mode-cli/src/main.rs"])["ui"])
        self.assertTrue(plan(["Cargo.lock"])["ui"])

    def test_wasm_crate_only_tests_itself(self):
        p = plan(["src/Rust/mod/globe-mode-wasm/src/lib.rs"])
        self.assertEqual(pkgs(p), {"globe-mode-wasm"})

    def test_facade_and_protocol_reach_cli_and_wasm(self):
        # S035: facade を変えたら cli と wasm、protocol を変えたら facade も
        p = plan(["src/Rust/mod/globe-mode/src/lib.rs"])
        self.assertEqual(pkgs(p), {"globe-mode", "globe-mode-cli", "globe-mode-lsp", "globe-mode-py", "globe-mode-c", "globe-mode-nif", "globe-mode-wasm"})
        self.assertTrue(p["wasm"])
        self.assertTrue(p["wasm_node"])
        p = plan(["src/Rust/mod/globe-mode-protocol/src/lib.rs"])
        self.assertEqual(pkgs(p), {"globe-mode-protocol", "globe-mode", "globe-mode-cli", "globe-mode-lsp", "globe-mode-py", "globe-mode-c", "globe-mode-nif", "globe-mode-wasm"})

    def test_python_runs_when_the_py_crate_is_affected(self):
        # S039: wheel は globe-mode-py をビルドするので、facade の変更でも Python のジョブを回す。wasm の束ねだけなら回さない
        self.assertTrue(plan(["src/Rust/mod/globe-mode/src/lib.rs"])["python"])
        self.assertFalse(plan(["src/Rust/mod/globe-mode-wasm/src/lib.rs"])["python"])

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
            {"globe-mode-frontend", "globe-mode-fmi", "globe-mode-fmi-import", "globe-mode-cli", "globe-mode-lsp", "globe-mode-py", "globe-mode-c", "globe-mode-nif", "globe-mode-db", "globe-mode-exec", "globe-mode-fmu", "globe-mode-fmu-cdylib", "globe-mode", "globe-mode-wasm",
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

    def test_lang_runs_for_c_abi_zig_julia(self):
        # S077: C ABI の crate（とその依存）か Zig・Julia の例を変えたときと、全体のときだけ
        for path in ["src/Rust/mod/globe-mode-c/src/lib.rs", "src/Rust/mod/globe-mode-core/src/lib.rs",
                     "src/Zig/build.zig", "src/Julia/test.jl", "mise.toml",
                     # S082: Elixir の NIF の crate と束ね、Gleam
                     "src/Rust/mod/globe-mode-nif/src/lib.rs", "src/Elixir/globe_mode/mix.exs",
                     "src/Gleam/README.md"]:
            with self.subTest(path=path):
                self.assertTrue(plan([path])["lang"])
        self.assertFalse(plan(["src/Rust/mod/globe-mode-wasm/src/lib.rs"])["lang"])
        # S111: facade が globe-mode-fmt に依存するので、facade と fmt の変更で C・NIF・py と lang・python を回す
        # S118: facade の feature `fmu` が globe-mode-fmi に依存するので、fmi の変更でも。S144 から lsp（globe/exportFmu）と wasm も
        for path in ["src/Rust/mod/globe-mode/src/lib.rs", "src/Rust/mod/globe-mode-fmt/src/lib.rs",
                     "src/Rust/mod/globe-mode-fmi/src/export.rs"]:
            with self.subTest(path=path):
                p = plan([path])
                self.assertTrue(p["lang"])
                self.assertTrue(p["python"])
                self.assertLessEqual({"globe-mode-c", "globe-mode-nif", "globe-mode-py", "globe-mode-lsp", "globe-mode-wasm"}, pkgs(p))
        self.assertFalse(plan(["doc/roadmap.md"])["lang"])

    def test_cli_apps(self):
        # #77: CLI を子プロセスで呼ぶアプリ（FastAPI・Streamlit・marimo、Phoenix・Wisp、Astro）は、CLI の変更でも回す
        for path in ["src/Rust/mod/globe-mode-cli/src/main.rs", "src/Rust/mod/globe-mode-fmt/src/lint.rs"]:
            with self.subTest(path=path):
                p = plan([path])
                self.assertTrue(p["python"] and p["lang"] and p["astro"])
        self.assertTrue(plan(["src/Astro/fmt/src/lib/globe.ts"])["astro"])
        self.assertFalse(plan(["src/Astro/fmt/src/lib/globe.ts"])["python"])
        self.assertFalse(plan(["src/Rust/mod/globe-mode-wasm/src/lib.rs"])["astro"])

    def test_web_apps(self):
        # S215（#22）: Rust のウェブアプリは別の workspace なので、自分のディレクトリか globe-mode-fmt を変えたときに回す
        for path in ["src/Rust/apps/web-axum/src/lib.rs", "src/Rust/apps/web-loco/Cargo.lock",
                     "src/Rust/mod/globe-mode-fmt/src/lib.rs"]:
            with self.subTest(path=path):
                self.assertTrue(plan([path])["web"])
        self.assertFalse(plan(["src/Rust/mod/globe-mode-sim/src/lib.rs"])["web"])
        self.assertFalse(plan(["src/Python/fastapi_fmt/app.py"])["web"])
        self.assertTrue(plan([], force_full=True)["web"])

    def test_ui_exp(self):
        # S233: 試験版 UI（別の workspace）は、自分のディレクトリか、リンクする facade（とその依存）を変えたときに回す
        for path in ["src/Rust/apps/experimental/leptos/src/lib.rs", "src/Rust/apps/experimental/dioxus/Cargo.lock",
                     "src/Rust/mod/globe-mode/src/lib.rs", "src/Rust/mod/globe-mode-core/src/lib.rs"]:
            with self.subTest(path=path):
                self.assertTrue(plan([path])["ui_exp"])
        self.assertFalse(plan(["src/Rust/mod/globe-mode-cli/src/main.rs"])["ui_exp"])
        self.assertFalse(plan(["src/Rust/apps/web-axum/src/lib.rs"])["ui_exp"])
        self.assertTrue(plan([], force_full=True)["ui_exp"])

    def test_full_triggers(self):
        for path in ["Cargo.lock", "Cargo.toml", "mise.toml", "mise.lock", "deny.toml", ".github/workflows/ci.yml",
                     # #7: ビルドの設定（wasm32 の rustflags・xtask の alias）と、checkout の改行の規則（S004 の Windows）
                     ".cargo/config.toml", ".gitattributes"]:
            with self.subTest(path=path):
                self.assertTrue(plan([path])["full"])
        # 別の workspace（web-*）の .cargo は ci.yml の対象でない
        self.assertFalse(plan(["src/Rust/apps/web-axum/.cargo/config.toml"])["full"])

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
        self.assertFalse(any(p[k] for k in ["full", "rust", "wasm", "wasm_node", "ui", "deny", "python", "lang", "web", "astro", "ui_exp", "docs", "slides", "msl"]))


if __name__ == "__main__":
    unittest.main(verbosity=1)
