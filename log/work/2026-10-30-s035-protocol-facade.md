# S035 `protocol-facade`

| | |
| --- | --- |
| 日付 | 2026-10-30 |
| ブランチ | `feature/s035-protocol-facade` |
| worktree | `.git-worktree/s035-protocol-facade` |
| Phase | P4 の準備（計画書の WP-0） |
| セッション文書 | `doc/sessions/S035-protocol-facade/` |
| セッション記録 | `log/sessions/2026-10-30-0e301cfb-c9ff-440e-b48e-89148a3cfe93.md`（S033 と同じセッション） |

## 依頼

S033 の計画書の方針どおり、このセッションで WP-0（`globe-mode-protocol` と facade `globe-mode`）を進める。サブエージェントは Sonnet / high（結局使わなかった。1 人分の作業だった）。

## 決めたこと

- **protocol の型は今の JSON の形をそのまま struct にした**（フィールドの順も同じ）。serde_json は `preserve_order` が有効（indexmap が Cargo.lock にある）なので、struct の順がそのまま JSON の順になる。
- **応答の型は Serialize だけ**。`Diagnostic` と各段の報告に Deserialize が無く、core・analysis を触らない約束のため。
- **facade の `simulate_with(request, now)`** に検査 → コンパイル → 実行を 1 本にまとめた。時計は引数（wasm32 は std の時計が無い）。結果 `Simulated { response, run, compiled }` で、JSON にする部分と Rust からだけ使う部分（CLI が MAT を書く）を分けた。
- **CLI の文字の表示は `ws.db().check_model()` のまま**。DAE の未知数の一覧や初期化系の構造まで読むので、protocol の型に入れると JSON が太る。`--json` だけを protocol の型にした。
- **facade が db・exec・sim を再公開する**（`globe_mode::{db, exec, sim}`）。CLI の直接の依存を facade + core + ir に、wasm を facade だけにした。
- **断る文言を `--force` → `force` に変えた**。facade は CLI のフラグ名を知らない。JSON で唯一の変化（既定のまま進めた）。
- **JSON Schema（計画の 4）は持ち越し**。schemars は core・ir・analysis の約 50 の型に derive が要り、WP-0 の「触らない」に当たる。WP-B が TypeScript の型を要るときに決める。
- **snapshot 試験**は時計の値を含まない 4 つの出力だけ。`UPDATE_SNAPSHOTS=1` で書き直す。

## やったこと

- 新規 `src/Rust/mod/globe-mode-protocol/`（`CheckResponse`・`StructureCounts`・`SimulateRequest`・`SimulateResponse`・`ParametersResponse`、再公開、単体試験 2）。
- 新規 `src/Rust/mod/globe-mode/`（`Workspace`、`Simulated`、`solvers()`、`now_ms()`）。`Cargo.toml` の members に `src/Rust/mod/globe-mode`（glob `globe-mode-*` に当たらない）。
- `globe-mode-wasm/src/lib.rs`: facade の `Workspace` を包むだけにし、`Request` / `Response` は protocol の型の別名。依存を facade + serde + wasm-bindgen に。
- `globe-mode-cli`: `main.rs`・`simulate.rs` を facade 経由に（`Loaded.ws`）。`check --json` と `report_early_failure` は `CheckResponse`、`--list-parameters --json` は `ParametersResponse`、`simulate` は `simulate_with`。`tests/json_snapshots.rs` と `tests/snapshots/*.json` を足した。
- CI: `ci_plan.py`（glob を `globe-mode*` に、依存の判定を `startswith("globe-mode")` に、`WASM_CRATES` に 2 つ）、`test_ci_plan.py`（期待する集合に 2 つを足し、facade と protocol の試験を 1 つ）、`ci.yml` の wasm32 の `cargo check` に 2 つ。
- 文書: `doc/architecture.md` §2.2 の 4 行、`doc/roadmap.md` の S035 の節、計画書の WP-0 に「済」、セッション文書、索引。

## 検証

- 変更前のバイナリ（main と同じ HEAD）で CLI の出力を 19 通り保存し、変更後と `diff -r`: 違いは時計の値（`stats.*_seconds`、文字の表示の秒）と `sim-unbalanced.json` の `refused` の 1 語だけ。
- `cargo test -p globe-mode-cli --test json_snapshots`: 2 本通過。
- `python3 -I .github/scripts/test_ci_plan.py`: 21 本通過。
- コードの検査（`mise run check`）: fmt OK、clippy OK、test 534 本通過・0 失敗・10 ignore、deny（advisories・bans・licenses・sources）OK、`uv sync --locked` OK
- `mise run wasm:node`: Decay が bdf・dopri45・auto で通過（誤差 2.7e-8 / 1.9e-9 / 2.7e-8）、panic の文言も読めた。wasm は 14,589,321 バイト（S012 の 14.5 MB と同じ。変更前の数字はこの機械で測っていない）。MSL は `data/msl` が無いので飛ばした
- `check_docs.py`: デッキ 40 件 OK / 文書 174 件・リンク 961 件中の切れ 0 件。`render_slides.py`: 問題 0 件（図の箱を 1 度広げた）
- セッション記録の `--check`: 10 件 / 取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件（28 ファイル）

## 残したこと

- JSON Schema の書き出し（xtask）と TypeScript / Python の型の生成。
- `mise run wasm:compare` は回していない（実行系を変えていない。wasm の経路は `wasm:node` の Decay と `cargo test -p globe-mode-wasm` で確かめた）。
- facade の `Workspace::outline` は db のクエリ経由（CLI の `outline` は S034 まで frontend を直接呼んでいた。出力は同じ）。
