# S367 `flat-expandable`

| | |
| --- | --- |
| 日付 | 2026-11-16 |
| ブランチ | `feature/s367-flat-expandable`（`main`〔S365 のマージ `9259cb02`〕から切った） |
| worktree | `.git-worktree/s367-flat-expandable` |
| Phase | Track B（B3 flatten・B4 DAE） |
| セッション文書 | `doc/sessions/S367-flat-expandable/` |
| セッション記録 | なし（この WP 専用の記録が `log/local/` に無いので省いた） |

## 依頼

S365 のあと、MSL の例題で自前の flat が断るのは 18 本（`composite-binding` 6・`expandable` 5・`non-constant-dim` 4・`connect-array` 1・`non-constant-for` 1・`unsized-array` 1）。多い順に `expandable` の 5 本と `composite-binding` の 6 本を平らにする（expandable connector: MLS 9.1.3）。余力があれば `non-constant-dim`。Rumoca の flat と変数の名前・方程式の数を `diff-frontend --class … --dump` で突き合わせて一致させる。回帰 0 が条件、S365 の上限（`MAX_ITERS`・`MAX_SCALARS`）は壊さない。最後に全体の検査を 1 回回し、数字と回帰を書く。

## 決めたこと

- **base は `main`**（S365 は `9259cb02` で main に入っていた）。回帰の基準は親が写した `target-shared/bin-copies/frontend-diff-s365.json`（`target/frontend-diff.json` に置いた）。
- **expandable は Rumoca の実際の動きに合わせた**。`rumoca-phase-flatten` の `generate_connection_set_equations`・`potential_set_is_unused_expandable` と `rumoca-phase-dae` の `model_roles.rs`（`UnusedExpandable`）を読んで、(1) 宣言済みの欄は変数に残す（つながらない欄も。依頼の「接続されない欄は作らない」は、Rumoca の flat の名前の集合とは合わない。DAE が数えないという意味で合わせた）、(2) 端がすべて宣言済みの欄（束縛なし）で、ほかの方程式・algorithm・束縛から読まれない集合は等式を作らない、(3) 等式を作った集合に入らない欄は DAE の未知数にしない、とした。接続集合は自前では階層ごとに作るので、(2) の判定のために、expandable の欄を含む集合を端の共有で束ねた（`settle_expandable`）。宣言していない欄への接続と欄の違う expandable どうしの接続は、Rumoca と同じく断る（`expandable`）。
- **`Var` に `unused_expandable` を足した**（`globe-mode-flat` の公開の構造体。`Var` を作るのは `flatten.rs` の 2 か所だけ）。DAE は `Role::Known` にする。
- **`composite-binding` は 6 つの原因に分けて直した**（解説書 2.2）。
  - `split_mod` の「値が配列への参照なら要素ごとに添字を付ける」は、MSL の例題の束縛の文字列を変える（`c[n](x = u)` の `u` が `u[i]` になる）。名前・方程式の数には効かず、全体の検査で回帰 0 を確かめた。
  - `eval` が parameter の配列の要素 `c[k]` を評価するので、`Complex w[n] = {if c[k] then … }` の束縛の文字列の `if` が、選んだ枝だけになった（スカラーの parameter の `if` と同じ。Rumoca の flat も落とす）。試験 `complex_arrays_expand_elementwise` の期待を直した。
- **shortcut**: `anyTrue`・`isPowerOf2` は MSL の関数を名前で引いて決め打つ（関数の評価器ができたら置き換える。`numberOfSymmetricBaseSystems` と同じ）。`ref_paths` は引用符つきの名前と添字の中の参照を見ない。
- **直さないと決めたもの**: 方程式の数が合わない `CCCVcharging`・`CCCV_CellRC`（Rumoca が `resistor[cellData.nRC]`・`capacitor[cellData.nRC]` を 1 要素分しか数えない。差はちょうど 10 本）。`CCCV_Stack`・`CCCV_StackRC` は Rumoca の flat が通らない（`connect(gainV.u, stackBus.cellBus.v)` を「欄を足す接続」と見て断る。`rumoca compile --emit flat-mo` で確かめた）。

## やったこと

- `src/Rust/mod/globe-mode-flat/src/lib.rs`: `Var::unused_expandable`。
- `src/Rust/mod/globe-mode-flat/src/flatten.rs`: expandable（`exp_depth`・`exp_vars`・`conn_vars`・`pot_sets`・`exp_sets`・`settle_expandable`・`ref_paths`・`adds_member`）、`composite-binding`（`record_value` の配列の束縛の接尾辞、`eval` の鎖・添字つきの参照・`anyTrue`・`isPowerOf2`・`eval_items`・`eval_at`、`split_kind`（配列への参照）、`bind_dims`・`field_bind` の `[i,j].f`）、2 次元の `connect`（`expand_end`・`split_array_var`）、範囲の切り出しの `connect`（`conn_ends`）、試験 1 本。
- `src/Rust/mod/globe-mode-flat/tests/flat.rs`: 試験 1 本と、`complex_arrays_expand_elementwise` の期待。
- `src/Rust/mod/globe-mode-dae/src/lib.rs`: `unused_expandable` を `Role::Known`、`parts` が 2 次元の部品の配列を通した欄を行ごとに引く。
- 「native が断る例」の差し替え（7 か所。検証の 3 つ目）: `globe-mode-cli`・`globe-mode`・`globe-mode-wasm`・`globe-mode-lsp` の試験、`src/Python/tests/test_api.py`、`src/Svelte/tests/test_ui.py`。
- 文書: ADR-0039 付記（S367）と推移の表・案内の表、roadmap（要約）・architecture・README・CHANGELOG・features の数字、セッション文書（`doc/sessions/S367-flat-expandable/`）、索引。`doc/frontend-diff.md` は全体の検査が書き直した（手では触っていない）。

