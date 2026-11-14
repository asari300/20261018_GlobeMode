# S266 `fmu-dependencies`

| | |
| --- | --- |
| 日付 | 2026-11-15 |
| ブランチ | `feature/s266-fmu-dependencies`（`feature/s256-blt-view` から切った） |
| worktree | `.git-worktree/s266-fmu-dependencies` |
| Phase | P5 |
| セッション文書 | `doc/sessions/S266-fmu-dependencies/` |
| セッション記録 | 省略（この WP のセッション記録は main の `log/local/` に無い） |

## 依頼

Export した FMU の `modelDescription.xml` の `ModelStructure` に、出力・状態の導関数（と初期未知数）の `dependencies` を書く。S224 の持ち越し。BLT の解く系で推移的な依存を求め、新しいグラフ算法は最小にする。P5a に入れにくければ P5b だけでよい。

## 決めたこと

- **仕様**（[調査](../../doc/research/fmi-model-structure-dependencies-2026-11-15.md)）: 省略は「すべてに依存」。2.0 の既知は入力・連続状態・独立変数（定義が無ければ暗黙）、3.0 はパラメータも既知で全部列挙。よって 2.0 はパラメータを並べず、3.0 は並べる。
- **既知の値の置き場**: `DaeStructure` に入力・パラメータを足すと構築が 10 か所以上変わるので、`KnownRefs`（IR）を別の表として返す（`structure::extract_with_knowns`、`CompiledModel::dae_structure_with_knowns`）。
- **言い切れない参照は書かない**: 離散変数・`pre`・条件・遅れを参照するブロックと、それに依るブロック、インデックス 2 以上は `None`。
- **P5a も書く**: 当初は P5b だけでよい想定だったが、P5a は入力を持つモデルを出せない（既存の制約）一方、状態だけの依存は XML の後処理（`<DefaultExperiment>`・Enumeration と同じ流儀）で数行で足せたので足した。テンプレートは改変していない。
- **書かないもの**: `dependenciesKind`（省略可）、`InitialUnknown(s)` の依存（初期化の既知と初期方程式を別に解く必要があり、省略が仕様どおり安全）。依頼の「初期未知数」はここだけ未達。

## やったこと

- `globe-mode-ir`: `KnownRefs`。
- `globe-mode-frontend`: `structure.rs` が入力・パラメータ・`time`・言い切れない参照を行ごとに集める。
- `globe-mode-analysis`: `analyze_dependencies`（解く順の閉包）と `unknown_dependencies`（名前で引く）。単体試験 1 本（代数ループ・`time`・離散の `None`）。
- `globe-mode-fmi`: `ModelStructure::dependencies`、読み書き、`model_structure_dependencies`（変数の表への引き当て）、P5a の後処理 `add_dependencies`。
- `globe-mode-fmu`: P5b の `describe` が書く。
- 試験の更新: `fmpy.rs`（新 1 本 `model_description_writes_dependencies`、既存 2 本の期待値）、`fmu_export.rs`（P5a の 2.0・3.0 に 1 つずつ）。
- 文書: 調査、ADR-0031 の追記、roadmap・features、スライドと解説書。

## 検証

- `cargo test`（ir・analysis・frontend・fmi・fmu、cli の `fmu_export`・`fmu_worker`、facade の `fmu_p5b`）: すべて成功、失敗 0。FMPy の `validate_fmu` と simulate を含む（2.0・3.0、ME・CS・SE）。
- `cargo clippy -D warnings`（関係する 6 crate、`globe-mode-fmi/export`）: 警告 0。
- fmusim（`fmu-export.yml`）: 後述（締めのとき追記）。

## 残したこと

- `InitialUnknown(s)` の `dependencies`、`dependenciesKind`、離散変数を経由する依存の分解。
- 3.0 は、DAE にあって P5b の変数の表に無いパラメータ（畳み込まれたもの）を参照する未知数は書かない（安全側）。MSL の例題でどれだけ書けるかは数えていない。
