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


if __name__ == "__main__":
    unittest.main()
