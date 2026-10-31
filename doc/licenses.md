# ライセンス方針

- GlobeMode は **MIT OR Apache-2.0** のデュアルライセンスで公開する（[ADR-0001](adr/0001-license-policy.md)）。
- この文書は依存を足すときの判断基準と、現時点の判定の一覧。依存を足す手順は `.claude/skills/add-dependency/SKILL.md` にある。
- 法的助言ではない。判断に迷ったら、依存を足す前に作業記録に書いて相談する。

## 1. 区分

| 区分 | ライセンス | 扱い |
| --- | --- | --- |
| **許可** | MIT、Apache-2.0、Apache-2.0 WITH LLVM-exception、BSD-2-Clause、BSD-3-Clause、ISC、Zlib、Unicode-3.0、CC0-1.0、BSL-1.0 | そのまま依存してよい。配布物にライセンス文を同梱する |
| **条件付き** | MPL-2.0、EPL-2.0（別ライセンスとの OR を含む） | 改変しないで使う場合に限る。改変するなら、そのファイルを元のライセンスのまま公開する。cargo-deny では個別に例外として登録する |
| **任意の feature に限る** | LGPL-2.1-or-later（KLU など） | 既定の配布物には含めない。利用者が自分でビルドする任意の feature に限る |
| **禁止** | GPL-2.0 / 3.0、AGPL-3.0、OSMC-PL、SSPL、CC BY-NC、PolyForm（非商用）、独自の source-available | 依存しない。リンクもしない。コードを複製しない |

## 2. 主な依存の判定（2026-10-18）

