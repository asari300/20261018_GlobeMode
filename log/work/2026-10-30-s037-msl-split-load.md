# S037 `msl-split-load`

| | |
| --- | --- |
| 日付 | 2026-10-30 |
| ブランチ | `feature/s037-msl-split-load` |
| worktree | `.git-worktree/s037-msl-split-load` |
| Phase | P3（候補 3 の残り。P6 の前提） |
| セッション文書 | `doc/sessions/S037-msl-split-load/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md` |

## 依頼

「3 進めて。サブエージェントは Sonnet/high。レスポンスは日本語。mslsim は GitHub Actions から実行を検討」。roadmap の候補 3（ブラウザ）の残り、MSL を読んだ後の wasm のメモリを減らすことと、`msl:sim` を GitHub Actions で回す道を作ること。

## 決めたこと

- 分割読み込みは字句の走査で名前をたどる（`split_library.rs`）。構文木を作るとメモリを減らす意味が薄れるため。広めに拾うのはよく、狭いと Rumoca の名前解決が落ちる。
- 足りないときは Session を作り直して文書を足し直し、読んだファイルは外さない（Rumoca の in-memory の組は差し替えで渡す）。
- 分割を既定にするのは wasm の JS の `new Workspace(json, name)` だけ。`eager = true` で全部を読む道を残す。ネイティブの CLI と `msl:funnel` の判定は変えない。
- 1 ファイルに 1 つのトップレベルのクラスを仮定する（`shortcut:` のコメント）。
- `msl:sim` は GitHub Actions の手動のワークフロー（`msl-sim.yml`）で回す（ユーザーの「GitHub Actions から実行を検討」）。push の CI には入れない。報告と JSON は artifact、前回の JSON は actions/cache から戻す。
- S 番号: main には S038（別のセッション）があり、S037 は空いていた。

## やったこと

- `globe-mode-frontend`: `split_library.rs`（字句の走査、修飾名の索引、閉包）、`rumoca.rs` の `Checker::with_library_sources_split` と `ensure_library`（`sync_documents` の先頭で呼ぶ）、`tests/split.rs`。
- `globe-mode-db`・`globe-mode`（facade）・`globe-mode-wasm`: `with_library_sources_split` を通し、JS の構築子に `eager` を足した。
- `node/smoke.mjs`: `Rotational.Examples.First` を足し、モデルごとに値とメモリを出す。
- レビューで直したこと: (1) `import A.B.C;` が `C` を別名にしていなかった（MultiBody の `Pendulum` が `Types.ShapeType` を解決できず GM-E1001。`tests/split.rs` の `--ignored` で見つけた）。`import A.{c, d}` も別名にした。単体試験に足した。(2) `tests/split.rs` の比較の相手を同じ本文の全部読み（`with_library_sources`）にした（ディスクから読む `Checker::new` とはファイルの名前が違い、常に不一致だった）。(3) wasm の doc コメントの「40 MiB 台」を実測の 62 MiB に。
- `.github/workflows/msl-sim.yml`、CLAUDE.md の `msl:sim` の行。
- 文書: roadmap（候補 3 と S037 の節）、features、architecture、セッション文書、索引。

## 検証

- `mise run check`: 通った（fmt・clippy・test・deny・uv sync --locked）。import の修正の後に frontend の fmt・clippy -D warnings と単体試験 3 件を回し直して通った。
- `cargo test -p globe-mode-frontend --test split -- --ignored`: `msl-compare.txt` の 20 本すべてで全部読みと `CheckReport` が一致（890 s、debug）。修正前は `Pendulum` で不一致。
- `mise run wasm:node`（修正の後）: 通った。MSL 2556 ファイルの読み込み 203 ms、読み込み後 62 MiB（S012 は 262 MiB）、`ChuaCircuit` の後 113 MiB（513 MiB）、C1.v(T) = -0.9928894683235796（S012 と同じ値）、`Rotational.Examples.First` の後 113 MiB。
- `mise run wasm:compare`: 347 / 360 一致、既知の差 13、不一致 0（修正の前に回した。修正は分割の閉包だけで、compare は MSL を使わない）。
- `mise run wasm:web`: Node と Chromium で Decay の誤差 2.7e-8、panic の後 restarts = 1（修正の前）。
- `msl-sim.yml`: YAML として読めることだけを確かめた。GitHub Actions ではまだ回していない。
