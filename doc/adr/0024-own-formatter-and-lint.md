# ADR-0024 整形器と lint は Rumoca に依存しない自前の crate `globe-mode-fmt` で作り、CLI・LSP・wasm・ウェブアプリ群の共通のエンジンにする

- 状態: Accepted（2026-10-31、S053）
- 関係: [ADR-0002](0002-rumoca-adapter.md)（Rumoca はアダプタからだけ使う）、[ADR-0023](0023-desktop-and-web-process-layout.md)（Desktop は `globe` の stdio、Web は Web Worker）。規則は [format-style.md](../format-style.md)、ウェブアプリ群の契約は [web-apps.md](../web-apps.md)

## 背景

- [features.md](../features.md) は整形・lint を「Rumoca の tool-fmt / tool-lint をアダプタ経由で使う」としていた。上流の `rumoca-tool-fmt`（Apache-2.0、同じ rev）は、インデントの正規化・演算子と `=` の空白・行末の空白・末尾の改行の正規化だけを持ち、**行幅での折り返し・annotation の引数の並べ替え・空行の規則・`origin` の補完**を持たない（`crates/rumoca-tool-fmt/src/format_options.rs` の `FormatOptions`、2026-10-31 確認）。ユーザーが対話で決めた規則（[format-style.md](../format-style.md)）はこの 4 つを要る。
- Rumoca の AST はコメントと空白を持たないので、上流の整形器もトリビアを別に扱っている。自前で書いても「字句解析 → 文の切り出し → 配置」の 3 段で足りる。
- ユーザーは IDE のほかに CLI・VS Code 拡張・ウェブアプリ（marimo、Streamlit、Phoenix、Astro、Axum、Actix-web、Rocket、loco など）から**単体で**動かせることを求めている。Rumoca を引くと wasm と配布物が重く、CLAUDE.md の「Rumoca は frontend と exec からだけ」とも合わない。

## 決定

1. 整形器と lint は新しい crate `globe-mode-fmt`（`src/Rust/mod/globe-mode-fmt`）に置く。**依存 0**、`unsafe` 禁止、wasm32 でビルドできる。公開 API は `format(src, &Options) -> Result<String, FormatError>`、`lint(src, &LintOptions) -> Vec<LintMessage>`、`KEYWORDS`（補完用）。
2. 整形の出力は字句解析し直して、空白以外のトークン列が意図どおりであることを確かめる。違えば `Err` で本文に触らない。文法が壊れた入力（括弧・`end` の不一致、`;` の欠け）も `Err`。
3. 入口は 4 つ。CLI `globe fmt` / `globe lint`（標準入力の `--json` が Rust 以外のアプリの契約）、LSP の `textDocument/formatting` と lint の診断（`GM-L0001`〜`GM-L0003`、warning）、wasm の `format` / `lint`、ブラウザ UI の「整形」と波線。補完は `textDocument/completion` で、予約語と `Outline` の名前だけ（新しい解析は入れない）。
4. ウェブアプリ群は [web-apps.md](../web-apps.md) の契約（画面・`POST /api/format` / `POST /api/lint` の JSON）に従い、Rust のアプリは crate を、他は CLI を子プロセスで呼ぶ。各アプリは独立したプロジェクト（ルートの workspace・`pyproject.toml`・CI に入れない）。
5. 上流の `rumoca-tool-fmt` / `rumoca-tool-lint` は採用しない。features.md の該当行を書き換える。

## 結果

- S053: 整形器のテスト（ユーザーの Before/After の一致、`src/Modelica` の 69 ファイルで冪等・末尾改行・行末空白なし、壊れた入力の `Err`、インデント 4、lint 3 規則）、LSP の統合テスト（formatting・completion・lint）、ブラウザ UI の試験（整形ボタンと波線）が通る。ウェブアプリは 10 種（axum、actix-web、Rocket、loco、Phoenix、Gleam wisp、Streamlit、marimo、FastAPI、Astro）。
- 既存の `src/Modelica/` は 69 本すべてが整形で変わる（クラス頭の後ろと `end` の前の空行）。書き換えは別のセッションでユーザーが決める。
- 持ち越し: 設定ファイル（`globe-fmt.toml`）、部品の型を辿る深い補完、定義ジャンプ、VS Code 拡張の実体、行コメントを含む長い式の折り返し。
- S143（2026-11-04）: Desktop も設定ファイルを読む（[ADR-0023](0023-desktop-and-web-process-layout.md) の S143）。LSP の独自の要求 `globe/format`・`globe/lint` が設定を見ていなかったので、`textDocument/formatting` と同じく設定を土台にし、params のキーがあれば優先する形にした。
- S130（2026-11-03）: 設定ファイルを `globe-mode.toml` の名前で足した（`[fmt]` の `indent`・`width`・`insert_origin`・`blank_around_sections`・`format_html`、`[lint]` の `disable`。[format-style.md](../format-style.md) の 5 章）。読む処理は `globe-mode-fmt` の **optional な feature `config`**（`LintOptions::from_toml`・`LintOptions::find`、依存は `toml` 0.8 だけ）に置き、CLI と LSP だけが立てる。理由: 読む処理を CLI と LSP に別々に書くと規則（知らないキーの扱い、規則のコードと名前の対応 `RULES`）が 2 か所に分かれる。既定の feature は空のままなので、**決定 1 の「依存 0」は既定のビルドと wasm では保つ**。`toml` 0.8.23 は rumoca-core の依存として `Cargo.lock` に既にあり、新しい crate は入らない。決定 3 の LSP の `textDocument/formatting` は、設定ファイルがあれば `tabSize` より設定ファイルの `indent` を採る。持ち越しは VS Code 拡張の設定 UI とエディタのオプションとの同期。
