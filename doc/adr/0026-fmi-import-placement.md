# ADR-0026 FMI Import の置き方: `modelDescription.xml` は自前の読み取り、共有ライブラリの読み込みは Desktop・CLI の別プロセスに限り、ME は GlobeMode のソルバで積分する

- 状態: **Accepted**（2026-11-06、S204）。S057（2026-11-01）では Proposed で、(b) は案が割れるのでユーザーが決める、とした。2026-11-06 にユーザーが「A、B 両方選択」と決め、下の「S204 の決定」で**案 A を既定にし、案 B を任意の feature として並べる**と解釈した。**この解釈はエージェント（親）のもので、ユーザーが違うと言えば直す**（A と B は同じ trait の 2 つの実装なので、組み替えは小さい）。ただし案 B は、2026-11-06 の時点で**入れられなかった**（理由は下の (g)。ユーザーの判断に回す）。(a)〜(e) と (b) の表は消さずに残す。**2026-11-07（S206）にユーザーが「案 B は自前で作成」と決めた**（下の (m)。`fmi` crate は入れない）
- 関係: [ADR-0013](0013-exec-backend.md)（Rumoca の実行系は Modelica のモデル用。FMU の driver は P5 で自前）、[ADR-0014](0014-session-route-wasm.md)（Rumoca の FMI ME 風の host に積分器を差す道）、[ADR-0023](0023-desktop-and-web-process-layout.md)（検査と実行は別プロセスのワーカー）。[architecture.md §3.4](../architecture.md#34-fmi-20--30) の「`fmi` 0.8 と `fmi-schema` の上に作る」を、(b) の決定に応じて直す
- 調査: [research/fmi-2026-11-01.md](../research/fmi-2026-11-01.md)

## 背景

- P5 は「FMI Import（ME / CS / SE）とマスター。Export は後」。完了条件は「Reference-FMUs が linux64 と win64 で一致する」（[roadmap.md](../roadmap.md)）。architecture.md は Import を jondo2010 の `fmi` 0.8 と `fmi-schema` の上に作ると書いていた（S002 の調査）。
- S057 の調査で分かったこと（出典と確認日は調査の文書）は次のとおり。
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

## S204 の決定（2026-11-06）: 案 A を既定、案 B を並べる口は trait、置き場は新しい crate

### (f) 決定

- **案 A（自前の C ABI）を既定にして実装した**。FMI 2.0 と 3.0 の **Co-Simulation** の最初の一切れ（読み込み・インスタンス化・初期化・`get`/`set`・`do_step`・終了）。ME・SE は未（次の WP）。
- **案 B（crate `fmi` 0.8）は入れていない**（(g)）。使えないコードを置かないために、feature の名前だけの口も置かない。A と B は trait [`FmuCs`](../../src/Rust/mod/globe-mode-fmi-import/src/lib.rs) の 2 つの実装として並べる設計にしたので、B を入れられる状態になれば、feature `fmi-crate`（仮称）の下に 1 ファイル（S204 で試した実装は 2.0・3.0 で約 90 行）足すだけでよい。
- **置き場は新しい crate `globe-mode-fmi-import`**。`globe-mode-fmi` の feature にしなかった理由は、(1) Export（`globe-mode-fmi` の `export`）と `parse_model_description` の利用者（wasm のブラウザ、Python、C ABI など。いまの facade `globe-mode` の `fmu` feature、LSP、Desktop）に `libloading` の依存を背負わせないため、(2) wasm32 で通る crate に wasm32 で通らない依存を feature で混ぜると、`--all-features` の検査（deny、clippy）が割れるため。依存は `globe-mode-fmi`（XML の読み取り）、`libloading` 0.9（ISC。Cargo.lock に既にあった）、`zip` 9（既にあった）、`tempfile` 3（既にあった）。facade・Python・C ABI・UI には**まだつなげていない**（呼ぶのは CLI の `globe fmu info` / `run` だけ）。
- **wasm32・Android では使えない**。共有ライブラリを読めない（wasm32）、FMI の規格に Android 向けの `binaries/` の置き場が無い（Android）。この crate を wasm 用の `cargo check`（ci.yml の wasm32 の行）に入れていないので、ブラウザ版・Android の APK には入らない。ブラウザは今までどおり `parse_model_description` で中身を見るだけ。
- **trait の形**（実装済み＝CS。ME は設計だけで、次の WP で足す）は次のとおり。

  | | `FmuCs`（実装済み） | `FmuMe`（設計） |
  | --- | --- | --- |
  | 初期化 | `initialize(start, stop)`（2.0 は `SetupExperiment` → `Enter/ExitInitializationMode`、3.0 は `EnterInitializationMode` → `Exit…`） | 同じ |
  | 値 | `get_real` / `set_real`（Float64 のスカラだけ） | 同じ |
  | 進める | `do_step(t, h) -> terminate?`（2.0 の `fmi2DoStep`、3.0 の `fmi3DoStep`） | `set_time`、`get/set_continuous_states`、`get_derivatives`、`get_event_indicators`、`enter_event_mode` / `new_discrete_states`（3.0 は `update_discrete_states`）/ `enter_continuous_time_mode`、`completed_integrator_step`。`HybridSystem` を実装して全ソルバを使う（(d)） |
  | 後片付け | `Drop` が `Terminate` → `FreeInstance`（呼び出しの失敗は後片付けの途中なので無視） | 同じ |

  インスタンスは `&Fmu` を借りる（展開先のディレクトリと読み込んだライブラリより先に落ちることを型で保つ）。`Send` ではない。ワーカーのプロセス（ADR-0023）で 1 スレッドが持つ前提。

### (g) 案 B を入れなかった理由（2026-11-06 に確かめた事実）

`fmi` 0.8.0（2026-07-28、MIT OR Apache-2.0）を optional な依存として足し、`cargo build`・`cargo deny --all-features check`・Reference-FMUs で試した。

- **zip 8.6.0 を引く。これには脆弱性の勧告が 4 件ある**（RUSTSEC-2026-0367〔SUID/SGID の保持〕・0368〔`extract` が作るシンボリックリンクが展開先の外を指せる〕・0369〔シンボリックリンク越しに権限を適用〕・0429〔`extract` がシンボリックリンクのエントリで abort〕。どれも `>=9.0.0` で直る）。`fmi` は FMU（＝信頼できないかもしれない zip）の展開にこの zip を使う（`fmi::import::from_path`）ので、**最も危ない道で脆弱な版を使う**。`fmi` の依存の指定は `zip ^8.0` で、GlobeMode 側からは 9 系に上げられない。S069 の licenses.md が「zip 2 系には脆弱性がある。9 系を使う」と書いた方針と衝突する。現行の `cargo deny check`（`all-features = false`）では見えないが、`--all-features` では advisories と licenses が落ちる。
- ライセンスは `libbz2-rs-sys`（`bzip2-1.0.6`。zip 8 の既定の feature `bzip2` 経由）が deny.toml の許可に無い。ほかは MIT・Apache-2.0・BSD-3-Clause（bindgen・zstd）・Unicode-3.0・CC0-1.0 で許可の範囲。GPL / LGPL の静的リンクは無い。
- **Cargo.lock が 67 パッケージ増える**（chrono・hard-xml・bindgen・clang-sys・zstd・aes・icu 系など）。`fmi-sys` の build が bindgen と libclang を要する（この機械では通ったが、CI の全 OS で要るかは確かめていない）。
- **FMI 3.0 が Reference-FMUs で動かない**: `fmi` 0.8.0 は `resourcePath` の末尾に区切りを付けず、Reference-FMUs（v0.0.42）が「resourceLocation must end with a file separator」と言って `Instantiate` が NULL を返す（`fmi3InstantiateCoSimulation`。BouncingBall・Dahlquist の 3.0 で再現）。FMI 2.0 の Co-Simulation は動き、BouncingBall・Dahlquist・VanDerPol の参照の CSV と全桁で一致した。
- ~~戻す条件~~（**S206 で打ち消した。案 B は採らない。(m)**。以下は S204 の時点の記録）: `fmi` が zip 9 以上に上がる（または GlobeMode が `[patch]` で上げられる）、3.0 の resourcePath が直る、deny.toml に `bzip2-1.0.6` を許可する（BSD 系の許諾。licenses.md に理由を書く）、の 3 つ。**ユーザーの判断を仰ぐ点**: B を「zip の脆弱性を受け入れて」入れるか、上流の更新を待つか、`[patch]` で自前に直すか。→ S206 で「自前で作る」と決まった（(m)）。

### (h) 信頼の境界

- **FMU は任意のネイティブコードを実行する**。`dlopen`（`libloading`）の時点で、共有ライブラリのコンストラクタが走りうる。`Fmu::open`（展開だけ）と `instantiate_cs`（読み込み）を分け、`read_model_description`（zip から XML だけを読む。ネイティブコードに触れない）を置いた。`globe fmu info` は後者だけを使う。
- **`globe fmu run` は実行の前に標準エラーへ 1 行出す**（`TRUST_WARNING`: 「警告: FMU の中のネイティブコードを実行する。信頼できる FMU だけを開くこと」）。`--trust` のような明示は要求しない（コマンドを自分で打つことが信頼の表明で、普通のバイナリを実行するのと同じ。S057 の「`--trust` を P5 で決める」の答え）。UI から開くときは、実行の前に確認のダイアログを必ず出す（UI は次の WP 以降。**ここでは未実装**）。
- **展開（zip slip など）**: `.fmu` を `tempfile` の一時ディレクトリに展開する。エントリの名前は `enclosed_name()`（`..`・絶対パス・NUL を拒む。S197 の Desktop の `msl.rs` と同じ考え方）で検査し、**拒むときは黙って飛ばさず失敗にする**。シンボリックリンクのエントリ（Unix のモードが `S_IFLNK`）も失敗にする。展開後の合計が 2 GiB を超えたら失敗（zip 爆弾への備え）。展開先は drop で消える（ワーカーの道では、親が作った一時ディレクトリを子に渡し、子が kill・クラッシュで終わっても親が消す。S218、#49）。
- 一時ディレクトリの中に `binaries/` の共有ライブラリを置いたまま読み込む（FMU の `resources/` を読むために、展開先を `resourcePath` に渡す必要があるため）。**読み込むライブラリは FMU の中のものだけ**で、`binaries/<置き場>/` の下の `<modelIdentifier>.<拡張子>` を引く。`modelIdentifier`（`modelDescription.xml` にあり、FMU の作者が書く）が空、`/`・`\`・NUL・`..` を含むときは、読み込む前に失敗にする（`binaries/` の外のファイルを読ませない）。
- 別プロセスのワーカーで回す（(c)）ことは**まだ実装していない**。いまの `globe fmu run` は CLI のプロセスの中で読み込む。FMU が落ちる（segfault）と CLI も落ちる。UI から使う WP で、ワーカーの中に入れる。

### (i) C ヘッダのライセンスと扱い

- 出典と確認日（2026-11-06に取得して確かめた）: `fmi2Functions.h`（tag v2.0.5）は「This file is licensed by the copyright holders under the 2-Clause BSD License」（<https://raw.githubusercontent.com/modelica/fmi-standard/v2.0.5/headers/fmi2Functions.h>）、`fmi3Functions.h`（tag v3.0.2）も同じ（<https://raw.githubusercontent.com/modelica/fmi-standard/v3.0.2/headers/fmi3Functions.h>）。リポジトリの `LICENSE.txt` は「Source code or other data, such as C-header and XML-schema files, that accompany the specification documents are released under the BSD 2-Clause license」（<https://raw.githubusercontent.com/modelica/fmi-standard/v3.0.2/LICENSE.txt>）。S057 の調査と同じ。
- **ヘッダは複製せず、型と関数の並びを Rust で手書きした**（`abi2.rs`・`abi3.rs`。`fmi2FunctionTypes.h`・`fmi3FunctionTypes.h` を見ながら。各ファイルの先頭に出典を書いた）。複製していないので BSD の表示は要らないが、複製するなら表示を付ける。bindgen は使わない（libclang を通常ビルドに入れない）。
- 工数の見積り（ADR の (b) の表は「各 300〜500 行」）は外れた。CS だけなら 2.0 が約 190 行、3.0 が約 170 行（ME を足すと倍になる程度）。ME の関数は 2.0 で 10 個ほど、3.0 で 10 個ほど増える。
- 2.0 の `fmi2CallbackLogger` は可変長引数（`...`）で、Rust の安定版では C の可変長の関数を定義できない。名前のついた 5 つの引数だけ受けて、残りは読まない非可変長の `extern "C" fn` を渡した（x86-64 の System V・Win64・AArch64 で、名前のついた引数の渡し方は通常の関数と同じなので、受け手が可変長を読まなければ安全。FMPy の ctypes も同じ）。書式 `message` は展開せずそのまま出す（Error / Fatal だけ）。

### (j) 検証

- 参照の相手は FMU が同梱する `extra/org.fmi-standard.fmi-ls-ref/<名前>_out.csv`（Reference-FMUs が自分の Co-Simulation を刻み 0.01 などで回した結果）。2.0 と 3.0 の BouncingBall・Dahlquist・VanDerPol を `run_cs` で同じ刻みで回し、**全行・全列で相対差 0**（ビット単位で一致）した（Linux x86_64、BouncingBall 301 行、Dahlquist 101 行、VanDerPol 2001 行）。2.0 の Dahlquist・VanDerPol は CSV を持たないので、同じモデルの 3.0 の CSV を使う。
- **往復**: `globe fmu export --build`（P5a、Rumoca のテンプレート）で作った Decay の FMU を `globe fmu run` で読み込み、x(1) − e⁻¹ = 7.8e-16（2.0・3.0 とも、刻み 0.001）。FMPy の c-code のヘッダと `cc` を使うので Linux の x86_64 だけ、無ければ飛ばす。
- OS ごと（`fmu-import.yml`。matrix: ubuntu-24.04・windows-2025・macos-latest〔arm64〕。run 42178862896、feature ブランチの push で回した）: 3 OS とも 9 本すべて通った。参照の CSV との最大の相対差は Linux・Windows が 0（ビット単位）、macOS（arm64）が BouncingBall 1.4e-16、Dahlquist 5.6e-17、VanDerPol 2.2e-14（CSV は x86_64 で作られたもので、arm64 の浮動小数点の演算の違い）。往復の試験は Linux だけ（Windows・macOS では飛ばす）。なお、新しい workflow は main に入るまで `gh workflow run --ref` で出せないので、この run は一時的に feature の push のトリガーを付けて回し、あとで外した。

### (l) S205 の決定（2026-11-06）: ME の積分器は crate の中に閉じ、依存を増やさない

- **Model Exchange を実装した**（`FmuMe`、`Fmu::instantiate_me`、`run_me`）。FMI 2.0・3.0 とも、Float64 のスカラ。呼び出しの順は 2.0.5 の疑似コードと 3.0.2 の状態機械に従う（呼び出しの表は [S205 の解説書](../sessions/S205-fmi-import-me/解説書.md)）。事象は state event（event indicator の領域 `> 0` と `≤ 0` の変化。3.0.2 §3.1.1 の定義）、time event（`nextEventTime`）、step event（`completedIntegratorStep` の `enterEventMode`）の 3 つ。事象モードの出入りと `newDiscreteStates` / `updateDiscreteStates` の反復、`valuesOfContinuousStatesChanged` / `nominalsOfContinuousStatesChanged` の旗、`terminateSimulation` を扱う。
- **積分器は `globe-mode-fmi-import` の中に自前で持つ**（`ode.rs`、試験を含めて約 240 行）: 固定刻みの古典 RK4 と、誤差制御つきの Dormand–Prince 5(4)（既定。許容誤差は `DefaultExperiment` の `tolerance`、無ければ 1e-6。絶対許容誤差は `rtol` × FMU の `nominal`）。state event の位置は、event indicator の領域が変わった歩の `[t, t1]` を二分法で詰める（試す時刻ごとに歩の頭から積分し直すので補間の誤差が入らない。幅が `1e-12 × max(1, t)` になるまで）。出力の刻み（`step`）は `DefaultExperiment` の `stepSize`、無ければ (終了 − 開始)/500。
- **依存の向き**: `globe-mode-fmi-import` は `globe-mode-sim`・`globe-mode-exec`（Rumoca を引く）に**依存しない**。Import の利用者に Rumoca を背負わせないため。(d) の「`HybridSystem` を実装して全ソルバを使う」は、ここでは**やっていない**（既存の crate に同じ Dormand–Prince の係数はあるが Rumoca / diffsol の中で、使えば依存が逆向きに太る）。`HybridSystem` へのアダプタは、両方に依存する別の薄い crate か facade の側で後から足せる（`FmuMe` は `HybridSystem` の RHS・事象に対応する関数を持つ）。依存は 1 つも増えていない。
- **限界（事実）**: (1) 陰的な積分は無い。硬い系は刻みが小さくなりすぎて `Error::Fmu` で止まる（`Roberts`〔3.0〕は未確認、試していない）。(2) ~~`fmi2Discard` / `fmi3Discard`（FMU が値を受け付けない）は失敗として止める。刻みを縮めて再試行しない。~~ → **S206 で直した（(m)）**: Dormand–Prince は `Discard` と有限でない誤差を却下して刻みを 0.2 倍にして再試行する。RK4（固定刻み）と二分法の中は縮めず `Error::Discard` で止まる。(3) 入力（`causality = input`）の設定は初期化の前のパラメータだけ。連続時間の入力の関数は渡せない。(4) `Integer`・`Boolean`・`String`・配列の変数は読み書きできない（Stair の `counter` は出力に出ない）。(5) 同時刻の事象は 100 回、全体では 10 万回で止める（Zeno の防止）。(6) FMU の状態の保存・復元（`GetFMUstate`）は使わない。(7) SE・Clocks は未。
- **確かめ方と許容差**: [解説書](../sessions/S205-fmi-import-me/解説書.md)の表。FMU が同梱する参照の CSV は FMU の **CS が前進 Euler で作った**ものなので、積分器が違う ME とはビット一致しない（特に BouncingBall は衝突の前後で `v` の符号が逆になるので点ごとの比較は意味がない）。そこで Dahlquist と BouncingBall は**解析解**と、VanDerPol は**独立な 2 つの積分（RK4 の刻み 1e-3 と、許容誤差 1e-10 の Dormand–Prince）の一致**と比べ、CSV は目安（BouncingBall は衝突の回数と時刻の格子への収まり、VanDerPol は最初の 2 秒）にした。

### (k) 持ち越し（次の WP の順番）

1. ~~ME（`FmuMe`）~~ **S205 で済み**（(l)）。残り: 陰的な積分（硬い系）、`HybridSystem` へのアダプタ、連続時間の入力（`Discard` の再試行は S206 で済み）。
2. ~~ワーカーのプロセスの中で読み込む（FMU が落ちても CLI・UI が落ちない）~~ **S210 で済み**（(n)。CLI だけ。facade・UI・Python は 3 の中）。残り: sandbox（ディスク・ネットワーク）、macOS・Windows のメモリの上限。
3. ~~facade（`import_fmu`）と UI の「FMU を開く」（確認のダイアログつき）、Python からの呼び出し~~ **S229 で済み**（(p)）。残り: C ABI・NIF。
4. コシミュレーションのマスター（Jacobi / Gauss–Seidel、系の構成は SSP か `.gmsys`）、SE（Clocks）、Integer・Boolean・String・配列の変数。
5. ~~案 B（(g) の条件がそろったら）~~（S206 で取り下げ。(m)）、Modelica のモデルから FMU の部品を使う。
6. (m) の表の「自前に無いもの」のうち、上の順番に入っていない項目（FMU の状態の保存・復元、方向微分、`Reset`・ログの切り替え、CS の early return・Event Mode・出力の微分、3.0 の構造パラメータ）。

### (m) S206 の決定（2026-11-07）: 案 B は「自前で作る」。FMI Import は全部、案 A の `globe-mode-fmi-import` で進める

- **ユーザーの決定**（2026-11-07）: 「FMI Import の案 B: 自前で作成」。**この解釈はエージェント（親）のもので、ユーザーが違うと言えば直す**: crate `fmi`（jondo2010、0.8）は入れない。案 B で得るはずだったもの（版を問わない Import の機能一式）も、依存を増やさずに `globe-mode-fmi-import` へ自前で足していく。(g) の「戻す条件」は**採らない**（上で打ち消した）。`doc/licenses.md` の `fmi` の行は「入れない（自前で作る）」にした。
- 理由: (g) の 3 点（zip 8 の勧告 4 件、Cargo.lock の 67 パッケージ増、3.0 の `resourcePath`）は上流の更新待ちで、S205 までに自前の ME・CS が 2.0・3.0 で Reference-FMUs と一致した（工数の見積りも外れて小さかった）。依存を増やす利点が薄い。
- **案 B（crate `fmi` 0.8.0）にあって、自前にまだ無いもの**（出典: <https://docs.rs/crate/fmi/0.8.0/features>、<https://docs.rs/fmi/0.8.0/fmi/>、<https://github.com/jondo2010/rust-fmi> の README、tag v0.8.0 の `fmi/src/fmi2|fmi3/instance/*.rs` の関数名。確認日 2026-11-07。コードは写していない）は次のとおり。

  | 案 B の機能 | 自前の現状 | 行き先 |
  | --- | --- | --- |
  | Integer・Boolean・String・Binary の get / set、配列 | Float64 のスカラだけ | (k) 4 に入っている |
  | Scheduled Execution（3.0 の `instantiate_se`、`activate_model_partition`）と Clock（`get_clock` / `set_clock`） | 無い | (k) 4 に入っている（SE・Clocks） |
  | 系の組み立て（`fmi-sim`「work-in-progress の FMU シミュレーションのマスター」） | 無い（`run_cs` は FMU 1 つ） | (k) 4 のマスターに入っている。案 B 側も未完成なので、持っていない |
  | `import::from_path` / `peek_descr_path`（版を見てから展開） | `read_model_description`（XML だけ）と `Fmu::open`（展開）がある | 足りている |
  | CS の `do_step_status` / `cancel_step` / `pending_status` / `last_successful_time` / `terminated`（2.0）、`enter_step_mode` / `get_output_derivatives` / early return / Event Mode（3.0 の CS） | `do_step` だけ。3.0 は `eventModeUsed = false`・`earlyReturnAllowed = false` で呼ぶ | **足す**: (k) 6。非同期の CS と、マスターの後ろ向きの修正に要る |
  | FMU の状態の保存・復元（`get_fmu_state` / `set_fmu_state` と直列化） | 使わない（`noSetFMUStatePriorToCurrentPoint = true`） | **足す**: (k) 6。マスターのやり直し・ME の巻き戻しに要る |
  | 方向微分（2.0 の `get_directional_derivative`）、3.0 の変数の依存（`get_variable_dependencies`） | 無い | **足す**: (k) 6。陰的な積分のヤコビアンに要る |
  | `reset`、`set_debug_logging`、logger の整形（書式の展開） | `Terminate` + `FreeInstance` だけ。logger は Error / Fatal を標準エラーに出すだけ（2.0 は可変長引数を読まない） | **足す**: (k) 6。`globe fmu run --log` |
  | 3.0 の構造パラメータ（`enter_configuration_mode` / `exit_configuration_mode`） | 無い | **足す**: (k) 6 |
  | `fmi-schema` の完全なスキーマ（単位、型定義、依存の情報など） | `globe-mode-fmi` が変数表・`DefaultExperiment`・`ModelStructure` の一部を読む | 必要になった所だけ `globe-mode-fmi` に足す（S206 で `stepSize` を足した） |
  | Export（`fmi-export`、`cargo-fmi`） | 自前の Export（P5a、`globe-mode-fmi` の `export`・`globe-mode-fmu`）がある | 足りている |
  | FMI-LS-BUS（`ls-bus`）、Arrow の出力（`arrow`） | 無い | 採らない（要望が出たら別の ADR） |

  上の「足す」は、(k) の 2〜4 の WP の中で必要になった順に入れる。
- **S206 のコードの直し**（レビュー [log/review/2026-11-07-s206-fmi-import-review-fixes-fable.md](../../log/review/2026-11-07-s206-fmi-import-review-fixes-fable.md) への対応）: (1) 有限でない誤差の却下と `Discard` の再試行（出典: FMI 2.0.5 §2.1.3「For "model exchange": It is recommended to perform a smaller step size and evaluate the model equations again」<https://github.com/modelica/fmi-standard/blob/v2.0.5/docs/2_1_common_api.adoc>、FMI 3.0 §2.2.4「Advanced importers may try alternative approaches to continue the simulation」<https://fmi-standard.org/docs/main/>、確認日 2026-11-07。`Error`・`Fatal` は止める）、(2) 出力の格子の上限 1000 万点、(3) `DefaultExperiment.step_size`、(4) 3.0 の真偽の出力引数を `u8` で受ける。

### (n) S210 の決定（2026-11-07）: FMU の読み込みと実行は別プロセスのワーカーに隔離する

(c)・(h) の「ワーカーの中で読み込む」を実装した。`globe fmu run` は FMU に一切触れず、**自分自身（`std::env::current_exe()`）を隠しの副命令 `globe fmu __worker` で子プロセスとして起動**し、FMU の展開・読み込み・実行は子の中だけで行う。

- **API**（`globe-mode-fmi-import` の `worker` モジュール）: `run_in_worker(&WorkerSpec, &WorkerRequest, progress) -> Result<WorkerRun>`（親）、`worker_main() -> i32`（子。呼び出し側の副命令から呼ぶ）、`run_in_process`（子の中身。プロセス内で回したい人向け）。**どの実行ファイルをワーカーにするかは呼び出し側が渡す**（`WorkerSpec { exe, args, timeout, memory_limit }`）ので、将来の facade・Desktop の sidecar（ADR-0023 の `globe lsp` と同じ `globe` を別の副命令で）・Python が同じ API を使える。今回はそれらに口を足していない（(k) の 3）。`run_cs` / `run_me` に、格子点ごとに時刻を受ける `run_cs_with` / `run_me_with` を足した（進捗用。既存の関数は挙動が同じ）。
- **要求と結果**: 要求は標準入力の 1 行の JSON（`WorkerRequest`: FMU のパス、kind、開始・終了時刻、刻み、出力、`set`）。結果は標準出力の行（`\u{1}GW ` で始まる行だけがメッセージ。印の無い行は FMU の `printf` として捨てて標準エラーの末尾と同じ所に溜める）: `P 時刻`（進捗。子が 0.25 秒以上あけて流す）、`H 変数名の JSON`、`M 要約の JSON`（ME だけ。歩数・事象の種類ごとの数・打ち切り）、`R 時刻 値…`（空白区切り、f64 は `{:?}` で書くので NaN・inf を含めビット単位で往復する。JSON にしないのは serde_json が NaN を `null` にするため）、`D`（正常終了の印。無いまま標準出力が閉じたら「落ちた」）、`E 文面`（子の中の実行の失敗）。protocol crate は使わない。依存は `serde`・`serde_json` を fmi-import に足しただけ（どちらも workspace に既にある。Cargo.lock の増減なし）。
- **落ちたとき**: 親は `Error::Worker`（「FMU のワーカー（別プロセス）: 結果を返す前に終わった（signal: 6 (SIGABRT)）。FMU が落ちたとみられる」に、標準エラーと印の無い標準出力の末尾 4 KiB を添える）を返す。子の中の失敗（変数が無い、`Fmu` の失敗など）は `Error::Child`（中の文面そのまま）。親（呼び出し側）は落ちず、終了コード 1。
- **上限**: 時間は `WorkerSpec::timeout`（`globe fmu run --timeout 秒`。既定は無し）で、超えたら kill して「N s で終わらないので止めた（最後の進捗は t = …）」。メモリは `memory_limit`（`--memory-limit MiB`）で、**Linux だけ**、子を `sh -c 'ulimit -v …; exec …'` で包んで仮想メモリを制限する（`unsafe` も `libc` も要らない。超えると FMU の `malloc` が失敗して FMU が落ち、上と同じ誤りになる）。Windows・macOS で指定すると `Error::Worker`（持ち越し: macOS は `RLIMIT_AS` が当てにならず、Windows は Job Object が要る）。**ワーカーが消えない**ことは `Reaper`（drop で kill して wait）で保証し、親が SIGKILL などで消えたときは、子が親の標準入力の EOF を見て自分で終わる（孤児を残さない）。
- **隔離するもの／しないもの**: 隔離するのは **プロセスのクラッシュ・無限ループ（時間）・メモリ（Linux）** だけ。**ディスク（FMU は任意のファイルを読み書きできる）とネットワーク、環境変数、他のプロセスへの信号は隔離しない**。sandbox（seccomp・Landlock・ファイルシステム／ネットワークの名前空間・AppContainer）は持ち越し。したがって「信頼できない FMU を安全に開ける」ではなく「FMU のバグで呼び出し側が落ちない」までを保証する。実行前の警告は「ネイティブコードを別プロセス（ワーカー）で実行する。落ちても呼び出し側は落ちないが、ディスクとネットワークは隔離しない」に変えた。
- **限界**: (1) FMU が自分で子プロセスを作って管を握ると、親の標準エラーの読み取りが戻らない（`shortcut` のコメント。Job Object / プロセスグループへ）。(2) 結果は全行を子の中のメモリに持ってから送る（格子点の上限 1000 万は (m) のまま）ので、メモリの上限は子の分を含めて見積もる。(3) 起動のたびに `globe` の起動（約 数十 ms）と FMU の展開が子で行われる。(4) 要求に ME の積分器と許容誤差は無い（CLI にも無い。要るようになったら足す）。
- **試験**（`globe-mode-cli/tests/fmu_worker.rs`）: Reference-FMUs の Dahlquist（2.0・3.0 の CS）・BouncingBall（2.0・3.0 の ME）・VanDerPol（3.0 の ME）が、プロセス内の結果と**ビット単位で同じ**。`cc` で作った小さな共有ライブラリで、`abort`（Instantiate）・segfault（DoStep）・無限ループ（DoStep、`--timeout`）・メモリの食いつぶし（GetReal、`--memory-limit`）を起こし、親が落ちずに誤りを返し、`/proc/self/task/*/children` に zombie も孤児も残らないことを確かめる（結果は解説書の表）。

### (p) S229 の決定（2026-11-08）: facade・Python・Desktop の UI から FMU を使う

(k) の 3 番目。**ワーカーの実行ファイルは呼び手が渡す**（(n) の `WorkerSpec`）。facade の口は 2 つだけで、CLI・LSP・Python が同じ道を通る（写しを作らない。CLI の `fmu info` / `run` も facade を呼ぶ形に寄せた）。

- **facade `globe-mode`（feature `fmu-import`）**: `import_fmu_info(path) -> FmuInfo`（`globe fmu info` と同じ情報。`Serialize`）、`run_fmu(worker, path, &FmuRunRequest, on_progress, cancel) -> FmuRun`。`FmuRunRequest` は JSON でも読める（`kind` は `"cs"` / `"me"`、`start_time`・`stop_time`・`step`・`outputs`・`set`〔名前 → 値〕・`timeout`・`memory_limit_mib`。未知の鍵は誤り）。`FmuRun { results: OpenedResults, me }`: 結果の列は `OpenedResults::columns()`（S222。`Simulated::columns` と同じ形。FMU の出力はすべて `Output` の役割で、既定で描くのは先頭の 3 本）。`worker` は `fmu __worker` を持つ実行ファイル: `globe` 自身（CLI・LSP の sidecar は `current_exe()`）、Python は `GLOBE_BIN` か PATH の `globe`（wheel の中には実行ファイルが無い。見つからなければ `FmuError` に理由）。wasm32・Android には付けない（共有ライブラリを読めない）ので feature にし、wasm が引く `globe-mode-lsp` でも既定では切ってある（CLI が付ける）。
- **中断**: `WorkerSpec::cancel`（`Arc<AtomicBool>`。facade は `CancelToken::flag()`）が立つと、親が 50 ms おきの確認で子を kill して `Error::Worker`（「中断した」）にする。進捗の出ない FMU（DoStep が戻らない）も止まる。`globe-mode-fmi-import` の `worker.rs` を触ったのはここと、`Kind` の JSON を小文字（`"cs"` / `"me"`）にした 1 行だけ。
- **LSP（Desktop の sidecar）**: `globe/fmuInfo {path}`（`FmuInfo` に実行前に見せる `warning` を足す）と `globe/fmuRun {path, request?, workDoneToken?}`（`{layout, columns, me}`。`layout`・`columns` は `globe/readResults` と同じ。`$/progress` は `{kind: "report", stage: "integration", time}`、`$/cancelRequest` でワーカーを kill して -32800、失敗は -32000、params の誤りは -32602）。`shell.rs` の `RPC_METHODS` は 20 から 22。ワーカーは sidecar と同じ `globe` なので、Desktop に新しい実行ファイルは要らない。
- **Python**: `fmu_info(path)`、`run_fmu(path, *, kind, start_time, stop_time, step, outputs, parameters, timeout, on_progress)`（`read_results` と同じ形の dict に `me` を足す）、例外 `FmuError`（`RuntimeError` も継ぐ）。`on_progress(t)` が真を返すか Ctrl-C で止まる（進捗が出たときだけ見る。出ない FMU は `timeout`）。
- **UI**: 「開く」で `.fmu` を選ぶと、`fmuInfo` の警告（S210 の文面）と要約を `confirm` に出し、承知したら `fmuRun`（FMU の既定の実験設定）で回して結果のタブ（S222）にプロットする。「止める」で kill。**ブラウザ・Android は zip を読まず「Desktop だけ」と理由を出す**。ME の積分器の選択・実験設定の入力欄は UI に出していない。
- **試験**: facade（`globe-mode-cli/tests/fmu_worker.rs`。Dahlquist の CS・ME、JSON の request、失敗、中断の旗〔`cc` の遅い FMU を 400 ms で止める。子が残らない〕）、LSP の rpc（`tests/lsp.rs` で `globe lsp` を起動して info・run・-32000・-32602。`globe-mode-lsp` の単体は params の誤りだけ。ワーカーが試験のバイナリになるため）、Python（`GLOBE_BIN` を試験の `globe` に向ける）、`test_ui.py`（ブラウザは「Desktop だけ」、Desktop の偽 fs で確認を断れば回さず、承知すれば結果のタブ）、`test_tauri.mjs`。
- **限界**: sandbox・macOS / Windows のメモリの上限は (n) のまま。UI は実験設定を変えられない。C ABI・NIF は未（(k) 3 の残り）。Desktop の中断は sidecar への `$/cancelRequest` が届いてから 50 ms 以内にワーカーを止める（sidecar の LSP は要求を 1 本ずつ処理するので、FMU の実行中はほかの要求が待つ）。

## 結果

- **S229 の結果**: 上の (p)。facade・Python・Desktop の UI から FMU を読んで回せる。
- **S210 の結果**: 上の (n)。`globe fmu run` は FMU を別プロセスで回し、落ちても `globe` は落ちない。
- **S206 の結果**: 上の (m)。案 B は採らず自前で進める。レビューの指摘 4 件を直した。
- **S205 の結果**: 上の (l)。`FmuMe` が FMI 2.0・3.0 で動き、BouncingBall の衝突 11 回の時刻が解析解と 3.2e-11 で一致する。`globe fmu run --kind me`。
- **S204 の結果**: 上の (f)〜(k)。`globe-mode-fmi-import` が workspace に入り、`globe fmu info` / `run` がある。FMI 2.0・3.0 の Co-Simulation が Reference-FMUs の参照の CSV と全桁で一致する。案 B は入れていない（ユーザーの判断待ち）。
- （以下は S057 の結果）`globe-mode-fmi` が workspace に入り、`modelDescription.xml` の変数表が 2.0 と 3.0 で同じ形で読める（試験 4 本、Reference-FMUs の Dahlquist と Feedthrough で 2.0 と 3.0 の変数表が一致）。
- 依存は `roxmltree` が 1 つ増えただけ（Cargo.lock は `roxmltree` と `globe-mode-fmi` の 2 行）。
- (b) をユーザーが決めたら、architecture.md §3.4 の「`fmi` 0.8 と `fmi-schema` の上に作る」と、表の `globe-mode-fmi` の行（`fmi, fmi-schema`）を直す。案 A なら「`libloading` と `roxmltree` の上に作る」。S057 では architecture.md を触っていない（並行セッションが編集中）。
- 次（P5 の最初）: 案 A の関数の一覧をヘッダと突き合わせ、2.0 の Dahlquist の ME（`libloading` で `.so` を読み、`GetDerivatives` を呼ぶ）を 1 本通す。`binaries/` のプラットフォーム名の選択（2.0 は `linux64`・`win64`、3.0 は `x86_64-linux`・`x86_64-windows`）も同時に作る。
- 残る不確かさ: 3.0 の `variability` の既定値（XSD に無く、仕様本文で未確認）、`fmu-runner` の FMI 3.0 対応、fmi-standard.org の validation のページの内容。
