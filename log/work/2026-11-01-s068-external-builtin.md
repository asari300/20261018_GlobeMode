# S068 `external-builtin`

| | |
| --- | --- |
| 日付 | 2026-11-01 |
| ブランチ | `feature/s068-external-builtin` |
| worktree | `.git-worktree/s068-external-builtin` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S068-external-builtin/` |
| セッション記録 | [2026-10-31-0b797c72-...](../sessions/2026-10-31-0b797c72-9651-45d3-b466-98f7c341d068.md)（親セッション。S048〜S052 と同じ） |

## 依頼

S067 の持ち越し。MSL 4.0.0 の `Modelica.Math` の `external "builtin"`（`sin`・`exp` など 13 個）を実行系で実行できるようにし、MSL 4.0.0 で止まっていた 70 本を実行できるようにする。4.1.0 の結果は変えない。道は小さい順に調べて最初に採れたものを採る。親（サブエージェントとして実装）。

## 決めたこと

- **道は (b)**: (a) Rumoca の公開 API に hook は無い（`external_functions.rs` は言語を分類するだけ、`numeric.rs` は全 external を断る）。`Dae` は不変で構築 API は閉包だが、serde の `Serialize` / `Deserialize` があり、読み戻しで Rumoca が検査し直すので、直列化の値の `call` を `builtin` に替えて読み戻す。(c)（MSL の文字列置換）は版とライブラリに結びつくので採らない。[ADR-0029](../../doc/adr/0029-external-builtin-rewrite.md)。
- **場所は frontend の `compile_strict`**: 実行・プリフライト・wasm が同じ `Dae` を使う。`check` の判定は元の `Dae` のまま（判定の数は変わらない）。
- **対象は external 節で判定**（言語 builtin・記号が組み込み関数・入力数と出力 1・引数がすべて入力で番号が増える順）。名前の固定表は組み込み関数の名前と引数の数だけ。
- **失敗したら元の `Dae`**（`GM-U0004` に戻るだけで誤った結果にならない）。
- `serde_json` を frontend に足した（workspace に既にある。Cargo.lock は frontend の依存の 1 行だけ）。新しい外部依存は無い。
- 検証用モデル `ExternalBuiltin` を `Simulation/package.order` に入れた（`wasm:compare` が回す）。
- 親の追加依頼: `.gitignore` に `data/msl-4.0.0/` を足した。MSL 4.0.0 は main の `data/msl-4.0.0` を `ln -s` で借りた。

## やったこと

- `globe-mode-frontend/src/external_builtin.rs`（新規）、`lib.rs`、`rumoca.rs`（`compile_strict` から呼ぶ。`collect_mo_files` の退行の修正と試験）、`Cargo.toml`・`Cargo.lock`。
- `globe-mode-exec/tests/simulate.rs` に `external_builtin_math_functions_run`。`ExternalBuiltin.mo`（`sin`・`exp`・`atan2`・`mod`）。
- 途中で 3 回直した（いずれも CI か AixLib の手元の実行で見つけた）:
  1. S066 の退行（AixLib が全部 EM001）。`X.mo` と `package.mo` の無い `X/` が並ぶとき `X/` を読まない。
  2. `external_tables`（CombiTable の表）を持つ `Dae` を書き換えていなかった。直列化の形から外して読み戻し、付け直す。
  3. `arguments` の `input` の式の番号で式を引くと、直列化の番号が内部の番号とずれて確かめが失敗した（AixLib の `Declination`）。番号の大小だけを見る。
- 上流の下書き `upstream/16-…`（`.md` と `.mo`）と README の行。ADR-0029、research、roadmap、索引、スライド、解説書。

## 検証

- `cargo fmt --all --check`: 差分 0。`cargo clippy -p globe-mode-exec -p globe-mode-frontend --all-targets -- -D warnings`: 0 件。`cargo test -p globe-mode-exec -p globe-mode-sim -p globe-mode -p globe-mode-frontend`: 失敗 0（exec の `simulate` は 31 本通過・3 ignore。`external_builtin_math_functions_run` は bdf・dopri45 で `x`・`z`・`w` が解析解と一致。直す前は `GM-U0004` で失敗することを確認）。
- `mise run msl:compare`: 20 / 20 一致（両方成功 17 本）。
- `mise run wasm:node`: ok。`mise run wasm:compare`: **367 / 380 件が一致、既知の差 13、一致しない 0**（`ExternalBuiltin` の 20 行を足した分。前は 347 / 360）。途中で `ExternalBuiltin` に `mod(time, 0.7)` を入れたら、イベントの時刻が wasm とネイティブでずれて 10 件が一致しなかったので外した。
- MSL 4.0.0 の例題（debug）: `Pendulum`・`Surfaces`・`IMC_DOL` が `GM-U0004` から実行できた。`TransformerYD` は別の原因で `GM-U0004` のまま。
- Actions `libraries.yml -f msl=4.0.0`（run 41313259246、DeltaRobot・IBPSA・AixLib）: success。実行できた AixLib 71 → 97、IBPSA 47 → 66、DeltaRobot 2 → 5。
- `check_docs.py`: 下で記す。セッション記録の `--check`・ステージ差分の漏洩検査は親が回す。

## 残したこと

- Buildings は 70 分かかるので CI で回し直していない（Buildings の 27 本は IBPSA・AixLib と同じ原因なので同様に直る見込み【未確認】）。
- 残り: Buildings の `Interior`・`InteriorConvection`（`invalid Solve IR contract`）。S067 が別の原因とした DeltaRobot の 3 本は直った（2 → 5）。AixLib の 4.1.0 との差は 1 本（97 / 98。4.1.0 の OK が 1 本多い差と同じ）。AixLib の残り 21 本の `GM-U0004` は `Modelica.Math` 以外（インデックス 3 の構造解析、`String` のメタデータ、非有限な値）。
- 変換が `external_tables` を付け直す試験はモデルでは無い（AixLib の `HumansSenTDep` を手元と CI で確かめた）。
- serde の形は Rumoca の公開契約ではない。rev を上げたら `ExternalBuiltin` の試験で確かめる。

- 親の最終検査（main d3e932e を取り込んだ後）: `mise run check` ok、`wasm:node` ok、`wasm:compare` 367 / 380 件が一致・既知の差 13・一致しない 0、`check_docs.py` リンク切れ 0。
