# ADR-0002 Rumoca をアダプタ経由の git 依存で使う

- 状態: Accepted（2026-10-18、S001）

## 背景

- Rumoca（Apache-2.0）は想定以上に成熟している。
  - MSL 4.1.0 の例題では parse 566、simulate 153 まで通る。
  - LSP、WASM、FMI 2 / 3 のテンプレートも持っている。
- 一方で、使うには難点がある。
  - nightly を固定している。
  - crates.io 版は 0.7.28 で止まっている。
  - 開発が速く、API が頻繁に変わる。
  - salsa のようなインクリメンタル計算の基盤が無い。

## 決定

- **Phase A**: Rumoca を git の `rev` 固定で依存させ、`globe-mode-frontend` と `globe-mode-exec` の中に閉じ込める。
  - umbrella crate `rumoca` は使わない。zenoh などを引き込むため。
  - 依存するのは `rumoca-compile`、`rumoca-phase-codegen`、`rumoca-sim`（必要な feature だけ）。
- Rumoca の型は中立 IR（`globe-mode-ir`）に変換してから外に出す。
- **Track B**: 余裕ができたら salsa と rowan で自前のコアを作り、`Frontend` の feature `native` として置き換える。Rumoca は差分テストの基準として残す。
- Rumoca の不具合や不足は、できるだけ上流に PR を出して直す。

## 結果

- **S002 で実装した**（2026-10-18）。
  - `rev` は v0.10.2（`276e82bd`）。依存するのは `rumoca-compile` だけ（`default-features = false`）。
  - アダプタは `globe-mode-frontend` の `rumoca.rs` 1 ファイルに閉じ込めた。外には中立 IR（`Outline`、`BalanceCount`）と GlobeMode の診断だけを出す。
  - Rumoca の失敗段階と GM コードの対応:

    | Rumoca | GlobeMode |
    | --- | --- |
    | 構文の誤り | S0 / GM-E0001 |
    | Typecheck | S2 / GM-E2001 |
    | Instantiate | S3 / GM-E3001 |
    | NeedsInner | S3 / GM-E3002 |
    | Flatten | S4 / GM-E4001 |
    | ToDae | S5 / GM-E5001 |
    | ToDae のうち ED001（数が合わない） | S5 / GM-E5002 |

  - Rumoca のコードは、診断の `notes` に残す（例: `rumoca: ED001`）。
- Rumoca 経由で、保守終了の勧告が出ている crate が 2 つ入る。`deny.toml` で理由を書いて例外にし、`rev` を上げるたびに見直す。
  - bincode 1.x（RUSTSEC-2025-0141）
  - paste（RUSTSEC-2024-0436、faer 経由）
- git 依存を持つので、GlobeMode の crate は crates.io に公開できない（`publish = false`）。
- **S003 で改めたこと**（2026-10-18）:
  - コンパイルを `compile_model_phases` から `compile_model_strict` に替えた（Rumoca の CLI と同じ経路）。前者は、MSL を解決するときの警告（WR006 など）も失敗として返していた。
  - ライブラリ（MSL）は `load_source_root_tolerant`（`DurableExternal`）で読む。読めなければ S1 / GM-E1003。
  - 数え方を 2 つ返す。`BalanceCount` は配列をスカラーに展開した数（S5 の釣り合い）。`DaeCounts` は DAE の変数を配列ごとに 1 個と数えた数（Rumoca の CLI の `-v` と同じ粒度）。
  - **MSL 4.1.0 の例題 20 本で、Rumoca の CLI と 20 本すべて一致した**（[報告](../sessions/S003-msl-check/msl-compare.md)）。両方成功が 17 本、両方同じ理由で失敗が 3 本。

