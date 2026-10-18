# ADR-0005 salsa による遅延評価とシミュレーション前のアサーション

- 状態: Accepted（2026-10-18、S001）

## 背景

- 要件は2つある。
  - 遅延評価で処理を軽くすること。
  - シミュレーションできるモデルかを実行時にアサートすること。
- Rumoca には salsa のようなインクリメンタル計算の基盤が無い。
- IDE では、打鍵ごとに MSL 全体を flatten し直すわけにはいかない。

## 決定

- Phase A から **salsa** のクエリ DB（`globe-mode-db`）を置き、Rumoca の各段階の出力をクエリとして覚えておく。
- Eq で比較できる中立 IR を**防火壁**にする。
  - `outline`: 空白やコメントだけの編集では、ここから先が再計算されない。
  - `dae_structure`: ここが変わらなければ、検査と実行形式は作り直さない。
- MSL は必要になったファイルだけ parse する（`class_index`）。connect グラフは flatten しなくても作れるようにする（`connect_graph`）。
- simulate と export の前に必ず `assert_simulatable`（S0〜S10）を走らせる。
  - Error があれば `--force` 無しでは実行しない。
  - 「コンパイラが未対応」は GM-U コードとして分ける。
- シミュレーションの実行は salsa の外のジョブにする。

## 結果

- **S004 で実装した**（`globe-mode-db`、salsa 0.28.5）。クエリの連鎖は次のとおり。

  ```text
  SourceFile → outline → outline_shape → class_index → connect_graph
               位置あり   位置なし（防火壁 1）   Workspace     GraphRequest
  ```

  - `outline_shape` は、位置（`TextRange`）をすべて 0 にした写し。
  - 空白やコメントだけの編集では前と等しくなるので、salsa が下流を再計算しない。
- **テストで確かめた**: 空白とコメントだけの編集では `outline` と `outline_shape` だけが再実行され、`class_index` と `connect_graph` は再実行されない（salsa の `WillExecute` イベントで数えた）。
  - 別のファイルのコメントの編集でも同じ。
  - connect を足すと `connect_graph` まで再実行される。
