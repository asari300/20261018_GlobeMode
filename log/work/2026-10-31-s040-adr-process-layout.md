# S040 `adr-process-layout`

| | |
| --- | --- |
| 日付 | 2026-10-31 |
| ブランチ | `feature/s040-adr-process-layout` |
| worktree | `.git-worktree/s040-adr-process-layout` |
| Phase | P4（[計画書](../../doc/plan/2026-10-30-release-estimate-and-parallel-plan.md)の WP-J） |
| セッション文書 | `doc/sessions/S040-adr-process-layout/` |
| セッション記録 | `log/sessions/2026-10-30-0e301cfb-c9ff-440e-b48e-89148a3cfe93.md`（S033・S035・S038・S039 と同じセッション） |

## 依頼

「進める。サブエージェントは Sonnet/high」。S038（WP-A）の次として、計画書の WP-J（ADR-0023「Desktop と Web のプロセス構成」。モデル A の作業）を Fable が自分で書く。同じ時間に WP-G（S039）を Sonnet のサブエージェントに渡した。

## 決めたこと

- **検査と実行は UI のプロセスから分ける**。ワーカーは 1 つで 1 度に 1 要求、落ちたら作り直して再生する（S036 の `GlobeWorker` と同じ手順）。理由は 1 モデルでも GiB の RSS、終わらない計算（`DrumBoiler`）、panic。
- **Desktop は `globe` の CLI を Tauri の sidecar にし、`globe lsp` の stdio の枠に独自の要求を足す**。専用のワーカー crate は作らない（枠が 1 つ増えるだけ）。Tauri の Rust 側は facade をリンクしない（コアの実体を配布物に 1 つにする）。
- **Web は Web Worker のまま**。線形メモリの目標は MSL を読んだ後 128 MiB・1 モデルの後 1 GiB、上限は 2 GiB 超で作り直し（上限は未実測と明記）。
- `Checker` の内側にメモリの上限を入れない（持ち越し 9 を閉じる）。
- 既定のまま: Android の sidecar は P6 で決める。独自の要求の名前は WP-C で決める。

## やったこと

- `doc/adr/0023-desktop-and-web-process-layout.md`（新規）と `doc/adr/README.md` の索引。
- `doc/architecture.md` §4（Desktop と Android の行を sidecar に）、`doc/roadmap.md`（持ち越し 9、S040 の節）、計画書の WP-J に「済」。
- 測定: `globe check` の ChuaCircuit の最大 RSS と壁時計、`globe lsp` の起動（initialize の応答まで。ライブラリなし / MSL あり）、`globe --help`。
- `doc/sessions/S040-adr-process-layout/`（スライド 6 枚と解説書）、`doc/sessions/README.md`。

## 検証

- コードは変えていない（`mise run check` は S039 と同じ main で通す）。
- 測定（x86_64 の WSL2、RAM 3 GB + swap 16 GB、release）: ChuaCircuit の検査の最大 RSS 1.48 GiB、壁時計 5.2〜6.6 s。`globe lsp` の起動 3 ms（ライブラリなし）/ 2.3〜2.8 s（MSL）。`globe --help` 10 ms 未満。
- `check_docs.py`: デッキ 44 件 OK / リンク 1024 件中の切れ 0 件
- `render_slides.py`: 問題 0 件
- セッション記録の `--check`: 取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件

## 残したこと

- WP-C で sidecar の独自の要求（`globe/check`・`globe/simulate`・`globe/graph`）の名前と形を決める。
- Android の sidecar（P6）、線形メモリの上限 2 GiB の実測（P6）。
- 測定は WSL2 の 1 回だけ。RPi と Windows では測っていない。
