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
