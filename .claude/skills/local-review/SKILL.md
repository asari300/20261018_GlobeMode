---
name: local-review
description: GlobeMode のレビューのとき、副次的に Windows 側の LM Studio のローカルモデル（既定は prism-ml/bonsai-27b）に git の差分を読ませ、セカンドオピニオンを得る。指摘は助言にとどめ、Claude がコードで確かめて採否を決める。「ローカル LLM でもレビュー」「Bonsai にも見せて」「セカンドオピニオン」と言われたとき、または大きめの変更をコミットする前のレビューで使う。遅い（1 塊 10〜20 分）ので、ユーザーが頼んだときか、時間に余裕があるときだけ使う。
allowed-tools: Bash(python3 -I .claude/skills/local-review/lmstudio_review.py:*)
---

# ローカル LLM の副レビュー

実装とレビューの主は Claude。このスキルは、WSL2 から Windows の LM Studio に差分を送り、小型モデルの指摘を**参考として**集める。背景と設定の手順は [S005 の解説書](../../../doc/sessions/S005-local-llm-review/解説書.md)、判断は [ADR-0007](../../../doc/adr/0007-local-llm-reviewer.md) にある。

## 前提

- LM Studio のサーバが Windows 側で動いている。WSL2 が mirrored モードなら `http://localhost:1234` で届く。
- モデルが**先に読み込まれている**。スクリプトは JIT 読み込みに任せない。既定のコンテキスト長（最大の 262K のことがある）で読み込まれると、メモリが足りなくなるため。
- 設定は環境変数で変えられる: `LMSTUDIO_BASE_URL`、`LMSTUDIO_MODEL`、`LMSTUDIO_API_TOKEN`（LM Studio で認証を有効にしたとき）。

## 手順

1. **状態を確かめる。**

   ```bash
   python3 -I .claude/skills/local-review/lmstudio_review.py --check
   ```

   - 終了コード 2 は、届かないか読み込まれていないことを表す。そのときはユーザーに伝え、次のどちらかで読み込んでもらう（自分で読み込むのは、ユーザーが頼んだときだけ）。
     - Windows 側: `lms load prism-ml/bonsai-27b -c 12288 --gpu off --parallel 1 --ttl 1800`
     - WSL 側: `python3 -I .claude/skills/local-review/lmstudio_review.py --load 12288`

2. **範囲と所要時間を見る。**

   ```bash
   python3 -I .claude/skills/local-review/lmstudio_review.py --dry-run
   ```

   - 既定の範囲は `main` との merge-base から作業ツリーまで（未追跡のファイルを含む）。範囲は `--range A..B`、`--staged`、`--path 'glob'` で変える。
   - ロックファイル、`data/`、`log/`、`.env*`、鍵らしいファイル、バイナリは送らない。
   - 見積もりが長いとき（目安は 20 分超）は、`--path` で変更の核心に絞る。
   - LM Studio のモデル一覧は、応答に数十秒かかることがある。直前の `--check` で読み込みを確かめたなら、`--ctx 12288` を付けて確認を省いてよい。

3. **レビューを走らせる。** 時間がかかるので、Bash を `run_in_background` で起動し、その間は自分のレビューを進める。

   ```bash
   python3 -I .claude/skills/local-review/lmstudio_review.py --max-chunks 2 --out log/local/local-review.md
   ```

   - 出力先は `log/local/`（コミットしない）にする。
   - 既定は「推論を切る」「出力 768 トークン」「1 塊 6000 トークン」「3 塊まで」。

4. **指摘を裁く。** 出力の各行を、実際のコードと照らして確かめる。
   - 小型の 1-bit モデルは、行番号のずれや存在しない問題の指摘（幻覚）がある。**確かめずに直さない。**
   - 結果は次の表にしてユーザーに報告する。

   | # | 場所 | ローカル LLM の指摘 | 判定 | 理由 |
   | --- | --- | --- | --- | --- |
   | 1 | `path:line` | … | 採用 / 誤検知 / 保留 | コードのどこを見てそう判断したか |

   - 「採用」したものだけを直す。Claude 自身のレビューで見つけたものとは分けて書く。

## してはいけないこと

- ローカル LLM の指摘を根拠に、確かめないまま変更しない。
- 秘密を含みうるファイルを `--path` で無理に送らない。LM Studio は設定によってはプロンプトを全文ログに書く。
- Stop フックなどで自動実行しない。1 回に 10 分以上かかり、会話を止めてしまう（ADR-0007）。