## 検証

- **全体の `diff-frontend`**（基準は S365 の JSON。写した release の `xtask-s367` を `flock` の下で）: MSL の例題 566 本で、平らにできた 548 → **561/566**、Rumoca も通った 521 → 532 本で名前 518/521 → **529/532**・方程式の数 508/521 → **517/532**、DAE にできた **561/561**、Rumoca の釣り合いまで通った 377 → **382** 本で判定 **382/382**・方程式と未知数の数 **381/382**・状態 **381/382**・構造の正則 **382/382**。`GlobeModeValidation` 75 本は変わらず（Rumoca も通った 74 本で 74/74・正則 73/73）。**回帰 0 本、改善 13 本**。1332 s、最大 RSS 5.8 GiB。方程式の数の食い違いは 13 → 15 本（増えた 2 本は `CCCVcharging`・`CCCV_CellRC`。Rumoca が配列 `resistor[cellData.nRC]`・`capacitor[cellData.nRC]` を 1 要素分しか数えない）。断る残りは `non-constant-dim` 4・`unsized-array` 1。
- `diff-frontend --class`（debug）: expandable の 5 本（`BusUsage`・`BusUsage_Utilities.Part`・`CCCV_Cell`・`OneAxis`・`FullRobot`）は全項目が一致（方程式の数・釣り合い・状態・正則）。`PolyphaseRectifier`・`HeatExchangerSimulation` は名前・方程式の数が一致。`CCCV_Stack`・`CCCV_StackRC` は自前の DAE が釣り合い（823 対 823・999 対 999）・正則（Rumoca の flat が通らない）。
- 試験: `cargo test --no-fail-fast -p globe-mode-flat -p globe-mode-dae -p globe-mode-frontend -p xtask -p globe-mode-db -p globe-mode-cli -p globe-mode -p globe-mode-lsp -p globe-mode-wasm`: 単体・結合の 577 本が合格。失敗 1 本は時間の試験 `time_reference_does_not_scan_every_variable`（他の担当と重なって負荷 14 で 4 s を超えた。負荷 6 で単独に回すと合格）。doctest は共有 target の古い rlib で `can't find crate`（E0463。変更と無関係）。新しい試験は 2 本（`flatten.rs` の `expandable_connector_drops_unused_members`、`flat.rs` の `record_arrays_bound_through_record_fields_and_2d_connector_arrays`。どちらも直す前なら断る）。
- **native が expandable を断らなくなったので、「native が断る例」に使っていた 7 か所を `model E extends Real; end E;`（`GM-U0006`: `extends-builtin`。Rumoca は通る）に替えた**: `globe-mode-cli/tests/check_native.rs`・`globe-mode/tests/check_with.rs`・`globe-mode-wasm/tests/decay.rs`・`globe-mode-lsp/src/lib.rs`（2 本）・`src/Python/tests/test_api.py`・`src/Svelte/tests/test_ui.py`。Rust の 5 つは合格を確かめた。Python と Svelte の試験は回していない（wheel・wasm・Playwright が要る。CI が回す。源の文字列を替えただけ）。最初に選んだ長い形の `redeclare model` は、自前の構文が `extends B(redeclare model M … end M)` を読めない（`GM-E0001`）ので使えなかった。
- `cargo fmt --all --check`・`cargo clippy -p globe-mode-flat -p globe-mode-dae -p globe-mode-frontend -p xtask --all-targets -- -D warnings`: 通過。
- `check_docs.py`: 全デッキ OK、相対リンク 5506 件中の切れ 0 件。`render_slides.py`（S367 のデッキだけ）: 問題 0 件。markdownlint-cli2 は入っていない（`npx` での取得は許可されなかった）ので、見出し・表・リスト・コードブロックの前後の空行を手で確かめた（0 件）。
- 共有 target の古い rlib（`globe-mode-flat` の `Var` にまだ `unused_expandable` が無い版）で `E0609` が 2 回出た。変えた crate の `lib.rs` を `touch -c` して直した。
- 全体の検査のあと、`globe-mode-flat` に挙動の同じ最適化（`conn_vars` は expandable があるときだけ集める）を 1 行足した。全体の検査のバイナリには入っていない（試験は通した）。
- セッション記録: 専用の記録が `log/local/` に無いので省いた。コミットの差分のメールアドレス・ホームのパス・トークンの検査: 0 件。

## 残したこと

- `non-constant-dim` の 4 本（FFT の 4 本）は `realFFTsamplePoints`（アルゴリズムのある関数）の評価が要る。関数の評価器ができたら。
- `unsized-array` の 1 本（`ReadRealMatrixFromFile`）は決められない。
- 方程式の数が合わない 2 本（`CCCVcharging`・`CCCV_CellRC`）と、Rumoca が通らない 2 本（`CCCV_Stack`・`CCCV_StackRC`）は Rumoca の側。上流に出すかは貯める（出さない）。
- expandable の欄を足す接続は断ったまま（MSL の例題に出ない）。
