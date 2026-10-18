# ADR-0008 ローカル LLM は副レビュアーとし、スキルから OpenAI 互換 API で呼ぶ

- 状態: Accepted（2026-10-18）

## 背景

- 実装とレビューは Claude Code（Opus）が行う。ユーザーは、レビューのときに副次的に、Windows 側の LM Studio で動くローカルモデル（Bonsai-27B、1-bit）にもセカンドオピニオンを出させたい。
- 作業は WSL2 の Ubuntu で行い、LM Studio は Windows 側で動く。WSL2 は mirrored モードで、`http://localhost:1234` で届く。
- PC は Ryzen 7 7730U と内蔵 GPU で、RAM は 31GB。実測で、Bonsai-27B の prompt 処理は約 8〜10 tok/s、生成は約 2 tok/s だった。内蔵 GPU にオフロードしても速くならなかった（[調査](../research/lmstudio-wsl2-review.md#3-bonsai)）。
- 候補は 4 つあった。
  - A. スキルとスクリプトで、差分を OpenAI 互換 API に送る
  - B. MCP サーバ（houtini-lm など）
  - C. 2 つ目の Claude Code（`claude --bare -p`）を、Anthropic 互換の `/v1/messages` に向ける
  - D. hooks で自動実行する

## 決定

**A を採る。** `.claude/skills/local-review/`（`SKILL.md` と、標準ライブラリだけの `lmstudio_review.py`）にした。

- ローカル LLM の指摘は**助言**にとどめる。Claude がコードを読んで「採用 / 誤検知 / 保留」を判定し、採用したものだけを直す。
- 差分は塊に分けて送る。
  - 各行に新しいファイルでの行番号を付ける。小さいモデルは @@ から行番号を数えられないため。
  - ロックファイル、取得物、ログ、秘密を含みうるファイル、バイナリは送らない。
- モデルが読み込まれていなければ止まる。JIT 読み込みに任せると、既定のコンテキスト長（最大の 262K のことがある）で読み込まれ、メモリが足りなくなるため。
- 推論（thinking）は既定で切る（`reasoning_effort: "none"`）。切らないと、短い質問でも推論だけで 2 分かかった。
- 推奨の読み込み設定は `lms load prism-ml/bonsai-27b -c 12288 --gpu off --parallel 1 --ttl 1800`。コンテキストを長くしても、入力の処理に時間がかかるだけで使い道が無い（16K の入力を処理するだけで約 30 分）。
- 依存を足さない。licenses.md も変えない。

### 採らなかった案

| 案 | 理由 |
| --- | --- |
| B. MCP（houtini-lm） | 動くが、Node の外部パッケージを `npx` で毎回取ってくる。要る設定（`HOUTINI_LM_MIN_TOKENS` の既定 4096 を下げる、推論を切る）を全員の環境で揃える必要がある。分割や除外を自分で制御できない。試し方は [S006 の解説書](../sessions/S006-local-llm-review/解説書.md) に残す |
| C. `claude --bare -p` | Claude Code の推奨は 25K 以上のコンテキストで、Claude 以外のモデルは Anthropic のサポート外。"pong" と返すだけで 179 秒かかった。Claude Code がこのモデルのコンテキストを 200K と仮定する |
| D. hooks | 1 回に 10 分以上かかり、会話が止まる。小さいモデルの誤った指摘に毎回 Claude が振り回される |

## 結果

- `/local-review` で、作業ツリーの差分を数分〜数十分でレビューさせられる。`--dry-run` で所要時間の見積もりを先に出せる。
- 小さいモデルの誤検知は Claude が裁くので、誤った修正は入らない。その代わり、確かめる手間が増える。
- LM Studio の設定はユーザーが GUI で変える（Windows 側）。推奨は次の 3 つで、[S006 の解説書](../sessions/S006-local-llm-review/解説書.md) にある。
  - 127.0.0.1 で待ち受ける
  - 必要なら認証を有効にする
  - プロンプトの全文ログを止める
- もっと速いモデル（MoE や小さい dense）に替えるときは、`LMSTUDIO_MODEL` を変えるだけで済む。
