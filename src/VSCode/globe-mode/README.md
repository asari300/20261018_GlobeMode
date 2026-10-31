# GlobeMode Modelica（VS Code 拡張）

S059 で作った。`.mo` を開くと `globe lsp` を起動してつなぐだけの小さな拡張で、整形・lint・補完・検査は Rust 側（`globe-mode-lsp`・`globe-mode-fmt`）が受け持つ。整形の規則は [doc/format-style.md](../../../doc/format-style.md)。

## できること

| 機能 | 操作 |
| --- | --- |
| 整形 | 標準の「ドキュメントのフォーマット」。Linux は Ctrl+Shift+I、Windows は Shift+Alt+F。保存時に整形するなら設定 `editor.formatOnSave` |
| lint と検査の診断 | 開いて編集すると波線が出る（`GM-L0001`〜`0003` と検査の `GM-E…`）。一覧は Ctrl+Shift+M |
| 補完 | Ctrl+Space、または `.` のあと |
| アウトライン・ホバー | エクスプローラーのアウトライン、クラス名にマウスを載せる |

拡張は `.mo` の既定を `editor.tabSize: 2`・80 桁のルーラーにする。インデント 4 にしたいときだけ、設定で `[modelica]` の `editor.tabSize` を 4 にする。

## 入れ方

```bash
cargo build --release -p globe-mode-cli          # target/release/globe
cd src/VSCode/globe-mode
mise exec -- pnpm install
mise exec -- pnpm package                        # globe-mode-0.0.1.vsix
code --install-extension globe-mode-0.0.1.vsix
```

`globe` が PATH に無いときは、設定 `globeMode.path` に絶対パスを書く。MSL などのライブラリは `globeMode.libraries` に並べる（`globe lsp --library` に渡る）。どちらも変えたら「ウィンドウの再読み込み」。

WSL の中のファイルを Windows の VS Code で開くときは、Remote - WSL の窓で入れる。拡張は `extensionKind: workspace` なので、`globe` と同じ WSL の側で動く。

外すときは `code --uninstall-extension globe-mode.globe-mode`。

## 検査

```bash
GLOBE_BIN=$PWD/../../../target/debug/globe mise exec -- pnpm test
```

`globe lsp` を VS Code と同じ形（`file://` の URI、拡張の既定の tabSize）で起こし、整形・lint の診断・補完を 1 往復ずつ確かめる。VS Code の本体は起動しない。

## 依存

| 名前 | 版 | ライセンス | 用途 | 確認日 |
| --- | --- | --- | --- | --- |
| vscode-languageclient（と vscode-jsonrpc ほか推移的に 5 つ） | 9.0.1 | MIT（minimatch・semver は ISC） | LSP のクライアント。`dist/extension.js` に束ねて同梱 | 2026-11-01 |
| esbuild | 0.25.12 | MIT | 束ねるだけ（開発依存） | 2026-11-01 |

`@vscode/vsce` は使わない。ライセンスを確かめられない署名用のバイナリ `@vscode/vsce-sign` を引き込むため。VSIX は `pack_vsix.py`（Python の標準ライブラリだけ）で作る。
