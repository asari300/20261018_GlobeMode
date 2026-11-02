# ADR-0032 P5b の自前のランタイムは、FMI 2.0 ME の C ABI を自前の cdylib で出す。S095 はモデルを crate に固定する

- 状態: Proposed（2026-11-02、S095。モデルの載せ替えの道は未決）
- 関係: [ADR-0026](0026-fmi-import-placement.md)（Import の置き方。C ABI を自前で持つ案）、[architecture.md](../architecture.md) の「Export」（P5a と P5b）。P5a（Rumoca のテンプレート、S094）の ADR-0031 とは別の番号にした

## 背景

- roadmap の P5 の Export は、P5a（Rumoca の `fmi2` / `fmi3` テンプレートでソース FMU を作る）の後に P5b（自前のランタイム）。2026-11-02 にユーザーが「P5a を先行し、それを参考にしつつ P5b を並行して進める」と決めた。
- S057 の調査（[research/fmi-2026-11-01.md](../research/fmi-2026-11-01.md)）で、Rust で FMU を出す crate は `fmi-export` 0.3.0（MIT OR Apache-2.0、FMI 3.0 の Export のみ）だけ。
- `globe-mode-exec` の公開 API は `simulate` / `run` などの「最後まで回す」ものだけで、任意の時刻と状態で右辺（微分）を評価する API は無い。ME はこれが要る。
- S095 では exec・sim・`globe-mode-fmi`・CLI を変えない（S094 と並行のため）。

## 決定

### (a) 置き方: 自前の FMI 2.0 ME の C ABI（採用）と `fmi-export` 0.3.0 を比べた

| 案 | 長所 | 短所 |
| --- | --- | --- |
| (a) `fmi-export` 0.3.0 の derive / trait にモデルを載せる | 関数の一覧と型を crate が持つ | FMI 3.0 のみ（P5 の完了条件の FMI 2.0 が無い）。依存が増え、`cargo deny` と licenses.md の手間が増える。1.0 前で API が動く |
| (b) 自前で `fmi2*` の 35 関数を `extern "C"` で出す（採用） | 依存 0。FMI 2.0 は FMPy・fmusim・fmuCheck のどれでも回せる。`globe-mode-c`（S075）と同じ形 | 関数の署名を自分で規格に合わせる（FMPy で確かめる）。FMI 3.0 は別に書く |

- 決め手は「依存 0 で小さい」と「FMI 2.0 が P5 の完了条件にある」の 2 つ。
- 新しい crate `globe-mode-fmu`（cdylib + rlib）。workspace のどの crate にも依存しない。exec に右辺の API が無いので、S095 では `Decay`（`der(x) = -k*x`）の式を Rust で書いて固定した。
- `modelDescription.xml` は crate に手書きで置き、`guid` を `src/lib.rs` の `GUID` と突き合わせる試験を置く。
- FMUstate・方向微分・Integer / Boolean / String の値は `fmi2Error` の stub（`modelDescription.xml` の `can*` / `provides*` は既定の false）。ログの callback は呼ばない。
- FMI 2.0 の共有ライブラリの置き場は `binaries/linux64/<modelIdentifier>.so`（FMI 3.0 なら `binaries/x86_64-linux/`）。

### (b) 任意のモデルを載せ替える道（持ち越し。候補）

| 道 | 要るもの | 長所 | 短所 |
| --- | --- | --- | --- |
| 1. FMU ごとに cdylib をビルドする（Solve IR から Rust のソースを生成） | Export のときに cargo と rustc（または Rumoca の `rust-ode` テンプレート） | 速い。FMU が小さい | 利用者の手元にツールチェーンが要る。target ごとにビルド |
| 2. 1 つの cdylib が `resources/` の Modelica のソースを読み、Rumoca で compile する | exec に「右辺を評価する」公開 API。cdylib に Rumoca と diffsol を入れる | ビルドが Export のたびに要らない | cdylib が大きい。`fmi2Instantiate` のたびに compile（MSL を読むと秒単位）。ライブラリの同梱 |
| 3. 1 つの cdylib が `resources/` に置いた Solve IR（直列化）を解釈実行する | Solve IR の直列化と、Rumoca に依存しない評価器 | コンパイラが要らない（architecture.md の P5b の「解釈実行」）。ブラウザ・Android から Export できる | 直列化の形を自分で決めて保つ。評価が遅い |

- どれでも、exec の側に「状態・パラメータ・時刻 → 微分・出力」の公開 API が要る（今は無い）。次に P5b を進めるときに、まずこの API を exec に足して、S095 の手書きの式をそれで置き換える。
- `modelDescription.xml` は、載せ替えのときに Solve IR から作る（`globe-mode-fmi` の型を使う。P5a と共有する）。

### (c) P5a との分担

- P5a（S094、`globe-mode-fmi` の `export_me_sources` と `globe fmu export`）はソース FMU を作る。P5b はコンパイラの要らないバイナリの FMU を作る。
- 両方とも検証は FMPy（BSD-2-Clause）で x(1) を解析解と比べる。P5a で作った `modelDescription.xml` の生成を P5b でも使う（重複させない）。

## 結果

- `cargo test -p globe-mode-fmu` の `fmpy_simulates_decay` が、cdylib と `modelDescription.xml` を FMU に詰め、FMPy 0.3.32 の ME（既定のソルバ CVode）で回す。x(1) = 0.3678794150955906（e⁻¹ = 0.36787944117144233、差 2.6e-8）。uv か FMPy を使えない環境では skip する。
- 関数を直接呼ぶ試験（陽的 Euler、k = 2）で x(1) が e⁻² に 1e-4 以内。
- 依存の追加 0。
