#!/usr/bin/env python3
"""lmstudio_review.py の単体テスト。HTTP はモックし、LM Studio が無くても通る。

  python3 -I -m unittest discover -s .claude/skills/local-review -p 'test_*.py'
"""

from __future__ import annotations

import argparse
import io
import json
import sys
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))

import lmstudio_review as lr  # noqa: E402

DIFF = """diff --git a/src/a.rs b/src/a.rs
index 1111111..2222222 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -10,3 +10,4 @@ fn main() {
     let x = 1;
-    let y = 2;
+    let y = x.checked_add(2).unwrap();
+    println!("{y}");
 }
diff --git a/Cargo.lock b/Cargo.lock
index 3333333..4444444 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -1,1 +1,1 @@
-old
+new
diff --git a/img.png b/img.png
index 5555555..6666666 100644
Binary files a/img.png and b/img.png differ
"""


def args(**kw: object) -> argparse.Namespace:
    base = lr.parse_args([])
    for k, v in kw.items():
        setattr(base, k, v)
    return base


class SplitAndFilter(unittest.TestCase):
    def test_split_files(self) -> None:
        files = lr.split_files(DIFF)
        self.assertEqual([f.path for f in files], ["src/a.rs", "Cargo.lock", "img.png"])
        self.assertTrue(files[2].binary)
        self.assertFalse(files[0].binary)

    def test_excluded(self) -> None:
        for p in ["Cargo.lock", "uv.lock", "data/msl/x.mo", "log/local/a.md", ".env", ".env.local",
                  "web/node_modules/x.js", "keys/server.pem", "doc/a.png"]:
            self.assertTrue(lr.excluded(p), p)
        for p in ["src/a.rs", "doc/sessions/S006/slides.html", ".claude/skills/x/y.py"]:
            self.assertFalse(lr.excluded(p), p)


class Annotate(unittest.TestCase):
    def test_new_file_line_numbers(self) -> None:
        text = lr.annotate(lr.split_files(DIFF)[0].text)
        self.assertIn("   10       let x = 1;", text)
        self.assertIn("      -     let y = 2;", text)
        self.assertIn("   11 +     let y = x.checked_add(2).unwrap();", text)
        self.assertIn("   12 +     println!(\"{y}\");", text)
        self.assertIn("   13   }", text)
        self.assertIn("+++ b/src/a.rs", text)          # 見出しはそのまま


class Budget(unittest.TestCase):
    def test_estimate_tokens(self) -> None:
        self.assertEqual(lr.estimate_tokens("abcdef"), 2)
        self.assertEqual(lr.estimate_tokens("日本語"), 3)
        self.assertEqual(lr.estimate_tokens("ab日"), 2)

    def test_chunks_respect_budget(self) -> None:
        files = [lr.FileDiff(f"f{i}.py", DIFF.split("diff --git a/Cargo.lock")[0].replace("src/a.rs", f"f{i}.py"))
                 for i in range(6)]
        one = lr.estimate_tokens(lr.annotate(files[0].text))
        chunks = lr.make_chunks(files, one * 2 + 1)
        self.assertEqual(len(chunks), 3)
        self.assertTrue(all(c.tokens <= one * 2 + 1 for c in chunks))
        self.assertEqual(sum(len(c.files) for c in chunks), 6)

    def test_large_hunk_continues_with_line_numbers(self) -> None:
        # 新しいファイルは全体が 1 つの @@ になる。切り詰めずに、行番号を引き継いで分ける。
        hunk = "@@ -0,0 +1,80 @@\n" + "".join(f"+line {i} " + "x" * 60 + "\n" for i in range(1, 81))
        fd = lr.FileDiff("big.py", "diff --git a/big.py b/big.py\n--- /dev/null\n+++ b/big.py\n" + hunk)
        pieces = lr.pieces_of(fd, 400)
        self.assertGreaterEqual(len(pieces), 2)
        self.assertFalse(any(t for _, t in pieces))
        self.assertTrue(all(lr.estimate_tokens(p) <= 400 for p, _ in pieces))
        joined = "".join(p for p, _ in pieces)
        for i in (1, 40, 80):
            self.assertIn(f"{i:>5} + line {i} ", joined)
        self.assertIn("(continued)", pieces[1][0])
        self.assertIn("+++ b/big.py", pieces[1][0])     # 続きにもファイル名を付ける

    def test_single_huge_line_is_truncated(self) -> None:
        hunk = "@@ -0,0 +1,1 @@\n+" + "y" * 5000 + "\n"
        fd = lr.FileDiff("one.txt", "diff --git a/one.txt b/one.txt\n--- /dev/null\n+++ b/one.txt\n" + hunk)
        pieces = lr.pieces_of(fd, 400)
        self.assertEqual(len(pieces), 1)
        self.assertTrue(pieces[0][1])
        self.assertLessEqual(lr.estimate_tokens(pieces[0][0]), 400)


