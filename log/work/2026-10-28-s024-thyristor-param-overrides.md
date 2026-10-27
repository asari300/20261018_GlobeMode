# S024 `thyristor-param-overrides`

| | |
| --- | --- |
| 日付 | 2026-10-28 |
| ブランチ | `feature/s024-thyristor-param-overrides` |
| worktree | `.git-worktree/s024-thyristor-param-overrides` |
| Phase | P3 |
| セッション文書 | `doc/sessions/S024-thyristor-param-overrides/` |
| セッション記録 | `log/sessions/2026-10-21-40b9a39f-6e5f-4daa-8928-20b6496f2093.md` |

## 依頼

「進めて。サブエージェントはSonnet/high」。S023 の残しから、esdirk34 の `ThyristorBehaviourTest`（消弧の逆回復で射影が発散）と、`--param` で時間イベントの時刻が変わらない件の手元での直し方の見積もりを選んだ。

## 決めたこと

- サブエージェント（Sonnet、effort high）を 3 本使った。T: `ThyristorBehaviourTest` の計測（`rk.rs` の一時的な計測は最後に外す）。O: `--param` の直し方の見積もり（読むだけ）。I: O の案 (a) の実装（`doc/` は触らない）。
- **`--param` は上書きを組み込んでコンパイルし直す（ADR-0022）**。O の調査で、S023 の見立て（時間イベントの時刻の畳み込み）は根の原因ではなく、同じ名前の修飾（`SineVoltage V(f = 1)` の `signalSource(final f = f)`）で外側の値が内側の束縛に書き込まれるため、実行時の上書きが内側に届かないと分かった。`SineVoltage` の `--param V.f=2` で V.v(0.05) = 0.309（f = 1 のまま）になることを自分でも確かめた。黙って誤るので、警告だけ（依存が DAE に残らず見分けられない）や Rumoca の時間イベントの patch（効いていた形しか変えない）ではなく、ソースを書き換えたのと同じになる `set_structural_overrides` を選んだ。
- 一度コンパイルした session で `set_structural_overrides` を呼ぶと ED020 になるので、ライブラリだけの session（`pristine`）を複製して使う。型（Real・Int・Bool）は既定の DAE から読む。実行時の上書きもそのまま渡し、GM-E10006 の拒否と `SimRun.parameters` を保つ（変えられる名前の範囲は前と同じことを `Parameters` で確かめた）。組み込みが落ちたら診断を返し、既定に戻さない。
- **`ThyristorBehaviourTest` は直さない**。T の計測で、消弧の後の再開で esdirk34 の最初の刻み（4.05e-6）が大きく、Newton 法の試しの点（y1 が 0.006 → 8.32）で host の射影が失敗し、復帰不能として記憶されると分かった（候補 13 と同じ機構）。再開の刻みを 0.9 倍で通り 1 倍で落ちる崖の調整で、通しても Hermite の補間が硬い変数で外れて管に収まらない見込みなので、上流の候補 19・20 にした。
- `msl:sim` は回さない。上書きの無い実行は変わらず（空の上書きは既存の道をそのまま通る）、実行系も変えていない。フロントエンドを変えたので `msl:compare` を回した。

## やったこと

- `src/Rust/mod/globe-mode-frontend/src/rumoca.rs`: `Checker::compile_retaining_with_overrides`（複製した `pristine` に組み込んでコンパイル）。変換と `CompiledModel` の組み立てを `compile_strict` に括り出して共用。
- `src/Rust/mod/globe-mode-db/src/lib.rs`: `GlobeDb::compiled_model_with_overrides`（`compiled_model` は空の上書きで呼ぶ）。
- `src/Rust/mod/globe-mode-cli/src/simulate.rs`・`src/Rust/mod/globe-mode-wasm/src/lib.rs`: 上書きを渡す。
- `src/Modelica/GlobeModeValidation/Simulation/InlinedParameters.mo`・`package.order`: 同じ名前の修飾の最小の再現（Integer・Boolean のパラメータも）。
- `src/Rust/mod/globe-mode-exec/tests/simulate.rs`: `structural_overrides_reach_bindings_inlined_by_same_name_modifications`。
- `doc/adr/0022-structural-parameter-overrides.md`（新規）と索引、`doc/architecture.md`・`doc/features.md`・`README.md`（`--param` の説明）、`doc/research/msl-sim-failures.md`（「S024 で直したもの」、候補 16 を改め 17〜20 を足した）、`doc/roadmap.md`、`doc/sessions/S024-thyristor-param-overrides/`、索引。

## 検証

- `SineVoltage` に `--param V.f=2`（bdf、出力の間隔 0.05）: V.v(0.05) = 0.309 → 0.5878、V.v(0.1) = 0.9511。上書きなしは 0.309・0.5878 で変わらない。`--param R.R=2` は R.i(0.05) = 0.1545 で前と同じ。
- `CauerLowPassSC` に `--param V.startTime=0.5 --stop-time 0.7`: V.v は前は全区間 0、直した後は t = 0.45 まで 0・0.5 から 1。
- `InlinedParameters`（CLI）: `src.startTime=0.5` で src.y(1) = 0.5、`n=3` で z = 3。wasm（Node の `simulate.mjs`）でも src.y(1) = 0.5（上書きなしは 0）。`Parameters` の `n`・`e`・`f`・`g` は前と同じく GM-E10006。
- 費用（I の計測、`CauerLowPassSC`）: 複製と文書の追加 0.4 s・コンパイル約 2 s、CPU 4.2 → 6.6 s、最大 RSS 1.58 → 2.0〜2.1 GB（WSL では swap に入り壁時計 7 s → 16〜26 s）。上書きの無い実行は変わらない。
- `ThyristorBehaviourTest`（T の計測）: 再開の刻みの倍率 1 で落ち、0.9・0.75・0.5 以下で通る。イベントの前の刻みの 4 倍で抑えても通り、その場合 `OvervoltageProtection`・`HeatingRectifier`・`CauerLowPassSC` の出力は変わらない（入れていない）。
- `msl:compare`: 20 本中 20 本が一致（両方とも成功 17 本）。
- `wasm:node`: 通った。`wasm:compare`: 153 件中 150 件が一致・既知の差 3・一致しない 0。
- コードの検査（fmt / clippy / test / deny）: `mise run check` で 492 本通過・0 失敗・10 ignore、fmt・clippy・deny OK（47 分）
- `check_docs.py`: 全デッキ OK / 文書 138 件・相対リンク 868 件中の切れ 0 件。`render_slides.py` 30 デッキ・問題 0 件（S04・S05 を目で確かめた）
- セッション記録の `--check`: 検査 9 件・取りこぼし 0 件
- ステージした差分の漏洩検査: 0 件

## 残したこと

- 上流への報告の候補 17〜20（ユーザーが出す）。
- 配列の要素のパラメータ（`a[1]`）の上書きは型の表に無く、実行時の上書きだけになる（確かめていない）。
- 上書きのある実行は 3 GB の WSL では swap に入る。wasm での費用は測っていない。
- `msl:sim`・`msl:funnel` は回していない（上書きの無い道と検査の段は変えていない）。
