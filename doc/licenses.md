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
| Rumoca（`rumoca-*`） | Apache-2.0（NOTICE は無い） | 許可 | git の `rev` で固定する。改変した場合は §4b に従って明記する。商標は許諾されないので、製品名に使わない |
| zenoh | EPL-2.0 OR Apache-2.0 | **禁止（`bans`）** | IDE に要らない。Rumoca の umbrella crate を使わなければ入らない |
| diffsol | MIT | 許可 | 既定のソルバ |
| SUNDIALS（sundials-sys） | BSD-3-Clause | 許可 | feature `sundials`。wasm では使えない |
| faer、egg、salsa、rowan、petgraph、lsp-types | MIT / Apache-2.0 | 許可 | — |
| serde、serde_json、clap | MIT OR Apache-2.0 | 許可 | S002 で導入 |
| sha2 | MIT OR Apache-2.0 | 許可 | S003 で導入（xtask の sha256） |
| salsa 0.28.5 | Apache-2.0 OR MIT | 許可 | S004 で導入（`globe-mode-db`）。推移的な依存も cargo deny で ok |
| zip | MIT | 許可 | S003 で導入（xtask で MSL を展開）。**9 系を使う**: 2 系には展開時の脆弱性（RUSTSEC-2026-0367 / 0368 / 0369）がある |
| MSL 4.1.0、Rumoca の CLI（照合用） | BSD-3-Clause / Apache-2.0 | 許可 | `cargo xtask fetch-*` で sha256 を固定して取得する。配布物には入れない（`data/` は git 管理外） |
| cargo-deny | MIT OR Apache-2.0 | 許可（ツール） | mise で入れる。配布物には入らない |
| fmi、fmi-schema、fmi-sys、fmi-export | MIT OR Apache-2.0 | 許可 | — |
| FMI ヘッダ、Reference-FMUs | BSD-2-Clause | 許可 | — |
| MSL 4.1.0 | BSD-3-Clause | 許可 | 同梱せず取得する。同梱するならライセンス文を付ける |
| cranelift | Apache-2.0 WITH LLVM-exception | 許可 | ネイティブ版のみ |
| PyO3、maturin、Rustler、wasm-bindgen | MIT / Apache-2.0 | 許可 | — |
| Tauri 2、Svelte、CodeMirror 6、sigma.js、graphology、cosmos.gl、xyflow、maxGraph | MIT / Apache-2.0 | 許可 | npm 側は `pnpm licenses` で確かめる |
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
