# S005 `local-llm-review`

| | |
| --- | --- |
| 日付 | 2026-10-18 |
| ブランチ | `dev/supervised-llms`（ユーザーの決定。S 番号の命名から外れる） |
| worktree | orca の worktree `merrow`（`.git-worktree/` の外） |
| Phase | 運用（開発環境） |
| セッション文書 | [doc/sessions/S005-local-llm-review/](../../doc/sessions/S005-local-llm-review/解説書.md) |
| セッション記録 | [log/sessions/2026-10-18-40153c9b-1d16-414d-85e1-aa59674d1077.md](../sessions/2026-10-18-40153c9b-1d16-414d-85e1-aa59674d1077.md) |

## 依頼

> orca で作業しているが、WSL2-Ubuntu 26.04 内で Claude Code と Win 側 LM Studio サーバーを使いたい。基本的な実装は Claude Code でレビュー時に副次的に LM Studio サーバーを使用する。手順を包括的に調査し詳細に解説してください。
>
> - LM Studio のモデルは Bonsai（既インストール）
> - PC リソースが小さいためコンテキスト長や KV キャッシュなど調整して
> - これまでと同様に HTML スライド（全体 1 枚、個別でそれぞれのページ）と解説書の .md 作成

## 決めたこと

### ユーザーと決めたこと（計画の段階で質問した）

- ブランチは `dev/supervised-llms` のまま。S 番号は S005 にした。
- 範囲は「文書＋スキル＋MCP 版の比較」。
- スライドは全体デッキ＋トピック別デッキ。トピック別は `NN-<topic>.html` にし、検査のツールも広げた。
- LM Studio のモデルは `lms.exe` で読み込み直してよい。サーバの GUI の設定は手順を書くだけにし、変更はユーザーが行う。

### 既定のまま進めたこと（理由つき）

- **主の方式はスキル＋標準ライブラリのスクリプト**にした（ADR-0007）。MCP（npx の外部パッケージ）と `claude --bare -p` は、実測して比較するにとどめた。
- **推奨の ctx は 12,288**。KV は 64 KiB/トークンと軽く、律速は prompt 処理（約 9 tok/s）。ctx を広げても待ち時間が延びるだけなので、差分 6,000 ＋ system ＋ 出力 768 が収まる長さにした。
- **GPU は off**。GPU max と速さが同じで、読み込みが 53 秒から 8.5 秒になる。
- **推論は切る**（`reasoning_effort: "none"`）。`chat_template_kwargs` は効かなかった。
- **KV の量子化は既定（fp16）のまま**。12K で 0.35 GiB しか減らず、GUI でしか変えられない。

## やったこと

- `.claude/skills/local-review/`: `SKILL.md`、`lmstudio_review.py`、`test_lmstudio_review.py` を新しく作った。
- `.claude/skills/session-docs/`: `check_docs.py`、`render_slides.py`、`SKILL.md` を、トピック別デッキに対応させた。
  - 対象を `doc/sessions/*/*.html` に広げた。
  - 全体デッキとトピック別デッキの相互リンクを検査するようにした。
  - スクリーンショットの置き場所を `<セッション>/<stem>/<view>` にした。
- `.claude/hooks/save_session_log.py`: 伏せ字の規則を足した。Windows のプロファイル名（語に分かれた形も含む）、LAN（192.168/16、172.16/12）と Tailscale（100.64/10）の IP。
- `doc/research/lmstudio-wsl2-review.md`、`doc/adr/0007-local-llm-reviewer.md` を書いた。ADR の索引と、`doc/architecture.md` §7 の表に行を足した。
- `doc/sessions/S005-local-llm-review/`: 全体のデッキ、トピック別の 5 本、解説書を書いた。索引の行も足した。
- LM Studio: Bonsai-27B を、計測のために何度か読み込み直した。最後は `-c 12288 --gpu off --parallel 1` にした。

## 検証

- コードの検査: Rust は触っていない。`uv sync --locked` は通過。
- `lmstudio_review.py` の単体テスト: 13 件中 13 件が通過。
- `check_docs.py`: デッキ 11 件 OK、リンク 235 件中の切れ 0 件。
- `render_slides.py`: デッキ 11 件 × 3 表示で問題 0 件。目視で見切れを 4 枚直した。
- markdownlint: 新しい 5 件で MD013 以外 0 件。
- セッション記録の `--check`: 取りこぼし 0 件。Windows のプロファイル名と私的 IP の伏せ字の規則を、フック（`save_session_log.py`）に足した。
- ステージした差分の漏洩検査: finish-session の規則（ログイン名、git のメール、トークンの形）で 0 件。追加で Windows のユーザー名、プロファイル名、私的 IP も探して 0 件。Windows の作業フォルダ名（`WDir`）は `log/sessions/` に 17 件あるが、個人を特定しないフォルダ名なので伏せ字の対象外とした。
- 実機の計測（すべて 2026-10-18）:
  - prompt 処理は約 8〜10 tok/s（6K の入力では 5.3 tok/s）、生成は約 2.3 tok/s。GPU を使っても同じだった。
  - 推論を切ると「17×23」は 126.7 秒から 3.8 秒になった。
  - `claude --bare -p` は "pong" に 179 秒。
  - MCP（houtini-lm）は 105.9 秒で、出力が途中で切れた。
  - `/local-review` は、自分の差分の 6,154 トークンで 19.4 分（`NO_FINDINGS`）。仕込んだバグ 3 つの差分では 91 秒で、1 件を正しい行番号で指摘した。

### 仕込んだバグへの指摘を Claude が裁いた結果

| # | 場所 | ローカル LLM の指摘 | 判定 | 理由 |
| --- | --- | --- | --- | --- |
| 1 | `tools/stats.py:6` | ファイルを閉じない | 採用 | `open()` の戻り値が閉じられていない。行番号も正しい |
| — | `tools/stats.py:13` | なし | 見逃し | `range(len(values) - window)` が最後の窓を落とす |
| — | `tools/stats.py:22` | なし | 見逃し | `avg` が空だと `avg[-1]` が IndexError |
| — | `tools/stats.py:23` | なし | 見逃し | 空の入力で ZeroDivisionError |

## 残したこと

- LM Studio の待ち受け（0.0.0.0 → 127.0.0.1）と、プロンプトの全文ログの停止は、ユーザーの作業として残した。
- 速いモデル（MoE など）との比較と、CPU のスレッド数の調整は、まだ測っていない。
- 本来の S 番号の作業（P2 の 2 回目）は次のセッションで行う。
