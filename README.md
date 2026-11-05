# GlobeMode

**An open-source Modelica IDE written in Rust.**

GlobeMode checks, simulates and edits [Modelica](https://modelica.org/) models. The same Rust core runs as a CLI (`globe`), a language server, a Tauri desktop app (Linux, Windows, macOS), an Android app, in the browser as WebAssembly (the computation stays on the visitor's machine), in Python, and from C, Zig, Julia, Elixir and Gleam. The compiler frontend is [Rumoca](https://github.com/CogniPilot/rumoca) (Apache-2.0, pinned git dependency) behind an adapter.

> **Status: pre-release.** Nothing is published yet: no GitHub Release, tag, GitHub Pages site, PyPI package, crates.io crate or VS Code Marketplace entry. Build from a checkout, or take the build artifacts of the GitHub Actions workflows. The desktop and Android builds are unsigned. Details are in Japanese below ("使い始め方"); changes are in [CHANGELOG.md](CHANGELOG.md).

## What works

- **Checks before simulation** (S0 to S10): syntax, names, types, connections, equation/variable balance, structural singularity, index, initialization, algebraic loops, parameter and start-value evaluation. Errors name the reason a model cannot be simulated. The balance check agrees with OpenModelica's `omc checkModel()` on all 382 MSL examples that have a verdict ([doc/omc-compare.md](doc/omc-compare.md)).
- **Simulation** with 20+ solvers (`bdf`, `dopri45`, `auto`, `esdirk34`, `radau5`, `rk4`, ...; `cvode` and `ida` only in a build with the `sundials` feature), parameter overrides, CSV and MAT v4 output, progress and cancel.
- **Editing**: formatter, lint, completion, hover, go-to-definition, references and rename (components and class names, within the open document) in the language server, the VS Code extension and the browser/desktop/Android UI; library tree, plot, connect graph, tabs, open/save, dark mode.
- **FMU export** (FMI 2.0 and 3.0, Model Exchange and Co-Simulation) from the CLI, the UI and every language API. FMI import is not implemented.
- **APIs**: Rust, Python (PyO3), C ABI, Zig, Julia, Elixir, Gleam, WebAssembly.

Not done: FMI import, SSP, Diagram/Icon/Documentation views, an own compiler frontend (Track B), signed installers and automatic updates (the updater is off until a key is registered), a public web deployment. The full list with priorities is [doc/features.md](doc/features.md); the plan is [doc/roadmap.md](doc/roadmap.md).

UI keyboard shortcuts (browser, Desktop, Android with a keyboard; Cmd instead of Ctrl on macOS; they work inside the editor and in the input fields). Diagnostic and reference rows are real buttons: Tab to one and press Enter or Space to jump to the editor.

| Keys | Action |
| --- | --- |
| Ctrl+Enter | Run (this replaces CodeMirror's "insert blank line") |
| Ctrl+Shift+Enter | Check |
| Ctrl+. | Stop a running simulation |
| Ctrl+S / Ctrl+Shift+S | Save / Save as (Desktop) |
| Ctrl+O | Open |
| F12, Shift+F12, F2, Alt+Left | Go to definition, references, rename, back (editor) |

## Getting started

All of these start from a checkout (`git clone https://github.com/asari300/20261018_GlobeMode`) and the tools pinned by [mise](https://mise.jdx.dev/) (`mise trust && mise install --locked`).

| Surface | How |
| --- | --- |
| CLI | `cargo build --release -p globe-mode-cli`, then `target/release/globe check -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit` (MSL: `cargo xtask fetch-msl`) |
| Browser | `mise run wasm:build`, then `cd src/Svelte && pnpm install --frozen-lockfile && pnpm build`, then serve `dist/` with any static server. Or run the workflow `web.yml` and download the artifact `globe-web`. |
| Desktop | Run the workflow `desktop.yml` and download the artifact `desktop-linux` (.deb), `desktop-windows` (.msi) or `desktop-macos` (.dmg). Unsigned, so the OS warns. |
| Android | Run the workflow `android.yml` and download the artifact `globe-android` (debug APK). |
| VS Code | Build `globe`, then `cd src/VSCode/globe-mode && pnpm install && pnpm package` and install the `.vsix`. |
| Python | `uv sync` in the checkout, then `from globe_mode import Workspace, table`. |
| C, Zig, Julia, Elixir, Gleam | `mise run c:zig`, `c:julia`, `elixir:test`, `gleam:test` run the reference scenario. |

## Known limits

Of the 566 MSL 4.1.0 examples, 369 pass all checks, 13 fail them (model errors or structural singularity) and 184 hit a compiler limitation (`GM-U`, mostly in Rumoca v0.10.2). Of the 356 runnable examples that pass, 312 run to their stop time, and 303 of the 311 that have a reference result stay inside the reference tube (relative 0.2 %) of the MSL 4.1.0 reference results ([doc/msl-funnel.md](doc/msl-funnel.md), [doc/msl-sim.md](doc/msl-sim.md), run of 2026-11-03).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option. Unless you explicitly state otherwise, any contribution you intentionally submit for inclusion in the work, as defined in the Apache-2.0 license, will be dual licensed as above, without any additional terms or conditions. How dependencies, Rumoca and the Modelica Standard Library are treated: [doc/licenses.md](doc/licenses.md).

---

## 日本語

GlobeMode は Rust で作る OSS の Modelica IDE。同じ Rust のコアを、CLI `globe`、言語サーバ、Tauri の Desktop（Linux・Windows・macOS）、Android、ブラウザ（WebAssembly。計算は閲覧者の端末）、Python、C・Zig・Julia・Elixir・Gleam から使う。コンパイラのフロントエンドは [Rumoca](https://github.com/CogniPilot/rumoca)（Apache-2.0。git の rev で固定）をアダプタ越しに使う。

> **状態: 公開前。** GitHub の Release・タグ・Pages・PyPI・crates.io・VS Code の Marketplace には、まだ何も出していない。入れるには、チェックアウトからビルドするか、GitHub Actions のワークフローの artifact を使う。Desktop と Android のビルドは署名していない。変更点は [CHANGELOG.md](CHANGELOG.md)。

### できること

[doc/features.md](doc/features.md) の ○ のうち、いま main で動くもの。

- **実行前の検査**（S0〜S10）: 構文、名前、型、接続、方程式数と変数数、構造特異、インデックスと状態選択、初期化、代数ループ、パラメータと start 値の評価。実行できない理由を名指しする。方程式数と変数数（S5）は OpenModelica の `omc checkModel()` と、判定のある MSL の例題 382 本すべてで一致した（omc は照合のために別プロセスで呼ぶだけ。[doc/omc-compare.md](doc/omc-compare.md)）。
- **シミュレーション**: 20 以上のソルバ（`bdf`・`dopri45`・`auto`・`esdirk34`・`radau5`・`rk4` など。`cvode`・`ida` は feature `sundials` のビルドだけ。一覧は [doc/msl-solvers.md](doc/msl-solvers.md)）、パラメータの上書き、CSV と MAT v4 の出力、進捗と中断。
- **編集**: 整形・lint・補完・ホバー・定義ジャンプ・参照・リネーム（部品とクラス名。開いている文書の中）を、言語サーバ、VS Code の拡張、ブラウザ・Desktop・Android の UI から。UI にはライブラリツリー、プロット（系列の選択・右の軸・CSV / PNG の書き出し）、connect グラフ、タブ、ファイルを開く・保存、実行の設定、FMU の書き出し、ダークモードがある。
- **FMU の書き出し**: FMI 2.0・3.0 の Model Exchange・Co-Simulation を、CLI・UI・各言語の API から。FMI の読み込みは未実装。
- **API**: Rust（facade `globe-mode`）、Python（PyO3）、C ABI、Zig、Julia、Elixir、Gleam、WebAssembly。

できていないもの: FMI の読み込み、SSP、Diagram / Icon / Documentation ビュー、自前のコンパイラ（Track B）、署名したインストーラと自動更新（鍵を登録するまで更新は未設定）、ブラウザ版の公開。順位つきの一覧は [doc/features.md](doc/features.md)、順序は [doc/roadmap.md](doc/roadmap.md)。

UI のキーボード（ブラウザ・Desktop・Android。macOS は Ctrl の代わりに Cmd。エディタの中でも入力欄でも効く）: Ctrl+Enter 実行（CodeMirror の「空行を挿入」は覆う）、Ctrl+Shift+Enter 検査、Ctrl+. 止める、Ctrl+S 保存（Desktop は Ctrl+Shift+S で名前を付けて保存）、Ctrl+O 開く。診断と参照の行は本物の button で、Tab で選んで Enter か Space を押すとエディタの該当位置へ移る。プロットの canvas には信号名と時間の範囲の代替テキストが付く。ページの先頭にスキップリンク（エディタへ・結果へ）があり、タブは左右の矢印・Home・End で移れ、未保存のタブには ● が付く。初めて開いたとき（保存したタブが無いとき）だけ、はじめの 3 手順の案内が出る（閉じられる）。

### 使い始め方

どれも、チェックアウト（`git clone https://github.com/asari300/20261018_GlobeMode`）と、[mise](https://mise.jdx.dev/) が固定した道具（`mise trust && mise install --locked`）から始める。道具の分担は [ADR-0003](doc/adr/0003-toolchain.md)、手順は [doc/mise-setup.html](doc/mise-setup.html)。

#### MSL を取る

多くの例題は Modelica Standard Library（MSL）4.1.0 を読む。

```bash
cargo xtask fetch-msl            # data/msl/ に取得（sha256 を確かめて展開。コミットしない）
```

CLI・言語サーバには `-L data/msl` で渡す（サブフォルダではなく `data/msl` ごと）。ブラウザ・Desktop・Android の UI は「MSL を読む」ボタンから読む（Desktop は初回に取得する。[ADR-0030](doc/adr/0030-desktop-msl-placement.md)）。

#### CLI `globe`

```bash
cargo build --release -p globe-mode-cli    # target/release/globe（PATH に置く）

# 検査（S0〜S10）と実行。MSL の ChuaCircuit で検査に約 2 秒、実行に約 2 秒（静かな WSL2 での実測 1.6 s・2.1 s）
globe check    -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit
globe simulate -L data/msl -m Modelica.Electrical.Analog.Examples.ChuaCircuit -o chua.csv   # .mat なら MAT v4
globe simulate --list-solvers
# -o を省くと結果は書かず、要約だけ（stderr に「結果は書いていない」）。-o - は標準出力。info の診断は件数だけ（-v で全部）

# 自分のファイルやフォルダ（モデルの完全修飾名を -m に）
globe check src/Modelica/GlobeModeValidation -m GlobeModeValidation.Simulation.Decay

globe fmt --check model.mo   # 自分のファイルかフォルダを指す。整形が要るファイルを列挙して終了コード 1。--check を外すと書き換える（リポジトリの根の `.` は検証用の入力も含むので通らない）
globe lint .              # line-too-long など。--disable <規則> で止める
globe fmu export src/Modelica/GlobeModeValidation -m GlobeModeValidation.Simulation.Decay --out out/ [--fmi-version 3]
globe lsp -L data/msl     # 標準入出力の LSP サーバ
```

ほかに `globe outline`・`graph`・`flat`・`deps`。`globe <コマンド> --help` で全部の引数が出る。設定はカレントディレクトリから上へたどって最初の `globe-mode.toml`（[doc/format-style.md](doc/format-style.md)）。`cvode`・`ida` が要るときは cmake と C コンパイラを入れて `--features sundials` を付ける。

#### ブラウザ

wasm と UI をビルドして、静的ファイルとして配る。計算はブラウザの Web Worker で行い、サーバでは計算しない。

```bash
cargo xtask fetch-msl                      # 先に。「MSL を読む」用の MSL.json が UI に入る（無くても動く）
mise run wasm:build                        # release の wasm（globe-mode-wasm）
cd src/Svelte
pnpm install --frozen-lockfile
pnpm build                                 # dist/
python3 -m http.server 8000 -d dist        # http://localhost:8000/ を開く
```

開発用の `pnpm dev` と試験は [src/Svelte/README.md](src/Svelte/README.md)。ビルド済みの束は、手動のワークフロー `web.yml`（`gh workflow run web.yml`）が artifact `globe-web` に出す（`gh run download <run-id> -n globe-web -D globe-web`、中身は `dist/` と CSP の header の断片）。公開先（Pages など）はまだ無い。

#### Desktop（Tauri）

手動のワークフロー `desktop.yml`（`gh workflow run desktop.yml`。Linux・Windows・macOS で約 30 分）が、インストーラを artifact に出す。

| OS | artifact | 入れ方 |
| --- | --- | --- |
| Linux（Debian 系） | `desktop-linux`（.deb） | `sudo apt install ./<ファイル>.deb` |
| Windows 11 | `desktop-windows`（.msi） | ダブルクリック |
| macOS（Apple silicon） | `desktop-macos`（.dmg）、`desktop-macos-app`（.app の tar） | .dmg を開いて `/Applications` へ |

タグ `v*` を打つと `release.yml` が macOS（Apple silicon）の .dmg・`globe` の tar.gz・Python の wheel も作って Release に付ける（まだタグは打っていない。手動の実行は dry run で artifact に残すだけ）。取得は `gh run download <run-id> -n <artifact>`。**署名も公証もしていない**ので、OS が警告を出す（Windows の SmartScreen、macOS の Gatekeeper。macOS は「システム設定」の「プライバシーとセキュリティ」で「このまま開く」を押す。Apple の案内 <https://support.apple.com/en-us/102445>、確認日 2026-11-05）。起動して「MSL を読む」を押すと MSL 4.1.0 を取得する。Desktop は検査と実行を sidecar の `globe lsp` に任せる（[ADR-0023](doc/adr/0023-desktop-and-web-process-layout.md)）。作業ディレクトリの `globe-mode.toml` を整形と lint に使う。

#### Android

手動のワークフロー `android.yml`（`gh workflow run android.yml`。main への push でも、Desktop と UI の変更のときに動く）が、署名なしの debug APK（artifact `globe-android`、aarch64・x86_64 入りの universal）を出す。

```bash
gh run download <run-id> -n globe-android
adb install app-universal-debug.apk
```

Android は sidecar を使えず、計算は Web Worker 上の wasm（ブラウザと同じ）。MSL は APK に同梱する。ABI ごとの release APK は artifact `globe-android-release`（署名は鍵の secret があるときだけ。無ければ署名なしで、そのままは入れられない）。実機での確認は、まだエミュレータ（API 34）の自動の操作だけ。

#### VS Code の拡張

`.mo` を開くと `globe lsp` につなぐ小さな拡張（整形・lint・補完・検査・定義ジャンプ・構文の色付け）。Marketplace には出していないので、`.vsix` を作って入れる。

```bash
cargo build --release -p globe-mode-cli       # globe（PATH に置くか、設定 globeMode.path に絶対パス）
cd src/VSCode/globe-mode
mise exec -- pnpm install --frozen-lockfile
mise exec -- pnpm package                      # globe-mode-<版>.vsix
```

VS Code の拡張機能ビュー（Ctrl+Shift+X）の「…」→「VSIX からのインストール...」で選ぶ。MSL は設定 `globeMode.libraries` に並べる。整形は Shift+Alt+F。詳細は [src/VSCode/globe-mode/README.md](src/VSCode/globe-mode/README.md)。

#### Python

```bash
uv sync          # チェックアウトの中。maturin が globe-mode-py をビルドして editable に入れる（Rust が要る）
```

```python
from globe_mode import Workspace, table

ws = Workspace(libraries=["data/msl"])  # cargo xtask fetch-msl で取得した MSL
print(ws.check("Modelica.Electrical.Analog.Examples.ChuaCircuit")["passed"])      # True
df = table(ws.simulate_columns("Modelica.Electrical.Analog.Examples.ChuaCircuit"))  # polars.DataFrame
print(df.shape, df.columns[:3])                                                    # (50219, 45) ['time', 'L.i', 'L.v']
```

PyPI には出していない。marimo・Jupyter の折れ線 widget は extra `widget`（`uv sync --extra widget`）。ノートブックは [src/Python/marimo/README.md](src/Python/marimo/README.md)、ダッシュボードは [src/Python/streamlit/README.md](src/Python/streamlit/README.md)。

#### C・Zig・Julia・Elixir・Gleam

どれも C ABI（`globe-mode-c`）か NIF（`globe-mode-nif`）の薄い束ねで、`GlobeModeValidation.Simulation.Decay` を開いて check → simulate → 列 → 整形 → FMU の書き出し（Elixir・Gleam は check → simulate まで）を通す参照シナリオがある。

| 言語 | 実行 | 説明 |
| --- | --- | --- |
| C / Zig | `mise run c:zig` | [src/Zig/README.md](src/Zig/README.md) |
| Julia | `mise run c:julia` | [src/Julia/README.md](src/Julia/README.md) |
| Elixir | `mise run elixir:test` | [src/Elixir/README.md](src/Elixir/README.md) |
| Gleam | `mise run gleam:test` | [src/Gleam/README.md](src/Gleam/README.md) |

Hex・Julia のパッケージ登録はしていない。

### 既知の限界

MSL 4.1.0 の例題 566 本のうち、S0〜S10 の検査を通るのは 369 本。通らないのは 13 本（モデルの誤りか構造特異）、コンパイラ側の限界で判定できないのは 184 本（`GM-U`。多くは Rumoca v0.10.2 の穴で、[doc/upstream.md](doc/upstream.md) に集めた）。検査を通った例題のうち実行の対象にした 356 本（partial などを除く）では、312 本が終了時刻まで実行でき、MAP-LIB の参照結果（MSL 4.1.0）と比べられた 311 本のうち 303 本（97.4%）が、参照結果の管（相対の幅 0.2%）に収まる。出典は [doc/msl-funnel.md](doc/msl-funnel.md) と [doc/msl-sim.md](doc/msl-sim.md)（2026-11-03 の実行）。MSL 以外の公開ライブラリ 7 つの結果は [doc/libraries.md](doc/libraries.md)。

そのほかの限界:

- `globe` を crates.io には出せない。Rumoca を git 依存で固定しているため（[ADR-0002](doc/adr/0002-rumoca-adapter.md)）。
- 参照・リネームの範囲は、開いている文書の中の部品とクラス名（S166）。別ファイル・ライブラリ全体の参照は未。
- 補完は予約語と本文の名前に加え、型の位置・点つきの名前でライブラリのクラス、`.` の後ろで部品の名前（S171。エディタ（ブラウザ・Desktop）と言語サーバ）。
- ライブラリツリーの検索は名前の部分一致（`.` を含めると完全修飾名。S167）。
- Desktop の自動更新は、鍵を登録するまで「未設定」。
- Desktop（Windows・macOS・Linux）と Android は、署名と、実機での長い操作の確認が未（CI の自動の操作は通っている）。

### 開発に加わる

作業は git worktree の feature ブランチで行い、`main` を直接編集しない。規則と締めの手順は [CLAUDE.md](CLAUDE.md)（worktree、ライセンス、文書、ログ、検査）。

```bash
mise run check        # Rust の fmt / clippy / test / deny と uv sync --locked
mise run docs:check   # スライドとリンクの検査（Playwright）
```

全体像は [doc/architecture.md](doc/architecture.md)、設計判断は [doc/adr/](doc/adr/)、セッションごとの記録は [doc/sessions/](doc/sessions/README.md)。

### エディタ機能（整形・lint・LSP）

`.mo` は CLI（`globe fmt` / `globe lint` / `globe lsp`）、VS Code 拡張（[src/VSCode/globe-mode](src/VSCode/globe-mode)、Shift+Alt+F で整形）、ブラウザの UI と Web アプリ（[doc/web-apps.md](doc/web-apps.md)）から同じ規則で整形・検査できる。規則の詳細は [doc/format-style.md](doc/format-style.md)。

#### 処理の流れ

```mermaid
flowchart LR
  src[".mo の本文"] --> entry{"入口<br/>CLI / VS Code / ブラウザ / Web アプリ"}
  entry -->|整形| fmt["globe-mode-fmt<br/>字句 → 構造 → 80 桁の配置<br/>annotation の並べ替え・HTML の段付け"]
  fmt --> guard{"空白以外の<br/>トークンが不変？<br/>（HTML の文字列は空白以外の文字）"}
  guard -->|はい| out["整形した本文"]
  guard -->|いいえ| keep["元のまま + エラー"]
  entry -->|lint| lint["globe-mode-fmt の lint<br/>GM-L0001〜0004"]
  entry -->|lint / LSP| outline["Rumoca のアウトライン<br/>→ クラスの索引"]
  outline --> types["connect の型の突き合わせ<br/>GM-L0005"]
  outline --> graph["connect グラフ"]
  entry -->|検査| check["S0〜S10 の検査<br/>（globe check・LSP の診断）"]
  check --> sim["コンパイル → 実行"]
```

#### 今の規則

| 種類 | 規則 | 既定 | 切り方 |
| --- | --- | --- | --- |
| 整形 | インデント 2・行幅 80 桁。収まらない呼び出しと配列は開き括弧で改行 | オン | `--indent 4` |
| 整形 | 図形プリミティブの引数を並べ替える（位置・形状 → points → fill → line） | オン | — |
| 整形 | `annotation` を experiment → Diagram → Icon → Documentation、`Documentation` を header → info → revisions に並べ替える | オン | — |
| 整形 | 図形プリミティブに `origin = {0, 0}` を足す | オン | `--no-insert-origin` |
| 整形 | `within` の後・クラス頭の後・`end` の前に空行 | オン | — |
| 整形 | `equation` / `algorithm` の前後に空行 | オン | `--no-blank-around-sections`、VS Code の `globeMode.blankAroundSections` |
| 整形 | `Documentation` の HTML を改行とインデントで整える（空白だけを変える。lint はしない。S062） | オン | `--no-format-html`、VS Code の `globeMode.formatHtml` |
| 整形 | 演算子の両側に空白、カンマは後ろだけ、行末の空白を消す | オン | — |
| lint | `line-too-long`（GM-L0001）: 整形しても 80 桁を超える行 | warning | `globe lint --disable <規則>` |
| lint | `class-name-pascal-case`（GM-L0002）: クラス名が大文字で始まらない | warning | 同上 |
| lint | `missing-description`（GM-L0003）: クラスに説明の文字列が無い | warning | 同上 |
| lint | `annotation-order`（GM-L0004）: annotation の引数が上の順でない。整形で直る | 通知 | 同上 |
| lint | `connect-type-mismatch`（GM-L0005）: connect の両端のコネクタの型が合わない（S062） | warning | 同上 |
| 検査 | S0〜S10（構文、方程式数と変数数、構造特異、初期化、代数ループ、プリフライト） | error / warning | — |

#### 今後必要になりそうな機能

| 機能 | 今 | 理由 |
| --- | --- | --- |
| LSP がワークスペースのフォルダ全体を読む | 開いたファイルだけ | 開いていないファイルの型を connect の型の突き合わせで解決するため |
| MSL などライブラリの型での connect の型の突き合わせ | ワークスペースの型だけ | `--library` で読んだ型は索引に無く、突き合わせを省いている |
| connect の向きの検査（input 同士、output 同士） | 無い | 型が合っていてもつなげない組み合わせがある |
| 配列の大きさ・単位の食い違いの検査 | 無い | 型の検査の次の段 |
| 参照・リネームの範囲の拡張 | 定義ジャンプ・参照・リネーム・ホバーは LSP と UI（ブラウザ・Desktop・Android の F12・Shift+F12・F2）にある（S132・S138・S149・S161）。範囲は開いている文書の中で、リネームはその文書で定義した部品とクラスに限る（S166） | 別ファイル・ライブラリ全体の参照と、継承した部品・型・クラスのリネームは未 |
| `globe fmt --diff` と `--exclude` | 無い | ruff と同じ操作性にするため |
| 整形の設定ファイル | `globe-mode.toml`（S130）。CLI・LSP・VS Code・Desktop（S143）が読む。ブラウザの UI は読まず、既定（インデント 2 ほか）を使う | プロジェクトごとに規則を揃えるため |
| ブラウザの UI と Web アプリで空行・HTML の規則を切る口 | 既定のオンだけ | VS Code・CLI と揃えるため |
| `src/Modelica` の既存ファイルを今の規則で整形 | していない | 差分が大きいので、ユーザーの判断を待つ |
| FMI の読み込み・書き出し | 書き出しは FMI 2.0・3.0 の ME・CS（S094・S098、事象の無いモデル）。読み込みは未 | [doc/roadmap.md](doc/roadmap.md) |

| 文書 | 内容 |
| --- | --- |
| [doc/features.md](doc/features.md) | OpenModelica と Rumoca を参考に整理した、必要な機能の一覧 |
| [doc/architecture.md](doc/architecture.md) | crate の構成、遅延評価、実行前のアサーション、ソルバ、FMI、connect グラフ |
| [doc/roadmap.md](doc/roadmap.md) | P0〜P8 と Track B（自前のコア）、各段階の完了条件 |
| [doc/licenses.md](doc/licenses.md) | ライセンス方針。依存の許可・条件付き・禁止 |
| [doc/upstream.md](doc/upstream.md) | 上流（Rumoca・diffsol）への issue の下書きの一覧、提出の順と手順 |
| [doc/adr/](doc/adr/) | 設計判断の記録 |
| [doc/research/](doc/research/) | 調査の記録（出典 URL と確認日つき） |
| [doc/sessions/](doc/sessions/) | セッションごとの HTML スライドと解説書 |
| [CLAUDE.md](CLAUDE.md) | このリポジトリでの作業ルール |

ライセンスは MIT OR Apache-2.0。依存を足すときは [doc/licenses.md](doc/licenses.md) の区分に従う。Rumoca（Apache-2.0）は `globe-mode-frontend` と `globe-mode-exec` からだけ使い、OpenModelica は読むだけ（`omc` を別プロセスで呼ぶ照合を除く）。MSL 4.1.0 は取得するだけでコミットしない。