- P1 で stable でビルドできるか確かめる必要がある（[ADR-0003](0003-toolchain.md)）。
- 月に1回 `xtask bump-rumoca` で更新し、MSL のファネルの差分を記録する。
- 配布物には Rumoca のライセンス文を同梱する。改変した場合は Apache-2.0 §4b に従って明記する。製品名に「Rumoca」を使わない。
- **S004 で分かった Rumoca v0.10.2 の制約**（2026-10-18）: ライブラリ（source root）として読んだクラスが、別のライブラリ（MSL）のクラスを extends すると解決できない（ER003）。
  - 例: `replaceable package Medium = Modelica.Media.Interfaces.PartialMedium` をライブラリの中に書いた場合。
  - コンパイルする文書そのものに書いた参照は解決できる。単一ファイルのパッケージでも、複数ファイルのパッケージでも同じだった。
  - **対応**: 作業中のパッケージは文書として渡し（`globe check <DIR>` はディレクトリの下の `.mo` を文書として読む）、`--library` には MSL などの外部ライブラリだけを渡す。IDE としても、作業中のファイルはワークスペース（文書）として扱うのが自然。
  - 上流への報告は P2 の中で行う。
- **S005 で改めたこと**（2026-10-18）:
  - 依存に `rumoca-ir-dae`・`rumoca-eval-dae`・`rumoca-core` を足した（同じ rev、Apache-2.0、`globe-mode-frontend` だけ）。
    - DAE の incidence（方程式 × 未知数）を自前で作るため。Rumoca の `build_incidence` は `pub(crate)` で、外から使えない。
    - いずれも `rumoca-compile` の推移的依存なので、外部の依存は増えない。
    - cargo-deny の `wrappers` には書けない。Rumoca の内部の crate からも使われるので、書くとその依存まで違反になる。GlobeMode 側で直接依存するのが frontend だけであることは、Cargo.toml のレビューで守る（理由は `deny.toml` に書いた）。
  - **`Checker`** を足した。ライブラリを一度だけ読んだ Rumoca の `Session` を、モデルの検査のたびに使い回す（[ADR-0005](0005-lazy-query-db.md) の結果）。
    - `Frontend::check` は、`Checker` を作って 1 回だけ呼ぶ薄い包みにした（既存のテストはそのまま通る）。
    - `Checker::reference_structure` は Rumoca 自身の `phase_structural::analyze` を呼ぶ。自前の S6 と突き合わせるためだけに使う。
  - アダプタを 3 つのファイルに分けた。
    - `rumoca.rs`: `Checker` と診断の変換。
    - `structure.rs`: DAE から `DaeStructure` を取り出す。`for_each_scalar_coordinate` で、方程式ごとに参照するスカラーを列挙する。未知数は Rumoca の構造解析と同じく、状態ごとの `der(x)` と代数・出力のスカラー。状態そのものの参照は `state_refs` に分けて残す。
    - `unsupported.rs`: GM-U の振り分け（[ADR-0007](0007-unsupported-classification.md)）。
  - **S1〜S5 の診断に位置を付けた**。Rumoca の失敗の記録の主なラベル（`primary_label` の span）を、ソースの対応表でファイルと範囲に直す。
  - S5 で数が合わない（ED001）ときは、flat モデルの部品ごとの方程式数（`FlatSummary`）を `notes` に足す。DAE に変換できなくても、flatten まで通れば flat モデルの要約を取る。
  - 段階と GM コードの対応に、次を足した。

    | Rumoca | GlobeMode |
    | --- | --- |
    | 「未対応の言語機能・実装の制限」と読めるコード（ED019 など） | 段階はそのまま / GM-U0001 |
    | 「コンパイラの内部エラー」と読めるコード（ED008 など） | 段階はそのまま / GM-U0002 |
    | 既知の制約の一覧に当たった（モデル, コード）の組（ED001 など） | 段階はそのまま / GM-U0003 |
    | （DAE まで通ったもの）`DaeStructure` の解析 | S6 / GM-E6001、S7 の情報 / GM-I7001（`globe-mode-analysis`、Rumoca に依存しない） |

  - 自前の der の形の最大マッチングは、MSL の全例題の最初の実行で、Rumoca の `analyze` と 381 本中 381 本が一致した（[msl-funnel.md](../sessions/S005-check-queries/msl-funnel.md)）。
  - MSL の例題 20 本での Rumoca の CLI との突き合わせ（`cargo xtask msl-compare`）: 20 本中 20 本が一致（両方とも成功 17 本。CoupledClutches と Friction の ED001 は GM-U0003 として表示されるが、成否の判定は同じ）。