- **計測**: 空白だけの編集のあとの再計算は中央値 0.20 ms、最大 0.30 ms（リリースビルド、20 回）。P2 の目標（50 ms 未満）を満たす。
- `globe-mode-db` と `globe-mode-graph` は wasm32 向けにビルドできる（salsa を含む）。
- **S005 で、防火壁を 3 段にし、検査（S0〜S6）をクエリに載せた**（2026-10-18）。

  ```text
  SourceFile ─┬─▶ outline ─▶ outline_shape ─▶ class_index ─▶ connect_graph   （S004 のまま。IDE の機能）
              └─▶ shape ─┬─▶ source_shape ─▶ check ─┬─▶ flat_summary
                         │   防火壁 0        S0〜S5 └─▶ dae_shape ─▶ structural
                         └─▶ shape_map                 防火壁 2      S6
  ```

  - **防火壁 0 は `source_shape`**（決定を直した）。決定では `outline` を第 1 の防火壁としたが、Rumoca のコンパイルに渡せるのは本文であって、アウトラインではない。
    - 空白やコメントだけの編集でコンパイルを止めるには、本文そのものを正規化して Eq で比べるしかない。
    - `globe_mode_core::normalize::source_shape`: `//` と `/* */` を除き、空白と改行を 1 個の空白に畳む。文字列 `"…"` と引用識別子 `'…'` の中は触らない。
    - 位置の対応表（`shape_map`）は別のクエリにし、`check` の依存に入れない。診断の位置は `GlobeDb::check_model` が元の本文に戻す。
    - `outline_shape`（防火壁 1）は、索引と connect グラフの防火壁として残す。
  - **MSL は salsa の外の `Checker` で一度だけ読む**（`GlobeDb::with_libraries(paths)`）。
    - `Checker` は、MSL を読んだ Rumoca の `Session` を `Arc<Mutex<_>>` で持つ。`check` クエリは `Db::checker()` からこれを使う。
    - 文書は、本文が変わったものだけ `add_document` で差し替え、無くなったものは `remove_document` する。
    - panic で Mutex が毒されたら、ライブラリを読んだ直後の写し（pristine）に戻して続ける。
    - `reset()` でモデルごとのキャッシュを捨てられる（1 回 0.4 s。pristine の写しを持つ分のメモリは約 50 MB）。同じ Session で MSL の例題 566 本を続けて検査すると最大 RSS が 7.5 GiB になったので足したが、50 本ごとに戻しても 7.2 GiB だった。大部分は 1 本の大きなモデル（`Spice3BenchmarkFourBitBinaryAdder`、単独で 6.3 GiB）の分で、キャッシュの成長ではなかった。
  - **`check(workspace, CheckRequest{model})`** は `CheckReport` を返す。中身は S0〜S5 の診断、位置付きの `DaeStructure`、`FlatSummary`。
    - ワークスペースの全ファイルの `source_shape` を文書として渡す。どれかのファイルの中身（空白とコメント以外）が変われば、`check` は再実行される。
    - 必要なファイルだけに絞る `deps_closure` は、まだ作っていない（S006 の候補）。
  - **防火壁 2 は `dae_shape`**（位置を落とした `DaeStructure`。決定の `dae_structure` にあたる）。S6 の `structural` はこれだけを読む。
    - パラメータの値を同じ長さで変えたときは、`check` は再実行されるが、結果が前と等しい。そのため `dae_shape` は再実行されない。
    - 値の長さが変わると、後ろの方程式の位置がずれて `check` の結果は変わる。それでも `dae_shape` は前と等しいので、`structural` は再実行されない。
  - **S6 は `globe-mode-analysis`**（Rumoca に依存しない）。判定は `x` と `der(x)` を 1 つの列にまとめた二部グラフに完全マッチングがあるかどうか。Pantelides 法が失敗するのと同じ条件。
    - 最初は der の形（未知数は `der(x)`・代数・出力。Rumoca の構造解析と同じ）で判定していた。これだと MSL の正しい例題 166 本が構造特異になった。微分インデックスが 1 より大きい DAE では、der の形に完全マッチングが無いのが正常なため。
    - der の形の結果は、S7 の情報 `GM-I7001`（微分インデックスが 1 より大きい見込み）として出す。
- **S005 のテストで確かめた**（salsa の `WillExecute` イベントで数えた。検証用パッケージ `GlobeModeValidation` を使った）:

  | 編集 | `check` | `dae_shape` | `structural` |
  | --- | ---: | ---: | ---: |
  | 変更なしで、もう一度検査する | 0 | 0 | 0 |
  | `RealCircuit.mo` に空白とコメントを足す | **0** | 0 | 0 |
  | 別のファイル（`Connectors.mo`）の先頭と末尾にコメント | **0** | 0 | 0 |
  | パラメータの値を同じ長さで変える | 1 | 0 | 0 |
  | パラメータの値を長さの違う値に変える | 1 | 1 | **0** |
  | 部品と connect を足す | 1 | 1 | 1（方程式 5 → 7） |

- **S005 の計測**（2026-10-18）:
  - 空白だけの編集のあとの再検査（`check_model`、S0〜S6）: 中央値 **0.01 ms**、最大 0.02 ms（リリースビルド、20 回、MSL を使わない `RealCircuit`）。最初の検査は 17.2 ms。P2 の目標（50 ms 未満で、`flat` が再実行されない）を、検査まで含めて満たした。
  - MSL の読み込みは 6〜16 s、1.24 GB。読んだあとの MSL の別のモデルの検査は 0.12 s、同じモデルの再検査は 0.08 s。
  - リリースビルドの `globe check -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit`（S6 を含む 1 回限りの CLI）は 11.3 s、1.5 GB。S6 は完全マッチング 44 / 44。

- P2 の完了条件: 空白だけの編集の再検査が 50 ms 未満で、`flat` が再実行されないこと。salsa のイベントで確かめる。
- S5 は `omc checkModel()` と照合して正しさを確かめる。
