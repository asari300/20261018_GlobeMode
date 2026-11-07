# ADR-0029 `external "builtin"` が組み込みの数学関数を呼ぶだけの関数は、Dae を書き換えて組み込み演算にする

- 状態: Accepted（2026-11-01、S068。手元の前後と CI（libraries.yml、run は [research](../research/external-builtin-2026-11-01.md)）で確かめた）
- 関係: [ADR-0002](0002-rumoca-adapter.md)（Rumoca は frontend と exec の中だけ）、[ADR-0013](0013-exec-backend.md)。Rumoca は改変しない。上流への報告の下書きは [upstream/16](../research/upstream/16-external-builtin-math-functions.md)

## 背景

- MSL 4.0.0 の `Modelica.Math` は `sin`・`cos`・`tan`・`asin`・`acos`・`atan`・`atan2`・`sinh`・`cosh`・`tanh`・`exp`・`log`・`log10` の 13 個を `external "builtin" y = sin(u);` と書く（4.1.0 は `algorithm y := .sin(u);`）。
- Rumoca の Dae は `external "builtin"` を言語 `Builtin`・記号 `sin` として持つが、どの実行の道（構造解析の射影、solve-IR の lowering）も実行を断る（`EL005` / `EX002`、`external builtin function … calls 'sin', which this runtime cannot execute`）。GlobeMode では `GM-U0004`。
- 公開ライブラリ 7 つを MSL 4.0.0 で回すと、funnel で OK の例題のうち 70 本（Buildings 27・AixLib 26・IBPSA 19・DeltaRobot 3 のうちの重なりを除く数は S067 の記録）がこれだけで実行できない（S067、run 41295445820）。
- 組み込みの先は Rumoca の Dae に既にある（`PureBuiltin` に `Sin`・`Cos`・…・`Log10`・`Sqrt`・`Abs`）。

## 決定

### 道の選び方（小さい順に調べ、採れた最初のものを採った）

- (a) Rumoca の公開 API に external builtin を組み込み演算に写す設定や hook: **無い**（`rumoca-ir-dae/src/model/external_functions.rs` は言語を分類するだけ、`rumoca-eval-dae/src/numeric.rs` の `external_function_failure` が全ての external を断る。確認日 2026-11-01、rev `276e82bd`）。採れない。
- (b) Dae を書き換える: `Dae` は不変で、構築 API は借用の印（ブランド）付きの閉包（`Dae::construct`）だけなので、公開の型を直接つなぎ替えることはできない。ただし `Dae` は `Serialize` と `Deserialize` を実装し、読み戻すときに Rumoca が構築と同じ検査をやり直す（`model/wire.rs`）。**直列化の形（serde_json の値）の上で書き換えて読み戻す**なら公開 API だけで成り立つ。**採る**。
- (c) frontend で MSL 4.0.0 の `Math/package.mo` の文字列を置換: 採らない（版と文字列に結びつき、ほかのライブラリの同じ書き方に効かない）。

### (b) の中身

- 場所は `globe-mode-frontend/src/external_builtin.rs`。`compile_strict`（`CompiledModel` を作る唯一の場所）が、Rumoca の Dae を `lower_builtin_externals` に通してから持つ。`check`（S0〜S10 の判定）は触らないので、判定の数は変わらない。
- 関数ごとに、定義の external 節から次をすべて満たすかを判定する（名前の固定表は組み込みの名前と引数の数だけ。`Modelica.Math.sin` という名前は見ない）: 言語 `builtin`、記号が `sin cos tan asin acos atan atan2 sinh cosh tanh exp log log10 sqrt abs` のどれか、本体の文が空、入力が引数の数と一致、出力が 1 つ、external の引数がすべて入力で、番号が増える順（`atan2(u2, u1)` のような入れ替えを弾く。式の番号は Rumoca の内部の番号とずれるので、入力そのものかまでは確かめない）。
- 満たす関数の `call` ノードを、同じ被演算子の `builtin` ノードに替える（被演算子は詰めた配列に順に入っているので、数が同じなら並びは変わらない）。関数の定義は残す（呼ばれないので害は無い）。
- 条件に合わない external（`"C"`、別の記号、引数が入力そのままでないもの）は触らない。今まで通り `GM-U0004`。
- 書き換えた Dae を Rumoca が受け付けなければ、元の Dae を返す（`GM-U0004` は今まで通り出る）。external の関数が無い Dae は直列化しない（費用なし）。

## 結果

- 検証用モデル `GlobeModeValidation.Simulation.ExternalBuiltin`（`external "builtin"` の自前の関数 `sin`・`exp`）: 直す前は `GM-U0004`、直した後は解析解（`1 - cos 1`・`e`）と bdf・dopri45 で一致する。
- 数の前後は [research/external-builtin-2026-11-01.md](../research/external-builtin-2026-11-01.md)。
- **限界**は次のとおり。
  - serde の形は Rumoca の公開契約ではない。rev を上げたら `external_builtin.rs` の試験（`ExternalBuiltin`）で確かめる。形が変われば書き換えは何もせず（元の Dae を返す）、`GM-U0004` に戻るだけで、誤った結果にはならない。
  - 大きな Dae は、external builtin を含むときだけ直列化と再構築の費用がかかる（MSL 4.0.0 で `Modelica.Math` に触れるモデルだけ）。
  - `external_tables`（読み込み済みの CombiTable の表）は直列化の形に入っていて読み戻せない（`deny_unknown_fields`）ので、外してから読み戻し、読み戻した Dae に `with_external_tables` で付け直す。最初の版は表を持つ Dae を書き換えなかったため、AixLib の 6 本ほど（`HumansSenTDep` など）が直らなかった。
  - 直列化の式の番号（`arguments` の `input`）は、Rumoca の内部の式の番号とずれることがある（関数の入力の式の番号が、直列化の配列の並びの位置と一致しない例: AixLib の `Declination`）。最初の版は番号で式を引いて確かめたため、そのモデルで書き換えが何もしなかった。今は番号の大小だけを見る。
  - `derivative` 注釈を持つ call は組み込みに替わるので、導関数の注釈は使われない（組み込みの導関数は Rumoca が知っている）。
  - Rumoca 側で直る（上流の下書き 16）か、rev を上げて 4.0.0 のモデルが同じ結果なら、この変換は外してよい。
