# UI・グラフ描画・言語バインディングの調査

- 確認日: 2026-10-18
- 確認できなかった項目には【未確認】と書いた。法的助言ではない。

## アプリのシェル

| 候補 | ライセンス | 対応先 | 評価 |
| --- | --- | --- | --- |
| **Tauri 2**（v2.11.5、2026-07-01） | Apache-2.0 OR MIT | Windows 10 以降、Linux（webkit2gtk-4.1）、Android 7 以降、iOS | **本線**。ブラウザ向けの target は無いので、同じフロントエンドを静的サイトとして配信する。公式プラグインの一部はモバイルに未対応 |
| Dioxus 0.7（2025-09-08。0.8-alpha あり） | MIT OR Apache-2.0 | Web は DOM、デスクトップとモバイルは WebView | **試験**（P7）。WGPU のネイティブ描画（Blitz）とホットパッチは実験段階 |
| Leptos | MIT | Web。デスクトップと Android は Tauri 経由 | **試験**（P7）。Tauri のガイドは Leptos 0.6 向け |
| egui / eframe | MIT OR Apache-2.0 | Web、Windows、Linux、macOS、Android | API の破壊的変更が多い。テキストウィジェットが弱く、本格的なエディタには向かない |
| Slint | GPL-3.0-only OR 独自の royalty-free OR 商用 | Android は 1.5 以降 | **不採用**。GPL を選ぶと配布物が GPL になる。royalty-free は OSI 承認ではない |
| Iced | MIT | デスクトップ、Web | 実験段階。Android は公式には無い【未確認】 |
| Makepad | MIT | wasm、デスクトップ、モバイル | エコシステムが小さい。CodeMirror や Monaco を埋め込めない |

## コードエディタ

- **CodeMirror 6**（MIT）を採用する。モバイルで良く動く。
  - `@codemirror/lsp-client`（MIT）は、自前で書いた Transport を通してサーバーと話す。
  - Transport を Web Worker への postMessage にすれば、wasm 化した言語サーバーにつなげられる。
- Monaco（MIT）は機能が多いが、モバイルブラウザでは非対応とされる【未確認】。

## グラフ描画（connect のネットワークグラフ）

| 候補 | ライセンス | 評価 |
| --- | --- | --- |
| **sigma.js + graphology** | MIT | **採用**。WebGL で描き、graphology の ForceAtlas2 でレイアウトする。Obsidian のグラフビューに最も近い |
| **cosmos.gl** | MIT | **大規模なグラフ用**。WebGL2 の GPU で力学シミュレーションを回し、数十万ノードを扱える。OpenJS Foundation で育成中 |
| Cosmograph（アプリとライブラリ） | **CC BY-NC 4.0** または商用 | **使用禁止**。cosmos.gl と名前が紛らわしいので注意する |
| force-graph / 3d-force-graph | MIT | 試作が最も速い。canvas と d3-force を使い、例では約75k 要素まで動く |
| d3-force | ISC | 可 |
| cytoscape.js | MIT | 可（今回は詳しく調べていない） |
| egui_graphs | MIT | petgraph ベース。O(n²) の FR レイアウトで、開発は止まっている |
| petgraph | MIT OR Apache-2.0 | Rust 側のグラフ構造に採用する |
| fdg-sim | 0.6 / 0.9 以降は MIT。**0.4 系と fdg-img・fdg-macroquad は GPL-3.0** | 使うなら版の固定が要る |
| layout-rs | MIT | Graphviz 風の階層レイアウト（v0.1.3） |

### 回路図（Diagram ビュー）

| 候補 | ライセンス | 評価 |
| --- | --- | --- |
| **Svelte Flow / React Flow**（xyflow） | MIT | 第一候補（Svelte 5 と相性が良い） |
| **maxGraph** | Apache-2.0 | mxGraph の後継。開発は活発 |
| JointJS core | MPL-2.0（JointJS+ は商用） | 可（ファイル単位のコピーレフト） |
| ELK.js | EPL-2.0 | 改変せずに使うなら可。GPL とは互換性が無い。Eclipse で GPLv3 を副ライセンスに加える提案があるが、結論は【未確認】 |

## 言語バインディング

```text
globe-mode（facade、純 Rust）
 ├─ globe-mode-capi  extern "C"、不透明ハンドル、JSON / バイト列。cbindgen でヘッダを生成
 │    ├─ Zig:   @cImport でヘッダを読む
 │    └─ Julia: ccall と BinaryBuilder の JLL（jlrs は任意）
 ├─ globe-mode-py    PyO3 と maturin（abi3-py312）
 ├─ globe-mode-nif   Rustler → Elixir。Gleam は @external で同じ Erlang モジュールを呼ぶ
 └─ globe-mode-wasm  wasm-bindgen → ブラウザの Worker、Gleam の JS target、marimo の anywidget
```

