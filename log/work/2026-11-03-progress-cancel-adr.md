# S121 `progress-cancel-adr`

| | |
| --- | --- |
| 日付 | 2026-11-03 |
| ブランチ | `feature/s121-progress-cancel-adr` |
| worktree | `.git-worktree/s121-progress-cancel-adr` |
| Phase | P7 |
| セッション文書 | `doc/sessions/S121-progress-cancel-adr/` |
| セッション記録 | （親が写す） |

## 依頼

S110 の言語 API の機能表の埋める順の 6「進捗と呼び出し側からの中断。先に ADR を書く」の ADR を書く。今ある口（exec の `run_with_progress`・`max_wall_seconds`、Desktop の `msl-progress`・`cancel_msl`、LSP、wasm の `GlobeWorker`、Python の `simulate_columns`、CLI、ADR-0014・ADR-0017）を表にし、進捗の単位・中断の仕組み・各口の形・中断後の結果を 1 つに絞り、却下した案を 1 行ずつ書く。文書だけ、cargo は回さない（親のサブエージェントとして。main へのマージ・push は親）。

## 決めたこと

既定のまま進めた（ユーザーとの相談は無い）。詳細は [ADR-0033](../../doc/adr/0033-progress-and-cancel.md)。

- 進捗: 段（`ExecStage`）は毎回、積分の時刻 t は 100 ms に 1 回まで。時計は facade の `now`（wasm で `Instant` を呼ばないため）。t を出せるのは session の道だけ（Rumoca の型付きの道に口が無い）。
- 中断: 呼び手の `CancelToken`（`Arc<AtomicBool>`）。段の境目、`auto` の回し直しの前、session の道で `run_to_stop` を `advance_to` の繰り返しに替えた区切りで見る。Rumoca の `advance_to` は公開で、区切りの時刻を呼び手が選べ、止めたあとも `finish()` で trace が取れる（rev `276e82bd` の `fmi_me/session.rs` 462〜513 行で確かめた）。
- 各口: facade に `SimulateHandle`。C はコールバックの戻り値、NIF は resource のトークンと `cancel`、Python は GIL を放してコールバックと `check_signals`、wasm は postMessage と `terminate()`（Worker の中は単一スレッドでトークンを外から立てられない）、LSP は `$/progress` と `$/cancelRequest`（読むループと計算のスレッドを分ける）。CLI は変えない。
- 中断後: 積分の中なら止めた時刻までの列と情報 `GM-I11004`（`Termination::Cancelled`、`ok: true`）。その前なら `ok: false` と `GM-I11004` だけ。
- 却下: 段だけ、出力の点ごと、割合、`max_wall_seconds` の拡張、スレッドを落とす、wasm の SharedArrayBuffer、戻り値だけで止める、診断だけ、誤りにする。
- 新しい依存: 0。

## やったこと

- `doc/adr/0033-progress-and-cancel.md`（新規、Proposed）、`doc/adr/README.md` に行。
- `doc/architecture.md` の「5. 言語 API」に 1 行。
- `doc/research/lang-api-matrix-2026-11-03.md` の進捗・中断の行と埋める順の 6 の下に ADR へのリンク。
- `doc/roadmap.md` の P7 の行と「S121 で済んだこと」、`doc/features.md` の言語 API の行に ADR へのリンク。
- `doc/sessions/S121-progress-cancel-adr/`（slides.html 8 枚・解説書.md）、索引の行。

## 検証

- コードの検査: 対象外（Rust を触っていない。cargo は回していない）
- `check_docs.py`: デッキ全件 OK（S121 を含む）/ 文書 511 件・相対リンク 2566 件中の切れ 0 件
- `render_slides.py`: デッキ 1 件（8 枚 × desktop-light・desktop-dark・mobile）/ 問題 0 件（main の `.venv/bin/python -I`、load average 8.69 のとき）
- セッション記録の `--check`: （親が写す）
- ステージした差分の漏洩検査: 0 件

## 残したこと

- 実装（facade と exec → wasm → Python → C・NIF → LSP）。各段の試験の案は ADR-0033 の「結果」。
- ネイティブの型付きの道の積分の中の t と中断（session の道をネイティブの既定にするなら速さを測る【未確認】）、lowering の中で止めること、CLI の Ctrl-C、wasm の途中の列。
- `uv sync` は回していない（cargo のビルドを避けた。render_slides は main の `.venv` で回した）。
