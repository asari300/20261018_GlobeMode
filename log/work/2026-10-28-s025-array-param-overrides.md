# S025 `array-param-overrides`

| | |
| --- | --- |
| 日付 | 2026-10-28 |
| ブランチ | `feature/s025-array-param-overrides` |
| worktree | `.git-worktree/s025-array-param-overrides` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S025-array-param-overrides/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md` |

## 依頼

「進めて。サブエージェントは Sonnet/high」。S021・S022 の worktree は消してよい（未コミットの変更はユーザーが確かめた）。S024 で残した「配列の要素のパラメータ（`a[1]`）の上書き」を確かめる。

## 決めたこと

- ユーザーの指示: サブエージェントは Sonnet・effort high。S021・S022 の worktree は消してよい（未コミットの変更はユーザーが確認済み）。消した（`checkout -- .` で `D data/.gitkeep` と `doc/msl-sim.md` の変更を捨ててから `worktree remove` と `branch -d`）。
- 配列全体（`a`）は構造的な上書きに渡さない。Rumoca は束縛をスカラのリテラルで置き換えて ED020 になり、「未対応」（GM-U0001）に見えていた。実行時の上書きが GM-E10006 で断る。
- 同じ名前の修飾で内側へ写された配列の要素（`src.k[1]`）は、変換の前に GM-E10006 で断る。サブエージェントは「直せないので文書化だけ」と報告したが、黙って誤る形なので断る方を選んだ（見分けは名前の形だけ。違う名前の修飾は見逃す。コードに `shortcut:` を書いた）。
- 直接使う配列の要素（`a[2]`）と時間イベントの配列の要素（`t[1]`）は、実行時の上書きで正しく届くので変えない。

## やったこと

- `globe-mode-frontend/src/rumoca.rs`: `compile_retaining_with_overrides` で、型の表から配列の名前を外し（サブエージェント）、写された配列の要素を変換の前に断る（手元）。
- `GlobeModeValidation/Simulation/ArrayParameters.mo`（`package.order` に追加）と、`globe-mode-exec/tests/simulate.rs` の試験 `array_parameter_overrides_use_elements_at_run_time_and_reject_whole_arrays`。
- 文書: ADR-0022 に配列の項、research「S025 で調べたこと」（候補 21）、roadmap・features・セッション文書・索引。

## 検証

- `cargo test -p globe-mode-exec --test simulate`: 30 本通過・失敗 0・無視 3。
- `mise run check`: 493 本通過・失敗 0・無視 10（fmt・clippy・deny・uv を含む。1 時間 15 分）。
- `mise run msl:compare`: 20 本中 20 本が一致（両方とも成功 17 本）。
- `mise run wasm:compare`: 162 件中 159 件が一致、既知の差 3、不一致 0（`ArrayParameters` は 9 件とも一致）。`wasm:node` は個別には回していない（`wasm:compare` が wasm のビルドと実行を含む）。
- `check_docs.py`: 文書 141 件 / 相対リンク 878 件 / 切れ 0 件（スライド S025 は OK）。

## 残したこと

- 違う名前の修飾（`final kk = k`）で写された配列の要素は断れない（名前の形で見分けているため）。Rumoca が配列の構造的な上書きを持ったら外す。上流の候補 21。
- wasm で `src.k[1]` の断りは走らせていない（同じ関数を通るが `wasm:compare` に上書きの例題は無い）。
- `msl:funnel`・`msl:sim` は回していない（上書きの無い道は変えていない）。
