# S076 `desktop-columns`

| | |
| --- | --- |
| 日付 | 2026-11-02 |
| ブランチ | `feature/s076-desktop-columns` |
| worktree | `.git-worktree/s076-desktop-columns` |
| Phase | P4 |
| セッション文書 | `doc/sessions/S076-desktop-columns/` |
| セッション記録 | （親が写す） |

## 依頼

S065 の持ち越し。Desktop（Tauri）は sidecar `globe lsp` の `globe/simulate` が CSV を返し、`TauriBackend` が CSV を解析して `Columns` にしていた。これを列の道にする。facade・protocol に列の形があればそれを使い、LSP に独自要求を足して列ごとの数の配列を JSON で返す（バイナリの道は作らない）。JSON で足りるかを 10 万点 × 7 列の時間で確かめる。親のサブエージェントとして、main へのマージと push はしない。

## 決めたこと

- **要求は `globe/simulateColumns`**（params は `globe/simulate` と同じ `{request}`）。`globe/simulate` に `format` を足す案より、既存の「1 要求 1 形」の表に 1 行足すほうが小さい。
- **結果は `{response, layout, columns}`**。wasm の `SimColumns`（`response`・`layout()`・`column(i)`）と同じ並びで、`layout` は protocol の既存の `ColumnsLayout`。`TauriBackend` の変換は `WorkerBackend` と同じ `names.slice(1)`。protocol・facade は変えていない（`simulate_with` の `run` と `ColumnsLayout`・`now_ms` が既にある）。
- **NaN / Inf は null**（serde_json の `Value` がそうする）。UI の欠損と同じなので `gapNonFinite` を通さない。
- **JSON で足りる**（下の数字）。バイナリの道（Tauri の `ipc::Response`）は作らない。
- **UI の `parseCsv` を消した**。使う所が無くなった。`globe-mode-wasm/node/columns.mjs` は自前の写しを持つので、注釈だけ直した。
- **desktop の `worker.rs`**: 予算 180 s と `files` の再生が `method == "globe/simulate"` で分けていたので、`globe/simulateColumns` も含めた。`main.rs`（`rpc` の中継、smoke）は変えていない。smoke の `globe/simulate` は S056 の RSS の基準値なのでそのまま。

## やったこと

- `src/Rust/mod/globe-mode-lsp/src/lib.rs`: `globe/simulateColumns` の腕（`request.no_csv = true` で `simulate_with`、`run.trajectory` を列に並べる）、モジュール注釈の表、試験 `custom_requests_check_simulate_and_graph` に往復（`names == ["time","x"]`、列の長さが `points`、最後の値が `final` と一致、`csv` が null）と -32602 を足した。
- `src/Rust/apps/desktop/src/worker.rs`: 予算と再生の条件、`GLOBE_BIN` の試験に `globe/simulateColumns` を 1 本。
- `src/Svelte/src/lib/tauri-backend.ts`: `simulateColumns` が `globe/simulateColumns` を呼ぶ。`src/Svelte/src/lib/plot/csv.ts`: `parseCsv` と補助を消した。
- 文書: ADR-0023 の独自要求の表、roadmap の S065 の持ち越しの取り消し線と S076 の節、索引、スライドと解説書。

## 検証

- 時間（WSL、release の `globe lsp` を Node から stdio で叩く一回きりのスクリプト。モデルは S051 の `Wide`＝4 状態 + 代数 2、`StopTime = 1000, Interval = 0.01` で 100,001 点 × 7 列、`bdf`。要求を送ってから `JSON.parse` と（CSV の道は）`parseCsv` が済むまで。`globe/simulate`（CSV）と `globe/simulateColumns` を交互に 5 回、その中央値。他のセッションが並走して負荷平均 25 前後で揺れが大きい）:

  | 計測 | CSV → parseCsv | 列の JSON |
  | --- | --- | --- |
  | 1 回目 | 1425 ms | 1110 ms |
  | 2 回目 | 1648 ms | 951 ms |
  | 3 回目 | 2538 ms | 1551 ms |

  応答の大きさはどちらも 13.8 MiB。値は 1 万点おきに 1e-9 の相対誤差で一致。1 回目は計測側の読み取りが `Buffer.concat` の二乗だったので、2・3 回目はそれを直した版。3〜4 割短く、JSON の数の配列で足りると判断した。wasm（S051、ランナーで 159 ms）より遅いのは、サーバの `Value` の組み立てと `JSON.parse`（2・3 回目で 1 回 64〜339 ms）とパイプ越しの 14 MiB。
- `cargo test -p globe-mode-lsp custom_requests`: 1 passed。
- `mise run check`: EXIT=0。テスト 58 本の束で 600 passed / 0 failed / 11 ignored、clippy の警告 0、`cargo deny` は advisories・bans・licenses・sources とも ok（1 回目は `cargo fmt --check` の差で落ち、整形して回し直した）
- `pnpm check`（svelte-check）: 260 ファイル、0 errors / 0 warnings。`mise run ui:check`: 1 回目は `test_ui.py` が ok（Decay、10 万点の列の道で最初の描画まで 23.2 ms・「実行」から戻るまで 217 ms、W1001）、`test_graph.py` が wasm の読み込み待ち 120 s で時間切れ（負荷平均 245。グラフは今回の変更と関係しない）。`test_graph.py` だけ回し直して ok（RealCircuit 4 / 4、Hierarchical 10）
- desktop.yml（feature ブランチから dispatch、run 41378682869）: success（ubuntu・windows の build、`worker::tests::talks_to_globe_lsp_and_recovers_from_a_kill` が本物の `globe` で 2 回 ok＝`globe/simulateColumns` の往復を含む、smoke 3 本の `passed=true`・`simulate ok=true`）
- `check_docs.py`: デッキ OK / 文書 337 件・相対リンク 1629 件中の切れ 0 件
- `render_slides.py`: デッキ 1 件・6 枚 × 3 表示（desktop-light・desktop-dark・mobile）で問題 0 件

## 残したこと

- 残りの約 1 s を詰めるなら、`Value` を介さず書き出すか、Tauri の生のバイト列で `Float64Array` を渡す。
- Desktop の UI を実機で開いての確認はしていない（Tauri の窓は CI の smoke と `worker.rs` の試験だけ）。
- `globe/simulate` の CSV の道は残した（CLI・他のクライアント用）。
