# ライセンス方針

- GlobeMode は **MIT OR Apache-2.0** のデュアルライセンスで公開する（[ADR-0001](adr/0001-license-policy.md)）。
- この文書は依存を足すときの判断基準と、現時点の判定の一覧。依存を足す手順は `.claude/skills/add-dependency/SKILL.md` にある。
- 法的助言ではない。判断に迷ったら、依存を足す前に作業記録に書いて相談する。

## 1. 区分

| 区分 | ライセンス | 扱い |
| --- | --- | --- |
| **許可** | MIT、Apache-2.0、Apache-2.0 WITH LLVM-exception、BSD-2-Clause、BSD-3-Clause、ISC、Zlib、Unicode-3.0、CC0-1.0、BSL-1.0、PSF-2.0（S053、Python の typing-extensions）、BlueOak-1.0.0（S053、npm の lru-cache など）、0BSD、CDLA-Permissive-2.0（S053、loco の lettre 経由の quoted_printable と webpki-roots。いずれも制約の無い寛容な許諾） | そのまま依存してよい。配布物にライセンス文を同梱する |
| **条件付き** | MPL-2.0、EPL-2.0（別ライセンスとの OR を含む） | 改変しないで使う場合に限る。改変するなら、そのファイルを元のライセンスのまま公開する。cargo-deny では個別に例外として登録する |
| **任意の feature に限る** | LGPL-2.1-or-later（KLU など） | 既定の配布物には含めない。利用者が自分でビルドする任意の feature に限る |
| **禁止** | GPL-2.0 / 3.0、AGPL-3.0、OSMC-PL、SSPL、CC BY-NC、PolyForm（非商用）、独自の source-available | 依存しない。リンクもしない。コードを複製しない |

## 2. 主な依存の判定（2026-10-18）

