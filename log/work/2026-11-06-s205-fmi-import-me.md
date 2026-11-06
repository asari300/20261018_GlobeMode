# S205 `fmi-import-me`

| | |
| --- | --- |
| 日付 | 2026-11-06 |
| ブランチ | `feature/s205-fmi-import-me` |
| worktree | `.git-worktree/s205-fmi-import-me` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S205-fmi-import-me/` |
| セッション記録 | （親が写す） |

## 依頼

ADR-0026 (k) の 1 番目: FMI 2.0・3.0 の Model Exchange の Import（`FmuMe`）。FMU が出す微分を GlobeMode の側で積分し、Reference-FMUs で確かめる。やらないこと: 別プロセスでの読み込み、facade・UI・Python・C ABI、マスター、Integer 以降の型、SE、案 B、`fmu.rs` 以外の CLI、ほかの crate。

## 決めたこと

- 積分器は `globe-mode-fmi-import` の中（`ode.rs`）。RK4（固定刻み）と Dormand–Prince 5(4)（誤差制御、既定）。`globe-mode-sim`・`globe-mode-exec` には依存しない（Rumoca を背負わせない）。依存は 0 件。既存の crate の Dormand–Prince の係数は Rumoca / diffsol の内側で、使えなかった。
- ADR (d) の「`HybridSystem` を実装して全ソルバ」は、ここではやらない（依存が逆向きに太る）。アダプタは別の薄い層で後から。ADR の (l) に事実として書いた。
- state event は 3.0.2 §3.1.1 の定義（領域 `> 0` と `≤ 0` の変化。2.0 の疑似コードの符号の比較ではなく）を両方に使う。位置は、領域が変わった歩を二分法で詰める（試す時刻ごとに歩の頭から積分し直す）。
- 確かめる相手: 参照の CSV は CS の前進 Euler が作ったので、ME とはビット一致しない。BouncingBall は衝突の前後で `v` の符号が逆になるので点ごとの比較も意味がない。解析解（Dahlquist、BouncingBall）と、独立な積分どうしの一致（VanDerPol: RK4 と Dormand–Prince）を主にし、CSV は目安（BouncingBall は衝突の回数と、時刻の差が 0 以上 0.1 未満、VanDerPol は最初の 2 秒）。
- CLI の `--kind` の既定: CS を持てば CS（S204 の挙動を変えない）、無ければ ME。`--solver`・`--tolerance` は足さない（`DefaultExperiment` の `tolerance` と `stepSize` を既定に使う）。
- `DefaultExperiment` の `stepSize` は `globe-mode-fmi` が読まない。ほかの crate は触らない約束なので、`Fmu::open` が XML の文字列を走査して拾う（`shortcut:` コメント付き。`globe-mode-fmi` に足せば要らなくなる）。
- `abi2.rs`・`abi3.rs` は CS と ME で共通の `Common`（読み込み・インスタンス・初期化・get/set・Drop）に整理し、`Cs2`/`Cs3`/`Me2`/`Me3` はその上に載せた。S204 の挙動（CS の参照の CSV との全桁一致）は変えていない。

## やったこと

- `src/Rust/mod/globe-mode-fmi-import/src/me.rs`（`FmuMe`・`EventInfo`・`StepInfo`・`MeOptions`・`MeRun`・`run_me`、模擬 FMU の試験 2 本）、`ode.rs`（RK4・Dormand–Prince 5(4)、試験 4 本）、`abi2.rs`・`abi3.rs`（ME の関数、`unsafe` の境界は変わらずこの 2 ファイルだけ）、`lib.rs`（`Fmu::instantiate_me`、`default_step_size`）、`tests/model_exchange.rs`（7 本）。
- `src/Rust/mod/globe-mode-cli/src/fmu.rs`（`--kind`、`info` に連続状態と event indicator の数、ME の実行の後に標準エラーへ事象の数）と `tests/fmu_import.rs`（ME の試験 2 本と、往復の ME）。
- ADR-0026 に (l)、(k) の 1 番目を済みに。`doc/features.md`・`roadmap.md`・`architecture.md`・CHANGELOG、`fmu-import.yml` のコメント、スライドと解説書（2.0 と 3.0 の呼び出しの表。出典は v2.0.5 / v3.0.2 の adoc とヘッダ、3.0.2 の HTML。2.0.5 の HTML は取得できず節番号は adoc の並びから数えた【未確認】）。

## 検証

- `cargo test -p globe-mode-fmi-import`: 単体 6、model_exchange 7、reference 3、unpack 5 すべて OK。`cargo test -p globe-mode-cli --test fmu_import`: 6 OK（往復の ME: x(1) − e⁻¹ = 1.2e-9、2.0・3.0）。
- 突き合わせの結果（Linux）は解説書の表。BouncingBall の衝突 11 回の時刻の最大の誤差 3.2e-11、高さ 7.2e-12。
- `fmu-import.yml`（feature へ dispatch、run 42193986375）: ubuntu-24.04・windows-2025・macos-latest 3 OS とも success。突き合わせの数字は 3 OS で同じ（macOS の VanDerPol だけ 1e-15 ほどの相対差）。
- `check_docs.py`: スライド OK、リンク 切れ 0 件（S205 の追加後）。
- 全体の検査（`mise run check`）・`ci.yml` full の結果は下に追記する。

## 残したこと

- 陰的な積分（硬い系。`Roberts`〔3.0〕は試していない）、`Discard` での刻みの再試行、連続時間の入力、`HybridSystem` へのアダプタ、step event を出す FMU での確認（Reference-FMUs にはなく、模擬 FMU の試験だけ）。
- 次の WP は別プロセスのワーカーの中での読み込み（ADR-0026 (k) の 2）。`FmuMe` は `&mut dyn` で扱えるので、ワーカーはその外側で包めばよい。
