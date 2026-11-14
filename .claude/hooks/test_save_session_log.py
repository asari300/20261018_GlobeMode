#!/usr/bin/env python3
"""save_session_log.py の伏せ字の規則の試験（S218）。トークンの種類ごとに redact と leftovers を確かめる。"""

import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from save_session_log import leftovers, redact  # noqa: E402

# 本物ではない並び（種類ごとの接頭辞と長さだけ合わせる）
TOKENS = [f"{p}_{'a1B2' * 9}" for p in ("ghp", "gho", "ghu", "ghs", "ghr")]


class Redact(unittest.TestCase):
    def test_github_tokens_of_every_kind(self):
        for t in TOKENS:
            text = f"token is {t} ok"
            self.assertNotIn(t, redact(text), t)
            self.assertTrue(leftovers(text), t)           # 伏せていない文は --check が拾う
            self.assertFalse(leftovers(redact(text)), t)

    def test_env_style_names_and_bearer(self):
        # 名前の前に `_` が付く形（\b では当たらなかった。#90）
        for text in ("ANDROID_KEYSTORE_PASSWORD=hunter2hunter2", "export GITHUB_TOKEN=abcd1234efgh",
                     'MY_SECRET="abcd1234efgh"', "TAURI_SIGNING_PRIVATE_KEY=abcd1234efgh",
                     "Authorization: Bearer abcd1234efgh"):
            self.assertNotIn("1234", redact(text), text)
            self.assertTrue(leftovers(text), text)
            self.assertFalse(leftovers(redact(text)), text)

    def test_max_tokens_is_kept(self):
        self.assertEqual(redact("max_tokens=12345678"), "max_tokens=12345678")


if __name__ == "__main__":
    unittest.main()