| 対象 | 道具 | ライセンス | 注意 |
| --- | --- | --- | --- |
| Python | PyO3 / maturin | Apache-2.0 OR MIT / MIT OR Apache-2.0 | Streamlit も同じ wheel を使う |
| marimo | anywidget | MIT | ESM 側で wasm-bindgen のモジュールを読む（Pyodide とは独立）。静的 HTML の出力では 0.19.10 以降で描画される。`html-wasm` の出力で動くかは【未確認】 |
| marimo（代替） | Pyodide 用の PyO3 wheel | — | nightly と、Pyodide と同じ emsdk（約 5.0.3）と `-Z emscripten-wasm-eh` が要る。marimo の WASM はメモリが 2 GB まで、スレッドはエミュレーション |
| Elixir | Rustler / rustler_precompiled | Apache-2.0 OR MIT / Apache-2.0 | 最新の OTP 3版に対応 |
| Gleam | `@external` | — | Erlang target では NIF を、JS target では wasm を呼ぶ |
| Zig | cbindgen | ツールは MPL-2.0 | 生成したヘッダは自前のコードから作られるので、`header=` で自前のライセンス表示を入れられる（Apache OpenDAL と同じ）。ヘッダが MPL の対象にならないのは一般的な解釈【未確認】 |
| Julia | ccall（jlrs は MIT、Julia 1.10〜1.13） | — | C ABI の ccall の方が単純で安定している |
| Kotlin（Android） | uniffi | MPL-2.0 | UI を Tauri にするので要らない |
| ブラウザ | wasm-bindgen | MIT OR Apache-2.0 | — |

## 既存の取り組み

| 名前 | ライセンス | メモ |
| --- | --- | --- |
| Rumoca | Apache-2.0 | 最も近い。[rumoca.md](rumoca.md) |
| modelica-language-server（OpenModelica） | OSMC-PL 1.8 | tree-sitter ベースで実験段階（v0.3.6）。**組み込まない** |
| moparse | MPL-2.0 | 作業中の Rust パーサ |
| modelica-rs | — | ANTLR ベース。2023 年から更新が無い |
| Dyad（JuliaHub） | 改変した PolyForm Strict（非商用のみ） | OSS ではない。ModelingToolkit.jl に変換する |
| ModelingToolkit.jl / Modia.jl | MIT【未確認】 | — |
| OMJulia | BSD-3 / GPL-3 / OSMC-PL から選べる | — |
| Modiator / Tiller の playground / Bodylight.js | コードのライセンスは見つからなかった | ブラウザ内で Modelica や FMU を動かした先例 |
| Modelon Impact、Wolfram System Modeler、MapleSim | 商用 | 機能の比較対象 |

## 出典

- <https://github.com/tauri-apps/tauri>
- <https://v2.tauri.app/release/tauri/>
- <https://dioxuslabs.com/blog/release-070>
- <https://v2.tauri.app/start/frontend/leptos/>
- <https://github.com/emilk/egui>
- <https://slint.dev/blog/slint-1.5-released>
- <https://github.com/iced-rs/iced>
- <https://github.com/makepad/makepad>
- <https://discuss.codemirror.net/t/codemirror-lsp-client/9309>
- <https://github.com/TypeFox/monaco-languageclient>
- <https://github.com/jacomyal/sigma.js>
- <https://github.com/cosmosgl/graph>
- <https://openjsf.org/blog/introducing-cosmos-gl>
- <https://cosmograph.app/licensing/>
- <https://github.com/vasturiano/force-graph>
- <https://github.com/blitzarx1/egui_graphs>
- <https://docs.rs/crate/fdg-sim/latest>
- <https://github.com/nadavrot/layout>
- <https://github.com/maxGraph/maxGraph>
- <https://github.com/clientIO/joint>
- <https://gitlab.eclipse.org/eclipsefdn/emo-team/emo/-/work_items/1257>
- <https://github.com/PyO3/pyo3>
- <https://docs.marimo.io/guides/wasm/>
- <https://docs.marimo.io/api/inputs/anywidget/>
- <https://simonwillison.net/2026/Feb/6/pyo3-pyodide-wasm/>
- <https://github.com/rusterlium/rustler>
- <https://gleam.run/documentation/externals/>
- <https://github.com/Taaitaaiger/jlrs>
- <https://github.com/mozilla/uniffi-rs>
- <https://github.com/EmbarkStudios/cargo-deny>
- <https://github.com/OpenModelica/modelica-language-server>
- <https://juliahub.com/blog/why-dyad-a-perspective-for-modelica-users>
- <https://ecp.ep.liu.se/index.php/modelica/article/view/1307>
- <https://ecp.ep.liu.se/index.php/modelica/article/view/628>