class Parse(unittest.TestCase):
    def test_findings(self) -> None:
        out = """src/a.rs:11 — high — unwrap on checked_add can panic — return an error instead
- `src/a.rs:12` - low - print in library code - use log
3. src/b.py:5 – **medium** – bare except – catch specific errors
Some free text the model added
NO_FINDINGS"""
        found, other = lr.parse_findings(out)
        self.assertEqual([(f.path, f.line, f.severity) for f in found],
                         [("src/a.rs", 11, "high"), ("src/a.rs", 12, "low"), ("src/b.py", 5, "medium")])
        self.assertEqual(other, ["Some free text the model added"])

    def test_no_findings(self) -> None:
        self.assertEqual(lr.parse_findings("NO_FINDINGS\n"), ([], []))


class Http(unittest.TestCase):
    def cfg(self) -> lr.Config:
        return lr.Config("http://localhost:1234", "prism-ml/bonsai-27b", "tok", 5)

    def test_review_chunk_payload(self) -> None:
        sent: dict = {}

        def fake(cfg: lr.Config, path: str, body: dict | None = None, timeout: float | None = None) -> dict:
            sent.update(path=path, body=body)
            return {"choices": [{"message": {"content": "NO_FINDINGS"}, "finish_reason": "stop"}],
                    "usage": {"prompt_tokens": 10, "completion_tokens": 2},
                    "stats": {"time_to_first_token": 1.0, "tokens_per_second": 2.0}}

        with mock.patch.object(lr, "api", side_effect=fake):
            r = lr.review_chunk(self.cfg(), lr.Chunk(files=["a.rs"], text="x"), 1, 1, args())
        self.assertEqual(sent["path"], "/api/v0/chat/completions")
        self.assertEqual(sent["body"]["reasoning_effort"], "none")
        self.assertEqual(sent["body"]["max_tokens"], 768)
        self.assertEqual(r["content"], "NO_FINDINGS")

        with mock.patch.object(lr, "api", side_effect=fake):
            lr.review_chunk(self.cfg(), lr.Chunk(files=["a.rs"], text="x"), 1, 1, args(think=True))
        self.assertNotIn("reasoning_effort", sent["body"])

    def test_bearer_header(self) -> None:
        captured = {}

        class Res(io.BytesIO):
            def __enter__(self) -> "Res":
                return self

            def __exit__(self, *a: object) -> None:
                return None

        def fake_urlopen(req, timeout):  # type: ignore[no-untyped-def]
            captured["auth"] = req.get_header("Authorization")
            return Res(json.dumps({"data": []}).encode())

        with mock.patch.object(lr.urllib.request, "urlopen", side_effect=fake_urlopen):
            self.assertIsNone(lr.model_info(self.cfg()))
        self.assertEqual(captured["auth"], "Bearer tok")

    def test_not_loaded_exits_2(self) -> None:
        info = {"id": "prism-ml/bonsai-27b", "state": "not-loaded"}
        with mock.patch.object(lr, "model_info", return_value=info), \
                mock.patch.object(lr, "collect_diff", return_value=DIFF), \
                mock.patch("sys.stderr", new_callable=io.StringIO):
            self.assertEqual(lr.main([]), 2)

    def test_dry_run_without_server(self) -> None:
        with mock.patch.object(lr, "model_info", side_effect=lr.ServerError("down")), \
                mock.patch.object(lr, "collect_diff", return_value=DIFF), \
                mock.patch("sys.stdout", new_callable=io.StringIO) as out:
            self.assertEqual(lr.main(["--dry-run"]), 0)
        text = out.getvalue()
        self.assertIn("塊 1", text)
        self.assertIn("`Cargo.lock`", text)              # 除外したことを書く
        self.assertIn("`img.png`", text)


if __name__ == "__main__":
    unittest.main()
