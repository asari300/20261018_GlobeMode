# ADR-0026 FMI Import の置き方: `modelDescription.xml` は自前の読み取り、共有ライブラリの読み込みは Desktop・CLI の別プロセスに限り、ME は GlobeMode のソルバで積分する

- 状態: **Proposed**（2026-11-01、S057。(b) は案が割れるので、ユーザーが決める）
- 関係: [ADR-0013](0013-exec-backend.md)（Rumoca の実行系は Modelica のモデル用。FMU の driver は P5 で自前）、[ADR-0014](0014-session-route-wasm.md)（Rumoca の FMI ME 風の host に積分器を差す道）、[ADR-0023](0023-desktop-and-web-process-layout.md)（検査と実行は別プロセスのワーカー）。[architecture.md §3.4](../architecture.md#34-fmi-20--30) の「`fmi` 0.8 と `fmi-schema` の上に作る」を、(b) の決定に応じて直す
- 調査: [research/fmi-2026-11-01.md](../research/fmi-2026-11-01.md)

## 背景

- P5 は「FMI Import（ME / CS / SE）とマスター。Export は後」。完了条件は「Reference-FMUs が linux64 と win64 で一致する」（[roadmap.md](../roadmap.md)）。architecture.md は Import を jondo2010 の `fmi` 0.8 と `fmi-schema` の上に作ると書いていた（S002 の調査）。
- S057 の調査で分かったこと（出典と確認日は調査の文書）:
  - FMI のヘッダと XSD、Reference-FMUs は **2-Clause BSD**。仕様書の文書だけ CC BY-SA 4.0。ライセンスの障害は無い。
  - Rust の FMI crate は 3 系統ある。`fmi` 0.8.0 系（MIT OR Apache-2.0。`fmi-sys` の build に bindgen、`fmi-schema` は `hard-xml` など 42 パッケージ）、**`fmi-rs` 0.3.4**（BSD-2-Clause。2026-10-29 公開で若い。公式の fmusim の基盤。build.rs が C をコンパイルし wasm32 で落ちた。`colored` が MPL-2.0）、`fmu-runner` 0.4.4（更新が止まっている）。
  - 検証の道具は fmusim（Rust 製、`fmi-rs` 基盤）と FMPy（Python）が生きていて、fmuCheck は FMI 1/2 のみで開発終了。
  - FMU のネイティブコードは任意のコードで、落ちる（segfault）ことも、止まらないこともある。ADR-0023 の別プロセスのワーカーと相性が良い。
  - Rumoca の FMI ME 風の host は Modelica のモデルを回すもので、外の FMU を読む道ではない。

## 決定

### (a) XML の読み取りは自前で、`roxmltree` を使う（S057 で済み）

- 新しい crate `globe-mode-fmi`。`parse_model_description`（FMI 2.0 と 3.0 の `modelDescription.xml` → `ModelDescription`。変数表と ME / CS / SE の有無）。依存は `roxmltree` 0.21（MIT OR Apache-2.0、依存は `memchr` だけ）の 1 つ。wasm32 でも通る。
- ブラウザでも「FMU の中身を見る」ことはできる（共有ライブラリは読まない）。

### (b) 共有ライブラリの読み込みと C ABI: 案を 2 つ示す

| | 案 A（推奨）: 自前 | 案 B: `fmi` 0.8 の Import を使う |
| --- | --- | --- |
| 中身 | `libloading`（ISC。Cargo.lock に 0.8.9 が既にある）で `.so` / `.dll` / `.dylib` を読み、FMI 2.0・3.0 の ME・CS に要る関数の型を Rust で手書きする（BSD-2 のヘッダを見て） | `fmi`・`fmi-schema`・`fmi-sys` を依存に足し、`Fmi2Import` / `Fmi3Import` で Import する |
| 依存の増え方 | `libloading` 0.9 と `zip` 9（xtask に既にある版）。bindgen を通常のビルドに入れない | `fmi` 系と `hard-xml`・`chrono`・`itertools`・`lenient_semver`・`tempfile` など。`fmi-sys` の build に bindgen と libclang |
| GlobeMode の driver との合い方 | `HybridSystem` を直接実装できる。ME の `GetDerivatives` / `GetEventIndicators` を RHS にし、イベントの反復は自前の driver が持つ | `fmi` の ME のインスタンスの API の上に薄い層を載せる。`fmi` はシミュレーションの driver を持たない（`fmi-sim` は別 crate で、作業中とされる）ので、driver は結局自前 |
| 検証の独立性 | fmusim（`fmi-rs` 基盤）、FMPy（Python）と、実装が重ならない | 同じ（`fmi` は `fmi-rs` と別の実装） |
| 工数（推測） | ヘッダの手書き 2.0 と 3.0 で各 300〜500 行ほど。バグを自分で抱える | 少ない。ただし 1.0 前で API が動く。FMI 3.0 の SE・early return の対応の深さは【未確認】 |
| 危険 | ヘッダとの食い違い（引数の型、呼び出し規約）。Reference-FMUs の突き合わせで拾う | 上流の更新と GlobeMode の MSRV（rust 1.99、edition 2024）の食い違い。依存が増える |

- **推奨は案 A**。理由は、(1) driver が自前なので C ABI の層は薄くてよい、(2) bindgen と libclang を通常ビルドに入れない（S032 の `sundials` は feature に限っているのと同じ方針）、(3) 依存を増やさずに済み、`cargo deny` と [licenses.md](../licenses.md) の手間が小さい、(4) 実装すべき関数は ME・CS で数十個と少ない（推測）。
- 案 A で最初の 1 回（P5 の最初のセッション）に、ヘッダと関数の一覧を突き合わせて、工数の見積りを確かめる。見積りが大きく外れたら案 B に切り替える（その場合は ADR を Superseded にして書き直す）。
- 案 C として `fmi-rs`（BSD-2-Clause）も検討したが採らない。公開から日が浅く（0.3.4、2026-10-29）、`colored`（MPL-2.0）の例外が要り、build.rs の C のコンパイルが wasm32 を落とすため。fmusim が基盤にしているので、検証の相手と実装が重なる点も避けたい。

### (c) 共有ライブラリを読むのは Desktop と CLI だけ。しかも別プロセスのワーカーの中で

- `libloading` を使う部分は cargo feature（仮称 `dylib`）の中に閉じ、wasm32 では無効にする。ブラウザの `globe-mode-wasm` は `parse_model_description` だけを使う。
- FMU の ME・CS の実行は、ADR-0023 のワーカー（`globe` の CLI を sidecar にした別プロセス）の中で行う。FMU が segfault しても UI のプロセスは落ちず、ワーカーを作り直せる。時間切れも外から止められる。
- FMU は任意のネイティブコードを実行する。信頼していない FMU を開くときは、実行前にユーザーに確認を求める（UI は P5 より後。CLI は `--trust` のような明示を要求する案を P5 で決める）。

### (d) ME は GlobeMode のソルバで積分し、CS は FMU の内部の積分器に任せる

- ME: FMU の導関数とイベント指標を `HybridSystem`（[architecture.md §3.3](../architecture.md#33-ソルバ)）の RHS とイベントにする。全ソルバと driver をそのまま使う（ADR-0014 の自前の積分器を含む）。Rumoca の host は使わない（Modelica のモデル用）。
- CS: マスターが `do_step` する。固定刻みから始める（architecture.md §3.4 のマスターの順）。
- SE（3.0 のみ）は ME・CS の後。Clocks で確かめる。
- Modelica のモデルを同じ driver に載せるか（ADR-0013 の持ち越し）はこの ADR では決めない。

### (e) 検証

- 取得は `cargo xtask fetch-reference-fmus`（Reference-FMUs v0.0.42、sha256 固定、`data/reference-fmus/`、コミットしない）。試験の fixture は `modelDescription.xml` だけをコミットする（BSD-2、LICENSE 同梱）。バイナリはコミットしない。
- 突き合わせは fmusim と FMPy を別プロセスで呼ぶ（omc と同じ扱い）。参照の CSV は FMU の中の `extra/.../<名前>_out.csv`。fmuCheck は FMI 2 の Export の検査にだけ使える（更新が止まっている）。

## 結果

- `globe-mode-fmi` が workspace に入り、`modelDescription.xml` の変数表が 2.0 と 3.0 で同じ形で読める（試験 4 本、Reference-FMUs の Dahlquist と Feedthrough で 2.0 と 3.0 の変数表が一致）。
- 依存は `roxmltree` が 1 つ増えただけ（Cargo.lock は `roxmltree` と `globe-mode-fmi` の 2 行）。
- (b) をユーザーが決めたら、architecture.md §3.4 の「`fmi` 0.8 と `fmi-schema` の上に作る」と、表の `globe-mode-fmi` の行（`fmi, fmi-schema`）を直す。案 A なら「`libloading` と `roxmltree` の上に作る」。S057 では architecture.md を触っていない（並行セッションが編集中）。
- 次（P5 の最初）: 案 A の関数の一覧をヘッダと突き合わせ、2.0 の Dahlquist の ME（`libloading` で `.so` を読み、`GetDerivatives` を呼ぶ）を 1 本通す。`binaries/` のプラットフォーム名の選択（2.0 は `linux64`・`win64`、3.0 は `x86_64-linux`・`x86_64-windows`）も同時に作る。
- 残る不確かさ: 3.0 の `variability` の既定値（XSD に無く、仕様本文で未確認）、`fmu-runner` の FMI 3.0 対応、fmi-standard.org の validation のページの内容。
