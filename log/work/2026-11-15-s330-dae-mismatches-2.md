# S330 `dae-mismatches-2`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s330-dae-mismatches-2`（`feature/s324-flat-composite` から切った。S324 が main に入る前だったため） |
| worktree | `.git-worktree/s330-dae-mismatches-2` |
| Phase | Track B |
| セッション文書 | `doc/sessions/S330-dae-mismatches-2/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S324 の後に残った食い違いの原因を確かめ、直せるものは直す。(1) 方程式の数が合わない 10 本（Rotational の摩擦・歯車）、(2) 構造の正則 4 本（`Transformers`）、(3) `dae-parse` 1 本（`MixtureGases`）。`globe-mode-dae`（と必要なら `globe-mode-syntax`）だけを変え、`globe-mode-flat` は S329 が触るので変えない。

## 決めたこと

- (1) は Rumoca の数え方の誤り。外部オブジェクトを作る `table = mu_pos`（行列）の大きさが、`getTable1DValueNoDer2(tableID, …)`（戻り値はスカラー）を束縛に持つ `mu0` を通って、`tau0 = mu0*cgeo*fn` の方程式の数になる。最小の再現（W4）で `mu_pos` の大きさに比例して増えることを確かめた。自前は MLS どおりの 1 本のまま。flat の数え方は変えない。
- (2) は dae の参照の読み。束縛が `plug.pin.v` を宣言より前に書くので flat が展開しない。`a.pin[i].v` の並びに直し、行 k は k 番目だけに触れる。`incidence` だけでなく `state_refs` も並べる `--dump` で突き止めた。
- (3) は構文。呼び出しのあとの `.欄` を `CallExpr` の節に取り込む。
- 計測は worktree 専用の `CARGO_TARGET_DIR`（`target-s330`）で回した（S325 の注意）。

## やったこと

- `src/Rust/mod/globe-mode-dae/src/lib.rs`: `Sel::Part`、`Ctx::parts`。試験 2 本。
- `src/Rust/mod/globe-mode-syntax/src/parser.rs`: 呼び出しのあとの `.欄`。試験 1 本（`tests/syntax.rs`）。
- `src/Rust/xtask/src/diff_frontend.rs`: `--dump` の行に `state_refs` を足した。
- 文書: ADR-0039 の付記（S330。S324 の「原因は未確認」も直した）、roadmap・architecture・features、`doc/frontend-diff.md`（`msl:diff-frontend` が書いた）、セッション文書、索引。

## 検証

- `mise run msl:diff-frontend`（release、460 s、5.1 GiB）: 自前で DAE にできた 357 → 358 / 358、構造の正則の一致 249/253 → 253/253。方程式の数 340/350・釣り合い 253/253・方程式と未知数 253/253・状態 253/253 は変わらず。`GlobeModeValidation` 75 本は変わらず。回帰 0、改善 5。
- `cargo test -p globe-mode-dae -p globe-mode-syntax` 通過、clippy（dae・syntax・xtask）0、`cargo fmt --check` 0。

## 残したこと

- `HeatLosses` の +21 の内訳、上流 issue の 4 点セット（方程式の数）、配列が 2 か所の欄の参照。
- 計測用の `target-s330` は締めで消す（親が worktree を片付けるとき）。
