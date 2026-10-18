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
    - 必要なファイルだけに絞る `deps_closure` は、まだ作っていない（S006 の候補）。→ S006 で作った（下）。
  - **防火壁 2 は `dae_shape`**（位置を落とした `DaeStructure`。決定の `dae_structure` にあたる）。S6 の `structural` はこれだけを読む。
    - パラメータの値を同じ長さで変えたときは、`check` は再実行されるが、結果が前と等しい。そのため `dae_shape` は再実行されない。
    - 値の長さが変わると、後ろの方程式の位置がずれて `check` の結果は変わる。それでも `dae_shape` は前と等しいので、`structural` は再実行されない。
  - **S6 は `globe-mode-analysis`**（Rumoca に依存しない）。判定は `x` と `der(x)` を 1 つの列にまとめた二部グラフに完全マッチングがあるかどうか。Pantelides 法が失敗するのと同じ条件。
    - 最初は der の形（未知数は `der(x)`・代数・出力。Rumoca の構造解析と同じ）で判定していた。これだと MSL の正しい例題 166 本が構造特異になった。微分インデックスが 1 より大きい DAE では、der の形に完全マッチングが無いのが正常なため。
    - der の形の結果は、S7 の情報 `GM-I7001`（微分インデックスが 1 より大きい見込み）として出す。→ S006 で、S7 は Pantelides 法の `IndexReport` に一本化した（[ADR-0008](0008-structural-index-and-init.md)）。
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

- **S006 で、`check` に渡すファイルを `deps_closure` で絞り、S7・S8 をクエリに載せた**（2026-10-18）。

  ```text
  SourceFile ─┬─▶ outline ─▶ outline_shape ─┬─▶ class_index ─▶ connect_graph           （S004）
              │              防火壁 1       └─▶ defined_classes ─▶ class_files ─┐
              │                                                                 ▼
              └─▶ shape ─┬─▶ source_shape ─┬─▶ identifiers ──────────────▶ deps_closure（S006）
                         │   防火壁 0      │                                    │ 閉包のファイルだけ
                         └─▶ shape_map     └─────────────────────────────────▶ check（S0〜S5）
                                                                                ├─▶ flat_summary
                                                                                ├─▶ dae_shape ─┬─▶ structural（S6）
                                                                                │   防火壁 2   └─▶ index_report（S7）
                                                                                │                         │ dynamic_states
                                                                                └─▶ init_shape ───────────┴─▶ init_report（S8）
  ```

  - **`deps_closure(workspace, request)`** は、`check` に渡すファイル（パスの昇順）を返す。
    1. モデルの完全修飾名を定義するファイルを起点にする（`Outline::defines`。`within` を前に付けて比べる）。
    2. 構文の誤りのあるファイルは、いつも入れる。何を定義しているか分からないため（S0 で止まる）。
    3. 閉包の中のファイルの `identifiers` にある名前ごとに、その名前のクラスをトップレベルに定義するファイル（`class_files`）を足す。これを固定点まで繰り返す。
  - **`identifiers(file)`**: `source_shape` に現れる識別子の集合。キーワードも残す。文字列 `"…"` の中は除く（説明文や `Documentation` の HTML がクラス名に当たらないように）。引用識別子は引用符ごとの形と中身の両方を入れる。数（`1e5`）の中の英字は除く。
  - **`defined_classes(file)`**: `outline_shape` から、ファイルが定義する**トップレベル**のクラスの単純名。防火壁 1 の下流なので、宣言の変わらない編集では値が変わらない。
  - **`class_files(workspace)`**: 単純名 → 定義するファイルの表と、構文の誤りのあるファイルの一覧。どのモデルの閉包からも共有される。
  - **単純名の一致で、広めに見積もる**。アウトライン（extends・部品の型・import）だけでは、式の中の関数・定数・redeclare の参照を拾えないため。余計なファイルが入ることはあるが、足りないことは無い（別のトップレベルのクラスを参照するときは、必ずその名前を綴るため）。
  - **入れ子のクラスは `defined_classes` に入れない**。
    - 入れると、`Components.mo` の `replaceable package Medium` と `MediumFlow.mo` の入れ子の `Medium` のような、ありふれた同名のクラスで無関係のファイルが入ってしまう。
    - トップレベルだけで足りる根拠: ほかのファイルから入れ子のクラス `T.N` に届く道（修飾名 `T.N`、`within …T…;` の字句的なスコープ、`import …T…`、`T` を継承したクラスを経由）には、どれも `T` か、`T` を名指すファイルが現れる。固定点で `T` のファイルが入る（`defined_classes` の doc コメント）。
  - **モデルがワークスペースに無い**（MSL などライブラリの中のモデル、存在しないモデル）ときは、閉包は空で、文書を 1 つも渡さない。「見つからない」の診断には、ワークスペースのモデルを候補として足す（Rumoca は渡した文書の中からしか候補を出さないため）。
  - IR の `ClassOutline` に `imports` を足した（`import A.B.C;`、`import D = A.B.C;`、`import A.B.*;`、`import A.B.{C,D};`）。今は保持するだけで、`deps_closure` は本文の識別子で import の名前も拾う。
  - **S7・S8 のクエリ**（判定は [ADR-0008](0008-structural-index-and-init.md)）:
    - `index_report(workspace, request)`: `dae_shape`（防火壁 2）→ `analyze_index`。
    - `init_shape(workspace, request)`: `check` の結果の初期化系の構造から位置を落とした写し（防火壁）。
    - `init_report(workspace, request)`: `init_shape` と `index_report` の `dynamic_states` → `analyze_init_with_index`。
    - `GlobeDb::check_model` は S0〜S8 をまとめる。S7 は S6 が regular のときだけ、S8 は S0〜S7 に Error が無いときだけ回す。結果の `FullReport` に `index` と `init` を足した。