| 対象 | ライセンス | 判定 | メモ |
| --- | --- | --- | --- |
| Rumoca（`rumoca-*`） | Apache-2.0（NOTICE は無い） | 許可 | git の `rev` で固定する。改変した場合は §4b に従って明記する。商標は許諾されないので、製品名に使わない。S012: `globe-mode-exec/src/rumoca/plugins/rk.rs` は Rumoca の `crates/rumoca-solver-diffsol/src/me_integrator.rs`（Apache-2.0）を写して改変したもので、ファイルの先頭に明記した |
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
| getrandom 0.3 | MIT OR Apache-2.0 | 許可 | S012 で `globe-mode-exec` の wasm32 のときだけの直接の依存（feature `wasm_js`。diffsol → faer → rand の乱数の backend）。Cargo.lock に既にある |
| SUNDIALS（sundials-sys 0.6.2） | BSD-3-Clause（sundials-sys、同梱の SUNDIALS 7.1.1 とも。sundials-sys の `vendor/LICENSE` で確認、2026-10-30） | 許可 | S032 で導入（S034 で IDA を足した）。`globe-mode-exec` の optional な依存で、feature `sundials` を立てたときだけ入る（CLI は `--features sundials` で転送）。wasm では使えない。sundials-sys の feature は `cvode`・`ida`・`build_libraries`・`static_libraries` だけ（`default-features = false` で arkode・kinsol を切る）で、同梱のソースから cmake で CVODE・IDA と serial N_Vector・密な行列・密な線形ソルバ・Newton 法などを静的ライブラリにしてリンクする。`klu`（LGPL。suitesparse_sys が入る）・`nvecopenmp`・`nvecpthreads` は使わない。ビルド時の依存: bindgen 0.68.1（BSD-3-Clause）・cmake 0.1.58（MIT OR Apache-2.0）と、bindgen の推移的依存（clang-sys、cexpr、nom、which、peeking_take_while、lazycell、libloading、prettyplease など）。ビルドには cmake・C コンパイラ・libclang が要る。`cargo deny --all-features check` は ok（advisories・bans・licenses・sources。2026-10-30）。`deny.toml` は `all-features = false` のままで、既定の `cargo deny check` はこの依存を見ない（Cargo.lock には載る）。個々の推移的依存のライセンスの確認は cargo-deny の判定に任せた【crates.io での個別の確認は未確認、2026-10-30】 |
| faer、egg、salsa、rowan、petgraph、lsp-types | MIT / Apache-2.0 | 許可 | — |
| lsp-types 0.97.0（fluent-uri 0.1.4、serde_repr 0.1.21） | MIT OR Apache-2.0（fluent-uri は MIT。crates.io の metadata で確認、2026-10-30） | 許可 | S038 で導入（`globe-mode-lsp`。型だけ使い、JSON-RPC は自前）。推移的依存で新しく入るのは fluent-uri と serde_repr の 2 つ。`cargo deny check` は ok |
| serde、serde_json、clap | MIT OR Apache-2.0 | 許可 | S002 で導入 |
| sha2 | MIT OR Apache-2.0 | 許可 | S003 で導入（xtask の sha256） |
| salsa 0.28.5 | Apache-2.0 OR MIT | 許可 | S004 で導入（`globe-mode-db`）。推移的な依存も cargo deny で ok |
| rumoca-ir-dae、rumoca-eval-dae、rumoca-core | Apache-2.0（Rumoca のワークスペース） | 許可 | S005 で導入（`globe-mode-frontend` だけ）。DAE の incidence を自前で作るため。rumoca-compile の推移的依存なので、新しい外部依存は増えない |
| zip | MIT | 許可 | S003 で導入（xtask で MSL を展開）。**9 系を使う**: 2 系には展開時の脆弱性（RUSTSEC-2026-0367 / 0368 / 0369）がある |
| MSL 4.1.0、Rumoca の CLI（照合用） | BSD-3-Clause / Apache-2.0 | 許可 | `cargo xtask fetch-*` で sha256 を固定して取得する。配布物には入れない（`data/` は git 管理外） |
| cargo-deny | MIT OR Apache-2.0 | 許可（ツール） | mise で入れる。配布物には入らない |
| fmi、fmi-schema、fmi-sys、fmi-export | MIT OR Apache-2.0 | 許可 | — |
| FMI ヘッダ、Reference-FMUs | BSD-2-Clause | 許可 | — |
| MSL 4.1.0 | BSD-3-Clause | 許可 | 同梱せず取得する。同梱するならライセンス文を付ける |
| cranelift | Apache-2.0 WITH LLVM-exception | 許可 | ネイティブ版のみ |
| PyO3、maturin、Rustler、wasm-bindgen | MIT / Apache-2.0 | 許可 | — |
| pyo3 0.30.1（pyo3-ffi、pyo3-macros、pyo3-macros-backend、pyo3-build-config） | MIT OR Apache-2.0（cargo metadata の license 欄と `cargo deny check` で確認、2026-10-31） | 許可 | S039 で導入（`globe-mode-py` の直接の依存。feature `abi3-py312` は maturin が pyproject.toml の `[tool.maturin]` から立てる）。Cargo.lock に新しく入るのはこの 5 つだけ（heck・target-lexicon・libc・once_cell・portable-atomic・proc-macro2・quote・syn は既にある）。`cargo deny check` は ok |
| maturin 1.x（Python 側のビルド依存） | MIT OR Apache-2.0 | 許可（ツール） | S039 で pyproject.toml の `[build-system]` に入れた（uv_build から切り替え）。wheel のビルドにだけ使い、配布物には入らない。mise には入れない（`uv sync` が隔離環境で取る） |
| pytest 9（Python、開発依存） | MIT | 許可 | S039。`uv run pytest`（`src/Python/tests`）。配布物には入らない。推移的依存は iniconfig・pluggy（どちらも MIT） |
| Tauri 2、Svelte、CodeMirror 6、sigma.js、graphology、cosmos.gl、xyflow、maxGraph | MIT / Apache-2.0 | 許可 | npm 側は `pnpm licenses` で確かめる |
| npm（src/Svelte、S041）: Svelte 5.57、Vite 8.3、@sveltejs/vite-plugin-svelte、CodeMirror 6（state・view・commands・language）、svelte-check、typescript 6 | MIT（49）/ Apache-2.0（4）/ ISC（1）/ BSD-3-Clause（1）/ MPL-2.0（2、下記）（`pnpm licenses list`、2026-10-31 確認。推移的依存を含め計 57 パッケージ。配布物に入る prod は CodeMirror の 12 個で全部 MIT） | 許可 | MPL-2.0 は Vite 8 が引く `lightningcss`（とその linux-x64 のバイナリ）だけ。ビルド時の CSS 変換にだけ使い、`dist/` にも配布物にも入らず、改変もしないので、cbindgen と同じく許可（ツールとして）。TypeScript は 7 でなく 6 に固定（svelte-check 4.7 が 7 を単独では受けない）。GPL・LGPL・NC は 0 |
| npm（src/Svelte、S046）: sigma 4.0.4、graphology 0.26.0、graphology-layout-forceatlas2 0.10.1（推移的に graphology-utils 2.5.2、graphology-types 0.24.8、events 3.3.0） | 6 パッケージとも MIT（`pnpm licenses list`、registry.npmjs.org の package.json、2026-10-31 確認。S041 の 57 と合わせて計 63 パッケージ） | 許可 | connect グラフの表示（`src/Svelte/src/lib/graph/`）で配布物に入る（prod）。GPL・LGPL・AGPL・NC は 0 |
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

## 3. 監査の仕組み

- **cargo-deny**（S002 で導入。設定は [deny.toml](../deny.toml)）: `licenses.allow` に §1 の許可リストを入れ、条件付きのものは `exceptions` で個別に許す。
  - 保守終了の勧告（unmaintained）は、Rumoca 経由のものに限って理由を書いて例外にする（bincode、paste）。
  - `bans` に `zenoh*`、`russell_sparse` と、umbrella crate の `rumoca` を入れる。
  - `wrappers = ["globe-mode-frontend", "globe-mode-exec"]` で、Rumoca を使える crate を制限する。
- **cargo-about**: 配布物に入れる `THIRD_PARTY_LICENSES.html` を生成する。
- **npm 側**: `pnpm licenses list` を CI で検査する。
- **SBOM**: リリースごとに CycloneDX 形式で出力する。
- **生成コード**: Rumoca のテンプレートで生成したコードのライセンスは P5a で確かめ、この表に書き足す。

## 4. GlobeMode の配布物に入れるもの

- `LICENSE-MIT`、`LICENSE-APACHE`
- `THIRD_PARTY_LICENSES.html`（cargo-about と npm の分）
- 寄稿の扱い: Apache-2.0 §5 に従い、寄稿は MIT OR Apache-2.0 で受け入れる。README に明記する。
