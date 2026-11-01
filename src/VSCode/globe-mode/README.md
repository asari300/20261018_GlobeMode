# GlobeMode Modelica（VS Code 拡張）

S059 で作った。`.mo` を開くと `globe lsp` を起動してつなぐだけの小さな拡張で、整形・lint・補完・検査は Rust 側（`globe-mode-lsp`・`globe-mode-fmt`）が受け持つ。整形の規則は [doc/format-style.md](../../../doc/format-style.md)。

## できること

| 機能 | 操作 |
| --- | --- |
| 整形 | Shift+Alt+F（拡張が `.mo` に割り当てる。Windows の VS Code の標準と同じキーで、Linux でも同じ）。コマンドパレットの「ドキュメントのフォーマット」でもよい |
| 保存時の整形 | 設定 `globeMode.formatOnSave`（既定はオフ）。設定画面で「globeMode」と検索してチェックを入れる。再読み込みは要らない |
| lint と検査の診断 | 開いて編集すると波線が出る（`GM-L0001`〜`0003` と検査の `GM-E…`）。一覧は Ctrl+Shift+M |
| 補完 | Ctrl+Space、または `.` のあと |
| アウトライン・ホバー | エクスプローラーのアウトライン、クラス名にマウスを載せる |

拡張は `.mo` の既定を `editor.tabSize: 2`・80 桁のルーラーにする。インデント 4 にしたいときだけ、設定で `[modelica]` の `editor.tabSize` を 4 にする。

## 入れ方

```bash
cargo build --release -p globe-mode-cli          # target/release/globe
cd src/VSCode/globe-mode
mise exec -- pnpm install
mise exec -- pnpm package                        # globe-mode-<版>.vsix
```

できた VSIX を VS Code に入れる。**プロファイルを使っているときは、画面から入れる**（CLI の `code --install-extension` は既定のプロファイルに入れるので、別のプロファイルの窓には出てこない）。

1. 拡張機能ビュー（Ctrl+Shift+X）の右上の「…」→「VSIX からのインストール...」で `globe-mode-<版>.vsix` を選ぶ。
2. 「ウィンドウの再読み込み」。
3. `.mo` を開き、右下の言語が「Modelica」になっていることを確かめる。ほかの言語（Lua など）のときは、そこを押して Modelica を選ぶ。

入ったかどうかは、拡張機能ビューの検索欄に `@installed GlobeMode` と打つと見える。Remote - WSL の窓では「WSL: …」の欄に出る。

`globe` が PATH に無いときは、設定 `globeMode.path` に絶対パスを書く。MSL などのライブラリは `globeMode.libraries` に並べる（`globe lsp --library` に渡る）。どちらも変えたら「ウィンドウの再読み込み」。

WSL の中のファイルを Windows の VS Code で開くときは、Remote - WSL の窓で入れる。拡張は `extensionKind: workspace` なので、`globe` と同じ WSL の側で動く。

外すときは拡張機能ビューで「アンインストール」。

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
