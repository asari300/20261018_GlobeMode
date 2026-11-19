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

    def test_json_quoted_key_pem_and_authorization_token(self):
        # S383: キーの直後に引用符が来る JSON、PEM の秘密鍵（END が切れたものも）、Authorization: token
        pem = "-----BEGIN RSA PRIVATE KEY-----\nMIIEowIBAAKCAQEAabcdefghij\nklmnopqrstuvwxyz0123456789\n-----END RSA PRIVATE KEY-----"
        cut = "-----BEGIN PRIVATE KEY-----\nMIIEvQIBADANBgkqhkiG9w0BAQEFAASC\nKgwggSkAgEAAoIBAQC7VJTUt9Us8cKj\n… （残り 9 字は生ログを参照）"
        for text, secret in (('{"password": "hunter2hunter2"}', "hunter2"), ('{"password":"hunter2hunter2"}', "hunter2"),
                             (f"key:\n{pem}\nend", "MIIE"), (f"key:\n{cut}", "MIIE"),
                             ("Authorization: token gho_short", "gho_short"), ('"Authorization": "Basic dXNlcjpwYXNz"', "dXNlcjpwYXNz")):
            self.assertNotIn(secret, redact(text), text)
            self.assertTrue(leftovers(text), text)
            self.assertFalse(leftovers(redact(text)), text)
        self.assertIn("end", redact(f"key:\n{pem}\nend"))      # PEM の外は残す
        self.assertIn("残り", redact(f"key:\n{cut}"))

    def test_numeric_value_after_quoted_key_is_kept(self):
        text = "{'time_to_first_token': 25.680112}"        # 数値は秘密ではない（log/sessions にある形）
        self.assertEqual(redact(text), text)
        self.assertFalse(leftovers(text))

    def test_max_tokens_is_kept(self):
        self.assertEqual(redact("max_tokens=12345678"), "max_tokens=12345678")


if __name__ == "__main__":
    unittest.main()