- **S006 のテストで確かめた**（salsa の `WillExecute` イベントで数えた。検証用パッケージ `GlobeModeValidation` の 23 ファイル）:
  - `deps_closure(GlobeModeValidation.Connect.RealCircuit)` は 5 / 23 ファイル（`package.mo`、`Connectors.mo`、`Components.mo`、`Connect/package.mo`、`Connect/RealCircuit.mo`）。
  - **閉包だけを渡した検査と、全ファイルを渡した検査の結果が、索引のモデルとブロック 20 本以上のすべてで一致した**（足りないファイルが無いことの確かめ）。
  - 使わないファイル（`Invalid/StructurallySingular.mo`）の式を変えても、`source_shape`・`deps_closure`・`check`・`structural` は **0 回**。
  - 閉包の中のファイル（`Components.mo`）のパラメータの値を変えると、`check` は 1 回。識別子は変わらないので、閉包は前と等しい。
  - パラメータの値の編集では、`index_report` と `init_report` は再実行されない（`check` は 1 回）。
- **S006 の計測**（2026-10-18、release、MSL 4.1.0、各 5 回。`cargo test --release -p globe-mode-db --test deps -- --ignored --nocapture`）:

  | 場合 | 中央値 | 最大 |
  | --- | ---: | ---: |
  | MSL の読み込み | 3822 ms（1 回目は 13096 ms） | – |
  | (a) `Connect.MediumFlow` の最初の検査 | 1963 ms | – |
  | (b) 使わないファイルの式を変えた後の再検査（`check` 0 回） | **0.19 ms** | 0.35 ms |
  | (c) 閉包の中のファイル（`Components.mo`）を変えた後の再検査 | 1815 ms | 1879 ms |
  | 参考: 全ファイルを渡して (c) と同じ編集（S005 の方式） | 1746 ms | 1856 ms |
  | (d) 編集の後に MediumFlow → RealCircuit | 1485 ms | 1543 ms |
  | (e) 編集なしで、閉包の違うモデルを初めて検査 | 1700 ms | 2851 ms |
  | 参考: 全ファイルを渡したまま (e) と同じモデル | 67 ms | 122 ms |

  - **読み方**: Rumoca は、渡す文書の組が変わるたびに、MSL ごと解決し直す（約 1.5〜1.8 s。[research/rumoca.md](../research/rumoca.md#s006-で調べた-apiv0102)）。本文の編集で文書を差し替えたときも同じことが起きる。
  - **得たもの**: 使わないファイルの編集のあとの再検査が、約 1.8 s（S005 の方式では、どのファイルの編集でも `check` が走る）から 0.2 ms になった。
  - **失ったもの**: 閉包の違うモデルへ初めて切り替えると、67 ms から約 1.7 s になった。文書の組が変わるため。2 回目からは salsa が結果を覚えている。
  - **トレードオフとして受け入れる**。文書の組が変わったときの再 resolve は、本文の編集で文書を差し替えたときにも起きる。モデルの切り替えで新しい種類の費用が増えるわけではない（作業記録の「決めたこと」）。
  - 切り替えの費用を和らげる案（`Checker` に文書の和集合を持たせ、組が変わらないようにする、など）は S007 の候補にした。

- P2 の完了条件: 空白だけの編集の再検査が 50 ms 未満で、`flat` が再実行されないこと。salsa のイベントで確かめる。
- S5 は `omc checkModel()` と照合して正しさを確かめる。