| 対象 | ライセンス | 判定 | メモ |
| --- | --- | --- | --- |
| Rumoca（`rumoca-*`） | Apache-2.0（NOTICE は無い） | 許可 | git の `rev` で固定する。改変した場合は §4b に従って明記する。商標は許諾されないので、製品名に使わない。S012: `globe-mode-exec/src/rumoca/plugins/rk.rs` は Rumoca の `crates/rumoca-solver-diffsol/src/me_integrator.rs`（Apache-2.0）を写して改変したもので、ファイルの先頭に明記した。S094: `globe fmu export` が出す FMU の C と `modelDescription.xml` は Rumoca の組み込みターゲット `fmi2` のテンプレート（改変していない）の描画物で、テンプレートの文面を多く含む。S098: 描画物はテンプレートの文面を写した Rumoca の Work の派生物として扱い、Apache-2.0 §4(a)(c) に合わせて FMU の `documentation/licenses/Rumoca-LICENSE.txt` に Rumoca の著作権表示（`Copyright 2025-2026 CogniPilot Foundation`）と Apache-2.0 の全文を同梱する（下の「生成コード」） |
| zenoh | EPL-2.0 OR Apache-2.0 | **禁止（`bans`）** | IDE に要らない。Rumoca の umbrella crate を使わなければ入らない |
| diffsol | MIT | 許可 | 既定のソルバ。S011 から Rumoca が数値の修正を当てた 0.13.2（Rumoca の `vendor/diffsol`、修正の記録は `RUMOCA_PATCH.md`）を `[patch.crates-io]` で使う（[ADR-0013](adr/0013-exec-backend.md)）。S012 で `globe-mode-exec` の直接の依存にした（自前の積分器 esdirk34・trbdf2・tsit45。0.13 で、同じ patch 版。[ADR-0014](adr/0014-session-route-wasm.md)） |
| rumoca-sim、rumoca-solver、rumoca-solver-diffsol、rumoca-solver-rk45、rumoca-phase-solve、rumoca-exec-cranelift | Apache-2.0（Rumoca のワークスペース） | 許可 | S011 で導入（`globe-mode-exec` だけ。cargo-deny の `wrappers`）。feature は `solver-diffsol` と `solver-rk45` だけ（zenoh・UDP・gamepad は入らない）。S012 で `rumoca-phase-solve`・`rumoca-solver-diffsol`・`rumoca-solver-rk45` を exec の直接の依存にした（session の道。どれも Cargo.lock に既にあり、外部の package は増えていない） |
| Cranelift 0.125（cranelift-*、regalloc2、target-lexicon、wasmtime-internal-*） | Apache-2.0 WITH LLVM-exception | 許可 | S011 で `rumoca-exec-cranelift` 経由（wasm 以外で無条件）。region（MIT）、arbitrary（MIT OR Apache-2.0）も同じ経路 |
| nalgebra-sparse 0.11 | Apache-2.0 | 許可 | S011 で diffsol 経由 |
| self_cell 1.3 | Apache-2.0 OR GPL-2.0-only | 許可（Apache-2.0 を選ぶ） | S011 で rumoca-solver-diffsol 経由。S012 で `globe-mode-exec` の直接の依存にした（問題とソルバの自己参照。rumoca-solver-diffsol と同じ使い方） |
| atomic-wait 1.1 | BSD-2-Clause | 許可 | S011 で faer の rayon（spindle）経由 |
| instant 0.1 | BSD-3-Clause | 許可 | S011 で rumoca-solver が wasm32 のときだけ使う。保守終了の勧告 RUSTSEC-2024-0384 を deny.toml の `ignore` に理由付きで足した |
| MSL 4.1.0 の参照結果（MAP-LIB_ReferenceResults、ブランチ v4.1.0） | リポジトリに LICENSE が無い【未確認】 | 取得だけ（コミットしない） | S011。`cargo xtask fetch-msl-ref` が固定したコミットから `data/msl-ref/` に取り、試験の比較にだけ使う。配布物に入れない |
| wasm-bindgen 0.2.129 | MIT OR Apache-2.0 | 許可 | S012 で導入（`globe-mode-wasm` の JS の束ね）。CLI と版を合わせるため Cargo.toml で `=0.2.129` に固定する |
| wasm-bindgen-cli 0.2.129（`github:wasm-bindgen/wasm-bindgen`） | MIT OR Apache-2.0 | 許可（ツール） | S012。mise で取得するツールで、リンクしない（`wasm-bindgen --target nodejs` で糊を作るだけ） |
| Leptos 0.8.23（S153、`src/Rust/apps/experimental/leptos` の試験版 UI だけ。`csr` feature。推移的に `leptos_macro`・`leptos_dom`・`reactive_graph`・`tachys`・`server_fn`・`any_spawner`・`gloo-net` など。その workspace の `Cargo.lock` の 531 パッケージ。`web-sys`・`wasm-bindgen-futures` は MIT OR Apache-2.0） | MIT（leptos 系は MIT。`cargo deny list`＝2026-11-05 で GPL・LGPL が出るのは OR の片側（`self_cell` の Apache-2.0 OR GPL-2.0-only、`r-efi` の MIT OR Apache-2.0 OR LGPL-2.1-or-later）だけで、許可側を選ぶ） | 許可 | [ADR-0035](adr/0035-experimental-ui-leptos.md)。別の workspace（ルートの workspace・配布物には入らない）。cargo deny は desktop と同じ流儀の `leptos/deny.toml` で通した（advisories・bans・licenses・sources とも ok）。出典: https://crates.io/crates/leptos （確認 2026-11-05。0.9.0-rc1 は採らない） |
| Dioxus 0.7.10（S155、`src/Rust/apps/experimental/dioxus` の試験版 UI だけ。`default-features = false`、feature `minimal`・`web`。推移的に `dioxus-core`・`dioxus-web`・`dioxus-html`・`dioxus-signals`・`dioxus-hooks`・`dioxus-interpreter-js` など dioxus 系。その workspace の `Cargo.lock` の 624 パッケージ。`js-sys`・`web-sys`・`wasm-bindgen-futures` は MIT OR Apache-2.0） | MIT OR Apache-2.0（dioxus 系はすべて。`cargo deny list`＝2026-11-05 で GPL・LGPL が出るのは Leptos 版と同じ OR の片側（`self_cell`、`r-efi`）だけで、許可側を選ぶ） | 許可 | [ADR-0036](adr/0036-experimental-ui-dioxus.md)。別の workspace（ルートの workspace・配布物には入らない）。cargo deny は `dioxus/deny.toml` で通した（advisories・bans・licenses・sources とも ok）。出典: https://crates.io/crates/dioxus （確認 2026-11-05。最新は 0.8.0-alpha.3 で、安定版の 0.7.10 を採る） |
| getrandom 0.3 | MIT OR Apache-2.0 | 許可 | S012 で `globe-mode-exec` の wasm32 のときだけの直接の依存（feature `wasm_js`。diffsol → faer → rand の乱数の backend）。Cargo.lock に既にある |
| SUNDIALS（sundials-sys 0.6.2） | BSD-3-Clause（sundials-sys、同梱の SUNDIALS 7.1.1 とも。sundials-sys の `vendor/LICENSE` で確認、2026-10-30） | 許可 | S032 で導入（S034 で IDA を足した）。`globe-mode-exec` の optional な依存で、feature `sundials` を立てたときだけ入る（CLI は `--features sundials` で転送）。wasm では使えない。sundials-sys の feature は `cvode`・`ida`・`build_libraries`・`static_libraries` だけ（`default-features = false` で arkode・kinsol を切る）で、同梱のソースから cmake で CVODE・IDA と serial N_Vector・密な行列・密な線形ソルバ・Newton 法などを静的ライブラリにしてリンクする。`klu`（LGPL。suitesparse_sys が入る）・`nvecopenmp`・`nvecpthreads` は使わない。ビルド時の依存: bindgen 0.68.1（BSD-3-Clause）・cmake 0.1.58（MIT OR Apache-2.0）と、bindgen の推移的依存（clang-sys、cexpr、nom、which、peeking_take_while、lazycell、libloading、prettyplease など）。ビルドには cmake・C コンパイラ・libclang が要る。`cargo deny --all-features check` は ok（advisories・bans・licenses・sources。2026-10-30）。`deny.toml` は `all-features = false` のままで、既定の `cargo deny check` はこの依存を見ない（Cargo.lock には載る）。個々の推移的依存のライセンスの確認は cargo-deny の判定に任せた【crates.io での個別の確認は未確認、2026-10-30】 |
| faer、egg、salsa、rowan、petgraph、lsp-types | MIT / Apache-2.0 | 許可 | — |
| lsp-types 0.97.0（fluent-uri 0.1.4、serde_repr 0.1.21） | MIT OR Apache-2.0（fluent-uri は MIT。crates.io の metadata で確認、2026-10-30） | 許可 | S038 で導入（`globe-mode-lsp`。型だけ使い、JSON-RPC は自前）。推移的依存で新しく入るのは fluent-uri と serde_repr の 2 つ。`cargo deny check` は ok |
| serde、serde_json、clap | MIT OR Apache-2.0 | 許可 | S002 で導入 |
| ignore 0.4.34、globset 0.4.21（globe-mode-cli、S060） | MIT OR Unlicense | 許可（MIT を選ぶ） | `globe fmt` / `globe lint` のディレクトリの走査で `.gitignore` を守る。ruff と同じ crate。ほかの推移的依存（walkdir・bstr・regex-automata・crossbeam-deque など）は Cargo.lock に既にある。`cargo deny check licenses` 通過、2026-11-01 |
| toml 0.8.23（globe-mode-fmt の optional な feature `config`、S130） | MIT OR Apache-2.0 | 許可 | 設定ファイル `globe-mode.toml` を読む。CLI と LSP だけが feature を立て、既定と wasm は依存 0 のまま（[ADR-0024](adr/0024-own-formatter-and-lint.md)）。rumoca-core の依存として Cargo.lock に既にあり（serde_spanned・toml_datetime・toml_edit・toml_write・winnow も同じ）、新しい crate は入らない |
| sha2 | MIT OR Apache-2.0 | 許可 | S003 で導入（xtask の sha256）。S069 で desktop（別 workspace）も MSL の sha256 の照合に使う（`Cargo.lock` に既にあった） |
| salsa 0.28.5 | Apache-2.0 OR MIT | 許可 | S004 で導入（`globe-mode-db`）。推移的な依存も cargo deny で ok |
| rumoca-ir-dae、rumoca-eval-dae、rumoca-core | Apache-2.0（Rumoca のワークスペース） | 許可 | S005 で導入（`globe-mode-frontend` だけ）。DAE の incidence を自前で作るため。rumoca-compile の推移的依存なので、新しい外部依存は増えない |
| zip | MIT | 許可 | S003 で導入（xtask で MSL を展開）。**9 系を使う**: 2 系には展開時の脆弱性（RUSTSEC-2026-0367 / 0368 / 0369）がある。S069 で desktop（別 workspace、`default-features = false, features = ["deflate"]`）も MSL の展開に使う。`zip` 9.2.1 のほか `typed-path`・`zopfli`・`nix`・`cfg_aliases` が `Cargo.lock` に増えた（desktop の `cargo deny` は ok）。S094 で `globe-mode-fmi` の feature `export` も FMU（zip）を書くのに使う（同じ 9 系・同じ feature。Cargo.lock に増えたパッケージは 0） |
| MSL 4.1.0、Rumoca の CLI（照合用） | BSD-3-Clause / Apache-2.0 | 許可 | `cargo xtask fetch-*` で sha256 を固定して取得する。配布物には入れない（`data/` は git 管理外） |
| cargo-deny | MIT OR Apache-2.0 | 許可（ツール） | mise で入れる。配布物には入らない |
| fmi、fmi-schema、fmi-sys、fmi-export | MIT OR Apache-2.0 | 許可 | 依存には足していない（S057 の調査。crates.io で確認、2026-11-01）。`fmi-sys` の build に bindgen（libclang）、`fmi-schema` は `hard-xml`（MIT）など。使うかは [ADR-0026](adr/0026-fmi-import-placement.md) の (b) |
| fmi-rs 0.3.4、fmusim | BSD-2-Clause | 許可（本体）。`fmi-rs` の依存 `colored`（MPL-2.0）は条件付き | 依存には足していない（S057。crates.io とリポジトリで確認、2026-11-01）。fmusim は検証の道具（別プロセスで呼ぶだけ）。`fmi-rs` の build.rs は C をコンパイルし wasm32 で落ちた |
| FMI ヘッダ（fmi2Functions.h など）と XSD、Reference-FMUs | BSD-2-Clause | 許可 | ヘッダの先頭と LICENSE.txt で確認（2026-11-01）。仕様書の文書は CC BY-SA 4.0 なので複製しない。Reference-FMUs は `cargo xtask fetch-reference-fmus` で sha256 を固定して取得（`data/`、コミットしない）。試験の fixture に `modelDescription.xml` 4 つをコミットし、`tests/data/LICENSE` に原文を置いた |
| roxmltree 0.21.2（memchr 2.8、既に Cargo.lock にある） | MIT OR Apache-2.0 | 許可 | S057 で導入（`globe-mode-fmi`。`modelDescription.xml` の読み取り）。crates.io の license 欄と `cargo deny check` で確認、2026-11-01。Cargo.lock に増えたのは roxmltree だけ。wasm32 でも通る |
| FMPy | BSD-2-Clause | 許可（ツール） | 検証の道具。リポジトリの LICENSE.txt で確認、2026-11-01。Python の開発依存に足すかは P5 で決める |
| FMU Compliance Checker（fmuCheck） | BSD-3-Clause | 許可（ツール） | 開発終了、FMI 1/2 のみ。FMI 2 の Export の検査にだけ使う |
| MSL 4.1.0 | BSD-3-Clause | 許可 | 同梱せず取得する。同梱するならライセンス文を付ける |
| cranelift | Apache-2.0 WITH LLVM-exception | 許可 | ネイティブ版のみ |
| PyO3、maturin、Rustler、wasm-bindgen | MIT / Apache-2.0 | 許可 | — |
| rustler 0.39.0（rustler_codegen 0.39.0、inventory 0.3.25、libloading 0.9.0） | rustler・rustler_codegen は MIT/Apache-2.0、inventory は MIT OR Apache-2.0、libloading は ISC（cargo metadata の license 欄と `cargo deny check` で確認、2026-11-02） | 許可 | S082 で導入（`globe-mode-nif` の直接の依存）。Cargo.lock に新しく入るのはこの 4 つ（libloading は Windows で enif_* を引くときだけ使う）。BEAM の API を呼ぶだけで Erlang/OTP（Apache-2.0）はリンクしない。Elixir 側（`src/Elixir/globe_mode`）の Hex の依存は 0。`cargo deny check` は ok |
| pyo3 0.30.1（pyo3-ffi、pyo3-macros、pyo3-macros-backend、pyo3-build-config） | MIT OR Apache-2.0（cargo metadata の license 欄と `cargo deny check` で確認、2026-10-31） | 許可 | S039 で導入（`globe-mode-py` の直接の依存。feature `abi3-py312` は maturin が pyproject.toml の `[tool.maturin]` から立てる）。Cargo.lock に新しく入るのはこの 5 つだけ（heck・target-lexicon・libc・once_cell・portable-atomic・proc-macro2・quote・syn は既にある）。`cargo deny check` は ok |
| maturin 1.x（Python 側のビルド依存） | MIT OR Apache-2.0 | 許可（ツール） | S039 で pyproject.toml の `[build-system]` に入れた（uv_build から切り替え）。wheel のビルドにだけ使い、配布物には入らない。mise には入れない（`uv sync` が隔離環境で取る） |
| pytest 9（Python、開発依存） | MIT | 許可 | S039。`uv run pytest`（`src/Python/tests`）。配布物には入らない。推移的依存は iniconfig・pluggy（どちらも MIT） |
| Svelte、CodeMirror 6、sigma.js、graphology、cosmos.gl、xyflow、maxGraph | MIT / Apache-2.0 | 許可 | npm 側は `pnpm licenses` で確かめる |
| Tauri 2.12（`tauri`、`tauri-build`、`tauri-runtime-wry`、`wry`、`tao`、`webkit2gtk`・`gtk`・`glib`（gtk-rs 0.18、Linux）、`webview2-com`（Windows）など。desktop の `Cargo.lock` の 406 パッケージ） | MIT OR Apache-2.0 が大半（`cargo metadata` の license 欄を集計、2026-10-31: MIT OR Apache-2.0 186、MIT 98、Apache-2.0 OR MIT 30、Unicode-3.0 18、MPL-2.0 5、ほかに Zlib・BSD-3-Clause・Unlicense など）。GPL・AGPL・NC は 0 | 許可（MPL-2.0 の 5 つだけ条件付き） | S049 で導入（`src/Rust/apps/desktop`。**別の Cargo workspace**）。MPL-2.0 は `cssparser`・`cssparser-macros`・`dtoa-short`・`selectors`（tauri-utils → dom_query 経由）と `option-ext`（dirs-sys 経由）。どれも改変せずに使い、`src/Rust/apps/desktop/deny.toml` の `exceptions` に crate ごとに足した。LGPL は `r-efi`（`MIT OR Apache-2.0 OR LGPL-2.1-or-later` の 2 つ。MIT を選ぶので LGPL は適用されない）だけ。保守終了の勧告 RUSTSEC-2024-0370（proc-macro-error。gtk-rs・dom_query の proc-macro でビルド時のみ）を同じ `deny.toml` の `ignore` に理由付きで足した。`cargo deny --manifest-path src/Rust/apps/desktop/Cargo.toml --config src/Rust/apps/desktop/deny.toml check` は ok（advisories・bans・licenses・sources）。ルートの `deny.toml` は desktop を見ない（別 workspace）ので、desktop.yml が同じ検査を回す。Linux の配布物は OS の WebKitGTK（`libwebkit2gtk-4.1`）を動的リンクで使い、同梱しない（WebKitGTK のライセンスは LGPL 系とされる【一次資料での確認は未了】）。Windows は WebView2 ランタイム（OS 側）。公式の `tauri-cli` は `cargo install`（ビルド用のツール。配布物に入らない） |
| tauri-plugin-updater 2.14.1（S135、Desktop の自動更新。推移的に `minisign-verify` 0.3.0（MIT）、`reqwest` の rustls 系（`rustls`・`ring` 0.17・`rustls-platform-verifier`・`webpki-root-certs` 1.0）、`tar`・`tempfile` など。desktop の `Cargo.lock` に 48 パッケージ（新しい版を含む）が増えた） | Apache-2.0 OR MIT。増えた 48 は MIT OR Apache-2.0 系 40、MIT 2、ISC 2、BSD-3-Clause 1、Zlib OR Apache-2.0 OR MIT 1、`ring` の Apache-2.0 AND ISC 1、`webpki-root-certs` の CDLA-Permissive-2.0 1（`cargo metadata` の license 欄、2026-11-03）。GPL・AGPL・NC は 0 | 許可 | `src/Rust/apps/desktop` の Desktop だけ（`cfg(not(any(target_os = "android", target_os = "ios")))`。Android の lib には入らない）。CDLA-Permissive-2.0 は許可の一覧にあり（S053）、desktop の `deny.toml` の `allow` にも足した。`cargo deny … --config src/Rust/apps/desktop/deny.toml check` は advisories・bans・licenses・sources とも ok（2026-11-03）。JS は npm の `@tauri-apps/plugin-updater` を入れず、`withGlobalTauri` が注入する `window.__TAURI__.updater` を使う（既存の `tauri-backend.ts` と同じ方針）。出典: [crates.io: tauri-plugin-updater](https://crates.io/crates/tauri-plugin-updater)（確認日 2026-11-03） |
| tauri-plugin-dialog 2.8.4・tauri-plugin-fs 2.7.1（S141、Desktop の「開く」「保存」。推移的に `rfd` 0.17.2。desktop の `Cargo.lock` に 3 パッケージが増えた） | `tauri-plugin-dialog`・`tauri-plugin-fs` は Apache-2.0 OR MIT、`rfd` は MIT（`cargo metadata` の license 欄、2026-11-04）。GPL・AGPL・NC は 0 | 許可 | `src/Rust/apps/desktop` の Desktop だけ（updater と同じ `cfg(not(any(target_os = "android", target_os = "ios")))`）。`rfd` の Linux の GTK3 は webkit2gtk が既に引く `gtk` と同じ。権限は `capabilities/desktop.json` の 4 つだけ（fs は dialog で選んだパスのみ）。`cargo deny … --config src/Rust/apps/desktop/deny.toml check` は advisories・bans・licenses・sources とも ok（2026-11-04、`deny.toml` の変更なし）。JS は npm の `@tauri-apps/plugin-*` を入れず `window.__TAURI__.dialog` / `fs` を使う。出典: [crates.io: tauri-plugin-dialog](https://crates.io/crates/tauri-plugin-dialog)・[tauri-plugin-fs](https://crates.io/crates/tauri-plugin-fs)（確認日 2026-11-04） |
| Android の Gradle の枠（`src/Rust/apps/desktop/gen/android`。`cargo tauri android init` の生成物）: Gradle Wrapper（`gradle-wrapper.jar`）、Android Gradle Plugin 9.3.1、Kotlin Gradle Plugin 2.2.10、`androidx.webkit` 1.14.0・`androidx.appcompat` 1.7.1・`androidx.activity` 1.10.1 | Apache-2.0【版ごとの一次資料での確認は未了】 | 許可 | S092 で導入。Gradle のプラグインはビルド用で APK に入らない。androidx の 3 つは APK に入る。Rust 側は desktop の `Cargo.lock` から変わっていない（Android 向けの crate も lock に元からある）。APK は署名なしの debug（`android.yml`）で、配布はしていない |
| npm（src/Svelte、S041・S042）: Svelte 5.57、Vite 8.3、@sveltejs/vite-plugin-svelte、CodeMirror 6（state・view・commands・language）、uPlot 1.6（S042、プロット）、svelte-check、typescript 6 | MIT（50）/ Apache-2.0（4）/ ISC（1）/ BSD-3-Clause（1）/ MPL-2.0（2、下記）（`pnpm licenses list`、2026-10-31 確認。推移的依存を含め計 58 パッケージ。配布物に入る prod は CodeMirror の 12 個と uPlot（依存なし）で全部 MIT） | 許可 | MPL-2.0 は Vite 8 が引く `lightningcss`（とその linux-x64 のバイナリ）だけ。ビルド時の CSS 変換にだけ使い、`dist/` にも配布物にも入らず、改変もしないので、cbindgen と同じく許可（ツールとして）。TypeScript は 7 でなく 6 に固定（svelte-check 4.7 が 7 を単独では受けない）。GPL・LGPL・NC は 0 |
| npm（src/Svelte、S046）: sigma 4.0.4、graphology 0.26.0、graphology-layout-forceatlas2 0.10.1（推移的に graphology-utils 2.5.2、graphology-types 0.24.8、events 3.3.0） | 6 パッケージとも MIT（`pnpm licenses list`、registry.npmjs.org の package.json、2026-10-31 確認。S041・S042 の 58 と合わせて計 64 パッケージ） | 許可 | connect グラフの表示（`src/Svelte/src/lib/graph/`）で配布物に入る（prod）。GPL・LGPL・AGPL・NC は 0 |
| Playwright（Python、開発依存） | Apache-2.0 | 許可 | スライドのブラウザ検査（`render_slides.py`）。配布物には入らない |
| mise | MIT | 許可（ツール） | ツールの版の管理。配布物には入らない |
| cbindgen | MPL-2.0（ツール） | 許可（ツールとして） | 生成するヘッダには `header=` で GlobeMode のライセンス表示を入れる |
| uniffi | MPL-2.0 | 条件付き | 今は使わない |
| JointJS core | MPL-2.0 | 条件付き | 回路図ライブラリの第二候補 |
| ELK.js | EPL-2.0 | 条件付き | 改変しないで使う場合に限る |
| KLU、BTF、LDL、CSparse | LGPL-2.1-or-later | 任意の feature に限る | 既定の配布物には入れない |
| UMFPACK、SPQR（russell_sparse 経由を含む） | GPL-2.0-or-later | **禁止** | russell_ode は使わない |
| OpenModelica（omc、OMSimulator、tree-sitter-modelica、modelica-language-server） | OSMC-PL 1.8 / AGPL-3.0 | **禁止（リンク・複製）** | `omc` を別プロセスで呼ぶ照合用途だけはよい |
| Slint | GPL-3.0 / 独自 | 禁止 | — |
| Cosmograph | CC BY-NC 4.0 / 商用 | 禁止 | cosmos.gl（MIT）とは別物 |
| symbolica | source-available | 禁止 | — |
| fdg-sim 0.4 系、fdg-img、fdg-macroquad | GPL-3.0 | 禁止 | 0.6 / 0.9 以降の fdg-sim は MIT |
| Dyad | PolyForm Strict を改変したもの | 禁止 | — |

| npm（src/Svelte、S053）: @codemirror/autocomplete 6.20、@codemirror/lint 6.9 | MIT | 許可 | 補完と波線。CodeMirror の他のパッケージと同じ作者・同じライセンス（`pnpm licenses list`、2026-10-31） |
| npm（src/VSCode/globe-mode、S059）: vscode-languageclient 9.0.1 と推移的に vscode-jsonrpc・vscode-languageserver-protocol・vscode-languageserver-types・minimatch・semver・brace-expansion・balanced-match | MIT（minimatch・semver は ISC） | 許可 | VS Code 拡張の LSP クライアント。`dist/extension.js` に束ねて VSIX に同梱する（`pnpm licenses list`、2026-11-01） |
| npm（src/VSCode/globe-mode、S059、開発依存）: esbuild 0.25.12 | MIT | 許可 | 束ねるだけ。配布物には入らない |
| npm: @vscode/vsce（VSIX を作る公式の CLI） | MIT（本体） | **使わない**（S059） | 推移的に `@vscode/vsce-sign`（ライセンス欄が無い署名用のバイナリ）と WTFPL・Artistic-2.0 などを引き込む。VSIX は `pack_vsix.py`（Python の標準ライブラリだけ）で作る |
| Python（extra `widget`、S078・S097）: anywidget 0.11.0 と推移的に ipywidgets 8.1・ipython 9.18・traitlets 5.16・psygnal 0.16・comm 0.2・jupyterlab-widgets・widgetsnbextension・matplotlib-inline・ipython-pygments-lexers・prompt-toolkit・pexpect・ptyprocess・wcwidth・executing・pure-eval・stack-data・asttokens | MIT（anywidget・wcwidth・executing・pure-eval・stack-data）、BSD-3-Clause（ipywidgets ほか）、ISC（pexpect・ptyprocess）、Apache-2.0（asttokens） | 許可 | `globe_mode.widget.SimulationPlot`（marimo・Jupyter の折れ線）。S097 から extra `widget`（`pip install "globe-mode[widget]"`）で、extra なしの wheel には付かない（wheel に同梱はしない）。インストール済みの METADATA で確認（2026-11-02） |
| ウェブアプリ群（S053、[web-apps.md](web-apps.md)。各アプリは独立したプロジェクトで、ルートの workspace・`pyproject.toml`・配布物には入らない。詳しい表は各アプリの README） | 下記 | 許可 | ライセンスは各アプリの README の表のとおり。確認日 2026-10-31 |
| ├ Rust: axum 0.8（MIT）、tokio（MIT）、actix-web 4.18（MIT OR Apache-2.0）、rocket 0.5.2（MIT OR Apache-2.0）、loco-rs 1.2（Apache-2.0） | MIT / Apache-2.0 | 許可 | `src/Rust/apps/web-*`。独立した workspace で、`cargo deny --config deny.toml check licenses` は axum・actix・rocket で ok。rocket の r-efi（MIT OR Apache-2.0 OR LGPL-2.1-or-later）は MIT を選び、Linux のビルドには入らない。**loco は通らない**: 推移的依存に colored 2.2 / 3.1（MPL-2.0。改変しないので条件付きで許可）、webpki-roots（CDLA-Permissive-2.0）、quoted_printable（0BSD）があり、lettre は loco-rs の feature で外せない。開発時だけ scraper 経由の cssparser・selectors・dtoa-short（MPL-2.0）。どれも改変せず、配布物にも入らないので §1 に足して許可した。ルートの `deny.toml` は変えていない |
| ├ Elixir: phoenix 1.8、bandit、jason、plug、telemetry ほか 16 本 | MIT / Apache-2.0 | 許可 | `src/Elixir/phoenix_fmt`。Hex のメタデータで確認 |
| ├ Gleam: wisp 2.2、mist 6.0、gleam_json、gleam_stdlib ほか 22 本 | Apache-2.0（hpack_erl だけ MIT） | 許可 | `src/Gleam/wisp_fmt`。hex.pm の `meta.licenses` で確認 |
| ├ Gleam: gleam_stdlib 1.0.5、gleeunit 1.11.0（dev） | Apache-2.0 | 許可 | `src/Gleam/globe_mode`（S087）。`wisp_fmt` と同じ版で、上の行に含まれる |
| ├ Python: streamlit 1.66（Apache-2.0）、marimo 0.25（Apache-2.0、ルートの依存）、fastapi 0.162（MIT）、uvicorn（BSD-3）、httpx（BSD-3）、starlette（BSD-3）、pydantic（MIT） | 許可 | 許可 | `src/Python/{streamlit,marimo,fastapi_fmt}`。推移的依存のうち certifi は MPL-2.0（改変しないので条件付きで許可）、typing-extensions は PSF-2.0（§1 に足した）。配布物には入らない |
| ├ TypeScript: astro 7.4（MIT）、@astrojs/node（MIT）、推移的依存 197 本（MIT 166、ISC 11、BSD 11、Apache-2.0 3、CC0 1、BlueOak-1.0.0 3、MPL-2.0 2） | 許可 | 許可 | `src/Astro/fmt`。MPL-2.0 は Vite の `lightningcss`（ビルド時だけ、上と同じ扱い）。astro の optional な `sharp`（LGPL-3.0-or-later）は `pnpm-workspace.yaml` の `ignoredOptionalDependencies` で除外した（画像機能は使わない） |

## 3. 監査の仕組み

- **cargo-deny**（S002 で導入。設定は [deny.toml](../deny.toml)）: `licenses.allow` に §1 の許可リストを入れ、条件付きのものは `exceptions` で個別に許す。
  - 保守終了の勧告（unmaintained）は、Rumoca 経由のものに限って理由を書いて例外にする（bincode、paste）。
  - `bans` に `zenoh*`、`russell_sparse` と、umbrella crate の `rumoca` を入れる。
  - `wrappers = ["globe-mode-frontend", "globe-mode-exec"]` で、Rumoca を使える crate を制限する。
- **cargo-about**（S052 で導入。設定は [about.toml](../about.toml)、テンプレートは [about.hbs](../about.hbs)）: 配布物に入れる `THIRD-PARTY.md` を生成する（4 を参照）。`accepted` は deny.toml の `licenses.allow` と同じ集合で、許可外のライセンスが 1 つでもあれば `--fail` で失敗する。
- **npm 側**: `pnpm licenses list` を CI で検査する。
- **SBOM**: リリースごとに CycloneDX 形式で出力する。
- **生成コード**（S098）: `globe fmu export`（FMI 2.0 / 3.0）が出す `sources/*.c|h`・`modelDescription.xml`・`buildDescription.xml` は Rumoca の組み込みターゲット `fmi2` / `fmi3` のテンプレート（Apache-2.0、改変していない）を描いたもので、テンプレートの C の文面をそのまま多く含む。生成物の扱いを Apache-2.0 は特に定めないので、安全側に倒して Rumoca の Work の派生物として扱い、§4(a)（ライセンスの写しを渡す）と §4(c)（著作権表示を残す）に合わせて、FMU の `documentation/licenses/Rumoca-LICENSE.txt`（FMI 3.0 の規格が使用許諾を置く場所。2.0 でも `documentation/` は自由に置ける）に、出典（リポジトリと rev）・著作権表示・Apache-2.0 の全文を入れる。Rumoca の rev `276e82bd` の LICENSE は Apache-2.0 の全文に `Copyright 2025-2026 CogniPilot Foundation` を書き足したもので、NOTICE は無い（<https://github.com/CogniPilot/rumoca/blob/276e82bd0fd48960ac201e782918d28bf19cf7ce/LICENSE>、cargo の git checkout で確認、2026-11-02）。モデルの式（利用者の Modelica から来た部分）の権利は利用者のもので、GlobeMode は主張しない。

## 4. GlobeMode の配布物に入れるもの

- `LICENSE-MIT`、`LICENSE-APACHE`
- `THIRD-PARTY.md`（cargo-about。`globe` のバイナリに入る Rust の依存の一覧とライセンス文。S052）
- 寄稿の扱い: Apache-2.0 §5 に従い、寄稿は MIT OR Apache-2.0 で受け入れる。README に明記する。

### 配布物のサードパーティのライセンス表（S052、2026-10-31）

- **何を同梱するか**: `release.yml` が作る 3 つのアーカイブ（linux x86_64 / linux aarch64 / windows x86_64）それぞれに、`globe`・`LICENSE-MIT`・`LICENSE-APACHE`・`README.md` と並べて `THIRD-PARTY.md` を入れる。中身は `globe`（globe-mode-cli。feature `sundials` なし）の依存の crate 名・版・リポジトリと、ライセンスごとの全文。
- **作り方**: release.yml のジョブ `third-party` が `cargo install --locked --features cli --version 0.9.2 cargo-about`（EmbarkStudios、MIT OR Apache-2.0）を入れ、次を 1 回だけ回す。3 つのビルドはその artifact を取り込む。

  ```bash
  cargo about generate --locked --fail --manifest-path src/Rust/mod/globe-mode-cli/Cargo.toml about.hbs -o THIRD-PARTY.md
  ```

- **範囲**: `about.toml` の `targets` は 3 つの配布ターゲットの和集合。他のターゲットにしか入らない crate は除く。dev 依存と GlobeMode 自身の crate（`publish = false`）は入れない。build 依存は入れる。
- **許可外のライセンス**: `accepted` が allow の一覧だけなので、満たせない crate があれば cargo-about が失敗し、成果物は出ない（確認の結果は作業記録の `log/work/2026-10-31-s052-release-rc1-about.md`）。
- **含めていないもの**: npm の分（UI は配布物に入らない。Tauri の配布で足す）、wasm と Python の wheel、`sundials` ビルド、CycloneDX の SBOM。
- 出典: https://github.com/EmbarkStudios/cargo-about 、https://embarkstudios.github.io/cargo-about/ （0.9.2 の docs/src、2026-10-31 確認）。
